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

from arc_runner.evidence import EvidenceBundler, blake3_file
from arc_runner.materialize import (
    MaterializationError,
    compute_state_diff,
    load_state_manifest,
    materialize_state,
    snapshot_workspace,
)


def _codebase_root(workspace: str | Path) -> Path:
    """Locate the codebase root inside a pulled workspace.

    Materialized workspaces contain a single top-level directory (the
    manifest's ``root_dir``). If the workspace itself holds files directly,
    it is its own root.
    """
    workspace = Path(workspace)
    entries = [p for p in workspace.iterdir() if p.name != "_arc_diff.patch"]
    dirs = [p for p in entries if p.is_dir()]
    if len(dirs) == 1 and all(p.is_dir() for p in entries):
        return dirs[0]
    return workspace


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

    def pull_frontier(self, state_ref: str | None = None) -> str:
        """Pull an assembled codebase state into a working directory.

        Parameters
        ----------
        state_ref : str, optional
            Content-addressed state reference to materialize — typically a
            frontier block's ``child_state_ref``. Defaults to the genesis
            ``seed_codebase_state_ref``.

        The state is materialized through ``arc_runner.materialize`` and
        verified against its reference (a workspace that does not hash
        back to ``state_ref`` raises ``MaterializationError``). Legacy
        gzip-tarball seed artifacts are still extractable but are not
        verifiable.

        Returns the path to the working directory.
        """
        ref = state_ref or self.config["seed_codebase_state_ref"]
        artifact = self.bundler.fetch(ref)
        if artifact is None:
            raise FileNotFoundError(
                f"Codebase state artifact not found in store: {ref}"
            )

        workspace = tempfile.mkdtemp(prefix="arc_workspace_")

        if artifact[:2] == b"\x1f\x8b":
            # Legacy gzip tarball (pre-materialization genesis packages).
            with tarfile.open(fileobj=io.BytesIO(artifact), mode="r:gz") as tar:
                tar.extractall(path=workspace)
        else:
            materialize_state(ref, self.bundler, workspace)

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

            actual_hash = blake3_file(file_path)
            if actual_hash != expected_hash:
                raise ValueError(
                    f"Frozen surface violation: {relpath} has been modified "
                    f"(expected {expected_hash[:16]}..., got {actual_hash[:16]}...)"
                )

    def capture_result(
        self,
        workspace: str,
        baseline_score: float,
        parent_state_ref: str | None = None,
    ) -> dict:
        """After an experiment, capture the diff, logs, metrics, and package as evidence.

        Parameters
        ----------
        workspace : str
            Path to the working directory containing modified code.
        baseline_score : float
            The score to compare against (from genesis seed_score or frontier).
        parent_state_ref : str, optional
            State reference of the parent codebase state this experiment
            started from. Defaults to the genesis
            ``seed_codebase_state_ref``.

        Returns a dict with:
            - ``diff_path``: path to the generated diff file
            - ``score``: the observed metric score
            - ``delta``: score - baseline_score
            - ``evidence_bundle``: EvidenceBundle if evidence files exist
            - ``child_state_ref``: content-addressed state manifest of the
              post-experiment codebase (verifiable materialized state)
            - ``diff_ref``: structured state diff from the parent state to
              the child state
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

        # Snapshot the post-experiment codebase as a verifiable
        # materialized state and record the structured diff from the
        # parent state. These become the block's child_state_ref and
        # diff_ref. The snapshot's root_dir is taken from the parent
        # manifest so state hashing is layout-stable across machines;
        # legacy (tarball) parents cannot be diffed against, so diff_ref
        # is omitted for them.
        parent_ref = parent_state_ref or self.config["seed_codebase_state_ref"]
        result["parent_state_ref"] = parent_ref
        try:
            parent_manifest = load_state_manifest(parent_ref, self.bundler)
        except (FileNotFoundError, MaterializationError, ValueError):
            parent_manifest = None

        root_dir_name = parent_manifest["root_dir"] if parent_manifest else None
        child_state_ref = snapshot_workspace(
            _codebase_root(workspace), self.bundler, root_dir_name=root_dir_name
        )
        result["child_state_ref"] = child_state_ref
        if parent_manifest is not None:
            result["diff_ref"] = compute_state_diff(
                parent_ref, child_state_ref, self.bundler
            )

        return result

    def should_submit(self, result: dict) -> bool:
        """Check if the result improves on the baseline enough to submit.

        Requires a positive delta and a complete evidence bundle.
        """
        if result.get("evidence_bundle") is None:
            return False
        delta = result.get("delta", 0.0)
        return delta > 0.0
