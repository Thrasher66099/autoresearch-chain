# Copyright (C) 2026 AutoResearch Chain contributors
#
# This file is part of AutoResearch Chain.
#
# AutoResearch Chain is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# AutoResearch Chain is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
# See the GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

"""
End-to-end integration tests for the Python protocol client and runners.

These tests require the ``arc-node`` binary to be built::

    cargo build --bin arc-node

The test exercises the full research loop via the Python client:
genesis activation, block submission via the proposer runner,
validation via the validator runner, and settlement/finalization.

All experiment data is synthetic — no real training runs.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

from arc_runner.autoresearch_adapter import AutoresearchAdapter
from arc_runner.client import ArcNodeClient, ProtocolError, generate_id
from arc_runner.domains import prepare_genesis
from arc_runner.domains.qmd_query_expansion import QMDGenesisPackager
from arc_runner.evidence import EvidenceBundle, EvidenceBundler, blake3_bytes
from arc_runner.proposer import ProposerConfig, ProposerRunner
from arc_runner.validator import ValidatorConfig, ValidatorRunner


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def hex_id(n: int) -> str:
    """Generate a 64-char hex ID from a byte value, matching Rust test pattern."""
    return format(n, "02x") * 32


def find_arc_node() -> str:
    """Locate the arc-node binary.

    Checks (in order):
    1. ARC_NODE_BIN environment variable
    2. target/debug/arc-node (cargo debug build)
    3. target/release/arc-node (cargo release build)
    """
    env_bin = os.environ.get("ARC_NODE_BIN")
    if env_bin and Path(env_bin).exists():
        return env_bin

    # Walk upward to find the repo root (contains Cargo.toml).
    search = Path(__file__).resolve().parent
    for _ in range(10):
        cargo_toml = search / "Cargo.toml"
        if cargo_toml.exists():
            for profile in ["debug", "release"]:
                candidate = search / "target" / profile / "arc-node"
                if candidate.exists():
                    return str(candidate)
                # Windows: arc-node.exe
                candidate_exe = candidate.with_suffix(".exe")
                if candidate_exe.exists():
                    return str(candidate_exe)
            break
        search = search.parent

    pytest.skip(
        "arc-node binary not found. Build with: cargo build --bin arc-node"
    )
    return ""  # unreachable, but satisfies type checker


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def arc_node_bin() -> str:
    """Path to the arc-node binary."""
    return find_arc_node()


@pytest.fixture
def work_dir(tmp_path: Path) -> Path:
    """Temporary working directory for a test."""
    return tmp_path


@pytest.fixture
def state_path(work_dir: Path) -> str:
    """Path for the protocol state file."""
    return str(work_dir / "state.json")


@pytest.fixture
def store_dir(work_dir: Path) -> Path:
    """Temporary artifact store."""
    store = work_dir / "artifacts"
    store.mkdir()
    return store


@pytest.fixture
def client(arc_node_bin: str, state_path: str) -> ArcNodeClient:
    """Initialized ArcNodeClient with fresh state."""
    c = ArcNodeClient(state_path, arc_node_bin)
    c.init()
    return c


# ---------------------------------------------------------------------------
# Canonical test data (matches Rust integration.rs)
# ---------------------------------------------------------------------------

def genesis_data() -> dict:
    """Genesis block matching the Rust test fixtures."""
    return {
        "id": hex_id(1),
        "rts_version": "Rts1",
        "domain_id": hex_id(1),
        "proposer": hex_id(1),
        "research_target_declaration": (
            "Improve CIFAR-10 training recipe accuracy "
            "within fixed compute budget"
        ),
        "domain_intent": "EndToEndRecipeImprovement",
        "seed_recipe_ref": hex_id(10),
        "seed_codebase_state_ref": hex_id(11),
        "frozen_surface": ["eval/", "datasets/"],
        "search_surface": ["train.py", "config/", "models/"],
        "canonical_dataset_ref": hex_id(20),
        "dataset_hash": hex_id(21),
        "dataset_splits": {
            "training": hex_id(22),
            "validation": hex_id(23),
            "test": hex_id(24),
        },
        "evaluation_harness_ref": hex_id(30),
        "metric_id": "test_accuracy",
        "metric_direction": "HigherBetter",
        "hardware_class": "RTX 4090",
        "time_budget_secs": 3600,
        "seed_environment_manifest_ref": hex_id(40),
        "seed_score": 0.93,
        "artifact_schema_ref": hex_id(50),
        "seed_bond": 1000,
        "license_declaration": "MIT",
        "timestamp": 1700000000,
    }


def seed_validation_data(validator_n: int) -> dict:
    """Seed validation record matching Rust test fixtures."""
    return {
        "validator": hex_id(validator_n),
        "vote": "Pass",
        "observed_score": 0.93,
        "timestamp": 1700000000 + validator_n,
    }


def validator_pool_data() -> dict:
    """Validator pool matching Rust test fixtures."""
    return {
        "domain_id": hex_id(1),
        "validators": [hex_id(i) for i in range(1, 11)],
    }


def synthetic_evidence_bundle(store_dir: Path) -> EvidenceBundle:
    """Create a synthetic evidence bundle with stored artifacts."""
    bundler = EvidenceBundler(store_dir)
    diff_hash = bundler.hash_bytes(b"--- a/train.py\n+++ b/train.py\n@@ -1 +1 @@\n-lr=0.001\n+lr=0.0005\n")
    config_hash = bundler.hash_bytes(b"lr: 0.0005\nepochs: 10")
    env_hash = bundler.hash_bytes(b'{"python_version": "3.10", "platform": "linux"}')
    log_hash = bundler.hash_bytes(b"epoch 1: loss=0.5\nepoch 2: loss=0.3\n")
    metrics_hash = bundler.hash_bytes(b'{"score": 0.945, "loss": 0.28}')
    return EvidenceBundle(
        diff_hash=diff_hash,
        config_hash=config_hash,
        env_manifest_hash=env_hash,
        training_log_hash=log_hash,
        metric_output_hash=metrics_hash,
    )


# ---------------------------------------------------------------------------
# Tests: client basics
# ---------------------------------------------------------------------------

class TestClientBasics:
    """Basic client operations that don't need the full domain setup."""

    def test_init_creates_state(self, arc_node_bin: str, work_dir: Path):
        state = str(work_dir / "fresh.json")
        c = ArcNodeClient(state, arc_node_bin)
        c.init()
        assert Path(state).exists()

    def test_inspect_returns_stderr(self, client: ArcNodeClient):
        text = client.inspect()
        assert "Epoch:" in text

    def test_init_fails_if_exists(self, client: ArcNodeClient):
        with pytest.raises(ProtocolError, match="already exists"):
            client.init()

    def test_advance_epoch(self, client: ArcNodeClient):
        result = client.advance_epoch()
        assert result["epoch"] == 1
        result = client.advance_epoch()
        assert result["epoch"] == 2

    def test_list_domains_empty(self, client: ArcNodeClient):
        result = client.list_domains()
        assert result["domain_count"] == 0

    def test_show_frontier_nonexistent_domain(self, client: ArcNodeClient):
        result = client.show_frontier(hex_id(99))
        assert result["canonical_frontier"] is None

    def test_show_block_nonexistent(self, client: ArcNodeClient):
        with pytest.raises(ProtocolError, match="not found"):
            client.show_block(hex_id(99))


