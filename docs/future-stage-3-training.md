<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Future: Stage 3 — Decentralized Training

**Status: Not yet formally specified. This document describes the vision and known requirements, not a complete design.**

---

## Overview

The long-term vision for AutoResearch Chain includes a decentralized training layer.

At that stage, participants would not only mine improvements to training recipes. They would also contribute GPU-hours directly to a shared long-running model training process, using battle-tested recipes produced by Stages 1 and 2.

## Why Stage 3 Is Separate

Stage 3 involves fundamentally different technical challenges than recipe discovery:

- **Stage 1** is short-horizon, independent, and easily replayable.
- **Stage 3** is long-horizon, collaborative, and requires sustained coordination across many participants.

Conflating the two would risk overclaiming the current protocol's capabilities. Stage 3 should be treated as a separate formal subsystem that builds on the foundation of the research-discovery layer.

## Known Requirements

A decentralized training layer would require at minimum:

- **Swarm coordination**: scheduling and synchronizing training work across distributed participants
- **Checkpoint tracking**: maintaining a shared record of training state across the swarm
- **Sync attestations**: verifying that participants are working from the correct shared state
- **Gradient validation**: detecting invalid or adversarial gradient contributions
- **Contribution proofs**: demonstrating that a participant performed useful training work
- **Adversarial training defense**: protecting the shared model from poisoning attacks
- **Reward proportional to useful sustained training work**: compensating contributors fairly for ongoing compute

## Relationship to Current Protocol

Stage 3 is a logical extension of the protocol. If Stages 1 and 2 produce battle-tested training recipes, Stage 3 uses those recipes for actual large-scale training.

The current protocol architecture (bonded participation, adversarial challenge, staged rewards, fork competition) provides a foundation that Stage 3 can build on, but the specific mechanisms for long-horizon distributed training are qualitatively different from short-horizon recipe search.

## Output

If realized, Stage 3 would produce:

- Trained open model weights
- Decentralized compute contribution records
- Sustained useful training work (the fullest expression of Proof of Useful Work)

## Current Status

Stage 3 is not yet part of the protocol specification. The requirements listed above are directional, not definitive. A rigorous specification would need to address the coordination, verification, and economic challenges of decentralized long-horizon training in detail.

---

[Back to docs index](README.md) | [Project Scope](project-scope.md) | [White Paper](whitepaper.md)
