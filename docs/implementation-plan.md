<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Implementation Plan

## Purpose

This document defines the technical implementation plan for AutoResearch Chain.

It is intentionally practical rather than aspirational.

The goal is not to freeze the architecture permanently.
The goal is to define:

- the chosen architectural direction,
- the core subsystem split,
- the build order,
- the implementation milestones and their current status,
- and the major engineering questions that should stay active during development.

This is a living implementation plan for a protocol that is still evolving.
It was originally written before any code existed and has been revised to reflect the experience of implementing Phase 0.

---

## Chosen Implementation Direction

AutoResearch Chain will be implemented using a **Rust-native custom chain architecture** as the primary protocol substrate.

This choice follows directly from the project's priorities:

1. security
2. extensibility
3. scalability
4. performance

EVM compatibility is explicitly **not** the design center.
It may be added later if useful, but it must not constrain the protocol.

The project is not a generic crypto product.
The blockchain is a necessary institutional technology for:
- permissionless participation,
- trustless rule enforcement,
- non-custodial reward settlement,
- and adversarial market legitimacy.

The chain must therefore be built around the actual research-market game, not around preexisting smart-contract ecosystem expectations.

---

## High-Level System Split

The system should be built as four major layers.

### Layer A — Protocol Core (Rust)

This is the canonical protocol brain.

Responsibilities:
- core protocol data types
- deterministic state transitions
- research track and domain initialization
- block submission rules
- validator assignment
- challenge lifecycle
- fork and dominance logic
- canonical frontier settlement
- domain-scoped accounting
- escrow and slashing logic
- successor-track and migration rules

This is the most important layer in the system.

**Status:** Core subsystems implemented and tested (protocol-types, protocol-rules, domain-engine, fork-engine, challenge-engine, reward-engine, simulator). See Current Status below.

---

### Layer B — Research Execution Layer (Python)

This is where useful work happens.

Responsibilities:
- autonomous research-agent integration
- proposer execution runner
- validator replay runner
- challenger replay/audit runner
- evidence bundle generation
- materialized code state packaging
- domain-specific experiment wrappers

This layer is off-chain but protocol-coupled.

It should integrate naturally with autoresearch-style loops.

**Status:** Initial implementation exists. The `arc-runner` Python package provides content-addressed evidence bundling (BLAKE3, matching the Rust storage-model), a QMD domain-specific genesis packager, and an autoresearch adapter with frozen/search surface enforcement. Proposer, validator, and challenger runners are structurally scaffolded. Full runner integration with the Rust protocol core (Phase 2) is not yet complete.

---

### Layer C — Artifact / Data Availability Layer

This layer stores and serves heavy protocol-relevant artifacts.

Responsibilities:
- diffs
- manifests
- configs
- logs
- metric outputs
- materialized code snapshots
- evidence bundles
- dataset and environment references

The protocol should store references and commitments, not large artifacts directly.

---

### Layer D — Operator and User Interfaces

This layer should come after the protocol core is behaving correctly.

Responsibilities:
- CLI
- validator tools
- domain discovery tools
- frontier inspection
- explorer
- dashboards
- developer APIs

This layer matters, but it should not be overbuilt before the game itself is validated.

---

## Strategic Implementation Principle

The system should be built as a **protocol simulator first** and a **networked chain second**.

That means the first technical target is not:
- wallets,
- explorers,
- networking,
- or polished public-node infrastructure.

The first target is:

**a local deterministic Rust implementation of the protocol state machine**

The first major question was:

> Does the game behave correctly when represented as executable state transitions?

That question has been substantially answered. The protocol state machine runs deterministically in the simulator, covering domain activation, block lifecycle, validation, challenge, fork competition, frontier settlement, and escrow management. The simulator-first approach proved its value: implementation exposed real design issues that would have been invisible in specification alone — proposer-fallback metric truth (the protocol was implicitly trusting proposer-claimed deltas), silent direction assumptions in metric comparisons, and acceptance of non-truth-bearing attestations (Pass votes without observed data).

The question now is whether the protocol survives adversarial pressure and whether real useful-work runners can connect to it.

---

## Current Status

Phase 0 is substantially complete. The Rust workspace contains 10 crates (~10,600 lines of code, 268 tests). The core protocol state machine runs deterministically in the simulator, composing all engines through integrated scenario tests. Whole-state snapshot persistence is implemented, enabling save/load of the complete protocol state.

