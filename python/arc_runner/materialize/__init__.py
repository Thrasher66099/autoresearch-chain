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
Materialized code state generation and resolution.

A materialized state is a full assembled snapshot of a domain's codebase,
represented as a content-addressed **state manifest**: a canonical JSON
object mapping relative file paths to the BLAKE3 hashes of their contents,
with every file stored individually in the artifact store. The manifest's
own hash is the state reference (`child_state_ref` on blocks,
`seed_codebase_state_ref` on genesis).

Block diffs are **structured state diffs**, not textual patches: a canonical
JSON object recording the parent state reference, the changed/added files
(path -> content hash), and the deleted paths. Structured diffs are chosen
over unified-diff patching deliberately — application is deterministic
(no context matching, no fuzz), verification is exact (the reconstructed
manifest must hash to the declared child state), and unchanged file
contents deduplicate across generations in the content store. Textual
diffs remain in evidence bundles for human review; they are not the
protocol's state-transfer format.

Verification model: materialization is not trusted assembly. After writing
files, the workspace is re-snapshotted and the resulting manifest hash must
equal the requested state reference. A mismatch is a protocol violation
(`MaterializationError`), never a warning.
"""

from __future__ import annotations

import fnmatch
import json
from pathlib import Path

from arc_runner.evidence import EvidenceBundler, blake3_bytes

STATE_FORMAT = "arc-materialized-state-v1"
DIFF_FORMAT = "arc-state-diff-v1"

# Run outputs and scratch files that are not part of the codebase state.
DEFAULT_EXCLUDES = [
    "__pycache__",
    "*.pyc",
    ".git",
    "_arc_diff.patch",
    "training.log",
    "metrics.json",
]


class MaterializationError(Exception):
    """A materialized state failed verification against its reference."""


def _is_excluded(relpath: Path, excludes: list[str]) -> bool:
    """Check whether any path component matches an exclude pattern."""
    return any(
        fnmatch.fnmatch(part, pattern)
        for part in relpath.parts
        for pattern in excludes
    )


def _canonical_json(obj: dict) -> bytes:
    """Serialize deterministically: sorted keys, no whitespace."""
    return json.dumps(obj, sort_keys=True, separators=(",", ":")).encode("utf-8")


def snapshot_workspace(
    codebase_root: str | Path,
    bundler: EvidenceBundler,
    *,
    root_dir_name: str | None = None,
    excludes: list[str] | None = None,
) -> str:
    """Snapshot a codebase directory into a content-addressed state manifest.

    Every file under ``codebase_root`` (minus excludes) is stored in the
    artifact store; the manifest maps sorted relative paths to content
    hashes. Returns the state reference (the manifest's own hash).

    ``root_dir_name`` records the directory name the state should be
    assembled under (e.g. ``finetune``), preserving workspace layout.
    """
    root = Path(codebase_root)
    if not root.is_dir():
        raise FileNotFoundError(f"Codebase root not found: {root}")
    excludes = DEFAULT_EXCLUDES if excludes is None else excludes

    files: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(root)
        if _is_excluded(rel, excludes):
            continue
        files[rel.as_posix()] = bundler.hash_file(path)

    manifest = {
        "format": STATE_FORMAT,
        "root_dir": root_dir_name if root_dir_name is not None else root.name,
        "files": files,
    }
    return bundler.hash_bytes(_canonical_json(manifest))


def load_state_manifest(state_ref: str, bundler: EvidenceBundler) -> dict:
    """Fetch and validate a state manifest from the artifact store."""
    raw = bundler.fetch(state_ref)
    if raw is None:
        raise FileNotFoundError(f"State manifest not in store: {state_ref}")
    manifest = json.loads(raw)
    if manifest.get("format") != STATE_FORMAT:
        raise MaterializationError(
            f"Artifact {state_ref} is not a state manifest "
            f"(format: {manifest.get('format')!r})"
        )
    return manifest


def materialize_state(
    state_ref: str,
    bundler: EvidenceBundler,
    target_dir: str | Path,
) -> Path:
    """Assemble a full workspace from a state reference and verify it.

    Files are written under ``target_dir/<root_dir>/``. Each file's bytes
    are fetched by content hash and verified on write; the assembled tree
    is then re-snapshotted and must hash back to ``state_ref``. Returns
    the codebase root path (``target_dir/<root_dir>``).

    Raises ``MaterializationError`` on any verification failure — an
    unverifiable materialization is a protocol violation, not a warning.
    """
    manifest = load_state_manifest(state_ref, bundler)
    target = Path(target_dir)
    root = target / manifest["root_dir"]
    root.mkdir(parents=True, exist_ok=True)

    for relpath, file_hash in manifest["files"].items():
        data = bundler.fetch(file_hash)
        if data is None:
            raise MaterializationError(
                f"State {state_ref}: file artifact missing from store: "
                f"{relpath} ({file_hash})"
            )
        if blake3_bytes(data) != file_hash:
            raise MaterializationError(
                f"State {state_ref}: store corruption for {relpath}: "
                f"content does not match hash {file_hash}"
            )
        dest = root / relpath
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)

    # Round-trip verification: the assembled tree must reproduce the
    # exact state reference.
    rebuilt = snapshot_workspace(
        root, bundler, root_dir_name=manifest["root_dir"]
    )
    if rebuilt != state_ref:
        raise MaterializationError(
            f"Materialized workspace does not verify against {state_ref} "
            f"(rebuilt: {rebuilt})"
        )
    return root


def compute_state_diff(
    parent_state_ref: str,
    child_state_ref: str,
    bundler: EvidenceBundler,
) -> str:
    """Compute and store the structured diff between two states.

    Returns the diff reference (hash of the stored diff object). The diff
    records changed/added files (path -> content hash) and deleted paths,
    plus both state references so application is self-verifying.
    """
    parent = load_state_manifest(parent_state_ref, bundler)["files"]
    child = load_state_manifest(child_state_ref, bundler)["files"]

    changed = {
        path: h for path, h in child.items() if parent.get(path) != h
    }
    deleted = sorted(path for path in parent if path not in child)

    diff = {
        "format": DIFF_FORMAT,
        "parent_state_ref": parent_state_ref,
        "child_state_ref": child_state_ref,
        "changed": changed,
        "deleted": deleted,
    }
    return bundler.hash_bytes(_canonical_json(diff))


def load_state_diff(diff_ref: str, bundler: EvidenceBundler) -> dict:
    """Fetch and validate a structured state diff from the artifact store."""
    raw = bundler.fetch(diff_ref)
    if raw is None:
        raise FileNotFoundError(f"State diff not in store: {diff_ref}")
    diff = json.loads(raw)
    if diff.get("format") != DIFF_FORMAT:
        raise MaterializationError(
            f"Artifact {diff_ref} is not a state diff "
            f"(format: {diff.get('format')!r})"
        )
    return diff


def apply_state_diff(diff_ref: str, bundler: EvidenceBundler) -> str:
    """Apply a structured diff to its parent state, producing the child.

    Reconstructs the child manifest from the parent manifest plus the
    diff's changed/deleted sets, stores it, and verifies that its hash
    equals the diff's declared ``child_state_ref``. Returns the verified
    child state reference.
    """
    diff = load_state_diff(diff_ref, bundler)
    parent = load_state_manifest(diff["parent_state_ref"], bundler)

    files = dict(parent["files"])
    for path in diff["deleted"]:
        files.pop(path, None)
    files.update(diff["changed"])

    manifest = {
        "format": STATE_FORMAT,
        "root_dir": parent["root_dir"],
        "files": files,
    }
    rebuilt_ref = bundler.hash_bytes(_canonical_json(manifest))
    if rebuilt_ref != diff["child_state_ref"]:
        raise MaterializationError(
            f"Diff {diff_ref} does not reproduce its declared child state "
            f"{diff['child_state_ref']} (rebuilt: {rebuilt_ref})"
        )
    return rebuilt_ref


def resolve_diff_chain(
    diff_refs: list[str],
    seed_state_ref: str,
    bundler: EvidenceBundler,
) -> str:
    """Resolve a chain of diffs from the seed state to a tip state.

    ``diff_refs`` must be ordered oldest-first (genesis-adjacent block
    first). Each diff's parent must match the state produced so far.
    Returns the verified tip state reference.

    This is the fallback path when a tip's full state manifest is not
    directly available; each step is verified, so a chain that does not
    reproduce its claimed states fails loudly.
    """
    current = seed_state_ref
    for diff_ref in diff_refs:
        diff = load_state_diff(diff_ref, bundler)
        if diff["parent_state_ref"] != current:
            raise MaterializationError(
                f"Diff chain broken at {diff_ref}: expected parent {current}, "
                f"diff declares {diff['parent_state_ref']}"
            )
        current = apply_state_diff(diff_ref, bundler)
    return current
