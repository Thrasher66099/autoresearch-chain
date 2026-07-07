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
QMD query expansion evaluation harness.

Frozen surface: this file must not be modified by proposers.
Generates expansions deterministically from a config dict, scores them
against test queries using the reward function.

Determinism guarantee: same config + same query = same expansion = same score.
This is essential for validator replay.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


def _load_reward_module(reward_path: str | Path):
    """Dynamically load the reward module from a file path."""
    reward_path = Path(reward_path)
    spec = importlib.util.spec_from_file_location("reward", reward_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Cannot load reward module from {reward_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def generate_expansion(query: str, config: dict) -> str:
    """Generate a deterministic query expansion from config parameters.

    The expansion is built from template parameters in the config dict.
    This is not ML inference — it is a deterministic template expansion
    that allows the training script to search over strategy space.

    Parameters
    ----------
    query : str
        The original search query.
    config : dict
        Strategy configuration with keys:
        - use_think_tags (bool): wrap expansion in <think>...</think>
        - diversity_terms (list[str]): extra terms to inject for diversity
        - template (str): "minimal", "standard", or "detailed"
        - min_length (int): pad expansion to this minimum length
    """
    parts = []

    # Template determines base expansion structure.
    template = config.get("template", "minimal")
    if template == "detailed":
        parts.append(f"comprehensive analysis of {query}")
        parts.append(f"covering key aspects and related topics for {query}")
    elif template == "standard":
        parts.append(f"expanded search for {query}")
        parts.append(f"including related topics for {query}")
    else:
        parts.append(f"search results for {query} with relevant information and details")

    # Inject diversity terms.
    diversity_terms = config.get("diversity_terms", [])
    if diversity_terms:
        parts.append(" ".join(diversity_terms))

    expansion = " ".join(parts)

    # Pad to minimum length if needed.
    min_length = config.get("min_length", 0)
    if len(expansion) < min_length:
        padding = " additional context and background information"
        while len(expansion) < min_length:
            expansion += padding

    # Wrap in think tags if configured.
    use_think_tags = config.get("use_think_tags", False)
    if use_think_tags:
        expansion = f"<think>analyzing: {query}</think> {expansion}"

    return expansion


def evaluate(config: dict, queries_path: str | Path, reward_path: str | Path) -> dict:
    """Evaluate a strategy config against test queries using the reward function.

    Parameters
    ----------
    config : dict
        Strategy configuration (passed to generate_expansion).
    queries_path : str or Path
        Path to the queries.txt file (one query per line).
    reward_path : str or Path
        Path to the reward.py module.

    Returns
    -------
    dict
        Evaluation results with keys:
        - reward_score: float, average score across all queries
        - num_queries: int
        - per_query: list of dicts with query, expansion, score
    """
    reward_module = _load_reward_module(reward_path)
    queries_path = Path(queries_path)

    queries = [
        line.strip()
        for line in queries_path.read_text().splitlines()
        if line.strip()
    ]

    per_query = []
    for query in queries:
        expansion = generate_expansion(query, config)
        score = reward_module.compute_reward(query, expansion)
        per_query.append({
            "query": query,
            "expansion": expansion,
            "score": score,
        })

    avg_score = sum(r["score"] for r in per_query) / len(per_query) if per_query else 0.0

    return {
        "reward_score": round(avg_score, 6),
        "num_queries": len(queries),
        "per_query": per_query,
    }
