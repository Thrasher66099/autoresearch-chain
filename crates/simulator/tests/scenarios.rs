// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0.2 scenario tests.
//!
//! These tests exercise the integrated protocol state machine through
//! realistic multi-step scenarios. Each test tells a story about the
//! protocol behaving (or correctly refusing to behave).

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::attestation::ProvisionalOutcome;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Create a simulator with a single activated domain (CIFAR-10 example).
///
/// Returns the simulator state, the domain ID, and the genesis block ID.
fn setup_active_domain() -> (SimulatorState, DomainId, GenesisBlockId) {
    let mut sim = SimulatorState::new();

    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;
    let domain_id = genesis.domain_id;

    // Submit and activate genesis.
    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();

    // Record 3 passing seed validations.
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

    // Register a validator pool.
    sim.register_validator_pool(ValidatorPool {
        domain_id,
        validators: (1..=10).map(test_validator_id).collect(),
    });

    (sim, domain_id, genesis_id)
}

/// Create a standard block for testing.
fn make_block(id: u8, parent_id: BlockId, domain_id: DomainId, delta: f64) -> Block {
    Block {
        id: test_block_id(id),
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

/// Submit a block, assign validators, record passing attestations, and
/// evaluate it. Returns the provisional outcome.
fn submit_and_validate(
    sim: &mut SimulatorState,
    block: Block,
) -> ProvisionalOutcome {
    let block_id = block.id;
    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // All validators pass.
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

// =======================================================================
// Scenario A: Valid genesis activates domain
// =======================================================================

#[test]
fn scenario_a_valid_genesis_activates_domain() {
    let (sim, domain_id, genesis_id) = setup_active_domain();

    // Domain is registered and active.
    assert!(sim.domain_registry.is_active(&domain_id));

    // Track tree exists.
    let tree = sim.domain_registry.get_track_tree(&genesis_id).unwrap();
    assert_eq!(tree.domain_id, domain_id);
    assert_eq!(tree.genesis_block_id, genesis_id);
    assert!(tree.fork_families.is_empty());
    assert!(tree.canonical_frontier_block_id.is_none());

    // Fork state initialized.
    assert!(sim.fork_states.contains_key(&domain_id));
}

// =======================================================================
// Scenario B: Invalid genesis fails
// =======================================================================

#[test]
fn scenario_b_structurally_invalid_genesis_rejected() {
    let mut sim = SimulatorState::new();
    let genesis = invalid_genesis_missing_metric_id();

    let result = sim.submit_genesis(genesis);
    assert!(result.is_err());
}

#[test]
fn scenario_b_rts_nonconformant_genesis_fails() {
    let mut sim = SimulatorState::new();
    let mut genesis = valid_genesis_block();
    genesis.time_budget_secs = 0; // violates RTS-1

    // Structural validation catches this first.
    let result = sim.submit_genesis(genesis);
    assert!(result.is_err());
}

#[test]
fn scenario_b_seed_validation_failure() {
    let mut sim = SimulatorState::new();
    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;

    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();

    // 3 validators all fail.
    for i in 1..=3 {
        sim.record_seed_validation(
            &genesis_id,
            SeedValidationRecord {
                validator: test_validator_id(i),
                vote: ValidatorVote::Fail,
                observed_score: None,
                timestamp: 1700000000 + i as u64,
            },
        )
        .unwrap();
    }

    let result = sim.finalize_activation(&genesis_id);
    assert!(result.is_err());
}

#[test]
fn scenario_b_fraud_suspected_kills_genesis() {
    let mut sim = SimulatorState::new();
    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;

    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();

    // 2 pass, 1 fraud.
    for i in 1..=2 {
        sim.record_seed_validation(
            &genesis_id,
            SeedValidationRecord {
                validator: test_validator_id(i),
                vote: ValidatorVote::Pass,
                observed_score: Some(MetricValue::new(0.93)),
                timestamp: 1700000000 + i as u64,
            },
        )
        .unwrap();
    }
    sim.record_seed_validation(
        &genesis_id,
        SeedValidationRecord {
            validator: test_validator_id(3),
            vote: ValidatorVote::FraudSuspected,
            observed_score: None,
            timestamp: 1700000003,
        },
    )
    .unwrap();

    let result = sim.finalize_activation(&genesis_id);
    assert!(result.is_err());
}

// =======================================================================
// Scenario C: Valid block gets accepted
// =======================================================================

#[test]
fn scenario_c_valid_block_accepted() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    let outcome = submit_and_validate(&mut sim, block);
    assert_eq!(outcome, ProvisionalOutcome::Accepted);

    // Block should be in UnderChallenge (challenge window opened).
    assert_eq!(
        sim.block_status(&block_id),
        Some(BlockStatus::UnderChallenge)
    );

    // Frontier should be updated.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_id));
}

#[test]
fn scenario_c_full_block_lifecycle() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    submit_and_validate(&mut sim, block);
    assert_eq!(
        sim.block_status(&block_id),
        Some(BlockStatus::UnderChallenge)
    );

    sim.close_challenge_window(&block_id).unwrap();
    assert_eq!(
        sim.block_status(&block_id),
        Some(BlockStatus::ChallengeWindowClosed)
    );

    sim.settle_block(&block_id).unwrap();
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Settled));

    sim.finalize_block(&block_id).unwrap();
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Final));
}

