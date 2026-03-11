// SPDX-License-Identifier: AGPL-3.0-or-later

//! Protocol scenario tests.
//!
//! These tests exercise the integrated protocol state machine through
//! realistic multi-step scenarios. Each test tells a story about the
//! protocol behaving (or correctly refusing to behave).
//!
//! Scenarios A-J: Phase 0.2 (basic lifecycle, validation, forks, challenges)
//! Scenarios K-Q: Phase 0.3 (challenge consequences, validated frontier,
//!                escrow, invalidation, descendant handling)
//! Scenarios T-Z: Phase 0.3d (derived branch validity, ancestry-invalid
//!                settlement/frontier/dominance/escrow gating)

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::attestation::ProvisionalOutcome;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

// Phase 0.3d: DerivedValidity is used in branch-truth tests.
use arc_protocol_types::DerivedValidity;

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

/// Submit a block with custom observed deltas for validators.
/// All validators pass, but with the specified observed delta.
fn submit_and_validate_with_delta(
    sim: &mut SimulatorState,
    block: Block,
    observed_delta: f64,
) -> ProvisionalOutcome {
    let block_id = block.id;
    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(observed_delta)),
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

    // Advance past the challenge window (release_epoch = 0 + 5 = 5).
    for _ in 0..5 {
        sim.advance_epoch();
    }

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

    // Block with validated delta 0.01.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));

    // Better block with validated delta 0.03.
    let block_b = make_block(11, parent, domain_id, 0.03);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.03);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Worse block with validated delta 0.005 — frontier should NOT change.
    let block_c = make_block(12, parent, domain_id, 0.005);
    submit_and_validate_with_delta(&mut sim, block_c, 0.005);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));
}

#[test]
fn scenario_f_chain_of_improvements() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Linear chain: genesis → block_a → block_b → block_c
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    let block_c = make_block(12, block_b_id, domain_id, 0.03);
    let block_c_id = block_c.id;
    submit_and_validate_with_delta(&mut sim, block_c, 0.03);
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
    assert_eq!(summary.truth_bearing_pass_count, 2);
    assert_eq!(summary.inconclusive_count, 1);
    assert!(summary.mean_observed_delta.is_some());
    let mean = summary.mean_observed_delta.unwrap();
    assert!((mean - 0.015).abs() < 1e-10);
}

// =======================================================================
// Phase 0.3 Scenario Tests
// =======================================================================

// =======================================================================
// Scenario K: Upheld challenge invalidates an accepted block
// =======================================================================

#[test]
fn scenario_k_upheld_challenge_invalidates_block() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::UnderChallenge));

    // Open challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();

    // Review and uphold.
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block should now be Invalidated.
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Invalidated));

    // Challenge should be Upheld.
    assert_eq!(sim.challenges[&challenge_id].status, ChallengeStatus::Upheld);

    // Validated outcome should be removed.
    assert!(sim.validated_outcome(&block_id).is_none());
}

#[test]
fn scenario_k_upheld_challenge_slashes_escrow() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Escrow should exist and be Held.
    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Held);
    assert_eq!(escrow.amount, TokenAmount::new(500)); // block bond

    // Uphold challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Escrow should now be Slashed.
    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Slashed);
}

// =======================================================================
// Scenario L: Upheld challenge prevents/reverses frontier advancement
// =======================================================================

#[test]
fn scenario_l_upheld_challenge_reverses_frontier() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Block A: observed delta 0.01.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    // Block B: observed delta 0.03 (better, becomes frontier).
    let block_b = make_block(11, parent, domain_id, 0.03);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.03);

    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Challenge and invalidate block B (the frontier).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_b_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block B invalidated.
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::Invalidated));

    // Frontier should now fall back to block A.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));
}

#[test]
fn scenario_l_invalidating_only_block_clears_frontier() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_id));

    // Invalidate the only block.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // No frontier should exist.
    assert!(sim.canonical_frontier(&domain_id).is_none());
}

