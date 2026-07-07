// SPDX-License-Identifier: AGPL-3.0-or-later

//! Simulator state: the complete local protocol state and operations.
//!
//! # Phase 0.3 / 0.3d implementation
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
//!
//! Phase 0.3d adds explicit derived branch validity:
//!
//! - `DerivedValidity` enum: `DirectValid`, `DirectInvalid`, `AncestryInvalid`
//! - Centralized `derived_validity()` method as the truth surface
//! - Frontier candidacy gated on `DirectValid`
//! - Dominance evaluation filtered by derived validity
//! - Settlement gated on `DirectValid` (ancestry-poisoned blocks cannot settle)
//! - Escrow release prevented for non-`DirectValid` blocks via settlement gate

use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use arc_protocol_types::{
    Block, BlockId, BlockStatus, ChallengeId, ChallengeRecord, ChallengeTarget,
    ChallengeType, DerivedValidity, DomainId, EpochId, EscrowId, EscrowKind,
    EscrowRecord, EscrowStatus, ForkFamilyId, GenesisBlock, GenesisBlockId,
    MetricDirection, MetricValue, ParticipantId, SlashDistribution, TokenAmount,
    ValidatedBlockOutcome, ValidationAttestation, ArtifactHash,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Reverse index: block ID → escrow IDs (bond + reward tranches).
    pub block_escrows: HashMap<BlockId, Vec<EscrowId>>,
    /// Reverse index: challenge ID → challenger bond escrow ID.
    pub challenge_escrows: HashMap<ChallengeId, EscrowId>,
    /// Slash distributions from upheld challenges, keyed by challenge.
    pub slash_distributions: HashMap<ChallengeId, SlashDistribution>,
    /// Monotonic counter for generating unique fork family IDs.
    fork_family_counter: u64,
    /// Monotonic counter for generating unique escrow IDs.
    escrow_counter: u64,
    /// Metric direction per domain (cached from DomainSpec).
    pub metric_directions: HashMap<DomainId, MetricDirection>,
    /// Whether actor-bearing submissions must carry a verified Ed25519
    /// signature (enforced at the node boundary; set at init).
    #[serde(default)]
    pub require_signatures: bool,
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
            challenge_escrows: HashMap::new(),
            slash_distributions: HashMap::new(),
            fork_family_counter: 0,
            escrow_counter: 0,
            metric_directions: HashMap::new(),
            require_signatures: false,
        }
    }

    /// Advance to the next epoch.
    pub fn advance_epoch(&mut self) {
        self.current_epoch = self.current_epoch.next();
    }

    /// Generate the next unique escrow ID.
    fn next_escrow_id(&mut self) -> EscrowId {
        self.escrow_counter += 1;
        let mut escrow_bytes = [0u8; 32];
        escrow_bytes[..8].copy_from_slice(&self.escrow_counter.to_le_bytes());
        EscrowId::from_bytes(escrow_bytes)
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
        let mut outcome =
            attestation::evaluate_provisional_outcome(&summary, &self.validation_config);

        // Minimum-improvement threshold: the validated mean improvement
        // (in the metric's improvement direction) must reach
        // min_accepted_delta. Claims inside the attestation tolerance band
        // are unfalsifiable by replay, so sub-threshold improvements must
        // not earn block rewards — otherwise noise mining is risk-free.
        if outcome == ProvisionalOutcome::Accepted
            && self.validation_config.min_accepted_delta > 0.0
        {
            let domain_id = self
                .blocks
                .get(block_id)
                .ok_or_else(|| format!("block {} not found", block_id))?
                .domain_id;
            let higher_is_better = self
                .metric_directions
                .get(&domain_id)
                .map(|d| *d == MetricDirection::HigherBetter)
                .unwrap_or(true);
            let improvement = summary
                .mean_observed_delta
                .map(|m| if higher_is_better { m } else { -m })
                .unwrap_or(f64::NEG_INFINITY);
            if improvement < self.validation_config.min_accepted_delta {
                outcome = ProvisionalOutcome::Rejected;
            }
        }

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
    /// Phase 0.3b: uses validated metric delta (mean observed delta from
    /// attestations) rather than the proposer's claimed_metric_delta.
    /// Returns an error if no validator-observed delta is available —
    /// protocol truth is never constructed from proposer claims.
    fn on_block_accepted(
        &mut self,
        block_id: &BlockId,
        summary: &AttestationSummary,
    ) -> Result<(), String> {
        let block = self.blocks.get(block_id).cloned()
            .ok_or_else(|| format!("block {} not found", block_id))?;

        // Compute the validated metric delta from attestation observations.
        // Protocol truth must come from validator-observed data, never from
        // the proposer's claim.
        //
        // Since Phase 0.3c, acceptance requires truth-bearing Pass
        // attestations (Pass with observed_delta), so mean_observed_delta
        // is structurally guaranteed to exist here. The assertion guards
        // against implementation bugs that bypass the protocol-rules layer.
        let mean_delta = summary.mean_observed_delta.unwrap_or_else(|| {
            panic!(
                "protocol invariant violation: block {} reached on_block_accepted \
                 but no validator-observed delta available; acceptance requires \
                 truth-bearing Pass attestations (Phase 0.3c)",
                block.id
            )
        });
        let validated_metric = MetricValue::new(mean_delta);

        // Store the validated outcome (protocol truth).
        self.validated_outcomes.insert(block.id, ValidatedBlockOutcome {
            block_id: block.id,
            validated_metric_delta: validated_metric,
            attestation_count: summary.total,
            validation_epoch: self.current_epoch,
        });

        // Fraud-exposure invariant: the provisional reward tranche is paid
        // at acceptance and cannot be clawed back once released, so the
        // proposer's slashable bond must cover it. Otherwise fraud would be
        // net-positive even when caught.
        let provisional_amount = self.reward_config.provisional_reward_amount();
        if block.bond.as_u64() < provisional_amount.as_u64() {
            return Err(format!(
                "cannot accept block {}: bond {} does not cover the provisional \
                 reward tranche {} (fraud-exposure invariant)",
                block.id,
                block.bond.as_u64(),
                provisional_amount.as_u64()
            ));
        }

        let proposer = ParticipantId::from_bytes(*block.proposer.as_bytes());

        // Create escrow record for the proposer's bond.
        let bond_escrow_id = self.next_escrow_id();
        let escrow = arc_reward_engine::create_block_escrow(
            bond_escrow_id,
            block.id,
            proposer,
            block.bond,
            self.current_epoch,
            &self.reward_config,
        );
        self.escrow_records.insert(bond_escrow_id, escrow);

        // Create the staged reward tranches. The provisional tranche is
        // released immediately (the spec's "immediate incentive"); the
        // survival tranche is held until settlement after the challenge
        // window.
        let provisional_id = self.next_escrow_id();
        let survival_id = self.next_escrow_id();
        let (mut provisional, survival) = arc_reward_engine::create_reward_tranches(
            provisional_id,
            survival_id,
            block.id,
            proposer,
            self.current_epoch,
            &self.reward_config,
        );
        arc_reward_engine::release_escrow(&mut provisional, self.current_epoch)
            .map_err(|e| e.to_string())?;
        self.escrow_records.insert(provisional_id, provisional);
        self.escrow_records.insert(survival_id, survival);
        self.block_escrows
            .insert(block.id, vec![bond_escrow_id, provisional_id, survival_id]);

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

        // Cumulative validated improvement from the seed — the comparable
        // frontier metric (computed before the fork-state borrow).
        let cumulative_metric =
            self.cumulative_validated_delta(&block.id).ok_or_else(|| {
                format!(
                    "block {} accepted but cumulative validated delta \
                     unavailable (ancestor missing validated outcome)",
                    block.id
                )
            })?;

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

            // Update frontier using VALIDATED metric delta (not claimed delta).
            let higher_is_better = self
                .metric_directions
                .get(&domain_id)
                .ok_or_else(|| {
                    format!(
                        "domain {} has no registered metric direction; \
                         cannot evaluate frontier",
                        domain_id
                    )
                })
                .map(|d| *d == MetricDirection::HigherBetter)?;

            // Frontier selection compares cumulative validated improvement
            // from the seed, not the block's own delta — a child's delta is
            // relative to its parent, so raw deltas are incomparable across
            // generations.
            fork_state.maybe_update_frontier(
                block.id,
                cumulative_metric,
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
    ///
    /// Settlement is gated on derived branch validity (Phase 0.3d):
    /// only blocks with `DerivedValidity::DirectValid` may settle.
    /// Blocks that are `DirectInvalid` or `AncestryInvalid` cannot settle
    /// and must not receive escrow release.
    ///
    /// Escrow release respects the release_epoch: the current epoch must
    /// be at or past the escrow's release_epoch (challenge survival boundary).
    /// The derived validity and escrow timing checks are performed before
    /// the block status transition to avoid inconsistent state on failure.
    pub fn settle_block(&mut self, block_id: &BlockId) -> Result<(), String> {
        // Phase 0.3d: derived branch validity gate.
        // A block with poisoned ancestry must not settle, even if its
        // own stored status would otherwise permit it.
        let validity = self.derived_validity(block_id);
        match validity {
            DerivedValidity::DirectValid => {}
            DerivedValidity::DirectInvalid => {
                return Err(format!(
                    "cannot settle block {}: block is directly invalidated",
                    block_id
                ));
            }
            DerivedValidity::AncestryInvalid => {
                return Err(format!(
                    "cannot settle block {}: block has invalidated ancestry \
                     (derived validity: AncestryInvalid)",
                    block_id
                ));
            }
        }

        // Pre-check: verify escrow release timing before mutating block status.
        // This prevents the block from transitioning to Settled while any of
        // its escrows (bond, survival tranche) cannot be released yet.
        let escrow_ids: Vec<EscrowId> = self
            .block_escrows
            .get(block_id)
            .cloned()
            .unwrap_or_default();
        for escrow_id in &escrow_ids {
            if let Some(escrow) = self.escrow_records.get(escrow_id) {
                if escrow.status == EscrowStatus::Held
                    && self.current_epoch.0 < escrow.release_epoch.0
                {
                    return Err(format!(
                        "cannot settle block {}: escrow {} not releasable until epoch {} \
                         (current epoch {})",
                        block_id, escrow_id, escrow.release_epoch.0, self.current_epoch.0
                    ));
                }
            }
        }

        let block = self
            .blocks
            .get_mut(block_id)
            .ok_or_else(|| format!("block {} not found", block_id))?;
        block_lifecycle::settle_block(block).map_err(|e| e.to_string())?;

        // Release all still-held escrows for the block (proposer bond and
        // survival tranche; the provisional tranche was released at
        // acceptance), enforcing release_epoch.
        for escrow_id in &escrow_ids {
            if let Some(escrow) = self.escrow_records.get_mut(escrow_id) {
                if escrow.status == EscrowStatus::Held {
                    arc_reward_engine::release_escrow(escrow, self.current_epoch)
                        .map_err(|e| e.to_string())?;
                }
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

        // Escrow the challenger's bond. It is released if the challenge is
        // upheld or expires unresolved, and slashed if the challenge is
        // rejected. For dominance challenges (no single target block) the
        // escrow records a zero block ID.
        let target_block_id =
            Self::challenge_target_block(&challenge.target).unwrap_or(BlockId::ZERO);
        let escrow_id = self.next_escrow_id();
        let escrow = arc_reward_engine::create_challenge_escrow(
            escrow_id,
            target_block_id,
            challenge.challenger,
            challenge.bond,
            self.current_epoch,
        );
        self.escrow_records.insert(escrow_id, escrow);
        self.challenge_escrows.insert(id, escrow_id);

        self.challenges.insert(id, challenge);
        Ok(id)
    }

    /// The block a challenge targets, if the target names one.
    fn challenge_target_block(target: &ChallengeTarget) -> Option<BlockId> {
        match target {
            ChallengeTarget::Block { block_id } => Some(*block_id),
            ChallengeTarget::Attestation { block_id, .. } => Some(*block_id),
            ChallengeTarget::Attribution { block_id, .. } => Some(*block_id),
            ChallengeTarget::DominanceDecision { .. } => None,
        }
    }

    /// Resolve a challenge's bond escrow: release it (bond returned to the
    /// challenger) or slash it (bond forfeited).
    fn resolve_challenge_escrow(
        &mut self,
        challenge_id: &ChallengeId,
        forfeit: bool,
    ) -> Result<(), String> {
        let Some(escrow_id) = self.challenge_escrows.get(challenge_id) else {
            // Challenges created before challenge escrows existed have no
            // bond escrow; nothing to resolve.
            return Ok(());
        };
        let escrow = self
            .escrow_records
            .get_mut(escrow_id)
            .ok_or_else(|| format!("challenge escrow {} not found", escrow_id))?;
        if forfeit {
            arc_reward_engine::slash_escrow(escrow).map_err(|e| e.to_string())
        } else {
            arc_reward_engine::release_escrow(escrow, self.current_epoch)
                .map_err(|e| e.to_string())
        }
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
        let challenger = challenge.challenger;
        let target = Self::challenge_target_block(&challenge.target);

        // The challenge succeeded: return the challenger's bond.
        self.resolve_challenge_escrow(challenge_id, false)?;

        let Some(target_block_id) = target else {
            // Dominance challenges don't invalidate a specific block.
            return Ok(());
        };

        // Invalidate the target block.
        let block = self
            .blocks
            .get_mut(&target_block_id)
            .ok_or_else(|| format!("target block {} not found", target_block_id))?;
        let domain_id = block.domain_id;
        block_lifecycle::invalidate_block(block)
            .map_err(|e| e.to_string())?;

        // Slash all still-held escrows of the block (proposer bond and
        // survival tranche; a released provisional tranche is the accepted
        // fraud exposure and is not clawed back).
        let mut slashed_total: u64 = 0;
        if let Some(escrow_ids) = self.block_escrows.get(&target_block_id) {
            for escrow_id in escrow_ids.clone() {
                if let Some(escrow) = self.escrow_records.get_mut(&escrow_id) {
                    if escrow.status == EscrowStatus::Held {
                        arc_reward_engine::slash_escrow(escrow)
                            .map_err(|e| e.to_string())?;
                        slashed_total += escrow.amount.as_u64();
                    }
                }
            }
        }

        // Distribute the slashed funds: a configured fraction to the
        // challenger, the residual burned. Recorded for auditability.
        let (challenger_payout, burned) = arc_reward_engine::compute_slash_distribution(
            TokenAmount::new(slashed_total),
            &self.reward_config,
        );
        self.slash_distributions.insert(
            *challenge_id,
            SlashDistribution {
                challenge_id: *challenge_id,
                block_id: target_block_id,
                slashed_amount: TokenAmount::new(slashed_total),
                challenger,
                challenger_payout,
                burned,
                epoch: self.current_epoch,
            },
        );

        // Remove from validated outcomes (invalidated blocks are no longer
        // protocol truth for frontier purposes).
        self.validated_outcomes.remove(&target_block_id);

        // Compute valid frontier candidates before mutating fork state,
        // to satisfy the borrow checker (valid_frontier_candidates borrows
        // self.blocks and self.validated_outcomes immutably).
        let higher_is_better = self
            .metric_directions
            .get(&domain_id)
            .ok_or_else(|| {
                format!(
                    "domain {} has no registered metric direction; \
                     cannot recompute frontier",
                    domain_id
                )
            })
            .map(|d| *d == MetricDirection::HigherBetter)?;
        let valid_candidates = self.valid_frontier_candidates(&domain_id);

        // Update fork state: remove from branch tips, clear dominance/frontier.
        if let Some(fork_state) = self.fork_states.get_mut(&domain_id) {
            fork_state.on_block_invalidated(target_block_id);
            fork_state.recompute_frontier(valid_candidates.into_iter(), higher_is_better);
        }

        Ok(())
    }

    /// Reject a challenge: the target block is preserved and the challenger
    /// forfeits their bond (escrow slashed).
    pub fn reject_challenge(
        &mut self,
        challenge_id: &ChallengeId,
    ) -> Result<(), String> {
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| format!("challenge {} not found", challenge_id))?;
        arc_challenge_engine::reject_challenge(challenge)
            .map_err(|e| e.to_string())?;

        // The challenge failed: the challenger loses the bond.
        self.resolve_challenge_escrow(challenge_id, true)
    }

    /// Expire a challenge that was not adjudicated in time.
    ///
    /// The challenger's bond is returned: expiry means the protocol failed
    /// to adjudicate, not that the challenge was wrong. Punishing unresolved
    /// challenges would chill challenging.
    pub fn expire_challenge(
        &mut self,
        challenge_id: &ChallengeId,
    ) -> Result<(), String> {
        let challenge = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| format!("challenge {} not found", challenge_id))?;
        arc_challenge_engine::expire_challenge(challenge)
            .map_err(|e| e.to_string())?;

        self.resolve_challenge_escrow(challenge_id, false)
    }

    // -----------------------------------------------------------------------
    // Derived branch validity (Phase 0.3d)
    // -----------------------------------------------------------------------

    /// Compute the derived branch validity for a block.
    ///
    /// This is the **centralized truth surface** for branch validity.
    /// All downstream logic — frontier candidacy, dominance evaluation,
    /// settlement, and escrow release — must use this rather than ad hoc
    /// status checks.
    ///
    /// # Returns
    ///
    /// - `DirectInvalid` if the block's own status is `Invalidated`.
    /// - `AncestryInvalid` if the block itself is not invalidated but
    ///   any ancestor in its parent chain has `Invalidated` status.
    /// - `DirectValid` if neither the block nor any ancestor is invalidated.
    ///
    /// Terminates at genesis blocks (not stored in `self.blocks`), which
    /// are considered valid chain roots.
    pub fn derived_validity(&self, block_id: &BlockId) -> DerivedValidity {
        let Some(block) = self.blocks.get(block_id) else {
            // Not a stored block (genesis or unknown) — treat as valid root.
            return DerivedValidity::DirectValid;
        };

        // Check the block itself first.
        if block.status == BlockStatus::Invalidated {
            return DerivedValidity::DirectInvalid;
        }

        // Walk the ancestor chain looking for invalidated parents.
        let mut current = block.parent_id;
        loop {
            let Some(ancestor) = self.blocks.get(&current) else {
                // Reached genesis or unknown parent — valid chain end.
                return DerivedValidity::DirectValid;
            };
            if ancestor.status == BlockStatus::Invalidated {
                return DerivedValidity::AncestryInvalid;
            }
            current = ancestor.parent_id;
        }
    }

    /// Check whether a block is on a valid chain (no invalidated ancestors).
    ///
    /// Convenience wrapper around [`derived_validity`]: returns true only
    /// when derived validity is `DirectValid`.
    pub fn is_on_valid_chain(&self, block_id: &BlockId) -> bool {
        self.derived_validity(block_id) == DerivedValidity::DirectValid
    }

    /// Gather all valid frontier candidates for a domain.
    ///
    /// Returns (block_id, validated_metric_delta) pairs for blocks whose
    /// derived validity is `DirectValid` and that belong to the specified
    /// domain with a recorded validated outcome.
    ///
    /// Blocks that are `DirectInvalid`, `AncestryInvalid`, or `Rejected`
    /// are excluded.
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
                            && !matches!(b.status, BlockStatus::Rejected)
                            && self.derived_validity(bid) == DerivedValidity::DirectValid
                    })
                    .unwrap_or(false)
            })
            .filter_map(|(bid, _)| {
                self.cumulative_validated_delta(bid).map(|m| (*bid, m))
            })
            .collect()
    }

    /// Cumulative validated improvement from the domain seed to a block.
    ///
    /// Sums the validator-observed metric deltas along the block's
    /// ancestor chain (protocol truth only — never proposer claims).
    /// Per-block deltas are relative to each block's own parent, so they
    /// are not comparable across generations; the cumulative sum from the
    /// seed is the quantity frontier selection and dominance evaluation
    /// must compare. Returns `None` if any block on the chain lacks a
    /// validated outcome (e.g. an invalidated ancestor).
    pub fn cumulative_validated_delta(
        &self,
        block_id: &BlockId,
    ) -> Option<MetricValue> {
        let mut total = 0.0;
        let mut current = *block_id;
        loop {
            let outcome = self.validated_outcomes.get(&current)?;
            total += outcome.validated_metric_delta.as_f64();
            let block = self.blocks.get(&current)?;
            if !self.blocks.contains_key(&block.parent_id) {
                // Reached the genesis boundary (genesis blocks are not
                // stored in `blocks`).
                return Some(MetricValue::new(total));
            }
            current = block.parent_id;
        }
    }

    /// Gather valid branch tip metrics for dominance evaluation.
    ///
    /// Returns a map from block ID to validated metric value, including
    /// only tips whose derived validity is `DirectValid`. This ensures
    /// ancestry-poisoned blocks cannot participate in dominance evaluation.
    pub fn valid_tip_metrics(
        &self,
        domain_id: &DomainId,
    ) -> HashMap<BlockId, MetricValue> {
        let fork_state = match self.fork_states.get(domain_id) {
            Some(fs) => fs,
            None => return HashMap::new(),
        };

        let mut metrics = HashMap::new();
        for family in fork_state.families.values() {
            for tip in &family.branch_tips {
                if self.derived_validity(tip) == DerivedValidity::DirectValid {
                    // Compare cumulative chain improvement, not per-block
                    // deltas — tips at different depths have different
                    // baselines.
                    if let Some(metric) = self.cumulative_validated_delta(tip) {
                        metrics.insert(*tip, metric);
                    }
                }
            }
        }
        metrics
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

    /// Get the proposer-bond escrow record for a block.
    pub fn block_escrow(
        &self,
        block_id: &BlockId,
    ) -> Option<&EscrowRecord> {
        self.block_escrow_records(block_id)
            .into_iter()
            .find(|e| e.kind == EscrowKind::ProposerBond)
    }

    /// Get all escrow records for a block (bond and reward tranches),
    /// in creation order.
    pub fn block_escrow_records(
        &self,
        block_id: &BlockId,
    ) -> Vec<&EscrowRecord> {
        self.block_escrows
            .get(block_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|eid| self.escrow_records.get(eid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the challenger-bond escrow record for a challenge.
    pub fn challenge_escrow(
        &self,
        challenge_id: &ChallengeId,
    ) -> Option<&EscrowRecord> {
        self.challenge_escrows
            .get(challenge_id)
            .and_then(|eid| self.escrow_records.get(eid))
    }

    /// Get the slash distribution recorded for an upheld challenge.
    pub fn slash_distribution(
        &self,
        challenge_id: &ChallengeId,
    ) -> Option<&SlashDistribution> {
        self.slash_distributions.get(challenge_id)
    }
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::new()
    }
}