// =======================================================================
// Scenario D: Invalid block rejected
// =======================================================================

#[test]
fn scenario_d_block_rejected_by_validators() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // All validators fail.
    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id,
            validator: *v,
            vote: ValidatorVote::Fail,
            observed_delta: None,
            replay_evidence_ref: test_artifact_hash(70),
            timestamp: 1700002000,
        })
        .unwrap();
    }

    let outcome = sim.evaluate_block(&block_id).unwrap();
    assert_eq!(outcome, ProvisionalOutcome::Rejected);
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Rejected));
}

#[test]
fn scenario_d_block_rejected_by_fraud() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // 2 pass, 1 fraud suspected.
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[0],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.015)),
        replay_evidence_ref: test_artifact_hash(70),
        timestamp: 1700002000,
    })
    .unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[1],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.014)),
        replay_evidence_ref: test_artifact_hash(71),
        timestamp: 1700002001,
    })
    .unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[2],
        vote: ValidatorVote::FraudSuspected,
        observed_delta: None,
        replay_evidence_ref: test_artifact_hash(72),
        timestamp: 1700002002,
    })
    .unwrap();

    let outcome = sim.evaluate_block(&block_id).unwrap();
    assert_eq!(outcome, ProvisionalOutcome::Rejected);
}

// =======================================================================
// Scenario E: Sibling accepted children create fork family
// =======================================================================

#[test]
fn scenario_e_fork_family_created() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Two competing blocks off the same parent.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_b = make_block(11, parent, domain_id, 0.02);

    let block_a_id = block_a.id;
    let block_b_id = block_b.id;

    submit_and_validate(&mut sim, block_a);
    submit_and_validate(&mut sim, block_b);

    // Both accepted.
    assert_eq!(
        sim.block_status(&block_a_id),
        Some(BlockStatus::UnderChallenge)
    );
    assert_eq!(
        sim.block_status(&block_b_id),
        Some(BlockStatus::UnderChallenge)
    );

    // A fork family should exist.
    let families = sim.fork_families(&domain_id);
    assert_eq!(families.len(), 1);

    let family = families[0];
    assert_eq!(family.branch_tips.len(), 2);
    assert!(family.branch_tips.contains(&block_a_id));
    assert!(family.branch_tips.contains(&block_b_id));
    assert_eq!(family.common_ancestor_id, parent);
}

#[test]
fn scenario_e_three_way_fork() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_b = make_block(11, parent, domain_id, 0.02);
    let block_c = make_block(12, parent, domain_id, 0.03);

    submit_and_validate(&mut sim, block_a);
    submit_and_validate(&mut sim, block_b);
    submit_and_validate(&mut sim, block_c);

    let families = sim.fork_families(&domain_id);
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].branch_tips.len(), 3);
}

// =======================================================================
// Scenario F: Canonical frontier updates
// =======================================================================

#[test]
fn scenario_f_frontier_tracks_best_block() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Block with delta 0.01.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate(&mut sim, block_a);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));

    // Better block with delta 0.03.
    let block_b = make_block(11, parent, domain_id, 0.03);
    let block_b_id = block_b.id;
    submit_and_validate(&mut sim, block_b);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Worse block with delta 0.005 — frontier should NOT change.
    let block_c = make_block(12, parent, domain_id, 0.005);
    submit_and_validate(&mut sim, block_c);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));
}

#[test]
fn scenario_f_chain_of_improvements() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Linear chain: genesis → block_a → block_b → block_c
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate(&mut sim, block_a);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate(&mut sim, block_b);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    let block_c = make_block(12, block_b_id, domain_id, 0.03);
    let block_c_id = block_c.id;
    submit_and_validate(&mut sim, block_c);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_c_id));
}

// =======================================================================
// Scenario G: Challenges can open against valid targets
// =======================================================================

#[test]
fn scenario_g_challenge_opens_against_block_under_challenge() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Block is UnderChallenge — challenges should be accepted.
    assert_eq!(
        sim.block_status(&block_id),
        Some(BlockStatus::UnderChallenge)
    );

    let challenge_id = sim
        .open_challenge(
            test_challenge_id(1),
            ChallengeType::BlockReplay,
            ChallengeTarget::Block { block_id },
            test_participant_id(5),
            TokenAmount::new(200),
            test_artifact_hash(80),
        )
        .unwrap();

    assert!(sim.challenges.contains_key(&challenge_id));
    assert_eq!(
        sim.challenges[&challenge_id].status,
        ChallengeStatus::Open
    );
}

