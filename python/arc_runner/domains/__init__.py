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
Domain-specific experiment wrappers.

Each research track defines a specific problem domain with its own
evaluation logic, dataset, environment requirements, and replay
procedure. This package provides domain-specific wrappers that
handle the concrete execution details.

A domain wrapper translates between the protocol's abstract
experiment interface and the actual training/evaluation machinery
for a specific research arena.
"""

from __future__ import annotations

from arc_runner.client import generate_id


def prepare_genesis(packager_output: dict, proposer_id: str) -> dict:
    """Add protocol identity fields to a domain packager output.

    Generates a content-addressed genesis ID from the seed artifact
    hashes and sets domain_id = genesis_id (protocol convention for
    genesis blocks, matching crates/node/tests/integration.rs).

    Parameters
    ----------
    packager_output : dict
        Raw output from a domain-specific packager (e.g. QMDGenesisPackager).
    proposer_id : str
        64-char hex ID of the proposer submitting this genesis.

    Returns
    -------
    dict
        A copy of packager_output with ``id``, ``domain_id``, and
        ``proposer`` fields set.
    """
    genesis = dict(packager_output)

    genesis_id = generate_id(
        genesis["seed_recipe_ref"].encode("utf-8"),
        genesis["seed_codebase_state_ref"].encode("utf-8"),
        genesis["canonical_dataset_ref"].encode("utf-8"),
        str(genesis["timestamp"]).encode("utf-8"),
    )
    genesis["id"] = genesis_id
    genesis["domain_id"] = genesis_id
    genesis["proposer"] = proposer_id

    return genesis
