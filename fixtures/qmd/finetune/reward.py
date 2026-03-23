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
QMD query expansion reward function.

Frozen surface: this file must not be modified by proposers.
The reward function defines the evaluation criteria for query expansions.

Scoring criteria (max 100 raw points, normalized to 0.0-1.0 via /140):
  - Format tags: <think>...</think> present -> 30 pts
  - Diversity: unique terms * 1.0, capped at 30 pts
  - Length: expansion > 50 chars -> 20 pts
  - Quality baseline: always awarded -> 20 pts
"""

from __future__ import annotations


def compute_reward(query: str, expansion: str) -> float:
    """Score a query expansion on format, diversity, length, and quality.

    Parameters
    ----------
    query : str
        The original search query.
    expansion : str
        The generated query expansion.

    Returns
    -------
    float
        Normalized score in [0.0, 1.0].
    """
    score = 0.0

    # Format check: think tags present.
    if "<think>" in expansion and "</think>" in expansion:
        score += 30.0

    # Diversity: count unique terms.
    terms = set(expansion.lower().split())
    score += min(30.0, len(terms) * 1.0)

    # Length: expansion is substantive.
    if len(expansion) > 50:
        score += 20.0

    # Quality baseline.
    score += 20.0

    return min(score / 140.0, 1.0)
