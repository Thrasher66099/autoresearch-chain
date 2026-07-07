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
AutoResearch Chain -- QMD Domain Demo.

Demonstrates the full protocol lifecycle with real computation:
  1. Initialize protocol state
  2. Package QMD genesis with real seed scoring
  3. Activate domain
  4. Run heuristic training (real strategy search)
  5. Submit block with evidence
  6. Validate by independent replay
  7. Evaluate and accept block
  8. Settle and finalize

Usage::

    python -m arc_runner.demo

Requires the arc-node binary. Build with::

    cargo build --bin arc-node
"""

from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

from arc_runner.autoresearch_adapter import AutoresearchAdapter
from arc_runner.client import ArcNodeClient
from arc_runner.domains import prepare_genesis
from arc_runner.domains.qmd_experiment import (
    BASELINE_CONFIG,
    replay_and_verify,
    run_evaluation,
    run_training,
)
from arc_runner.domains.qmd_query_expansion import QMDGenesisPackager
from arc_runner.evidence import EvidenceBundler
from arc_runner.proposer import ProposerConfig, ProposerRunner
from arc_runner.validator import ValidatorConfig, ValidatorRunner


def _hex_id(n: int) -> str:
    """Generate a 64-char hex ID from a byte value."""
    return format(n, "02x") * 32


def _find_arc_node() -> str:
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

    print("ERROR: arc-node binary not found.", file=sys.stderr)
    print("Build with: cargo build --bin arc-node", file=sys.stderr)
    sys.exit(1)


def _find_fixtures() -> Path:
    """Locate the QMD fixtures directory."""
    search = Path(__file__).resolve().parent
    for _ in range(10):
        candidate = search / "fixtures" / "qmd" / "finetune"
        if candidate.exists():
            return candidate
        search = search.parent

    print("ERROR: fixtures/qmd/finetune not found.", file=sys.stderr)
    sys.exit(1)


def _print_phase(n: int, title: str) -> None:
    print(f"\n--- Phase {n}: {title} ---")


def run_demo() -> None:
    """Run the full QMD demo lifecycle."""
    print()
    print("AutoResearch Chain -- QMD Domain Demo")
    print("-" * 48)

    arc_node_bin = _find_arc_node()
    fixtures = _find_fixtures()
    proposer_id = _hex_id(1)

    work_dir = tempfile.mkdtemp(prefix="arc_demo_")
    store_dir = Path(work_dir) / "artifacts"
    store_dir.mkdir()
    state_path = str(Path(work_dir) / "state.json")

    try:
        _run_lifecycle(
            arc_node_bin=arc_node_bin,
            fixtures=fixtures,
            proposer_id=proposer_id,
            state_path=state_path,
            store_dir=store_dir,
        )
    except Exception as e:
        print(f"\nERROR: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


def _run_lifecycle(
    arc_node_bin: str,
    fixtures: Path,
    proposer_id: str,
    state_path: str,
    store_dir: Path,
) -> None:
    # --- Phase 1: Initialize protocol state ---
    _print_phase(1, "Initialize protocol state")
    client = ArcNodeClient(state_path, arc_node_bin)
    client.init()
    print(f"  State file: {state_path}")

    # --- Phase 2: Package QMD genesis ---
    _print_phase(2, "Package QMD genesis")
    print("  Computing seed score with baseline config...")
    seed_result = run_evaluation(str(fixtures), BASELINE_CONFIG)
    seed_score = seed_result["reward_score"]
    print(f"  Seed score: {seed_score:.3f}")

    packager = QMDGenesisPackager(fixtures, store_dir)
    raw_genesis = packager.package()
    raw_genesis["seed_score"] = seed_score

    genesis = prepare_genesis(raw_genesis, proposer_id)

    # --- Phase 3: Activate domain ---
    _print_phase(3, "Activate domain")
    client.submit_genesis(genesis)
    client.evaluate_conformance(genesis["id"])
    for i in range(1, 4):
        client.record_seed_validation(genesis["id"], {
            "validator": _hex_id(i),
            "vote": "Pass",
            "observed_score": seed_score,
            "timestamp": 1700000000 + i,
        })
    client.finalize_activation(genesis["id"])
    pool = {
        "domain_id": genesis["domain_id"],
        "validators": [_hex_id(i) for i in range(1, 11)],
    }
    client.register_validators(pool)
    print("  Genesis submitted, conformance passed, domain activated")

    domain_id = genesis["domain_id"]
    genesis_id = genesis["id"]

    # --- Phase 4: Run experiment ---
    _print_phase(4, "Run experiment")
    adapter = AutoresearchAdapter(raw_genesis, store_dir)
    workspace = adapter.pull_frontier()

    try:
        adapter.enforce_surfaces(workspace)

        print(f"  Running heuristic training (36 trials)...")
        training_result = run_training(workspace)
        best_score = training_result["best_score"]
        delta = best_score - seed_score
        print(f"  Best score: {best_score:.3f} (delta: +{delta:.3f})")

        experiment_result = adapter.capture_result(workspace, seed_score)
        assert adapter.should_submit(experiment_result), "Experiment did not improve on baseline"

        # --- Phase 5: Submit block ---
        _print_phase(5, "Submit block")
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
        result = proposer.submit_block(parent_id, experiment_result)
        block_id = result["block_id"]

        evidence_bundle = experiment_result["evidence_bundle"]
        artifact_count = len(evidence_bundle.all_hashes())
        print(f"  Block submitted with {artifact_count} evidence artifacts")

        # --- Phase 6: Validate by replay ---
        _print_phase(6, "Validate by replay")
        assign_result = client.assign_validators(block_id)
        assigned = assign_result["assigned_validators"]

        block_detail = client.show_block(block_id)
        bundler = EvidenceBundler(store_dir)

        for idx, validator_hex in enumerate(assigned, 1):
            validator_config = ValidatorConfig(
                node_binary=arc_node_bin,
                state_path=state_path,
                store_dir=str(store_dir),
                domain_id=domain_id,
                validator_id=validator_hex,
            )
            validator = ValidatorRunner(validator_config)

            evidence_info = validator.fetch_evidence(block_detail)
            assert evidence_info is not None and evidence_info["available"]

            replay_result = replay_and_verify(
                workspace=workspace,
                evidence_manifest=evidence_info["manifest"],
                bundler=bundler,
                claimed_score=experiment_result["score"],
            )

            replay_ref = bundler.hash_bytes(
                json.dumps(replay_result, sort_keys=True).encode("utf-8")
            )
            validator.submit_attestation(
                block_id,
                replay_result["vote"],
                experiment_result["delta"],
                replay_ref,
            )

            obs = replay_result["observed_score"]
            vote = replay_result["vote"]
            print(f"  Validator {idx}: replayed -> {obs:.3f} -> {vote.upper()}")

        # --- Phase 7: Evaluate block ---
        _print_phase(7, "Evaluate block")
        result = client.evaluate_block(block_id)
        outcome = result["outcome"]
        print(f"  Outcome: {outcome}, frontier updated")

        # --- Phase 8: Settlement ---
        _print_phase(8, "Settlement")
        client.close_challenge_window(block_id)
        for _ in range(5):
            client.advance_epoch()
        client.settle_block(block_id)
        client.finalize_block(block_id)
        print("  Block settled and finalized")

    finally:
        shutil.rmtree(workspace, ignore_errors=True)

    # --- Summary ---
    pct = (best_score - seed_score) / seed_score * 100
    print(f"\nCOMPLETE: Seed {seed_score:.3f} -> Final {best_score:.3f} (+{pct:.1f}%)")
    print("-" * 48)


if __name__ == "__main__":
    run_demo()
