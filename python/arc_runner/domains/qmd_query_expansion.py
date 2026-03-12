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
QMD Query Expansion domain — genesis packager.

Takes the ``tobi/qmd/finetune/`` directory and produces a genesis package
with real content-addressed artifact hashes, matching the Rust
``GenesisBlock`` struct fields.
"""

from __future__ import annotations

import io
import json
import tarfile
from pathlib import Path

from arc_runner.evidence import EvidenceBundler, sha256_bytes, sha256_file


class QMDGenesisPackager:
    """Packages the QMD finetune directory into a protocol genesis.

    Parameters
    ----------
    finetune_dir : str or Path
        Path to the ``qmd/finetune/`` directory containing reward.py,
        eval.py, train.py, configs/, data/, evals/, etc.
    store_dir : str or Path
        Path to the content-addressed artifact store directory.
    """

    # Known baseline from SFT model evaluation (92.0% average).
    DEFAULT_SEED_SCORE = 0.920

    # Frozen surface: files that must not be modified by proposers.
    FROZEN_SURFACE = [
        "reward.py",
        "eval.py",
        "evals/queries.txt",
    ]

    # Search surface: files that proposers may modify.
    SEARCH_SURFACE = [
        "train.py",
        "configs/",
    ]

    def __init__(self, finetune_dir: str | Path, store_dir: str | Path) -> None:
        self.finetune_dir = Path(finetune_dir)
        self.store_dir = Path(store_dir)
        self.bundler = EvidenceBundler(self.store_dir)

    def _hash_and_store_file(self, relpath: str) -> str:
        """Hash and store a single file from the finetune directory."""
        full_path = self.finetune_dir / relpath
        if not full_path.exists():
            raise FileNotFoundError(f"Required file not found: {full_path}")
        return self.bundler.hash_file(full_path)

    def _hash_and_store_directory(self, relpath: str) -> str:
        """Hash and store all files in a directory, returning a combined hash.

        Stores each file individually and computes a manifest hash over
        the sorted list of (relative_path, file_hash) pairs.
        """
        dir_path = self.finetune_dir / relpath
        if not dir_path.is_dir():
            raise FileNotFoundError(f"Required directory not found: {dir_path}")

        entries = []
        for path in sorted(dir_path.rglob("*")):
            if path.is_file():
                rel = path.relative_to(self.finetune_dir)
                file_hash = self.bundler.hash_file(path)
                entries.append(f"{rel}:{file_hash}")

        manifest = "\n".join(entries).encode("utf-8")
        return self.bundler.hash_bytes(manifest)

    def _create_seed_tarball(self) -> str:
        """Create a tarball of the entire finetune directory and store it."""
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w:gz") as tar:
            for path in sorted(self.finetune_dir.rglob("*")):
                if path.is_file():
                    arcname = str(path.relative_to(self.finetune_dir.parent))
                    tar.add(path, arcname=arcname)
        tarball_bytes = buf.getvalue()
        return self.bundler.hash_bytes(tarball_bytes)

    def _hash_dataset_files(self) -> dict:
        """Hash all dataset files in data/ and return split hashes."""
        data_dir = self.finetune_dir / "data"
        if not data_dir.is_dir():
            raise FileNotFoundError(f"Dataset directory not found: {data_dir}")

        # Hash each JSONL file.
        file_hashes = []
        for path in sorted(data_dir.glob("*.jsonl")):
            h = self.bundler.hash_file(path)
            file_hashes.append(f"{path.name}:{h}")

        # Overall dataset hash from manifest.
        manifest = "\n".join(file_hashes).encode("utf-8")
        dataset_hash = self.bundler.hash_bytes(manifest)

        # For simplicity, all data files are treated as training data.
        # The dataset_splits reference the same hash since QMD doesn't
        # have explicit train/val/test partition files — the eval harness
        # uses a separate queries.txt.
        return {
            "canonical_dataset_ref": dataset_hash,
            "dataset_hash": dataset_hash,
            "dataset_splits": {
                "training": dataset_hash,
                "validation": dataset_hash,
                "test": None,
            },
        }

    def package(self) -> dict:
        """Produce a genesis package with real artifact hashes.

        Returns a dict whose keys match the Rust ``GenesisBlock`` struct
        field names, suitable for JSON serialization and consumption by
        the Rust protocol side.
        """
        # Frozen surface artifacts.
        reward_hash = self._hash_and_store_file("reward.py")
        eval_hash = self._hash_and_store_file("eval.py")
        queries_hash = self._hash_and_store_file("evals/queries.txt")

        # Search surface artifacts.
        train_hash = self._hash_and_store_file("train.py")
        configs_hash = self._hash_and_store_directory("configs")

        # Evaluation harness = eval.py + reward.py + queries.txt combined.
        eval_manifest = "\n".join([
            f"reward.py:{reward_hash}",
            f"eval.py:{eval_hash}",
            f"evals/queries.txt:{queries_hash}",
        ]).encode("utf-8")
        evaluation_harness_hash = self.bundler.hash_bytes(eval_manifest)

        # Seed recipe = train.py + configs/.
        recipe_manifest = "\n".join([
            f"train.py:{train_hash}",
            f"configs/:{configs_hash}",
        ]).encode("utf-8")
        seed_recipe_hash = self.bundler.hash_bytes(recipe_manifest)

        # Seed codebase state (tarball of entire finetune dir).
        seed_codebase_hash = self._create_seed_tarball()

        # Dataset hashes.
        dataset_info = self._hash_dataset_files()

        # Environment manifest (capture current environment).
        env = self.bundler.capture_environment()
        env_bytes = json.dumps(env, sort_keys=True, indent=2).encode("utf-8")
        env_manifest_hash = self.bundler.hash_bytes(env_bytes)

        # Artifact schema (placeholder — describes expected evidence format).
        schema = json.dumps({
            "format": "arc-evidence-v1",
            "required_fields": [
                "diff_ref", "config_ref", "training_log_ref",
                "metric_output_ref", "environment_manifest_ref",
            ],
        }, sort_keys=True).encode("utf-8")
        artifact_schema_hash = self.bundler.hash_bytes(schema)

        # Assemble the genesis package.
        genesis = {
            "rts_version": "Rts1",
            "proposer": "0" * 64,  # Placeholder — real proposer ID comes from protocol.
            "research_target_declaration": (
                "Improve QMD query expansion quality via SFT/GRPO training "
                "recipe optimization on Qwen3-1.7B"
            ),
            "domain_intent": "EndToEndRecipeImprovement",

            # Codebase and evaluation.
            "seed_recipe_ref": seed_recipe_hash,
            "seed_codebase_state_ref": seed_codebase_hash,
            "frozen_surface": self.FROZEN_SURFACE,
            "search_surface": self.SEARCH_SURFACE,

            # Dataset.
            **dataset_info,

            # Evaluation.
            "evaluation_harness_ref": evaluation_harness_hash,
            "metric_id": "reward_score",
            "metric_direction": "HigherBetter",

            # Environment and budget.
            "hardware_class": "A10G",
            "time_budget_secs": 2700,  # 45 minutes.
            "seed_environment_manifest_ref": env_manifest_hash,

            # Baseline.
            "seed_score": self.DEFAULT_SEED_SCORE,

            # Artifacts and economics.
            "artifact_schema_ref": artifact_schema_hash,
            "seed_bond": 1000,
            "license_declaration": "Apache-2.0",

            # Metadata.
            "timestamp": 0,  # Placeholder — set at submission time.

            # Extra: individual frozen surface hashes for verification.
            "_frozen_surface_hashes": {
                "reward.py": reward_hash,
                "eval.py": eval_hash,
                "evals/queries.txt": queries_hash,
            },
            "_search_surface_hashes": {
                "train.py": train_hash,
                "configs/": configs_hash,
            },
        }

        return genesis
