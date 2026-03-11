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

//! Canonical protocol enums.
//!
//! These enums define the fixed vocabularies used throughout the protocol:
//! lifecycle statuses, classifications, vote types, and policy kinds. They are
//! drawn from the protocol specification (v0.2) and terminology document.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Block lifecycle
// ---------------------------------------------------------------------------

/// Lifecycle status of a block in the protocol.
///
/// Blocks progress through: Submitted → UnderValidation → ValidationComplete
/// → UnderChallenge → ChallengeWindowClosed → Settled → Final.
/// A block may be Rejected at any point before settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Block has been submitted with evidence bundle and bond.
    Submitted,
    /// Validator set sampled; replay in progress.
    UnderValidation,
    /// Attestations aggregated; validation phase complete.
    ValidationComplete,
    /// Challenge window is open.
    UnderChallenge,
    /// Challenge window has closed without upheld challenges.
    ChallengeWindowClosed,
    /// Rewards settled and released.
    Settled,
    /// Block is finalized and immutable.
    Final,
    /// Block was rejected (failed validation or upheld challenge).
    Rejected,
}

// ---------------------------------------------------------------------------
// Challenge types and status
// ---------------------------------------------------------------------------

/// Categories of challenge the protocol supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChallengeType {
    /// Challenge against a block's claimed metric delta (replay dispute).
    BlockReplay,
    /// Challenge against a validator's attestation (fraud claim).
    AttestationFraud,
    /// Challenge against an attribution claim.
    Attribution,
    /// Challenge against a fork dominance declaration.
    Dominance,
    /// Challenge against a genesis proposal's metric adequacy.
    MetricAdequacy,
}

/// Lifecycle status of a challenge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChallengeStatus {
    /// Challenge has been opened and bond posted.
    Open,
    /// Challenge is under review (evidence being evaluated).
    UnderReview,
    /// Challenge was upheld; target is invalidated.
    Upheld,
    /// Challenge was rejected; challenger loses bond.
    Rejected,
    /// Challenge expired without resolution.
    Expired,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Outcome of a validation replay.
///
/// Validators replay the proposer's claimed improvement using the evidence
/// bundle and cast one of these votes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidatorVote {
    /// Claimed improvement reproduces within declared tolerance.
    Pass,
    /// Claimed improvement does not reproduce.
    Fail,
    /// Replay produced ambiguous or indeterminate results.
    Inconclusive,
    /// Evidence of fabrication or manipulation detected.
    FraudSuspected,
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Direction of metric optimization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricDirection {
    /// Higher metric values are better (e.g., accuracy).
    HigherBetter,
    /// Lower metric values are better (e.g., loss, latency).
    LowerBetter,
}

// ---------------------------------------------------------------------------
// Domain classification
// ---------------------------------------------------------------------------

/// Descriptive classification of a problem domain.
///
/// Domain type may influence default policy but does not override explicit
/// rules in the domain specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainType {
    /// Top-level domain with no parent.
    Root,
    /// Full model training target.
    Model,
    /// Component of a larger training system.
    Subsystem,
    /// Narrow optimization technique.
    Technique,
    /// Supporting infrastructure (data pipelines, checkpointing).
    Infrastructure,
    /// Domain focused on integrating results from other domains.
    Integration,
    /// Exploratory domain with relaxed validation rules.
    Experimental,
}

/// Protocol-legible declaration of the intended class of value a domain produces.
///
/// Domain intents are declared at genesis and help participants understand
/// the scope and nature of research within a domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainIntent {
    /// End-to-end training recipe improvement.
    EndToEndRecipeImprovement,
    /// Optimization of a subsystem within a larger pipeline.
    SubsystemOptimization,
    /// Research into optimizers or techniques that transfer across tasks.
    TransferableOptimizerResearch,
    /// Infrastructure efficiency improvements.
    InfrastructureEfficiency,
    /// Training efficiency for consumer-grade hardware.
    ConsumerGpuTrainingEfficiency,
}

// ---------------------------------------------------------------------------
// Research track standards
// ---------------------------------------------------------------------------

/// Version identifier for research track standards.
///
/// Each version defines the minimum structure a genesis block must declare
/// and the rules the track must follow. Conformance checking is performed
/// by the `arc-domain-engine` crate, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchTrackStandardVersion {
    /// RTS-1: Single-metric, fixed-budget, bounded single-node replay.
    /// This is the only standard defined for Stage 1.
    Rts1,
}

// ---------------------------------------------------------------------------
// Track activation
// ---------------------------------------------------------------------------

/// Lifecycle state of a track being activated.
///
/// Tracks go through a multi-step activation process after a genesis block
/// is proposed. The detailed states here extend the simpler
/// Proposed -> Validating -> Active | Failed model from the spec overview.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrackActivationState {
    /// Genesis block has been submitted.
    Proposed,
    /// RTS conformance is being checked.
    ConformanceChecking,
    /// Seed score reproduction is in progress.
    ValidationInProgress,
    /// All checks passed; awaiting activation thresholds (bonds, validators).
    ActivationPending,
    /// Track is active and accepting blocks.
    Active,
    /// Activation failed (unreproducible seed, upheld challenge, etc.).
    Failed,
    /// Proposal window closed without reaching activation thresholds.
    Expired,
}

// ---------------------------------------------------------------------------
// Frontier status
// ---------------------------------------------------------------------------

/// Status of a canonical frontier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrontierStatus {
    /// Frontier is active and accepting new work.
    Active,
    /// Frontier is contested by competing forks.
    Contested,
    /// Frontier has settled after fork competition resolved.
    Settled,
    /// Frontier has been superseded by a successor track or migration.
    Superseded,
}

// ---------------------------------------------------------------------------
// Materialization policy
// ---------------------------------------------------------------------------

/// Policy kinds for when materialized state snapshots should be created.
///
/// Materialization is expensive, so the protocol defines trigger conditions
/// rather than materializing after every block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterializationPolicyKind {
    /// Materialize when a fork becomes dominant.
    OnDominance,
    /// Materialize at scheduled epoch boundaries.
    Scheduled,
    /// Materialize when the diff chain exceeds a length threshold.
    DiffChainThreshold,
    /// Materialize only on explicit request.
    Manual,
}

// ---------------------------------------------------------------------------
// Attribution and escrow
// ---------------------------------------------------------------------------

/// Attribution claim types for reward distribution.
///
/// Attribution determines how rewards are split among contributors along
/// the ancestry chain of a validated improvement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributionType {
    /// Credit for first validated appearance of a useful idea.
    Origin,
    /// Credit for successfully porting/combining an idea into a stronger branch.
    Integration,
    /// Credit for moving the best validated frontier forward.
    Frontier,
}

/// Status of an escrow record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EscrowStatus {
    /// Funds are held pending challenge window expiration.
    Held,
    /// Funds have been released to the beneficiary.
    Released,
    /// Funds have been slashed due to upheld challenge.
    Slashed,
}
