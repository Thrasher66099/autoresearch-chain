# crates/

Rust workspace for the AutoResearch Chain protocol core.

## Status

Phase 0 scaffold. Crate structure, type stubs, and inter-crate dependencies are in place. No protocol logic is implemented yet.

## Crate Layout

| Crate | Kind | Purpose |
|-------|------|---------|
| `protocol-types` | lib | Core structs, enums, IDs, hashes, references — the foundational type vocabulary |
| `protocol-rules` | lib | Deterministic state transition logic |
| `domain-engine` | lib | Domains, research track standards, genesis activation, track trees |
| `fork-engine` | lib | Fork families, dominance evaluation, frontier selection |
| `challenge-engine` | lib | Challenge types, resolution rules, remedies |
| `reward-engine` | lib | Staged rewards, escrows, slashing, domain-local accounting |
| `storage-model` | lib | Artifact references, content-addressed metadata, materialized state storage |
| `simulator` | lib | Local protocol simulator and scenario engine (primary Phase 0 target) |
| `node` | bin | Minimal local runtime (Phase 1 target) |
| `cli` | bin | Command-line interface (Phase 1 target) |

## Dependency graph (simplified)

```
protocol-types  (no internal deps — foundation crate)
    ↑
    ├── protocol-rules
    ├── domain-engine
    ├── fork-engine
    ├── challenge-engine
    ├── reward-engine
    ├── storage-model
    │
    └── simulator  (depends on all above)
            ↑
            ├── node
            └── cli
```

## Build

From the repository root:

```sh
cargo build --workspace
cargo test --workspace
```

See [Implementation Plan](../docs/implementation-plan.md) for build phases and priorities.
