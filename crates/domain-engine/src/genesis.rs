// SPDX-License-Identifier: AGPL-3.0-or-later

//! Genesis activation state machine.
//!
//! Lifecycle: Proposed → ConformanceChecking → ValidationInProgress
//! → ActivationPending → Active | Failed | Expired
//!
//! Each transition is a pure function that takes current state + input
//! and returns new state or error.

use serde::{Serialize, Deserialize};

use arc_protocol_types::{
    DomainSpecId, DomainType, ForkFamilyId, GenesisBlock,
    MaterializationPolicyKind, ProblemDomain, DomainSpec,
    TrackActivationState, TrackInitialization, TrackTree, ValidatorVote,
};

use crate::config::GenesisActivationConfig;
use crate::error::DomainError;

/// A seed validation record submitted by a validator during genesis activation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedValidationRecord {
    pub validator: arc_protocol_types::ValidatorId,
    pub vote: ValidatorVote,
    pub observed_score: Option<arc_protocol_types::MetricValue>,
    pub timestamp: u64,
}

/// The in-progress state for a genesis activation.
///
/// Tracks the activation lifecycle and collects seed validation records.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisActivation {
    pub track_init: TrackInitialization,
    pub genesis_block: GenesisBlock,
    pub seed_validations: Vec<SeedValidationRecord>,
}

// ---------------------------------------------------------------------------
// State machine transitions
// ---------------------------------------------------------------------------

/// Submit a genesis proposal. Creates a new GenesisActivation in Proposed state.
pub fn submit_genesis_proposal(
    genesis: GenesisBlock,
    config: &GenesisActivationConfig,
) -> Result<GenesisActivation, DomainError> {
    // Check structural validity first.
    if let Err(errors) = arc_protocol_types::validate::validate_genesis_block_structure(&genesis) {
        return Err(DomainError::StructuralValidationFailed {
            genesis_id: genesis.id,
            reasons: errors.iter().map(|e| e.to_string()).collect(),
        });
    }

    // Policy check: minimum bond.
    if genesis.seed_bond.as_u64() < config.min_genesis_bond {
        return Err(DomainError::InsufficientGenesisBond {
            genesis_id: genesis.id,
            provided: genesis.seed_bond.as_u64(),
            required: config.min_genesis_bond,
        });
    }

    let track_init = TrackInitialization {
        genesis_block_id: genesis.id,
        domain_id: genesis.domain_id,
        state: TrackActivationState::Proposed,
        proposer: genesis.proposer,
        timestamp: genesis.timestamp,
    };

    Ok(GenesisActivation {
        track_init,
        genesis_block: genesis,
        seed_validations: Vec::new(),
    })
}

/// Evaluate RTS conformance. Transitions Proposed → ConformanceChecking → result.
///
/// On success, moves to ValidationInProgress.
/// On failure, moves to Failed.
pub fn evaluate_rts_conformance(
    activation: &mut GenesisActivation,
) -> Result<(), DomainError> {
    if activation.track_init.state != TrackActivationState::Proposed {
        return Err(DomainError::InvalidActivationTransition {
            genesis_id: activation.track_init.genesis_block_id,
            from: activation.track_init.state,
            to: TrackActivationState::ConformanceChecking,
        });
    }

    activation.track_init.state = TrackActivationState::ConformanceChecking;

    match crate::rts::check_rts_conformance(&activation.genesis_block) {
        Ok(()) => {
            activation.track_init.state = TrackActivationState::ValidationInProgress;
            Ok(())
        }
        Err(reasons) => {
            activation.track_init.state = TrackActivationState::Failed;
            Err(DomainError::RtsConformanceFailed {
                genesis_id: activation.track_init.genesis_block_id,
                reasons,
            })
        }
    }
}

/// Record a seed validation attestation.
///
/// Can only be called in ValidationInProgress state.
pub fn record_seed_validation(
    activation: &mut GenesisActivation,
    record: SeedValidationRecord,
) -> Result<(), DomainError> {
    if activation.track_init.state != TrackActivationState::ValidationInProgress {
        return Err(DomainError::InvalidActivationTransition {
            genesis_id: activation.track_init.genesis_block_id,
            from: activation.track_init.state,
            to: TrackActivationState::ValidationInProgress,
        });
    }

    activation.seed_validations.push(record);
    Ok(())
}

