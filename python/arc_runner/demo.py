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

Demonstrates the full protocol lifecycle with real computation across
two generations of improvement:
  1. Initialize protocol state
  2. Package QMD genesis with real seed scoring
  3. Activate domain
  4. Run heuristic training (real strategy search)
  5. Submit block with evidence
  6. Validate by independent replay
  7. Evaluate and accept block
  8. Settle and finalize
  9. Pull the frontier block's verified materialized state
  10. Agent edit + second-generation training on that state
  11. Submit, validate, and accept generation 2; resolve the diff chain

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
    extend_diversity_vocabulary,
    persist_best_config,
    replay_and_verify,
    run_evaluation,
    run_training,
)
from arc_runner.domains.qmd_query_expansion import QMDGenesisPackager
from arc_runner.evidence import EvidenceBundler
from arc_runner.materialize import resolve_diff_chain
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


def _validate_block(
    client: ArcNodeClient,
    arc_node_bin: str,
    state_path: str,
    store_dir: Path,
    domain_id: str,
    block_id: str,
    workspace: str,
    claimed_score: float,
    delta: float,
) -> None:
    """Each assigned validator independently replays and attests."""
    assign_result = client.assign_validators(block_id)
    assigned = assign_result["assigned_validators"]

    block_detail = client.show_block(block_id)
    bundler = EvidenceBundler(store_dir)

    for idx, validator_hex in enumerate(assigned, 1):
        validator = ValidatorRunner(ValidatorConfig(
            node_binary=arc_node_bin,
            state_path=state_path,
            store_dir=str(store_dir),
            domain_id=domain_id,
            validator_id=validator_hex,
        ))

        evidence_info = validator.fetch_evidence(block_detail)
        assert evidence_info is not None and evidence_info["available"]

        replay_result = replay_and_verify(
            workspace=workspace,
            evidence_manifest=evidence_info["manifest"],
            bundler=bundler,
            claimed_score=claimed_score,
        )

        replay_ref = bundler.hash_bytes(
            json.dumps(replay_result, sort_keys=True).encode("utf-8")
        )
        validator.submit_attestation(
            block_id, replay_result["vote"], delta, replay_ref
        )

        obs = replay_result["observed_score"]
        vote = replay_result["vote"]
        print(f"  Validator {idx}: replayed -> {obs:.3f} -> {vote.upper()}")


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

    # --- Phase 4: Run experiment (generation 1) ---
    _print_phase(4, "Run experiment (generation 1)")
    adapter = AutoresearchAdapter(raw_genesis, store_dir)
    workspace = adapter.pull_frontier()
    workspace2 = None

    try:
        adapter.enforce_surfaces(workspace)

        print("  Running heuristic training (36 trials)...")
        training_result = run_training(workspace)
        best_score = training_result["best_score"]
        delta = best_score - seed_score
        print(f"  Best score: {best_score:.3f} (delta: +{delta:.3f})")

        # Commit the winning config into the codebase (search surface) so
        # the improvement is part of the block's materialized state.
        persist_best_config(workspace, training_result["best_config"])

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
        _validate_block(
            client, arc_node_bin, state_path, store_dir, domain_id,
            block_id, workspace, experiment_result["score"],
            experiment_result["delta"],
        )

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

        # --- Phase 9: Pull materialized frontier state ---
        _print_phase(9, "Pull materialized frontier state")
        block1 = client.show_block(block_id)["block"]
        gen1_state_ref = block1["child_state_ref"]
        workspace2 = adapter.pull_frontier(state_ref=gen1_state_ref)
        adapter.enforce_surfaces(workspace2)
        print(f"  Materialized state {gen1_state_ref[:16]}... verified")
        print("  Frozen surfaces intact; generation-1 config present")

        # --- Phase 10: Second-generation experiment ---
        _print_phase(10, "Second-generation experiment")
        # Agent edit on the search surface: extend the diversity
        # vocabulary the strategy search can draw from.
        extend_diversity_vocabulary(workspace2)
        print("  Agent edited train.py (extended diversity vocabulary)")

        training_result2 = run_training(workspace2)
        best_score2 = training_result2["best_score"]
        delta2 = best_score2 - best_score
        print(f"  Best score: {best_score2:.3f} (delta: +{delta2:.3f})")
        assert best_score2 > best_score, "Generation 2 did not improve"

        persist_best_config(workspace2, training_result2["best_config"])
        experiment_result2 = adapter.capture_result(
            workspace2, best_score, parent_state_ref=gen1_state_ref
        )
        assert adapter.should_submit(experiment_result2)

        # --- Phase 11: Submit and validate generation 2 ---
        _print_phase(11, "Submit and validate generation 2")
        parent_id = proposer.get_frontier_parent()
        assert parent_id == block_id, "Frontier should be the generation-1 block"
        result = proposer.submit_block(parent_id, experiment_result2)
        block2_id = result["block_id"]
        _validate_block(
            client, arc_node_bin, state_path, store_dir, domain_id,
            block2_id, workspace2, experiment_result2["score"],
            experiment_result2["delta"],
        )
        result = client.evaluate_block(block2_id)
        print(f"  Outcome: {result['outcome']}")

        frontier = client.show_frontier(domain_id)["canonical_frontier"]
        assert frontier == block2_id, "Frontier should advance to generation 2"

        # The chain is a versioned substrate: the diff chain resolves from
        # the genesis seed to the tip state.
        block2 = client.show_block(block2_id)["block"]
        bundler = EvidenceBundler(store_dir)
        resolved = resolve_diff_chain(
            [block1["diff_ref"], block2["diff_ref"]],
            raw_genesis["seed_codebase_state_ref"],
            bundler,
        )
        assert resolved == block2["child_state_ref"]
        print("  Frontier advanced to generation 2")
        print("  Diff chain seed -> gen1 -> gen2 resolves and verifies")

    finally:
        shutil.rmtree(workspace, ignore_errors=True)
        if workspace2 is not None:
            shutil.rmtree(workspace2, ignore_errors=True)

    # --- Summary ---
    pct1 = (best_score - seed_score) / seed_score * 100
    pct2 = (best_score2 - seed_score) / seed_score * 100
    print(f"\nCOMPLETE: Seed {seed_score:.3f} -> Gen1 {best_score:.3f} (+{pct1:.1f}%)"
          f" -> Gen2 {best_score2:.3f} (+{pct2:.1f}%)")
    print("-" * 48)


if __name__ == "__main__":
    run_demo()
