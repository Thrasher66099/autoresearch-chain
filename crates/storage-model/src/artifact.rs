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

//! Artifact classification and metadata.
//!
//! Every stored artifact has a kind (what role it plays in the protocol)
//! and metadata (size, timestamp, kind). These are annotations stored
//! alongside the content — they do not affect the content-addressed hash,
//! which is derived purely from content bytes.

use serde::{Deserialize, Serialize};

use arc_protocol_types::ArtifactHash;

/// Classification of what an artifact represents in the protocol.
///
/// Artifact kind is metadata — it does not affect the content-addressed
/// hash. Two artifacts of different kinds but identical content bytes
/// produce the same hash.
///
/// These kinds correspond to the reference fields used throughout the
/// protocol types: blocks carry `diff_ref` and `evidence_bundle_hash`,
/// genesis blocks carry `seed_recipe_ref` and `evaluation_harness_ref`,
/// frontier states carry `source_tree_ref` and `config_ref`, etc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Code diff from parent state to child state.
    ///
    /// Referenced by [`Block::diff_ref`].
    ///
    /// [`Block::diff_ref`]: arc_protocol_types::Block::diff_ref
    Diff,
    /// Complete evidence bundle for block replay verification.
    ///
    /// Referenced by [`Block::evidence_bundle_hash`].
    ///
    /// [`Block::evidence_bundle_hash`]: arc_protocol_types::Block::evidence_bundle_hash
    EvidenceBundle,
    /// Resolved configuration set.
    Config,
    /// Environment manifest (dependencies, versions, hardware).
    EnvironmentManifest,
    /// Evaluation procedure specification and harness.
    EvaluationManifest,
    /// Full assembled source tree snapshot.
    SourceTree,
    /// Canonical training logs from a run.
    TrainingLog,
    /// Metric output artifacts from evaluation.
    MetricOutput,
    /// Dataset partition reference.
    Dataset,
    /// Full materialized state snapshot.
    Snapshot,
    /// Seed recipe reference from genesis.
    SeedRecipe,
    /// Seed codebase state from genesis.
    SeedCodebase,
    /// Artifact schema definition.
    ArtifactSchema,
    /// Validator replay evidence.
    ReplayEvidence,
    /// Challenge evidence submitted by a challenger.
    ChallengeEvidence,
    /// Dependency manifest.
    DependencyManifest,
    /// Proposed child state from a block.
    ChildState,
}

/// Metadata about a stored artifact.
///
/// Stored alongside the content bytes, keyed by the content-addressed
/// hash. Metadata is annotation — it does not affect the hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// The content-addressed hash (identity) of this artifact.
    pub hash: ArtifactHash,
    /// What kind of artifact this is.
    pub kind: ArtifactKind,
    /// Size of the content in bytes.
    pub size_bytes: u64,
    /// Unix timestamp when the artifact was first stored.
    pub stored_at: u64,
}
