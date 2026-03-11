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

//! Canonical frontier state, materialized state, and codebase state references.

use serde::{Deserialize, Serialize};

use crate::enums::FrontierStatus;
use crate::ids::{ArtifactHash, BlockId, DomainId, EpochId, MaterializedStateId};

/// The protocol-recognized best assembled state of a problem domain.
///
/// This is what participants pull to begin new work. It includes or resolves
/// to the dominant frontier block, full source tree, resolved configuration,
/// dependency manifest, environment manifest, and evaluation manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalFrontierState {
    /// The domain this frontier belongs to.
    pub domain_id: DomainId,
    /// The block at the tip of the dominant frontier.
    pub frontier_block_id: BlockId,
    /// Content-addressed reference to the full assembled source tree.
    pub source_tree_ref: ArtifactHash,
    /// Reference to the resolved configuration set.
    pub config_ref: ArtifactHash,
    /// Reference to the resolved dependency manifest.
    pub dependency_manifest_ref: ArtifactHash,
    /// Reference to the environment manifest.
    pub environment_manifest_ref: ArtifactHash,
    /// Reference to the evaluation manifest.
    pub evaluation_manifest_ref: ArtifactHash,
    /// Content-addressed snapshot reference for the complete state.
    pub snapshot_ref: ArtifactHash,
    /// Current frontier status.
    pub status: FrontierStatus,
    /// Epoch at which this frontier was established.
    pub epoch_id: EpochId,
}

/// A full assembled working snapshot of a domain's codebase and execution context.
///
/// Distinguished from a block diff (incremental change). A materialized state
/// is content-addressed and publicly fetchable. Materialization occurs at fork
/// dominance transitions, scheduled checkpoints, or when diff chains exceed
/// policy thresholds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedState {
    /// Unique materialized state identifier.
    pub id: MaterializedStateId,
    /// The domain this state belongs to.
    pub domain_id: DomainId,
    /// Content-addressed hash of the full source tree.
    pub root_tree_hash: ArtifactHash,
    /// Hash of the resolved dependency manifest.
    pub resolved_dependency_manifest_hash: ArtifactHash,
    /// Hash of the resolved configuration.
    pub resolved_config_hash: ArtifactHash,
    /// Hash of the environment specification.
    pub environment_manifest_hash: ArtifactHash,
    /// Hash of the evaluation specification.
    pub evaluation_manifest_hash: ArtifactHash,
    /// The block from which this state was materialized.
    pub materialized_from_block_id: BlockId,
    /// Unix timestamp of materialization.
    pub timestamp: u64,
}

/// A protocol-resolvable reference to a full assembled codebase state.
///
/// This is how external consumers (proposers, validators, autonomous agents)
/// request a pullable codebase. It resolves to either the current canonical
/// frontier or a specific historical snapshot.
///
/// Modeled as an enum rather than a struct because the spec defines distinct
/// resolution modes with different semantics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodebaseStateRef {
    /// Resolve to the current canonical frontier state for a domain.
    LatestFrontier {
        /// The domain whose frontier to resolve.
        domain_id: DomainId,
    },
    /// Resolve to a specific historical materialized state.
    Historical {
        /// The specific materialized state to resolve.
        materialized_state_id: MaterializedStateId,
    },
    /// Resolve to the state at a specific block.
    AtBlock {
        /// The block whose state to resolve.
        block_id: BlockId,
    },
}
