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

//! Escrow state machine and reward hooks for AutoResearch Chain.
//!
//! # Phase 0 implementation (Milestone A)
//!
//! Escrow operations and challenge economics:
//!
//! - Proposer bond escrow when blocks are provisionally accepted
//! - Staged reward tranches: provisional (released on acceptance) and
//!   survival (released at settlement after the challenge window)
//! - Challenger bond escrow with payout/forfeiture on adjudication
//! - Slash distribution: a configured fraction of slashed funds is
//!   redirected to the successful challenger, the residual is burned
//!
//! Integration, frontier, and transfer reward stages, attribution-weighted
//! distribution, and ancestry farming prevention are deferred to later
//! phases. The interfaces here are designed to support those extensions
//! without structural rewrites.
//!
//! # Fraud-exposure invariant
//!
//! The provisional tranche is paid at acceptance and cannot be clawed back
//! once released. For fraud to be net-negative even when caught, the
//! proposer's bond must exceed the provisional reward amount. The simulator
//! enforces `block.bond >= provisional_reward_amount()` at acceptance.

use serde::{Serialize, Deserialize};

use arc_protocol_types::{
    BlockId, EpochId, EscrowId, EscrowKind, EscrowRecord, EscrowStatus, ParticipantId,
    TokenAmount,
};

/// Basis points denominator (100% = 10_000 bps).
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Multiply an amount by a basis-points fraction with u128 intermediate
/// arithmetic (no overflow for any u64 amount), truncating downward.
fn apply_bps(amount: u64, bps: u64) -> u64 {
    (amount as u128 * bps as u128 / BPS_DENOMINATOR as u128) as u64
}

/// Configuration for reward-engine policy.
///
/// All economic parameters are configuration, not policy constants —
/// Phase 4 adversarial simulation calibrates them.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardConfig {
    /// Number of epochs the challenge window lasts.
    /// Escrow release epoch = created_epoch + challenge_window_epochs.
    pub challenge_window_epochs: u64,
    /// Total block reward escrowed for an accepted block, split into
    /// provisional and survival tranches.
    pub base_block_reward: u64,
    /// Fraction of `base_block_reward` paid as the provisional tranche,
    /// in basis points (e.g. 2_000 = 20%). The remainder is the survival
    /// tranche.
    pub provisional_reward_bps: u64,
    /// Fraction of the total slashed amount redirected to the successful
    /// challenger, in basis points (e.g. 5_000 = 50%). The residual is
    /// burned.
    pub challenger_payout_bps: u64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            challenge_window_epochs: 5,
            base_block_reward: 1_000,
            provisional_reward_bps: 2_000,
            challenger_payout_bps: 5_000,
        }
    }
}

impl RewardConfig {
    /// The provisional tranche amount implied by this config.
    pub fn provisional_reward_amount(&self) -> TokenAmount {
        TokenAmount::new(apply_bps(self.base_block_reward, self.provisional_reward_bps))
    }

    /// The survival tranche amount implied by this config.
    pub fn survival_reward_amount(&self) -> TokenAmount {
        TokenAmount::new(
            self.base_block_reward - apply_bps(self.base_block_reward, self.provisional_reward_bps),
        )
    }

    /// Validate internal consistency of the config.
    pub fn validate(&self) -> Result<(), String> {
        if self.provisional_reward_bps > BPS_DENOMINATOR {
            return Err(format!(
                "provisional_reward_bps {} exceeds {} (100%)",
                self.provisional_reward_bps, BPS_DENOMINATOR
            ));
        }
        if self.challenger_payout_bps > BPS_DENOMINATOR {
            return Err(format!(
                "challenger_payout_bps {} exceeds {} (100%)",
                self.challenger_payout_bps, BPS_DENOMINATOR
            ));
        }
        Ok(())
    }
}

/// Errors from reward-engine operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewardError {
    /// Invalid escrow status transition.
    InvalidEscrowTransition {
        escrow_id: EscrowId,
        from: EscrowStatus,
        to: EscrowStatus,
    },
    /// Escrow release attempted before the release epoch.
    EscrowReleaseTooEarly {
        escrow_id: EscrowId,
        current_epoch: EpochId,
        release_epoch: EpochId,
    },
}

