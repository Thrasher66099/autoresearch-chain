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

//! Fork families, dominance evaluation, and frontier selection
//! for AutoResearch Chain.
//!
//! # Phase 0.3 implementation
//!
//! - Fork family creation when sibling accepted blocks share a parent
//! - Branch tip tracking
//! - Dominance evaluation using validated metrics (not claimed deltas)
//! - Canonical frontier update logic using validated outcomes
//! - Block invalidation handling: removal from branch tips, frontier,
//!   and dominance when a block is proven invalid by upheld challenge
//! - Frontier recomputation after invalidation

use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use arc_protocol_types::{
    Block, BlockId, BlockStatus, DomainId, ForkFamily, ForkFamilyId, MetricValue, TrackTreeId,
};

/// Errors from fork-engine operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForkError {
    /// Fork family not found.
    FamilyNotFound { family_id: ForkFamilyId },
    /// Block is not in an accepted state.
    BlockNotAccepted { block_id: BlockId, status: BlockStatus },
    /// Blocks do not share the same parent.
    ParentMismatch { block_a: BlockId, block_b: BlockId },
    /// Domain mismatch between block and fork family.
    DomainMismatch { block_domain: DomainId, family_domain: DomainId },
}

impl std::fmt::Display for ForkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FamilyNotFound { family_id } => {
                write!(f, "fork family {} not found", family_id)
            }
            Self::BlockNotAccepted { block_id, status } => {
                write!(f, "block {} is {:?}, not accepted", block_id, status)
            }
            Self::ParentMismatch { block_a, block_b } => {
                write!(f, "blocks {} and {} do not share a parent", block_a, block_b)
            }
            Self::DomainMismatch { block_domain, family_domain } => {
                write!(
                    f,
                    "block domain {} != family domain {}",
                    block_domain, family_domain
                )
            }
        }
    }
}

/// Domain-local fork state tracker.
///
/// Manages fork families and frontier candidates for a single domain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainForkState {
    pub domain_id: DomainId,
    pub track_tree_id: TrackTreeId,
    /// Fork families indexed by their ID.
    pub families: HashMap<ForkFamilyId, ForkFamily>,
    /// Map from parent block ID to the fork family that emerged from it.
    pub parent_to_family: HashMap<BlockId, ForkFamilyId>,
    /// Current canonical frontier block (best valid accepted block).
    pub canonical_frontier: Option<BlockId>,
    /// Best known validated metric value at the frontier.
    pub frontier_metric: Option<MetricValue>,
}

impl DomainForkState {
    pub fn new(domain_id: DomainId, track_tree_id: TrackTreeId) -> Self {
        Self {
            domain_id,
            track_tree_id,
            families: HashMap::new(),
            parent_to_family: HashMap::new(),
            canonical_frontier: None,
            frontier_metric: None,
        }
    }

    /// Record an accepted block. If it creates a fork (sibling of an existing
    /// accepted child of the same parent), create or update a ForkFamily.
    ///
    /// Returns the ForkFamilyId if a fork was created or updated, None if
    /// this is the only child of its parent.
    ///
    /// `existing_children` is the list of already-accepted child block IDs
    /// for the same parent. The caller is responsible for tracking this.
    pub fn record_accepted_block(
        &mut self,
        block: &Block,
        existing_siblings: &[BlockId],
        family_id_generator: impl FnOnce() -> ForkFamilyId,
    ) -> Result<Option<ForkFamilyId>, ForkError> {
        if block.domain_id != self.domain_id {
            return Err(ForkError::DomainMismatch {
                block_domain: block.domain_id,
                family_domain: self.domain_id,
            });
        }

        if existing_siblings.is_empty() {
            // First child of this parent — no fork yet.
            return Ok(None);
        }

        // There are existing siblings — this creates or extends a fork family.
        let family_id = if let Some(&fid) = self.parent_to_family.get(&block.parent_id) {
            // Fork family already exists; add this block as a branch tip.
            if let Some(family) = self.families.get_mut(&fid) {
                if !family.branch_tips.contains(&block.id) {
                    family.branch_tips.push(block.id);
                }
            }
            fid
        } else {
            // Create a new fork family.
            let fid = family_id_generator();
            let mut branch_tips: Vec<BlockId> = existing_siblings.to_vec();
            branch_tips.push(block.id);

            let family = ForkFamily {
                id: fid,
                domain_id: self.domain_id,
                track_tree_id: self.track_tree_id,
                common_ancestor_id: block.parent_id,
                branch_tips,
                dominant_branch_tip: None,
            };

            self.families.insert(fid, family);
            self.parent_to_family.insert(block.parent_id, fid);
            fid
        };

        Ok(Some(family_id))
    }