// =======================================================================
// Scenario M: Rejected challenge preserves accepted state
// =======================================================================

#[test]
fn scenario_m_rejected_challenge_preserves_state() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::UnderChallenge));
    let frontier_before = sim.canonical_frontier(&domain_id);

    // Open, review, and reject the challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.reject_challenge(&challenge_id).unwrap();

    // Block status unchanged.
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::UnderChallenge));

    // Frontier unchanged.
    assert_eq!(sim.canonical_frontier(&domain_id), frontier_before);

    // Escrow still Held.
    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Held);

    // Validated outcome still present.
    assert!(sim.validated_outcome(&block_id).is_some());

    // Challenge marked Rejected.
    assert_eq!(sim.challenges[&challenge_id].status, ChallengeStatus::Rejected);
}

// =======================================================================
// Scenario N: Frontier selection uses validated metrics
// =======================================================================

#[test]
fn scenario_n_frontier_uses_validated_not_claimed() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Block A: claims delta 0.05, but validators observe only 0.01.
    let block_a = make_block(10, parent, domain_id, 0.05);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    // Block B: claims delta 0.01, but validators observe 0.03.
    let block_b = make_block(11, parent, domain_id, 0.01);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.03);

    // If frontier used claimed values, block A (0.05) would win.
    // With validated values, block B (0.03) wins over block A (0.01).
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Validate the stored outcomes match the observed values.
    let outcome_a = sim.validated_outcome(&block_a_id).unwrap();
    assert!((outcome_a.validated_metric_delta.as_f64() - 0.01).abs() < 1e-10);

    let outcome_b = sim.validated_outcome(&block_b_id).unwrap();
    assert!((outcome_b.validated_metric_delta.as_f64() - 0.03).abs() < 1e-10);
}

// =======================================================================
// Scenario O: Fork dominance ignores invalidated branches
// =======================================================================

#[test]
fn scenario_o_dominance_ignores_invalidated_branch() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Create a fork: two competing blocks off genesis.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, parent, domain_id, 0.05);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.05);

    // Block B is frontier (higher validated metric).
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Fork family should exist with both tips.
    let families = sim.fork_families(&domain_id);
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].branch_tips.len(), 2);

    // Invalidate block B.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_b_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block B removed from branch tips.
    let families = sim.fork_families(&domain_id);
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].branch_tips.len(), 1);
    assert!(families[0].branch_tips.contains(&block_a_id));
    assert!(!families[0].branch_tips.contains(&block_b_id));

    // Frontier falls back to block A.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_a_id));
}

// =======================================================================
// Scenario P: Accepted blocks create escrow records
// =======================================================================

#[test]
fn scenario_p_accepted_block_creates_escrow() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    // Before acceptance: no escrow.
    assert!(sim.block_escrow(&block_id).is_none());

    submit_and_validate(&mut sim, block);

    // After acceptance: escrow exists and is Held.
    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Held);
    assert_eq!(escrow.block_id, block_id);
    assert_eq!(escrow.amount, TokenAmount::new(500));
}

#[test]
fn scenario_p_settlement_releases_escrow() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Escrow is Held.
    assert_eq!(sim.block_escrow(&block_id).unwrap().status, EscrowStatus::Held);

    // Settle the block (advance past release_epoch first).
    sim.close_challenge_window(&block_id).unwrap();
    for _ in 0..5 {
        sim.advance_epoch();
    }
    sim.settle_block(&block_id).unwrap();

    // Escrow is Released.
    assert_eq!(sim.block_escrow(&block_id).unwrap().status, EscrowStatus::Released);
}

#[test]
fn scenario_p_validated_outcome_records_attestation_truth() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Validated outcome should exist.
    let outcome = sim.validated_outcome(&block_id).unwrap();
    assert_eq!(outcome.block_id, block_id);
    assert_eq!(outcome.attestation_count, 3); // 3 validators
    // All validators reported 0.015 in submit_and_validate.
    assert!((outcome.validated_metric_delta.as_f64() - 0.015).abs() < 1e-10);
}