impl std::fmt::Display for RewardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEscrowTransition { escrow_id, from, to } => {
                write!(
                    f,
                    "escrow {} invalid transition {:?} -> {:?}",
                    escrow_id, from, to
                )
            }
            Self::EscrowReleaseTooEarly {
                escrow_id,
                current_epoch,
                release_epoch,
            } => {
                write!(
                    f,
                    "escrow {} release attempted at epoch {} but not releasable until epoch {}",
                    escrow_id, current_epoch.0, release_epoch.0
                )
            }
        }
    }
}

/// Create an escrow record for a provisionally accepted block.
///
/// The escrow holds the proposer's bond pending challenge window
/// expiration. If no challenge is upheld, the escrow is released.
/// If a challenge is upheld, the escrow is slashed.
pub fn create_block_escrow(
    id: EscrowId,
    block_id: BlockId,
    proposer: ParticipantId,
    bond_amount: TokenAmount,
    created_epoch: EpochId,
    config: &RewardConfig,
) -> EscrowRecord {
    let release_epoch = EpochId(created_epoch.0 + config.challenge_window_epochs);
    EscrowRecord {
        id,
        block_id,
        kind: EscrowKind::ProposerBond,
        beneficiary: proposer,
        amount: bond_amount,
        status: EscrowStatus::Held,
        created_epoch,
        release_epoch,
    }
}

/// Create the staged reward tranches for an accepted block.
///
/// Returns `(provisional, survival)`:
///
/// - The **provisional** tranche is releasable immediately
///   (`release_epoch == created_epoch`) — the spec's "immediate incentive
///   for proposers". Callers release it at acceptance.
/// - The **survival** tranche is releasable only after the challenge
///   window (`release_epoch == created_epoch + challenge_window_epochs`) —
///   the spec's reward for surviving falsification.
///
/// Both tranches are slashed if a challenge against the block is upheld
/// while they are still held.
pub fn create_reward_tranches(
    provisional_id: EscrowId,
    survival_id: EscrowId,
    block_id: BlockId,
    proposer: ParticipantId,
    created_epoch: EpochId,
    config: &RewardConfig,
) -> (EscrowRecord, EscrowRecord) {
    let provisional = EscrowRecord {
        id: provisional_id,
        block_id,
        kind: EscrowKind::ProvisionalReward,
        beneficiary: proposer,
        amount: config.provisional_reward_amount(),
        status: EscrowStatus::Held,
        created_epoch,
        release_epoch: created_epoch,
    };
    let survival = EscrowRecord {
        id: survival_id,
        block_id,
        kind: EscrowKind::SurvivalReward,
        beneficiary: proposer,
        amount: config.survival_reward_amount(),
        status: EscrowStatus::Held,
        created_epoch,
        release_epoch: EpochId(created_epoch.0 + config.challenge_window_epochs),
    };
    (provisional, survival)
}

/// Create an escrow for a challenger's bond.
///
/// The bond is held during adjudication and is releasable as soon as the
/// challenge resolves (`release_epoch == created_epoch`):
///
/// - **Upheld** or **expired**: released back to the challenger.
/// - **Rejected**: slashed (the challenger loses the bond).
pub fn create_challenge_escrow(
    id: EscrowId,
    target_block_id: BlockId,
    challenger: ParticipantId,
    bond_amount: TokenAmount,
    created_epoch: EpochId,
) -> EscrowRecord {
    EscrowRecord {
        id,
        block_id: target_block_id,
        kind: EscrowKind::ChallengerBond,
        beneficiary: challenger,
        amount: bond_amount,
        status: EscrowStatus::Held,
        created_epoch,
        release_epoch: created_epoch,
    }
}

