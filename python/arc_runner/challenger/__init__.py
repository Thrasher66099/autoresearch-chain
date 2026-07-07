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
Challenger replay/audit runner.

A challenger disputes a block by replaying evidence and submitting
a bonded challenge when the replay reveals a score mismatch.

The runner handles protocol interaction only:
  - Discovers accepted blocks in the domain
  - Fetches evidence and delegates replay to an injected function
  - Opens a challenge via the protocol client when fraud is detected

Like the proposer and validator, actual experiment replay is injected —
the runner handles protocol interaction only.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass

from arc_runner.client import ArcNodeClient, generate_id
from arc_runner.evidence import EvidenceBundler


@dataclass
class ChallengerConfig:
    """Configuration for a challenger runner."""

    node_binary: str    # path to arc-node binary
    state_path: str     # path to protocol state file
    store_dir: str      # path to artifact store
    domain_id: str      # 64-char hex domain ID
    challenger_id: str  # 64-char hex challenger ID


class ChallengerRunner:
    """Executes the Stage 1 challenger audit loop.

    The runner discovers accepted blocks in the domain, fetches
    evidence from the local artifact store, delegates replay to a
    caller-provided function, and opens challenges when fraud is
    detected.

    Like the proposer and validator, actual experiment replay is
    injected — the runner handles protocol interaction only.
    """

    def __init__(self, config: ChallengerConfig) -> None:
        self.client = ArcNodeClient(config.state_path, config.node_binary)
        self.bundler = EvidenceBundler(config.store_dir)
        self.config = config

    def find_suspect_blocks(self) -> list[dict]:
        """Query for challengeable blocks in this domain.

        Returns block details for blocks with status UnderChallenge —
        these are blocks that have passed validation and are within
        the challenge window.
        """
        result = self.client.list_blocks(self.config.domain_id)
        blocks = result.get("blocks", [])
        suspects = []
        for entry in blocks:
            if entry.get("status") == "UnderChallenge":
                block_detail = self.client.show_block(entry["block_id"])
                suspects.append(block_detail)
        return suspects

    def fetch_evidence(self, block: dict) -> dict | None:
        """Fetch and parse the evidence manifest for a block.

        Same pattern as ValidatorRunner.fetch_evidence — retrieves
        the JSON manifest by its hash and checks artifact availability.

        Returns a dict with evidence_bundle_hash, available, manifest.
        Returns None if the block has no evidence hash.
        """
        block_data = block.get("block", block)
        evidence_hash = block_data.get("evidence_bundle_hash")
        if evidence_hash is None:
            return None

        manifest_bytes = self.bundler.fetch(evidence_hash)
        if manifest_bytes is None:
            return {
                "evidence_bundle_hash": evidence_hash,
                "available": False,
            }

        manifest = json.loads(manifest_bytes)
        all_available = all(
            self.bundler.exists(h)
            for h in manifest.values()
            if isinstance(h, str) and len(h) == 64
        )
        return {
            "evidence_bundle_hash": evidence_hash,
            "available": all_available,
            "manifest": manifest,
        }

    def open_challenge(
        self,
        block_id: str,
        evidence_ref: str,
        bond: int,
    ) -> dict:
        """Open a challenge against a block.

        Args:
            block_id: The 64-char hex ID of the block being challenged.
            evidence_ref: 64-char hex hash of the challenge evidence
                artifact stored in the local store.
            bond: The bond amount posted by the challenger.

        Returns:
            The arc-node response dict from open-challenge.
        """
        timestamp = int(time.time())
        challenge_id = generate_id(
            self.config.challenger_id.encode("utf-8"),
            block_id.encode("utf-8"),
            str(timestamp).encode("utf-8"),
        )

        params = {
            "challenge_id": challenge_id,
            "challenge_type": "BlockReplay",
            "target": {"Block": {"block_id": block_id}},
            "challenger": self.config.challenger_id,
            "bond": bond,
            "evidence_ref": evidence_ref,
        }

        return self.client.open_challenge(params)
