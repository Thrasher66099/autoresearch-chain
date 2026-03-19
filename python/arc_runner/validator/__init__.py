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
Validator replay runner.

A validator replays a parent/child transition and attests whether
the claimed improvement reproduces under protocol rules.

The runner discovers blocks assigned for validation, fetches
evidence from the local artifact store, delegates replay to a
caller-provided function, and submits attestations.

Like the proposer, actual experiment replay is injected — the
runner handles protocol interaction only.
"""

from __future__ import annotations

import time
from dataclasses import dataclass

from arc_runner.client import ArcNodeClient
from arc_runner.evidence import EvidenceBundler


@dataclass
class ValidatorConfig:
    """Configuration for a validator runner."""

    node_binary: str    # path to arc-node binary
    state_path: str     # path to protocol state file
    store_dir: str      # path to artifact store
    domain_id: str      # 64-char hex domain ID
    validator_id: str   # 64-char hex validator ID


class ValidatorRunner:
    """Executes the Stage 1 validator replay loop.

    The runner discovers blocks assigned for validation, fetches
    evidence from the local artifact store, delegates replay to a
    caller-provided function, and submits attestations.

    Like the proposer, actual experiment replay is injected — the
    runner handles protocol interaction only.
    """

    def __init__(self, config: ValidatorConfig) -> None:
        self.client = ArcNodeClient(config.state_path, config.node_binary)
        self.bundler = EvidenceBundler(config.store_dir)
        self.config = config

    def get_pending_blocks(self) -> list[dict]:
        """Query for blocks under validation in this domain.

        Returns block details for blocks with status UnderValidation.
        Filters the block list by domain and status.
        """
        result = self.client.list_blocks(self.config.domain_id)
        blocks = result.get("blocks", [])
        pending = []
        for entry in blocks:
            if entry.get("status") == "UnderValidation":
                block_detail = self.client.show_block(entry["id"])
                pending.append(block_detail)
        return pending

    def fetch_evidence(self, block: dict) -> dict | None:
        """Fetch the evidence bundle artifacts for a block from the local store.

        Looks up the evidence_bundle_hash from the block and checks
        that the artifact exists in the local store.

        Returns a dict with the evidence_bundle_hash and whether
        it was found, or None if the block has no evidence hash.
        """
        block_data = block.get("block", block)
        evidence_hash = block_data.get("evidence_bundle_hash")
        if evidence_hash is None:
            return None

        found = self.bundler.exists(evidence_hash)
        return {
            "evidence_bundle_hash": evidence_hash,
            "available": found,
        }

    def submit_attestation(
        self,
        block_id: str,
        vote: str,
        observed_delta: float | None,
        replay_evidence_ref: str,
    ) -> dict:
        """Construct and submit a validation attestation.

        Args:
            block_id: The 64-char hex ID of the block being validated.
            vote: One of "Pass", "Fail", "Inconclusive", "FraudSuspected".
            observed_delta: The metric delta observed during replay,
                or None if replay did not produce a measurement.
            replay_evidence_ref: 64-char hex hash of the replay evidence
                artifact stored in the local store.

        Returns:
            The arc-node response dict from submit-attestation.
        """
        attestation = {
            "block_id": block_id,
            "validator": self.config.validator_id,
            "vote": vote,
            "observed_delta": observed_delta,
            "replay_evidence_ref": replay_evidence_ref,
            "timestamp": int(time.time()),
        }
        return self.client.submit_attestation(attestation)