# ---------------------------------------------------------------------------
# Tests: domain activation
# ---------------------------------------------------------------------------

class TestDomainActivation:
    """Test the genesis → conformance → seed validation → activation flow."""

    def test_full_activation(self, client: ArcNodeClient):
        genesis_id = hex_id(1)
        domain_id = hex_id(1)

        # Submit genesis.
        result = client.submit_genesis(genesis_data())
        assert result["genesis_id"] == genesis_id

        # Evaluate conformance.
        result = client.evaluate_conformance(genesis_id)
        assert result["status"] == "conformance_passed"

        # Record 3 seed validations.
        for i in range(1, 4):
            result = client.record_seed_validation(
                genesis_id, seed_validation_data(i)
            )
            assert result["status"] == "seed_validation_recorded"

        # Finalize activation.
        result = client.finalize_activation(genesis_id)
        assert result["status"] == "domain_activated"
        assert result["domain_id"] == domain_id

        # Register validators.
        result = client.register_validators(validator_pool_data())
        assert result["status"] == "validators_registered"
        assert result["validator_count"] == 10

        # Verify domain is listed.
        result = client.list_domains()
        assert result["domain_count"] == 1


# ---------------------------------------------------------------------------
# Helpers for full lifecycle tests
# ---------------------------------------------------------------------------

