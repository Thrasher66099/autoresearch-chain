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

//! Canonical test fixtures for protocol types.
//!
//! These fixtures provide structurally valid and invalid instances of
//! core protocol types. They are intended for use in unit tests,
//! integration tests, and simulator scenarios across the workspace.
//!
//! All valid fixtures pass their corresponding structural validation.
//! Invalid fixtures violate exactly one structural invariant each,
//! making failures easy to diagnose.
//!
//! # Naming convention
//!
//! - `valid_*()` — returns a structurally valid instance
//! - `invalid_*_missing_*()` — returns an instance missing one required field
//! - `invalid_*_bad_*()` — returns an instance with a malformed field

use crate::block::Block;
use crate::challenge::{ChallengeRecord, ChallengeTarget};
use crate::enums::*;
use crate::frontier::CanonicalFrontierState;
use crate::genesis::{DatasetSplits, GenesisBlock};
use crate::ids::*;
use crate::metric::MetricValue;
use crate::token::TokenAmount;
use crate::validation::ValidationAttestation;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Create a non-zero ArtifactHash where all 32 bytes are set to `n`.
///
/// Using different `n` values produces distinct hashes. `n = 0` would
/// produce `ArtifactHash::ZERO`, which is invalid for artifact refs,
/// so callers should use `n >= 1`.
pub fn test_artifact_hash(n: u8) -> ArtifactHash {
    ArtifactHash::from_bytes([n; 32])
}

/// Create a non-zero ID of the given type from a byte value.
///
/// Convenience for fixtures that need distinct, non-zero identifiers.
pub fn test_domain_id(n: u8) -> DomainId {
    DomainId::from_bytes([n; 32])
}

pub fn test_block_id(n: u8) -> BlockId {
    BlockId::from_bytes([n; 32])
}

pub fn test_genesis_block_id(n: u8) -> GenesisBlockId {
    GenesisBlockId::from_bytes([n; 32])
}

pub fn test_proposer_id(n: u8) -> ProposerId {
    ProposerId::from_bytes([n; 32])
}

pub fn test_validator_id(n: u8) -> ValidatorId {
    ValidatorId::from_bytes([n; 32])
}

pub fn test_participant_id(n: u8) -> ParticipantId {
    ParticipantId::from_bytes([n; 32])
}

pub fn test_challenge_id(n: u8) -> ChallengeId {
    ChallengeId::from_bytes([n; 32])
}

// -----------------------------------------------------------------------
// Valid genesis block
// -----------------------------------------------------------------------

/// A structurally valid RTS-1 genesis block.
///
/// Models a CIFAR-10 training recipe improvement track: single metric
/// (test_accuracy), higher-is-better, fixed compute budget, with
/// distinct non-zero artifact references for every required field.
pub fn valid_genesis_block() -> GenesisBlock {
    GenesisBlock {
        id: test_genesis_block_id(1),
        rts_version: ResearchTrackStandardVersion::Rts1,
        domain_id: test_domain_id(1),
        proposer: test_proposer_id(1),
        research_target_declaration:
            "Improve CIFAR-10 training recipe accuracy within fixed compute budget"
                .to_string(),
        domain_intent: DomainIntent::EndToEndRecipeImprovement,
        seed_recipe_ref: test_artifact_hash(10),
        seed_codebase_state_ref: test_artifact_hash(11),
        frozen_surface: vec!["eval/".to_string(), "datasets/".to_string()],
        search_surface: vec![
            "train.py".to_string(),
            "config/".to_string(),
            "models/".to_string(),
        ],
        canonical_dataset_ref: test_artifact_hash(20),
        dataset_hash: test_artifact_hash(21),
        dataset_splits: DatasetSplits {
            training: test_artifact_hash(22),
            validation: test_artifact_hash(23),
            test: Some(test_artifact_hash(24)),
        },
        evaluation_harness_ref: test_artifact_hash(30),
        metric_id: "test_accuracy".to_string(),
        metric_direction: MetricDirection::HigherBetter,
        hardware_class: "RTX 4090".to_string(),
        time_budget_secs: 3600,
        seed_environment_manifest_ref: test_artifact_hash(40),
        seed_score: MetricValue::new(0.9300),
        artifact_schema_ref: test_artifact_hash(50),
        seed_bond: TokenAmount::new(1000),
        license_declaration: "MIT".to_string(),
        timestamp: 1700000000,
    }
}

// -----------------------------------------------------------------------
// Invalid genesis blocks (each violates exactly one invariant)
// -----------------------------------------------------------------------

/// Genesis block with empty metric_id.
pub fn invalid_genesis_missing_metric_id() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.metric_id = String::new();
    g
}

/// Genesis block with zero dataset_hash.
pub fn invalid_genesis_missing_dataset_hash() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.dataset_hash = ArtifactHash::ZERO;
    g
}

/// Genesis block with empty search_surface.
pub fn invalid_genesis_empty_search_surface() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.search_surface = vec![];
    g
}

/// Genesis block with empty frozen_surface.
pub fn invalid_genesis_empty_frozen_surface() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.frozen_surface = vec![];
    g
}

/// Genesis block with empty research_target_declaration.
pub fn invalid_genesis_missing_research_target() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.research_target_declaration = String::new();
    g
}

/// Genesis block with NaN seed_score.
pub fn invalid_genesis_nan_seed_score() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.seed_score = MetricValue::new(f64::NAN);
    g
}

/// Genesis block with infinite seed_score.
pub fn invalid_genesis_inf_seed_score() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.seed_score = MetricValue::new(f64::INFINITY);
    g
}

