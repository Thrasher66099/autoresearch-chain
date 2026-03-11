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

An autoresearch-style loop (similar to Karpathy's `autoresearch`) is a
local agent-driven research loop where an AI agent:

    1. Reads the current training recipe
    2. Modifies the code (train.py or equivalent search surface)
    3. Runs a short bounded training experiment
    4. Measures the result
    5. Keeps improvements, discards failures
    6. Iterates

This adapter bridges such loops with the AutoResearch Chain protocol:

    - Wraps an existing autoresearch-style agent as a protocol proposer
    - Pulls the canonical frontier state as the agent's starting point
    - Constrains the agent to the track's declared search surface
    - Captures the agent's output as a protocol-conformant diff
    - Packages the full evidence bundle for protocol submission
    - Respects frozen surface constraints (evaluation harness immutability)

This is the primary integration point between autonomous research
agents and the decentralized protocol.

Implementation status:
    Not yet implemented. This is a Phase 2 target but the adapter
    interface should be designed early.
"""

# TODO: Define AutoresearchAdapter class or protocol.
# TODO: Define agent loop interface (what the adapter expects from the agent).
# TODO: Define search surface enforcement.
# TODO: Define frozen surface verification.
# TODO: Define frontier pull → agent workspace setup pipeline.