// =======================================================================
// Scenario Q: Descendant handling after parent invalidation
// =======================================================================

#[test]
fn scenario_q_child_on_invalidated_parent_excluded_from_frontier() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → block_a → block_b
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.05);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.05);

    // Block B is frontier (0.05 > 0.01).
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Invalidate block A (the parent).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block A is Invalidated.
    assert_eq!(sim.block_status(&block_a_id), Some(BlockStatus::Invalidated));

    // Block B's status is NOT automatically changed (Phase 0.3 does not
    // cascade invalidation to descendants).
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::UnderChallenge));

    // But block B should NOT be the frontier, because its ancestor (A)
    // is invalidated. The protocol recognizes this via is_on_valid_chain.
    // The frontier should be cleared since no valid candidates remain.
    assert!(sim.canonical_frontier(&domain_id).is_none());

    // Verify is_on_valid_chain correctly identifies the tainted chain.
    assert!(!sim.is_on_valid_chain(&block_b_id));
    assert!(!sim.is_on_valid_chain(&block_a_id));
}

#[test]
fn scenario_q_sibling_survives_parent_invalidation() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Two independent blocks off genesis.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, parent, domain_id, 0.03);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.03);

    // Block B is frontier.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Invalidate block A — block B should be unaffected.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Block A invalidated, block B unaffected.
    assert_eq!(sim.block_status(&block_a_id), Some(BlockStatus::Invalidated));
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::UnderChallenge));

    // Frontier should still be block B.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Block B is on a valid chain (its parent is genesis, not block A).
    assert!(sim.is_on_valid_chain(&block_b_id));
}

#[test]
fn scenario_q_cannot_submit_child_against_invalidated_parent() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate(&mut sim, block_a);

    // Invalidate block A.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Try to submit a child against invalidated block A.
    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let result = sim.submit_block(block_b);

    // Should fail because parent is not in accepted state.
    assert!(result.is_err());
}

// =======================================================================
// Phase 0.3b Scenario Tests
// =======================================================================

// =======================================================================
// Scenario R: Acceptance requires validator-observed deltas
// =======================================================================

/// Phase 0.3c: Pass attestations without `observed_delta` do not count
/// toward acceptance quorum. A block with nominal Pass votes but no
/// truth-bearing Passes must not be Accepted.
#[test]
fn scenario_r_pass_without_observed_delta_does_not_count_toward_acceptance() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // All validators Pass but WITHOUT observed_delta.
    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: None,
            replay_evidence_ref: test_artifact_hash(70),
            timestamp: 1700002000,
        })
        .unwrap();
    }

    // Evaluation succeeds (no error), but the outcome is not Accepted
    // because truth-bearing pass count is 0, below quorum.
    let outcome = sim.evaluate_block(&block_id).unwrap();
    assert_ne!(outcome, ProvisionalOutcome::Accepted,
        "Pass without observed_delta must not count toward acceptance");

    // Block should be Rejected (inconclusive_is_rejection = true by default).
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Rejected));

    // No validated outcome should exist.
    assert!(sim.validated_outcome(&block_id).is_none());

    // No escrow should exist.
    assert!(sim.block_escrow(&block_id).is_none());
}

/// Phase 0.3c: When some Pass attestations have observed_delta and some
/// don't, only the truth-bearing ones count toward quorum.
#[test]
fn scenario_r_mixed_pass_with_and_without_delta() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // 1 truth-bearing Pass, 2 Pass-without-delta.
    // Default quorum = 2, so this should NOT be accepted.
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[0],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.015)),
        replay_evidence_ref: test_artifact_hash(70),
        timestamp: 1700002000,
    }).unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[1],
        vote: ValidatorVote::Pass,
        observed_delta: None,
        replay_evidence_ref: test_artifact_hash(71),
        timestamp: 1700002001,
    }).unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[2],
        vote: ValidatorVote::Pass,
        observed_delta: None,
        replay_evidence_ref: test_artifact_hash(72),
        timestamp: 1700002002,
    }).unwrap();

    let outcome = sim.evaluate_block(&block_id).unwrap();
    assert_ne!(outcome, ProvisionalOutcome::Accepted,
        "only 1 truth-bearing Pass, below quorum of 2");
}

