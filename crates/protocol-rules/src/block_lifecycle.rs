// SPDX-License-Identifier: AGPL-3.0-or-later

//! Block lifecycle state transitions.
//!
//! Submitted → UnderValidation → ValidationComplete
//! → UnderChallenge → ChallengeWindowClosed → Settled → Final
//! or Rejected at various points.
//!
//! State transition functions assume structurally valid input.
//! Structural validation is the caller's responsibility and should
//! happen before any transition function is called.

use arc_protocol_types::{Block, BlockStatus};
use crate::attestation::ProvisionalOutcome;
use crate::config::ValidationConfig;
use crate::error::ProtocolError;

/// Transition a block from Submitted to UnderValidation.
///
/// Called after validators have been assigned.
pub fn begin_validation(block: &mut Block) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::Submitted {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::UnderValidation,
        });
    }
    block.status = BlockStatus::UnderValidation;
    Ok(())
}

/// Transition a block from UnderValidation to the appropriate status
/// based on the provisional outcome.
///
/// Accepted → ValidationComplete (proceeds to challenge window)
/// Rejected → Rejected
/// Inconclusive → depends on config.inconclusive_is_rejection
pub fn complete_validation(
    block: &mut Block,
    outcome: ProvisionalOutcome,
    config: &ValidationConfig,
) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::UnderValidation {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::ValidationComplete,
        });
    }

    match outcome {
        ProvisionalOutcome::Accepted => {
            block.status = BlockStatus::ValidationComplete;
        }
        ProvisionalOutcome::Rejected => {
            block.status = BlockStatus::Rejected;
        }
        ProvisionalOutcome::Inconclusive => {
            if config.inconclusive_is_rejection {
                block.status = BlockStatus::Rejected;
            } else {
                // Future: could remain in UnderValidation or move to a
                // pending state. For now, reject if config says so.
                block.status = BlockStatus::Rejected;
            }
        }
    }
    Ok(())
}

/// Open the challenge window for a validated block.
pub fn open_challenge_window(block: &mut Block) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::ValidationComplete {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::UnderChallenge,
        });
    }
    block.status = BlockStatus::UnderChallenge;
    Ok(())
}

/// Close the challenge window (no upheld challenges).
pub fn close_challenge_window(block: &mut Block) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::UnderChallenge {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::ChallengeWindowClosed,
        });
    }
    block.status = BlockStatus::ChallengeWindowClosed;
    Ok(())
}

/// Settle a block (release rewards).
pub fn settle_block(block: &mut Block) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::ChallengeWindowClosed {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::Settled,
        });
    }
    block.status = BlockStatus::Settled;
    Ok(())
}

/// Finalize a settled block.
pub fn finalize_block(block: &mut Block) -> Result<(), ProtocolError> {
    if block.status != BlockStatus::Settled {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::Final,
        });
    }
    block.status = BlockStatus::Final;
    Ok(())
}

/// Reject a block during validation (can happen from multiple pre-settlement states).
///
/// Rejection means the block never reached a valid accepted state.
/// For blocks that were accepted but subsequently proven invalid by a
/// challenge, use [`invalidate_block`] instead.
pub fn reject_block(block: &mut Block) -> Result<(), ProtocolError> {
    match block.status {
        BlockStatus::Submitted
        | BlockStatus::UnderValidation
        | BlockStatus::ValidationComplete
        | BlockStatus::UnderChallenge => {
            block.status = BlockStatus::Rejected;
            Ok(())
        }
        _ => Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::Rejected,
        }),
    }
}

/// Invalidate a previously accepted block due to an upheld challenge.
///
/// Distinct from rejection: rejected blocks never passed validation,
/// while invalidated blocks were provisionally accepted but subsequently
/// proven invalid through the challenge mechanism.
///
/// Can be called from accepted states: ValidationComplete, UnderChallenge,
/// ChallengeWindowClosed. Cannot invalidate Settled or Final blocks
/// (economic finality prevents this in Phase 0.3).
pub fn invalidate_block(block: &mut Block) -> Result<(), ProtocolError> {
    match block.status {
        BlockStatus::ValidationComplete
        | BlockStatus::UnderChallenge
        | BlockStatus::ChallengeWindowClosed => {
            block.status = BlockStatus::Invalidated;
            Ok(())
        }
        _ => Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::Invalidated,
        }),
    }
}

