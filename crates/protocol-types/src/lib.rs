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

//! Core protocol types for AutoResearch Chain.
//!
//! This crate defines the foundational data structures used across the protocol.
//! It contains identifiers, blocks, domains, tracks, forks, challenges, rewards,
//! and canonical state references.
//!
//! These are structural definitions only. State transition logic lives in
//! `arc-protocol-rules`. Domain lifecycle lives in `arc-domain-engine`.
//! Fork logic lives in `arc-fork-engine`. And so on.
//!
//! # Implementation status
//!
//! Stub types only. No fields, no serialization, no hashing yet.
//! The type names and module boundaries are drawn from the protocol spec (v0.2)
//! and terminology document to ensure the code matches the protocol language.

// TODO: Decide on canonical hash function (SHA-256, BLAKE3, etc.)
// TODO: Add serde derives once canonical serialization format is chosen.
// TODO: Add content-addressing traits once hash model is locked.

// ---------------------------------------------------------------------------
// Identifier and hash primitives
// ---------------------------------------------------------------------------

/// Opaque 32-byte identifier. Placeholder until hash model is decided.
///
/// TODO: Replace with a proper content-addressed hash type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hash32(pub [u8; 32]);

/// Unique identifier for a problem domain.
pub type DomainId = Hash32;

/// Unique identifier for a block within the protocol.
pub type BlockId = Hash32;

/// Unique identifier for a research track (rooted at its genesis block).
pub type TrackId = Hash32;

/// Unique identifier for a fork family within a domain.
pub type ForkFamilyId = Hash32;

/// Unique identifier for a participant (proposer, validator, challenger, governor).
///
/// TODO: Decide on identity/address model.
pub type ParticipantId = Hash32;

/// Unique identifier for an epoch.
pub type EpochId = u64;

// ---------------------------------------------------------------------------
// Domain and track types
// ---------------------------------------------------------------------------

/// A protocol-defined research arena.
///
/// Each domain defines a specific problem participants are trying to improve,
/// with its own codebase root, evaluation logic, fork competition space,
/// canonical frontier, and reward context.
///
/// TODO: Add fields from DomainSpec, parent/child domain references,
///       domain type classification, and lifecycle state.
#[derive(Debug)]
pub struct ProblemDomain {
    pub id: DomainId,
    // TODO: spec, domain_type, parent, children, active track references
}

/// The structural specification of a ProblemDomain.
///
/// Defines codebase root, evaluation targets, metrics, modification surface,
/// epoch policy, fork policy, integration rules, canonicalization behavior,
/// and materialization rules.
///
/// TODO: Add all spec fields from protocol-v0.2.
#[derive(Debug)]
pub struct DomainSpec {
    pub domain_id: DomainId,
    // TODO: codebase_root, evaluation_target, primary_metric, search_surface,
    //       frozen_surface, epoch_policy, fork_policy, materialization_policy
}

/// Descriptive classification of a ProblemDomain.
///
/// Types: root, model, subsystem, technique, infrastructure, integration, experimental.
/// Types may influence default policy but do not override explicit rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainType {
    Root,
    Model,
    Subsystem,
    Technique,
    Infrastructure,
    Integration,
    Experimental,
}

/// Protocol-legible declaration of the intended class of value a domain produces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainIntent {
    pub label: String,
    // TODO: structured classification from protocol spec
}

/// An interface specification defining the minimum shape a research track must
/// satisfy to participate in the protocol.
///
/// The first standard is RTS-1: single-metric fixed-budget, bounded replay,
/// autonomous agent loops, Stage 1 research-discovery.
///
/// TODO: Add RTS version enum, required field declarations, conformance checking trait.
#[derive(Debug)]
pub struct ResearchTrackStandard {
    pub version: RtsVersion,
    // TODO: required fields, constraints, conformance rules
}

/// Version identifier for research track standards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtsVersion {
    /// Single-metric, fixed-budget, bounded single-node replay.
    Rts1,
    // Future: Rts2, Rts3, etc.
}

// ---------------------------------------------------------------------------
// Genesis and track initialization
// ---------------------------------------------------------------------------

/// The root block of a new research track.
///
/// A genesis block is not a claim of improvement — it is a claim that a new
/// research arena is well-defined enough to become a protocol-recognized market.
///
/// TODO: Add all genesis fields: research target declaration, seed recipe,
///       baseline score, dataset references, evaluation harness, search/frozen
///       surface, hardware class, time budget, seed bond.
#[derive(Debug)]
pub struct GenesisBlock {
    pub track_id: TrackId,
    pub proposer: ParticipantId,
    pub rts_version: RtsVersion,
    // TODO: research_target, seed_recipe_ref, baseline_score, dataset_ref,
    //       evaluation_harness_ref, search_surface, frozen_surface,
    //       hardware_class, time_budget, seed_bond
}