/// Phase 0.3c: When enough truth-bearing Passes exist (even with some
/// non-truth-bearing ones), block is accepted and mean delta uses only
/// truth-bearing attestations.
#[test]
fn scenario_r_enough_truth_bearing_passes_accepted() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;

    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();

    // 2 truth-bearing Passes (quorum = 2), 1 Pass without delta.
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[0],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.010)),
        replay_evidence_ref: test_artifact_hash(70),
        timestamp: 1700002000,
    }).unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[1],
        vote: ValidatorVote::Pass,
        observed_delta: Some(MetricValue::new(0.020)),
        replay_evidence_ref: test_artifact_hash(71),
        timestamp: 1700002001,
    }).unwrap();
    sim.record_attestation(ValidationAttestation {
        block_id,
        validator: assigned[2],
        vote: ValidatorVote::Pass,
        observed_delta: None,
        replay_evidence_ref: test_artifact_hash(72),
        timestamp: 1700002002,
    }).unwrap();

    let outcome = sim.evaluate_block(&block_id).unwrap();
    assert_eq!(outcome, ProvisionalOutcome::Accepted);

    // Validated outcome should exist with mean of 0.010 and 0.020 only
    // (the Pass without delta is excluded from the mean).
    let validated = sim.validated_outcome(&block_id).unwrap();
    assert!((validated.validated_metric_delta.as_f64() - 0.015).abs() < 1e-10,
        "mean delta must use only truth-bearing Pass attestations");
}

/// Phase 0.3c: Every Accepted block must have a constructible
/// validated_metric_delta. This is a simulator-level invariant check.
#[test]
fn scenario_r_accepted_implies_validated_delta_exists() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Accept several blocks and verify each has a validated outcome.
    for i in 10..=12 {
        let block = make_block(i, genesis_id.as_block_id(), domain_id, 0.01 * i as f64);
        let block_id = block.id;
        let outcome = submit_and_validate(&mut sim, block);
        assert_eq!(outcome, ProvisionalOutcome::Accepted);

        let validated = sim.validated_outcome(&block_id);
        assert!(validated.is_some(),
            "accepted block {} must have validated outcome", block_id);

        let delta = validated.unwrap().validated_metric_delta.as_f64();
        assert!(delta.is_finite(),
            "validated delta for block {} must be finite", block_id);
    }
}

#[test]
fn scenario_r_validated_outcome_uses_observed_delta_not_claim() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Block claims delta 0.10 but validators observe 0.02.
    let block = make_block(10, parent, domain_id, 0.10);
    let block_id = block.id;
    submit_and_validate_with_delta(&mut sim, block, 0.02);

    let outcome = sim.validated_outcome(&block_id).unwrap();
    // Protocol truth should be the observed 0.02, not the claimed 0.10.
    assert!(
        (outcome.validated_metric_delta.as_f64() - 0.02).abs() < 1e-10,
        "protocol truth must use observed delta, not claimed delta"
    );
}

// =======================================================================
// Scenario S: Escrow release timing enforcement
// =======================================================================

#[test]
fn scenario_s_early_settlement_rejected() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    sim.close_challenge_window(&block_id).unwrap();

    // Do NOT advance epoch — current_epoch is still 0,
    // but release_epoch is 5. Settlement should fail.
    let result = sim.settle_block(&block_id);
    assert!(result.is_err(), "settlement before release_epoch must fail");

    // Block lifecycle advanced to ChallengeWindowClosed but escrow
    // should still be Held.
    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Held);
}

