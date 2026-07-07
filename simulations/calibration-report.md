<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Adversarial Calibration Report (Milestone C)

**Date:** 2026-07. **Harness:** `crates/adversarial-sim` (deterministic,
seed 42, 200 rounds per episode, reproducible via
`cargo run --release -p arc-adversarial-sim --bin calibrate`).

This report records the first parameter calibration of the protocol's
incentive system under scripted adversaries, driving the real protocol
state machine (`SimulatorState`). Model assumptions are documented in the
crate. Token units are arbitrary; what matters is the *ratios* between
reward, bond, and compute cost (here: base reward 1000, honest experiment
cost 300, replay cost 30).

## Headline findings

1. **Fraud is profitable at the naive parameters.** With bond 500 vs
   base reward 1000 (the readable-test defaults), a fraudster claiming a
   fake improvement is +EV unless audit coverage approaches ~80%. The
   slashable stake must dwarf what fraud can earn.
2. **Calibrated economics fix this.** With bond = 2x base reward and the
   provisional tranche reduced to 10%, the analytic break-even audit
   coverage drops to ~35%, and fraud is robustly -EV at 50% coverage.
3. **The challenge game, not validator diligence, is the binding security
   layer.** With a fully lazy (or colluding) validator pool and no
   challengers, fraud earns full rewards. A single auditor at 50%
   coverage makes the same fraud strategy deeply negative — and auditing
   itself is strongly profitable.
4. **Noise mining was free money — a protocol change was required.**
   Claims inside the replay tolerance band are unfalsifiable: validators
   pass them, challengers cannot demonstrate a mismatch, and each
   accepted noise claim earns the full block reward (+90,000/episode at
   zero risk). Fix implemented: `ValidationConfig::min_accepted_delta` —
   acceptance requires the validated mean improvement to clear a
   threshold that must be calibrated **above** the tolerance band. With
   it, noise mining nets only its attempt costs (-EV).
5. **Griefing loses; bounty hunting works.** Randomly challenging valid
   blocks forfeits bonds (-EV in an all-honest world). Randomly
   challenging in a fraud-dense world is profitable — that is the
   mechanism functioning, not a defect.
6. **Validator compensation is a real gap.** The protocol pays validators
   nothing: honest replay is strictly -EV; rubber-stamping is free. A
   per-attestation fee equal to replay cost makes honest validation break
   even, but pays lazy validators the same fee for no work. Fees alone
   cannot differentiate honest from lazy validation — **attestation-level
   slashing** (challengeable attestations; specified in the protocol,
   not implemented) is required.
7. **Rejected fraud is free.** Bonds are escrowed only at acceptance, so
   a fraud block rejected at validation costs the fraudster nothing and
   can be retried indefinitely. Recommendation: commit the bond at
   submission, forfeit a share on FraudSuspected rejection.

## Recommended parameters (Stage 1, pre-testnet)

| Parameter | Test default | Recommended | Basis |
|-----------|--------------|-------------|-------|
| block bond | 500 | >= 2x base block reward | fraud break-even coverage ~35% |
| provisional tranche | 20% | 10% | provisional is unclawable fraud exposure |
| challenger payout | 50% of slashed | 50% (unchanged) | auditing strongly +EV at all tested coverages |
| `min_accepted_delta` | 0 (disabled) | >= 2x tolerance band | closes noise mining |
| tolerance band | -- | >= measured replay noise, < min_accepted_delta / 2 | Milestone D measures real replay noise |
| validator fee | 0 (unimplemented) | >= replay cost, plus attestation slashing | compensation gap |

Code defaults are intentionally left at the readable-test values;
`arc_adversarial_sim::recommended_world()` encodes the calibrated set and
the scenario tests assert honest-play dominance under it. Production
parameters must be re-derived once Milestone D measures real replay costs
and noise on actual ML workloads.

## §18: degenerate evaluation surface

The abstract waste model (`degenerate_surface_waste`) confirms the shape
of the problem: honest compute waste on an exploited domain grows
linearly and unboundedly with the horizon absent a deprecation mechanism,
and is bounded by exploit-discovery time with one. The chosen mechanism —
the bonded, budget-bounded **evaluation-surface challenge** — is now
specified in `protocol-v0.2.md` (§ Evaluation-Surface Challenges) and
`attack-model.md` §18. Implementation is future work.

