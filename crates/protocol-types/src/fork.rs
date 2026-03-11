// Copyright (C) 2026 AutoResearch Chain contributors
//
// This file is part of AutoResearch Chain.
//
// AutoResearch Chain is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AutoResearch Chain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Fork family types.

use serde::{Deserialize, Serialize};

use crate::ids::{BlockId, DomainId, ForkFamilyId, TrackTreeId};

/// A set of competing branches within a domain that share a common ancestor.
///
/// Fork families are first-class protocol objects, not failure states. They
/// represent parallel exploration of different research directions. The
/// protocol evaluates dominance based on validated metric evidence and settles
/// the canonical frontier when sufficient confidence is reached.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkFamily {
    /// Unique fork family identifier.
    pub id: ForkFamilyId,
    /// The domain this fork family belongs to.
    pub domain_id: DomainId,
    /// The track tree containing this fork family.
    pub track_tree_id: TrackTreeId,
    /// The block where this family's branches diverge.
    pub common_ancestor_id: BlockId,
    /// Current tip blocks of all competing branches.
    pub branch_tips: Vec<BlockId>,
    /// The dominant branch tip, if dominance has been established.
    pub dominant_branch_tip: Option<BlockId>,
}