    /// Evaluate dominance for a fork family.
    ///
    /// `tip_metrics` maps each branch tip to its validated metric value.
    /// Only tips present in `tip_metrics` are considered — this allows
    /// the caller to filter out invalidated or disputed blocks before
    /// calling this method.
    ///
    /// The direction (higher/lower is better) is provided by the caller
    /// based on the domain spec.
    ///
    /// Returns the dominant tip, if one can be determined.
    pub fn evaluate_dominance(
        &mut self,
        family_id: &ForkFamilyId,
        tip_metrics: &HashMap<BlockId, MetricValue>,
        higher_is_better: bool,
    ) -> Result<Option<BlockId>, ForkError> {
        let family = self
            .families
            .get_mut(family_id)
            .ok_or(ForkError::FamilyNotFound {
                family_id: *family_id,
            })?;

        let mut best_tip: Option<BlockId> = None;
        let mut best_value: Option<f64> = None;

        for tip in &family.branch_tips {
            if let Some(metric) = tip_metrics.get(tip) {
                let val = metric.as_f64();
                let is_better = match best_value {
                    None => true,
                    Some(prev) => {
                        if higher_is_better {
                            val > prev
                        } else {
                            val < prev
                        }
                    }
                };
                if is_better {
                    best_tip = Some(*tip);
                    best_value = Some(val);
                }
            }
        }

        if let Some(tip) = best_tip {
            family.dominant_branch_tip = Some(tip);
        }

        Ok(best_tip)
    }

    /// Update the canonical frontier to the given block.
    ///
    /// This is called when dominance evaluation or a new accepted block
    /// produces a new best frontier candidate.
    pub fn update_canonical_frontier(
        &mut self,
        block_id: BlockId,
        metric_value: MetricValue,
    ) {
        self.canonical_frontier = Some(block_id);
        self.frontier_metric = Some(metric_value);
    }

    /// Simple frontier update: if the new block's metric is better than
    /// the current frontier, update the frontier.
    ///
    /// Returns true if the frontier was updated.
    pub fn maybe_update_frontier(
        &mut self,
        block_id: BlockId,
        metric_value: MetricValue,
        higher_is_better: bool,
    ) -> bool {
        let should_update = match self.frontier_metric {
            None => true,
            Some(current) => {
                if higher_is_better {
                    metric_value.as_f64() > current.as_f64()
                } else {
                    metric_value.as_f64() < current.as_f64()
                }
            }
        };

        if should_update {
            self.update_canonical_frontier(block_id, metric_value);
            true
        } else {
            false
        }
    }

    /// Handle a block being invalidated by an upheld challenge.
    ///
    /// Removes the block from any fork family branch tips, clears
    /// dominance if the invalidated block was the dominant tip,
    /// and clears the frontier if the invalidated block was the
    /// frontier.
    ///
    /// The caller is responsible for recomputing the frontier after
    /// invalidation using [`recompute_frontier`].
    pub fn on_block_invalidated(&mut self, block_id: BlockId) {
        // Remove from branch tips in all families.
        for family in self.families.values_mut() {
            family.branch_tips.retain(|t| *t != block_id);
            if family.dominant_branch_tip == Some(block_id) {
                family.dominant_branch_tip = None;
            }
        }

        // Clear frontier if it was the invalidated block.
        if self.canonical_frontier == Some(block_id) {
            self.canonical_frontier = None;
            self.frontier_metric = None;
        }
    }

