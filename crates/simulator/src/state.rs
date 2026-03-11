// SPDX-License-Identifier: AGPL-3.0-or-later

//! Simulator state: the complete local protocol state and operations.
//!
//! # Phase 0.3 implementation
//!
//! The simulator composes all protocol engines into a single local
//! deterministic state machine. Phase 0.3 adds:
//!
//! - Validated block outcomes (protocol truth, not claimed values)
//! - Escrow record creation for accepted blocks
//! - Challenge consequence semantics (upheld → invalidation + slash,
//!   rejected → preserve state)
//! - Frontier selection using validated metrics
//! - Ancestor validity checking for frontier candidates
//! - Frontier recomputation after block invalidation

use std::collections::HashMap;

use arc_protocol_types::{
    Block, BlockId, BlockStatus, ChallengeId, ChallengeRecord, ChallengeTarget,
    ChallengeType, DomainId, EpochId, EscrowId, EscrowRecord, ForkFamilyId,
    GenesisBlock, GenesisBlockId, MetricDirection, MetricValue, ParticipantId,
    TokenAmount, ValidatedBlockOutcome, ValidationAttestation, ArtifactHash,
};

use arc_domain_engine::config::GenesisActivationConfig;
use arc_domain_engine::genesis::{
    self, ActivatedDomain, GenesisActivation, SeedValidationRecord,
};
use arc_domain_engine::registry::DomainRegistry;
use arc_protocol_rules::attestation::{self, AttestationSummary, ProvisionalOutcome};
use arc_protocol_rules::block_lifecycle;
use arc_protocol_rules::config::ValidationConfig;
use arc_protocol_rules::validator::{self, ValidatorPool};
use arc_challenge_engine::ChallengeConfig;
use arc_fork_engine::DomainForkState;
use arc_reward_engine::RewardConfig;

/// The complete local simulator state.
///
/// Holds all protocol objects in memory. This is not persistent storage —
/// it exists for local deterministic testing.
#[derive(Clone, Debug)]
pub struct SimulatorState {
    /// Current epoch.
    pub current_epoch: EpochId,
    /// Genesis activation config.
    pub genesis_config: GenesisActivationConfig,
    /// Block validation config.
    pub validation_config: ValidationConfig,
    /// Challenge config.
    pub challenge_config: ChallengeConfig,
    /// Reward/escrow config.
    pub reward_config: RewardConfig,
    /// Domain registry (active domains, specs, track trees).
    pub domain_registry: DomainRegistry,
    /// In-progress genesis activations.
    pub pending_activations: HashMap<GenesisBlockId, GenesisActivation>,
    /// All blocks indexed by ID.
    pub blocks: HashMap<BlockId, Block>,
    /// Attestations indexed by block ID.
    pub attestations: HashMap<BlockId, Vec<ValidationAttestation>>,
    /// Validator pools indexed by domain ID.
    pub validator_pools: HashMap<DomainId, ValidatorPool>,
    /// Validator assignments: block ID → assigned validator IDs.
    pub validator_assignments: HashMap<BlockId, Vec<arc_protocol_types::ValidatorId>>,
    /// Children of each block (for fork detection).
    pub children: HashMap<BlockId, Vec<BlockId>>,
    /// Fork state per domain.
    pub fork_states: HashMap<DomainId, DomainForkState>,
    /// Active challenges.
    pub challenges: HashMap<ChallengeId, ChallengeRecord>,
    /// Validated block outcomes — the protocol's truth about what validators
    /// actually observed, as opposed to what proposers claimed.
    pub validated_outcomes: HashMap<BlockId, ValidatedBlockOutcome>,
    /// Escrow records for accepted blocks.
    pub escrow_records: HashMap<EscrowId, EscrowRecord>,
    /// Reverse index: block ID → escrow ID.
    pub block_escrows: HashMap<BlockId, EscrowId>,
    /// Monotonic counter for generating unique fork family IDs.
    fork_family_counter: u64,
    /// Monotonic counter for generating unique escrow IDs.
    escrow_counter: u64,
    /// Metric direction per domain (cached from DomainSpec).
    pub metric_directions: HashMap<DomainId, MetricDirection>,
}

