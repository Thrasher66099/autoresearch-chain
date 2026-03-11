// SPDX-License-Identifier: AGPL-3.0-or-later

//! Simulator state: the complete local protocol state and operations.

use std::collections::HashMap;

use arc_protocol_types::{
    Block, BlockId, BlockStatus, ChallengeId, ChallengeRecord, ChallengeTarget,
    ChallengeType, DomainId, EpochId, ForkFamilyId, GenesisBlock,
    GenesisBlockId, MetricDirection, ParticipantId, TokenAmount,
    ValidationAttestation, ArtifactHash,
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
    /// Monotonic counter for generating unique fork family IDs.
    fork_family_counter: u64,
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
            domain_registry: DomainRegistry::new(),
            pending_activations: HashMap::new(),
            blocks: HashMap::new(),
            attestations: HashMap::new(),
            validator_pools: HashMap::new(),
            validator_assignments: HashMap::new(),
            children: HashMap::new(),
            fork_states: HashMap::new(),
            challenges: HashMap::new(),
            fork_family_counter: 0,
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
        // This is an orchestration check (not a state machine rule) since
        // the attestation record is kept separately from block state.
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
            self.on_block_accepted(block_id)?;
        }

        Ok(outcome)
    }

    /// Handle a block being accepted: open challenge window, update
    /// fork state and frontier.
    fn on_block_accepted(&mut self, block_id: &BlockId) -> Result<(), String> {
        let block = self.blocks.get(block_id).cloned()
            .ok_or_else(|| format!("block {} not found", block_id))?;

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

            // Update frontier if this block is better.
            let higher_is_better = self
                .metric_directions
                .get(&domain_id)
                .map(|d| *d == MetricDirection::HigherBetter)
                .unwrap_or(true);

            fork_state.maybe_update_frontier(
                block.id,
                block.claimed_metric_delta,
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

    /// Settle a block.
    pub fn settle_block(&mut self, block_id: &BlockId) -> Result<(), String> {
        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        block_lifecycle::settle_block(block).map_err(|e| e.to_string())
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
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::new()
    }
}
