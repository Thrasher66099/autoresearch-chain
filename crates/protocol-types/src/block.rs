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

//! Block and epoch types.

use serde::{Deserialize, Serialize};

use crate::enums::BlockStatus;
use crate::ids::{ArtifactHash, BlockId, DomainId, EpochId, ProposerId};

/// A claim that a child training recipe improves on a parent training recipe.
///
/// Blocks are the fundamental unit of research progress in the protocol.
/// Each block references its parent state, proposes a diff, claims a metric
/// improvement, and includes an evidence bundle hash for validators to replay.
///
/// Note: `PartialEq` without `Eq` because `claimed_metric_delta` is `f64`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Unique block identifier.
    pub id: BlockId,
    /// The domain this block targets.
    pub domain_id: DomainId,
    /// Parent block. For the first block in a track, this is the genesis
    /// block's ID (via [`GenesisBlockId::as_block_id`]).
    pub parent_id: BlockId,
    /// Who proposed this block.
    pub proposer: ProposerId,
    /// Reference to the proposed new recipe/codebase state.
    pub child_state_ref: ArtifactHash,
    /// Reference to the code diff (parent to child).
    pub diff_ref: ArtifactHash,
    /// Claimed metric improvement over parent.
    ///
    /// Uses `f64` as a placeholder. A production implementation should use a
    /// deterministic numeric representation.
    pub claimed_metric_delta: f64,
    /// Hash of the full evidence bundle for replay verification.
    pub evidence_bundle_hash: ArtifactHash,
    /// Submission fee.
    ///
    /// Placeholder: will use a proper token/amount type in a future phase.
    pub fee: u64,
    /// Slashable bond posted by the proposer.
    ///
    /// Placeholder: will use a proper token/amount type in a future phase.
    pub bond: u64,
    /// Protocol epoch at time of submission.
    pub epoch_id: EpochId,
    /// Current lifecycle status.
    pub status: BlockStatus,
    /// Unix timestamp of submission.
    pub timestamp: u64,
}

/// Protocol epoch specification.
///
/// Defines the rules of a research game during a fixed interval: challenge
/// window duration, validation quorum requirements, and (eventually) active
/// datasets, metric definitions, environment requirements, compute policies,
/// and reward parameters.
///
/// Most fields are placeholders for Phase 0.1. The epoch structure will be
/// fleshed out when the state machine is implemented.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochSpec {
    /// Sequential epoch identifier.
    pub epoch_id: EpochId,
    /// Duration of the challenge window in epochs.
    pub challenge_window_epochs: u64,
    /// Minimum number of Pass votes required for block acceptance.
    pub validation_quorum: u32,
}