### Crate status

| Crate | Status | Notes |
|-------|--------|-------|
| `protocol-types` | Implemented | Full type vocabulary, structural validation, serde, test fixtures |
| `protocol-rules` | Implemented | Attestation aggregation (truth-bearing), block lifecycle state machine, validator assignment |
| `domain-engine` | Implemented | Genesis activation state machine, RTS-1 conformance, domain registry |
| `fork-engine` | Implemented | Fork families, metric-based dominance, frontier selection, invalidation handling |
| `challenge-engine` | Partial | State machine implemented; economics, escalation, remedy application deferred |
| `reward-engine` | Partial | Escrow create/release/slash implemented; staged rewards, attribution distribution deferred |
| `simulator` | Implemented | Integrated state machine composing all engines; 51 scenario tests; whole-state snapshot persistence (serde JSON) |
| `storage-model` | Partial | Content-addressed artifact store (BLAKE3), ArtifactStore trait, InMemoryArtifactStore, evidence bundling, file-from-disk ingestion; Python-Rust hash agreement verified; materialization triggers and frontier assembly not yet implemented |
| `node` | Bootstrap | Phase 1 target; `init`/`inspect` commands and file-based state persistence implemented |
| `cli` | Stub | Phase 1 target |

### Protocol-truth hardening

The Phase 0 implementation established several protocol-truth invariants that were not fully articulated in the original specification:

- Protocol truth is derived exclusively from validator-observed data, never from proposer claims.
- Invalidated and Rejected block semantics are properly separated (upheld challenge vs. failed validation).
- `DerivedValidity` serves as a centralized truth surface for all downstream logic (`DirectValid`, `DirectInvalid`, `AncestryInvalid`).
- Escrow release is gated on both epoch timing and branch validity.
- Truth-bearing attestation semantics: a Pass vote without `observed_delta` does not count toward acceptance.

---

## Build Phases

## Phase 0 — Protocol Modeling

### Goal

Turn the protocol spec into executable Rust types and transitions.

### Deliverables

The protocol core should define at minimum:

- `ProblemDomain` — **done**
- `DomainSpec` — **done**
- `ResearchTrackStandard` — **done**
- `GenesisBlock` — **done**
- `TrackInitialization` — **done**
- `TrackTree` — **done**
- `EpochSpec` — **done**
- `Block` — **done**
- `ForkFamily` — **done**
- `ValidationAttestation` — **done**
- `ChallengeRecord` — **done**
- `AttributionClaim` — **done**
- `CanonicalFrontierState` — **done**
- `MaterializedState` — **done** (type defined; artifact store available, frontier assembly not implemented)
- `CodebaseStateRef` — **done** (type defined; artifact store available, reference resolution not implemented)
- `EscrowRecord` — **done**
- `MetricIntegrityPolicy` — **done**
- `DatasetIntegrityPolicy` — **done**

Types created beyond the original list:

- `DerivedValidity` — centralized truth surface for branch validity (`DirectValid`, `DirectInvalid`, `AncestryInvalid`)
- `ValidatedBlockOutcome` — protocol-truth record of validator-observed metrics per block
- `AttestationSummary` — aggregated attestation counts, truth-bearing pass count, median delta
- `ProvisionalOutcome` — provisional acceptance/rejection/inconclusive decision
- `MaterializationPolicyKind` — trigger categories for frontier materialization
- `DatasetSplits` — structured train/test/validation dataset partitioning

And core logic for:

- domain creation — **done**
- track activation — **done** (full genesis state machine with RTS-1 conformance)
- seed-score verification state — **done** (structurally embedded in genesis validation)
- block submission — **done** (structural validation, policy checks, parent lineage)
- validator assignment — **done** (deterministic hash-based pool assignment)
- attestation aggregation — **done** (truth-bearing semantics, median delta, provisional outcome)
- challenge opening and resolution — **done** (state machine; economics and remedy deferred)
- fork activation and dominance — **done** (fork families, metric-based dominance, derived validity filtering)
- frontier settlement — **done** (validity-aware frontier selection with invalidation recomputation)
- reward staging — **partial** (escrow create/release/slash implemented; staged multi-epoch release deferred)
- slashing outcomes — **done** (escrow slashing on upheld challenges)
- cross-domain integration — **not started**
- successor-track creation — **not started**

