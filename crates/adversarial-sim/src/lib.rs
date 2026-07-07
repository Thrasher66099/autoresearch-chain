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

//! Adversarial simulation harness for AutoResearch Chain (Phase 4).
//!
//! Drives the real protocol state machine (`SimulatorState`) with
//! strategy-parameterized actors and measures per-strategy expected value
//! per episode. The purpose is the project's central scientific test:
//! **does honest play dominate under the implemented economics?**
//!
//! # Model assumptions (stated, not hidden)
//!
//! - Every block has a ground-truth delta known to the harness. Honest
//!   replay observes it plus Gaussian measurement noise; challenge
//!   adjudication resolves by ground truth (the challenge game is modeled
//!   as correct — its own failure modes are a separate concern).
//! - Compute costs (running an experiment, replaying one) are exogenous
//!   parameters in token units.
//! - Validators are currently *unpaid by the protocol* — a known gap this
//!   harness quantifies via the hypothetical `validator_fee` parameter.
//! - Randomness is a deterministic xorshift generator; every episode is
//!   reproducible from its seed.

use std::collections::HashMap;

use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::attestation::ProvisionalOutcome;
use arc_protocol_rules::validator::ValidatorPool;
use arc_protocol_types::*;
use arc_simulator::state::SimulatorState;

// -----------------------------------------------------------------------
// Deterministic RNG (xorshift64*)
// -----------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Approximately standard normal (Irwin–Hall, 12 uniforms).
    pub fn next_gauss(&mut self) -> f64 {
        (0..12).map(|_| self.next_f64()).sum::<f64>() - 6.0
    }
}

// -----------------------------------------------------------------------
// Strategies and world parameters
// -----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProposerStrategy {
    /// Pays `work_cost`, produces a genuine delta ~ N(mean, sd), submits
    /// when the measured result is positive.
    Honest,
    /// Pays nothing, does no work (true delta 0), claims `fraud_claim`.
    Fraudster,
    /// Pays `attempt_cost` per try; no real improvement (true delta 0);
    /// submits its own noisy measurement whenever it comes out positive.
    NoiseMiner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValidatorStrategy {
    /// Pays `replay_cost`, observes truth + noise, votes on the evidence:
    /// Pass within tolerance, FraudSuspected on gross mismatch, else Fail.
    HonestReplay,
    /// Pays nothing, rubber-stamps the proposer's claim (models both lazy
    /// validators and colluders — behaviorally identical).
    RubberStamp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChallengerStrategy {
    /// Audits each accepted block with probability `coverage`: pays
    /// `replay_cost`, challenges when the replay contradicts the claim.
    Auditor { coverage: f64 },
    /// Challenges valid blocks at random with probability `rate`
    /// (bond-griefing; should always be -EV).
    Griefer { rate: f64 },
}

/// World and economic parameters for an episode.
#[derive(Clone, Debug)]
pub struct WorldConfig {
    pub rounds: usize,
    pub seed: u64,
    /// Compute cost of a real experiment (token units).
    pub work_cost: f64,
    /// Compute cost of one replay (validation or audit).
    pub replay_cost: f64,
    /// Compute cost of one noise-mining attempt.
    pub attempt_cost: f64,
    /// Hypothetical per-attestation fee paid to validators. The protocol
    /// does not implement validator compensation yet; this parameter
    /// quantifies the gap.
    pub validator_fee: f64,
    /// Mean and sd of a genuine improvement's delta.
    pub true_delta_mean: f64,
    pub true_delta_sd: f64,
    /// Measurement noise sd for any single replay.
    pub noise_sd: f64,
    /// Attestation tolerance band: |observed - claimed| within this passes.
    pub tolerance: f64,
    /// Gross-mismatch threshold for a FraudSuspected vote.
    pub fraud_threshold: f64,
    /// Delta a fraudster claims.
    pub fraud_claim: f64,
    /// Proposer bond per block.
    pub block_bond: u64,
    /// Challenger bond per challenge.
    pub challenge_bond: u64,
    /// Minimum validated improvement required for acceptance
    /// (`ValidationConfig::min_accepted_delta`). Must exceed the
    /// tolerance band or noise mining is risk-free.
    pub min_accepted_delta: f64,
    /// Reward-engine economics under test.
    pub reward: arc_reward_engine::RewardConfig,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            rounds: 200,
            seed: 42,
            work_cost: 300.0,
            replay_cost: 30.0,
            attempt_cost: 10.0,
            validator_fee: 0.0,
            true_delta_mean: 0.02,
            true_delta_sd: 0.005,
            noise_sd: 0.001,
            tolerance: 0.004,
            fraud_threshold: 0.04,
            fraud_claim: 0.1,
            block_bond: 500,
            challenge_bond: 200,
            min_accepted_delta: 0.0,
            reward: arc_reward_engine::RewardConfig::default(),
        }
    }
}

