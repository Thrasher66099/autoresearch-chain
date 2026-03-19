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
Protocol client wrapping the arc-node CLI.

Provides a Python interface to the arc-node binary via subprocess calls.
Each method maps 1:1 to an arc-node command. No convenience shortcuts
that combine multiple protocol transitions — the caller must drive each
step explicitly.

This is the correct approach for a local-only runtime. The arc-node CLI
was designed for JSON in/out, one state mutation per invocation. FFI or
HTTP can be added later without changing runner logic.
"""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

import blake3 as _blake3


class ProtocolError(Exception):
    """Raised when arc-node returns a non-zero exit code.

    Attributes:
        stderr: The stderr output from arc-node.
        returncode: The process exit code.
    """

    def __init__(self, stderr: str, returncode: int) -> None:
        self.stderr = stderr
        self.returncode = returncode
        super().__init__(stderr.strip())


def generate_id(*components: bytes) -> str:
    """Generate a 64-char hex ID by BLAKE3-hashing the concatenated components."""
    h = _blake3.blake3()
    for component in components:
        h.update(component)
    return h.hexdigest()


class ArcNodeClient:
    """Python interface to the arc-node CLI.

    Each method maps 1:1 to an arc-node command. No convenience
    shortcuts that combine multiple protocol transitions.
    """

    def __init__(self, state_path: str, node_binary: str = "arc-node") -> None:
        self.state_path = state_path
        self.node_binary = node_binary

    def _run(self, args: list[str]) -> dict:
        """Run an arc-node command and return parsed JSON stdout.

        Raises ProtocolError on non-zero exit code.
        """
        result = subprocess.run(
            [self.node_binary, *args],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise ProtocolError(result.stderr, result.returncode)
        stdout = result.stdout.strip()
        if not stdout:
            return {}
        return json.loads(stdout)

    def _run_with_json_file(self, command: str, data: dict) -> dict:
        """Write data to a temp JSON file, run the command, return result.

        The command receives the temp file path as its argument.
        The temp file is cleaned up after the command completes.
        """
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False
        ) as f:
            json.dump(data, f, indent=2)
            tmp_path = f.name
        try:
            return self._run(
                ["--state", self.state_path, command, tmp_path]
            )
        finally:
            Path(tmp_path).unlink(missing_ok=True)

    # ------------------------------------------------------------------
    # State management
    # ------------------------------------------------------------------

    def init(self) -> None:
        """Create a fresh protocol state file."""
        self._run(["init", self.state_path])

    def inspect(self) -> str:
        """Display basic info about a saved state.

        Note: inspect writes to stderr, not stdout. Returns the
        stderr text directly.
        """
        result = subprocess.run(
            [self.node_binary, "inspect", self.state_path],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise ProtocolError(result.stderr, result.returncode)
        return result.stderr

    # ------------------------------------------------------------------
    # Genesis / domain activation
    # ------------------------------------------------------------------

    def submit_genesis(self, genesis: dict) -> dict:
        """Submit a genesis proposal."""
        return self._run_with_json_file("submit-genesis", genesis)

    def evaluate_conformance(self, genesis_id: str) -> dict:
        """Run RTS conformance check on a genesis proposal."""
        return self._run(
            ["--state", self.state_path, "evaluate-conformance", genesis_id]
        )

    def record_seed_validation(self, genesis_id: str, record: dict) -> dict:
        """Record a seed validation for a genesis proposal."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False
        ) as f:
            json.dump(record, f, indent=2)
            tmp_path = f.name
        try:
            return self._run(
                [
                    "--state", self.state_path,
                    "record-seed-validation", genesis_id, tmp_path,
                ]
            )
        finally:
            Path(tmp_path).unlink(missing_ok=True)

    def finalize_activation(self, genesis_id: str) -> dict:
        """Finalize track activation for a genesis proposal."""
        return self._run(
            ["--state", self.state_path, "finalize-activation", genesis_id]
        )

    def register_validators(self, pool: dict) -> dict:
        """Register a validator pool for a domain."""
        return self._run_with_json_file("register-validators", pool)

    # ------------------------------------------------------------------
    # Block lifecycle
    # ------------------------------------------------------------------

    def submit_block(self, block: dict) -> dict:
        """Submit a block."""
        return self._run_with_json_file("submit-block", block)

    def assign_validators(self, block_id: str) -> dict:
        """Assign validators and begin validation for a block."""
        return self._run(
            ["--state", self.state_path, "assign-validators", block_id]
        )

    def submit_attestation(self, attestation: dict) -> dict:
        """Record a validation attestation."""
        return self._run_with_json_file("submit-attestation", attestation)

    def evaluate_block(self, block_id: str) -> dict:
        """Aggregate attestations and evaluate a block."""
        return self._run(
            ["--state", self.state_path, "evaluate-block", block_id]
        )

    def close_challenge_window(self, block_id: str) -> dict:
        """Close the challenge window for a block."""
        return self._run(
            ["--state", self.state_path, "close-challenge-window", block_id]
        )

    def settle_block(self, block_id: str) -> dict:
        """Settle a block and release escrow."""
        return self._run(
            ["--state", self.state_path, "settle-block", block_id]
        )

    def finalize_block(self, block_id: str) -> dict:
        """Finalize a settled block."""
        return self._run(
            ["--state", self.state_path, "finalize-block", block_id]
        )

    # ------------------------------------------------------------------
    # Challenges
    # ------------------------------------------------------------------

    def open_challenge(self, params: dict) -> dict:
        """Open a challenge against a block."""
        return self._run_with_json_file("open-challenge", params)

    # ------------------------------------------------------------------
    # Epoch
    # ------------------------------------------------------------------

    def advance_epoch(self) -> dict:
        """Advance to the next epoch."""
        return self._run(["--state", self.state_path, "advance-epoch"])

    # ------------------------------------------------------------------
    # Queries (read-only)
    # ------------------------------------------------------------------

    def list_domains(self) -> dict:
        """List all registered domains."""
        return self._run(["--state", self.state_path, "list-domains"])

    def show_block(self, block_id: str) -> dict:
        """Show block details."""
        return self._run(
            ["--state", self.state_path, "show-block", block_id]
        )

    def show_frontier(self, domain_id: str) -> dict:
        """Show canonical frontier and fork families for a domain."""
        return self._run(
            ["--state", self.state_path, "show-frontier", domain_id]
        )

    def show_challenge(self, challenge_id: str) -> dict:
        """Show challenge details."""
        return self._run(
            ["--state", self.state_path, "show-challenge", challenge_id]
        )

    def list_blocks(self, domain_id: str | None = None) -> dict:
        """List blocks, optionally filtered by domain."""
        args = ["--state", self.state_path, "list-blocks"]
        if domain_id is not None:
            args.extend(["--domain", domain_id])
        return self._run(args)