#[test]
fn scenario_s_settlement_at_release_epoch_succeeds() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    sim.close_challenge_window(&block_id).unwrap();

    // Advance to exactly the release epoch (0 + 5 = 5).
    for _ in 0..5 {
        sim.advance_epoch();
    }

    sim.settle_block(&block_id).unwrap();

    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn scenario_s_settlement_after_release_epoch_succeeds() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    sim.close_challenge_window(&block_id).unwrap();

    // Advance well past release_epoch.
    for _ in 0..10 {
        sim.advance_epoch();
    }

    sim.settle_block(&block_id).unwrap();

    let escrow = sim.block_escrow(&block_id).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// =======================================================================
// Phase 0.3d Scenario Tests: Derived Branch Validity
// =======================================================================

// =======================================================================
// Scenario T: DerivedValidity enum classification
// =======================================================================

/// T1: A directly invalidated block is reported as DirectInvalid.
#[test]
fn scenario_t1_direct_invalidation_classification() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Before invalidation: DirectValid.
    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectValid);

    // Invalidate via upheld challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // After invalidation: DirectInvalid.
    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectInvalid);
}

/// T2: A descendant of an invalidated block is reported as AncestryInvalid.
#[test]
fn scenario_t2_ancestry_invalidation_classification() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → block_a → block_b
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);

    // Both are DirectValid before invalidation.
    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectValid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::DirectValid);

    // Invalidate block_a (the parent).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_a is DirectInvalid, block_b is AncestryInvalid.
    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectInvalid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::AncestryInvalid);

    // block_b's stored status is NOT changed — it still looks locally accepted.
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::UnderChallenge));
}

/// T3: A valid sibling/unrelated branch remains DirectValid after
/// another branch is invalidated.
#[test]
fn scenario_t3_valid_sibling_remains_direct_valid() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Two independent branches off genesis.
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, parent, domain_id, 0.03);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.03);

    // Both are DirectValid.
    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectValid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::DirectValid);

    // Invalidate block_a.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_a is DirectInvalid, block_b remains DirectValid.
    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectInvalid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::DirectValid);
}

// =======================================================================
// Scenario U: Ancestry-invalid block cannot become frontier
// =======================================================================

/// U1: An ancestry-invalid block cannot be a frontier candidate,
/// even if it was the frontier before the ancestor was invalidated.
#[test]
fn scenario_u_ancestry_invalid_cannot_be_frontier() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → block_a → block_b (frontier)
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.05);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.05);

    // block_b is frontier.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Invalidate block_a (parent of block_b).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_b is AncestryInvalid.
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::AncestryInvalid);

    // Frontier must be cleared — no valid candidates remain.
    assert!(sim.canonical_frontier(&domain_id).is_none());
}

/// U2: When an ancestor is invalidated but a valid sibling branch exists,
/// the frontier falls to the valid sibling.
#[test]
fn scenario_u_frontier_falls_to_valid_sibling() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Branch 1: genesis → block_a → block_b (best metric)
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.05);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.05);

    // Branch 2: genesis → block_c (independent, lower metric)
    let block_c = make_block(12, parent, domain_id, 0.02);
    let block_c_id = block_c.id;
    submit_and_validate_with_delta(&mut sim, block_c, 0.02);

    // block_b is frontier (0.05 > 0.02 > 0.01).
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_b_id));

    // Invalidate block_a — poisons block_b.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_b is AncestryInvalid, block_c is DirectValid.
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::AncestryInvalid);
    assert_eq!(sim.derived_validity(&block_c_id), DerivedValidity::DirectValid);

    // Frontier should fall to block_c.
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_c_id));
}

// =======================================================================
// Scenario V: Ancestry-invalid block cannot dominate
// =======================================================================

