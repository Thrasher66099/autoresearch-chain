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

"""Unit tests for materialized state snapshots and structured diffs."""

from pathlib import Path

import pytest

from arc_runner.evidence import EvidenceBundler
from arc_runner.materialize import (
    MaterializationError,
    apply_state_diff,
    compute_state_diff,
    load_state_diff,
    load_state_manifest,
    materialize_state,
    resolve_diff_chain,
    snapshot_workspace,
)


@pytest.fixture
def bundler(tmp_path: Path) -> EvidenceBundler:
    return EvidenceBundler(tmp_path / "store")


def make_codebase(base: Path, files: dict[str, str]) -> Path:
    """Create a codebase directory from a {relpath: content} mapping."""
    for relpath, content in files.items():
        dest = base / relpath
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(content)
    return base


BASE_FILES = {
    "train.py": "print('train v1')\n",
    "eval.py": "print('eval')\n",
    "configs/sft.yaml": "lr: 0.001\n",
    "data/corpus.jsonl": '{"q": "one"}\n',
}


class TestSnapshot:
    def test_snapshot_is_deterministic(self, tmp_path: Path, bundler):
        a = make_codebase(tmp_path / "a" / "finetune", BASE_FILES)
        b = make_codebase(tmp_path / "b" / "finetune", BASE_FILES)
        ref_a = snapshot_workspace(a, bundler)
        ref_b = snapshot_workspace(b, bundler)
        assert ref_a == ref_b

    def test_root_dir_name_affects_ref(self, tmp_path: Path, bundler):
        # Same content under different root names is a different state
        # (layout is part of the state), unless normalized explicitly.
        a = make_codebase(tmp_path / "a" / "finetune", BASE_FILES)
        b = make_codebase(tmp_path / "b" / "other", BASE_FILES)
        assert snapshot_workspace(a, bundler) != snapshot_workspace(b, bundler)
        assert snapshot_workspace(
            a, bundler, root_dir_name="x"
        ) == snapshot_workspace(b, bundler, root_dir_name="x")

    def test_excludes_run_outputs(self, tmp_path: Path, bundler):
        clean = make_codebase(tmp_path / "clean" / "finetune", BASE_FILES)
        dirty = make_codebase(
            tmp_path / "dirty" / "finetune",
            {
                **BASE_FILES,
                "training.log": "noise\n",
                "metrics.json": "{}\n",
                "_arc_diff.patch": "diff\n",
                "__pycache__/train.cpython-310.pyc": "bytecode",
            },
        )
        assert snapshot_workspace(clean, bundler) == snapshot_workspace(
            dirty, bundler
        )

    def test_content_change_changes_ref(self, tmp_path: Path, bundler):
        a = make_codebase(tmp_path / "a" / "finetune", BASE_FILES)
        ref_before = snapshot_workspace(a, bundler)
        (a / "train.py").write_text("print('train v2')\n")
        assert snapshot_workspace(a, bundler) != ref_before


