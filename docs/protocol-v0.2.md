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
- [Multi-Domain Research and Canonical Frontier States](#multi-domain-research-and-canonical-frontier-states)
  - [ProblemDomain](#problemdomain)
  - [DomainSpec](#domainspec)
  - [Hierarchical Domains](#hierarchical-domains)
  - [Domain-Local and Cross-Domain Work](#domain-local-and-cross-domain-work)
  - [Cross-Domain Integration Blocks](#cross-domain-integration-blocks)
  - [Domain-Specific Fork Competition](#domain-specific-fork-competition)
  - [Canonical Frontier State](#canonical-frontier-state)
  - [MaterializedState](#materializedstate)
  - [CodebaseStateRef](#codebasestateref)
  - [User and Agent Workflow Guarantee](#user-and-agent-workflow-guarantee)
  - [Domain Reward Separation](#domain-reward-separation)
  - [Upstream Integration Rule](#upstream-integration-rule)
  - [Required Protocol Guarantees](#required-protocol-guarantees)

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
- governance parameters,
- problem domain registrations and domain specs,
- canonical frontier state references,
- materialized state records.

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
| Domain reference | The `ProblemDomain` this block targets |
| Parent reference | Hash of the parent block/recipe state |
| Child state reference | Hash of the proposed new recipe state |
| Diff reference | Hash of the code diff from parent to child |
| Claimed metric delta | The improvement claimed (e.g., delta in the domain's primary metric) |
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

## Multi-Domain Research and Canonical Frontier States

### Purpose

AutoResearch Chain is not limited to a single benchmark, a single model, or a single research target.

The protocol supports many concurrent research arenas, including:

- full-model recipe optimization
- narrow sub-technique optimization
- tertiary infrastructure problems
- component-level performance improvements
- transfer-focused specialization domains
- future domain-specific model families

To support this, the protocol treats research targets as first-class objects.

The protocol introduces the following objects:

- `ProblemDomain`
- `DomainSpec`
- `CanonicalFrontierState`
- `MaterializedState`
- `CodebaseStateRef`

These objects allow the chain to support multiple parallel research markets while preserving a canonical pullable codebase for each domain.

**Status note:** Multi-domain support is specified at the protocol level. It is not yet implemented in any reference client. The object definitions and guarantees below are part of the protocol specification, not claims about existing software.

---

### ProblemDomain

A `ProblemDomain` is a protocol-defined research arena.

A domain defines what problem participants are trying to improve.

Examples include:

- `nanochat-base`
- `small-language-model-training`
- `optimizer-subspace`
- `scheduler-subspace`
- `data-pipeline-efficiency`
- `memory-reduction-techniques`
- `distributed-sync-strategy`
- `checkpointing-subsystems`
- `multimodal-pretraining`

A domain may represent:

- a full end-to-end training recipe,
- a component of a larger training system,
- a specialized sub-problem,
- a tertiary infrastructure problem,
- or a future training market for a specific model family.

A domain is not merely a label. It is a formal protocol object with its own:

- codebase root
- benchmark/evaluation logic
- epoch constraints
- fork competition space
- canonical frontier
- reward context
- integration rules

---

### DomainSpec

A `DomainSpec` defines the structural rules of a `ProblemDomain`.

Each `ProblemDomain` must have exactly one active `DomainSpec` per active protocol interval, unless protocol rules explicitly allow staged migration.

A `DomainSpec` includes at minimum:

| Field | Description |
|-------|-------------|
| `domain_id` | Unique identifier for the domain |
| `domain_name` | Human-readable domain name |
| `parent_domain_id` | Parent domain reference, if any |
| `domain_type` | One of: `root`, `model`, `subsystem`, `technique`, `infrastructure`, `integration`, `experimental` |
| `base_codebase_ref` | Reference to the domain's base codebase |
| `base_state_ref` | Reference to the domain's base state |
| `evaluation_targets` | Metrics and benchmarks used for evaluation |
| `primary_metric` | The primary metric for judging improvements |
| `secondary_metrics` | Additional tracked metrics |
| `allowed_modification_surface` | What parts of the codebase may be modified |
| `artifact_schema` | Schema for required submission artifacts |
| `epoch_policy_ref` | Reference to the domain's epoch policy |
| `hardware_tier_policy` | Hardware requirements and tier rules |
| `reward_budget_policy` | Reward allocation policy for the domain |
| `fork_policy` | Fork competition rules for the domain |
| `integration_policy` | Rules for cross-domain integration |
| `canonicalization_policy` | Rules for frontier state canonicalization |
| `materialization_policy` | Rules for when and how states are materialized |

#### Domain Types

A domain may be typed as:

- `root` — top-level domain with no parent
- `model` — a full model training target
- `subsystem` — a component of a larger training system
- `technique` — a narrow optimization technique
- `infrastructure` — supporting infrastructure (e.g., data pipelines, checkpointing)
- `integration` — a domain focused on integrating results from other domains
- `experimental` — an exploratory domain with relaxed validation rules

These types are descriptive and may influence default policy, but they do not override explicit protocol rules.

---

### Hierarchical Domains

Domains may be hierarchical. A domain may have a parent domain and may also have child domains.

This allows the protocol to support both:

- broad end-to-end optimization
- narrow specialized optimization

For example:

```
language-model-training
├── nanochat-base
├── optimizer-subspace
├── scheduler-subspace
└── data-pipeline-subspace
```

Or:

```
distributed-training
├── gradient-compression
├── sync-strategy
└── checkpoint-recovery
```

Hierarchy exists so that specialized work can happen in narrow domains without polluting the main frontier, while still allowing successful results to be integrated upward through explicit integration blocks.

---

### Domain-Local and Cross-Domain Work

The protocol distinguishes between two categories of improvement:

#### Domain-Local Improvements

A block that improves performance only within its own domain under that domain's evaluation logic.

Example: a dataloader optimization improves `data-pipeline-efficiency` but has not yet been tested in `nanochat-base`.

#### Cross-Domain Integration Improvements

A block that imports or ports an improvement from one domain into another domain and validates it under the destination domain's rules.

Example: an optimization discovered in `optimizer-subspace` is later integrated into `nanochat-base` and validated there.

This distinction is essential. Without it, local wins in specialized domains cannot be cleanly separated from broader end-to-end gains.

---

### Cross-Domain Integration Blocks

The protocol supports explicit `CrossDomainIntegrationBlock` behavior.

A cross-domain integration block is a block that:

- references a source domain
- references one or more source attributed units, blocks, or frontier states
- ports or integrates the relevant improvement into a destination domain
- validates the improvement under the destination domain's evaluation rules

A successful cross-domain integration block may receive:

- proposer reward
- integration attribution
- frontier attribution in the destination domain
- downstream transfer reward if applicable

The source domain retains:

- origin attribution
- domain-local reward
- integration-derived downstream reward where policy allows

This preserves the principle that discovering a useful idea and successfully integrating it into a broader frontier are both valuable acts.

---

### Domain-Specific Fork Competition

Forks are scoped per domain.

Each `ProblemDomain` has its own:

- active fork families
- frontier blocks
- dominance logic
- canonical frontier state
- reward competition surface

Forks in one domain do not directly interfere with fork accounting in unrelated domains. This prevents unrelated branch growth elsewhere in the protocol from distorting local research incentives.

---

### Canonical Frontier State

Each active `ProblemDomain` exposes a `CanonicalFrontierState`.

This is the current protocol-recognized best assembled state of the domain. The `CanonicalFrontierState` is the object that participants should pull when beginning new work in that domain.

It must include or resolve to:

| Component | Description |
|-----------|-------------|
| Current dominant frontier block | The block at the tip of the dominant fork |
| Full assembled source tree | Complete working codebase |
| Resolved configuration set | All configuration parameters |
| Resolved dependency manifest | Pinned dependencies and versions |
| Environment manifest | Hardware and software environment spec |
| Evaluation manifest | Benchmark and metric definitions |
| Epoch/domain policy references | Current governing policy |
| Ancestry and attribution metadata | Provenance chain as required |
| Content-addressed snapshot reference | Immutable reference to this state |

The `CanonicalFrontierState` exists so that the chain is not merely a ledger of diffs, but a usable research substrate. Participants must always be able to retrieve the current working codebase for a domain.

---

### MaterializedState

A `MaterializedState` is a full assembled working snapshot of a domain's codebase and execution context.

The protocol distinguishes between:

- `BlockDiff`: an incremental proposed change
- `MaterializedState`: a fully resolved assembled state

This distinction is necessary because long chains of diffs become operationally fragile over time.

A `MaterializedState` includes:

| Field | Description |
|-------|-------------|
| `materialized_state_id` | Unique identifier |
| `domain_id` | Domain this state belongs to |
| `root_tree_hash` | Content-addressed hash of the full source tree |
| `resolved_dependency_manifest_hash` | Hash of resolved dependencies |
| `resolved_config_hash` | Hash of resolved configuration |
| `environment_manifest_hash` | Hash of environment specification |
| `evaluation_manifest_hash` | Hash of evaluation specification |
| `materialized_from_block_id` | Block this state was materialized from |
| `timestamp` | Time of materialization |

The protocol permits or requires materialization:

- when a fork becomes dominant
- at scheduled checkpoints
- when a diff chain exceeds policy thresholds
- when a domain policy requires snapshot compaction

The output of materialization must be content-addressed and publicly fetchable.

---

### CodebaseStateRef

A `CodebaseStateRef` is a protocol-resolvable reference to a full assembled codebase state for a given domain.

It is the user-facing abstraction that allows participants and agents to pull the current codebase without manually reconstructing it from raw diffs.

A `CodebaseStateRef` resolves to:

- a `CanonicalFrontierState`, or
- a specific historical `MaterializedState`

It supports use cases such as:

- pulling the latest canonical codebase for a domain
- pinning a historical state for replay
- comparing current frontier against a prior materialized state
- initializing a new autonomous agent run

Conceptually, this makes the protocol usable as both:

- a ledger of validated improvements
- a live versioned code substrate for further research

---

### User and Agent Workflow Guarantee

For every active `ProblemDomain`, the protocol guarantees that a participant can retrieve the current canonical assembled state and begin work from it.

In practical terms, the protocol supports a workflow equivalent to:

```bash
arc domains list
arc domain show nanochat-base
arc pull nanochat-base
arc mine nanochat-base
```

And for narrower domains:

```bash
arc mine optimizer-subspace
arc mine data-pipeline-efficiency
arc submit --from optimizer-subspace --into nanochat-base
```

These commands are illustrative, not normative, but they express a required user-experience invariant: participants must be able to discover domains, pull their current frontier state, and contribute local or cross-domain improvements without reconstructing protocol state manually.

---

### Domain Reward Separation

Each domain has its own reward context.

This does not necessarily require fully isolated treasuries, but it does require clear accounting boundaries for:

- domain-local proposer rewards
- domain-local validation rewards
- domain-local challenge rewards
- cross-domain integration rewards
- downstream transfer rewards

This allows specialized domains to be economically meaningful without automatically claiming end-to-end value they have not yet demonstrated.

---

### Upstream Integration Rule

A result discovered in a child or sibling domain does not automatically modify the parent domain's canonical frontier.

Upstream movement requires an explicit integration event. A child-domain improvement becomes upstream-relevant only when:

1. it is referenced by an integration block,
2. it is ported into the destination domain,
3. it is validated under the destination domain's rules,
4. and it survives the destination domain's challenge and fork logic.

This prevents speculative local wins from silently contaminating the broader frontier.

---

### Required Protocol Guarantees

The multi-domain protocol satisfies the following guarantees:

1. The chain supports multiple concurrent problem domains.
2. Each domain has a distinct evaluation and reward context.
3. Each domain has a current canonical frontier state that can be pulled.
4. Specialized sub-problem work can occur without polluting broader domains.
5. Useful local results can be integrated upstream through explicit cross-domain blocks.
6. The protocol exposes pullable assembled code states, not only raw diffs.
7. Fork competition remains domain-local unless explicit integration occurs.

---

### Rationale

A mature Proof-of-Useful-Work protocol cannot remain bound to a single benchmark.

If the protocol is to become a real market for useful AI research work, it must support:

- many models
- many benchmarks
- many subsystems
- many narrow research loops
- many future training markets

At the same time, it preserves a simple invariant for users: for any domain, there is always a current best protocol-recognized codebase that can be pulled, inspected, and improved.

This turns the chain from a passive ledger of patches into a living public substrate for autonomous research.

---

[Back to docs index](README.md) | [White Paper](whitepaper.md) | [Project Scope](project-scope.md)