### Success condition

A local simulator can model:

- genesis proposal — **done**
- track activation — **done**
- block submission — **done**
- validation outcomes — **done**
- challenge outcomes — **done**
- fork competition — **done**
- frontier updates — **done**
- reward accounting — **partial** (escrow lifecycle works; staged attribution deferred)
- domain-local and cross-domain behavior — **partial** (domain-local works; cross-domain effects not implemented)

without any real networking.

This phase should be heavily test-driven. It has been: 268 tests cover the protocol core, storage model, and snapshot persistence.

### Phase 0 — Remaining Work

1. Challenge economics and escalation (bond distribution, remedy application, escalation to governance)
2. Staged reward release (multi-stage escrow, attribution-weighted distribution)
3. Storage-model: materialization triggers, frontier state assembly, data availability checking (content-addressed artifact store with BLAKE3 hashing now implemented)
4. Successor-track creation and metric migration
5. Cross-domain integration effects
6. Reproducibility tolerance model (formalized thresholds beyond configurable parameters)

---

## Phase 1 — Local Single-Node Runtime

**Status:** Bootstrap started. Whole-state snapshot persistence implemented (JSON file save/load). Node binary has `init` and `inspect` commands. Full transaction submission flow, state queries, and CLI commands remain to be built.

### Goal

Wrap the protocol core in a minimal executable local runtime.

### Deliverables

- single-node chain state persistence — **done** (whole-state JSON snapshot; incremental persistence deferred)
- transaction-like submission flow for:
  - genesis proposals
  - blocks
  - attestations
  - challenges
- local state queries
- event log or state-transition trace
- minimal CLI commands for:
  - inspect domains
  - inspect tracks
  - inspect frontiers
  - submit local actions

### Success condition

A single machine can host a functioning local version of the protocol and execute the entire research-market lifecycle in dev mode.

---

## Phase 2 — Python Research Runner Integration

**Status:** Partially started. Evidence bundle packaging with content-addressed hashing (BLAKE3) is implemented with verified Python-Rust hash agreement. A QMD domain-specific genesis packager exists. The autoresearch adapter implements frozen/search surface enforcement. Proposer, validator, and challenger runners are scaffolded but not yet connected to the Rust protocol core. Canonical frontier pull helper is not yet implemented.

### Goal

Connect real useful-work loops to the protocol.

### Deliverables

- proposer runner — scaffolded
- validator runner — scaffolded
- challenger runner — scaffolded
- evidence bundle packager — **done** (BLAKE3 content-addressed hashing, Python-Rust hash agreement verified)
- domain experiment wrappers — **partial** (QMD query-expansion genesis packager implemented)
- canonical frontier pull helper — not started
- autoresearch-style adapter — **partial** (frozen/search surface enforcement implemented)

The Python side should support:

- pulling the current frontier state
- running a bounded experiment
- producing a diff
- packaging all required artifacts
- submitting protocol-ready evidence

### Success condition

A user can:

- pull a domain frontier,
- run a research agent,
- generate a candidate improvement,
- submit it,
- and have a validator replay it locally.

This is the first point at which the protocol and useful-work layer truly connect.

---

## Phase 3 — Artifact Layer and Frontier Materialization

**Status:** Partially started. The type-level foundation exists (`MaterializedState`, `CanonicalFrontierState`, `CodebaseStateRef`, `MaterializationPolicyKind`). The `storage-model` crate now implements content-addressed artifact storage (BLAKE3 hashing, `ArtifactStore` trait, `InMemoryArtifactStore`, `ArtifactKind` classification, `ArtifactMetadata`). Materialization triggers, frontier assembly, and data availability checking remain future work.

### Goal

Make the protocol usable as a real research substrate.

### Deliverables

- content-addressed artifact store or equivalent reference model
- evidence bundle storage rules
- materialized code state generation
- canonical frontier snapshot generation
- pullable codebase state resolution
- artifact retrieval tooling

### Success condition

For every active domain, a participant can pull:

- the current canonical codebase,
- current config,
- current environment manifest,
- current evaluation harness,
- and enough metadata to begin mining from the latest frontier.

This is essential to make the chain more than a ledger of diffs.

---

## Phase 4 — Multi-Actor Simulation and Adversarial Testing