/// Parameters recommended by the calibration sweep (see
/// `simulations/calibration-report.md`). Code defaults elsewhere are
/// tuned for readable tests, not adversarial robustness.
pub fn recommended_world() -> WorldConfig {
    let mut w = WorldConfig::default();
    // Fraud exposure: slashable stake must dwarf the unclawable
    // provisional payout so fraud is -EV at realistic audit coverage.
    w.block_bond = 2_000;
    w.reward.provisional_reward_bps = 1_000; // 10% provisional
    // Noise-mining defense: sub-tolerance claims are unfalsifiable, so
    // the acceptance threshold must sit above the tolerance band.
    w.min_accepted_delta = 2.0 * w.tolerance;
    w
}

// -----------------------------------------------------------------------
// Actors
// -----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Actors {
    pub proposers: Vec<ProposerStrategy>,
    pub validators: Vec<ValidatorStrategy>,
    pub challengers: Vec<ChallengerStrategy>,
}

fn proposer_pid(i: usize) -> ParticipantId {
    let mut b = [0u8; 32];
    b[0] = 0xA0;
    b[1] = i as u8;
    ParticipantId::from_bytes(b)
}

fn challenger_pid(i: usize) -> ParticipantId {
    let mut b = [0u8; 32];
    b[0] = 0xC0;
    b[1] = i as u8;
    ParticipantId::from_bytes(b)
}

fn validator_vid(i: usize) -> ValidatorId {
    // Offset to avoid colliding with the seed validators used in setup.
    test_validator_id(50 + i as u8)
}

// -----------------------------------------------------------------------
// Episode outcome
// -----------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct EpisodeReport {
    /// Net token EV per participant (protocol flows minus compute costs).
    pub net: HashMap<ParticipantId, f64>,
    /// Mean net EV per proposer strategy.
    pub proposer_ev: HashMap<String, f64>,
    /// Mean net EV per validator strategy.
    pub validator_ev: HashMap<String, f64>,
    /// Mean net EV per challenger strategy.
    pub challenger_ev: HashMap<String, f64>,
    pub blocks_submitted: usize,
    pub blocks_accepted: usize,
    pub blocks_invalidated: usize,
    pub challenges_opened: usize,
    pub challenges_upheld: usize,
}

// -----------------------------------------------------------------------
// Episode runner
// -----------------------------------------------------------------------

