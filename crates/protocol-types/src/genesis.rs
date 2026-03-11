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

//! Genesis block, research track standards, track initialization, and track
//! tree types.

use serde::{Deserialize, Serialize};

use crate::enums::{
    DomainIntent, MetricDirection, ResearchTrackStandardVersion, TrackActivationState,
};
use crate::ids::{
    ArtifactHash, BlockId, DomainId, ForkFamilyId, GenesisBlockId, ProposerId, TrackTreeId,
};
use crate::metric::MetricValue;
use crate::token::TokenAmount;

/// An interface specification defining the minimum structure a research track
/// must satisfy to participate in the protocol.
///
/// The standard is a schema, not a data instance. Genesis blocks declare which
/// RTS version they conform to, and the domain-engine checks conformance.
///
/// Currently only RTS-1 is defined: single-metric, fixed-budget, bounded
/// single-node replay, autonomous agent loops.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchTrackStandard {
    /// Which version of the standard.
    pub version: ResearchTrackStandardVersion,
    /// Human-readable description of what this standard requires.
    pub description: String,
}

/// The root block of a new research track.
///
/// A genesis block is not a claim of improvement --- it is a claim that a new
/// research arena is well-defined enough to become a protocol-recognized market.
/// It must fully declare the research problem, baseline, evaluation method,
/// and economic parameters.
///
/// Note: `PartialEq` without `Eq` because `seed_score` uses [`MetricValue`],
/// which wraps `f64` internally.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenesisBlock {
    /// Unique genesis block identifier.
    pub id: GenesisBlockId,
    /// Research track standard this genesis conforms to.
    pub rts_version: ResearchTrackStandardVersion,
    /// The domain this track belongs to.
    pub domain_id: DomainId,
    /// Who proposed this genesis block.
    pub proposer: ProposerId,
    /// Human-readable declaration of the research target.
    pub research_target_declaration: String,
    /// Intended class of value this domain produces.
    pub domain_intent: DomainIntent,

    // --- Codebase and evaluation references ---
    /// Reference to the initial baseline recipe.
    pub seed_recipe_ref: ArtifactHash,
    /// Reference to the initial codebase state.
    pub seed_codebase_state_ref: ArtifactHash,
    /// Files and modules that must remain fixed.
    pub frozen_surface: Vec<String>,
    /// Files and modules that participants may modify.
    pub search_surface: Vec<String>,

    // --- Dataset ---
    /// Reference to the canonical dataset.
    pub canonical_dataset_ref: ArtifactHash,
    /// Content-addressed hash of the dataset.
    pub dataset_hash: ArtifactHash,
    /// Train/validation/test split declarations.
    pub dataset_splits: DatasetSplits,

    // --- Evaluation ---
    /// Reference to the frozen evaluation harness.
    pub evaluation_harness_ref: ArtifactHash,
    /// Evaluation metric identifier (e.g., "test_accuracy").
    pub metric_id: String,
    /// Direction of metric optimization.
    pub metric_direction: MetricDirection,

    // --- Environment and budget ---
    /// Hardware requirements description.
    ///
    /// Placeholder: will become a structured type in a future phase.
    pub hardware_class: String,
    /// Wall-clock or compute time budget in seconds.
    pub time_budget_secs: u64,
    /// Reference to the seed environment manifest.
    pub seed_environment_manifest_ref: ArtifactHash,

    // --- Baseline ---
    /// Baseline score achieved by the seed recipe.
    pub seed_score: MetricValue,

    // --- Artifacts and economics ---
    /// Reference to the required submission artifact schema.
    pub artifact_schema_ref: ArtifactHash,
    /// Economic bond posted by the proposer.
    pub seed_bond: TokenAmount,
    /// License status declaration for dataset and assets.
    pub license_declaration: String,

    // --- Metadata ---
    /// Unix timestamp of genesis proposal.
    pub timestamp: u64,
}

/// Declaration of dataset partitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetSplits {
    /// Content-addressed reference to the training partition.
    pub training: ArtifactHash,
    /// Content-addressed reference to the validation partition.
    pub validation: ArtifactHash,
    /// Content-addressed reference to the test partition, if declared.
    pub test: Option<ArtifactHash>,
}

/// Track initialization record.
///
/// Tracks the lifecycle of a genesis block's activation process. A track is
/// not active until it passes all activation conditions: RTS conformance,
/// seed reproducibility, minimum validator participation, and survival of
/// any challenges.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackInitialization {
    /// The genesis block being activated.
    pub genesis_block_id: GenesisBlockId,
    /// The domain this track belongs to.
    pub domain_id: DomainId,
    /// Current activation state.
    pub state: TrackActivationState,
    /// Who proposed this track.
    pub proposer: ProposerId,
    /// Unix timestamp of initialization start.
    pub timestamp: u64,
}

/// A domain-scoped descendant tree rooted at a single genesis block.
///
/// Each track tree has its own fork families, validator sampling scope,
/// reward accounting context, canonical frontier state, and challenge surface.
/// The chain is a forest of independent track trees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackTree {
    /// Unique track tree identifier (derived from the genesis block).
    pub id: TrackTreeId,
    /// The domain this tree belongs to.
    pub domain_id: DomainId,
    /// The genesis block at the root of this tree.
    pub genesis_block_id: GenesisBlockId,
    /// Active fork families within this tree.
    pub fork_families: Vec<ForkFamilyId>,
    /// Current canonical frontier block, if one has been established.
    pub canonical_frontier_block_id: Option<BlockId>,
}
