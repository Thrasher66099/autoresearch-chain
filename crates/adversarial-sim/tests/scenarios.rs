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

//! Adversarial scenarios keyed to docs/attack-model.md.
//!
//! Each test asserts an economic property of the mechanism under the
//! calibrated parameters (`recommended_world`), or documents a known
//! failure mode of weaker parameters. See
//! simulations/calibration-report.md for the full sweep.

use arc_adversarial_sim::*;

fn standard_actors(coverage: f64) -> Actors {
    Actors {
        proposers: vec![
            ProposerStrategy::Honest,
            ProposerStrategy::Honest,
            ProposerStrategy::Fraudster,
        ],
        validators: vec![
            ValidatorStrategy::HonestReplay,
            ValidatorStrategy::HonestReplay,
            ValidatorStrategy::HonestReplay,
            ValidatorStrategy::RubberStamp,
            ValidatorStrategy::RubberStamp,
            ValidatorStrategy::RubberStamp,
        ],
        challengers: vec![ChallengerStrategy::Auditor { coverage }],
    }
}

#[test]
fn honest_proposing_is_profitable() {
    let r = run_episode(&recommended_world(), &standard_actors(0.3));
    assert!(
        r.proposer_ev["Honest"] > 0.0,
        "honest EV must be positive, got {}",
        r.proposer_ev["Honest"]
    );
}

#[test]
fn fraud_is_positive_ev_under_naive_test_parameters() {
    // Documents WHY calibration matters: with the readable-test defaults
    // (bond 500 vs reward 1000) fraud is profitable unless audit coverage
    // approaches 80%. This is a pinned failure mode, not a target.
    let r = run_episode(&WorldConfig::default(), &standard_actors(0.3));
    assert!(
        r.proposer_ev["Fraudster"] > 0.0,
        "expected fraud to be profitable at naive params (the finding), got {}",
        r.proposer_ev["Fraudster"]
    );
}

#[test]
fn fraud_is_negative_ev_under_recommended_parameters() {
    // bond 2000 (2x reward), provisional 10%: analytic break-even audit
    // coverage is ~0.35 (vs ~0.77 at naive params); 0.5 is robustly -EV.
    // Note: fraud that is REJECTED at validation costs the fraudster
    // nothing, because bonds are only escrowed at acceptance — a flagged
    // protocol gap (bond should be committed at submission).
    let r = run_episode(&recommended_world(), &standard_actors(0.5));
    assert!(
        r.proposer_ev["Fraudster"] < 0.0,
        "fraud must be -EV under recommended params, got {}",
        r.proposer_ev["Fraudster"]
    );
    // And honesty strictly dominates fraud.
    assert!(r.proposer_ev["Honest"] > r.proposer_ev["Fraudster"]);
}

#[test]
fn rubber_stamp_pool_lets_fraud_through_without_challengers() {
    // All-lazy validator pool, no challengers: fraud earns full rewards.
    // Rubber-stamping includes collusion — behaviorally identical.
    let mut a = standard_actors(0.0);
    a.validators = vec![ValidatorStrategy::RubberStamp; 6];
    a.challengers = vec![];
    let r = run_episode(&recommended_world(), &a);
    assert!(
        r.proposer_ev["Fraudster"] > 0.0,
        "without challengers, lazy validation must let fraud profit (the finding)"
    );
    assert_eq!(r.blocks_invalidated, 0);
}

#[test]
fn challenger_backstop_defeats_fraud_even_with_fully_lazy_validators() {
    // Same lazy pool, but one auditor at 50% coverage: fraud collapses.
    // The challenge game, not validator diligence, is the binding
    // security layer.
    let mut a = standard_actors(0.5);
    a.validators = vec![ValidatorStrategy::RubberStamp; 6];
    let r = run_episode(&recommended_world(), &a);
    assert!(
        r.proposer_ev["Fraudster"] < 0.0,
        "auditor must make fraud -EV despite lazy validators, got {}",
        r.proposer_ev["Fraudster"]
    );
    assert!(r.challenges_upheld > 0);
    assert!(
        r.challenger_ev["Auditor"] > 0.0,
        "successful auditing must be profitable"
    );
}

