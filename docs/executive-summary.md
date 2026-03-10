<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Executive Summary

AutoResearch Chain is a fully decentralized protocol that turns GPU compute into **Proof of Useful Work**.

Instead of spending energy on arbitrary hashing, participants use their GPUs to:

- propose improvements to AI training recipes,
- validate improvements discovered by others,
- challenge invalid or fraudulent claims,
- compete across forks,
- and converge on better training code over time.

The useful work being mined is not a meaningless cryptographic puzzle. It is measurable progress in AI training methodology.

## Core Thesis

- People optimize what is rewarded.
- Any reward surface will be gamed.
- Therefore the goal is not to eliminate adversaries.
- The goal is to structure an adversarial market in which selfish participants are incentivized to expose false claims, validate real gains, and compete to produce better training recipes.

## Where the Value Comes From

Blockchain is **not** the source of value.

The value is the research output.

Blockchain is the mechanism that makes a **trustless, permissionless, auditable market** for that research possible.

## What the Protocol Produces

AutoResearch Chain is best understood as a decentralized market for:

- AI training recipe discovery across multiple concurrent research domains,
- validation,
- falsification,
- specialized sub-problem and component-level optimization,
- cross-domain integration of useful results,
- and eventually decentralized large-scale training.

## Stage 1: The Mining Primitive

The canonical mining loop is agent-driven, closely mirroring autonomous research tools like [`karpathy/autoresearch`](https://github.com/karpathy/autoresearch).

A participant runs a local training environment with an AI agent. The agent:

1. Reads the current training recipe
2. Modifies the training code
3. Runs a short GPU experiment
4. Measures the result against the parent recipe
5. Submits the diff as a candidate block if it improves the metric
6. Otherwise discards it and continues searching

Validators replay transitions. Challengers dispute false claims. The chain records and rewards validated progress.

## Multi-Domain Research

The protocol is not limited to a single benchmark or model. It supports multiple concurrent problem domains — from full end-to-end recipe optimization to narrow sub-technique and infrastructure problems. Each domain has its own evaluation logic, fork competition, and reward context. For any active domain, participants can pull the current canonical assembled codebase and begin mining improvements from that state.

## What Exists Today

The protocol specifies a decentralized **research-discovery layer** including block submission, bonded validation, adversarial challenge, fork-native search, staged rewards, scale-validation hooks, multi-domain research support, and canonical frontier state exposure.

## What Does Not Exist Yet

The full decentralized long-horizon training layer (Stage 3) is compatible with the design but not yet formally specified. See [Future: Stage 3 Training](future-stage-3-training.md).

## Short Thesis

> AutoResearch Chain is a fully decentralized, adversarial, fork-native market in which GPU owners and AI agents mine validated improvements to AI training recipes. Its success depends on whether the protocol makes real progress more profitable than exploitative behavior.

---

[Back to docs index](README.md) | [Full White Paper](whitepaper.md)
