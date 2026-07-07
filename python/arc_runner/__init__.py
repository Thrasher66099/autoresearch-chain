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
arc_runner — Off-chain research execution runners for AutoResearch Chain.

This package contains the Python-side useful-work execution layer.
It is protocol-coupled but runs off-chain. The Rust protocol core
(crates/) is the authority on state transitions; this package executes
the actual research work and packages results for protocol submission.

Subpackages:
    proposer            — Proposer execution runner
    validator           — Validator replay runner
    challenger          — Challenger replay/audit runner
    autoresearch_adapter — Integration with autoresearch-style agent loops
    domains             — Domain-specific experiment wrappers
    evidence            — Evidence bundle creation and validation
    materialize         — Materialized code state generation and packaging

Implementation status:
    client              — Implemented (wraps arc-node CLI via subprocess)
    proposer            — Implemented (frontier query, block submission)
    validator           — Implemented (block discovery, attestation submission)
    evidence            — Implemented (BLAKE3 hashing, content-addressed store)
    autoresearch_adapter — Implemented (frontier pull, surface enforcement)
    domains             — Partial (QMD genesis packager implemented)
    challenger          — Stub only
    materialize         — Stub only
"""
