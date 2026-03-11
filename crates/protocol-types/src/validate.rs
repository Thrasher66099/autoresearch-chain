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

//! Lightweight structural validation helpers.
//!
//! These functions check that protocol type instances are structurally
//! well-formed: required fields are populated, artifact references are
//! non-zero, numeric fields are finite, and surface declarations are
//! non-empty and non-overlapping.
//!
//! These are NOT protocol-level rules. State transitions, challenge
//! resolution, track activation, and referential integrity belong in
//! `arc-protocol-rules`. This module catches malformed data before it
//! reaches the state machine.

use crate::block::Block;
use crate::challenge::ChallengeRecord;
use crate::genesis::GenesisBlock;
use crate::ids::ArtifactHash;
use crate::validation::ValidationAttestation;

/// A structural validation failure.
///
/// Reports which field failed and why. These are structural issues
/// (empty strings, zero hashes, non-finite floats, surface overlaps),
/// not protocol rule violations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralError {
    /// The field that failed validation.
    pub field: &'static str,
    /// Why it failed.
    pub reason: &'static str,
}

impl std::fmt::Display for StructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.reason)
    }
}

/// Validate structural invariants of a genesis block.
///
/// Checks that all required fields are populated and semantically
/// valid for an RTS-1 genesis block. Does NOT check protocol-level
/// rules (seed reproducibility, RTS conformance, bond adequacy, etc.).
pub fn validate_genesis_block_structure(
    g: &GenesisBlock,
) -> Result<(), Vec<StructuralError>> {
    let mut errors = Vec::new();

    // --- String fields must be non-empty ---

    if g.research_target_declaration.is_empty() {
        errors.push(StructuralError {
            field: "research_target_declaration",
            reason: "must not be empty",
        });
    }
    if g.metric_id.is_empty() {
        errors.push(StructuralError {
            field: "metric_id",
            reason: "must not be empty",
        });
    }
    if g.hardware_class.is_empty() {
        errors.push(StructuralError {
            field: "hardware_class",
            reason: "must not be empty",
        });
    }

    // --- Surface declarations must be non-empty ---

    if g.search_surface.is_empty() {
        errors.push(StructuralError {
            field: "search_surface",
            reason: "must contain at least one modifiable path",
        });
    }
    if g.frozen_surface.is_empty() {
        errors.push(StructuralError {
            field: "frozen_surface",
            reason: "must contain at least one frozen path",
        });
    }

    // --- Search and frozen surfaces must not overlap ---
    // A path appearing in both surfaces is ambiguous: is it modifiable
    // or frozen? This must be caught before any state machine logic
    // relies on the surface separation.

    if !g.search_surface.is_empty() && !g.frozen_surface.is_empty() {
        for path in &g.search_surface {
            if g.frozen_surface.contains(path) {
                errors.push(StructuralError {
                    field: "search_surface",
                    reason: "contains path also present in frozen_surface",
                });
                break;
            }
        }
    }

    // --- Artifact references must be non-zero ---
    // ZERO is a test sentinel, not a valid content-addressed hash.

    if g.dataset_hash == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "dataset_hash",
            reason: "must not be zero hash",
        });
    }
    if g.canonical_dataset_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "canonical_dataset_ref",
            reason: "must not be zero hash",
        });
    }
    if g.seed_recipe_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "seed_recipe_ref",
            reason: "must not be zero hash",
        });
    }
    if g.seed_codebase_state_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "seed_codebase_state_ref",
            reason: "must not be zero hash",
        });
    }
    if g.evaluation_harness_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "evaluation_harness_ref",
            reason: "must not be zero hash",
        });
    }
    if g.seed_environment_manifest_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "seed_environment_manifest_ref",
            reason: "must not be zero hash",
        });
    }
    if g.artifact_schema_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "artifact_schema_ref",
            reason: "must not be zero hash",
        });
    }

    // --- Numeric invariants ---

    if !g.seed_score.is_finite() {
        errors.push(StructuralError {
            field: "seed_score",
            reason: "must be finite (not NaN or infinite)",
        });
    }
    if g.time_budget_secs == 0 {
        errors.push(StructuralError {
            field: "time_budget_secs",
            reason: "must be positive",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate structural invariants of a block.
///
/// Checks that artifact references are populated and the claimed
/// metric delta is finite. Does NOT check referential integrity
/// (parent exists, domain exists) or protocol state transitions.
pub fn validate_block_structure(
    b: &Block,
) -> Result<(), Vec<StructuralError>> {
    let mut errors = Vec::new();

    // --- Artifact references must be non-zero ---

    if b.evidence_bundle_hash == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "evidence_bundle_hash",
            reason: "must not be zero hash",
        });
    }
    if b.child_state_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "child_state_ref",
            reason: "must not be zero hash",
        });
    }
    if b.diff_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "diff_ref",
            reason: "must not be zero hash",
        });
    }

    // --- Numeric invariants ---

    if !b.claimed_metric_delta.is_finite() {
        errors.push(StructuralError {
            field: "claimed_metric_delta",
            reason: "must be finite (not NaN or infinite)",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate structural invariants of a validation attestation.
///
/// Checks that the replay evidence reference is populated and that
/// any observed delta is finite. Does NOT check whether the block or
/// validator exist in protocol state.
pub fn validate_attestation_structure(
    a: &ValidationAttestation,
) -> Result<(), Vec<StructuralError>> {
    let mut errors = Vec::new();

    if a.replay_evidence_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "replay_evidence_ref",
            reason: "must not be zero hash",
        });
    }

    if let Some(delta) = a.observed_delta {
        if !delta.is_finite() {
            errors.push(StructuralError {
                field: "observed_delta",
                reason: "must be finite (not NaN or infinite)",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate structural invariants of a challenge record.
///
/// Checks that the evidence reference is populated. Does NOT check
/// whether the target exists or whether the challenge type is
/// applicable to the target.
pub fn validate_challenge_structure(
    c: &ChallengeRecord,
) -> Result<(), Vec<StructuralError>> {
    let mut errors = Vec::new();

    if c.evidence_ref == ArtifactHash::ZERO {
        errors.push(StructuralError {
            field: "evidence_ref",
            reason: "must not be zero hash",
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