class TestMaterialize:
    def test_round_trip(self, tmp_path: Path, bundler):
        src = make_codebase(tmp_path / "src" / "finetune", BASE_FILES)
        ref = snapshot_workspace(src, bundler)

        root = materialize_state(ref, bundler, tmp_path / "out")
        assert root == tmp_path / "out" / "finetune"
        for relpath, content in BASE_FILES.items():
            assert (root / relpath).read_text() == content

        # The materialized tree re-snapshots to the same reference.
        assert snapshot_workspace(root, bundler) == ref

    def test_missing_manifest_raises(self, tmp_path: Path, bundler):
        with pytest.raises(FileNotFoundError):
            materialize_state("ab" * 32, bundler, tmp_path / "out")

    def test_non_manifest_artifact_raises(self, tmp_path: Path, bundler):
        ref = bundler.hash_bytes(b'{"format": "something-else"}')
        with pytest.raises(MaterializationError):
            load_state_manifest(ref, bundler)

    def test_missing_file_artifact_raises(self, tmp_path: Path, bundler):
        src = make_codebase(tmp_path / "src" / "finetune", BASE_FILES)
        ref = snapshot_workspace(src, bundler)

        # Remove one stored file artifact from the store.
        manifest = load_state_manifest(ref, bundler)
        victim_hash = manifest["files"]["train.py"]
        (bundler.store_dir / victim_hash).unlink()

        with pytest.raises(MaterializationError, match="missing from store"):
            materialize_state(ref, bundler, tmp_path / "out")

    def test_store_corruption_detected(self, tmp_path: Path, bundler):
        src = make_codebase(tmp_path / "src" / "finetune", BASE_FILES)
        ref = snapshot_workspace(src, bundler)

        # Corrupt a stored file artifact in place.
        manifest = load_state_manifest(ref, bundler)
        victim_hash = manifest["files"]["train.py"]
        (bundler.store_dir / victim_hash).write_bytes(b"tampered")

        with pytest.raises(MaterializationError, match="corruption"):
            materialize_state(ref, bundler, tmp_path / "out")


class TestStateDiff:
    def test_diff_records_changes_and_deletions(self, tmp_path: Path, bundler):
        parent = make_codebase(tmp_path / "p" / "finetune", BASE_FILES)
        parent_ref = snapshot_workspace(parent, bundler)

        child = make_codebase(
            tmp_path / "c" / "finetune",
            {
                "train.py": "print('train v2')\n",  # changed
                "eval.py": BASE_FILES["eval.py"],  # unchanged
                "configs/sft.yaml": BASE_FILES["configs/sft.yaml"],
                "configs/best.json": '{"lr": 0.01}\n',  # added
                # data/corpus.jsonl deleted
            },
        )
        child_ref = snapshot_workspace(child, bundler)

        diff_ref = compute_state_diff(parent_ref, child_ref, bundler)
        diff = load_state_diff(diff_ref, bundler)

        assert set(diff["changed"]) == {"train.py", "configs/best.json"}
        assert diff["deleted"] == ["data/corpus.jsonl"]
        assert diff["parent_state_ref"] == parent_ref
        assert diff["child_state_ref"] == child_ref

    def test_apply_reproduces_child(self, tmp_path: Path, bundler):
        parent = make_codebase(tmp_path / "p" / "finetune", BASE_FILES)
        parent_ref = snapshot_workspace(parent, bundler)
        child = make_codebase(
            tmp_path / "c" / "finetune",
            {**BASE_FILES, "train.py": "print('train v2')\n"},
        )
        child_ref = snapshot_workspace(child, bundler)

        diff_ref = compute_state_diff(parent_ref, child_ref, bundler)
        assert apply_state_diff(diff_ref, bundler) == child_ref

    def test_resolve_diff_chain_two_generations(self, tmp_path: Path, bundler):
        gen0 = make_codebase(tmp_path / "g0" / "finetune", BASE_FILES)
        gen0_ref = snapshot_workspace(gen0, bundler)

        gen1 = make_codebase(
            tmp_path / "g1" / "finetune",
            {**BASE_FILES, "configs/best.json": '{"lr": 0.01}\n'},
        )
        gen1_ref = snapshot_workspace(gen1, bundler)

        gen2 = make_codebase(
            tmp_path / "g2" / "finetune",
            {
                **BASE_FILES,
                "configs/best.json": '{"lr": 0.02}\n',
                "train.py": "print('train v3')\n",
            },
        )
        gen2_ref = snapshot_workspace(gen2, bundler)

        d1 = compute_state_diff(gen0_ref, gen1_ref, bundler)
        d2 = compute_state_diff(gen1_ref, gen2_ref, bundler)

        assert resolve_diff_chain([d1, d2], gen0_ref, bundler) == gen2_ref

        # A chain applied against the wrong base fails loudly.
        with pytest.raises(MaterializationError, match="chain broken"):
            resolve_diff_chain([d2], gen0_ref, bundler)