    /// Recompute the canonical frontier from a set of valid block outcomes.
    ///
    /// Called after invalidation to find the next best frontier candidate.
    /// Iterates all provided (block_id, metric_value) pairs and selects
    /// the best one according to the metric direction.
    ///
    /// The caller is responsible for filtering out invalidated blocks
    /// and blocks with invalidated ancestors before calling this method.
    pub fn recompute_frontier(
        &mut self,
        valid_outcomes: impl Iterator<Item = (BlockId, MetricValue)>,
        higher_is_better: bool,
    ) {
        self.canonical_frontier = None;
        self.frontier_metric = None;

        for (block_id, metric) in valid_outcomes {
            self.maybe_update_frontier(block_id, metric, higher_is_better);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::*;
    use arc_protocol_types::*;

    fn make_accepted_block(id: u8, parent: u8, domain: u8, delta: f64) -> Block {
        Block {
            id: test_block_id(id),
            domain_id: test_domain_id(domain),
            parent_id: test_block_id(parent),
            proposer: test_proposer_id(1),
            child_state_ref: test_artifact_hash(60),
            diff_ref: test_artifact_hash(61),
            claimed_metric_delta: MetricValue::new(delta),
            evidence_bundle_hash: test_artifact_hash(62),
            fee: TokenAmount::new(10),
            bond: TokenAmount::new(500),
            epoch_id: EpochId(1),
            status: BlockStatus::ValidationComplete,
            timestamp: 1700001000,
        }
    }

    fn fid(n: u8) -> ForkFamilyId {
        ForkFamilyId::from_bytes([n; 32])
    }

    #[test]
    fn first_child_no_fork() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );
        let block = make_accepted_block(10, 1, 1, 0.01);
        let result = state
            .record_accepted_block(&block, &[], || fid(1))
            .unwrap();
        assert!(result.is_none());
        assert!(state.families.is_empty());
    }