#[test]
fn honest_validator_minority_rejects_gross_fraud() {
    // With honest validators in the pool, a single honest assignment
    // votes FraudSuspected on gross mismatch, which rejects the block
    // outright (fraud_triggers_rejection). Collusion needs the full
    // assignment to be colluding.
    let r = run_episode(&recommended_world(), &standard_actors(0.0));
    // Fraud blocks only survive when all 3 assigned validators are
    // rubber-stampers (3 of 6 in the pool) — well under half of rounds.
    assert!(r.blocks_accepted < r.blocks_submitted);
}

#[test]
fn noise_mining_profits_without_min_delta_and_loses_with_it() {
    let mut actors = standard_actors(0.3);
    actors.proposers = vec![ProposerStrategy::Honest, ProposerStrategy::NoiseMiner];

    // Without a minimum-improvement threshold, sub-tolerance claims are
    // unfalsifiable and farm full block rewards (attack-model: noise
    // mining). Pinned failure mode.
    let mut w = recommended_world();
    w.min_accepted_delta = 0.0;
    let r = run_episode(&w, &actors);
    assert!(
        r.proposer_ev["NoiseMiner"] > 0.0,
        "noise mining must be profitable without min_accepted_delta (the finding)"
    );

    // With the threshold above the tolerance band, noise claims are
    // rejected at evaluation and the miner only burns attempt costs.
    let w = recommended_world();
    assert!(w.min_accepted_delta > w.tolerance);
    let r = run_episode(&w, &actors);
    assert!(
        r.proposer_ev["NoiseMiner"] < 0.0,
        "noise mining must be -EV with min_accepted_delta, got {}",
        r.proposer_ev["NoiseMiner"]
    );
    // The threshold must not de-incentivize honest work.
    assert!(r.proposer_ev["Honest"] > 0.0);
}

#[test]
fn griefing_valid_blocks_always_loses() {
    // All-honest proposer population: every random challenge targets a
    // valid block, is rejected, and forfeits the challenger bond.
    let mut a = standard_actors(0.0);
    a.proposers = vec![ProposerStrategy::Honest, ProposerStrategy::Honest];
    a.challengers = vec![ChallengerStrategy::Griefer { rate: 0.3 }];
    let r = run_episode(&recommended_world(), &a);
    assert!(
        r.challenger_ev["Griefer"] < 0.0,
        "griefing must be -EV, got {}",
        r.challenger_ev["Griefer"]
    );
    assert_eq!(r.challenges_upheld, 0);
}

#[test]
fn validator_compensation_gap_is_real() {
    // The protocol pays validators nothing: honest replay is strictly
    // -EV while rubber-stamping is free. A per-attestation fee equal to
    // the replay cost makes honest validation break even but pays lazy
    // validators the same fee for no work — fees alone cannot
    // differentiate; attestation-level slashing (specified, not
    // implemented) is required.
    let w = recommended_world();
    let r = run_episode(&w, &standard_actors(0.3));
    assert!(
        r.validator_ev["HonestReplay"] < 0.0,
        "unpaid honest validation must be -EV (the compensation gap)"
    );
    assert!(r.validator_ev["RubberStamp"] >= 0.0);

    let mut w_paid = recommended_world();
    w_paid.validator_fee = w_paid.replay_cost;
    let r = run_episode(&w_paid, &standard_actors(0.3));
    // Real proposer-fee shares (economics step 3) now supplement the
    // hypothetical fee: at fee == replay_cost, honest validation is at
    // or above break-even.
    assert!(r.validator_ev["HonestReplay"] >= 0.0);
    assert!(
        r.validator_ev["RubberStamp"] > r.validator_ev["HonestReplay"],
        "fees alone reward laziness at least as much as honesty"
    );
}

#[test]
fn episodes_are_deterministic() {
    let w = recommended_world();
    let a = standard_actors(0.3);
    let r1 = run_episode(&w, &a);
    let r2 = run_episode(&w, &a);
    assert_eq!(r1.blocks_submitted, r2.blocks_submitted);
    assert_eq!(r1.blocks_accepted, r2.blocks_accepted);
    assert_eq!(r1.challenges_upheld, r2.challenges_upheld);
    assert_eq!(r1.proposer_ev, r2.proposer_ev);
    assert_eq!(r1.net, r2.net);
}