def activate_domain(client: ArcNodeClient) -> str:
    """Activate a domain and register validators. Returns domain_id."""
    genesis_id = hex_id(1)
    client.submit_genesis(genesis_data())
    client.evaluate_conformance(genesis_id)
    for i in range(1, 4):
        client.record_seed_validation(genesis_id, seed_validation_data(i))
    client.finalize_activation(genesis_id)
    client.register_validators(validator_pool_data())
    return hex_id(1)


# ---------------------------------------------------------------------------
# Tests: full block lifecycle via client
# ---------------------------------------------------------------------------

class TestBlockLifecycleViaClient:
    """Exercise the full block lifecycle using the client directly."""

    def test_submit_and_validate_block(self, client: ArcNodeClient):
        domain_id = activate_domain(client)
        genesis_id = hex_id(1)

        # Submit a block.
        block_id = hex_id(10)
        block = {
            "id": block_id,
            "domain_id": domain_id,
            "parent_id": genesis_id,
            "proposer": hex_id(1),
            "child_state_ref": hex_id(60),
            "diff_ref": hex_id(160),
            "claimed_metric_delta": 0.015,
            "evidence_bundle_hash": hex_id(200),
            "fee": 10,
            "bond": 500,
            "epoch_id": 1,
            "status": "Submitted",
            "timestamp": 1700001000,
        }
        result = client.submit_block(block)
        assert result["block_id"] == block_id

        # Assign validators.
        result = client.assign_validators(block_id)
        assigned = result["assigned_validators"]
        assert len(assigned) == 3

        # Submit attestations.
        for validator_hex in assigned:
            attestation = {
                "block_id": block_id,
                "validator": validator_hex,
                "vote": "Pass",
                "observed_delta": 0.015,
                "replay_evidence_ref": hex_id(70),
                "timestamp": 1700002000,
            }
            result = client.submit_attestation(attestation)
            assert result["status"] == "attestation_recorded"

        # Evaluate block.
        result = client.evaluate_block(block_id)
        assert result["outcome"] == "Accepted"

        # Verify block details.
        result = client.show_block(block_id)
        assert result["derived_validity"] == "DirectValid"

        # Verify frontier.
        result = client.show_frontier(domain_id)
        assert result["canonical_frontier"] == block_id

        # List blocks.
        result = client.list_blocks()
        assert result["block_count"] == 1
        result = client.list_blocks(domain_id)
        assert result["block_count"] == 1

        # Close challenge window.
        result = client.close_challenge_window(block_id)
        assert result["status"] == "challenge_window_closed"

        # Advance epochs.
        for _ in range(5):
            client.advance_epoch()

        # Settle.
        result = client.settle_block(block_id)
        assert result["status"] == "block_settled"

        # Finalize.
        result = client.finalize_block(block_id)
        assert result["status"] == "block_finalized"


# ---------------------------------------------------------------------------
# Tests: full lifecycle via proposer + validator runners
# ---------------------------------------------------------------------------