/// The lifecycle state of a track being initialized.
///
/// Tracks go through: Proposed -> Validating -> Active | Failed.
///
/// TODO: Add conformance check state, seed reproduction state,
///       activation threshold tracking, challenge state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackInitializationState {
    Proposed,
    Validating,
    Active,
    Failed,
}

/// Track initialization record.
///
/// TODO: Add activation conditions, validator participation tracking,
///       bonded threshold state, challenge records.
#[derive(Debug)]
pub struct TrackInitialization {
    pub track_id: TrackId,
    pub genesis: GenesisBlock,
    pub state: TrackInitializationState,
}

/// The domain-scoped descendant tree rooted at a single genesis block.
///
/// Each TrackTree has its own fork families, validator scope, reward context,
/// canonical frontier, and challenge surface. The chain is a forest of
/// independent domain-rooted TrackTrees.
///
/// TODO: Add tree structure, fork family index, frontier reference,
///       validator pool reference, reward context.
#[derive(Debug)]
pub struct TrackTree {
    pub track_id: TrackId,
    pub domain_id: DomainId,
    // TODO: root_genesis, fork_families, canonical_frontier, reward_context
}

// ---------------------------------------------------------------------------
// Block types
// ---------------------------------------------------------------------------

/// A protocol epoch specification.
///
/// Defines the rules of a research game during a fixed interval: datasets,
/// metrics, environment requirements, compute policies, thresholds, reward
/// parameters, and challenge windows.
///
/// TODO: Add all epoch fields.
#[derive(Debug)]
pub struct EpochSpec {
    pub epoch_id: EpochId,
    // TODO: datasets, metrics, environment, compute_policy, thresholds,
    //       reward_params, challenge_windows
}

/// A claim that a child training recipe improves on a parent training recipe.
///
/// TODO: Add all block fields: domain reference, parent/child state refs,
///       diff reference, claimed metric delta, evidence bundle hash,
///       proposer identity, fee/bond, epoch reference.
#[derive(Debug)]
pub struct Block {
    pub id: BlockId,
    pub domain_id: DomainId,
    pub parent_id: Option<BlockId>, // None only for genesis blocks
    pub proposer: ParticipantId,
    pub epoch_id: EpochId,
    // TODO: child_state_ref, diff_ref, claimed_metric_delta, evidence_hash,
    //       fee, bond, timestamp
}

// ---------------------------------------------------------------------------
// Validation types
// ---------------------------------------------------------------------------

/// Possible outcomes of a validation replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttestationVote {
    Pass,
    Fail,
    Inconclusive,
    FraudSuspected,
}

/// A signed validator claim about whether a proposed improvement reproduces.
///
/// TODO: Add validator identity, replay metadata, signature, timestamp.
#[derive(Debug)]
pub struct ValidationAttestation {
    pub block_id: BlockId,
    pub validator: ParticipantId,
    pub vote: AttestationVote,
    // TODO: replay_metadata, signature, timestamp
}

// ---------------------------------------------------------------------------
// Fork types
// ---------------------------------------------------------------------------

/// A set of competing branches within a domain that share a common ancestor.
///
/// TODO: Add branch references, dominance state, frontier tracking.
#[derive(Debug)]
pub struct ForkFamily {
    pub id: ForkFamilyId,
    pub domain_id: DomainId,
    // TODO: common_ancestor, branches, dominant_branch, frontier_block
}

// ---------------------------------------------------------------------------
// Challenge types
// ---------------------------------------------------------------------------

/// Categories of challenge the protocol supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChallengeType {
    /// Challenge against a block's claimed metric delta.
    BlockReplay,
    /// Challenge against a validator's attestation.
    AttestationFraud,
    /// Challenge against an attribution claim.
    Attribution,
    /// Challenge against a fork dominance declaration.
    Dominance,
    /// Challenge against a genesis proposal's metric adequacy.
    MetricAdequacy,
}

/// A bonded dispute object in the protocol.
///
/// TODO: Add target reference, challenger identity, bond, evidence,
///       resolution state, remedy.
#[derive(Debug)]
pub struct ChallengeRecord {
    pub challenge_type: ChallengeType,
    pub challenger: ParticipantId,
    // TODO: target_ref, bond, evidence_ref, state, resolution, remedy
}

// ---------------------------------------------------------------------------
// Reward and escrow types
// ---------------------------------------------------------------------------

