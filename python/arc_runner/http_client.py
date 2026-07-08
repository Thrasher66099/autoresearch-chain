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
HTTP protocol client for a networked arc-node sequencer (Milestone E4).

Mirrors :class:`arc_runner.client.ArcNodeClient`'s method surface, but
talks to an ``arc-node serve`` sequencer over HTTP instead of shelling a
local binary. Write methods POST to ``/tx/<kind>``; queries hit the read
endpoints; artifacts move through the sequencer's content-addressed
``/artifact`` store, so runners on different machines share evidence.

Payloads may carry a ``signature`` field (see ``arc_runner.identity``);
sequencers initialized with ``--require-signatures`` refuse unsigned
actor-bearing submissions.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request

from arc_runner.client import ProtocolError


class HttpArcClient:
    """Protocol client over HTTP against an arc-node sequencer."""

    def __init__(self, base_url: str, timeout: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    # ------------------------------------------------------------------
    # Transport
    # ------------------------------------------------------------------

    def _request(
        self, method: str, path: str, body: bytes | None = None
    ) -> bytes:
        req = urllib.request.Request(
            f"{self.base_url}{path}", data=body, method=method
        )
        if body is not None:
            req.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return resp.read()
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", errors="replace")
            try:
                detail = json.loads(detail).get("error", detail)
            except (json.JSONDecodeError, AttributeError):
                pass
            raise ProtocolError(f"HTTP {e.code}: {detail}", e.code) from e
        except urllib.error.URLError as e:
            raise ProtocolError(f"sequencer unreachable: {e.reason}", -1) from e

    def _get(self, path: str) -> dict:
        return json.loads(self._request("GET", path))

    def submit_tx(self, kind: str, payload: dict) -> dict:
        """Submit a transaction; returns {seq, result}."""
        body = json.dumps(payload).encode("utf-8")
        return json.loads(self._request("POST", f"/tx/{kind}", body))

    # ------------------------------------------------------------------
    # Write methods (mirror ArcNodeClient; results unwrap to the same
    # shape the CLI prints)
    # ------------------------------------------------------------------

    def submit_genesis(self, genesis: dict) -> dict:
        return self.submit_tx("submit-genesis", genesis)["result"]

    def evaluate_conformance(self, genesis_id: str) -> dict:
        return self.submit_tx("evaluate-conformance", {"id": genesis_id})["result"]

    def record_seed_validation(self, genesis_id: str, record: dict) -> dict:
        payload = {"genesis_id": genesis_id, **record}
        return self.submit_tx("record-seed-validation", payload)["result"]

    def finalize_activation(self, genesis_id: str) -> dict:
        return self.submit_tx("finalize-activation", {"id": genesis_id})["result"]

    def register_validators(self, pool: dict) -> dict:
        return self.submit_tx("register-validators", pool)["result"]

    def submit_block(self, block: dict) -> dict:
        return self.submit_tx("submit-block", block)["result"]

    def assign_validators(self, block_id: str) -> dict:
        return self.submit_tx("assign-validators", {"id": block_id})["result"]

    def submit_attestation(self, attestation: dict) -> dict:
        return self.submit_tx("submit-attestation", attestation)["result"]

    def evaluate_block(self, block_id: str) -> dict:
        return self.submit_tx("evaluate-block", {"id": block_id})["result"]

    def close_challenge_window(self, block_id: str) -> dict:
        return self.submit_tx("close-challenge-window", {"id": block_id})["result"]

    def settle_block(self, block_id: str) -> dict:
        return self.submit_tx("settle-block", {"id": block_id})["result"]

    def finalize_block(self, block_id: str) -> dict:
        return self.submit_tx("finalize-block", {"id": block_id})["result"]

    def open_challenge(self, params: dict) -> dict:
        return self.submit_tx("open-challenge", params)["result"]

    def begin_challenge_review(self, challenge_id: str) -> dict:
        return self.submit_tx("begin-review", {"id": challenge_id})["result"]

    def uphold_challenge(self, challenge_id: str) -> dict:
        return self.submit_tx("uphold-challenge", {"id": challenge_id})["result"]

    def reject_challenge(self, challenge_id: str) -> dict:
        return self.submit_tx("reject-challenge", {"id": challenge_id})["result"]

    def expire_challenge(self, challenge_id: str) -> dict:
        return self.submit_tx("expire-challenge", {"id": challenge_id})["result"]

    def advance_epoch(self) -> dict:
        return self.submit_tx("advance-epoch", {})["result"]

    def top_up_pool(self, domain_id: str, amount: int) -> dict:
        payload = {"domain_id": domain_id, "amount": amount}
        return self.submit_tx("top-up-pool", payload)["result"]

    # ------------------------------------------------------------------
    # Queries
    # ------------------------------------------------------------------

    def status(self) -> dict:
        return self._get("/status")

    def show_block(self, block_id: str) -> dict:
        return self._get(f"/block/{block_id}")

    def show_frontier(self, domain_id: str) -> dict:
        return self._get(f"/frontier/{domain_id}")

    def show_challenge(self, challenge_id: str) -> dict:
        return self._get(f"/challenge/{challenge_id}")

    def list_blocks(self) -> dict:
        return self._get("/blocks")

    def show_pool(self, domain_id: str) -> dict:
        return self._get(f"/pool/{domain_id}")

    # ------------------------------------------------------------------
    # Content-addressed artifacts
    # ------------------------------------------------------------------

    def put_artifact(self, data: bytes) -> str:
        """Store bytes in the sequencer's artifact store; returns the hash."""
        result = json.loads(self._request("POST", "/artifact", data))
        return result["hash"]

    def get_artifact(self, hex_hash: str) -> bytes:
        """Fetch artifact bytes by content hash."""
        return self._request("GET", f"/artifact/{hex_hash}")
