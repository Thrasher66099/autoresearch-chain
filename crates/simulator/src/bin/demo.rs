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

//! Interactive demo of the AutoResearch Chain protocol lifecycle.
//!
//! Demonstrates the complete Phase 0.3d protocol: genesis activation,
//! block validation, epoch advancement, escrow, challenges, invalidation,
//! derived validity, and settlement.

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

/// Submit a block, assign validators, record passing attestations, and evaluate.
fn submit_and_validate(sim: &mut SimulatorState, block: Block) {
    let block_id = block.id;
    let delta = block.claimed_metric_delta.as_f64();
    sim.submit_block(block).unwrap();
    let assigned = sim.assign_validators(&block_id).unwrap();
    // Validators observe slightly below claimed (realistic).
    let observed = delta * 0.9;
    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(observed)),
            replay_evidence_ref: test_artifact_hash(70),
            timestamp: 1700099000,
        })
        .unwrap();
    }
    sim.evaluate_block(&block_id).unwrap();
}

fn main() {
    println!("=== AutoResearch Chain — Protocol Lifecycle Demo ===\n");

    // ------------------------------------------------------------------
    // Phase 1: Genesis activation
    // ------------------------------------------------------------------
    println!("--- Phase 1: Genesis Activation ---\n");

    let mut sim = SimulatorState::new();

    let genesis = valid_genesis_block();
    let genesis_id = genesis.id;
    let domain_id = genesis.domain_id;

    println!("Submitting genesis block: {}", genesis_id);
    sim.submit_genesis(genesis).unwrap();

    println!("Evaluating RTS conformance...");
    sim.evaluate_conformance(&genesis_id).unwrap();
    println!("  RTS-1 conformance: PASS");

    println!("Recording 3 seed validations...");
    for i in 1..=3u8 {
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
        println!("  Validator {} attested seed score 0.93", i);
    }

    let activated = sim.finalize_activation(&genesis_id).unwrap();
    println!("Domain activated: {} ({})", activated.domain.name, domain_id);
    println!("  Metric: {} (higher is better)", activated.domain_spec.primary_metric);

    // Register validator pool.
    sim.register_validator_pool(ValidatorPool {
        domain_id,
        validators: (1..=10).map(test_validator_id).collect(),
    });
    println!("  Validator pool registered (10 validators)\n");

    // ------------------------------------------------------------------
    // Phase 2: Submit and validate Block A
    // ------------------------------------------------------------------
    println!("--- Phase 2: Block A — Validated Improvement ---\n");

    let block_a = Block {
        id: test_block_id(10),
        domain_id,
        parent_id: genesis_id.as_block_id(),
        proposer: test_proposer_id(1),
        child_state_ref: test_artifact_hash(60),
        diff_ref: test_artifact_hash(61),
        claimed_metric_delta: MetricValue::new(0.020),
        evidence_bundle_hash: test_artifact_hash(62),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: sim.current_epoch,
        status: BlockStatus::Submitted,
        timestamp: 1700010000,
    };
    let block_a_id = block_a.id;

    println!("Submitting Block A (claimed delta: +0.020)...");
    sim.submit_block(block_a).unwrap();

    let assigned = sim.assign_validators(&block_a_id).unwrap();
    println!("  {} validators assigned", assigned.len());

    // Validators observe delta of 0.018 (slightly below claimed — realistic).
    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id: block_a_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(0.018)),
            replay_evidence_ref: test_artifact_hash(70),
            timestamp: 1700011000,
        })
        .unwrap();
    }

    let outcome = sim.evaluate_block(&block_a_id).unwrap();
    println!("  Evaluation outcome: {:?}", outcome);

    // Show validated outcome (protocol truth).
    if let Some(vo) = sim.validated_outcome(&block_a_id) {
        println!(
            "  Validated metric delta: {:.4} (from {} attestations)",
            vo.validated_metric_delta.as_f64(),
            vo.attestation_count
        );
        println!(
            "  (Proposer claimed +0.020, validators observed +{:.4})",
            vo.validated_metric_delta.as_f64()
        );
    }

    // Show escrow.
    if let Some(escrow) = sim.block_escrow(&block_a_id) {
        println!(
            "  Escrow: {:?}, amount={}, release_epoch={}",
            escrow.status,
            escrow.amount.as_u64(),
            escrow.release_epoch.0
        );
    }

    // Show frontier.
    if let Some(frontier) = sim.canonical_frontier(&domain_id) {
        println!("  Canonical frontier: {}", frontier);
    }

    println!(
        "  Derived validity: {:?}",
        sim.derived_validity(&block_a_id)
    );
    println!();

    // ------------------------------------------------------------------
    // Phase 3: Submit Block B (building on Block A)
    // ------------------------------------------------------------------
    println!("--- Phase 3: Block B — Builds on Block A ---\n");

    let block_b = Block {
        id: test_block_id(20),
        domain_id,
        parent_id: block_a_id,
        proposer: test_proposer_id(2),
        child_state_ref: test_artifact_hash(80),
        diff_ref: test_artifact_hash(81),
        claimed_metric_delta: MetricValue::new(0.010),
        evidence_bundle_hash: test_artifact_hash(82),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: sim.current_epoch,
        status: BlockStatus::Submitted,
        timestamp: 1700020000,
    };
    let block_b_id = block_b.id;

    println!("Submitting Block B (child of A, claimed delta: +0.010)...");
    sim.submit_block(block_b).unwrap();

    let assigned = sim.assign_validators(&block_b_id).unwrap();
    for v in &assigned {
        sim.record_attestation(ValidationAttestation {
            block_id: block_b_id,
            validator: *v,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(0.009)),
            replay_evidence_ref: test_artifact_hash(71),
            timestamp: 1700021000,
        })
        .unwrap();
    }

    let outcome = sim.evaluate_block(&block_b_id).unwrap();
    println!("  Evaluation outcome: {:?}", outcome);
    if let Some(vo) = sim.validated_outcome(&block_b_id) {
        println!(
            "  Validated metric delta: {:.4}",
            vo.validated_metric_delta.as_f64()
        );
    }
    if let Some(frontier) = sim.canonical_frontier(&domain_id) {
        println!("  Canonical frontier updated to: {}", frontier);
    }
    println!();

    // ------------------------------------------------------------------
    // Phase 4: Epoch advancement and settlement of Block A
    // ------------------------------------------------------------------
    println!("--- Phase 4: Epoch Advancement & Settlement ---\n");

    println!("Current epoch: {}", sim.current_epoch);

    // Close challenge window for Block A.
    sim.close_challenge_window(&block_a_id).unwrap();
    println!("Closed challenge window for Block A");

    // Try settling too early.
    println!("Attempting early settlement of Block A...");
    match sim.settle_block(&block_a_id) {
        Ok(()) => println!("  Settled (unexpected!)"),
        Err(e) => println!("  Correctly rejected: {}", e),
    }

    // Advance epochs to pass the challenge window (default: 5 epochs).
    println!("\nAdvancing epochs...");
    for _ in 0..6 {
        sim.advance_epoch();
    }
    println!("Current epoch: {}", sim.current_epoch);

    // Now settle Block A.
    println!("Settling Block A...");
    sim.settle_block(&block_a_id).unwrap();
    println!("  Block A status: {:?}", sim.block_status(&block_a_id).unwrap());

    // Show escrow after settlement.
    if let Some(escrow) = sim.block_escrow(&block_a_id) {
        println!(
            "  Escrow after settlement: {:?}, amount={}",
            escrow.status,
            escrow.amount.as_u64()
        );
    }

    // Finalize Block A.
    sim.finalize_block(&block_a_id).unwrap();
    println!(
        "  Block A finalized: {:?}",
        sim.block_status(&block_a_id).unwrap()
    );
    println!();

    // ------------------------------------------------------------------
    // Phase 5: Challenge and invalidation of Block B
    // ------------------------------------------------------------------
    println!("--- Phase 5: Challenge and Invalidation ---\n");

    let challenge_id = test_challenge_id(1);
    println!("Opening challenge against Block B...");
    sim.open_challenge(
        challenge_id,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block {
            block_id: block_b_id,
        },
        test_participant_id(5),
        TokenAmount::new(250),
        test_artifact_hash(90),
    )
    .unwrap();
    println!("  Challenge opened: {}", challenge_id);

    sim.begin_challenge_review(&challenge_id).unwrap();
    println!("  Challenge under review");

    // Show Block B derived validity before upheld.
    println!(
        "  Block B derived validity (before): {:?}",
        sim.derived_validity(&block_b_id)
    );

    // Uphold the challenge — invalidates Block B, slashes its escrow.
    sim.uphold_challenge(&challenge_id).unwrap();
    println!("  Challenge UPHELD — Block B invalidated");

    println!(
        "  Block B status: {:?}",
        sim.block_status(&block_b_id).unwrap()
    );
    println!(
        "  Block B derived validity: {:?}",
        sim.derived_validity(&block_b_id)
    );

    if let Some(escrow) = sim.block_escrow(&block_b_id) {
        println!(
            "  Block B escrow: {:?} (slashed)",
            escrow.status
        );
    }

    // Show frontier after invalidation.
    if let Some(frontier) = sim.canonical_frontier(&domain_id) {
        println!("  Canonical frontier after invalidation: {}", frontier);
    } else {
        println!("  Canonical frontier: none (reverted to Block A)");
    }
    println!();

    // ------------------------------------------------------------------
    // Phase 6: Demonstrating DerivedValidity with ancestry chains
    // ------------------------------------------------------------------
    println!("--- Phase 6: Derived Validity & Ancestry Chains ---\n");

    // Attempting to submit a new block on an invalidated parent is rejected.
    let block_c = Block {
        id: test_block_id(30),
        domain_id,
        parent_id: block_b_id,
        proposer: test_proposer_id(3),
        child_state_ref: test_artifact_hash(100),
        diff_ref: test_artifact_hash(101),
        claimed_metric_delta: MetricValue::new(0.005),
        evidence_bundle_hash: test_artifact_hash(102),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: sim.current_epoch,
        status: BlockStatus::Submitted,
        timestamp: 1700030000,
    };

    println!("Attempting to submit Block C on invalidated parent B...");
    match sim.submit_block(block_c) {
        Ok(_) => println!("  Submitted (unexpected)"),
        Err(e) => println!("  Correctly rejected: {}", e),
    }

    // Demonstrate ancestry invalidation: submit Block D on A (valid),
    // then submit Block E on D. Challenge D → E becomes AncestryInvalid.
    println!("\nSubmitting Block D (child of valid, finalized Block A)...");
    let block_d = Block {
        id: test_block_id(40),
        domain_id,
        parent_id: block_a_id,
        proposer: test_proposer_id(4),
        child_state_ref: test_artifact_hash(110),
        diff_ref: test_artifact_hash(111),
        claimed_metric_delta: MetricValue::new(0.012),
        evidence_bundle_hash: test_artifact_hash(112),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: sim.current_epoch,
        status: BlockStatus::Submitted,
        timestamp: 1700040000,
    };
    let block_d_id = block_d.id;
    submit_and_validate(&mut sim, block_d);
    println!("  Block D accepted, derived validity: {:?}", sim.derived_validity(&block_d_id));

    println!("Submitting Block E (child of Block D)...");
    let block_e = Block {
        id: test_block_id(50),
        domain_id,
        parent_id: block_d_id,
        proposer: test_proposer_id(5),
        child_state_ref: test_artifact_hash(120),
        diff_ref: test_artifact_hash(121),
        claimed_metric_delta: MetricValue::new(0.008),
        evidence_bundle_hash: test_artifact_hash(122),
        fee: TokenAmount::new(10),
        bond: TokenAmount::new(500),
        epoch_id: sim.current_epoch,
        status: BlockStatus::Submitted,
        timestamp: 1700050000,
    };
    let block_e_id = block_e.id;
    submit_and_validate(&mut sim, block_e);
    println!("  Block E accepted, derived validity: {:?}", sim.derived_validity(&block_e_id));

    // Now invalidate Block D — Block E becomes AncestryInvalid.
    println!("\nChallenging Block D...");
    let challenge_d = test_challenge_id(2);
    sim.open_challenge(
        challenge_d,
        ChallengeType::BlockReplay,
        ChallengeTarget::Block { block_id: block_d_id },
        test_participant_id(6),
        TokenAmount::new(250),
        test_artifact_hash(95),
    ).unwrap();
    sim.begin_challenge_review(&challenge_d).unwrap();
    sim.uphold_challenge(&challenge_d).unwrap();
    println!("  Challenge upheld — Block D invalidated");

    println!("  Block D derived validity: {:?}", sim.derived_validity(&block_d_id));
    println!("  Block E derived validity: {:?}", sim.derived_validity(&block_e_id));
    println!("  Block E is_on_valid_chain: {}", sim.is_on_valid_chain(&block_e_id));

    // Advance epochs and try to settle Block E.
    sim.close_challenge_window(&block_e_id).unwrap();
    for _ in 0..6 {
        sim.advance_epoch();
    }

    println!("\nAttempting to settle Block E (ancestry-poisoned)...");
    match sim.settle_block(&block_e_id) {
        Ok(()) => println!("  Settled (should not happen!)"),
        Err(e) => println!("  Correctly rejected: {}", e),
    }
    println!();

    // ------------------------------------------------------------------
    // Summary
    // ------------------------------------------------------------------
    println!("--- Summary ---\n");
    println!("Block A ({}):", block_a_id);
    println!("  Status: {:?}", sim.block_status(&block_a_id).unwrap());
    println!("  Derived validity: {:?}", sim.derived_validity(&block_a_id));

    println!("Block B ({}):", block_b_id);
    println!("  Status: {:?}", sim.block_status(&block_b_id).unwrap());
    println!("  Derived validity: {:?}", sim.derived_validity(&block_b_id));

    println!("Block D ({}):", block_d_id);
    println!("  Status: {:?}", sim.block_status(&block_d_id).unwrap());
    println!("  Derived validity: {:?}", sim.derived_validity(&block_d_id));

    println!("Block E ({}):", block_e_id);
    println!("  Status: {:?}", sim.block_status(&block_e_id).unwrap());
    println!("  Derived validity: {:?}", sim.derived_validity(&block_e_id));

    if let Some(frontier) = sim.canonical_frontier(&domain_id) {
        println!("\nCanonical frontier: {}", frontier);
    }

    println!("\n=== Demo complete ===");
}