/// Attempt to finalize track activation.
///
/// Evaluates seed validation records against config thresholds.
/// On success: transitions to ActivationPending then Active, and returns
/// the instantiated domain objects.
/// On failure: transitions to Failed.
pub fn finalize_track_activation(
    activation: &mut GenesisActivation,
    config: &GenesisActivationConfig,
) -> Result<ActivatedDomain, DomainError> {
    if activation.track_init.state != TrackActivationState::ValidationInProgress {
        return Err(DomainError::InvalidActivationTransition {
            genesis_id: activation.track_init.genesis_block_id,
            from: activation.track_init.state,
            to: TrackActivationState::ActivationPending,
        });
    }

    let total = activation.seed_validations.len() as u32;
    if total < config.min_seed_validations {
        activation.track_init.state = TrackActivationState::Failed;
        return Err(DomainError::SeedValidationFailed {
            genesis_id: activation.track_init.genesis_block_id,
            reason: format!(
                "insufficient validations: {} < {}",
                total, config.min_seed_validations
            ),
        });
    }

    let pass_count = activation
        .seed_validations
        .iter()
        .filter(|v| v.vote == ValidatorVote::Pass)
        .count() as u32;
    let pass_ratio = pass_count as f64 / total as f64;

    if pass_ratio < config.seed_pass_threshold {
        activation.track_init.state = TrackActivationState::Failed;
        return Err(DomainError::SeedValidationFailed {
            genesis_id: activation.track_init.genesis_block_id,
            reason: format!(
                "pass ratio {:.2} below threshold {:.2}",
                pass_ratio, config.seed_pass_threshold
            ),
        });
    }

    // Policy check: fraud suspicion.
    let fraud_count = activation
        .seed_validations
        .iter()
        .filter(|v| v.vote == ValidatorVote::FraudSuspected)
        .count();
    if config.fraud_triggers_failure && fraud_count > 0 {
        activation.track_init.state = TrackActivationState::Failed;
        return Err(DomainError::SeedValidationFailed {
            genesis_id: activation.track_init.genesis_block_id,
            reason: format!("{} validator(s) suspected fraud", fraud_count),
        });
    }

    // All checks pass — activate.
    activation.track_init.state = TrackActivationState::ActivationPending;
    activation.track_init.state = TrackActivationState::Active;

    let genesis = &activation.genesis_block;
    Ok(instantiate_domain(genesis))
}

/// The bundle of objects created when a domain activates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivatedDomain {
    pub domain: ProblemDomain,
    pub domain_spec: DomainSpec,
    pub track_tree: TrackTree,
}

/// Instantiate domain objects from an activated genesis block.
fn instantiate_domain(genesis: &GenesisBlock) -> ActivatedDomain {
    let domain_spec_id = DomainSpecId::from_bytes(*genesis.id.as_bytes());

    let domain = ProblemDomain {
        id: genesis.domain_id,
        name: genesis.research_target_declaration.clone(),
        domain_type: DomainType::Root,
        parent_domain_id: None,
        spec_id: domain_spec_id,
    };

    let domain_spec = DomainSpec {
        id: domain_spec_id,
        domain_id: genesis.domain_id,
        base_codebase_ref: genesis.seed_codebase_state_ref,
        primary_metric: genesis.metric_id.clone(),
        metric_direction: genesis.metric_direction,
        secondary_metrics: Vec::new(),
        search_surface: genesis.search_surface.clone(),
        frozen_surface: genesis.frozen_surface.clone(),
        artifact_schema_ref: genesis.artifact_schema_ref,
        hardware_class: genesis.hardware_class.clone(),
        materialization_policy: MaterializationPolicyKind::OnDominance,
    };

    let track_tree = TrackTree {
        id: genesis.id.as_track_tree_id(),
        domain_id: genesis.domain_id,
        genesis_block_id: genesis.id,
        fork_families: Vec::<ForkFamilyId>::new(),
        canonical_frontier_block_id: None,
    };

    ActivatedDomain {
        domain,
        domain_spec,
        track_tree,
    }
}
