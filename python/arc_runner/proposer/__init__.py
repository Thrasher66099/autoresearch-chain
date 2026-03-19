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
Proposer execution runner.

A proposer runs the Stage 1 research-mining loop:

    1. Pull the current canonical frontier state for a domain.
    2. Run a bounded experiment (autonomous agent modifies the recipe,
       trains, evaluates).
    3. If the result improves the metric, produce a diff.
    4. Package a complete evidence bundle (diff, config, environment
       manifest, dataset refs, logs, metrics).
    5. Submit the block to the protocol.

The runner does NOT own experiment execution — the experiment function
is injected by the caller. The runner handles protocol interaction only:
frontier query, block construction, and submission. This keeps it
testable with synthetic experiment data.
"""

from __future__ import annotations

import time
from dataclasses import dataclass

from arc_runner.client import ArcNodeClient, generate_id
from arc_runner.evidence import EvidenceBundler, blake3_bytes


@dataclass
class ProposerConfig:
    """Configuration for a proposer runner."""

    node_binary: str    # path to arc-node binary
    state_path: str     # path to protocol state file
    store_dir: str      # path to artifact store
    domain_id: str      # 64-char hex domain ID
    proposer_id: str    # 64-char hex proposer ID
    genesis_id: str     # 64-char hex genesis block ID (parent for first block)
    bond: int           # bond amount per block
    fee: int            # fee per block


class ProposerRunner:
    """Executes the Stage 1 proposer loop.

    The runner queries the protocol for the current frontier,
    delegates experiment execution to a caller-provided function,
    packages the result as an evidence bundle, and submits a block.

    It does NOT run experiments itself — the experiment function
    is injected, keeping the runner protocol-focused.
    """

    def __init__(self, config: ProposerConfig) -> None:
        self.client = ArcNodeClient(config.state_path, config.node_binary)
        self.bundler = EvidenceBundler(config.store_dir)
        self.config = config

    def get_frontier_parent(self) -> str:
        """Query the protocol for the current frontier block ID.

        Returns the block ID to use as parent. If no frontier exists
        (fresh domain with no accepted blocks), falls back to the
        genesis block ID from the config.
        """
        result = self.client.show_frontier(self.config.domain_id)
        frontier = result.get("canonical_frontier")
        if frontier is None:
            return self.config.genesis_id
        return frontier

    def submit_block(
        self,
        parent_id: str,
        experiment_result: dict,
    ) -> dict:
        """Package experiment result and submit a block to the protocol.

        Args:
            parent_id: The block ID of the parent (frontier) block.
            experiment_result: A dict from AutoresearchAdapter.capture_result()
                or equivalent, containing at minimum:
                - evidence_bundle: an EvidenceBundle with all artifact hashes
                - delta: the claimed metric improvement

        Returns:
            The arc-node response dict from submit-block.

        The method:
            1. Computes the evidence_bundle_hash from all artifact hashes
            2. Computes a child_state_ref from the diff hash
            3. Generates a unique block ID via BLAKE3
            4. Constructs the Block JSON
            5. Calls client.submit_block()
        """
        evidence_bundle = experiment_result["evidence_bundle"]
        delta = experiment_result["delta"]

        # Evidence bundle hash: BLAKE3 of sorted artifact hashes concatenated.
        sorted_hashes = sorted(evidence_bundle.all_hashes())
        bundle_hash_input = "".join(sorted_hashes).encode("utf-8")
        evidence_bundle_hash = blake3_bytes(bundle_hash_input)

        # Child state ref: BLAKE3 of the diff content hash.
        # For Phase 2, this is the diff hash itself — full materialized
        # state references are deferred to Phase 3.
        child_state_ref = evidence_bundle.diff_hash

        # Diff ref: the diff hash from the evidence bundle.
        diff_ref = evidence_bundle.diff_hash

        # Generate a unique block ID.
        timestamp = int(time.time())
        block_id = generate_id(
            self.config.proposer_id.encode("utf-8"),
            parent_id.encode("utf-8"),
            diff_ref.encode("utf-8"),
            str(timestamp).encode("utf-8"),
        )

        block = {
            "id": block_id,
            "domain_id": self.config.domain_id,
            "parent_id": parent_id,
            "proposer": self.config.proposer_id,
            "child_state_ref": child_state_ref,
            "diff_ref": diff_ref,
            "claimed_metric_delta": delta,
            "evidence_bundle_hash": evidence_bundle_hash,
            "fee": self.config.fee,
            "bond": self.config.bond,
            "epoch_id": 1,
            "status": "Submitted",
            "timestamp": timestamp,
        }

        return self.client.submit_block(block)