    #[test]
    fn second_child_creates_fork() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.02);

        // First child: no fork.
        state
            .record_accepted_block(&block_a, &[], || fid(1))
            .unwrap();

        // Second child: creates fork.
        let result = state
            .record_accepted_block(&block_b, &[block_a.id], || fid(1))
            .unwrap();
        assert_eq!(result, Some(fid(1)));

        let family = &state.families[&fid(1)];
        assert_eq!(family.branch_tips.len(), 2);
        assert!(family.branch_tips.contains(&block_a.id));
        assert!(family.branch_tips.contains(&block_b.id));
        assert_eq!(family.common_ancestor_id, test_block_id(1));
    }

    #[test]
    fn third_child_extends_fork() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.02);
        let block_c = make_accepted_block(12, 1, 1, 0.03);

        state
            .record_accepted_block(&block_a, &[], || fid(1))
            .unwrap();
        state
            .record_accepted_block(&block_b, &[block_a.id], || fid(1))
            .unwrap();
        state
            .record_accepted_block(&block_c, &[block_a.id, block_b.id], || fid(2))
            .unwrap();

        let family = &state.families[&fid(1)];
        assert_eq!(family.branch_tips.len(), 3);
    }

    #[test]
    fn dominance_higher_is_better() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.05);

        state
            .record_accepted_block(&block_a, &[], || fid(1))
            .unwrap();
        state
            .record_accepted_block(&block_b, &[block_a.id], || fid(1))
            .unwrap();

        let mut metrics = HashMap::new();
        metrics.insert(block_a.id, MetricValue::new(0.01));
        metrics.insert(block_b.id, MetricValue::new(0.05));

        let dominant = state
            .evaluate_dominance(&fid(1), &metrics, true)
            .unwrap();
        assert_eq!(dominant, Some(block_b.id));

        let family = &state.families[&fid(1)];
        assert_eq!(family.dominant_branch_tip, Some(block_b.id));
    }

    #[test]
    fn frontier_update_higher_better() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        assert!(state.canonical_frontier.is_none());

        // First block becomes frontier.
        let updated = state.maybe_update_frontier(
            test_block_id(10),
            MetricValue::new(0.93),
            true,
        );
        assert!(updated);
        assert_eq!(state.canonical_frontier, Some(test_block_id(10)));

        // Better block replaces frontier.
        let updated = state.maybe_update_frontier(
            test_block_id(11),
            MetricValue::new(0.95),
            true,
        );
        assert!(updated);
        assert_eq!(state.canonical_frontier, Some(test_block_id(11)));

        // Worse block does not replace frontier.
        let updated = state.maybe_update_frontier(
            test_block_id(12),
            MetricValue::new(0.91),
            true,
        );
        assert!(!updated);
        assert_eq!(state.canonical_frontier, Some(test_block_id(11)));
    }

    #[test]
    fn domain_mismatch_rejected() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        // Block is in domain 2, state is for domain 1.
        let block = make_accepted_block(10, 1, 2, 0.01);
        let err = state
            .record_accepted_block(&block, &[], || fid(1))
            .unwrap_err();
        matches!(err, ForkError::DomainMismatch { .. });
    }

    // -------------------------------------------------------------------
    // Phase 0.3: invalidation and frontier recomputation tests
    // -------------------------------------------------------------------

    #[test]
    fn invalidation_clears_frontier() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        state.maybe_update_frontier(test_block_id(10), MetricValue::new(0.95), true);
        assert_eq!(state.canonical_frontier, Some(test_block_id(10)));

        state.on_block_invalidated(test_block_id(10));
        assert!(state.canonical_frontier.is_none());
        assert!(state.frontier_metric.is_none());
    }

    #[test]
    fn invalidation_removes_from_branch_tips() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.05);

        state.record_accepted_block(&block_a, &[], || fid(1)).unwrap();
        state.record_accepted_block(&block_b, &[block_a.id], || fid(1)).unwrap();

        let family = &state.families[&fid(1)];
        assert_eq!(family.branch_tips.len(), 2);

        state.on_block_invalidated(block_b.id);

        let family = &state.families[&fid(1)];
        assert_eq!(family.branch_tips.len(), 1);
        assert!(family.branch_tips.contains(&block_a.id));
        assert!(!family.branch_tips.contains(&block_b.id));
    }

    #[test]
    fn invalidation_clears_dominance() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.05);

        state.record_accepted_block(&block_a, &[], || fid(1)).unwrap();
        state.record_accepted_block(&block_b, &[block_a.id], || fid(1)).unwrap();

        // Set block_b as dominant.
        let mut metrics = HashMap::new();
        metrics.insert(block_a.id, MetricValue::new(0.01));
        metrics.insert(block_b.id, MetricValue::new(0.05));
        state.evaluate_dominance(&fid(1), &metrics, true).unwrap();
        assert_eq!(state.families[&fid(1)].dominant_branch_tip, Some(block_b.id));

        // Invalidate the dominant block.
        state.on_block_invalidated(block_b.id);
        assert_eq!(state.families[&fid(1)].dominant_branch_tip, None);
    }

    #[test]
    fn recompute_frontier_after_invalidation() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        // Set frontier to block 10 (best).
        state.maybe_update_frontier(test_block_id(10), MetricValue::new(0.95), true);
        state.maybe_update_frontier(test_block_id(11), MetricValue::new(0.92), true);
        assert_eq!(state.canonical_frontier, Some(test_block_id(10)));

        // Invalidate block 10.
        state.on_block_invalidated(test_block_id(10));
        assert!(state.canonical_frontier.is_none());

        // Recompute from remaining valid blocks.
        let remaining = vec![
            (test_block_id(11), MetricValue::new(0.92)),
        ];
        state.recompute_frontier(remaining.into_iter(), true);
        assert_eq!(state.canonical_frontier, Some(test_block_id(11)));
    }

    #[test]
    fn dominance_filters_invalidated_tips() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        let block_a = make_accepted_block(10, 1, 1, 0.01);
        let block_b = make_accepted_block(11, 1, 1, 0.05);

        state.record_accepted_block(&block_a, &[], || fid(1)).unwrap();
        state.record_accepted_block(&block_b, &[block_a.id], || fid(1)).unwrap();

        // Invalidate block_b, then evaluate dominance.
        // Only block_a's metric is provided (block_b filtered by caller).
        state.on_block_invalidated(block_b.id);

        let mut metrics = HashMap::new();
        metrics.insert(block_a.id, MetricValue::new(0.01));
        // block_b is not in metrics — caller excluded it.

        let dominant = state.evaluate_dominance(&fid(1), &metrics, true).unwrap();
        assert_eq!(dominant, Some(block_a.id));
    }

    #[test]
    fn invalidation_of_non_frontier_block_preserves_frontier() {
        let mut state = DomainForkState::new(
            test_domain_id(1),
            test_genesis_block_id(1).as_track_tree_id(),
        );

        state.maybe_update_frontier(test_block_id(10), MetricValue::new(0.95), true);
        state.maybe_update_frontier(test_block_id(11), MetricValue::new(0.92), true);
        assert_eq!(state.canonical_frontier, Some(test_block_id(10)));

        // Invalidate block 11, which is NOT the frontier.
        state.on_block_invalidated(test_block_id(11));
        assert_eq!(state.canonical_frontier, Some(test_block_id(10)));
    }
}
