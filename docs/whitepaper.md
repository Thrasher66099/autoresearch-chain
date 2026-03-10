<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# AutoResearch Chain: White Paper

**A fully decentralized, fork-native Proof-of-Useful-Work protocol for mining validated improvements to AI training recipes.**

---

## 1. Introduction

AutoResearch Chain is a protocol for coordinating GPU owners, AI agents, validators, and challengers around a shared competitive research game.

At a high level:

1. An AI agent modifies a training recipe.
2. The modified recipe is tested on GPU.
3. If it improves the target metric, it is submitted as a block.
4. Validators replay the result.
5. If the claim survives challenge and replay, it receives reward.
6. Competing branches can coexist as forks.
7. Better branches converge over time.
8. Winning ideas can later be validated at larger scale and eventually used for decentralized long-horizon model training.

## 2. Core Thesis

AutoResearch Chain treats AI training improvements as mineable discoveries.

Participants use GPUs to mine:

- training-code improvements,
- optimizer improvements,
- scheduler improvements,
- architecture improvements,
- scaling insights,
- and later, sustained training contribution itself.

This is **Proof of Useful Work** because GPU cycles are spent producing useful AI research rather than arbitrary hash puzzles.

The core economic thesis is:

- People optimize what is rewarded.
- Any reward surface will be gamed.
- Therefore the goal is not to eliminate adversaries.
- The goal is to structure an adversarial market in which selfish participants are incentivized to expose false claims, validate real gains, and compete to produce better training recipes.

Blockchain is **not** the source of value.
The value is the research output.
Blockchain is the mechanism that makes a **trustless, permissionless, auditable market** for that research possible.

## 3. Why Decentralization Is Non-Negotiable

If any essential function is centralized, the trustless market thesis breaks.

A centralized system can still be useful, but it is no longer a true adversarial market if some trusted operator can:

- decide who may validate,
- decide which results count,
- settle disputes by discretion,
- release or block rewards,
- manually resolve attribution,
- or overrule scientific outcomes.

AutoResearch Chain therefore requires:

- permissionless participation,
- no trusted coordinator,
- no privileged validator set,
- no manual scientific arbiter,
- no off-chain payout authority,
- governance that can tune parameters but not decree truth.

If any of those fail, the system becomes a platform run by an organization rather than an institutionless market.

## 4. Relationship to `karpathy/autoresearch`

This project is explicitly inspired by and designed around the `autoresearch` paradigm.

In the original concept, Stage 1 is built on **Andrej Karpathy's `autoresearch`**, specifically the Windows/RTX fork, where an AI agent:

- reads `program.md`,
- modifies `train.py`,
- runs a short GPU training experiment,
- measures `val_bpb`,
- keeps improvements,
- discards failures,
- and repeats autonomously.

AutoResearch Chain wraps that loop in a decentralized protocol.

That means the intended workflow is:

- users provide GPU access,
- an AI agent runs the autoresearch loop locally,
- successful diffs become candidate protocol blocks,
- validators replay the same parent/child recipe transition,
- the chain records and rewards validated progress.

The protocol is designed to work with `karpathy/autoresearch` or something very similar as the canonical Stage 1 research engine. No code from `autoresearch` is currently included in this repository.

## 5. Stages

### Stage 1 — Recipe Discovery

Consumer GPUs run independent short-horizon experiments on small models.

An AI agent modifies the training recipe, runs the experiment, measures the delta, and submits improvements as blocks.

Output:

- better training code,
- better training recipes,
- a forked history of validated improvements.

### Stage 2 — Scale Validation

Higher-end hardware tests whether Stage 1 improvements transfer to larger models and longer training budgets.

Output:

- scale-validation signals,
- dead-end detection,
- transfer confidence.

### Stage 3 — Decentralized Training

Once a recipe is sufficiently battle-tested, contributors form a decentralized training swarm and use the winning recipe to train a shared model over long horizons.

Output:

- trained open model weights,
- decentralized compute contribution,
- sustained useful training work.