impl SimulatorState {
    pub fn new() -> Self {
        Self {
            current_epoch: EpochId::GENESIS,
            genesis_config: GenesisActivationConfig::default(),
            validation_config: ValidationConfig::default(),
            challenge_config: ChallengeConfig::default(),
            reward_config: RewardConfig::default(),
            domain_registry: DomainRegistry::new(),
            pending_activations: HashMap::new(),
            blocks: HashMap::new(),
            attestations: HashMap::new(),
            validator_pools: HashMap::new(),
            validator_assignments: HashMap::new(),
            children: HashMap::new(),
            fork_states: HashMap::new(),
            challenges: HashMap::new(),
            validated_outcomes: HashMap::new(),
            escrow_records: HashMap::new(),
            block_escrows: HashMap::new(),
            fork_family_counter: 0,
            escrow_counter: 0,
            metric_directions: HashMap::new(),
        }
    }

    /// Advance to the next epoch.
    pub fn advance_epoch(&mut self) {
        self.current_epoch = self.current_epoch.next();
    }

    // -----------------------------------------------------------------------
    // Genesis / domain activation
    // -----------------------------------------------------------------------

    /// Submit a genesis proposal.
    pub fn submit_genesis(
        &mut self,
        genesis_block: GenesisBlock,
    ) -> Result<GenesisBlockId, String> {
        let id = genesis_block.id;
        let activation =
            genesis::submit_genesis_proposal(genesis_block, &self.genesis_config)
                .map_err(|e| e.to_string())?;
        self.pending_activations.insert(id, activation);
        Ok(id)
    }

    /// Run RTS conformance on a pending genesis.
    pub fn evaluate_conformance(
        &mut self,
        genesis_id: &GenesisBlockId,
    ) -> Result<(), String> {
        let activation = self
            .pending_activations
            .get_mut(genesis_id)
            .ok_or_else(|| format!("genesis {} not found", genesis_id))?;
        genesis::evaluate_rts_conformance(activation).map_err(|e| e.to_string())
    }

    /// Record a seed validation for a pending genesis.
    pub fn record_seed_validation(
        &mut self,
        genesis_id: &GenesisBlockId,
        record: SeedValidationRecord,
    ) -> Result<(), String> {
        let activation = self
            .pending_activations
            .get_mut(genesis_id)
            .ok_or_else(|| format!("genesis {} not found", genesis_id))?;
        genesis::record_seed_validation(activation, record).map_err(|e| e.to_string())
    }

    /// Finalize track activation: if successful, registers the domain.
    pub fn finalize_activation(
        &mut self,
        genesis_id: &GenesisBlockId,
    ) -> Result<ActivatedDomain, String> {
        let activation = self
            .pending_activations
            .get_mut(genesis_id)
            .ok_or_else(|| format!("genesis {} not found", genesis_id))?;

        let activated =
            genesis::finalize_track_activation(activation, &self.genesis_config)
                .map_err(|e| e.to_string())?;

        let domain_id = activated.domain.id;
        let track_tree_id = activated.track_tree.id;
        let metric_direction = activated.domain_spec.metric_direction;

        self.domain_registry
            .register(activated.clone())
            .map_err(|e| e.to_string())?;

        // Initialize fork state for the domain.
        self.fork_states.insert(
            domain_id,
            DomainForkState::new(domain_id, track_tree_id),
        );

        // Cache metric direction.
        self.metric_directions.insert(domain_id, metric_direction);

        // Register the genesis block ID as a "parent" for child blocks.
        let genesis_as_block = genesis_id.as_block_id();
        self.children.entry(genesis_as_block).or_default();

        Ok(activated)
    }

    /// Register a validator pool for a domain.
    pub fn register_validator_pool(&mut self, pool: ValidatorPool) {
        self.validator_pools.insert(pool.domain_id, pool);
    }

