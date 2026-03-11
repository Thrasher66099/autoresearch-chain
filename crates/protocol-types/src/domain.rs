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

//! Problem domain and domain specification types.

use serde::{Deserialize, Serialize};

use crate::enums::{DomainType, MaterializationPolicyKind, MetricDirection};
use crate::ids::{ArtifactHash, DomainId, DomainSpecId};

/// A protocol-defined research arena.
///
/// Each domain defines a specific problem participants are trying to improve,
/// with its own evaluation logic, fork competition space, canonical frontier,
/// and reward context. Domains may form hierarchies through parent references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDomain {
    /// Unique domain identifier.
    pub id: DomainId,
    /// Human-readable domain name.
    pub name: String,
    /// Classification of this domain's purpose.
    pub domain_type: DomainType,
    /// Parent domain, if this domain is a sub-problem.
    pub parent_domain_id: Option<DomainId>,
    /// Reference to the domain's structural specification.
    pub spec_id: DomainSpecId,
}

/// The structural specification of a problem domain.
///
/// Defines the technical parameters under which research in this domain
/// operates: codebase references, metrics, modification surfaces, hardware
/// requirements, and materialization policy.
///
/// The domain owns its name and classification; the spec owns the technical
/// parameters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainSpec {
    /// Unique specification identifier.
    pub id: DomainSpecId,
    /// The domain this specification belongs to.
    pub domain_id: DomainId,
    /// Reference to the domain's base codebase.
    pub base_codebase_ref: ArtifactHash,
    /// Primary optimization metric name.
    pub primary_metric: String,
    /// Direction of primary metric optimization.
    pub metric_direction: MetricDirection,
    /// Additional tracked metrics (non-primary).
    pub secondary_metrics: Vec<String>,
    /// Files and modules that participants may modify.
    pub search_surface: Vec<String>,
    /// Files and modules that must remain fixed.
    pub frozen_surface: Vec<String>,
    /// Reference to the required submission artifact schema.
    pub artifact_schema_ref: ArtifactHash,
    /// Hardware tier description (e.g., "RTX 4090", "A100 80GB").
    ///
    /// Placeholder: will become a structured type in a future phase.
    pub hardware_class: String,
    /// When materialized state snapshots should be created.
    pub materialization_policy: MaterializationPolicyKind,
}
