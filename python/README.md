# python/

Python research execution runners for AutoResearch Chain.

## Status

Phase 0 scaffold. Package structure and module stubs are in place. No runner logic is implemented yet.

## Package Layout

| Module | Purpose |
|--------|---------|
| `arc_runner/` | Top-level package; shared protocol client logic (future) |
| `arc_runner/proposer/` | Proposer execution runner — runs experiments and submits blocks |
| `arc_runner/validator/` | Validator replay runner — replays transitions and generates attestations |
| `arc_runner/challenger/` | Challenger replay/audit runner — disputes suspect claims |
| `arc_runner/autoresearch_adapter/` | Integration with autoresearch-style autonomous agent loops |
| `arc_runner/domains/` | Domain-specific experiment wrappers |
| `arc_runner/evidence/` | Evidence bundle creation and validation |
| `arc_runner/materialize/` | Materialized code state generation and packaging |

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
