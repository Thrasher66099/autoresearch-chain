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

//! Metric and dataset integrity policy types.

use serde::{Deserialize, Serialize};

use crate::enums::MetricDirection;
use crate::genesis::DatasetSplits;
use crate::ids::{ArtifactHash, GenesisBlockId};

/// Per-track policy defining evaluation metric integrity requirements.
///
/// Declared immutably at genesis. The metric, its direction, the evaluation
/// harness, and the search/frozen surface separation cannot be changed after
/// track activation. If the metric is found flawed, a successor track must
/// be created rather than silently mutating the existing one.
///
/// Note: `PartialEq` without `Eq` because `tolerance` is `f64`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricIntegrityPolicy {
    /// The track this policy belongs to (identified by genesis block).
    pub track_id: GenesisBlockId,
    /// Evaluation metric identifier.
    pub metric_id: String,
    /// Direction of metric optimization.
    pub metric_direction: MetricDirection,
    /// Reference to the frozen evaluation harness.
    pub evaluation_harness_ref: ArtifactHash,
    /// Acceptable tolerance for metric reproduction.
    ///
    /// Uses `f64` as a placeholder. A production implementation should use a
    /// deterministic numeric representation.
    pub tolerance: f64,
    /// Maximum wall-clock seconds allowed for a single replay.
    pub max_replay_budget_secs: u64,
}

/// Per-track policy defining dataset integrity requirements.
///
/// Declared at genesis. The canonical dataset, its content hash, and the
/// split declarations are immutable for the lifetime of the track.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetIntegrityPolicy {
    /// The track this policy belongs to (identified by genesis block).
    pub track_id: GenesisBlockId,
    /// Reference to the canonical dataset.
    pub canonical_dataset_ref: ArtifactHash,
    /// Content-addressed hash of the dataset.
    pub dataset_hash: ArtifactHash,
    /// Train/validation/test split declarations.
    pub splits: DatasetSplits,
    /// Dataset availability requirements (human-readable for now).
    pub availability_requirement: String,
    /// License status declaration.
    pub license_declaration: String,
}
