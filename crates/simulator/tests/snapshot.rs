// SPDX-License-Identifier: AGPL-3.0-or-later

//! Snapshot persistence tests.
//!
//! These tests prove that SimulatorState can be serialized to JSON,
//! deserialized back, and that the round-trip preserves protocol state
//! faithfully — including the ability to continue operating after load.

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::attestation::ProvisionalOutcome;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

// -----------------------------------------------------------------------
// Helpers (shared with scenarios.rs pattern)
// -----------------------------------------------------------------------

/// Create a simulator with a single activated domain.
fn setup_active_domain() -> (SimulatorState, DomainId, GenesisBlockId) {
    let mut sim = SimulatorState::new();

    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;
    let domain_id = genesis.domain_id;

    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();

    for i in 1..=3 {
        sim.record_seed_validation(
            &genesis_id,
            SeedValidationRecord {
                validator: test_validator_id(i),
                vote: ValidatorVote::Pass,
                observed_score: Some(MetricValue::new(0.9300)),
                timestamp: 1700000000 + i as u64,
            },
        )
        .unwrap();
    }

    sim.finalize_activation(&genesis_id).unwrap();

    sim.register_validator_pool(ValidatorPool {
        domain_id,
        validators: (1..=10).map(test_validator_id).collect(),
    });

    (sim, domain_id, genesis_id)
}

/// Create a standard block for testing.
///
/// Block IDs start at 10 to avoid collision with genesis block IDs
/// (test_genesis_block_id(1) == test_block_id(1) because both use [n; 32]).
fn make_block(id: u8, parent_id: BlockId, domain_id: DomainId, delta: f64) -> Block {
    let block_id = id + 10; // Offset to avoid collision with genesis IDs
    Block {
        id: test_block_id(block_id),
        domain_id,
        parent_id,
        proposer: test_proposer_id(1),
        child_state_ref: test_artifact_hash(60 + id),
        diff_ref: test_artifact_hash(160 + id),
        claimed_metric_delta: MetricValue::new(delta),
        evidence_bundle_hash: test_artifact_hash(200 + id),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: EpochId(1),
        status: BlockStatus::Submitted,
        timestamp: 1700001000 + id as u64 * 1000,
    }
}

/// Submit a block, assign validators, record passing attestations, evaluate.
fn submit_and_validate(
    sim: &mut SimulatorState,
    block: Block,
) -> ProvisionalOutcome {
    let block_id = block.id;
    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(0.015)),
            replay_evidence_ref: test_artifact_hash(70),
            timestamp: 1700002000,
        })
        .unwrap();
    }

    sim.evaluate_block(&block_id).unwrap()
}

/// Serialize → deserialize round-trip via JSON.
fn round_trip(state: &SimulatorState) -> SimulatorState {
    let json = serde_json::to_string(state).expect("serialize failed");
    serde_json::from_str(&json).expect("deserialize failed")
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[test]
fn empty_state_round_trip() {
    let state = SimulatorState::new();
    let loaded = round_trip(&state);

    assert_eq!(state.current_epoch, loaded.current_epoch);
    assert!(loaded.blocks.is_empty());
    assert!(loaded.attestations.is_empty());
    assert!(loaded.challenges.is_empty());
    assert!(loaded.escrow_records.is_empty());
}

#[test]
fn round_trip_preserves_config() {
    let mut state = SimulatorState::new();
    state.genesis_config.min_seed_validations = 7;
    state.validation_config.validators_per_block = 5;
    state.challenge_config.min_challenge_bond = 999;
    state.reward_config.challenge_window_epochs = 42;

    let loaded = round_trip(&state);

    assert_eq!(loaded.genesis_config.min_seed_validations, 7);
    assert_eq!(loaded.validation_config.validators_per_block, 5);
    assert_eq!(loaded.challenge_config.min_challenge_bond, 999);
    assert_eq!(loaded.reward_config.challenge_window_epochs, 42);
}

#[test]
fn nontrivial_state_round_trip() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Submit and validate a block.
    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    let outcome = submit_and_validate(&mut sim, block1);
    assert_eq!(outcome, ProvisionalOutcome::Accepted);

    // Advance epoch.
    sim.advance_epoch();

    // Submit a second block (child of block 1).
    let block2 = make_block(2, block1_id, domain_id, 0.010);
    let block2_id = block2.id;
    let outcome2 = submit_and_validate(&mut sim, block2);
    assert_eq!(outcome2, ProvisionalOutcome::Accepted);

    // --- Round trip ---
    let loaded = round_trip(&sim);

    // Verify structural state survived.
    assert_eq!(loaded.current_epoch, sim.current_epoch);
    assert_eq!(loaded.blocks.len(), sim.blocks.len());
    assert_eq!(loaded.attestations.len(), sim.attestations.len());
    assert_eq!(loaded.validated_outcomes.len(), sim.validated_outcomes.len());
    assert_eq!(loaded.escrow_records.len(), sim.escrow_records.len());
    assert_eq!(loaded.block_escrows.len(), sim.block_escrows.len());
    assert_eq!(loaded.children.len(), sim.children.len());
    assert_eq!(loaded.fork_states.len(), sim.fork_states.len());

    // Verify specific blocks survived.
    assert!(loaded.blocks.contains_key(&block1_id));
    assert!(loaded.blocks.contains_key(&block2_id));
    assert_eq!(
        loaded.blocks[&block1_id].status,
        sim.blocks[&block1_id].status,
    );
    assert_eq!(
        loaded.blocks[&block2_id].status,
        sim.blocks[&block2_id].status,
    );

    // Verify validated outcomes survived.
    let orig_outcome = sim.validated_outcome(&block1_id).unwrap();
    let loaded_outcome = loaded.validated_outcome(&block1_id).unwrap();
    assert_eq!(
        orig_outcome.validated_metric_delta.as_f64(),
        loaded_outcome.validated_metric_delta.as_f64(),
    );

    // Verify domain registry survived.
    assert!(loaded.domain_registry.is_active(&domain_id));

    // Verify frontier survived.
    assert_eq!(
        loaded.canonical_frontier(&domain_id),
        sim.canonical_frontier(&domain_id),
    );
}

