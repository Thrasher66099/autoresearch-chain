<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Project Scope

This document separates what the AutoResearch Chain protocol currently specifies from what is planned but not yet formally defined.

---

## Current Protocol Scope

As currently specified, the protocol implements a decentralized **research-discovery layer**.

### What Exists

The protocol specifies:

- **Block submission**: proposers submit candidate recipe improvements with evidence bundles
- **Bonded validation**: validators replay parent/child transitions and attest results
- **Adversarial challenge**: any claim can be disputed by bonded challengers
- **Fork-native search**: multiple competing branches can coexist from the same parent
- **Staged rewards**: rewards are released incrementally based on survival through falsification
- **Scale-validation hooks**: Stage 2 validators can test whether improvements transfer to larger models
- **Agent-driven Stage 1**: the mining primitive is designed around autonomous research agents running short-horizon GPU experiments
- **Multi-domain research**: the protocol supports multiple concurrent problem domains, each with its own codebase, evaluation logic, fork competition space, and reward context
- **Canonical frontier states**: each domain exposes a pullable assembled codebase representing the current best state, so participants can retrieve the working codebase for any domain and begin mining improvements from it
- **Hierarchical domains and cross-domain integration**: specialized sub-problem work can occur in narrow domains without polluting broader frontiers, and successful results can be integrated upstream through explicit cross-domain blocks
- **Research Track Standards and genesis blocks**: new research tracks can be instantiated permissionlessly through standardized genesis blocks conforming to a Research Track Standard (RTS), with economic filtering via seed bonds and validator reproduction
- **Track initialization and activation lifecycle**: genesis proposals pass through conformance checking, seed reproduction, and activation thresholds before becoming active tracks; failed proposals have defined bond outcomes

### What This Implements

The current protocol implements:

**Proof of Useful Research Work** — specifically, **Proof of Useful Training-Optimization Work**.

GPU cycles are spent discovering, validating, and falsifying improvements to AI training recipes. This is useful work because the output is measurable progress in training methodology.

### Current Identity

> AutoResearch Chain is a fully decentralized Proof-of-Useful-Work protocol for mining validated improvements to AI training recipes.

That statement accurately describes the current protocol.

---

## Staging Model

### Stage 1 — Recipe Discovery (Current Focus)

Consumer GPUs run independent short-horizon experiments on small models.

An AI agent (modeled on `autoresearch`-style loops) modifies the training recipe, runs the experiment, measures the delta, and submits improvements as blocks.

**Output:**
- Better training code
- Better training recipes
- A forked history of validated improvements

### Stage 2 — Scale Validation (Partially Specified)

Higher-end hardware tests whether Stage 1 improvements transfer to larger models and longer training budgets.

**Output:**
- Scale-validation signals
- Dead-end detection
- Transfer confidence

The protocol includes hooks for Stage 2 validation, but the full economic and operational details are not yet complete.

### Stage 3 — Decentralized Training (Future Work)

Once a recipe is sufficiently battle-tested, contributors form a decentralized training swarm and use the winning recipe to train a shared model over long horizons.

**Output:**
- Trained open model weights
- Decentralized compute contribution
- Sustained useful training work

Stage 3 is **not yet formally specified**. It is compatible with the protocol design but requires separate rigorous specification. See [Future: Stage 3 Training](future-stage-3-training.md).

---

## Explicitly In Scope

- Agent-driven Stage 1 recipe search
- `autoresearch`-style local research loops as the mining primitive
- Block submission with evidence bundles
- Replay-based validation
- Fork-native competition
- Challenge-based falsification
- Larger-scale validation hooks (Stage 2)
- Multi-domain recipe-mining across concurrent problem domains
- Sub-problem mining in hierarchical child domains (e.g., optimizer, scheduler, data pipeline)
- Cross-domain integration blocks for porting results between domains
- Canonical frontier state exposure per domain (pullable assembled codebase)
- Domain-local reward separation and accounting
- Research Track Standards (RTS-1 and future RTS classes)
- Permissionless genesis block proposals with economic filtering
- Track initialization, activation conditions, and failed genesis lifecycle
- Search surface and frozen surface separation at genesis
- Metric and dataset integrity policies
- Domain-scoped validator pools and reward context
- Metric migration via successor tracks

## Not Yet Fully Specified

- Full Stage 3 swarm training protocol
- Full gradient attestation system
- Full long-horizon compute contribution accounting
- Token economics (beyond staged reward structure)
- Governance implementation details
- Reference implementation
- Multi-domain reference client tooling (the protocol specifies multi-domain objects and guarantees; client-side tooling for domain discovery and state retrieval is not yet implemented)
- Genesis block submission tooling (the protocol specifies genesis mechanics; no reference tooling for proposing or activating tracks exists yet)

---

## What the Protocol Does Not Claim

- It does not claim to already solve decentralized frontier model training.
- It does not claim that Stage 3 is fully designed.
- It does not claim that the research-discovery layer alone is sufficient for training production models.
- It does not claim to have a reference implementation.

The current scope is research discovery and validation. That scope is meaningful on its own and is the foundation for future layers.

---

[Back to docs index](README.md) | [Protocol Specification](protocol-v0.2.md) | [Future: Stage 3 Training](future-stage-3-training.md)
