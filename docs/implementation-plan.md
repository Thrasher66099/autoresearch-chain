<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Implementation Plan

## Purpose

This document defines the initial technical implementation plan for AutoResearch Chain.

It is intentionally practical rather than aspirational.

The goal is not to freeze the architecture permanently before implementation begins.
The goal is to define:

- the chosen architectural direction,
- the core subsystem split,
- the recommended build order,
- the first implementation milestones,
- and the major engineering questions that should stay active during development.

This is a living implementation plan for a protocol that is still evolving.

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

The first major question is:

> Does the game behave correctly when represented as executable state transitions?

That is the right place to start.

---

## Build Phases

## Phase 0 — Protocol Modeling

### Goal

Turn the protocol spec into executable Rust types and transitions.

### Deliverables

The protocol core should define at minimum:

- `ProblemDomain`
- `DomainSpec`
- `ResearchTrackStandard`
- `GenesisBlock`
- `TrackInitialization`
- `TrackTree`
- `EpochSpec`
- `Block`
- `ForkFamily`
- `ValidationAttestation`
- `ChallengeRecord`
- `AttributionClaim`
- `CanonicalFrontierState`
- `MaterializedState`
- `CodebaseStateRef`
- `EscrowRecord`
- `MetricIntegrityPolicy`
- `DatasetIntegrityPolicy`

And core logic for:

- domain creation
- track activation
- seed-score verification state
- block submission
- validator assignment
- attestation aggregation
- challenge opening and resolution
- fork activation and dominance
- frontier settlement
- reward staging
- slashing outcomes
- cross-domain integration
- successor-track creation

### Success condition

A local simulator can model:

- genesis proposal
- track activation
- block submission
- validation outcomes
- challenge outcomes
- fork competition
- frontier updates
- reward accounting
- domain-local and cross-domain behavior

without any real networking.

This phase should be heavily test-driven.

---

## Phase 1 — Local Single-Node Runtime

### Goal

Wrap the protocol core in a minimal executable local runtime.

### Deliverables

- single-node chain state persistence
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

### Goal

Connect real useful-work loops to the protocol.

### Deliverables

- proposer runner
- validator runner
- challenger runner
- evidence bundle packager
- domain experiment wrappers
- canonical frontier pull helper
- autoresearch-style adapter

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

## Recommended Repository Architecture

The implementation should be organized around clear subsystem boundaries.

### Rust

Suggested crate layout:

- `crates/protocol-types/`
  - core structs, enums, IDs, hashes, references

- `crates/protocol-rules/`
  - deterministic state transition logic

- `crates/domain-engine/`
  - domains, standards, genesis activation, track trees

- `crates/fork-engine/`
  - fork families, dominance, frontier selection

- `crates/challenge-engine/`
  - challenge types, resolution rules, remedies

- `crates/reward-engine/`
  - staged rewards, escrows, slashing, domain-local accounting

- `crates/storage-model/`
  - references, artifact metadata, materialized state references

- `crates/simulator/`
  - local protocol simulator and scenario engine

- `crates/node/`
  - minimal local runtime / future node executable

- `crates/cli/`
  - command-line interface

### Python

Suggested layout:

- `python/arc_runner/`
  - shared protocol client logic

- `python/arc_runner/proposer/`
  - proposer execution runner

- `python/arc_runner/validator/`
  - validator replay runner

- `python/arc_runner/challenger/`
  - challenger replay/audit runner

- `python/arc_runner/autoresearch_adapter/`
  - integration with autoresearch-style loops

- `python/arc_runner/domains/`
  - domain-specific experiment wrappers

- `python/arc_runner/evidence/`
  - evidence bundle creation and validation

- `python/arc_runner/materialize/`
  - materialized code state generation and packaging

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

### 1. Domain and Genesis Activation

The chain must not remain implicitly tied to one domain.
Genesis and track activation logic should be among the first things implemented.

### 2. Base Block / Validation / Challenge Loop

The core game loop must work in one domain before broadening.

### 3. Multi-Domain Support

The protocol should support multiple domains early enough that it does not ossify around a single benchmark.

### 4. Canonical Frontier Materialization

Users must be able to pull the current best assembled state.
This is essential to the protocol's practical usability.

### 5. Python Runner Integration

The chain becomes real only when it can actually receive useful work from autonomous agent loops and replay workers.

---

## Immediate Technical Decisions to Lock

Not every design choice needs to be final now.
But several should be decided relatively early.

### Canonical Serialization

The protocol needs canonical formats for:

- configs
- manifests
- evidence bundles
- research target declarations
- metric reports
- track initialization packages

Without canonical serialization, hashing and replay become fragile.

### Reference and Hash Model

The protocol needs a clean model for how it references:

- states
- diffs
- evidence bundles
- materialized snapshots
- datasets
- evaluation harnesses

This should be explicit and stable early.

### Reproducibility Tolerance Model

The protocol must define:

- score tolerance
- seed score reproduction tolerance
- attestation aggregation thresholds
- inconclusive conditions

### Materialization Policy

The protocol must define when a frontier or chain of diffs becomes a materialized state.

Potential triggers include:

- dominance
- depth threshold
- scheduled checkpoint
- domain policy rule

### Domain Activation Lifecycle

The protocol now explicitly includes genesis and track creation.
Those state transitions should be implemented early as first-class logic, not bolted on later.

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

**Does the research-market protocol behave coherently when executed?**

---

## First Major Deliverable

The first major engineering deliverable should be:

**A local protocol simulator with real domain/genesis/block/validation/challenge/fork/frontier logic and Python runner hooks**

This is the inflection point where the project becomes more than a white paper.

If this works, the team will have:

- executable protocol logic
- adversarial testability
- a place to discover spec contradictions
- a base for real runner integration
- a foundation for a future networked chain

---

## Main Open Engineering Questions

These questions should stay active during implementation:

- how strict should genesis activation thresholds be?
- how should artifact references be structured?
- how often should frontier states be materialized?
- how should challenge escalation be encoded in v0?
- how should domain-local reward accounting be represented internally?
- how should successor tracks and metric migration work in code?
- how formulaic should attribution be in early versions?
- how much replay metadata must be mandatory at protocol level versus runner level?

These are not reasons to delay implementation.
They are reasons to build in a way that allows change.

---

## Near-Term Development Sequence

The most practical short-term sequence is:

### Step 1
Implement Rust protocol types and genesis/domain activation logic.

### Step 2
Implement the single-domain research loop:
- block
- attestation
- challenge
- fork
- frontier settlement

### Step 3
Generalize to multi-domain support and domain-scoped accounting.

### Step 4
Implement canonical frontier materialization and pull logic.

### Step 5
Integrate Python proposer and validator runners.

This is the order most likely to produce useful learning quickly.

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