/// Check if a block status is considered "accepted" for the purpose
/// of allowing child blocks to be submitted against it.
pub fn is_block_accepted(status: BlockStatus) -> bool {
    matches!(
        status,
        BlockStatus::ValidationComplete
            | BlockStatus::UnderChallenge
            | BlockStatus::ChallengeWindowClosed
            | BlockStatus::Settled
            | BlockStatus::Final
    )
}

/// Check block submission preconditions (state check only, no structural validation).
///
/// Verifies the block is in Submitted state and the parent is accepted.
/// Structural validation must be done by the caller beforehand.
pub fn check_submission_preconditions(
    block: &Block,
    parent_status: BlockStatus,
) -> Result<(), ProtocolError> {
    // Check parent is in an accepted state.
    if !is_block_accepted(parent_status) {
        return Err(ProtocolError::ParentNotAccepted {
            block_id: block.id,
            parent_id: block.parent_id,
            parent_status,
        });
    }

    // Block must be in Submitted state.
    if block.status != BlockStatus::Submitted {
        return Err(ProtocolError::InvalidBlockTransition {
            block_id: block.id,
            from: block.status,
            to: BlockStatus::Submitted,
        });
    }

    Ok(())
}

/// Check that the block's bond meets the configured minimum.
pub fn check_block_bond(
    block: &Block,
    config: &ValidationConfig,
) -> Result<(), ProtocolError> {
    if block.bond.as_u64() < config.min_block_bond {
        return Err(ProtocolError::BlockBondInsufficient {
            block_id: block.id,
            provided: block.bond.as_u64(),
            required: config.min_block_bond,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::valid_block;

    fn default_config() -> ValidationConfig {
        ValidationConfig::default()
    }

    #[test]
    fn full_happy_path_lifecycle() {
        let mut block = valid_block();
        let config = default_config();
        assert_eq!(block.status, BlockStatus::Submitted);

        begin_validation(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::UnderValidation);

        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        assert_eq!(block.status, BlockStatus::ValidationComplete);

        open_challenge_window(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::UnderChallenge);

        close_challenge_window(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::ChallengeWindowClosed);

        settle_block(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::Settled);

        finalize_block(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::Final);
    }

    #[test]
    fn rejected_during_validation() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Rejected, &config).unwrap();
        assert_eq!(block.status, BlockStatus::Rejected);
    }

    #[test]
    fn cannot_validate_already_validated() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        assert!(begin_validation(&mut block).is_err());
    }

    #[test]
    fn reject_from_challenge() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        reject_block(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::Rejected);
    }

    #[test]
    fn cannot_reject_finalized() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        close_challenge_window(&mut block).unwrap();
        settle_block(&mut block).unwrap();
        finalize_block(&mut block).unwrap();
        assert!(reject_block(&mut block).is_err());
    }

    #[test]
    fn check_submission_with_accepted_parent() {
        let block = valid_block();
        assert!(check_submission_preconditions(&block, BlockStatus::Final).is_ok());
    }

    #[test]
    fn check_submission_rejects_unaccepted_parent() {
        let block = valid_block();
        assert!(check_submission_preconditions(&block, BlockStatus::Submitted).is_err());
    }

    #[test]
    fn check_bond_sufficient() {
        let block = valid_block();
        let config = default_config();
        assert!(check_block_bond(&block, &config).is_ok());
    }

    #[test]
    fn invalidate_from_under_challenge() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::UnderChallenge);

        invalidate_block(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::Invalidated);
    }

    #[test]
    fn invalidate_from_challenge_window_closed() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        close_challenge_window(&mut block).unwrap();

        invalidate_block(&mut block).unwrap();
        assert_eq!(block.status, BlockStatus::Invalidated);
    }

    #[test]
    fn cannot_invalidate_settled() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        close_challenge_window(&mut block).unwrap();
        settle_block(&mut block).unwrap();

        assert!(invalidate_block(&mut block).is_err());
    }

    #[test]
    fn cannot_invalidate_finalized() {
        let mut block = valid_block();
        let config = default_config();
        begin_validation(&mut block).unwrap();
        complete_validation(&mut block, ProvisionalOutcome::Accepted, &config).unwrap();
        open_challenge_window(&mut block).unwrap();
        close_challenge_window(&mut block).unwrap();
        settle_block(&mut block).unwrap();
        finalize_block(&mut block).unwrap();

        assert!(invalidate_block(&mut block).is_err());
    }

    #[test]
    fn cannot_invalidate_submitted() {
        let mut block = valid_block();
        assert!(invalidate_block(&mut block).is_err());
    }

    #[test]
    fn invalidated_block_is_not_accepted() {
        assert!(!is_block_accepted(BlockStatus::Invalidated));
    }
}
