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
//! # Phase 0.3 implementation
//!
//! Minimal but real escrow operations:
//!
//! - Escrow creation when blocks are provisionally accepted
//! - Escrow release when blocks settle without upheld challenges
//! - Escrow slashing when challenges are upheld
//!
//! Full staged reward release, attribution-weighted distribution,
//! slashing economics, and ancestry farming prevention are deferred
//! to later phases. The interfaces here are designed to support those
//! extensions without structural rewrites.

use arc_protocol_types::{
    BlockId, EpochId, EscrowId, EscrowRecord, EscrowStatus, ParticipantId, TokenAmount,
};

/// Configuration for reward-engine policy.
#[derive(Clone, Debug)]
pub struct RewardConfig {
    /// Number of epochs the challenge window lasts.
    /// Escrow release epoch = created_epoch + challenge_window_epochs.
    pub challenge_window_epochs: u64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            challenge_window_epochs: 5,
        }
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
        beneficiary: proposer,
        amount: bond_amount,
        status: EscrowStatus::Held,
        created_epoch,
        release_epoch,
    }
}

/// Release escrowed funds to the beneficiary.
///
/// Called when a block settles without upheld challenges. The proposer's
/// bond (and eventually reward) become available.
///
/// Transition: Held → Released.
pub fn release_escrow(record: &mut EscrowRecord) -> Result<(), RewardError> {
    if record.status != EscrowStatus::Held {
        return Err(RewardError::InvalidEscrowTransition {
            escrow_id: record.id,
            from: record.status,
            to: EscrowStatus::Released,
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
    fn release_held_escrow() {
        let mut escrow = create_block_escrow(
            EscrowId::from_bytes([1u8; 32]),
            test_block_id(10),
            test_participant_id(1),
            TokenAmount::new(500),
            EpochId(3),
            &default_config(),
        );
        release_escrow(&mut escrow).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Released);
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
        assert!(release_escrow(&mut escrow).is_err());
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
        release_escrow(&mut escrow).unwrap();
        assert!(slash_escrow(&mut escrow).is_err());
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
        release_escrow(&mut escrow).unwrap();
        assert!(release_escrow(&mut escrow).is_err());
    }
}
