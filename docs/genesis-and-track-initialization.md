<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Genesis Blocks and Track Initialization

## Purpose

This document provides a conceptual overview of how new research tracks are created in AutoResearch Chain.

The full technical specification is in the [Protocol Specification](protocol-v0.2.md#research-track-standards-genesis-blocks-and-domain-initialization). This document is a companion for readers who want to understand the concepts before reading the formal definitions.

---

## The Problem

In a local tool like `autoresearch`, one human chooses the seed model, the dataset, the metric, the evaluation harness, the time budget, and the search surface.

In a decentralized market, those choices must become explicit, reproducible, challengeable, economically filtered, and protocol-legible.

Without a formalized genesis mechanism, the protocol would implicitly depend on a socially pre-agreed single-player setup. That is incompatible with permissionless multi-domain operation.

---

## How It Works

### Research Track Standards

A `ResearchTrackStandard` (RTS) is an interface specification that defines what a research track must look like to participate in the protocol.

The first standard, `RTS-1`, covers the near-term case: single-metric optimization, fixed-budget experiments, bounded single-node or single-GPU replay, and autonomous research-agent loops. This covers the majority of Stage 1 use cases.

Future standards (`RTS-2`, `RTS-3`, etc.) can support multi-metric tracks, efficiency-normalized evaluation, distributed replay, and longer-horizon experiments — without rewriting the core chain logic.

### Genesis Blocks

A `GenesisBlock` is the root block of a new research track. It is not a claim that a recipe improved — it is a claim that a new research arena is well-defined enough to become a protocol-recognized market.

A genesis block declares:

- the research target and domain intent
- the seed recipe and its baseline score
- the canonical dataset and its splits
- the evaluation harness and metric
- the search surface (what agents may modify) and frozen surface (what must stay fixed)
- hardware class and time budget for replay
- the proposer's seed bond

### Track Initialization

Track creation is permissionless but economically filtered:

1. A participant posts a seed bond and submits a complete RTS-conformant genesis proposal.
2. The protocol checks formal conformance.
3. Validators reproduce the seed recipe and score.
4. If all activation conditions are met — conformance, artifact availability, seed reproduction, minimum validator participation, minimum bonded threshold, no upheld challenge — the track activates.
5. If conditions are not met, the proposal fails and the seed bond is returned, partially slashed, or fully slashed depending on the failure reason.

This avoids centralized gatekeeping. The protocol does not decide which research problems are interesting. The market does.

---

## Key Concepts

### The Chain as a Forest

Each active research track forms a `TrackTree`: a domain-scoped descendant tree rooted at a single genesis block.

The chain is not a single tree with forks. It is a **forest of independent domain-rooted trees**. Each tree has its own fork families, validator pools, reward context, canonical frontier, and challenge surface.

### Search Surface vs. Frozen Surface

Every genesis block separates:

- **search surface**: what participants may modify (e.g., training logic, hyperparameters)
- **frozen surface**: what must remain fixed (e.g., evaluation harness, dataset preparation)

Without this separation, agents could optimize the metric by modifying the metric-producing machinery itself.

### Metric Integrity and Migration

A track's metric and evaluation harness are immutable for the life of the track. If a metric later proves flawed, the protocol does not silently mutate the active track. Instead, a **successor track** is created that references the prior track, declares the new metric, and preserves historical integrity.

### Dataset Integrity

Each track declares a content-addressed canonical dataset, its splits, availability requirements, and license status. If the dataset becomes unretrievable, the track becomes unvalidatable. Dataset availability is part of the protocol's reproducibility core.

### Domain-Scoped Validation and Rewards

Validator eligibility is scoped to the track (filtered by hardware, dataset availability, environment support). Reward accounting is also track-scoped, preventing dominant tracks from starving smaller but valuable research domains.

---

## Relationship to Existing Protocol Objects

The genesis layer integrates with the existing multi-domain architecture:

| Object | Role |
|--------|------|
| `ResearchTrackStandard` | Defines the required interface class |
| `GenesisBlock` | Instantiates a new track under that standard |
| `ProblemDomain` | The active research arena created by the accepted genesis |
| `DomainSpec` | The active structural rules of that domain |
| `TrackTree` | The descendant tree rooted at the genesis |

---

## What This Means for Users

A participant who wants to create a new research arena does not need permission from governance or any central authority. They need:

1. A well-defined research problem
2. A seed recipe with a reproducible baseline score
3. A canonical dataset
4. An evaluation harness
5. Enough economic commitment to post a seed bond and attract validators

If the proposal meets the standard and survives validation, it becomes an active track that any participant can mine, validate, or challenge.

---

## Implementation Maturity

Research track standards and genesis block mechanics are specified at the protocol level. They are not yet implemented in any reference client. This document and the protocol specification describe the intended design, not currently running software.

---

[Back to docs index](README.md) | [Protocol Specification](protocol-v0.2.md) | [Terminology](terminology.md)
