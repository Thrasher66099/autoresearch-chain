# Roadmap

## Purpose

This document separates the project into clear stages so readers understand:

- what exists conceptually now,
- what is specified at the protocol level,
- what is partially defined,
- and what remains future work.

This separation is critical because AutoResearch Chain is intentionally staged.

---

## Stage 1 — Decentralized Research Discovery

### Summary

Stage 1 is the current conceptual and protocol focus.

Participants use GPUs plus autonomous research agents to mine validated improvements to AI training recipes.

### Core loop

An agent:
- pulls the current canonical frontier state for a domain,
- modifies the training recipe,
- runs a bounded experiment,
- measures the result,
- submits the change if it improves the target metric.

The protocol then:
- samples validators,
- replays the result,
- allows challenges,
- tracks forks,
- updates the frontier,
- and settles rewards.

### Stage 1 scope

Stage 1 includes:
- autonomous recipe search
- proof-of-useful research work
- replay-based validation
- challenge games
- fork-native competition
- domain-local and cross-domain work
- canonical frontier states
- materialized code snapshots
- Research Track Standards and permissionless genesis
- track initialization and activation lifecycle
- search surface and frozen surface separation
- metric and dataset integrity policies
- domain-scoped validator pools and reward context

### Stage 1 examples

- `nanochat-base`
- `optimizer-subspace`
- `scheduler-subspace`
- `data-pipeline-efficiency`

### Status

This is the most developed part of the project and the best-defined entry point.

---

## Stage 2 — Scale Validation

### Summary

Stage 2 tests whether Stage 1 improvements survive at larger scales or longer budgets.

A local win is not automatically a meaningful win.

Scale validation exists to distinguish:
- genuine transferable gains
from
- local benchmark artifacts

### Core function

Scale validators:
- take high-confidence Stage 1 improvements,
- rerun them under larger compute budgets or stronger model settings,
- determine whether the improvement transfers.

### Stage 2 scope

Stage 2 includes:
- transfer testing
- larger-budget replay
- stronger confidence settlement
- deferred escrow release
- downstream reward multipliers

### Why Stage 2 matters

Without Stage 2, the protocol risks overpaying:
- local noise,
- benchmark hacks,
- brittle short-horizon tricks.

### Status

Conceptually clear and economically important, but less fully specified than Stage 1.

---

## Stage 3 — Decentralized Long-Horizon Shared Training

### Summary

Stage 3 is the long-term execution layer.

This is where users do not just mine improvements to training recipes. They also contribute GPU-hours directly to a shared long-running model training process.

### Core function

A winning or sufficiently battle-tested recipe from earlier stages becomes the basis for:
- decentralized training swarms,
- persistent checkpoint progression,
- shared model updates,
- continuous useful training work.

### Stage 3 scope

Stage 3 would require:
- long-horizon participant coordination
- checkpoint lifecycle management
- adversarially robust worker contribution accounting
- training sync protocols
- contribution proofs
- reward for sustained useful participation
- recipe upgrade logic during ongoing training

### Why Stage 3 is separate

Stage 3 is meaningfully different from Stage 1:
- Stage 1 mines improvements to the recipe
- Stage 3 mines direct participation in the training run itself

These should not be blurred together.

### Status

Future work. Not fully specified.

---

## Cross-Stage Dependency Structure

The stages are related, but not identical.

### Stage 1 feeds Stage 2
Stage 1 generates candidate improvements.  
Stage 2 tests whether they survive scale.

### Stage 2 filters Stage 3 inputs
Stage 2 helps decide which improvements are good enough to influence a persistent shared training run.

### Stage 3 is not required for Stage 1 to matter
Even without Stage 3, Stage 1 already creates a useful protocol:
a decentralized market for mining validated improvements to AI training recipes.

---

## Current Project Identity

The cleanest accurate description of the current project is:

> Stage 1-focused fully decentralized Proof-of-Useful-Work for mining validated improvements to AI training recipes.

That is the base reality today.

---

## Near-Term Goals

The strongest near-term goals are:

1. clarify and harden the Stage 1 protocol
2. improve docs and terminology
3. define domain and canonical frontier behavior clearly
4. harden genesis block mechanics, activation conditions, and failed-genesis lifecycle
5. specify scale validation incentives
6. make user workflows legible
7. begin mapping the eventual Stage 3 design space
8. execute the initial implementation plan: Rust protocol types, local simulator, Python runner integration (see [Implementation Plan](implementation-plan.md))

---

## Long-Term Vision

The long-term vision is broader:

- a decentralized market for research discovery
- a decentralized market for transfer validation
- a decentralized market for long-horizon shared AI training
- a living public substrate for intelligence production

But the project should communicate that this vision is layered and staged, not already complete.

---

## Why This Roadmap Matters

Without a clear roadmap, readers will make one of two mistakes:

- assume the project is “just” Stage 1 and miss the larger ambition
- assume the project already includes full Stage 3 swarm training and dismiss it as overclaimed

A good roadmap prevents both errors.