pub fn run_episode(world: &WorldConfig, actors: &Actors) -> EpisodeReport {
    let mut rng = Rng::new(world.seed);
    let mut sim = SimulatorState::new();
    sim.reward_config = world.reward.clone();
    sim.validation_config.min_accepted_delta = world.min_accepted_delta;

    // --- Domain setup (CIFAR-10 fixture genesis) ---
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
                timestamp: 1_700_000_000 + i as u64,
            },
        )
        .unwrap();
    }
    sim.finalize_activation(&genesis_id).unwrap();
    sim.register_validator_pool(ValidatorPool {
        domain_id,
        validators: (0..actors.validators.len()).map(validator_vid).collect(),
    });

    let validator_by_id: HashMap<ValidatorId, ValidatorStrategy> = actors
        .validators
        .iter()
        .enumerate()
        .map(|(i, s)| (validator_vid(i), *s))
        .collect();

    // Compute costs and hypothetical fees, tracked outside protocol flows.
    let mut costs: HashMap<ParticipantId, f64> = HashMap::new();
    let mut report = EpisodeReport::default();
    // Ground truth per block: (true_delta, claimed_delta).
    let mut truth: HashMap<BlockId, (f64, f64)> = HashMap::new();
    let mut block_counter: u64 = 0;
    let mut challenge_counter: u64 = 0;

    for _round in 0..world.rounds {
        for (pi, strategy) in actors.proposers.iter().enumerate() {
            let pid = proposer_pid(pi);

            // Strategy decides whether to submit and what to claim.
            let (true_delta, claimed) = match strategy {
                ProposerStrategy::Honest => {
                    *costs.entry(pid).or_default() += world.work_cost;
                    let t = world.true_delta_mean
                        + world.true_delta_sd * rng.next_gauss();
                    let measured = t + world.noise_sd * rng.next_gauss();
                    if measured <= 0.0 {
                        continue; // no improvement found; nothing to submit
                    }
                    (t, measured)
                }
                ProposerStrategy::Fraudster => (0.0, world.fraud_claim),
                ProposerStrategy::NoiseMiner => {
                    *costs.entry(pid).or_default() += world.attempt_cost;
                    let measured = world.noise_sd * rng.next_gauss();
                    if measured <= 0.0 {
                        continue;
                    }
                    (0.0, measured)
                }
            };

            // Submit the block.
            block_counter += 1;
            let mut idb = [0u8; 32];
            idb[..8].copy_from_slice(&block_counter.to_le_bytes());
            idb[31] = 0xBB;
            let block_id = BlockId::from_bytes(idb);
            let block = Block {
                id: block_id,
                domain_id,
                parent_id: genesis_id.as_block_id(),
                proposer: ProposerId::from_bytes(*pid.as_bytes()),
                child_state_ref: test_artifact_hash(1),
                diff_ref: test_artifact_hash(2),
                claimed_metric_delta: MetricValue::new(claimed),
                evidence_bundle_hash: test_artifact_hash(3),
                fee: TokenAmount::new(10),
                bond: TokenAmount::new(world.block_bond),
                epoch_id: EpochId(1),
                status: BlockStatus::Submitted,
                timestamp: 1_700_001_000 + block_counter,
            };
            if sim.submit_block(block).is_err() {
                continue;
            }
            report.blocks_submitted += 1;
            truth.insert(block_id, (true_delta, claimed));

            // Assigned validators attest per strategy.
            let assigned = sim.assign_validators(&block_id).unwrap();
            for v in &assigned {
                let vstrat = validator_by_id[v];
                let vpid = ParticipantId::from_bytes(*v.as_bytes());
                let (vote, observed) = match vstrat {
                    ValidatorStrategy::HonestReplay => {
                        *costs.entry(vpid).or_default() +=
                            world.replay_cost - world.validator_fee;
                        let obs = true_delta + world.noise_sd * rng.next_gauss();
                        let mismatch = (obs - claimed).abs();
                        if mismatch <= world.tolerance {
                            (ValidatorVote::Pass, Some(MetricValue::new(obs)))
                        } else if mismatch > world.fraud_threshold {
                            (ValidatorVote::FraudSuspected, None)
                        } else {
                            (ValidatorVote::Fail, None)
                        }
                    }
                    ValidatorStrategy::RubberStamp => {
                        *costs.entry(vpid).or_default() -= world.validator_fee;
                        (ValidatorVote::Pass, Some(MetricValue::new(claimed)))
                    }
                };
                sim.record_attestation(ValidationAttestation {
                    block_id,
                    validator: *v,
                    vote,
                    observed_delta: observed,
                    replay_evidence_ref: test_artifact_hash(4),
                    timestamp: 1_700_002_000,
                })
                .unwrap();
            }

            let outcome = match sim.evaluate_block(&block_id) {
                Ok(o) => o,
                Err(_) => continue,
            };
            if outcome != ProvisionalOutcome::Accepted {
                continue;
            }
            report.blocks_accepted += 1;

            // Challengers act while the block is in its challenge window.
            let mut upheld = false;
            for (ci, cstrat) in actors.challengers.iter().enumerate() {
                if upheld {
                    break;
                }
                let cpid = challenger_pid(ci);
                let wants_challenge = match cstrat {
                    ChallengerStrategy::Auditor { coverage } => {
                        if rng.next_f64() >= *coverage {
                            false
                        } else {
                            *costs.entry(cpid).or_default() += world.replay_cost;
                            let obs =
                                true_delta + world.noise_sd * rng.next_gauss();
                            (obs - claimed).abs() > world.tolerance
                        }
                    }
                    ChallengerStrategy::Griefer { rate } => {
                        rng.next_f64() < *rate
                    }
                };
                if !wants_challenge {
                    continue;
                }

                challenge_counter += 1;
                let mut cb = [0u8; 32];
                cb[..8].copy_from_slice(&challenge_counter.to_le_bytes());
                cb[31] = 0xCC;
                let challenge_id = ChallengeId::from_bytes(cb);
                if sim
                    .open_challenge(
                        challenge_id,
                        ChallengeType::BlockReplay,
                        ChallengeTarget::Block { block_id },
                        cpid,
                        TokenAmount::new(world.challenge_bond),
                        test_artifact_hash(5),
                    )
                    .is_err()
                {
                    continue;
                }
                report.challenges_opened += 1;
                sim.begin_challenge_review(&challenge_id).unwrap();

                // Adjudication resolves by ground truth: the claim is
                // fraudulent iff it exceeds the true delta by more than
                // the tolerance band.
                if (claimed - true_delta).abs() > world.tolerance {
                    sim.uphold_challenge(&challenge_id).unwrap();
                    report.challenges_upheld += 1;
                    report.blocks_invalidated += 1;
                    upheld = true;
                } else {
                    sim.reject_challenge(&challenge_id).unwrap();
                }
            }

            if !upheld {
                sim.close_challenge_window(&block_id).unwrap();
            }
        }
        sim.advance_epoch();
    }

    // Let every challenge window elapse, then settle all closeable blocks.
    for _ in 0..world.reward.challenge_window_epochs {
        sim.advance_epoch();
    }
    let ids: Vec<BlockId> = sim.blocks.keys().copied().collect();
    for id in ids {
        let _ = sim.settle_block(&id);
    }

    // --- Accounting: protocol truth (escrows + distributions) - costs ---
    let mut net: HashMap<ParticipantId, f64> = HashMap::new();
    for escrow in sim.escrow_records.values() {
        let entry = net.entry(escrow.beneficiary).or_default();
        match (escrow.kind, escrow.status) {
            (EscrowKind::ProposerBond, EscrowStatus::Slashed) => {
                *entry -= escrow.amount.as_u64() as f64;
            }
            (EscrowKind::ProvisionalReward, EscrowStatus::Released)
            | (EscrowKind::SurvivalReward, EscrowStatus::Released) => {
                *entry += escrow.amount.as_u64() as f64;
            }
            (EscrowKind::ChallengerBond, EscrowStatus::Slashed) => {
                *entry -= escrow.amount.as_u64() as f64;
            }
            _ => {} // Held or returned-to-owner flows net to zero
        }
    }
    for dist in sim.slash_distributions.values() {
        *net.entry(dist.challenger).or_default() +=
            dist.challenger_payout.as_u64() as f64;
    }
    for (pid, cost) in &costs {
        *net.entry(*pid).or_default() -= cost;
    }

    // --- Group by strategy ---
    let group = |label: String, pid: ParticipantId,
                     table: &mut HashMap<String, (f64, usize)>| {
        let e = table.entry(label).or_insert((0.0, 0));
        e.0 += net.get(&pid).copied().unwrap_or(0.0);
        e.1 += 1;
    };
    let mut p_table: HashMap<String, (f64, usize)> = HashMap::new();
    for (i, s) in actors.proposers.iter().enumerate() {
        group(format!("{:?}", s), proposer_pid(i), &mut p_table);
    }
    let mut v_table: HashMap<String, (f64, usize)> = HashMap::new();
    for (i, s) in actors.validators.iter().enumerate() {
        group(
            format!("{:?}", s),
            ParticipantId::from_bytes(*validator_vid(i).as_bytes()),
            &mut v_table,
        );
    }
    let mut c_table: HashMap<String, (f64, usize)> = HashMap::new();
    for (i, s) in actors.challengers.iter().enumerate() {
        let label = match s {
            ChallengerStrategy::Auditor { .. } => "Auditor".to_string(),
            ChallengerStrategy::Griefer { .. } => "Griefer".to_string(),
        };
        group(label, challenger_pid(i), &mut c_table);
    }
    let mean = |t: HashMap<String, (f64, usize)>| {
        t.into_iter()
            .map(|(k, (sum, n))| (k, sum / n.max(1) as f64))
            .collect::<HashMap<String, f64>>()
    };
    report.proposer_ev = mean(p_table);
    report.validator_ev = mean(v_table);
    report.challenger_ev = mean(c_table);
    report.net = net;
    report
}


