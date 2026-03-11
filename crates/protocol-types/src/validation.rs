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

//! Validation attestation and evidence bundle types.

use serde::{Deserialize, Serialize};

use crate::enums::ValidatorVote;
use crate::ids::{ArtifactHash, BlockId, ValidatorId};

/// A signed validator claim about whether a proposed improvement reproduces.
///
/// Validators replay the proposer's claimed improvement using the evidence
/// bundle and cast a vote on whether the metric delta holds within tolerance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationAttestation {
    /// The block being validated.
    pub block_id: BlockId,
    /// The validator casting this attestation.
    pub validator: ValidatorId,
    /// The validator's verdict.
    pub vote: ValidatorVote,
    /// Reference to the validator's replay evidence (logs, outputs).
    pub replay_evidence_ref: ArtifactHash,
    /// Unix timestamp of attestation.
    pub timestamp: u64,
}

/// The complete public set of artifacts required to replay and verify a block.
///
/// Everything in an evidence bundle must be publicly retrievable. If a claim
/// cannot be fetched, replayed, or challenged, it cannot be trusted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// The block this evidence supports.
    pub block_id: BlockId,
    /// Reference to the code diff (parent to child).
    pub diff_ref: ArtifactHash,
    /// Reference to the fully resolved configuration.
    pub config_ref: ArtifactHash,
    /// Reference to the environment manifest (dependencies, versions, hardware).
    pub environment_manifest_ref: ArtifactHash,
    /// References to dataset partitions used.
    pub dataset_refs: Vec<ArtifactHash>,
    /// Reference to the evaluation procedure specification.
    pub evaluation_procedure_ref: ArtifactHash,
    /// Reference to canonical training logs.
    pub training_log_ref: ArtifactHash,
    /// Reference to metric output artifacts.
    pub metric_output_ref: ArtifactHash,
}
