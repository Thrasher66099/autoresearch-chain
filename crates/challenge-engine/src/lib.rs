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

//! Challenge lifecycle and state transitions for AutoResearch Chain.
//!
//! # Phase 0.2 implementation
//!
//! - Challenge creation and target validation
//! - Challenge state machine (Open → UnderReview → Upheld | Rejected | Expired)
//! - Challenge record management
//!
//! Full challenge economics, escalation, and remedy application are
//! deferred to a later phase.

use serde::{Serialize, Deserialize};

use arc_protocol_types::{
    ArtifactHash, BlockId, BlockStatus, ChallengeId, ChallengeRecord,
    ChallengeStatus, ChallengeTarget, ChallengeType, EpochId,
    ParticipantId, TokenAmount,
};

/// Configuration for challenge-engine policy thresholds.
///
/// Separated from logic so thresholds can be tuned without touching
/// state machine code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChallengeConfig {
    /// Minimum bond required to open a challenge.
    pub min_challenge_bond: u64,
}

impl Default for ChallengeConfig {
    fn default() -> Self {
        Self {
            min_challenge_bond: 100,
        }
    }
}

/// Errors from challenge-engine operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChallengeError {
    /// Challenge target is invalid (block not found, etc.).
    InvalidTarget { reason: String },
    /// Challenge evidence is structurally invalid (zero hash).
    InvalidEvidence,
    /// Challenge bond is below the configured minimum.
    InsufficientBond { provided: u64, required: u64 },
    /// Invalid state transition.
    InvalidTransition {
        challenge_id: ChallengeId,
        from: ChallengeStatus,
        to: ChallengeStatus,
    },
    /// Challenge not found.
    NotFound { challenge_id: ChallengeId },
    /// Target block is not in a challengeable state.
    TargetNotChallengeable { block_id: BlockId, status: BlockStatus },
}

impl std::fmt::Display for ChallengeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTarget { reason } => write!(f, "invalid target: {}", reason),
            Self::InvalidEvidence => write!(f, "challenge evidence is invalid (zero hash)"),
            Self::InsufficientBond { provided, required } => {
                write!(f, "bond {} below minimum {}", provided, required)
            }
            Self::InvalidTransition { challenge_id, from, to } => {
                write!(f, "challenge {} invalid {:?} -> {:?}", challenge_id, from, to)
            }
            Self::NotFound { challenge_id } => {
                write!(f, "challenge {} not found", challenge_id)
            }
            Self::TargetNotChallengeable { block_id, status } => {
                write!(f, "block {} is {:?}, not challengeable", block_id, status)
            }
        }
    }
}

/// Validate that a challenge target is in a challengeable state.
///
/// For block challenges: the block must be in UnderChallenge status.
/// For other target types: Phase 0.2 does basic structural checks only.
pub fn validate_challenge_target(
    target: &ChallengeTarget,
    block_status_lookup: impl Fn(&BlockId) -> Option<BlockStatus>,
) -> Result<(), ChallengeError> {
    match target {
        ChallengeTarget::Block { block_id } => {
            match block_status_lookup(block_id) {
                Some(BlockStatus::UnderChallenge) => Ok(()),
                Some(status) => Err(ChallengeError::TargetNotChallengeable {
                    block_id: *block_id,
                    status,
                }),
                None => Err(ChallengeError::InvalidTarget {
                    reason: format!("block {} not found", block_id),
                }),
            }
        }
        ChallengeTarget::Attestation { block_id, .. } => {
            match block_status_lookup(block_id) {
                Some(BlockStatus::UnderChallenge) => Ok(()),
                Some(status) => Err(ChallengeError::TargetNotChallengeable {
                    block_id: *block_id,
                    status,
                }),
                None => Err(ChallengeError::InvalidTarget {
                    reason: format!("block {} not found", block_id),
                }),
            }
        }
        ChallengeTarget::Attribution { block_id, .. } => {
            if block_status_lookup(block_id).is_none() {
                return Err(ChallengeError::InvalidTarget {
                    reason: format!("block {} not found", block_id),
                });
            }
            Ok(())
        }
        ChallengeTarget::DominanceDecision { .. } => {
            // Dominance challenges are validated differently; accept for now.
            Ok(())
        }
    }
}