#[test]
fn scenario_g_challenge_rejected_for_submitted_block() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    sim.submit_block(block).unwrap();

    // Block is Submitted — not challengeable.
    let result = sim.open_challenge(
        test_challenge_id(1),
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    );
    assert!(result.is_err());
}

// =======================================================================
// Scenario H: Multi-domain isolation
// =======================================================================

#[test]
fn scenario_h_two_domains_independent() {
    let mut sim = SimulatorState::new();

    // Activate domain 1.
    let genesis_1 = valid_genesis_block();
    let gid_1 = genesis_1.id;
    let did_1 = genesis_1.domain_id;
    sim.submit_genesis(genesis_1).unwrap();
    sim.evaluate_conformance(&gid_1).unwrap();
    for i in 1..=3 {
        sim.record_seed_validation(
            &gid_1,
            SeedValidationRecord {
                validator: test_validator_id(i),
                vote: ValidatorVote::Pass,
                observed_score: Some(MetricValue::new(0.93)),
                timestamp: 1700000000 + i as u64,
            },
        )
        .unwrap();
    }
    sim.finalize_activation(&gid_1).unwrap();
    sim.register_validator_pool(ValidatorPool {
        domain_id: did_1,
        validators: (1..=10).map(test_validator_id).collect(),
    });

    // Activate domain 2 (different IDs).
    let mut genesis_2 = valid_genesis_block();
    genesis_2.id = test_genesis_block_id(2);
    genesis_2.domain_id = test_domain_id(2);
    let gid_2 = genesis_2.id;
    let did_2 = genesis_2.domain_id;
    sim.submit_genesis(genesis_2).unwrap();
    sim.evaluate_conformance(&gid_2).unwrap();
    for i in 1..=3 {
        sim.record_seed_validation(
            &gid_2,
            SeedValidationRecord {
                validator: test_validator_id(i + 10),
                vote: ValidatorVote::Pass,
                observed_score: Some(MetricValue::new(0.93)),
                timestamp: 1700000000 + i as u64,
            },
        )
        .unwrap();
    }
    sim.finalize_activation(&gid_2).unwrap();
    sim.register_validator_pool(ValidatorPool {
        domain_id: did_2,
        validators: (11..=20).map(test_validator_id).collect(),
    });

    // Submit blocks to each domain independently.
    let block_1 = make_block(10, gid_1.as_block_id(), did_1, 0.01);
    let block_1_id = block_1.id;
    submit_and_validate(&mut sim, block_1);

    let block_2 = make_block(20, gid_2.as_block_id(), did_2, 0.02);
    let block_2_id = block_2.id;
    submit_and_validate(&mut sim, block_2);

    // Each domain has its own frontier.
    assert_eq!(sim.canonical_frontier(&did_1), Some(block_1_id));
    assert_eq!(sim.canonical_frontier(&did_2), Some(block_2_id));

    // Domains don't share fork families.
    assert!(sim.fork_families(&did_1).is_empty()); // only 1 block per domain
    assert!(sim.fork_families(&did_2).is_empty());
}

// =======================================================================
// Scenario I: Block against inactive domain rejected
// =======================================================================

#[test]
fn scenario_i_block_against_nonexistent_domain_rejected() {
    let mut sim = SimulatorState::new();

    let block = make_block(10, test_block_id(1), test_domain_id(99), 0.01);
    let result = sim.submit_block(block);
    assert!(result.is_err());
}

// =======================================================================
// Scenario J: Attestation summary computation
// =======================================================================

#[test]
fn scenario_j_attestation_summary() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // Mixed attestations.
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[0],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.014)),
        replay_evidence_ref: test_artifact_hash(70),
        timestamp: 1700002000,
    })
    .unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[1],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.016)),
        replay_evidence_ref: test_artifact_hash(71),
        timestamp: 1700002001,
    })
    .unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[2],
        vote: ValidatorVote::Inconclusive,
        observed_delta: None,
        replay_evidence_ref: test_artifact_hash(72),
        timestamp: 1700002002,
    })
    .unwrap();

    let summary = sim.attestation_summary(&block_id).unwrap();
    assert_eq!(summary.total, 3);
    assert_eq!(summary.pass_count, 2);
    assert_eq!(summary.inconclusive_count, 1);
    assert!(summary.mean_observed_delta.is_some());
    let mean = summary.mean_observed_delta.unwrap();
    assert!((mean - 0.015).abs() < 1e-10);
}