#[test]
fn loaded_state_can_continue_operating() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Submit and validate one block.
    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    submit_and_validate(&mut sim, block1);

    // Round-trip.
    let mut loaded = round_trip(&sim);

    // The loaded state should be able to continue protocol operations.
    // Submit a new block on top of block 1.
    let block2 = make_block(2, block1_id, domain_id, 0.010);
    let block2_id = block2.id;
    let outcome = submit_and_validate(&mut loaded, block2);
    assert_eq!(outcome, ProvisionalOutcome::Accepted);

    // The new block should be in the loaded state.
    assert!(loaded.blocks.contains_key(&block2_id));
    assert_eq!(loaded.blocks.len(), 2);

    // Validated outcome should exist for the new block.
    let new_outcome = loaded.validated_outcome(&block2_id);
    assert!(new_outcome.is_some());

    // Escrow should exist for the new block.
    let new_escrow = loaded.block_escrow(&block2_id);
    assert!(new_escrow.is_some());
}

#[test]
fn round_trip_with_challenge() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Submit and validate a block.
    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    submit_and_validate(&mut sim, block1);

    // Open a challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block1_id },
        test_participant_id(10),
        TokenAmount::new(200),
        test_artifact_hash(99),
    )
    .unwrap();

    // Round-trip.
    let loaded = round_trip(&sim);

    // Challenge should survive.
    assert_eq!(loaded.challenges.len(), 1);
    assert!(loaded.challenges.contains_key(&challenge_id));
    let loaded_challenge = &loaded.challenges[&challenge_id];
    assert_eq!(loaded_challenge.status, ChallengeStatus::Open);
}

#[test]
fn round_trip_with_upheld_challenge_and_invalidation() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Submit and validate two blocks.
    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    submit_and_validate(&mut sim, block1);

    let block2 = make_block(2, block1_id, domain_id, 0.010);
    let block2_id = block2.id;
    submit_and_validate(&mut sim, block2);

    // Challenge and uphold against block 1.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block1_id },
        test_participant_id(10),
        TokenAmount::new(200),
        test_artifact_hash(99),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block 1 should be invalidated, block 2 ancestry-invalid.
    assert_eq!(sim.derived_validity(&block1_id), DerivedValidity::DirectInvalid);
    assert_eq!(sim.derived_validity(&block2_id), DerivedValidity::AncestryInvalid);

    // Round-trip.
    let loaded = round_trip(&sim);

    // Derived validity should be preserved.
    assert_eq!(loaded.derived_validity(&block1_id), DerivedValidity::DirectInvalid);
    assert_eq!(loaded.derived_validity(&block2_id), DerivedValidity::AncestryInvalid);

    // Escrow for block 1 should be slashed.
    let escrow = loaded.block_escrow(&block1_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Slashed);
}

#[test]
fn round_trip_with_epoch_advancement() {
    let (mut sim, _domain_id, _genesis_id) = setup_active_domain();

    for _ in 0..10 {
        sim.advance_epoch();
    }

    let loaded = round_trip(&sim);
    assert_eq!(loaded.current_epoch, sim.current_epoch);
    assert_eq!(loaded.current_epoch.0, 10);
}

#[test]
fn round_trip_preserves_pending_activations() {
    let mut sim = SimulatorState::new();

    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;

    // Submit but don't finalize — should remain in pending_activations.
    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();

    assert_eq!(sim.pending_activations.len(), 1);

    let loaded = round_trip(&sim);
    assert_eq!(loaded.pending_activations.len(), 1);
    assert!(loaded.pending_activations.contains_key(&genesis_id));
}

#[test]
fn loaded_state_can_settle_blocks() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    submit_and_validate(&mut sim, block1);

    // Close challenge window.
    sim.close_challenge_window(&block1_id).unwrap();

    // Advance past the escrow release epoch.
    for _ in 0..10 {
        sim.advance_epoch();
    }

    // Round-trip, then settle on the loaded state.
    let mut loaded = round_trip(&sim);
    loaded.settle_block(&block1_id).unwrap();

    assert_eq!(
        loaded.block_status(&block1_id),
        Some(BlockStatus::Settled),
    );

    // Escrow should be released.
    let escrow = loaded.block_escrow(&block1_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn json_output_is_human_inspectable() {
    let state = SimulatorState::new();
    let json = serde_json::to_string_pretty(&state).expect("serialize failed");

    // Should contain recognizable field names.
    assert!(json.contains("current_epoch"));
    assert!(json.contains("blocks"));
    assert!(json.contains("challenges"));
    assert!(json.contains("escrow_records"));
    assert!(json.contains("fork_family_counter"));
}

#[test]
fn round_trip_preserves_validator_assignments() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block1 = make_block(1, genesis_id.as_block_id(), domain_id, 0.015);
    let block1_id = block1.id;
    sim.submit_block(block1).unwrap();
    let assigned = sim.assign_validators(&block1_id).unwrap();

    let loaded = round_trip(&sim);

    let loaded_assignments = loaded.validator_assignments.get(&block1_id).unwrap();
    assert_eq!(loaded_assignments, &assigned);
}