/// Open a new challenge.
///
/// Structural checks (evidence non-zero) are separated from policy checks
/// (bond minimum from config) and state checks (target challengeability).
pub fn open_challenge(
    id: ChallengeId,
    challenge_type: ChallengeType,
    target: ChallengeTarget,
    challenger: ParticipantId,
    bond: TokenAmount,
    evidence_ref: ArtifactHash,
    epoch_id: EpochId,
    timestamp: u64,
    config: &ChallengeConfig,
    block_status_lookup: impl Fn(&BlockId) -> Option<BlockStatus>,
) -> Result<ChallengeRecord, ChallengeError> {
    // Structural check: evidence must be non-zero.
    if evidence_ref == ArtifactHash::ZERO {
        return Err(ChallengeError::InvalidEvidence);
    }

    // Policy check: bond must meet configured minimum.
    if bond.as_u64() < config.min_challenge_bond {
        return Err(ChallengeError::InsufficientBond {
            provided: bond.as_u64(),
            required: config.min_challenge_bond,
        });
    }

    // State check: target must be challengeable.
    validate_challenge_target(&target, block_status_lookup)?;

    Ok(ChallengeRecord {
        id,
        challenge_type,
        target,
        challenger,
        bond,
        evidence_ref,
        status: ChallengeStatus::Open,
        epoch_id,
        timestamp,
    })
}

/// Transition a challenge from Open to UnderReview.
pub fn begin_review(challenge: &mut ChallengeRecord) -> Result<(), ChallengeError> {
    if challenge.status != ChallengeStatus::Open {
        return Err(ChallengeError::InvalidTransition {
            challenge_id: challenge.id,
            from: challenge.status,
            to: ChallengeStatus::UnderReview,
        });
    }
    challenge.status = ChallengeStatus::UnderReview;
    Ok(())
}

/// Uphold a challenge (target is invalidated).
pub fn uphold_challenge(challenge: &mut ChallengeRecord) -> Result<(), ChallengeError> {
    if challenge.status != ChallengeStatus::UnderReview {
        return Err(ChallengeError::InvalidTransition {
            challenge_id: challenge.id,
            from: challenge.status,
            to: ChallengeStatus::Upheld,
        });
    }
    challenge.status = ChallengeStatus::Upheld;
    Ok(())
}

/// Reject a challenge (challenger loses bond).
pub fn reject_challenge(challenge: &mut ChallengeRecord) -> Result<(), ChallengeError> {
    if challenge.status != ChallengeStatus::UnderReview {
        return Err(ChallengeError::InvalidTransition {
            challenge_id: challenge.id,
            from: challenge.status,
            to: ChallengeStatus::Rejected,
        });
    }
    challenge.status = ChallengeStatus::Rejected;
    Ok(())
}

/// Expire a challenge that was not resolved in time.
pub fn expire_challenge(challenge: &mut ChallengeRecord) -> Result<(), ChallengeError> {
    match challenge.status {
        ChallengeStatus::Open | ChallengeStatus::UnderReview => {
            challenge.status = ChallengeStatus::Expired;
            Ok(())
        }
        _ => Err(ChallengeError::InvalidTransition {
            challenge_id: challenge.id,
            from: challenge.status,
            to: ChallengeStatus::Expired,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::*;

    fn challenge_block_lookup(status: BlockStatus) -> impl Fn(&BlockId) -> Option<BlockStatus> {
        move |_| Some(status)
    }

    fn default_config() -> ChallengeConfig {
        ChallengeConfig::default()
    }

    #[test]
    fn open_challenge_happy_path() {
        let challenge = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap();

        assert_eq!(challenge.status, ChallengeStatus::Open);
    }

    #[test]
    fn open_challenge_insufficient_bond() {
        let err = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(10),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap_err();

        assert!(matches!(err, ChallengeError::InsufficientBond { .. }));
    }

    #[test]
    fn open_challenge_target_not_challengeable() {
        let err = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::Submitted),
        )
        .unwrap_err();

        assert!(matches!(err, ChallengeError::TargetNotChallengeable { .. }));
    }

    #[test]
    fn challenge_lifecycle_upheld() {
        let mut challenge = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap();

        begin_review(&mut challenge).unwrap();
        assert_eq!(challenge.status, ChallengeStatus::UnderReview);

        uphold_challenge(&mut challenge).unwrap();
        assert_eq!(challenge.status, ChallengeStatus::Upheld);
    }

    #[test]
    fn challenge_lifecycle_rejected() {
        let mut challenge = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap();

        begin_review(&mut challenge).unwrap();
        reject_challenge(&mut challenge).unwrap();
        assert_eq!(challenge.status, ChallengeStatus::Rejected);
    }

    #[test]
    fn challenge_expire_from_open() {
        let mut challenge = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &default_config(),
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap();

        expire_challenge(&mut challenge).unwrap();
        assert_eq!(challenge.status, ChallengeStatus::Expired);
    }

    #[test]
    fn custom_config_bond_threshold() {
        let config = ChallengeConfig {
            min_challenge_bond: 500,
        };
        let err = open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id: test_block_id(2) },
            test_participant_id(3),
            TokenAmount::new(200),
            test_artifact_hash(80),
            EpochId(5),
            2000,
            &config,
            challenge_block_lookup(BlockStatus::UnderChallenge),
        )
        .unwrap_err();

        match err {
            ChallengeError::InsufficientBond { required, .. } => {
                assert_eq!(required, 500);
            }
            _ => panic!("expected InsufficientBond"),
        }
    }
}
