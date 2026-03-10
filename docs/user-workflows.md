# User Workflows

## Purpose

This document explains what actual participants do in AutoResearch Chain.

The protocol is abstract enough that readers may understand the mechanics without understanding the operational workflows.

This file closes that gap.

---

## 1. Proposer Workflow — Domain Miner

### Goal

Mine validated improvements to a given research domain.

### Example user

A GPU owner running an autonomous research agent against `nanochat-base` or `optimizer-subspace`.

### Workflow

1. Discover active domains.
2. Select a domain.
3. Pull the current canonical frontier state.
4. Initialize the agent against that state.
5. Run bounded local search.
6. If the result improves the metric, submit a block with evidence.
7. Wait for validation, challenge, and settlement.

### Conceptual commands

    arc domains list
    arc domain show nanochat-base
    arc pull nanochat-base
    arc mine nanochat-base

### What the proposer contributes

- GPU time
- agent-driven search
- candidate recipe diffs
- evidence bundles

---

## 2. Validator Workflow — Replay Operator

### Goal

Verify whether a proposed improvement actually reproduces.

### Example user

A participant with sufficient hardware and a validator bond.

### Workflow

1. Receive or discover assigned validation work.
2. Pull the parent and child state.
3. Pull the evidence bundle.
4. Reconstruct the environment.
5. Replay the required runs.
6. Measure the result.
7. Submit a signed attestation.

### Conceptual commands

    arc validator assignments
    arc validator fetch <block-id>
    arc validator replay <block-id>
    arc validator attest <block-id>

### What the validator contributes

- replay labor
- evidence-backed attestation
- protocol-level epistemic pressure

---

## 3. Challenger Workflow — Adversarial Auditor

### Goal

Dispute false or weak claims and profit from successful falsification.

### Example user

A technically strong participant who spots:
- a reproducibility failure
- suspicious artifacts
- attribution theft
- false dominance
- domain contamination

### Workflow

1. Inspect a target block, attestation, or claim.
2. Gather evidence.
3. Post a challenge bond.
4. Trigger dispute resolution.
5. If the challenge succeeds, collect reward.

### Conceptual commands

    arc inspect <target-id>
    arc challenge open <target-id>
    arc challenge status <challenge-id>

### What the challenger contributes

- falsification pressure
- protocol defense against fraud and weak claims
- economic pressure against lazy validation

---

## 4. Subdomain Miner Workflow

### Goal

Work on a narrow or tertiary problem without directly modifying the broader frontier.

### Example user

A participant specializing in:
- optimizer search
- scheduler search
- memory efficiency
- dataloader performance
- distributed sync logic

### Workflow

1. Discover specialized domains.
2. Pull the canonical frontier state for that domain.
3. Run agent or manual search in the narrow domain.
4. Submit domain-local improvements.
5. Optionally port the result into a broader domain later.

### Conceptual commands

    arc domain show optimizer-subspace
    arc pull optimizer-subspace
    arc mine optimizer-subspace

### Why this matters

Not every useful improvement should immediately affect the root model domain.

Specialization should be possible without polluting the main frontier.

---

## 5. Cross-Domain Integrator Workflow

### Goal

Take an improvement discovered in one domain and prove it helps in another.

### Example user

A participant who sees a useful improvement in `optimizer-subspace` and wants to integrate it into `nanochat-base`.

### Workflow

1. Identify a valuable source-domain result.
2. Pull the source attributed unit or frontier block.
3. Pull the destination domain frontier state.
4. Port the improvement into the destination codebase.
5. Validate under the destination domain’s rules.
6. Submit a cross-domain integration block.

### Conceptual commands

    arc pull optimizer-subspace
    arc pull nanochat-base
    arc submit --from optimizer-subspace --into nanochat-base

### What the integrator contributes

- synthesis
- transfer proof
- upward movement of specialized knowledge into broader domains

---

## 6. Domain Explorer Workflow

### Goal

Understand what research markets are active and where contribution is most useful.

### Example user

A new participant deciding where to deploy hardware.

### Workflow

1. List active domains.
2. Review domain metadata.
3. Inspect canonical frontier state.
4. Inspect challenge pressure, activity, and reward context.
5. Choose a domain to mine or validate.

### Conceptual commands

    arc domains list
    arc domain show <domain-id>
    arc frontier show <domain-id>

---

## 7. Governance Participant Workflow

### Goal

Help tune protocol parameters without deciding scientific truth.

### Example user

A token-bearing or otherwise authorized governance participant.

### Workflow

1. Review governance proposals.
2. Evaluate whether they affect:
   - fees
   - bonds
   - threshold rules
   - epoch transitions
   - benchmark rotations
3. Vote within protocol constraints.
4. Do not attempt to adjudicate scientific outcomes directly.

### What governance should not do

Governance should not manually decide:
- which branch is correct
- which result is “true”
- who deserves scientific credit outside protocol rules

---

## 8. Cold-Start Reader Workflow

### Goal

Understand the project quickly.

### Suggested reading order

1. README
2. executive summary
3. what-this-is-and-is-not
4. first principles
5. protocol spec
6. roadmap
7. attack model

---

## User Experience Invariant

For every active domain, a participant should be able to:

1. discover the domain
2. inspect the current frontier
3. pull the canonical assembled state
4. run work from that state
5. submit, validate, challenge, or integrate results

If this is not possible, the protocol is not yet usable as a living public research substrate.