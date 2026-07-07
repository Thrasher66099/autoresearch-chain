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

//! Calibration sweep: fraud EV as a function of bond size, provisional
//! fraction, and audit coverage; plus honest baselines and secondary
//! scenarios. Output feeds `simulations/calibration-report.md`.

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

fn main() {
    println!("# Calibration sweep (deterministic, seed 42, 200 rounds)\n");

    println!("## Fraud EV vs bond x provisional x coverage");
    println!("(per-episode mean net EV; fraud must be negative)\n");
    println!("| bond | provisional bps | coverage | fraud EV | honest EV | auditor EV |");
    println!("|------|-----------------|----------|----------|-----------|------------|");
    for &bond in &[500u64, 1_000, 2_000, 4_000] {
        for &prov_bps in &[2_000u64, 1_000] {
            for &coverage in &[0.2f64, 0.3, 0.5, 0.8] {
                let mut w = WorldConfig::default();
                w.block_bond = bond;
                w.reward.provisional_reward_bps = prov_bps;
                let r = run_episode(&w, &standard_actors(coverage));
                println!(
                    "| {} | {} | {:.1} | {:.0} | {:.0} | {:.0} |",
                    bond,
                    prov_bps,
                    coverage,
                    r.proposer_ev["Fraudster"],
                    r.proposer_ev["Honest"],
                    r.challenger_ev["Auditor"],
                );
            }
        }
    }

    println!("\n## Rubber-stamp validators, no challengers (fraud unpunished)");
    let mut w = WorldConfig::default();
    w.block_bond = 2_000;
    w.reward.provisional_reward_bps = 1_000;
    let mut a = standard_actors(0.0);
    a.validators = vec![ValidatorStrategy::RubberStamp; 6];
    a.challengers = vec![];
    let r = run_episode(&w, &a);
    println!(
        "fraud EV {:.0}, honest EV {:.0} (accepted {}, invalidated {})",
        r.proposer_ev["Fraudster"],
        r.proposer_ev["Honest"],
        r.blocks_accepted,
        r.blocks_invalidated
    );

    println!("\n## Rubber-stamp validators + auditor (challenger as backstop)");
    let mut a = standard_actors(0.5);
    a.validators = vec![ValidatorStrategy::RubberStamp; 6];
    let r = run_episode(&recommended_world(), &a);
    println!(
        "fraud EV {:.0}, honest EV {:.0}, auditor EV {:.0} (upheld {}/{} challenges)",
        r.proposer_ev["Fraudster"],
        r.proposer_ev["Honest"],
        r.challenger_ev["Auditor"],
        r.challenges_upheld,
        r.challenges_opened
    );

    println!("\n## Noise miner vs minimum-improvement threshold");
    println!("(tolerance 0.004; min_accepted_delta is the lever)\n");
    println!("| min_accepted_delta | noise miner EV | honest EV |");
    println!("|--------------------|----------------|-----------|");
    for &min_delta in &[0.0f64, 0.004, 0.008, 0.012] {
        let mut w = recommended_world();
        w.min_accepted_delta = min_delta;
        let mut a = standard_actors(0.3);
        a.proposers = vec![ProposerStrategy::Honest, ProposerStrategy::NoiseMiner];
        let r = run_episode(&w, &a);
        println!(
            "| {:.3} | {:.0} | {:.0} |",
            min_delta, r.proposer_ev["NoiseMiner"], r.proposer_ev["Honest"]
        );
    }

    println!("\n## Challenging valid blocks (griefing) vs bounty-hunting fraud");
    // Griefing proper: an all-honest world, so every random challenge
    // targets a valid block and loses its bond.
    let mut a = standard_actors(0.0);
    a.proposers = vec![ProposerStrategy::Honest, ProposerStrategy::Honest];
    a.challengers = vec![ChallengerStrategy::Griefer { rate: 0.3 }];
    let r = run_episode(&recommended_world(), &a);
    println!("griefer EV in honest world: {:.0}", r.challenger_ev["Griefer"]);
    // In a fraud-dense world, random challenging is bounty hunting and
    // may be profitable — that is the mechanism working, not a defect.
    let mut a = standard_actors(0.0);
    a.challengers = vec![ChallengerStrategy::Griefer { rate: 0.3 }];
    let r = run_episode(&recommended_world(), &a);
    println!(
        "random challenger EV with a fraudster present: {:.0} (upheld {}/{})",
        r.challenger_ev["Griefer"], r.challenges_upheld, r.challenges_opened
    );

    println!("\n## Validator compensation gap");
    println!("| validator fee | honest validator EV | rubber-stamp EV |");
    println!("|---------------|---------------------|-----------------|");
    for &fee in &[0.0f64, 15.0, 30.0, 45.0] {
        let mut w = recommended_world();
        w.validator_fee = fee;
        let r = run_episode(&w, &standard_actors(0.3));
        println!(
            "| {:.0} | {:.0} | {:.0} |",
            fee,
            r.validator_ev["HonestReplay"],
            r.validator_ev["RubberStamp"]
        );
    }
}