    // -----------------------------------------------------------------------
    // Block lifecycle
    // -----------------------------------------------------------------------

    /// Submit a block to the protocol.
    ///
    /// Orchestration layer: runs structural validation, policy checks,
    /// and state precondition checks in sequence.
    pub fn submit_block(&mut self, block: Block) -> Result<BlockId, String> {
        let block_id = block.id;
        let domain_id = block.domain_id;

        // Check domain is active.
        if !self.domain_registry.is_active(&domain_id) {
            return Err(format!("domain {} is not active", domain_id));
        }

        // Structural validation (caller's gate — done here, not in transition).
        if let Err(errors) = arc_protocol_types::validate::validate_block_structure(&block) {
            return Err(format!(
                "block {} structural validation failed: {}",
                block_id,
                errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ")
            ));
        }

        // Policy check: bond minimum.
        block_lifecycle::check_block_bond(&block, &self.validation_config)
            .map_err(|e| e.to_string())?;

        // Look up parent status.
        let parent_status = if let Some(parent) = self.blocks.get(&block.parent_id) {
            parent.status
        } else if self.children.contains_key(&block.parent_id) {
            // Parent is a genesis block (represented as a key in children map).
            BlockStatus::Final
        } else {
            return Err(format!("parent {} not found", block.parent_id));
        };

        // State precondition checks (no structural validation inside).
        block_lifecycle::check_submission_preconditions(&block, parent_status)
            .map_err(|e| e.to_string())?;

        // Register parent-child relationship.
        self.children
            .entry(block.parent_id)
            .or_default()
            .push(block_id);

        self.blocks.insert(block_id, block);
        Ok(block_id)
    }

    /// Assign validators to a submitted block and begin validation.
    pub fn assign_validators(
        &mut self,
        block_id: &BlockId,
    ) -> Result<Vec<arc_protocol_types::ValidatorId>, String> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;

        let domain_id = block.domain_id;
        let pool = self
            .validator_pools
            .get(&domain_id)
            .ok_or_else(|| format!("no validator pool for domain {}", domain_id))?;

        let assigned = validator::assign_validators(
            pool,
            block_id,
            self.validation_config.validators_per_block,
        )
        .map_err(|e| e.to_string())?;

        block_lifecycle::begin_validation(block).map_err(|e| e.to_string())?;

        self.validator_assignments
            .insert(*block_id, assigned.clone());
        self.attestations.entry(*block_id).or_default();

