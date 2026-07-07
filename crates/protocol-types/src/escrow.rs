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
use crate::ids::{BlockId, ChallengeId, EpochId, EscrowId, ParticipantId};
use crate::token::TokenAmount;

/// What an escrow record holds and why.
///
/// Different escrow kinds have different lifecycles:
///
/// - `ProposerBond`: the proposer's slashable bond, released at settlement,
///   slashed on an upheld challenge.
/// - `ProvisionalReward`: the immediate reward tranche paid on acceptance
///   (spec: "Provisional reward"). Released as soon as the block is
///   accepted; this is the protocol's accepted fraud exposure and must be
///   covered by the proposer's bond.
/// - `SurvivalReward`: the reward tranche paid for surviving the challenge
///   window (spec: "Survival reward"). Released at settlement, slashed on
///   an upheld challenge.
/// - `ChallengerBond`: the challenger's slashable bond. Released when the
///   challenge is upheld or expires unresolved; slashed when the challenge
///   is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowKind {
    /// Proposer's slashable block bond.
    ProposerBond,
    /// Immediate reward tranche released on acceptance.
    ProvisionalReward,
    /// Reward tranche released after challenge-window survival.
    SurvivalReward,
    /// Challenger's slashable challenge bond.
    ChallengerBond,
    /// Validator's slashable registration bond (slashed when an
    /// attestation challenge against the validator is upheld).
    ValidatorBond,
}

/// Temporary holding of rewards pending challenge-window expiration.
///
/// Rewards are not released immediately upon validation. They are held in
/// escrow until the challenge window closes without an upheld challenge.
/// If a challenge is upheld, escrowed funds may be slashed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowRecord {
    /// Unique escrow identifier.
    pub id: EscrowId,
    /// The block whose rewards are escrowed. For `ChallengerBond` escrows,
    /// this is the challenge's target block.
    pub block_id: BlockId,
    /// What this escrow holds.
    pub kind: EscrowKind,
    /// Who will receive the funds if released.
    pub beneficiary: ParticipantId,
    /// Escrowed amount.
    pub amount: TokenAmount,
    /// Current escrow status.
    pub status: EscrowStatus,
    /// Epoch at which the escrow was created.
    pub created_epoch: EpochId,
    /// Epoch at which the escrow may be released (challenge window close).
    pub release_epoch: EpochId,
}

/// Auditable record of how slashed funds were distributed after an
/// upheld challenge.
///
/// Phase 0 accounting: a configured fraction of the total slashed amount
/// is redirected to the challenger as payout; the residual is burned.
/// No treasury exists yet — recording the burned residual explicitly keeps
/// the accounting auditable and leaves room for later redistribution policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashDistribution {
    /// The upheld challenge that triggered the slash.
    pub challenge_id: ChallengeId,
    /// The invalidated target block whose escrows were slashed.
    pub block_id: BlockId,
    /// Total amount slashed across the block's held escrows.
    pub slashed_amount: TokenAmount,
    /// The challenger receiving the payout.
    pub challenger: ParticipantId,
    /// Amount redirected to the challenger.
    pub challenger_payout: TokenAmount,
    /// Residual recorded as burned.
    pub burned: TokenAmount,
    /// Epoch at which the slash was applied.
    pub epoch: EpochId,
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