**Status:** Not started as a dedicated phase. The simulator already covers some adversarial scenarios at unit/integration level (fork competition, invalidation cascades, ancestry poisoning, truth-bearing attestation filtering). Phase 4 is about sustained multi-actor economic stress testing.

### Goal

Stress the mechanism under realistic and adversarial conditions.

### Deliverables

Simulation harnesses for:

- multiple proposers
- multiple validators
- multiple challengers
- branch spam
- bad genesis proposals
- fork proliferation
- failed tracks
- challenge abuse
- domain creation pressure
- reward starvation across domains
- successor-track migration scenarios

### Success condition

The team can run adversarial simulations to test whether:

- rewards behave sensibly,
- challenges actually matter,
- domains remain tractable,
- frontier settlement remains coherent,
- and no obvious economic failure mode dominates.

This phase is vital because the protocol is fundamentally a game-theoretic system.

---

## Phase 5 — Minimal Real Testnet

### Goal

Allow real external participants to interact with the protocol.

### Deliverables

- networked node implementation
- permissionless validator registration
- real identity/address model
- actual bonded actions
- queryable state APIs
- external CLI
- minimal explorer or inspection tools
- artifact availability integration

### Success condition

External users can:

- propose a track
- activate a track
- mine recipe improvements
- validate results
- challenge claims
- inspect frontiers
- pull canonical states

At this point, the protocol becomes a real decentralized system rather than just a local model.

---

## Repository Architecture

The implementation is organized around clear subsystem boundaries.

### Rust

Crate layout:

- `crates/protocol-types/`
  - core structs, enums, IDs, hashes, references, structural validation, serde, test fixtures

- `crates/protocol-rules/`
  - attestation aggregation (truth-bearing semantics), block lifecycle state machine, validator assignment, configuration

- `crates/domain-engine/`
  - genesis activation state machine, RTS-1 conformance checking, domain registry, track trees

- `crates/fork-engine/`
  - fork families, metric-based dominance evaluation, frontier selection, invalidation handling, derived validity filtering

- `crates/challenge-engine/`
  - challenge state machine, target validation, bond checks (economics and remedy deferred)

- `crates/reward-engine/`
  - escrow create/release/slash, epoch-gated release timing (staged rewards and attribution deferred)

- `crates/storage-model/`
  - content-addressed artifact store (BLAKE3 hashing), `ArtifactStore` trait, `InMemoryArtifactStore`, `ArtifactKind` classification, `ArtifactMetadata`, evidence bundling, content verification

- `crates/simulator/`
  - integrated protocol state machine composing all engines, scenario test harness

- `crates/node/`
  - minimal local runtime / future node executable (stub — Phase 1 target)

- `crates/cli/`
  - command-line interface (stub — Phase 1 target)

### Python

The `arc-runner` package (`python/arc_runner/`) is partially implemented:

- `python/arc_runner/`
  - shared protocol client logic

- `python/arc_runner/proposer/`
  - proposer execution runner (scaffolded)

- `python/arc_runner/validator/`
  - validator replay runner (scaffolded)

- `python/arc_runner/challenger/`
  - challenger replay/audit runner (scaffolded)

- `python/arc_runner/autoresearch_adapter/`
  - integration with autoresearch-style loops; frozen/search surface enforcement implemented

- `python/arc_runner/domains/`
  - domain-specific experiment wrappers; QMD query-expansion genesis packager implemented

- `python/arc_runner/evidence/`
  - evidence bundle creation and validation; content-addressed hashing (BLAKE3) with Python-Rust hash agreement

- `python/arc_runner/materialize/`
  - materialized code state generation and packaging (scaffolded)

### Shared

- `spec/`
  - normative protocol documents

- `fixtures/`
  - example genesis blocks, test domains, example recipes

- `simulations/`
  - adversarial scenarios and economic test cases

---

## First Implementation Priorities

The following items should be implemented before broader infrastructure work.

### 1. Domain and Genesis Activation — Done

The chain must not remain implicitly tied to one domain.
Genesis and track activation logic should be among the first things implemented.

Full genesis activation state machine implemented with RTS-1 conformance checking.

### 2. Base Block / Validation / Challenge Loop — Done

The core game loop must work in one domain before broadening.

Block submission, validation (truth-bearing attestation aggregation), challenge lifecycle, fork competition, frontier settlement, and escrow management all implemented in the simulator.

### 3. Multi-Domain Support — Partial

The protocol should support multiple domains early enough that it does not ossify around a single benchmark.

