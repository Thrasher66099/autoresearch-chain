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
Integration adapter for autoresearch-style autonomous agent loops.

Bridges an autoresearch-style agent loop with the AutoResearch Chain protocol,
enforcing frozen/search surface constraints and packaging experiment results
as protocol-conformant evidence bundles.
"""

from __future__ import annotations

import io
import json
import subprocess
import tarfile
import tempfile
from pathlib import Path

from arc_runner.evidence import EvidenceBundler, sha256_file


class AutoresearchAdapter:
    """Bridges an autoresearch-style agent loop with the protocol.

    Parameters
    ----------
    domain_config : dict
        The genesis package dict (from QMDGenesisPackager.package() or similar).
        Must contain ``seed_codebase_state_ref``, ``frozen_surface``,
        ``_frozen_surface_hashes``, and ``seed_score``.
    store_dir : str or Path
        Path to the content-addressed artifact store.
    """

    def __init__(self, domain_config: dict, store_dir: str | Path) -> None:
        self.config = domain_config
        self.store_dir = Path(store_dir)
        self.bundler = EvidenceBundler(self.store_dir)

    def pull_frontier(self) -> str:
        """Pull the current canonical codebase into a working directory.

        For now, extracts the genesis seed codebase from the artifact store
        into a temporary directory. In later phases this will pull the actual
        frontier state from the Rust protocol node.

        Returns the path to the working directory.
        """
        seed_hash = self.config["seed_codebase_state_ref"]
        seed_bytes = self.bundler.fetch(seed_hash)
        if seed_bytes is None:
            raise FileNotFoundError(
                f"Seed codebase artifact not found in store: {seed_hash}"
            )

        workspace = tempfile.mkdtemp(prefix="arc_workspace_")

        with tarfile.open(fileobj=io.BytesIO(seed_bytes), mode="r:gz") as tar:
            tar.extractall(path=workspace)

        return workspace

    def enforce_surfaces(self, workspace: str) -> None:
        """Verify frozen surface files match their declared hashes.

        Raises ``ValueError`` if any frozen surface file has been modified
        or is missing.
        """
        frozen_hashes = self.config.get("_frozen_surface_hashes", {})
        if not frozen_hashes:
            raise ValueError(
                "No frozen surface hashes in domain config — cannot enforce"
            )

        workspace_path = Path(workspace)

        for relpath, expected_hash in frozen_hashes.items():
            file_path = workspace_path / relpath
            if not file_path.exists():
                # Try looking one directory down (tarball may include parent dir).
                candidates = list(workspace_path.rglob(relpath))
                if candidates:
                    file_path = candidates[0]
                else:
                    raise ValueError(
                        f"Frozen surface file missing: {relpath}"
                    )

            actual_hash = sha256_file(file_path)
            if actual_hash != expected_hash:
                raise ValueError(
                    f"Frozen surface violation: {relpath} has been modified "
                    f"(expected {expected_hash[:16]}..., got {actual_hash[:16]}...)"
                )

    def capture_result(
        self,
        workspace: str,
        baseline_score: float,
    ) -> dict:
        """After an experiment, capture the diff, logs, metrics, and package as evidence.

        Parameters
        ----------
        workspace : str
            Path to the working directory containing modified code.
        baseline_score : float
            The score to compare against (from genesis seed_score or frontier).

        Returns a dict with:
            - ``diff_path``: path to the generated diff file
            - ``score``: the observed metric score
            - ``delta``: score - baseline_score
            - ``evidence_bundle``: EvidenceBundle if evidence files exist
        """
        workspace_path = Path(workspace)
        result: dict = {
            "workspace": workspace,
            "baseline_score": baseline_score,
        }

        # Generate diff against original (best-effort with git).
        diff_path = workspace_path / "_arc_diff.patch"
        try:
            proc = subprocess.run(
                ["git", "diff", "--no-index", "/dev/null", "."],
                cwd=workspace,
                capture_output=True,
                text=True,
                timeout=30,
            )
            diff_path.write_text(proc.stdout or "# no diff captured\n")
        except (subprocess.TimeoutExpired, FileNotFoundError):
            diff_path.write_text("# diff generation failed\n")

        result["diff_path"] = str(diff_path)

        # Look for standard output files.
        log_path = workspace_path / "training.log"
        metrics_path = workspace_path / "metrics.json"
        config_path = workspace_path / "config.yaml"

        # Read score from metrics if available.
        if metrics_path.exists():
            try:
                metrics = json.loads(metrics_path.read_text())
                score = metrics.get("reward_score", metrics.get("score", 0.0))
                result["score"] = score
                result["delta"] = score - baseline_score
            except (json.JSONDecodeError, KeyError):
                result["score"] = 0.0
                result["delta"] = -baseline_score
        else:
            result["score"] = 0.0
            result["delta"] = -baseline_score

        # Bundle evidence if all files exist.
        if all(p.exists() for p in [diff_path, config_path, log_path, metrics_path]):
            bundle = self.bundler.bundle(
                diff_path=diff_path,
                config_path=config_path,
                log_path=log_path,
                metrics_path=metrics_path,
            )
            result["evidence_bundle"] = bundle
        else:
            result["evidence_bundle"] = None

        return result

    def should_submit(self, result: dict) -> bool:
        """Check if the result improves on the baseline enough to submit.

        Requires a positive delta and a complete evidence bundle.
        """
        if result.get("evidence_bundle") is None:
            return False
        delta = result.get("delta", 0.0)
        return delta > 0.0
