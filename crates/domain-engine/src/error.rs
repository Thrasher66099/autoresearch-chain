// SPDX-License-Identifier: AGPL-3.0-or-later

//! Error types for domain-engine operations.

use arc_protocol_types::{GenesisBlockId, DomainId, TrackActivationState};

/// Errors produced by domain-engine operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    /// Genesis block failed structural validation.
    StructuralValidationFailed {
        genesis_id: GenesisBlockId,
        reasons: Vec<String>,
    },
    /// Genesis bond is below the configured minimum.
    InsufficientGenesisBond {
        genesis_id: GenesisBlockId,
        provided: u64,
        required: u64,
    },
    /// Genesis block failed RTS conformance.
    RtsConformanceFailed {
        genesis_id: GenesisBlockId,
        reasons: Vec<String>,
    },
    /// Seed validation failed (score not reproducible).
    SeedValidationFailed {
        genesis_id: GenesisBlockId,
        reason: String,
    },
    /// Invalid state transition for track activation.
    InvalidActivationTransition {
        genesis_id: GenesisBlockId,
        from: TrackActivationState,
        to: TrackActivationState,
    },
    /// Genesis proposal not found.
    GenesisNotFound {
        genesis_id: GenesisBlockId,
    },
    /// Domain already exists.
    DomainAlreadyExists {
        domain_id: DomainId,
    },
    /// Domain not found.
    DomainNotFound {
        domain_id: DomainId,
    },
    /// Domain is not active (cannot accept blocks).
    DomainNotActive {
        domain_id: DomainId,
    },
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StructuralValidationFailed { genesis_id, reasons } => {
                write!(f, "genesis {} structural validation failed: {}", genesis_id, reasons.join(", "))
            }
            Self::InsufficientGenesisBond { genesis_id, provided, required } => {
                write!(f, "genesis {} bond {} below minimum {}", genesis_id, provided, required)
            }
            Self::RtsConformanceFailed { genesis_id, reasons } => {
                write!(f, "genesis {} RTS conformance failed: {}", genesis_id, reasons.join(", "))
            }
            Self::SeedValidationFailed { genesis_id, reason } => {
                write!(f, "genesis {} seed validation failed: {}", genesis_id, reason)
            }
            Self::InvalidActivationTransition { genesis_id, from, to } => {
                write!(f, "genesis {} invalid transition {:?} -> {:?}", genesis_id, from, to)
            }
            Self::GenesisNotFound { genesis_id } => {
                write!(f, "genesis {} not found", genesis_id)
            }
            Self::DomainAlreadyExists { domain_id } => {
                write!(f, "domain {} already exists", domain_id)
            }
            Self::DomainNotFound { domain_id } => {
                write!(f, "domain {} not found", domain_id)
            }
            Self::DomainNotActive { domain_id } => {
                write!(f, "domain {} is not active", domain_id)
            }
        }
    }
}