/// Split a slashed total into (challenger payout, burned residual).
///
/// The payout fraction is `config.challenger_payout_bps`; the residual is
/// burned (no treasury exists in Phase 0). Integer division truncates in
/// favor of the burn, so `payout + burned == slashed_total` always holds.
pub fn compute_slash_distribution(
    slashed_total: TokenAmount,
    config: &RewardConfig,
) -> (TokenAmount, TokenAmount) {
    let total = slashed_total.as_u64();
    let payout = apply_bps(total, config.challenger_payout_bps);
    (TokenAmount::new(payout), TokenAmount::new(total - payout))
}

/// Release escrowed funds to the beneficiary.
///
/// Called when a block settles without upheld challenges. The proposer's
/// bond (and eventually reward) become available.
///
/// Release is only permitted at or after `release_epoch`, enforcing the
/// challenge survival boundary. Attempting to release before the challenge
/// window has elapsed is an error.
///
/// Transition: Held → Released.
pub fn release_escrow(
    record: &mut EscrowRecord,
    current_epoch: EpochId,
) -> Result<(), RewardError> {
    if record.status != EscrowStatus::Held {
        return Err(RewardError::InvalidEscrowTransition {
            escrow_id: record.id,
            from: record.status,
            to: EscrowStatus::Released,
        });
    }
    if current_epoch.0 < record.release_epoch.0 {
        return Err(RewardError::EscrowReleaseTooEarly {
            escrow_id: record.id,
            current_epoch,
            release_epoch: record.release_epoch,
        });
    }
    record.status = EscrowStatus::Released;
    Ok(())
}