## Reproducing

```sh
cargo test -p arc-adversarial-sim --release   # scenario assertions
cargo run --release -p arc-adversarial-sim --bin calibrate  # full sweep
```

## Full sweep output

# Calibration sweep (deterministic, seed 42, 200 rounds)

## Fraud EV vs bond x provisional x coverage
(per-episode mean net EV; fraud must be negative)

| bond | provisional bps | coverage | fraud EV | honest EV | auditor EV |
|------|-----------------|----------|----------|-----------|------------|
| 500 | 2000 | 0.2 | 75300 | 140000 | 9680 |
| 500 | 2000 | 0.3 | 54500 | 140000 | 18400 |
| 500 | 2000 | 0.5 | 40200 | 140000 | 21930 |
| 500 | 2000 | 0.8 | -100 | 140000 | 37660 |
| 500 | 1000 | 0.2 | 73400 | 140000 | 10630 |
| 500 | 1000 | 0.3 | 51000 | 140000 | 20150 |
| 500 | 1000 | 0.5 | 35600 | 140000 | 24230 |
| 500 | 1000 | 0.8 | -7800 | 140000 | 41510 |
| 1000 | 2000 | 0.2 | 65800 | 140000 | 14430 |
| 1000 | 2000 | 0.3 | 37000 | 140000 | 27150 |
| 1000 | 2000 | 0.5 | 17200 | 140000 | 33430 |
| 1000 | 2000 | 0.8 | -38600 | 140000 | 56910 |
| 1000 | 1000 | 0.2 | 63900 | 140000 | 15380 |
| 1000 | 1000 | 0.3 | 33500 | 140000 | 28900 |
| 1000 | 1000 | 0.5 | 12600 | 140000 | 35730 |
| 1000 | 1000 | 0.8 | -46300 | 140000 | 60760 |
| 2000 | 2000 | 0.2 | 46800 | 140000 | 23930 |
| 2000 | 2000 | 0.3 | 2000 | 140000 | 44650 |
| 2000 | 2000 | 0.5 | -28800 | 140000 | 56430 |
| 2000 | 2000 | 0.8 | -115600 | 140000 | 95410 |
| 2000 | 1000 | 0.2 | 44900 | 140000 | 24880 |
| 2000 | 1000 | 0.3 | -1500 | 140000 | 46400 |
| 2000 | 1000 | 0.5 | -33400 | 140000 | 58730 |
| 2000 | 1000 | 0.8 | -123300 | 140000 | 99260 |
| 4000 | 2000 | 0.2 | 8800 | 140000 | 42930 |
| 4000 | 2000 | 0.3 | -68000 | 140000 | 79650 |
| 4000 | 2000 | 0.5 | -120800 | 140000 | 102430 |
| 4000 | 2000 | 0.8 | -269600 | 140000 | 172410 |
| 4000 | 1000 | 0.2 | 6900 | 140000 | 43880 |
| 4000 | 1000 | 0.3 | -71500 | 140000 | 81400 |
| 4000 | 1000 | 0.5 | -125400 | 140000 | 104730 |
| 4000 | 1000 | 0.8 | -277300 | 140000 | 176260 |

## Rubber-stamp validators, no challengers (fraud unpunished)
fraud EV 200000, honest EV 140000 (accepted 600, invalidated 0)

## Rubber-stamp validators + auditor (challenger as backstop)
fraud EV -61000, honest EV 137500, auditor EV 122090 (upheld 90/92 challenges)

## Noise miner vs minimum-improvement threshold
(tolerance 0.004; min_accepted_delta is the lever)

| min_accepted_delta | noise miner EV | honest EV |
|--------------------|----------------|-----------|
| 0.000 | 90000 | 140000 |
| 0.004 | -2000 | 140000 |
| 0.008 | -2000 | 140000 |
| 0.012 | -2000 | 133000 |

## Challenging valid blocks (griefing) vs bounty-hunting fraud
griefer EV in honest world: -21600
random challenger EV with a fraudster present: 19500 (upheld 30/150)

## Validator compensation gap
| validator fee | honest validator EV | rubber-stamp EV |
|---------------|---------------------|-----------------|
| 0 | -9000 | 0 |
| 15 | -4500 | 4500 |
| 30 | 0 | 9000 |
| 45 | 4500 | 13500 |
