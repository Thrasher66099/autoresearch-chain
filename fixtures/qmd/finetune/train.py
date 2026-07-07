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
QMD query expansion training script.

Search surface: proposers may modify this file.

Performs heuristic search over expansion strategy configurations.
Evaluates each candidate against test queries using the real reward
function. No ML dependencies required — this is pure strategy search.

The search space covers:
  - use_think_tags: True/False
  - diversity_terms: 3 vocabulary sets
  - template: minimal/standard/detailed
  - min_length: 0/60

Total: 2 * 3 * 3 * 2 = 36 candidate configurations.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def _build_search_space() -> list[dict]:
    """Build the grid of strategy configurations to evaluate."""
    think_tag_options = [False, True]

    diversity_options = [
        [],
        ["context", "background", "overview", "synthesis"],
        [
            "reference", "exploration", "insight", "perspective",
            "documentation", "framework", "interpretation", "summary",
            "synthesis", "evaluation",
        ],
    ]

    template_options = ["minimal", "standard", "detailed"]
    min_length_options = [0, 60]

    configs = []
    for think in think_tag_options:
        for diversity in diversity_options:
            for template in template_options:
                for min_len in min_length_options:
                    configs.append({
                        "use_think_tags": think,
                        "diversity_terms": diversity,
                        "template": template,
                        "min_length": min_len,
                    })

    return configs


def train(finetune_dir: str | Path, output_dir: str | Path) -> dict:
    """Run heuristic search over strategy configurations.

    Parameters
    ----------
    finetune_dir : str or Path
        Path to the finetune directory containing eval.py, reward.py,
        evals/queries.txt, and configs/sft.yaml.
    output_dir : str or Path
        Path to write output files: config.yaml, training.log, metrics.json.

    Returns
    -------
    dict
        Training metadata with keys:
        - best_score: float
        - best_config: dict
        - num_trials: int
        - all_trials: list of (score, config) pairs
    """
    finetune_dir = Path(finetune_dir)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Import eval module from the finetune directory.
    sys.path.insert(0, str(finetune_dir))
    try:
        # Use importlib to avoid module caching issues.
        import importlib.util
        spec = importlib.util.spec_from_file_location(
            "eval", finetune_dir / "eval.py"
        )
        if spec is None or spec.loader is None:
            raise ImportError(f"Cannot load eval.py from {finetune_dir}")
        eval_module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(eval_module)
    finally:
        sys.path.pop(0)

    reward_path = finetune_dir / "reward.py"
    queries_path = finetune_dir / "evals" / "queries.txt"

    # Build search space.
    search_space = _build_search_space()

    # Evaluate each configuration.
    log_lines = []
    all_trials = []
    best_score = -1.0
    best_config = None

    for i, config in enumerate(search_space):
        result = eval_module.evaluate(config, queries_path, reward_path)
        score = result["reward_score"]
        all_trials.append({"score": score, "config": config})

        tag = "+" if config.get("use_think_tags") else "-"
        div = len(config.get("diversity_terms", []))
        tmpl = config.get("template", "?")
        log_lines.append(
            f"trial {i + 1:3d}: score={score:.4f}  "
            f"tags={tag} div={div} tmpl={tmpl:8s} "
            f"minlen={config.get('min_length', 0)}"
        )

        if score > best_score:
            best_score = score
            best_config = config

    # Write output files.
    # training.log
    (output_dir / "training.log").write_text("\n".join(log_lines) + "\n")

    # config.yaml (simple key-value format, no yaml dependency needed)
    config_lines = []
    if best_config:
        for key, value in sorted(best_config.items()):
            if isinstance(value, list):
                config_lines.append(f"{key}:")
                for item in value:
                    config_lines.append(f"  - {item}")
            else:
                config_lines.append(f"{key}: {value}")
    (output_dir / "config.yaml").write_text("\n".join(config_lines) + "\n")

    # metrics.json
    metrics = {
        "reward_score": best_score,
        "num_trials": len(all_trials),
        "best_config": best_config,
    }
    (output_dir / "metrics.json").write_text(
        json.dumps(metrics, indent=2, sort_keys=True) + "\n"
    )

    return {
        "best_score": best_score,
        "best_config": best_config,
        "num_trials": len(all_trials),
        "all_trials": all_trials,
    }


# Allow direct execution: python train.py <finetune_dir> <output_dir>
if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <finetune_dir> <output_dir>", file=sys.stderr)
        sys.exit(1)
    result = train(sys.argv[1], sys.argv[2])
    print(f"Best score: {result['best_score']:.4f} ({result['num_trials']} trials)")
