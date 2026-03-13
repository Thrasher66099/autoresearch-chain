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
Evidence bundle creation and validation.

Provides content-addressed artifact storage and evidence bundle assembly
using BLAKE3 hashing (matching the Rust storage-model crate).
"""

from __future__ import annotations

import blake3
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path


def blake3_file(path: str | Path) -> str:
    """Compute the BLAKE3 hex digest of a file's contents."""
    h = blake3.blake3()
    with open(path, "rb") as f:
        while True:
            chunk = f.read(65536)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def blake3_bytes(data: bytes) -> str:
    """Compute the BLAKE3 hex digest of raw bytes."""
    return blake3.blake3(data).hexdigest()


@dataclass
class EvidenceBundle:
    """A collection of content-addressed artifacts for a protocol block."""

    diff_hash: str
    config_hash: str
    env_manifest_hash: str
    training_log_hash: str
    metric_output_hash: str

    def as_dict(self) -> dict:
        """Return the bundle as a plain dict (for JSON serialization)."""
        return {
            "diff_hash": self.diff_hash,
            "config_hash": self.config_hash,
            "env_manifest_hash": self.env_manifest_hash,
            "training_log_hash": self.training_log_hash,
            "metric_output_hash": self.metric_output_hash,
        }

    def all_hashes(self) -> list[str]:
        """Return all artifact hashes as a list."""
        return [
            self.diff_hash,
            self.config_hash,
            self.env_manifest_hash,
            self.training_log_hash,
            self.metric_output_hash,
        ]

    def is_complete(self) -> bool:
        """Check that all hashes are non-empty 64-char hex strings."""
        return all(len(h) == 64 for h in self.all_hashes())


class EvidenceBundler:
    """Creates evidence bundles by hashing and storing artifacts.

    Stores files in a content-addressed directory: ``store_dir/<hex-hash>``.
    Uses BLAKE3 to match the Rust ``arc-storage-model`` crate.
    """

    def __init__(self, store_dir: str | Path) -> None:
        self.store_dir = Path(store_dir)
        self.store_dir.mkdir(parents=True, exist_ok=True)

    def hash_file(self, path: str | Path) -> str:
        """Hash a file and store it. Returns the hex digest."""
        hex_hash = blake3_file(path)
        dest = self.store_dir / hex_hash
        if not dest.exists():
            shutil.copy2(path, dest)
        return hex_hash

    def hash_bytes(self, data: bytes) -> str:
        """Hash raw bytes and store them. Returns the hex digest."""
        hex_hash = blake3_bytes(data)
        dest = self.store_dir / hex_hash
        if not dest.exists():
            dest.write_bytes(data)
        return hex_hash

    def fetch(self, hex_hash: str) -> bytes | None:
        """Retrieve stored artifact bytes by hash. Returns None if missing."""
        path = self.store_dir / hex_hash
        if path.exists():
            return path.read_bytes()
        return None

    def exists(self, hex_hash: str) -> bool:
        """Check if an artifact exists in the store."""
        return (self.store_dir / hex_hash).exists()

    def capture_environment(self) -> dict:
        """Collect the current execution environment manifest.

        Returns a dict with Python version, and optionally PyTorch/CUDA info
        and pip freeze output. Non-GPU environments gracefully degrade.
        """
        env: dict = {
            "python_version": sys.version,
            "platform": sys.platform,
        }

        # PyTorch info (optional — may not be installed).
        try:
            import torch
            env["torch_version"] = torch.__version__
            env["cuda_available"] = torch.cuda.is_available()
            if torch.cuda.is_available():
                env["cuda_version"] = torch.version.cuda or "unknown"
                env["gpu_model"] = torch.cuda.get_device_name(0)
            else:
                env["cuda_version"] = None
                env["gpu_model"] = None
        except ImportError:
            env["torch_version"] = None
            env["cuda_available"] = False
            env["cuda_version"] = None
            env["gpu_model"] = None

        # pip freeze (best-effort).
        try:
            result = subprocess.run(
                [sys.executable, "-m", "pip", "freeze", "--local"],
                capture_output=True,
                text=True,
                timeout=30,
            )
            env["pip_freeze"] = result.stdout.strip()
        except (subprocess.TimeoutExpired, FileNotFoundError):
            env["pip_freeze"] = ""

        return env

    def bundle(
        self,
        diff_path: str | Path,
        config_path: str | Path,
        log_path: str | Path,
        metrics_path: str | Path,
    ) -> EvidenceBundle:
        """Create a complete evidence bundle from experiment artifacts.

        Hashes and stores each file, captures the environment, and returns
        an EvidenceBundle with all hashes populated.
        """
        import json

        diff_hash = self.hash_file(diff_path)
        config_hash = self.hash_file(config_path)
        training_log_hash = self.hash_file(log_path)
        metric_output_hash = self.hash_file(metrics_path)

        # Capture and store environment manifest.
        env = self.capture_environment()
        env_bytes = json.dumps(env, sort_keys=True, indent=2).encode("utf-8")
        env_manifest_hash = self.hash_bytes(env_bytes)

        return EvidenceBundle(
            diff_hash=diff_hash,
            config_hash=config_hash,
            env_manifest_hash=env_manifest_hash,
            training_log_hash=training_log_hash,
            metric_output_hash=metric_output_hash,
        )