Domain independence works (domain registry, per-domain fork state, domain-scoped block lineage). Cross-domain integration effects are not yet implemented.

### 4. Canonical Frontier Materialization — Not done

Users must be able to pull the current best assembled state.
This is essential to the protocol's practical usability.

The frontier types exist (`CanonicalFrontierState`, `MaterializedState`, `CodebaseStateRef`). The `storage-model` crate now implements content-addressed artifact storage, but reference resolution into pullable assembled states is not yet implemented.

### 5. Python Runner Integration — Partial

The chain becomes real only when it can actually receive useful work from autonomous agent loops and replay workers.

The Python evidence layer now exists (`arc-runner` package): content-addressed evidence bundling with BLAKE3 hashing matching the Rust storage-model, a QMD domain-specific genesis packager, and an autoresearch adapter with frozen/search surface enforcement. Full proposer/validator/challenger runner integration with the Rust protocol core is not yet done.

---

## Immediate Technical Decisions to Lock

Not every design choice needs to be final now.
But several should be decided relatively early.

### Canonical Serialization — Partially locked

The protocol needs canonical formats for:

- configs
- manifests
- evidence bundles
- research target declarations
- metric reports
- track initialization packages

Without canonical serialization, hashing and replay become fragile.

Serde JSON serialization is implemented for all protocol types, including the complete simulator state (enabling whole-state snapshot persistence). Protocol identifier types serialize as hex strings, making JSON output human-readable and usable as map keys. A canonical binary format for hashing (deterministic byte-level representation) has not yet been chosen.

### Reference and Hash Model — Locked

The protocol needs a clean model for how it references:

- states
- diffs
- evidence bundles
- materialized snapshots
- datasets
- evaluation harnesses

This should be explicit and stable early.

`ArtifactHash` is defined as the type-level reference primitive. Content-addressed hashing uses BLAKE3 (32-byte output mapping directly to `ArtifactHash`). The hash is determined solely by content bytes — artifact kind is metadata, not identity. The `ArtifactStore` trait defines storage/retrieval, and `InMemoryArtifactStore` provides the in-memory implementation. The Python evidence layer uses the same BLAKE3 algorithm, and hash agreement between Rust and Python is verified by tests. Resolution of references into pullable assembled states (frontier materialization) is not yet implemented.

### Reproducibility Tolerance Model — Partially locked

The protocol must define:

- score tolerance
- seed score reproduction tolerance
- attestation aggregation thresholds
- inconclusive conditions

Configurable thresholds exist in `ProtocolConfig` (acceptance threshold, pass threshold). A broader model covering cross-run variance and environment-dependent tolerance is not yet formalized.

### Materialization Policy — Partially locked

The protocol must define when a frontier or chain of diffs becomes a materialized state.

Potential triggers include:

- dominance
- depth threshold
- scheduled checkpoint
- domain policy rule

`MaterializationPolicyKind` enum exists with these trigger categories. Actual trigger evaluation and materialization execution are not yet implemented.

### Domain Activation Lifecycle — Done

The protocol now explicitly includes genesis and track creation.
Those state transitions should be implemented early as first-class logic, not bolted on later.

Full state machine implemented: Proposed → ConformanceChecking → ValidationInProgress → ActivationPending → Active/Failed/Expired, with RTS-1 conformance checking.

### Derived Validity Model — Locked during implementation

`DerivedValidity` enum (`DirectValid`, `DirectInvalid`, `AncestryInvalid`) serves as the centralized truth surface for all downstream decisions about whether a block's branch is viable. Fork dominance evaluation, frontier selection, and escrow release all gate on derived validity.

### Protocol Truth Source — Locked during implementation

Protocol truth is derived exclusively from validator-observed data. Proposer-claimed metrics are recorded but never used for acceptance, dominance, or frontier decisions. `ValidatedBlockOutcome` captures the validator-observed metric values that the protocol treats as ground truth.

---

## What Not to Overbuild Yet

The following should not be the early focus:

- polished tokenomics
- governance UI
- wallet integrations
- EVM compatibility
- public-node operations
- bridge strategy
- advanced explorer design
- advanced cross-track synthesis
- final Stage 3 long-horizon training execution design

These may matter later, but they are not the current bottleneck.

The current bottleneck is:

**Can the protocol survive adversarial pressure, and can real useful-work runners connect to it?**

