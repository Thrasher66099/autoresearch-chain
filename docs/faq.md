# FAQ

## What is AutoResearch Chain?

AutoResearch Chain is a fully decentralized, fork-native Proof-of-Useful-Work protocol for mining validated improvements to AI training recipes.

---

## What does “Proof of Useful Work” mean here?

It means GPU compute is used to produce useful AI research work rather than arbitrary hash puzzles.

In the current design, the useful work is:
- proposing recipe improvements
- validating improvements
- challenging false claims
- integrating useful ideas

---

## Is this just a blockchain for AI?

No.

The project is not “AI plus token.”
It is a mechanism-design project for building a trustless market around useful AI research work.

The value proposition is the research output.
Blockchain is the substrate that allows the market to be trustless.

---

## Is this just Karpathy’s `autoresearch` with crypto added on?

No.

Autoresearch-style agent loops are an enabling primitive for Stage 1.
They show that bounded research loops can be run by an agent.

AutoResearch Chain adds:
- decentralized validation
- challenge markets
- fork-native competition
- attribution
- multi-domain research
- canonical frontier states
- incentive settlement

So the relationship is:
- autoresearch provides a useful-work primitive
- AutoResearch Chain provides the decentralized market around that primitive

---

## Does the protocol already support long-horizon decentralized shared model training?

Not fully.

The current protocol is focused on the research-discovery layer:
- mining validated improvements to AI training recipes

A future Stage 3 would cover long-horizon decentralized shared training, but that is not yet fully specified.

---

## What is Stage 1?

Stage 1 is the autonomous research-discovery layer.

Users run GPUs plus autonomous research agents to search for recipe improvements in active domains.

---

## What is Stage 2?

Stage 2 is larger-scale transfer validation.

It tests whether Stage 1 improvements survive stronger budgets or larger settings.

---

## What is Stage 3?

Stage 3 is the future long-horizon shared-training layer.

It would allow decentralized contributors to participate directly in persistent model training.

---

## Why does the project need to be fully decentralized?

Because any essential centralized authority breaks the trustless market thesis.

If a central actor can decide:
- who validates,
- what counts,
- who gets paid,
- or which result is true,

then the system becomes a managed platform instead of an institutionless market.

---

## Why are forks necessary?

Because research is not linear.

Different contributors and agents may find different plausible improvements at the same time.
Forks allow parallel search and later synthesis.

---

## Why not just make it a centralized benchmark platform?

Because the project is not trying to be a better hosted leaderboard.

It is trying to create a trustless, adversarial market where useful AI research work is coordinated without discretionary gatekeepers.

---

## What prevents gaming?

Nothing prevents all gaming.

The point is not to eliminate gaming.
The point is to make:
- false claims challengeable,
- validation profitable,
- exploitative behavior costly,
- and robust improvements more profitable than weak ones.

---

## What are domains?

Domains are research arenas.

A domain might represent:
- a full training recipe
- a subsystem
- a sub-technique
- a tertiary infrastructure problem

Each domain has its own frontier, evaluation logic, and reward context.

---

## Can users work on sub-problems, not just one model like `nanochat`?

Yes.

The protocol is intended to support multiple domains and subdomains so users can mine improvements to:
- full models
- components
- subsystems
- tertiary problems

---

## Can I pull the current codebase from the chain?

Conceptually, yes.

The protocol is designed so each domain has a canonical frontier state that participants can pull and use as the parent state for new work.

This is essential to making the chain a usable research substrate rather than just a ledger of diffs.

---

## Is the protocol already implemented?

The repo should be read as protocol design and project documentation first.

Readers should not assume the entire protocol has already been implemented just because it has been specified.

---

## What is the single-sentence version of the project?

AutoResearch Chain is a fully decentralized protocol where GPUs and autonomous research agents mine validated improvements to AI training recipes through replay, challenge, and fork-native competition.