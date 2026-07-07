# simulations/

Adversarial and economic simulation results for AutoResearch Chain.

## Status

The simulation harness lives in `crates/adversarial-sim` (Rust; drives
the real protocol state machine with strategy-parameterized proposers,
validators, and challengers, with deterministic seeded randomness). This
directory holds its written outputs.

- [`calibration-report.md`](calibration-report.md) — first parameter
  calibration (Milestone C): fraud/noise-mining/griefing EV analysis,
  recommended Stage 1 parameters, and identified protocol gaps
  (validator compensation, bond-at-submission).

Run the scenario suite and sweep:

```sh
cargo test -p arc-adversarial-sim --release
cargo run --release -p arc-adversarial-sim --bin calibrate
```

Future work: economic stress tests (reward starvation, domain creation
pressure, fork proliferation) and successor-track migration scenarios.
These simulations correspond to Phase 4 in the
[Implementation Plan](../docs/implementation-plan.md).