        Ok(assigned)
    }

    /// Record an attestation for a block.
    pub fn record_attestation(
        &mut self,
        attestation: ValidationAttestation,
    ) -> Result<(), String> {
        let block_id = attestation.block_id;

        // Check block exists and is under validation.
        let block = self
            .blocks
            .get(&block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        if block.status != BlockStatus::UnderValidation {
            return Err(format!(
                "block {} is {:?}, not under validation",
                block_id, block.status
            ));
        }

        self.attestations
            .entry(block_id)
            .or_default()
            .push(attestation);
        Ok(())
    }

    /// Evaluate a block: aggregate attestations and determine outcome.
    ///
    /// Returns the provisional outcome and transitions the block status.
    pub fn evaluate_block(
        &mut self,
        block_id: &BlockId,
    ) -> Result<ProvisionalOutcome, String> {
        let atts = self
            .attestations
            .get(block_id)
            .ok_or_else(|| format!("no attestations for block {}", block_id))?
            .clone();

        let summary = attestation::aggregate_attestations(&atts);
        let outcome =
            attestation::evaluate_provisional_outcome(&summary, &self.validation_config);

        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;

        block_lifecycle::complete_validation(block, outcome, &self.validation_config)
            .map_err(|e| e.to_string())?;

        // If accepted, proceed through challenge window.
        if outcome == ProvisionalOutcome::Accepted {
            self.on_block_accepted(block_id, &summary)?;
        }

        Ok(outcome)
    }

    /// Handle a block being accepted: create validated outcome, escrow,
    /// open challenge window, update fork state and frontier.
    ///
    /// Phase 0.3: uses validated metric value (mean observed delta from
    /// attestations) rather than the proposer's claimed_metric_delta.
    fn on_block_accepted(
        &mut self,
        block_id: &BlockId,
        summary: &AttestationSummary,
    ) -> Result<(), String> {
        let block = self.blocks.get(block_id).cloned()
            .ok_or_else(|| format!("block {} not found", block_id))?;

        // Compute the validated metric value from attestation observations.
        let validated_metric = MetricValue::new(
            summary.mean_observed_delta.unwrap_or(block.claimed_metric_delta.as_f64()),
        );

        // Store the validated outcome (protocol truth).
        self.validated_outcomes.insert(block.id, ValidatedBlockOutcome {
            block_id: block.id,
            validated_metric_value: validated_metric,
            attestation_count: summary.total,
            validation_epoch: self.current_epoch,
        });

        // Create escrow record for the proposer's bond.
        self.escrow_counter += 1;
        let mut escrow_bytes = [0u8; 32];
        escrow_bytes[..8].copy_from_slice(&self.escrow_counter.to_le_bytes());
        let escrow_id = EscrowId::from_bytes(escrow_bytes);

        let escrow = arc_reward_engine::create_block_escrow(
            escrow_id,
            block.id,
            ParticipantId::from_bytes(*block.proposer.as_bytes()),
            block.bond,
            self.current_epoch,
            &self.reward_config,
        );
        self.escrow_records.insert(escrow_id, escrow);
        self.block_escrows.insert(block.id, escrow_id);

        // Open challenge window.
        {
            let block_mut = self.blocks.get_mut(block_id).unwrap();
            block_lifecycle::open_challenge_window(block_mut)
                .map_err(|e| e.to_string())?;
        }

        // Record in fork state.
        let domain_id = block.domain_id;
        let existing_siblings: Vec<BlockId> = self
            .children
            .get(&block.parent_id)
            .map(|children| {
                children
                    .iter()
                    .filter(|&&cid| {
                        cid != block.id
                            && self
                                .blocks
                                .get(&cid)
                                .map(|b| {
                                    block_lifecycle::is_block_accepted(b.status)
                                })
                                .unwrap_or(false)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default();

        if let Some(fork_state) = self.fork_states.get_mut(&domain_id) {
            let counter = &mut self.fork_family_counter;
            fork_state
                .record_accepted_block(&block, &existing_siblings, || {
                    *counter += 1;
                    let mut bytes = [0u8; 32];
                    bytes[..8].copy_from_slice(&counter.to_le_bytes());
                    ForkFamilyId::from_bytes(bytes)
                })
                .map_err(|e| e.to_string())?;

            // Update frontier using VALIDATED metric value (not claimed delta).
            let higher_is_better = self
                .metric_directions
                .get(&domain_id)
                .map(|d| *d == MetricDirection::HigherBetter)
                .unwrap_or(true);

            fork_state.maybe_update_frontier(
                block.id,
                validated_metric,
                higher_is_better,
            );
        }

        Ok(())
    }

    /// Close challenge window for a block (no upheld challenges).
    pub fn close_challenge_window(
        &mut self,
        block_id: &BlockId,
    ) -> Result<(), String> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        block_lifecycle::close_challenge_window(block).map_err(|e| e.to_string())
    }

    /// Settle a block and release its escrow.
    pub fn settle_block(&mut self, block_id: &BlockId) -> Result<(), String> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        block_lifecycle::settle_block(block).map_err(|e| e.to_string())?;

        // Release the corresponding escrow.
        if let Some(escrow_id) = self.block_escrows.get(block_id) {
            if let Some(escrow) = self.escrow_records.get_mut(escrow_id) {
                arc_reward_engine::release_escrow(escrow)
                    .map_err(|e| e.to_string())?;
            }
        }

        Ok(())
    }

    /// Finalize a block.
    pub fn finalize_block(&mut self, block_id: &BlockId) -> Result<(), String> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        block_lifecycle::finalize_block(block).map_err(|e| e.to_string())
    }

    // -----------------------------------------------------------------------
    // Challenges
    // -----------------------------------------------------------------------

    /// Open a challenge against a block.
    pub fn open_challenge(
        &mut self,
        challenge_id: ChallengeId,
        challenge_type: ChallengeType,
        target: ChallengeTarget,
        challenger: ParticipantId,
        bond: TokenAmount,
        evidence_ref: ArtifactHash,
    ) -> Result<ChallengeId, String> {
        let blocks = &self.blocks;
        let challenge = arc_challenge_engine::open_challenge(
            challenge_id,
            challenge_type,
            target,
            challenger,
            bond,
            evidence_ref,
            self.current_epoch,
            0, // timestamp placeholder — will be wired to epoch/clock in later phase
            &self.challenge_config,
            |bid| blocks.get(bid).map(|b| b.status),
        )
        .map_err(|e| e.to_string())?;

        let id = challenge.id;
        self.challenges.insert(id, challenge);
        Ok(id)
    }

    /// Begin review of an open challenge.
    pub fn begin_challenge_review(
        &mut self,
        challenge_id: &ChallengeId,
    ) -> Result<(), String> {
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| format!("challenge {} not found", challenge_id))?;
        arc_challenge_engine::begin_review(challenge).map_err(|e| e.to_string())
    }

    /// Uphold a challenge: invalidate the target block and apply
    /// protocol consequences.
    ///
    /// Phase 0.3 consequences:
    /// - Target block status → Invalidated
    /// - Block's escrow → Slashed
    /// - Block removed from fork family branch tips
    /// - Frontier recomputed if the invalidated block was the frontier
    /// - Dominance cleared if the invalidated block was the dominant tip
    ///
    /// Descendant blocks are not automatically invalidated in Phase 0.3,
    /// but they are excluded from frontier consideration because
    /// `is_on_valid_chain` checks for invalidated ancestors.
    pub fn uphold_challenge(
        &mut self,
        challenge_id: &ChallengeId,
    ) -> Result<(), String> {
        // Transition challenge to Upheld.
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| format!("challenge {} not found", challenge_id))?;
        arc_challenge_engine::uphold_challenge(challenge)
            .map_err(|e| e.to_string())?;

        // Identify the target block.
        let target_block_id = match &challenge.target {
            ChallengeTarget::Block { block_id } => *block_id,
            ChallengeTarget::Attestation { block_id, .. } => *block_id,
            ChallengeTarget::Attribution { block_id, .. } => *block_id,
            ChallengeTarget::DominanceDecision { .. } => {
                // Dominance challenges don't invalidate a specific block.
                return Ok(());
            }
        };

        // Invalidate the target block.
        let block = self
            .blocks
            .get_mut(&target_block_id)
            .ok_or_else(|| format!("target block {} not found", target_block_id))?;
        let domain_id = block.domain_id;
        block_lifecycle::invalidate_block(block)
            .map_err(|e| e.to_string())?;

        // Slash the block's escrow.
        if let Some(escrow_id) = self.block_escrows.get(&target_block_id) {
            if let Some(escrow) = self.escrow_records.get_mut(escrow_id) {
                arc_reward_engine::slash_escrow(escrow)
                    .map_err(|e| e.to_string())?;
            }
        }

        // Remove from validated outcomes (invalidated blocks are no longer
        // protocol truth for frontier purposes).
        self.validated_outcomes.remove(&target_block_id);

        // Compute valid frontier candidates before mutating fork state,
        // to satisfy the borrow checker (valid_frontier_candidates borrows
        // self.blocks and self.validated_outcomes immutably).
        let higher_is_better = self
            .metric_directions
            .get(&domain_id)
            .map(|d| *d == MetricDirection::HigherBetter)
            .unwrap_or(true);
        let valid_candidates = self.valid_frontier_candidates(&domain_id);

        // Update fork state: remove from branch tips, clear dominance/frontier.
        if let Some(fork_state) = self.fork_states.get_mut(&domain_id) {
            fork_state.on_block_invalidated(target_block_id);
            fork_state.recompute_frontier(valid_candidates.into_iter(), higher_is_better);
        }

        Ok(())
    }

    /// Reject a challenge: the target block is preserved, the challenger
    /// loses their bond (bond distribution deferred to later phases).
    pub fn reject_challenge(
        &mut self,
        challenge_id: &ChallengeId,
    ) -> Result<(), String> {
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| format!("challenge {} not found", challenge_id))?;
        arc_challenge_engine::reject_challenge(challenge)
            .map_err(|e| e.to_string())
    }

    // -----------------------------------------------------------------------
    // Validity helpers
    // -----------------------------------------------------------------------

    /// Check whether a block is on a valid chain (no invalidated ancestors).
    ///
    /// Walks up the parent chain from the given block. Returns false if
    /// any ancestor (including the block itself) has status Invalidated.
    /// Terminates at genesis blocks (which are not stored in self.blocks).
    pub fn is_on_valid_chain(&self, block_id: &BlockId) -> bool {
        let mut current = *block_id;
        loop {
            let Some(block) = self.blocks.get(&current) else {
                // Reached a genesis block or unknown parent — valid chain end.
                return true;
            };
            if block.status == BlockStatus::Invalidated {
                return false;
            }
            current = block.parent_id;
        }
    }

    /// Gather all valid frontier candidates for a domain.
    ///
    /// Returns (block_id, validated_metric_value) pairs for blocks that:
    /// - Belong to the specified domain
    /// - Have a validated outcome recorded
    /// - Are not themselves invalidated or rejected
    /// - Have no invalidated ancestors in their chain
    fn valid_frontier_candidates(
        &self,
        domain_id: &DomainId,
    ) -> Vec<(BlockId, MetricValue)> {
        self.validated_outcomes
            .iter()
            .filter(|(bid, _)| {
                self.blocks
                    .get(bid)
                    .map(|b| {
                        b.domain_id == *domain_id
                            && !matches!(
                                b.status,
                                BlockStatus::Rejected | BlockStatus::Invalidated
                            )
                            && self.is_on_valid_chain(bid)
                    })
                    .unwrap_or(false)
            })
            .map(|(bid, outcome)| (*bid, outcome.validated_metric_value))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Query helpers
    // -----------------------------------------------------------------------

    /// Get a block's current status.
    pub fn block_status(&self, block_id: &BlockId) -> Option<BlockStatus> {
        self.blocks.get(block_id).map(|b| b.status)
    }

    /// Get the canonical frontier block for a domain.
    pub fn canonical_frontier(&self, domain_id: &DomainId) -> Option<BlockId> {
        self.fork_states
            .get(domain_id)
            .and_then(|fs| fs.canonical_frontier)
    }

    /// Get fork families for a domain.
    pub fn fork_families(
        &self,
        domain_id: &DomainId,
    ) -> Vec<&arc_protocol_types::ForkFamily> {
        self.fork_states
            .get(domain_id)
            .map(|fs| fs.families.values().collect())
            .unwrap_or_default()
    }

    /// Get the attestation summary for a block.
    pub fn attestation_summary(
        &self,
        block_id: &BlockId,
    ) -> Option<AttestationSummary> {
        self.attestations
            .get(block_id)
            .map(|atts| attestation::aggregate_attestations(atts))
    }

    /// Get the validated outcome for a block.
    pub fn validated_outcome(
        &self,
        block_id: &BlockId,
    ) -> Option<&ValidatedBlockOutcome> {
        self.validated_outcomes.get(block_id)
    }

    /// Get the escrow record for a block.
    pub fn block_escrow(
        &self,
        block_id: &BlockId,
    ) -> Option<&EscrowRecord> {
        self.block_escrows
            .get(block_id)
            .and_then(|eid| self.escrow_records.get(eid))
    }
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::new()
    }
}
