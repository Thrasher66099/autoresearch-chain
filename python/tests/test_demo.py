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
Tests for the QMD experiment execution engine and demo lifecycle.

TestQMDExperiment: pure computation tests — no arc-node required.
TestRealComputationThroughProtocol: full lifecycle with real computation,
    requires arc-node binary.
"""

from __future__ import annotations

import json
import os
import shutil
from pathlib import Path

import pytest

from arc_runner.domains.qmd_experiment import (
    BASELINE_CONFIG,
    find_codebase_root,
    replay_and_verify,
    run_evaluation,
    run_training,
)
from arc_runner.evidence import EvidenceBundler


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _fixtures_dir() -> Path:
    """Return the path to the real QMD fixtures directory."""
    repo_root = Path(__file__).resolve().parent.parent.parent
    fixtures = repo_root / "fixtures" / "qmd" / "finetune"
    if not fixtures.exists():
        pytest.skip("fixtures/qmd/finetune not found")
    return fixtures


def hex_id(n: int) -> str:
    """Generate a 64-char hex ID from a byte value."""
    return format(n, "02x") * 32


def find_arc_node() -> str:
    """Locate the arc-node binary."""
    env_bin = os.environ.get("ARC_NODE_BIN")
    if env_bin and Path(env_bin).exists():
        return env_bin

    search = Path(__file__).resolve().parent
    for _ in range(10):
        cargo_toml = search / "Cargo.toml"
        if cargo_toml.exists():
            for profile in ["debug", "release"]:
                candidate = search / "target" / profile / "arc-node"
                if candidate.exists():
                    return str(candidate)
                candidate_exe = candidate.with_suffix(".exe")
                if candidate_exe.exists():
                    return str(candidate_exe)
            break
        search = search.parent

    pytest.skip(
        "arc-node binary not found. Build with: cargo build --bin arc-node"
    )
    return ""


# ---------------------------------------------------------------------------
# Tests: pure computation (no arc-node needed)
# ---------------------------------------------------------------------------

class TestQMDExperiment:
    """Pure computation tests for the QMD experiment engine."""

    def test_baseline_score_is_stable(self):
        """Baseline config produces a deterministic, known score."""
        fixtures = _fixtures_dir()
        result = run_evaluation(str(fixtures), BASELINE_CONFIG)

        assert result["num_queries"] == 5
        assert result["reward_score"] == pytest.approx(0.3786, abs=0.01)

        # Run again — must be identical.
        result2 = run_evaluation(str(fixtures), BASELINE_CONFIG)
        assert result["reward_score"] == result2["reward_score"]

    def test_training_finds_improvement_over_baseline(self, tmp_path: Path):
        """Training search finds a config that beats the baseline."""
        fixtures = _fixtures_dir()

        # Copy fixtures to workspace (train writes output to workspace).
        workspace = tmp_path / "workspace"
        shutil.copytree(fixtures, workspace / "finetune")

        baseline = run_evaluation(str(workspace), BASELINE_CONFIG)
        training_result = run_training(str(workspace))

        assert training_result["best_score"] > baseline["reward_score"]
        assert training_result["num_trials"] == 36
        assert training_result["best_config"] is not None

        # Verify output files were written.
        assert (workspace / "config.yaml").exists()
        assert (workspace / "training.log").exists()
        assert (workspace / "metrics.json").exists()

        # Verify metrics.json content.
        metrics = json.loads((workspace / "metrics.json").read_text())
        assert metrics["reward_score"] == training_result["best_score"]

    def test_evaluation_is_deterministic(self):
        """Same config + same queries = same score, always."""
        fixtures = _fixtures_dir()
        config = {
            "use_think_tags": True,
            "diversity_terms": ["context", "background"],
            "template": "standard",
            "min_length": 60,
        }

        scores = [
            run_evaluation(str(fixtures), config)["reward_score"]
            for _ in range(5)
        ]
        assert len(set(scores)) == 1, f"Non-deterministic scores: {scores}"

    def test_replay_matches_training_output(self, tmp_path: Path):
        """Validator replay of training output produces identical score."""
        fixtures = _fixtures_dir()

        # Copy fixtures to workspace.
        workspace = tmp_path / "workspace"
        shutil.copytree(fixtures, workspace / "finetune")

        # Run training.
        training_result = run_training(str(workspace))
        claimed_score = training_result["best_score"]

        # Create evidence bundle from output files.
        store_dir = tmp_path / "store"
        bundler = EvidenceBundler(store_dir)
        bundle = bundler.bundle(
            diff_path=workspace / "config.yaml",  # Reuse as diff placeholder.
            config_path=workspace / "config.yaml",
            log_path=workspace / "training.log",
            metrics_path=workspace / "metrics.json",
        )

        # Replay using the evidence manifest.
        replay_result = replay_and_verify(
            workspace=str(workspace),
            evidence_manifest=bundle.as_dict(),
            bundler=bundler,
            claimed_score=claimed_score,
        )

        assert replay_result["config_recovered"] is True
        assert replay_result["vote"] == "Pass"
        assert replay_result["observed_score"] == pytest.approx(
            claimed_score, abs=1e-4
        )

    def test_find_codebase_root_direct(self, tmp_path: Path):
        """find_codebase_root handles direct extraction."""
        (tmp_path / "reward.py").write_text("# reward")
        assert find_codebase_root(tmp_path) == tmp_path

    def test_find_codebase_root_nested(self, tmp_path: Path):
        """find_codebase_root handles nested (tarball) extraction."""
        nested = tmp_path / "finetune"
        nested.mkdir()
        (nested / "reward.py").write_text("# reward")
        assert find_codebase_root(tmp_path) == nested

    def test_find_codebase_root_missing_raises(self, tmp_path: Path):
        """find_codebase_root raises when no finetune dir is found."""
        with pytest.raises(FileNotFoundError, match="Cannot locate"):
            find_codebase_root(tmp_path)

    def test_replay_fails_on_wrong_score(self, tmp_path: Path):
        """Replay detects score mismatch and votes Fail."""
        fixtures = _fixtures_dir()

        workspace = tmp_path / "workspace"
        shutil.copytree(fixtures, workspace / "finetune")

        training_result = run_training(str(workspace))

        store_dir = tmp_path / "store"
        bundler = EvidenceBundler(store_dir)
        bundle = bundler.bundle(
            diff_path=workspace / "config.yaml",
            config_path=workspace / "config.yaml",
            log_path=workspace / "training.log",
            metrics_path=workspace / "metrics.json",
        )

        # Claim a different score.
        replay_result = replay_and_verify(
            workspace=str(workspace),
            evidence_manifest=bundle.as_dict(),
            bundler=bundler,
            claimed_score=0.999,
        )

        assert replay_result["vote"] == "Fail"
        assert replay_result["difference"] > 0.1


# ---------------------------------------------------------------------------
# Tests: full lifecycle with real computation (requires arc-node)
# ---------------------------------------------------------------------------

class TestRealComputationThroughProtocol:
    """Full protocol lifecycle with real QMD computation.

    Requires arc-node binary — tests are skipped if not available.
    """

    def test_real_training_through_protocol(self, tmp_path: Path):
        """Real training → submission → validator replay → accepted block."""
        arc_node_bin = find_arc_node()

        from arc_runner.autoresearch_adapter import AutoresearchAdapter
        from arc_runner.client import ArcNodeClient
        from arc_runner.domains import prepare_genesis
        from arc_runner.domains.qmd_query_expansion import QMDGenesisPackager
        from arc_runner.proposer import ProposerConfig, ProposerRunner
        from arc_runner.validator import ValidatorConfig, ValidatorRunner

        store_dir = tmp_path / "artifacts"
        store_dir.mkdir()
        state_path = str(tmp_path / "state.json")
        proposer_id = hex_id(1)

        # --- Phase 1: Initialize protocol ---
        client = ArcNodeClient(state_path, arc_node_bin)
        client.init()

        # --- Phase 2: Package genesis with real fixtures ---
        fixtures = _fixtures_dir()
        packager = QMDGenesisPackager(fixtures, store_dir)
        raw_genesis = packager.package()

        # Compute real seed score.
        seed_result = run_evaluation(str(fixtures), BASELINE_CONFIG)
        raw_genesis["seed_score"] = seed_result["reward_score"]

        genesis = prepare_genesis(raw_genesis, proposer_id)

        # --- Phase 3: Activate domain ---
        client.submit_genesis(genesis)
        client.evaluate_conformance(genesis["id"])
        for i in range(1, 4):
            client.record_seed_validation(genesis["id"], {
                "validator": hex_id(i),
                "vote": "Pass",
                "observed_score": seed_result["reward_score"],
                "timestamp": 1700000000 + i,
            })
        client.finalize_activation(genesis["id"])
        pool = {
            "domain_id": genesis["domain_id"],
            "validators": [hex_id(i) for i in range(1, 11)],
        }
        client.register_validators(pool)

        domain_id = genesis["domain_id"]
        genesis_id = genesis["id"]

        # --- Phase 4: Run real experiment ---
        adapter = AutoresearchAdapter(raw_genesis, store_dir)
        workspace = adapter.pull_frontier()
        try:
            adapter.enforce_surfaces(workspace)

            # Run real training.
            training_result = run_training(workspace)
            assert training_result["best_score"] > seed_result["reward_score"]

            # Capture result.
            experiment_result = adapter.capture_result(
                workspace, seed_result["reward_score"]
            )
            assert adapter.should_submit(experiment_result)
        finally:
            # Keep workspace alive for validator replay.
            pass

        # --- Phase 5: Submit block ---
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

        # --- Phase 6: Validate by real replay ---
        assign_result = client.assign_validators(block_id)
        assigned = assign_result["assigned_validators"]
        assert len(assigned) == 3

        block_detail = client.show_block(block_id)
        bundler = EvidenceBundler(store_dir)

        for validator_hex in assigned:
            validator_config = ValidatorConfig(
                node_binary=arc_node_bin,
                state_path=state_path,
                store_dir=str(store_dir),
                domain_id=domain_id,
                validator_id=validator_hex,
            )
            validator = ValidatorRunner(validator_config)

            # Fetch evidence.
            evidence_info = validator.fetch_evidence(block_detail)
            assert evidence_info is not None
            assert evidence_info["available"] is True

            # Real replay.
            replay_result = replay_and_verify(
                workspace=workspace,
                evidence_manifest=evidence_info["manifest"],
                bundler=bundler,
                claimed_score=experiment_result["score"],
            )
            assert replay_result["vote"] == "Pass"

            # Store replay evidence and submit attestation.
            replay_ref = bundler.hash_bytes(
                json.dumps(replay_result, sort_keys=True).encode("utf-8")
            )
            result = validator.submit_attestation(
                block_id,
                replay_result["vote"],
                experiment_result["delta"],
                replay_ref,
            )
            assert result["status"] == "attestation_recorded"

        # --- Phase 7: Evaluate block ---
        result = client.evaluate_block(block_id)
        assert result["outcome"] == "Accepted"

        frontier = client.show_frontier(domain_id)
        assert frontier["canonical_frontier"] == block_id

        # --- Phase 8: Settlement ---
        client.close_challenge_window(block_id)
        for _ in range(5):
            client.advance_epoch()
        result = client.settle_block(block_id)
        assert result["status"] == "block_settled"
        result = client.finalize_block(block_id)
        assert result["status"] == "block_finalized"

        # Cleanup workspace.
        shutil.rmtree(workspace, ignore_errors=True)
