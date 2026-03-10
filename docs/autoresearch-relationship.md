# Relationship to `autoresearch`

## Purpose

This document explains how AutoResearch Chain relates to autoresearch-style autonomous research loops.

This is important because:
- the Stage 1 protocol is deeply enabled by this paradigm,
- but the project is not reducible to “crypto around autoresearch.”

---

## Why `autoresearch` Matters

Autonomous research loops make this project possible.

The key primitive is:

- an agent can inspect a codebase or training recipe,
- modify it,
- run a bounded experiment,
- measure whether the modification helped,
- and keep useful changes.

This is a major shift because it turns research behavior into something closer to a repeatable compute loop.

That makes it plausible to ask:

**What if globally distributed GPUs competed to run such loops under a trustless market?**

That is the bridge from autoresearch-style loops to AutoResearch Chain.

---

## What Stage 1 Assumes

Stage 1 of AutoResearch Chain assumes users may run something like:
- Karpathy’s `autoresearch`
- a derivative
- or another autonomous research-agent system with similar behavior

The protocol does not depend on one specific implementation, but it does assume this category of loop.

The key pattern is:

1. pull current canonical frontier state
2. let the agent modify the recipe
3. run a bounded experiment
4. compare result to parent
5. submit if improved

That is the canonical Stage 1 mining primitive.

---

## What AutoResearch Chain Adds

Autoresearch-style loops alone do not provide:

- decentralized validation
- open challenge
- fork-native competition
- trustless reward settlement
- attribution
- domain-local and cross-domain research markets
- canonical frontier settlement
- permissionless genesis and track bootstrapping
- eventual transfer validation

AutoResearch Chain adds the market and protocol structure around the loop.

In a local tool, one human chooses the seed model, the metric, the dataset, and the search surface. In the protocol, those choices are formalized through Research Track Standards and genesis blocks — making them explicit, reproducible, challengeable, and permissionless.

This is the difference between:
- a local research tool
and
- a decentralized institutionless market for research work

---

## What the Project Does Not Claim

The project does **not** claim that:
- `autoresearch` alone is the full solution
- any one upstream repo is the entirety of the idea
- current protocol docs imply vendored upstream code by default

Inspiration is not the same as code inclusion.

If upstream code is later copied or adapted, repository licensing and notice files must reflect that explicitly.

---

## Why This Relationship Should Be Explicit

Without this explanation, readers may misunderstand the project in one of two ways:

### Wrong conclusion 1
“This is just a blockchain wrapper around autoresearch.”

### Wrong conclusion 2
“This has nothing to do with autoresearch-style loops.”

Both are false.

The accurate view is:
- autonomous research loops are the enabling primitive,
- AutoResearch Chain is the trustless adversarial market around that primitive.

---

## One-Sentence Relationship

The cleanest summary is:

> Autoresearch-style agents provide the Stage 1 useful-work primitive; AutoResearch Chain provides the decentralized market, validation, challenge, and incentive structure around that primitive.