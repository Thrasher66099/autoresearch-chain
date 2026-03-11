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

//! Escrow and attribution types.

use serde::{Deserialize, Serialize};

use crate::enums::{AttributionType, EscrowStatus};
use crate::ids::{BlockId, EpochId, EscrowId, ParticipantId};

/// Temporary holding of rewards pending challenge-window expiration.
///
/// Rewards are not released immediately upon validation. They are held in
/// escrow until the challenge window closes without an upheld challenge.
/// If a challenge is upheld, escrowed funds may be slashed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowRecord {
    /// Unique escrow identifier.
    pub id: EscrowId,
    /// The block whose rewards are escrowed.
    pub block_id: BlockId,
    /// Who will receive the funds if released.
    pub beneficiary: ParticipantId,
    /// Escrowed amount.
    ///
    /// Placeholder: will use a proper token/amount type in a future phase.
    pub amount: u64,
    /// Current escrow status.
    pub status: EscrowStatus,
    /// Epoch at which the escrow was created.
    pub created_epoch: EpochId,
    /// Epoch at which the escrow may be released (challenge window close).
    pub release_epoch: EpochId,
}

/// A claim of credit for a contribution to protocol progress.
///
/// Attribution claims are used in reward distribution to determine how
/// rewards are split among contributors (original discoverer, integrator,
/// frontier advancer).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributionClaim {
    /// Type of attribution being claimed.
    pub attribution_type: AttributionType,
    /// Who is claiming the credit.
    pub claimant: ParticipantId,
    /// The block this attribution relates to.
    pub block_id: BlockId,
}
