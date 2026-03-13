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
End-to-end integration tests for the QMD genesis packager, evidence bundler,
and autoresearch adapter.

These tests use a synthetic finetune directory (no GPU required) to verify
that packaging, hashing, surface enforcement, and artifact storage work
correctly end-to-end.
"""

from __future__ import annotations

import json
import os
import shutil
import tempfile
from pathlib import Path

import pytest

from arc_runner.evidence import EvidenceBundle, EvidenceBundler, blake3_bytes, blake3_file
from arc_runner.domains.qmd_query_expansion import QMDGenesisPackager
from arc_runner.autoresearch_adapter import AutoresearchAdapter


# ---------------------------------------------------------------------------
# Fixtures: synthetic finetune directory
# ---------------------------------------------------------------------------

REWARD_PY_CONTENT = b"""\
# Reward function for QMD query expansion scoring.
# Scores on Format (30), Diversity (30), HyDE (20), Quality (20).
# Max 140 points. Returns 0.0-1.0 normalized score.

def compute_reward(query: str, expansion: str) -> float:
    score = 0.0
    # Format check
    if "<think>" in expansion and "</think>" in expansion:
        score += 30.0
    # Diversity: count unique terms
    terms = set(expansion.lower().split())
    score += min(30.0, len(terms) * 0.5)
    # HyDE: hypothetical document quality
    if len(expansion) > 50:
        score += 20.0
    # Quality baseline
    score += 20.0
    return min(score / 140.0, 1.0)
"""

EVAL_PY_CONTENT = b"""\
# Evaluation harness for QMD query expansion.
# Loads model, generates expansions against test queries, scores with reward.py.

import json

def evaluate(model_path: str, queries_path: str) -> dict:
    # Placeholder: in production, loads the model and runs inference.
    return {"average_score": 0.92, "num_queries": 80, "excellent_count": 30}
"""

QUERIES_TXT_CONTENT = b"""\
best pizza restaurants near me
how to learn Python programming
climate change effects on coral reefs
latest iPhone reviews and comparisons
history of the Roman Empire
"""

TRAIN_PY_CONTENT = b"""\
# Training script for QMD query expansion SFT.
# Uses LoRA rank 16, all projection layers, 5 epochs.

def train(config_path: str):
    # Placeholder training logic.
    print(f"Training with config: {config_path}")