class TestRunnerIntegration:
    """End-to-end test using ProposerRunner and ValidatorRunner."""

    def test_proposer_submit_and_validator_attest(
        self,
        arc_node_bin: str,
        state_path: str,
        store_dir: Path,
        client: ArcNodeClient,
    ):
        domain_id = activate_domain(client)
        genesis_id = hex_id(1)

        # --- Proposer side ---

        proposer_config = ProposerConfig(
            node_binary=arc_node_bin,
            state_path=state_path,
            store_dir=str(store_dir),
            domain_id=domain_id,
            proposer_id=hex_id(1),
            genesis_id=genesis_id,
            bond=500,
            fee=10,
        )
        proposer = ProposerRunner(proposer_config)

        # Query frontier — should be the genesis block (no blocks yet).
        parent_id = proposer.get_frontier_parent()
        assert parent_id == genesis_id

        # Create synthetic experiment result.
        evidence = synthetic_evidence_bundle(store_dir)
        experiment_result = {
            "evidence_bundle": evidence,
            "delta": 0.015,
        }

        # Submit block via proposer.
        result = proposer.submit_block(parent_id, experiment_result)
        block_id = result["block_id"]
        assert len(block_id) == 64

        # --- Protocol: assign validators ---

        assign_result = client.assign_validators(block_id)
        assigned = assign_result["assigned_validators"]
        assert len(assigned) == 3

        # --- Validator side ---

        # Submit attestations from each assigned validator.
        replay_evidence_ref = hex_id(70)
        for validator_hex in assigned:
            validator_config = ValidatorConfig(
                node_binary=arc_node_bin,
                state_path=state_path,
                store_dir=str(store_dir),
                domain_id=domain_id,
                validator_id=validator_hex,
            )
            validator = ValidatorRunner(validator_config)
            result = validator.submit_attestation(
                block_id, "Pass", 0.015, replay_evidence_ref
            )
            assert result["status"] == "attestation_recorded"

        # --- Protocol: evaluate block ---

        result = client.evaluate_block(block_id)
        assert result["outcome"] == "Accepted"

        # Verify via queries.
        block_detail = client.show_block(block_id)
        assert block_detail["derived_validity"] == "DirectValid"

        frontier = client.show_frontier(domain_id)
        assert frontier["canonical_frontier"] == block_id

    def test_proposer_frontier_updates_after_block(
        self,
        arc_node_bin: str,
        state_path: str,
        store_dir: Path,
        client: ArcNodeClient,
    ):
        """After a block is accepted, the proposer should see it as the new frontier."""
        domain_id = activate_domain(client)
        genesis_id = hex_id(1)

        proposer_config = ProposerConfig(
            node_binary=arc_node_bin,
            state_path=state_path,
            store_dir=str(store_dir),
            domain_id=domain_id,
            proposer_id=hex_id(1),
            genesis_id=genesis_id,
            bond=500,
            fee=10,
        )
        proposer = ProposerRunner(proposer_config)

        # First block.
        parent_id = proposer.get_frontier_parent()
        assert parent_id == genesis_id

        evidence = synthetic_evidence_bundle(store_dir)
        result = proposer.submit_block(parent_id, {
            "evidence_bundle": evidence,
            "delta": 0.015,
        })
        block_id = result["block_id"]

        # Accept block: assign → attest → evaluate.
        assign_result = client.assign_validators(block_id)
        for v in assign_result["assigned_validators"]:
            client.submit_attestation({
                "block_id": block_id,
                "validator": v,
                "vote": "Pass",
                "observed_delta": 0.015,
                "replay_evidence_ref": hex_id(70),
                "timestamp": 1700002000,
            })
        client.evaluate_block(block_id)

        # Frontier should now be the new block.
        new_parent = proposer.get_frontier_parent()
        assert new_parent == block_id
        assert new_parent != genesis_id

    def test_full_lifecycle_through_settlement(
        self,
        arc_node_bin: str,
        state_path: str,
        store_dir: Path,
        client: ArcNodeClient,
    ):
        """Complete lifecycle: propose → validate → accept → settle → finalize."""
        domain_id = activate_domain(client)
        genesis_id = hex_id(1)

        # Propose.
        proposer_config = ProposerConfig(
            node_binary=arc_node_bin,
            state_path=state_path,
            store_dir=str(store_dir),
            domain_id=domain_id,
            proposer_id=hex_id(1),
            genesis_id=genesis_id,
            bond=500,
            fee=10,
        )
        proposer = ProposerRunner(proposer_config)
        parent_id = proposer.get_frontier_parent()
        evidence = synthetic_evidence_bundle(store_dir)
        result = proposer.submit_block(parent_id, {
            "evidence_bundle": evidence,
            "delta": 0.015,
        })
        block_id = result["block_id"]

        # Validate.
        assign_result = client.assign_validators(block_id)
        for v in assign_result["assigned_validators"]:
            validator_config = ValidatorConfig(
                node_binary=arc_node_bin,
                state_path=state_path,
                store_dir=str(store_dir),
                domain_id=domain_id,
                validator_id=v,
            )
            validator = ValidatorRunner(validator_config)
            validator.submit_attestation(block_id, "Pass", 0.015, hex_id(70))

        # Accept.
        result = client.evaluate_block(block_id)
        assert result["outcome"] == "Accepted"

        # Close challenge window.
        result = client.close_challenge_window(block_id)
        assert result["status"] == "challenge_window_closed"

        # Advance epochs.
        for _ in range(5):
            client.advance_epoch()

        # Settle.
        result = client.settle_block(block_id)
        assert result["status"] == "block_settled"

        # Finalize.
        result = client.finalize_block(block_id)
        assert result["status"] == "block_finalized"

        # Verify final state.
        inspect_text = client.inspect()
        assert "Epoch:" in inspect_text