/// V1: valid_tip_metrics excludes ancestry-invalid tips.
#[test]
fn scenario_v_ancestry_invalid_excluded_from_dominance() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();
    let parent = genesis_id.as_block_id();

    // Fork: genesis → block_a (0.01), genesis → block_b → block_c (0.05)
    let block_a = make_block(10, parent, domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, parent, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);

    // Both are DirectValid and in the tip metrics.
    let metrics = sim.valid_tip_metrics(&domain_id);
    assert!(metrics.contains_key(&block_a_id));
    assert!(metrics.contains_key(&block_b_id));

    // Now add a child of block_b.
    let block_c = make_block(12, block_b_id, domain_id, 0.05);
    let block_c_id = block_c.id;
    submit_and_validate_with_delta(&mut sim, block_c, 0.05);

    // Invalidate block_b — poisons block_c.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_b_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_c is AncestryInvalid.
    assert_eq!(sim.derived_validity(&block_c_id), DerivedValidity::AncestryInvalid);

    // valid_tip_metrics must not include block_b (DirectInvalid) or
    // block_c (AncestryInvalid). Only block_a should remain.
    let metrics = sim.valid_tip_metrics(&domain_id);
    assert!(!metrics.contains_key(&block_b_id),
        "DirectInvalid block must not be in valid tip metrics");
    assert!(!metrics.contains_key(&block_c_id),
        "AncestryInvalid block must not be in valid tip metrics");
    assert!(metrics.contains_key(&block_a_id),
        "DirectValid block must remain in valid tip metrics");
}

// =======================================================================
// Scenario W: Ancestry-invalid block cannot settle
// =======================================================================

/// W1: Settlement fails cleanly for an ancestry-invalid block.
#[test]
fn scenario_w_ancestry_invalid_cannot_settle() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → block_a → block_b
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);

    // Advance block_b through challenge window.
    sim.close_challenge_window(&block_b_id).unwrap();

    // Advance past release_epoch.
    for _ in 0..6 {
        sim.advance_epoch();
    }

    // Now invalidate block_a (the parent).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_b is AncestryInvalid.
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::AncestryInvalid);

    // block_b is in ChallengeWindowClosed — would normally be settleable.
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::ChallengeWindowClosed));

    // Settlement must fail.
    let result = sim.settle_block(&block_b_id);
    assert!(result.is_err(), "ancestry-invalid block must not settle");
    assert!(result.unwrap_err().contains("AncestryInvalid"),
        "error message must mention AncestryInvalid");

    // Block status must remain ChallengeWindowClosed (not Settled).
    assert_eq!(sim.block_status(&block_b_id), Some(BlockStatus::ChallengeWindowClosed));
}

/// W2: Settlement also fails for a directly invalidated block.
#[test]
fn scenario_w_direct_invalid_cannot_settle() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Invalidate the block directly.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectInvalid);

    // Advance past release_epoch.
    for _ in 0..6 {
        sim.advance_epoch();
    }

    // Settlement must fail.
    let result = sim.settle_block(&block_id);
    assert!(result.is_err(), "directly invalidated block must not settle");
}

// =======================================================================
// Scenario X: Ancestry-invalid block does not get escrow released
// =======================================================================

/// X1: Escrow remains Held when settlement is blocked by ancestry invalidity.
#[test]
fn scenario_x_ancestry_invalid_escrow_not_released() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → block_a → block_b
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);

    // block_b has escrow.
    let escrow_before = sim.block_escrow(&block_b_id).unwrap();
    assert_eq!(escrow_before.status, EscrowStatus::Held);

    // Advance block_b through challenge window.
    sim.close_challenge_window(&block_b_id).unwrap();
    for _ in 0..6 {
        sim.advance_epoch();
    }

    // Invalidate block_a (parent).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // Try to settle block_b (will fail due to AncestryInvalid).
    let _ = sim.settle_block(&block_b_id);

    // Escrow for block_b must still be Held (not Released).
    let escrow_after = sim.block_escrow(&block_b_id).unwrap();
    assert_eq!(escrow_after.status, EscrowStatus::Held,
        "escrow must remain Held for ancestry-invalid block");
}

// =======================================================================
// Scenario Y: Existing upheld-challenge invalidation behavior intact
// =======================================================================

