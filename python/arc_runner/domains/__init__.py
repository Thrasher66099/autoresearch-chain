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

Implementation status:
    Not yet implemented. Domain wrappers will be added as
    specific research tracks are created.
"""

# TODO: Define DomainWrapper base class or protocol.
# TODO: Define experiment execution interface.
# TODO: Define metric extraction interface.
