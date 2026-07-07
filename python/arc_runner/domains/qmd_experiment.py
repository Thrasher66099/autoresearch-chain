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
QMD query expansion experiment execution engine.

Bridges the adapter layer with domain-specific execution. Handles:
  - Locating the finetune codebase within an extracted workspace
  - Running training (heuristic search over strategy space)
  - Running evaluation (deterministic scoring against test queries)
  - Validator replay (re-run evaluation, compare to claimed score)

Used by both the demo script and tests. Encapsulates domain knowledge
shared between proposer and validator without coupling to either.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

from arc_runner.evidence import EvidenceBundler


# Baseline config matching configs/sft.yaml defaults.
BASELINE_CONFIG: dict = {
    "use_think_tags": False,
    "diversity_terms": [],
    "template": "minimal",
    "min_length": 0,
}


def find_codebase_root(workspace: str | Path) -> Path:
    """Locate the finetune directory within an extracted workspace.

    The tarball created by QMDGenesisPackager preserves the parent directory
    (``finetune/``), so the codebase root is usually one level down from
    the workspace. This function handles both cases: direct extraction
    and nested extraction.

    Returns the path containing reward.py, eval.py, train.py, etc.
    """
    workspace = Path(workspace)

    # Direct case: workspace itself contains the expected files.
    if (workspace / "reward.py").exists():
        return workspace

    # Nested case: look for a subdirectory containing reward.py.
    for candidate in workspace.iterdir():
        if candidate.is_dir() and (candidate / "reward.py").exists():
            return candidate

    # Deep search as fallback.
    for reward in workspace.rglob("reward.py"):
        return reward.parent

    raise FileNotFoundError(
        f"Cannot locate finetune codebase root in workspace: {workspace}"
    )


def _load_module(name: str, path: Path):
    """Load a Python module from a file path."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Cannot load {name} from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_evaluation(workspace: str | Path, config: dict) -> dict:
    """Run the evaluation harness with a given config.

    Parameters
    ----------
    workspace : str or Path
        Path to workspace containing the extracted finetune codebase.
    config : dict
        Strategy configuration to evaluate.

    Returns
    -------
    dict
        Evaluation results from eval.evaluate():
        - reward_score: float
        - num_queries: int
        - per_query: list of per-query results
    """
    root = find_codebase_root(workspace)
    eval_module = _load_module("eval", root / "eval.py")
    return eval_module.evaluate(
        config,
        root / "evals" / "queries.txt",
        root / "reward.py",
    )


def run_training(workspace: str | Path) -> dict:
    """Run the training script (heuristic search) in a workspace.

    Executes train.train() which searches over strategy configurations,
    evaluating each against the frozen test queries using the real reward
    function. Writes output files (config.yaml, training.log, metrics.json)
    to the workspace root.

    Parameters
    ----------
    workspace : str or Path
        Path to workspace containing the extracted finetune codebase.
        Output files are written directly to this path.

    Returns
    -------
    dict
        Training metadata from train.train():
        - best_score: float
        - best_config: dict
        - num_trials: int
        - all_trials: list
    """
    workspace = Path(workspace)
    root = find_codebase_root(workspace)
    train_module = _load_module("train", root / "train.py")
    return train_module.train(str(root), str(workspace))


def replay_and_verify(
    workspace: str | Path,
    evidence_manifest: dict,
    bundler: EvidenceBundler,
    claimed_score: float,
    tolerance: float = 1e-4,
) -> dict:
    """Validator replay: re-run evaluation and compare to claimed score.

    The validator:
    1. Extracts the proposer's output config from the stored config_hash artifact
    2. Re-runs eval.evaluate() with that config against the frozen queries
    3. Compares observed score to claimed score
    4. Returns pass/fail verdict

    Parameters
    ----------
    workspace : str or Path
        Path to workspace containing the extracted finetune codebase.
    evidence_manifest : dict
        The evidence bundle manifest (from EvidenceBundle.as_dict()).
    bundler : EvidenceBundler
        Artifact store for fetching evidence artifacts.
    claimed_score : float
        The proposer's claimed reward_score.
    tolerance : float
        Acceptable difference between claimed and observed scores.

    Returns
    -------
    dict
        Replay results:
        - vote: "Pass" or "Fail"
        - observed_score: float
        - claimed_score: float
        - difference: float
        - config_recovered: bool
    """
    # Fetch the proposer's config artifact.
    config_hash = evidence_manifest.get("config_hash")
    if config_hash is None:
        return {
            "vote": "Fail",
            "observed_score": 0.0,
            "claimed_score": claimed_score,
            "difference": claimed_score,
            "config_recovered": False,
        }

    config_bytes = bundler.fetch(config_hash)
    if config_bytes is None:
        return {
            "vote": "Fail",
            "observed_score": 0.0,
            "claimed_score": claimed_score,
            "difference": claimed_score,
            "config_recovered": False,
        }

    # Parse the config. The config.yaml written by train.py is simple
    # key-value format. We also check if it's valid JSON (metrics.json
    # stores best_config as JSON).
    config_text = config_bytes.decode("utf-8")
    config = _parse_config(config_text)

    # Re-run evaluation with recovered config.
    eval_result = run_evaluation(workspace, config)
    observed_score = eval_result["reward_score"]

    difference = abs(observed_score - claimed_score)
    vote = "Pass" if difference <= tolerance else "Fail"

    return {
        "vote": vote,
        "observed_score": observed_score,
        "claimed_score": claimed_score,
        "difference": difference,
        "config_recovered": True,
    }


def _parse_config(config_text: str) -> dict:
    """Parse a config from YAML-like key-value format or JSON.

    Handles the simple YAML format written by train.py:
        key: value
        list_key:
          - item1
          - item2
    """
    # Try JSON first (e.g., from metrics.json best_config).
    try:
        return json.loads(config_text)
    except (json.JSONDecodeError, ValueError):
        pass

    # Parse simple YAML-like format.
    config: dict = {}
    lines = config_text.strip().splitlines()
    current_key = None
    current_list: list | None = None

    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if stripped.startswith("- "):
            # List item.
            if current_key is not None and current_list is not None:
                value = stripped[2:].strip()
                current_list.append(_coerce_value(value))
            continue

        if ":" in stripped:
            # Flush previous list if any.
            if current_key is not None and current_list is not None:
                config[current_key] = current_list

            key, _, value = stripped.partition(":")
            key = key.strip()
            value = value.strip()

            if not value:
                # Start of a list.
                current_key = key
                current_list = []
            else:
                current_key = None
                current_list = None
                config[key] = _coerce_value(value)

    # Flush final list.
    if current_key is not None and current_list is not None:
        config[current_key] = current_list

    return config


def _coerce_value(value: str):
    """Coerce a string value to its Python type."""
    if value.lower() == "true":
        return True
    if value.lower() == "false":
        return False
    if value == "[]":
        return []
    try:
        return int(value)
    except ValueError:
        pass
    try:
        return float(value)
    except ValueError:
        pass
    return value