# ---------------------------------------------------------------------------
# Full pipeline: real component outputs through the protocol
# ---------------------------------------------------------------------------

# Finetune directory content (matches test_qmd_genesis.py fixtures).

_REWARD_PY = b"""\
# Reward function for QMD query expansion scoring.
def compute_reward(query: str, expansion: str) -> float:
    score = 0.0
    if "<think>" in expansion and "</think>" in expansion:
        score += 30.0
    terms = set(expansion.lower().split())
    score += min(30.0, len(terms) * 0.5)
    if len(expansion) > 50:
        score += 20.0
    score += 20.0
    return min(score / 140.0, 1.0)
"""

_EVAL_PY = b"""\
import json
def evaluate(model_path: str, queries_path: str) -> dict:
    return {"average_score": 0.92, "num_queries": 80, "excellent_count": 30}
"""

_QUERIES_TXT = b"""\
best pizza restaurants near me
how to learn Python programming
climate change effects on coral reefs
latest iPhone reviews and comparisons
history of the Roman Empire
"""

_TRAIN_PY = b"""\
def train(config_path: str):
    print(f"Training with config: {config_path}")
"""

_SFT_CONFIG = b"""\
model_name: Qwen3-1.7B
lora_rank: 16
epochs: 5
learning_rate: 2e-5
batch_size: 4
"""

_DATASET = b"""\
{"query": "best pizza", "expansion": "<think>pizza types</think> best pizza restaurants near me"}
{"query": "learn python", "expansion": "<think>programming</think> how to learn Python programming"}
{"query": "climate change", "expansion": "<think>environment</think> climate change effects"}
"""


def _make_finetune_dir(base: Path) -> Path:
    """Create a synthetic QMD finetune directory."""
    ft = base / "finetune"
    ft.mkdir()
    (ft / "reward.py").write_bytes(_REWARD_PY)
    (ft / "eval.py").write_bytes(_EVAL_PY)
    (ft / "evals").mkdir()
    (ft / "evals" / "queries.txt").write_bytes(_QUERIES_TXT)
    (ft / "train.py").write_bytes(_TRAIN_PY)
    (ft / "configs").mkdir()
    (ft / "configs" / "sft.yaml").write_bytes(_SFT_CONFIG)
    (ft / "data").mkdir()
    (ft / "data" / "train_00.jsonl").write_bytes(_DATASET)
    (ft / "data" / "train_01.jsonl").write_bytes(_DATASET + b"\n")
    return ft