#[test]
fn degenerate_surface_waste_is_bounded_by_deprecation_mechanism() {
    // attack-model.md §18: without a deprecation mechanism, honest waste
    // on an exploited domain grows with the horizon; with the
    // evaluation-surface challenge it is bounded by exploit discovery
    // time (expected 1/discovery_prob rounds).
    let (without_short, with_short) =
        degenerate_surface_waste(1_000, 5, 300.0, 0.05, 7);
    let (without_long, with_long) =
        degenerate_surface_waste(10_000, 5, 300.0, 0.05, 7);

    // Unbounded: waste scales with the horizon.
    assert!(without_long > 9.0 * without_short);
    // Bounded: waste is independent of the horizon once discovered.
    assert_eq!(with_short, with_long);
    assert!(with_long < 0.1 * without_long);
}

#[test]
fn wash_mining_extraction_is_bounded_by_subsidy_caps() {
    // Economics step 5 (wash-mining): an attacker who funds a domain,
    // mines it, AND validates it recycles pool spend back to themselves —
    // the matching ratio alone does not make self-dealing unprofitable.
    // The binding defenses are the per-epoch and lifetime subsidy caps
    // (and, on a real network, open validator assignment taking fees).
    // This test pins that extraction never exceeds the caps.
    use arc_domain_engine::genesis::SeedValidationRecord;
    use arc_protocol_types::*;
    use arc_protocol_rules::validator::ValidatorPool;
    use arc_simulator::state::SimulatorState;

    let mut sim = SimulatorState::new();
    sim.reward_config.subsidy_total_cap = 800;
    sim.reward_config.subsidy_epoch_cap = 300;
    sim.reward_config.subsidy_rate_bps = 5_000;

    let mut genesis = valid_genesis_block();
    genesis.reward_pool = TokenAmount::new(10_000);
    genesis.validation_reserve_bps = 0;
    genesis.base_block_reward = TokenAmount::new(1_000);
    let genesis_id = genesis.id;
    let domain_id = genesis.domain_id;
    sim.submit_genesis(genesis).unwrap();
    sim.evaluate_conformance(&genesis_id).unwrap();
    for i in 1..=3 {
        sim.record_seed_validation(&genesis_id, SeedValidationRecord {
            validator: test_validator_id(i),
            vote: ValidatorVote::Pass,
            observed_score: Some(MetricValue::new(0.9300)),
            timestamp: 1_700_000_000 + i as u64,
        }).unwrap();
    }
    sim.finalize_activation(&genesis_id).unwrap();
    sim.register_validator_pool(ValidatorPool {
        domain_id,
        validators: (1..=3).map(test_validator_id).collect(), // all attacker-run
    });

    // Self-mine 5 blocks through settlement.
    for n in 1..=5u8 {
        let mut block = Block {
            id: test_block_id(100 + n),
            domain_id,
            parent_id: genesis_id.as_block_id(),
            proposer: test_proposer_id(1),
            child_state_ref: test_artifact_hash(60 + n),
            diff_ref: test_artifact_hash(160 + n),
            claimed_metric_delta: MetricValue::new(0.015),
            evidence_bundle_hash: test_artifact_hash(200 + n),
            fee: TokenAmount::new(10),
            bond: TokenAmount::new(500),
            epoch_id: EpochId(1),
            status: BlockStatus::Submitted,
            timestamp: 1_700_001_000 + n as u64,
        };
        block.claimed_metric_delta = MetricValue::new(0.015);
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
                timestamp: 1_700_002_000,
            }).unwrap();
        }
        sim.evaluate_block(&block_id).unwrap();
        sim.close_challenge_window(&block_id).unwrap();
        for _ in 0..5 {
            sim.advance_epoch();
        }
        sim.settle_block(&block_id).unwrap();
    }

    // 5 settled blocks x 500 match = 2500 wanted, but the lifetime cap
    // (800) binds; per-epoch cap (300) binds each settlement epoch.
    assert_eq!(sim.subsidy_minted_total, 800);
    assert!(sim.subsidy_payouts.iter().all(|p| p.amount.as_u64() <= 300));
    // The attacker spent 5000 of their own pool to extract 800 minted.
    assert_eq!(sim.domain_pool(&domain_id).unwrap().spent, TokenAmount::new(5_000));
}