---

## First Major Deliverable

The original first major deliverable was:

**A local protocol simulator with real domain/genesis/block/validation/challenge/fork/frontier logic and Python runner hooks**

This is substantially achieved. The simulator implements domain activation, block lifecycle, validation with truth-bearing attestations, challenge lifecycle, fork competition with metric-based dominance, frontier settlement with derived validity filtering, and escrow management. Python runner hooks remain the outstanding piece.

What the implementation produced:

- executable protocol logic that matches and refines the spec
- adversarial testability (268 tests, including invalidation cascades, ancestry poisoning, and snapshot persistence round-trips)
- spec contradiction discovery (proposer-truth fallback, silent metric direction, non-truth-bearing attestation acceptance)
- a foundation for all subsequent phases

### Next major deliverable

**A local end-to-end research loop: the hardened protocol simulator, a minimal node runtime with persistence, one real RTS-1 domain, and Python proposer/validator/challenger runner hooks.**

This connects the protocol to actual useful work for the first time.

---

## Main Open Engineering Questions

These questions should stay active during implementation:

- how strict should genesis activation thresholds be? — **partially resolved.** Configurable thresholds exist; real-world calibration requires testnet data.
- how should artifact references be structured? — **resolved.** `ArtifactHash` is the reference primitive, content-addressed via BLAKE3. `ArtifactStore` trait defines storage/retrieval. `ArtifactKind` classifies artifact roles. Python-Rust hash agreement is verified. Resolution into assembled pullable states (frontier materialization) remains open.
- how often should frontier states be materialized? — **still open.** `MaterializationPolicyKind` enumerates trigger categories but policy evaluation is not implemented.
- how should challenge escalation be encoded in v0? — **still open.** Basic challenge state machine works; escalation path and governance interaction are deferred.
- how should domain-local reward accounting be represented internally? — **partially resolved.** Per-block escrow works; attribution-weighted distribution across contributors is deferred.
- how should successor tracks and metric migration work in code? — **still open.**
- how formulaic should attribution be in early versions? — **still open.** `AttributionClaim` and `AttributionType` exist as types but no distribution logic is implemented.
- how much replay metadata must be mandatory at protocol level versus runner level? — **still open.** `EvidenceBundle` captures references; the boundary between protocol-mandatory and runner-optional metadata is not yet drawn.

Questions that emerged during implementation:

- How should deep-history invalidation interact with settlement finality? An upheld challenge invalidates a block and poisons its descendants, but if descendants have already settled, the cascade creates retroactive state changes.
- What is the gap between structural validation (types are correct, lineage exists) and data availability checking (the referenced artifacts can actually be fetched)? The `ArtifactStore` trait now provides the `contains()` operation needed for DA checks; the question is when and how the protocol enforces availability.
- How should cross-domain effects work when domains share participants but have independent fork competition?
- How do escrow economics behave under sustained fork competition, where multiple competing branches each hold escrows?

These are not reasons to delay implementation.
They are reasons to build in a way that allows change.

---

## Near-Term Development Sequence

### Completed

**Step 1** — Implemented Rust protocol types and genesis/domain activation logic.

**Step 2** — Implemented the single-domain research loop: block, attestation, challenge, fork, frontier settlement, escrow management.

### In progress

**Step 3** — Generalizing to multi-domain support and domain-scoped accounting. Domain independence works; cross-domain effects not yet implemented.

### Forward sequence

1. Complete Phase 0 remaining items: challenge economics, staged rewards, storage-model references, successor-track creation, cross-domain effects.
2. Complete local single-node runtime (Phase 1): transaction flow, state queries, CLI commands (persistence already implemented).
3. Complete Python runner integration (Phase 2): connect proposer, validator, challenger runners to local runtime (evidence bundling, QMD genesis packaging, and surface enforcement already implemented).
4. Implement frontier materialization and artifact resolution (Phase 3).
5. Run sustained adversarial simulations (Phase 4, can overlap with Phase 2/3).

---

## Closing Principle

The implementation plan should remain subordinate to the core mission.

AutoResearch Chain is not trying to become a generic crypto platform with an AI wrapper.

It is trying to become the world-leading open decentralized market for AI research and, later, decentralized AI training.

The implementation should therefore optimize for:
- protocol correctness
- mechanism integrity
- adaptability
- and real useful-work integration

Everything else is secondary.
