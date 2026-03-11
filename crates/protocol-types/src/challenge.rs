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

//! Challenge record types.

use serde::{Deserialize, Serialize};

use crate::enums::{ChallengeStatus, ChallengeType};
use crate::ids::{ArtifactHash, BlockId, ChallengeId, EpochId, ParticipantId};

/// A bonded dispute object in the protocol.
///
/// Anyone may challenge a block, attestation, attribution claim, fork
/// dominance declaration, or metric adequacy by posting a bond and evidence.
/// If the challenge is upheld, the target is invalidated and the challenger
/// may receive a reward. If rejected, the challenger loses their bond.
///
/// The `challenger` field uses [`ParticipantId`] because any participant may
/// file a challenge regardless of their primary role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengeRecord {
    /// Unique challenge identifier.
    pub id: ChallengeId,
    /// What kind of dispute this is.
    pub challenge_type: ChallengeType,
    /// The block being challenged.
    pub target_block_id: BlockId,
    /// Who filed this challenge.
    pub challenger: ParticipantId,
    /// Slashable bond posted by the challenger.
    ///
    /// Placeholder: will use a proper token/amount type in a future phase.
    pub bond: u64,
    /// Reference to the challenger's supporting evidence.
    pub evidence_ref: ArtifactHash,
    /// Current challenge lifecycle status.
    pub status: ChallengeStatus,
    /// Epoch when the challenge was filed.
    pub epoch_id: EpochId,
    /// Unix timestamp of challenge creation.
    pub timestamp: u64,
}