class TestFullPipeline:
    """End-to-end pipeline using real component outputs.

    Exercises the full flow: QMD genesis packager → prepare_genesis() →
    protocol submission → autoresearch adapter → proposer → validator
    evidence fetch → attestation → evaluation.
    """

    def test_packager_genesis_through_protocol(
        self,
        arc_node_bin: str,
        work_dir: Path,
        state_path: str,
        store_dir: Path,
        client: ArcNodeClient,
    ):
        """Package a QMD genesis, bridge it, and activate through the protocol."""
        proposer_id = hex_id(1)

        # 1. Package genesis from synthetic finetune directory.
        finetune_dir = _make_finetune_dir(work_dir)
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        raw_genesis = packager.package()

        # 2. Bridge: add protocol identity fields.
        genesis = prepare_genesis(raw_genesis, proposer_id)
        assert len(genesis["id"]) == 64
        assert genesis["domain_id"] == genesis["id"]
        assert genesis["proposer"] == proposer_id

        # 3. Submit through the protocol.
        result = client.submit_genesis(genesis)
        assert result["genesis_id"] == genesis["id"]

        # 4. Conformance check.
        result = client.evaluate_conformance(genesis["id"])
        assert result["status"] == "conformance_passed"

        # 5. Seed validations.
        for i in range(1, 4):
            result = client.record_seed_validation(
                genesis["id"], seed_validation_data(i)
            )
            assert result["status"] == "seed_validation_recorded"

        # 6. Activate.
        result = client.finalize_activation(genesis["id"])
        assert result["status"] == "domain_activated"
        assert result["domain_id"] == genesis["domain_id"]

        # 7. Register validators.
        pool = {
            "domain_id": genesis["domain_id"],
            "validators": [hex_id(i) for i in range(1, 11)],
        }
        result = client.register_validators(pool)
        assert result["status"] == "validators_registered"

        # 8. Verify domain is listed.
        result = client.list_domains()
        assert result["domain_count"] == 1

    def test_adapter_through_proposer_and_validator(
        self,
        arc_node_bin: str,
        work_dir: Path,
        state_path: str,
        store_dir: Path,
        client: ArcNodeClient,
    ):
        """Full pipeline: adapter experiment → proposer submit → validator evidence fetch."""
        import shutil

        proposer_id = hex_id(1)

        # --- Activate domain using real packager output ---

        finetune_dir = _make_finetune_dir(work_dir)
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        raw_genesis = packager.package()
        genesis = prepare_genesis(raw_genesis, proposer_id)

        client.submit_genesis(genesis)
        client.evaluate_conformance(genesis["id"])
        for i in range(1, 4):
            client.record_seed_validation(genesis["id"], seed_validation_data(i))
        client.finalize_activation(genesis["id"])
        pool = {
            "domain_id": genesis["domain_id"],
            "validators": [hex_id(i) for i in range(1, 11)],
        }
        client.register_validators(pool)

        domain_id = genesis["domain_id"]
        genesis_id = genesis["id"]

        # --- Adapter: pull frontier and capture result ---

        adapter = AutoresearchAdapter(raw_genesis, store_dir)
        workspace = adapter.pull_frontier()
        try:
            # Enforce surfaces — should pass on untouched workspace.
            adapter.enforce_surfaces(workspace)

            # Create synthetic experiment output files in the workspace.
            workspace_path = Path(workspace)
            (workspace_path / "config.yaml").write_text(
                "lr: 0.0005\nepochs: 10\n"
            )
            (workspace_path / "training.log").write_text(
                "epoch 1: loss=0.5\nepoch 2: loss=0.3\n"
            )
            (workspace_path / "metrics.json").write_text(
                '{"reward_score": 0.945, "loss": 0.28}'
            )

            # Capture result.
            seed_score = raw_genesis["seed_score"]
            experiment_result = adapter.capture_result(workspace, seed_score)
            assert experiment_result["delta"] > 0
            assert adapter.should_submit(experiment_result)
        finally:
            shutil.rmtree(workspace, ignore_errors=True)

        # --- Proposer: submit block ---

        proposer_config = ProposerConfig(
            node_binary=arc_node_bin,
            state_path=state_path,
            store_dir=str(store_dir),
            domain_id=domain_id,
            proposer_id=proposer_id,
            genesis_id=genesis_id,
            bond=500,
            fee=10,
        )
        proposer = ProposerRunner(proposer_config)

        parent_id = proposer.get_frontier_parent()
        assert parent_id == genesis_id

        result = proposer.submit_block(parent_id, experiment_result)
        block_id = result["block_id"]
        assert len(block_id) == 64

        # --- Protocol: assign validators ---

        assign_result = client.assign_validators(block_id)
        assigned = assign_result["assigned_validators"]
        assert len(assigned) == 3

        # --- Validator: fetch evidence and attest ---

        block_detail = client.show_block(block_id)

        for validator_hex in assigned:
            validator_config = ValidatorConfig(
                node_binary=arc_node_bin,
                state_path=state_path,
                store_dir=str(store_dir),
                domain_id=domain_id,
                validator_id=validator_hex,
            )
            validator = ValidatorRunner(validator_config)

            # Fetch evidence — must find the manifest and all artifacts.
            evidence_info = validator.fetch_evidence(block_detail)
            assert evidence_info is not None
            assert evidence_info["available"] is True
            assert "manifest" in evidence_info
            manifest = evidence_info["manifest"]
            assert "diff_hash" in manifest
            assert "config_hash" in manifest
            assert "training_log_hash" in manifest
            assert "metric_output_hash" in manifest
            assert "env_manifest_hash" in manifest

            # Submit attestation.
            replay_ref = hex_id(70)
            result = validator.submit_attestation(
                block_id, "Pass", experiment_result["delta"], replay_ref
            )
            assert result["status"] == "attestation_recorded"

        # --- Protocol: evaluate block ---

        result = client.evaluate_block(block_id)
        assert result["outcome"] == "Accepted"

        # Verify frontier updated.
        frontier = client.show_frontier(domain_id)
        assert frontier["canonical_frontier"] == block_id