/// Temporary holding of rewards pending challenge-window expiration.
///
/// TODO: Add amount, beneficiary, release conditions, slashing conditions.
#[derive(Debug)]
pub struct EscrowRecord {
    pub block_id: BlockId,
    pub beneficiary: ParticipantId,
    // TODO: amount, release_epoch, slashing_conditions, state
}

/// Attribution claim types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributionType {
    /// Credit for first validated appearance of a useful idea.
    Origin,
    /// Credit for porting a useful idea into a stronger branch.
    Integration,
    /// Credit for moving the best validated frontier forward.
    Frontier,
}

/// A claim of credit for a contribution.
///
/// TODO: Add source references, validation state, reward fraction.
#[derive(Debug)]
pub struct AttributionClaim {
    pub attribution_type: AttributionType,
    pub claimant: ParticipantId,
    pub block_id: BlockId,
    // TODO: source_refs, validation_state, reward_fraction
}

// ---------------------------------------------------------------------------
// Canonical state types
// ---------------------------------------------------------------------------

/// The protocol-recognized best assembled state of a ProblemDomain.
///
/// Includes or resolves to the dominant frontier block, full source tree,
/// resolved configuration, dependency manifest, environment manifest,
/// evaluation manifest, and content-addressed snapshot reference.
///
/// This is what participants pull to begin new work.
///
/// TODO: Add all frontier state fields and resolution logic.
#[derive(Debug)]
pub struct CanonicalFrontierState {
    pub domain_id: DomainId,
    pub frontier_block_id: BlockId,
    // TODO: source_tree_ref, config_ref, dependency_manifest_ref,
    //       environment_manifest_ref, evaluation_manifest_ref, snapshot_ref
}

/// A full assembled working snapshot of a domain's codebase and execution context.
///
/// Distinguished from a BlockDiff (incremental change). Content-addressed and
/// publicly fetchable. Required at fork dominance transitions, scheduled
/// checkpoints, or when diff chains exceed policy thresholds.
///
/// TODO: Add content-addressed reference, assembly metadata.
#[derive(Debug)]
pub struct MaterializedState {
    pub domain_id: DomainId,
    // TODO: content_hash, assembly_metadata, block_id, timestamp
}

/// A protocol-resolvable reference to a full assembled codebase state.
///
/// Resolves to a CanonicalFrontierState or a specific historical MaterializedState.
///
/// TODO: Add resolution variants and lookup logic.
#[derive(Debug)]
pub struct CodebaseStateRef {
    pub domain_id: DomainId,
    // TODO: resolution target (latest frontier vs. specific historical state)
}

// ---------------------------------------------------------------------------
// Integrity policy types
// ---------------------------------------------------------------------------

/// Per-track policy for evaluation metric integrity.
///
/// Immutable metric declaration, immutable direction, frozen evaluation harness,
/// search/frozen surface separation, replay requirements, tolerance rules.
///
/// TODO: Add all policy fields.
#[derive(Debug)]
pub struct MetricIntegrityPolicy {
    pub track_id: TrackId,
    // TODO: metric_name, direction, tolerance, harness_ref, replay_requirements
}

/// Per-track policy for dataset integrity.
///
/// Canonical reference, content-addressed identity, split declaration,
/// availability requirements, preprocessing rules, license status.
///
/// TODO: Add all policy fields.
#[derive(Debug)]
pub struct DatasetIntegrityPolicy {
    pub track_id: TrackId,
    // TODO: dataset_ref, content_hash, splits, availability, preprocessing, license
}

// ---------------------------------------------------------------------------
// Evidence types
// ---------------------------------------------------------------------------

/// The complete public set of artifacts required to replay and verify a block.
///
/// TODO: Add all evidence bundle fields: code diff, config, environment manifest,
///       dataset references, evaluation procedure, training budget, seeds,
///       logs, metric outputs, artifact hashes.
#[derive(Debug)]
pub struct EvidenceBundle {
    pub block_id: BlockId,
    // TODO: diff_ref, config_ref, env_manifest_ref, dataset_refs,
    //       eval_procedure_ref, training_budget, seeds, log_ref, metric_outputs
}

// ---------------------------------------------------------------------------
// Cross-domain types
// ---------------------------------------------------------------------------

/// A block that ports an improvement from one domain into another.
///
/// TODO: Add source domain, source artifacts, destination domain,
///       validation under destination rules.
#[derive(Debug)]
pub struct CrossDomainIntegrationBlock {
    pub id: BlockId,
    pub source_domain_id: DomainId,
    pub destination_domain_id: DomainId,
    // TODO: source_artifact_refs, destination_block, validation_state
}