/// Slash escrowed funds due to an upheld challenge.
///
/// Called when a challenge against the escrowed block is upheld.
/// The proposer's bond is forfeited. Distribution of slashed funds
/// (to challenger, treasury, etc.) is deferred to later phases.
///
/// Transition: Held → Slashed.
pub fn slash_escrow(record: &mut EscrowRecord) -> Result<(), RewardError> {
    if record.status != EscrowStatus::Held {
        return Err(RewardError::InvalidEscrowTransition {
            escrow_id: record.id,
            from: record.status,
            to: EscrowStatus::Slashed,
        });
    }
    record.status = EscrowStatus::Slashed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::*;

    fn default_config() -> RewardConfig {
        RewardConfig::default()
    }

    #[test]
    fn create_escrow_for_block() {
        let escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        assert_eq!(escrow.status, EscrowStatus::Held);
        assert_eq!(escrow.block_id, test_block_id(10));
        assert_eq!(escrow.amount, TokenAmount::new(500));
        assert_eq!(escrow.created_epoch, EpochId(3));
        assert_eq!(escrow.release_epoch, EpochId(8)); // 3 + 5
    }

    #[test]
    fn release_held_escrow_at_release_epoch() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        // release_epoch is 8 (3 + 5). Release at exactly epoch 8 should work.
        release_escrow(&mut escrow, EpochId(8)).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    #[test]
    fn release_held_escrow_after_release_epoch() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        // Release at epoch 10 (after release_epoch 8) should work.
        release_escrow(&mut escrow, EpochId(10)).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    #[test]
    fn cannot_release_escrow_before_release_epoch() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        // release_epoch is 8. Trying to release at epoch 5 should fail.
        let err = release_escrow(&mut escrow, EpochId(5)).unwrap_err();
        assert!(matches!(err, RewardError::EscrowReleaseTooEarly { .. }));
        // Escrow should still be Held.
        assert_eq!(escrow.status, EscrowStatus::Held);
    }

    #[test]
    fn cannot_release_escrow_one_epoch_early() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        // release_epoch is 8. Epoch 7 is one short.
        let err = release_escrow(&mut escrow, EpochId(7)).unwrap_err();
        assert!(matches!(err, RewardError::EscrowReleaseTooEarly { .. }));
    }

    #[test]
    fn slash_held_escrow() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        slash_escrow(&mut escrow).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Slashed);
    }

    #[test]
    fn cannot_release_slashed_escrow() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        slash_escrow(&mut escrow).unwrap();
        assert!(release_escrow(&mut escrow, EpochId(8)).is_err());
    }

    #[test]
    fn cannot_slash_released_escrow() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        release_escrow(&mut escrow, EpochId(8)).unwrap();
        assert!(slash_escrow(&mut escrow).is_err());
    }

    #[test]
    fn default_config_is_valid_and_splits_reward() {
        let config = default_config();
        config.validate().unwrap();
        // 1000 total, 20% provisional.
        assert_eq!(config.provisional_reward_amount(), TokenAmount::new(200));
        assert_eq!(config.survival_reward_amount(), TokenAmount::new(800));
        // Tranches always sum to the base reward.
        assert_eq!(
            config.provisional_reward_amount().as_u64()
                + config.survival_reward_amount().as_u64(),
            config.base_block_reward
        );
    }

    #[test]
    fn config_validation_rejects_excess_bps() {
        let mut config = default_config();
        config.provisional_reward_bps = 10_001;
        assert!(config.validate().is_err());

        let mut config = default_config();
        config.challenger_payout_bps = 10_001;
        assert!(config.validate().is_err());
    }

    #[test]
    fn reward_tranches_have_staged_release_epochs() {
        let config = default_config();
        let (provisional, survival) = create_reward_tranches(
            EscrowId::from_bytes([2u8; 32]),
            EscrowId::from_bytes([3u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            EpochId(3),
            &config,
        );

        assert_eq!(provisional.kind, EscrowKind::ProvisionalReward);
        assert_eq!(provisional.amount, TokenAmount::new(200));
        // Provisional is releasable immediately.
        assert_eq!(provisional.release_epoch, EpochId(3));

        assert_eq!(survival.kind, EscrowKind::SurvivalReward);
        assert_eq!(survival.amount, TokenAmount::new(800));
        // Survival is releasable only after the challenge window.
        assert_eq!(survival.release_epoch, EpochId(8));

        // Provisional releases at creation epoch; survival does not.
        let mut provisional = provisional;
        let mut survival = survival;
        release_escrow(&mut provisional, EpochId(3)).unwrap();
        assert!(release_escrow(&mut survival, EpochId(3)).is_err());
        release_escrow(&mut survival, EpochId(8)).unwrap();
    }

    #[test]
    fn challenge_escrow_lifecycle() {
        let mut escrow = create_challenge_escrow(
            EscrowId::from_bytes([4u8; 32]),
            test_block_id(10),
            test_participant_id(5),
            TokenAmount::new(200),
            EpochId(4),
        );
        assert_eq!(escrow.kind, EscrowKind::ChallengerBond);
        assert_eq!(escrow.status, EscrowStatus::Held);
        // Releasable as soon as adjudication resolves (same epoch).
        release_escrow(&mut escrow, EpochId(4)).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Released);

        // Rejected path: slash instead.
        let mut escrow = create_challenge_escrow(
            EscrowId::from_bytes([5u8; 32]),
            test_block_id(10),
            test_participant_id(5),
            TokenAmount::new(200),
            EpochId(4),
        );
        slash_escrow(&mut escrow).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Slashed);
    }

    #[test]
    fn slash_distribution_splits_and_conserves() {
        let config = default_config(); // 50% payout
        let (payout, burned) =
            compute_slash_distribution(TokenAmount::new(1_300), &config);
        assert_eq!(payout, TokenAmount::new(650));
        assert_eq!(burned, TokenAmount::new(650));

        // Odd totals truncate in favor of the burn and still conserve.
        let (payout, burned) =
            compute_slash_distribution(TokenAmount::new(1_301), &config);
        assert_eq!(payout, TokenAmount::new(650));
        assert_eq!(burned, TokenAmount::new(651));
        assert_eq!(payout.as_u64() + burned.as_u64(), 1_301);
    }

    #[test]
    fn slash_distribution_no_overflow_at_u64_max() {
        let config = default_config();
        let (payout, burned) =
            compute_slash_distribution(TokenAmount::new(u64::MAX), &config);
        assert_eq!(payout.as_u64() + burned.as_u64(), u64::MAX);
    }

    #[test]
    fn cannot_double_release() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        release_escrow(&mut escrow, EpochId(8)).unwrap();
        assert!(release_escrow(&mut escrow, EpochId(8)).is_err());
    }
}