"""

SFT_CONFIG_CONTENT = b"""\
model_name: Qwen3-1.7B
lora_rank: 16
epochs: 5
learning_rate: 2e-5
batch_size: 4
"""

DATASET_CONTENT = b"""\
{"query": "best pizza", "expansion": "<think>pizza types</think> best pizza restaurants near me"}
{"query": "learn python", "expansion": "<think>programming</think> how to learn Python programming"}
{"query": "climate change", "expansion": "<think>environment</think> climate change effects"}
"""


@pytest.fixture
def finetune_dir(tmp_path: Path) -> Path:
    """Create a synthetic finetune directory matching QMD structure."""
    ft = tmp_path / "finetune"
    ft.mkdir()

    # Frozen surface files.
    (ft / "reward.py").write_bytes(REWARD_PY_CONTENT)
    (ft / "eval.py").write_bytes(EVAL_PY_CONTENT)
    (ft / "evals").mkdir()
    (ft / "evals" / "queries.txt").write_bytes(QUERIES_TXT_CONTENT)

    # Search surface files.
    (ft / "train.py").write_bytes(TRAIN_PY_CONTENT)
    (ft / "configs").mkdir()
    (ft / "configs" / "sft.yaml").write_bytes(SFT_CONFIG_CONTENT)

    # Dataset files.
    (ft / "data").mkdir()
    (ft / "data" / "train_00.jsonl").write_bytes(DATASET_CONTENT)
    (ft / "data" / "train_01.jsonl").write_bytes(DATASET_CONTENT + b"\n")

    return ft


@pytest.fixture
def store_dir(tmp_path: Path) -> Path:
    """Create a temporary artifact store directory."""
    store = tmp_path / "artifacts"
    store.mkdir()
    return store


# ---------------------------------------------------------------------------
# Evidence bundler tests
# ---------------------------------------------------------------------------

class TestEvidenceBundler:
    def test_hash_file_roundtrip(self, tmp_path: Path, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        test_file = tmp_path / "test.txt"
        test_file.write_bytes(b"hello world")

        hex_hash = bundler.hash_file(test_file)
        assert len(hex_hash) == 64
        assert bundler.exists(hex_hash)

        fetched = bundler.fetch(hex_hash)
        assert fetched == b"hello world"

    def test_hash_bytes_roundtrip(self, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        data = b"some raw bytes"

        hex_hash = bundler.hash_bytes(data)
        assert len(hex_hash) == 64
        assert bundler.exists(hex_hash)

        fetched = bundler.fetch(hex_hash)
        assert fetched == data

    def test_hash_deterministic(self, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        h1 = bundler.hash_bytes(b"deterministic")
        h2 = bundler.hash_bytes(b"deterministic")
        assert h1 == h2

    def test_different_content_different_hash(self, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        h1 = bundler.hash_bytes(b"content A")
        h2 = bundler.hash_bytes(b"content B")
        assert h1 != h2

    def test_fetch_missing_returns_none(self, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        assert bundler.fetch("a" * 64) is None

    def test_python_rust_hash_agreement(self, store_dir: Path):
        """BLAKE3 of known content should match across Python and Rust."""
        bundler = EvidenceBundler(store_dir)
        # BLAKE3 of b"hello, world!" — must match Rust content_hash().
        hex_hash = bundler.hash_bytes(b"hello, world!")
        expected = "5b92a0a84fbc50a58c74f4717bc0d5f403282ae4cd7d7a384311ed3c418a15d8"
        assert hex_hash == expected

    def test_bundle_creates_complete_evidence(self, tmp_path: Path, store_dir: Path):
        bundler = EvidenceBundler(store_dir)

        # Create evidence files.
        diff = tmp_path / "diff.patch"
        config = tmp_path / "config.yaml"
        log = tmp_path / "train.log"
        metrics = tmp_path / "metrics.json"

        diff.write_bytes(b"--- a/train.py\n+++ b/train.py\n")
        config.write_bytes(b"lr: 0.001")
        log.write_bytes(b"epoch 1: loss=0.5")
        metrics.write_bytes(b'{"score": 0.95}')

        bundle = bundler.bundle(diff, config, log, metrics)

        assert isinstance(bundle, EvidenceBundle)
        assert bundle.is_complete()
        assert all(len(h) == 64 for h in bundle.all_hashes())
        assert all(bundler.exists(h) for h in bundle.all_hashes())

    def test_capture_environment(self, store_dir: Path):
        bundler = EvidenceBundler(store_dir)
        env = bundler.capture_environment()

        assert "python_version" in env
        assert "platform" in env


# ---------------------------------------------------------------------------
# QMD genesis packager tests
# ---------------------------------------------------------------------------

class TestQMDGenesisPackager:
    def test_package_produces_valid_genesis(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        # All required GenesisBlock fields present.
        required_fields = [
            "rts_version",
            "research_target_declaration",
            "domain_intent",
            "seed_recipe_ref",
            "seed_codebase_state_ref",
            "frozen_surface",
            "search_surface",
            "canonical_dataset_ref",
            "dataset_hash",
            "dataset_splits",
            "evaluation_harness_ref",
            "metric_id",
            "metric_direction",
            "hardware_class",
            "time_budget_secs",
            "seed_environment_manifest_ref",
            "seed_score",
            "artifact_schema_ref",
            "seed_bond",
            "license_declaration",
            "timestamp",
        ]
        for field in required_fields:
            assert field in genesis, f"Missing required field: {field}"

    def test_all_hashes_are_valid(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        # All hash fields should be 64-char hex strings.
        hash_fields = [
            "seed_recipe_ref",
            "seed_codebase_state_ref",
            "canonical_dataset_ref",
            "dataset_hash",
            "evaluation_harness_ref",
            "seed_environment_manifest_ref",
            "artifact_schema_ref",
        ]
        for field in hash_fields:
            h = genesis[field]
            assert isinstance(h, str), f"{field} is not a string"
            assert len(h) == 64, f"{field} has wrong length: {len(h)}"
            # Verify it's valid hex.
            int(h, 16)

    def test_artifacts_are_retrievable(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        bundler = EvidenceBundler(store_dir)

        # All stored artifacts should be retrievable.
        hash_fields = [
            "seed_recipe_ref",
            "seed_codebase_state_ref",
            "canonical_dataset_ref",
            "dataset_hash",
            "evaluation_harness_ref",
            "seed_environment_manifest_ref",
            "artifact_schema_ref",
        ]
        for field in hash_fields:
            h = genesis[field]
            assert bundler.exists(h), f"Artifact for {field} not in store: {h[:16]}..."

    def test_frozen_surface_hashes_match(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        frozen_hashes = genesis["_frozen_surface_hashes"]

        # Verify frozen surface hashes match direct file hashing.
        assert frozen_hashes["reward.py"] == blake3_file(finetune_dir / "reward.py")
        assert frozen_hashes["eval.py"] == blake3_file(finetune_dir / "eval.py")
        assert frozen_hashes["evals/queries.txt"] == blake3_file(
            finetune_dir / "evals" / "queries.txt"
        )

    def test_search_surface_declared(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        assert "train.py" in genesis["search_surface"]
        assert "configs/" in genesis["search_surface"]

        search_hashes = genesis["_search_surface_hashes"]
        assert "train.py" in search_hashes
        assert "configs/" in search_hashes
        assert len(search_hashes["train.py"]) == 64
        assert len(search_hashes["configs/"]) == 64

    def test_frozen_and_search_do_not_overlap(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        frozen_set = set(genesis["frozen_surface"])
        search_set = set(genesis["search_surface"])
        assert frozen_set.isdisjoint(search_set), "Frozen and search surfaces overlap"

    def test_genesis_as_json(self, finetune_dir: Path, store_dir: Path):
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        # Should be JSON-serializable.
        json_str = json.dumps(genesis, indent=2)
        assert len(json_str) > 100

        # Round-trip.
        parsed = json.loads(json_str)
        assert parsed["metric_id"] == "reward_score"
        assert parsed["metric_direction"] == "HigherBetter"
        assert parsed["seed_score"] == 0.920

    def test_hash_determinism(self, finetune_dir: Path, store_dir: Path):
        """Packaging twice should produce identical hashes."""
        store1 = store_dir / "run1"
        store2 = store_dir / "run2"

        g1 = QMDGenesisPackager(finetune_dir, store1).package()
        g2 = QMDGenesisPackager(finetune_dir, store2).package()

        # All hash fields should be identical.
        for key in ["seed_recipe_ref", "canonical_dataset_ref", "evaluation_harness_ref"]:
            assert g1[key] == g2[key], f"Non-deterministic hash for {key}"

    def test_missing_file_raises(self, tmp_path: Path, store_dir: Path):
        """Missing required files should raise FileNotFoundError."""
        empty_dir = tmp_path / "empty_finetune"
        empty_dir.mkdir()

        packager = QMDGenesisPackager(empty_dir, store_dir)
        with pytest.raises(FileNotFoundError):
            packager.package()


# ---------------------------------------------------------------------------
# Autoresearch adapter tests
# ---------------------------------------------------------------------------

class TestAutoresearchAdapter:
    def test_enforce_surfaces_passes_for_unmodified(
        self, finetune_dir: Path, store_dir: Path
    ):
        """enforce_surfaces should pass when frozen files are untouched."""
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        adapter = AutoresearchAdapter(genesis, store_dir)

        # Pull frontier (extracts seed tarball).
        workspace = adapter.pull_frontier()
        try:
            # Should pass — files are untouched.
            adapter.enforce_surfaces(workspace)
        finally:
            shutil.rmtree(workspace, ignore_errors=True)

    def test_enforce_surfaces_detects_modification(
        self, finetune_dir: Path, store_dir: Path
    ):
        """enforce_surfaces should raise when a frozen file is modified."""
        packager = QMDGenesisPackager(finetune_dir, store_dir)
        genesis = packager.package()

        adapter = AutoresearchAdapter(genesis, store_dir)

        workspace = adapter.pull_frontier()
        try:
            # Modify a frozen surface file.
            workspace_path = Path(workspace)
            # Find reward.py in the extracted workspace.
            reward_files = list(workspace_path.rglob("reward.py"))
            assert len(reward_files) > 0, "reward.py not found in workspace"
            reward_files[0].write_text("# tampered content\n")

            with pytest.raises(ValueError, match="Frozen surface violation"):
                adapter.enforce_surfaces(workspace)
        finally:
            shutil.rmtree(workspace, ignore_errors=True)

    def test_should_submit_requires_positive_delta(
        self, finetune_dir: Path, store_dir: Path
    ):
        adapter = AutoresearchAdapter({}, store_dir)

        # No evidence bundle → don't submit.
        assert not adapter.should_submit({"delta": 0.1, "evidence_bundle": None})

        # Negative delta → don't submit.
        from arc_runner.evidence import EvidenceBundle
        bundle = EvidenceBundle("a" * 64, "b" * 64, "c" * 64, "d" * 64, "e" * 64)
        assert not adapter.should_submit({"delta": -0.01, "evidence_bundle": bundle})

        # Positive delta with bundle → submit.
        assert adapter.should_submit({"delta": 0.01, "evidence_bundle": bundle})
