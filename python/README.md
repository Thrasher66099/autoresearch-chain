# python/

Python research execution runners for AutoResearch Chain.

## Status

Phase 2 substantially complete. Proposer, validator, and challenger runners are
implemented and exercised end-to-end against the Rust `arc-node` binary,
including a real-computation demo (`python -m arc_runner.demo`) that runs the
full protocol lifecycle on the QMD query-expansion domain across two
generations of improvement. Frontier materialization
(`arc_runner/materialize/`) is implemented: content-addressed state
manifests, structured state diffs, verified assembly, and diff-chain
resolution.

## Package Layout

| Module | Purpose |
|--------|---------|
| `arc_runner/client.py` | `ArcNodeClient` — protocol transport; shells the `arc-node` CLI, one state transition per invocation, JSON in/out |
| `arc_runner/proposer/` | Proposer runner — queries the frontier, submits blocks with evidence bundles |
| `arc_runner/validator/` | Validator runner — finds pending blocks, fetches evidence, submits replay attestations |
| `arc_runner/challenger/` | Challenger runner — finds suspect blocks, fetches evidence, opens bonded challenges |
| `arc_runner/autoresearch_adapter/` | Pulls frontier state, enforces frozen/search surfaces, captures results into evidence bundles |
| `arc_runner/domains/` | Domain-specific experiment wrappers — QMD genesis packaging and training/eval/replay engine |
| `arc_runner/evidence/` | Content-addressed (BLAKE3) evidence bundle creation, matching the Rust storage model |
| `arc_runner/materialize/` | Materialized state snapshots (content-addressed manifests), structured state diffs, verified assembly, diff-chain resolution |
| `arc_runner/demo.py` | End-to-end lifecycle demo with real computation (requires a built `arc-node`) |

## Requirements

Integration tests and the demo shell out to the `arc-node` binary. Build it
first from the repo root (`cargo build`); it is located via the `ARC_NODE_BIN`
environment variable or `target/{debug,release}/arc-node`. Tests that need it
skip automatically when it is absent.

## Development

```sh
cd python
pip install -e ".[dev]"
python -m pytest
python -m ruff check .
```

## Relationship to the Rust core

The Rust protocol core (`crates/`) is the authority on state transitions.
This Python package executes the actual research work off-chain and packages
results for protocol submission. It is protocol-coupled but not protocol-defining.

See [Implementation Plan](../docs/implementation-plan.md) for build phases and priorities.
