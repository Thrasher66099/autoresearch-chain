<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Protocol Specification v0.2

This document describes the technical structure of the AutoResearch Chain protocol as currently specified. It covers the system model, participant roles, block structure, evidence requirements, validation, fork mechanics, challenge system, and reward distribution.

This is a design-stage specification. No reference implementation exists yet.

---

## Table of Contents

- [System Model](#system-model)
- [Core Roles](#core-roles)
- [The Canonical Stage 1 Loop](#the-canonical-stage-1-loop)
- [Blocks](#blocks)
- [Evidence Bundles](#evidence-bundles)
- [Validation](#validation)
- [Forks](#forks)
- [Cross-Fork Porting](#cross-fork-porting)
- [Challenges](#challenges)
- [Reward Structure](#reward-structure)
- [Ancestry Farming Prevention](#ancestry-farming-prevention)

---

## System Model

AutoResearch Chain has three layers:

### 1. On-Chain State Layer

Tracks:

- epochs,
- blocks,
- fork families,
- validator registrations,
- validation attestations,
- challenge records,
- attribution claims,
- escrow,
- slashing,
- governance parameters.

### 2. Off-Chain Execution Layer

All actual training and replay happen off-chain.

The chain does not run the training itself. It adjudicates **claims about training** through evidence, replay, and bonded challenge.

### 3. Data Availability Layer

All reward-relevant evidence must be publicly retrievable.

If a claim cannot be fetched, replayed, or challenged, it cannot be trusted.

---

## Core Roles

### Proposers

Submit candidate recipe improvements as blocks. Must include a complete evidence bundle and post a bond.

### Validators

Replay parent/child transitions and attest whether the claimed improvement reproduces. Validators are sampled deterministically from a bonded pool.

### Challengers

Dispute false blocks, false attestations, false attribution, or false dominance. Challenges require a bond and trigger replay or evidence review under protocol rules.

### Scale Validators

Test whether local improvements transfer at larger model scales or longer training budgets. Operate in Stage 2.

### Governors

Tune protocol parameters without deciding scientific truth. Cannot override validation outcomes or block challenges.

All roles are permissionless and bonded.

---

## The Canonical Stage 1 Loop

The canonical Stage 1 loop is agent-driven and closely mirrors autonomous research tools such as `karpathy/autoresearch`.

A participant runs:

1. A local training environment
2. An AI agent (e.g., LLM-based code modifier)
3. A fixed benchmark/evaluation setup
4. A short training budget
5. A protocol client for submitting evidence and claims

The agent repeatedly:

1. Reads the current recipe
2. Modifies `train.py` or the equivalent recipe surface
3. Runs a short training experiment
4. Measures the result
5. Compares it to the parent recipe
6. Submits the diff if it improves the metric
7. Otherwise discards it and continues searching

This is the base "mining" primitive.

---

## Blocks

A block is a claim that a child training recipe improves on a parent training recipe.

### Block Contents

| Field | Description |
|-------|-------------|
| Parent reference | Hash of the parent block/recipe state |
| Child state reference | Hash of the proposed new recipe state |
| Diff reference | Hash of the code diff from parent to child |
| Claimed metric delta | The improvement claimed (e.g., delta in `val_bpb`) |
| Evidence bundle hash | Hash of the full evidence bundle |
| Proposer identity | Public key or address of the proposer |
| Fee and bond | Submission fee and slashable bond |
| Epoch reference | Protocol epoch at time of submission |

### Block Lifecycle

A block does not become final just because it is submitted. It must survive:

1. Validation (replay by bonded validators)
2. Challenge (adversarial dispute window)
3. Fork competition (competing branches targeting the same parent)
4. Confidence settlement (final reward release)

---

## Evidence Bundles

Every block must include a public evidence bundle sufficient for replay.

### Required Contents

- Code diff (parent to child)
- Fully resolved configuration
- Environment manifest (dependencies, versions, hardware spec)
- Dataset references (hashes or canonical identifiers)
- Evaluation procedure specification
- Training budget declaration (steps, tokens, wall-clock limit)
- Seed or seed schedule (if deterministic replay is required)
- Canonical training logs
- Metric outputs
- Output artifact hashes
- Machine-readable run summary

Without a complete evidence bundle, the protocol collapses into unverifiable claims. Evidence availability is enforced at the protocol level.

---

## Validation

When a block is submitted, a bonded validator set is sampled deterministically from the eligible pool.

### Validation Procedure

1. Retrieve the parent and child state
2. Reconstruct the environment from the evidence bundle
3. Replay both parent and child training runs
4. Compute the target metric for each
5. Submit a signed attestation

### Attestation Votes

| Vote | Meaning |
|------|---------|
| `PASS` | Claimed improvement reproduces within tolerance |
| `FAIL` | Claimed improvement does not reproduce |
| `INCONCLUSIVE` | Replay produced ambiguous results |
| `FRAUD_SUSPECTED` | Evidence of fabrication or manipulation detected |

A block is provisionally accepted only if threshold rules are met (e.g., supermajority of `PASS` votes from the sampled validator set).

---

## Forks

Forks are a first-class protocol feature.

If two or more valid improvements target the same parent, the protocol allows them all to exist simultaneously as competing branches.

### Why Forks Matter

- Real research is parallel.
- Premature convergence is harmful.
- Different branches may find different useful ideas.
- Later branches may merge or import ideas from each other.

### Fork Economics

During unresolved fork competition, immediate rewards are reduced. This creates economic pressure to converge while still allowing exploration.

Forks compete until evidence-based convergence determines a dominant branch.

---

## Cross-Fork Porting

Cross-fork idea porting is allowed by design.

If one branch finds a useful technique and another branch ports it faster or integrates it better, that is a feature. The protocol rewards:

- Discovery of useful ideas (origin credit)
- Successful integration of useful ideas (integration credit)
- Advancement of the best frontier

This turns forks into a competitive synthesis process rather than a winner-take-all tournament.

---

## Challenges

Any economically meaningful claim must be challengeable.

### Challengeable Objects

- Blocks (proposed recipe improvements)
- Validator attestations
- Attribution claims
- Fork dominance declarations
- Scale-stage results

### Challenge Mechanics

Challenges require a bond and trigger replay or evidence review under protocol rules.

If a challenge **succeeds**, the protocol may:

- reject a block,
- slash stake,
- amend attribution,
- reopen fork settlement,
- redirect escrow.

If a challenge **fails**, the challenger loses their bond.

This is the core truth-seeking mechanism. The system relies on the economic incentive for challengers to identify and dispute false claims.

---

## Reward Structure

Rewards are staged to pay for survival through falsification, not merely for making claims.

### Reward Stages

| Stage | Trigger | Purpose |
|-------|---------|---------|
| Provisional reward | Initial validation passes | Immediate incentive for proposers |
| Survival reward | Challenge window closes without successful challenge | Reward for robust claims |
| Integration reward | Idea imported into a dominant branch | Reward for useful contributions across forks |
| Frontier reward | Block advances the dominant lineage | Reward for pushing the state of the art |
| Transfer reward | Improvement survives larger-scale validation (Stage 2) | Reward for ideas that generalize |

This structure makes local wins economically meaningful without overpaying noise or fraud.

---

## Ancestry Farming Prevention

The protocol must prevent participants from farming reward merely by inserting themselves into lineage.

### Attack Patterns

- Trivial intermediate blocks (no-op or near-no-op changes to claim ancestry)
- Genealogy rent extraction (extracting downstream royalties from historical position)
- Synthetic fork positioning (creating artificial fork points for reward capture)
- Passive royalty capture (claiming reward without meaningful contribution)

### Mitigations

- Ancestry alone does not entitle a block to reward.
- Deep ancestor claims must decay over time.
- Trivial blocks receive little or no downstream share.
- Reward follows causal contribution, not just position in history.

---

[Back to docs index](README.md) | [White Paper](whitepaper.md) | [Project Scope](project-scope.md)
