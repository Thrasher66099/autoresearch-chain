// SPDX-License-Identifier: AGPL-3.0-or-later

//! Error types for protocol-rules operations.

use arc_protocol_types::{BlockId, BlockStatus, DomainId};

/// Errors produced by protocol-rules operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// Block failed structural validation.
    BlockStructurallyInvalid {
        block_id: BlockId,
        reasons: Vec<String>,
    },
    /// Block references a domain that is not active.
    DomainNotActive {
        block_id: BlockId,
        domain_id: DomainId,
    },
    /// Block references a parent that does not exist.
    ParentNotFound {
        block_id: BlockId,
        parent_id: BlockId,
    },
    /// Block's parent is not in an acceptable status.
    ParentNotAccepted {
        block_id: BlockId,
        parent_id: BlockId,
        parent_status: BlockStatus,
    },
    /// Invalid block status transition.
    InvalidBlockTransition {
        block_id: BlockId,
        from: BlockStatus,
        to: BlockStatus,
    },
    /// Block not found.
    BlockNotFound {
        block_id: BlockId,
    },
    /// Attestation for unknown block.
    AttestationBlockNotFound {
        block_id: BlockId,
    },
    /// Block not in a state that accepts attestations.
    BlockNotUnderValidation {
        block_id: BlockId,
        status: BlockStatus,
    },
    /// Insufficient validators in pool.
    InsufficientValidators {
        domain_id: DomainId,
        available: usize,
        required: usize,
    },
    /// Block bond is below the configured minimum.
    BlockBondInsufficient {
        block_id: BlockId,
        provided: u64,
        required: u64,
    },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockStructurallyInvalid { block_id, reasons } => {
                write!(f, "block {} invalid: {}", block_id, reasons.join(", "))
            }
            Self::DomainNotActive { block_id, domain_id } => {
                write!(f, "block {} targets inactive domain {}", block_id, domain_id)
            }
            Self::ParentNotFound { block_id, parent_id } => {
                write!(f, "block {} parent {} not found", block_id, parent_id)
            }
            Self::ParentNotAccepted { block_id, parent_id, parent_status } => {
                write!(
                    f,
                    "block {} parent {} is {:?}, not accepted",
                    block_id, parent_id, parent_status
                )
            }
            Self::InvalidBlockTransition { block_id, from, to } => {
                write!(f, "block {} invalid transition {:?} -> {:?}", block_id, from, to)
            }
            Self::BlockNotFound { block_id } => {
                write!(f, "block {} not found", block_id)
            }
            Self::AttestationBlockNotFound { block_id } => {
                write!(f, "attestation for unknown block {}", block_id)
            }
            Self::BlockNotUnderValidation { block_id, status } => {
                write!(f, "block {} is {:?}, not under validation", block_id, status)
            }
            Self::InsufficientValidators { domain_id, available, required } => {
                write!(
                    f,
                    "domain {} has {} validators, need {}",
                    domain_id, available, required
                )
            }
            Self::BlockBondInsufficient { block_id, provided, required } => {
                write!(f, "block {} bond {} below minimum {}", block_id, provided, required)
            }
        }
    }
}