Stage 3 is not yet fully specified. See [Future: Stage 3 Training](future-stage-3-training.md).

## 6. Why "Lucky" Improvements Still Count

The protocol does not care whether an improvement came from genius insight, stochastic search, brute force, mutation, or blind experimentation.

It only cares whether the improvement is:

- reproducible,
- challenge-resistant,
- and useful under protocol evaluation.

A lucky discovery that survives replay is still useful work.

The distinction is not "insightful vs. non-insightful." It is:

- **replayable edge** vs. **measurement artifact**

## 7. Reward Philosophy

The protocol should not mainly pay for making claims.

It should pay for surviving falsification.

That means rewards are staged:

1. **Provisional reward** after initial validation.
2. **Survival reward** after challenge windows.
3. **Integration reward** if the idea is imported into a dominant branch.
4. **Frontier reward** for dominant-lineage advancement.
5. **Transfer reward** if the improvement survives larger-scale validation.

This makes local wins economically meaningful without overpaying noise or fraud.

## 8. Forks as a Feature

Forks are a feature, not a bug.

If two or more valid improvements target the same parent, the protocol allows them all to exist simultaneously.

Why this matters:

- real research is parallel,
- premature convergence is bad,
- different branches may find different useful ideas,
- later branches may merge or import ideas from each other.

Forks compete until evidence-based convergence.

During unresolved fork competition, immediate rewards are reduced. This creates pressure to converge while still allowing exploration.

### Cross-Fork Synthesis

Cross-fork idea porting is allowed by design.

If one branch finds a useful trick and another branch ports it faster or integrates it better, that is often a feature, not a bug.

The protocol should reward:

- discovery of useful ideas,
- successful integration of useful ideas,
- advancement of the best frontier.

This turns forks into a competitive synthesis process rather than a winner-take-all tournament.

## 9. Ancestry Farming

The protocol must prevent participants from farming reward merely by inserting themselves into lineage.

Examples of ancestry farming:

- trivial intermediate blocks,
- genealogy rent extraction,
- synthetic fork positioning,
- passive royalty capture without meaningful contribution.

To prevent this:

- ancestry alone should not entitle a block to reward,
- deep ancestor claims must decay,
- trivial blocks should receive little or no downstream share,
- reward should follow causal contribution, not just position in history.

## 10. Human Value Proposition

AutoResearch Chain could create real value for humanity if it succeeds in creating a global market where GPU owners and AI agents compete to generate, validate, and falsify useful AI research work.

Potential benefits include:

- converting compute into public research output,
- broadening participation in AI progress,
- creating a ledger of validated training improvements,
- reducing dependence on centralized research institutions,
- producing open training recipes and eventually open model weights.

The value is not "because blockchain."
The value is the research.
Blockchain is the mechanism that lets the trustless market exist.

## 11. Design Principles

1. Parallel search is first-class.
2. Validation is provisional.
3. Claims must be challengeable.
4. Rewards should follow survival through falsification.
5. Decentralization is non-negotiable.
6. Governance may tune the game, but not decide truth.
7. Forks are native.
8. Cross-fork synthesis is desirable.
9. Ancestry alone should not produce rent.
10. Useful work, not arbitrary hashing, is the core mining primitive.

## 12. Current Identity

The cleanest description of the current system is:

> AutoResearch Chain is a fully decentralized Proof-of-Useful-Work protocol for mining validated improvements to AI training recipes.

That statement is accurate.

A future version may also become:

> A fully decentralized Proof-of-Useful-Work protocol for long-horizon shared AI model training.

But that is a later layer, not the current complete spec.

## 13. Short Thesis

AutoResearch Chain is a fully decentralized, adversarial, fork-native market in which GPU owners and AI agents mine validated improvements to AI training recipes.

Its success depends on one thing:

> Whether the protocol makes real progress more profitable than exploitative behavior.

If it does, it becomes a new institutionless market for AI research progress.

---

[Back to docs index](README.md) | [Protocol Specification](protocol-v0.2.md) | [Project Scope](project-scope.md)