/// Genesis block with zero time_budget_secs.
pub fn invalid_genesis_zero_time_budget() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.time_budget_secs = 0;
    g
}

/// Genesis block with empty hardware_class.
pub fn invalid_genesis_missing_hardware_class() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.hardware_class = String::new();
    g
}

/// Genesis block with zero evaluation_harness_ref.
pub fn invalid_genesis_missing_eval_harness() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.evaluation_harness_ref = ArtifactHash::ZERO;
    g
}

/// Genesis block with zero seed_recipe_ref.
pub fn invalid_genesis_missing_seed_recipe() -> GenesisBlock {
    let mut g = valid_genesis_block();
    g.seed_recipe_ref = ArtifactHash::ZERO;
    g
}

/// Genesis block with a path appearing in both search_surface and
/// frozen_surface.
pub fn invalid_genesis_overlapping_surfaces() -> GenesisBlock {
    let mut g = valid_genesis_block();
    // Add a path from frozen_surface into search_surface.
    g.search_surface.push("eval/".to_string());
    g
}

// -----------------------------------------------------------------------
// Valid block
// -----------------------------------------------------------------------

/// A structurally valid block claiming a metric improvement.
///
/// All artifact references are non-zero and distinct. The claimed
/// delta is a small positive finite value.
pub fn valid_block() -> Block {
    Block {
        id: test_block_id(2),
        domain_id: test_domain_id(1),
        parent_id: test_genesis_block_id(1).as_block_id(),
        proposer: test_proposer_id(2),
        child_state_ref: test_artifact_hash(60),
        diff_ref: test_artifact_hash(61),
        claimed_metric_delta: MetricValue::new(0.015),
        evidence_bundle_hash: test_artifact_hash(62),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: EpochId(1),
        status: BlockStatus::Submitted,
        timestamp: 1700001000,
    }
}

// -----------------------------------------------------------------------
// Invalid blocks (each violates exactly one invariant)
// -----------------------------------------------------------------------

/// Block with zero evidence_bundle_hash.
pub fn invalid_block_missing_evidence() -> Block {
    let mut b = valid_block();
    b.evidence_bundle_hash = ArtifactHash::ZERO;
    b
}

/// Block with zero child_state_ref.
pub fn invalid_block_missing_child_state() -> Block {
    let mut b = valid_block();
    b.child_state_ref = ArtifactHash::ZERO;
    b
}

/// Block with zero diff_ref.
pub fn invalid_block_missing_diff() -> Block {
    let mut b = valid_block();
    b.diff_ref = ArtifactHash::ZERO;
    b
}

/// Block with NaN claimed_metric_delta.
pub fn invalid_block_nan_delta() -> Block {
    let mut b = valid_block();
    b.claimed_metric_delta = MetricValue::new(f64::NAN);
    b
}

/// Block with infinite claimed_metric_delta.
pub fn invalid_block_inf_delta() -> Block {
    let mut b = valid_block();
    b.claimed_metric_delta = MetricValue::new(f64::NEG_INFINITY);
    b
}

// -----------------------------------------------------------------------
// Valid validation attestation
// -----------------------------------------------------------------------

/// A structurally valid validation attestation with a Pass vote.
///
/// Includes an observed_delta matching the claimed improvement.
pub fn valid_attestation() -> ValidationAttestation {
    ValidationAttestation {
        block_id: test_block_id(2),
        validator: test_validator_id(1),
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.014)),
        replay_evidence_ref: test_artifact_hash(70),
        timestamp: 1700002000,
    }
}

/// Attestation with zero replay_evidence_ref.
pub fn invalid_attestation_missing_evidence() -> ValidationAttestation {
    let mut a = valid_attestation();
    a.replay_evidence_ref = ArtifactHash::ZERO;
    a
}

/// Attestation with NaN observed_delta.
pub fn invalid_attestation_nan_observed_delta() -> ValidationAttestation {
    let mut a = valid_attestation();
    a.observed_delta = Some(MetricValue::new(f64::NAN));
    a
}

// -----------------------------------------------------------------------
// Valid challenge record
// -----------------------------------------------------------------------

/// A structurally valid challenge record (BlockReplay type).
pub fn valid_challenge() -> ChallengeRecord {
    ChallengeRecord {
        id: test_challenge_id(1),
        challenge_type: ChallengeType::BlockReplay,
        target: ChallengeTarget::Block {
            block_id: test_block_id(2),
        },
        challenger: test_participant_id(3),
        bond: TokenAmount::new(250),
        evidence_ref: test_artifact_hash(80),
        status: ChallengeStatus::Open,
        epoch_id: EpochId(2),
        timestamp: 1700003000,
    }
}

/// Challenge with zero evidence_ref.
pub fn invalid_challenge_missing_evidence() -> ChallengeRecord {
    let mut c = valid_challenge();
    c.evidence_ref = ArtifactHash::ZERO;
    c
}

// -----------------------------------------------------------------------
// Valid canonical frontier state
// -----------------------------------------------------------------------

/// A structurally valid canonical frontier state.
pub fn valid_frontier_state() -> CanonicalFrontierState {
    CanonicalFrontierState {
        domain_id: test_domain_id(1),
        frontier_block_id: test_block_id(5),
        source_tree_ref: test_artifact_hash(90),
        config_ref: test_artifact_hash(91),
        dependency_manifest_ref: test_artifact_hash(92),
        environment_manifest_ref: test_artifact_hash(93),
        evaluation_manifest_ref: test_artifact_hash(94),
        snapshot_ref: test_artifact_hash(95),
        status: FrontierStatus::Active,
        epoch_id: EpochId(10),
    }
}