/// Y1: Direct invalidation via upheld challenge still works correctly
/// (regression test for existing Phase 0.3 behavior).
#[test]
fn scenario_y_upheld_challenge_direct_invalidation_still_works() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    // Escrow exists and is Held.
    assert_eq!(sim.block_escrow(&block_id).unwrap().status, EscrowStatus::Held);
    assert_eq!(sim.canonical_frontier(&domain_id), Some(block_id));
    assert!(sim.validated_outcome(&block_id).is_some());
    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectValid);

    // Upheld challenge invalidates.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // All Phase 0.3 consequences remain intact:
    assert_eq!(sim.block_status(&block_id), Some(BlockStatus::Invalidated));
    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectInvalid);
    assert_eq!(sim.block_escrow(&block_id).unwrap().status, EscrowStatus::Slashed);
    assert!(sim.canonical_frontier(&domain_id).is_none());
    assert!(sim.validated_outcome(&block_id).is_none());
    assert_eq!(sim.challenges[&challenge_id].status, ChallengeStatus::Upheld);
}

/// Y2: Rejected challenge preserves derived validity.
#[test]
fn scenario_y_rejected_challenge_preserves_derived_validity() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    let block = make_block(10, genesis_id.as_block_id(), domain_id, 0.015);
    let block_id = block.id;
    submit_and_validate(&mut sim, block);

    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectValid);

    // Open, review, and reject challenge.
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.reject_challenge(&challenge_id).unwrap();

    // Derived validity unchanged.
    assert_eq!(sim.derived_validity(&block_id), DerivedValidity::DirectValid);
}

// =======================================================================
// Scenario Z: Deep ancestry chain and multi-level invalidation
// =======================================================================

/// Z1: Grandchild is AncestryInvalid when grandparent is invalidated.
#[test]
fn scenario_z_deep_ancestry_invalidation() {
    let (mut sim, domain_id, genesis_id) = setup_active_domain();

    // Chain: genesis → a → b → c
    let block_a = make_block(10, genesis_id.as_block_id(), domain_id, 0.01);
    let block_a_id = block_a.id;
    submit_and_validate_with_delta(&mut sim, block_a, 0.01);

    let block_b = make_block(11, block_a_id, domain_id, 0.02);
    let block_b_id = block_b.id;
    submit_and_validate_with_delta(&mut sim, block_b, 0.02);

    let block_c = make_block(12, block_b_id, domain_id, 0.03);
    let block_c_id = block_c.id;
    submit_and_validate_with_delta(&mut sim, block_c, 0.03);

    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectValid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::DirectValid);
    assert_eq!(sim.derived_validity(&block_c_id), DerivedValidity::DirectValid);

    // Invalidate block_a (the root of the chain).
    let challenge_id = test_challenge_id(1);
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_a_id },
        test_participant_id(5),
        TokenAmount::new(200),
        test_artifact_hash(80),
    )
    .unwrap();
    sim.begin_challenge_review(&challenge_id).unwrap();
    sim.uphold_challenge(&challenge_id).unwrap();

    // block_a is DirectInvalid, block_b and block_c are AncestryInvalid.
    assert_eq!(sim.derived_validity(&block_a_id), DerivedValidity::DirectInvalid);
    assert_eq!(sim.derived_validity(&block_b_id), DerivedValidity::AncestryInvalid);
    assert_eq!(sim.derived_validity(&block_c_id), DerivedValidity::AncestryInvalid);

    // Neither can settle.
    sim.close_challenge_window(&block_b_id).unwrap();
    sim.close_challenge_window(&block_c_id).unwrap();
    for _ in 0..6 {
        sim.advance_epoch();
    }
    assert!(sim.settle_block(&block_b_id).is_err());
    assert!(sim.settle_block(&block_c_id).is_err());

    // Escrows remain Held.
    assert_eq!(sim.block_escrow(&block_b_id).unwrap().status, EscrowStatus::Held);
    assert_eq!(sim.block_escrow(&block_c_id).unwrap().status, EscrowStatus::Held);

    // Frontier must be cleared.
    assert!(sim.canonical_frontier(&domain_id).is_none());
}