// -----------------------------------------------------------------------
// §18: degenerate evaluation surface — abstract waste model
// -----------------------------------------------------------------------

/// Abstract model of attack-model.md §18 (degenerate evaluation surface).
///
/// A domain's metric is secretly exploitable: submissions matching the
/// exploit dominate the frontier without genuine research. Honest
/// proposers keep paying `work_cost` per round for rewards they cannot
/// win. The question is whether a deprecation mechanism bounds that
/// waste.
///
/// With the **evaluation-surface challenge** (bonded, budget-bounded
/// generator demonstration; see protocol spec), each round an independent
/// participant discovers and demonstrates the exploit with probability
/// `discovery_prob`, deprecating the domain and stopping further waste.
/// Without it, waste accrues for the full horizon.
///
/// Returns (waste_without_mechanism, waste_with_mechanism) over `rounds`.
pub fn degenerate_surface_waste(
    rounds: usize,
    honest_proposers: usize,
    work_cost: f64,
    discovery_prob: f64,
    seed: u64,
) -> (f64, f64) {
    let mut rng = Rng::new(seed);
    let per_round = honest_proposers as f64 * work_cost;

    let waste_without = rounds as f64 * per_round;

    let mut waste_with = 0.0;
    for _ in 0..rounds {
        waste_with += per_round;
        if rng.next_f64() < discovery_prob {
            break; // exploit demonstrated; domain deprecated
        }
    }
    (waste_without, waste_with)
}
