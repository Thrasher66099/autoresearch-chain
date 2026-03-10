<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Terminology

## Purpose

This document provides canonical definitions for key terms used throughout the AutoResearch Chain repository.

It exists to reduce ambiguity and keep the protocol legible.

---

| Term | Definition |
|------|------------|
| **AutoResearch Chain** | The overall project and protocol. A fully decentralized, fork-native Proof-of-Useful-Work system for mining validated improvements to AI training recipes. |
| **Proof of Useful Work** | A proof-of-work paradigm in which economically rewarded computation is directed toward useful external output rather than arbitrary hash production. In this project, the useful work is discovering, validating, challenging, and integrating improvements to AI training recipes. |
| **Trustless market** | A market in which participation, validation, challenge, and payout legitimacy do not depend on discretionary trust in a central operator. |
| **Open intelligence production** | The broader vision that globally distributed compute and agents can be coordinated to produce useful AI research work in an open adversarial system. |
| **Stage 1** | Recipe Discovery: the research-discovery layer. Participants run GPUs plus autonomous research agents to mine validated improvements to training recipes. The current protocol focus. |
| **Stage 2** | Scale Validation: testing whether Stage 1 improvements transfer to larger models or longer training budgets. Partially specified. |
| **Stage 3** | Decentralized Training: the future long-horizon shared-training layer in which participants contribute directly to decentralized model training over time. Not yet specified. |
| **Research-discovery layer** | The portion of the protocol that handles recipe submission, validation, challenge, and reward. The current scope. |
| **Autonomous research agent** | An AI agent that reads a codebase or training recipe, modifies it, runs experiments, measures results, and iterates. This is the canonical Stage 1 research worker. |
| **Autoresearch-style loop** | A local agent-driven research loop similar to Karpathy's `autoresearch`, where an agent modifies training code, runs bounded experiments, and keeps improvements. |
| **Proposer** | A participant who submits a candidate recipe improvement as a block. |
| **Validator** | A bonded participant who replays a parent/child transition and submits a signed attestation of the result. |
| **Challenger** | A participant who disputes a block, attestation, attribution claim, or fork dominance declaration. |
| **Scale validator** | A validator who tests whether a Stage 1 improvement transfers to larger models or longer training budgets (Stage 2). |
| **Governor** | A participant who votes on protocol parameter changes. Cannot override validation outcomes. |
| **State** | An immutable snapshot of a training recipe or protocol-relevant configuration state. |
| **Block** | A claim that a child training recipe improves on a parent training recipe. A proposed transition from a parent state to a child state. Includes a diff, evidence bundle, and bond. |
| **BlockDiff** | An incremental code or recipe change proposed in a block. Distinguished from a `MaterializedState` (fully resolved assembly). |
| **Training recipe** | The complete specification for a training run: code, configuration, hyperparameters, dataset references, and evaluation procedure. |
| **Metric delta** | The measured difference in the target evaluation metric between a parent and child recipe. |
| **Evidence bundle** | The complete public set of artifacts (code diff, config, environment manifest, dataset references, logs, metrics, artifact hashes) required to replay and verify a block. |
| **Validation attestation** | A signed validator claim about whether a proposed improvement reproduces under protocol rules. Votes are `PASS`, `FAIL`, `INCONCLUSIVE`, or `FRAUD_SUSPECTED`. |
| **Replay** | Re-executing a training run from the evidence bundle to verify the claimed metric delta. |
| **Challenge record** | A bonded dispute object in the protocol, filed against a block, attestation, attribution, or related protocol claim. |
| **Confidence settlement** | The final determination of reward eligibility after all challenge windows have closed. |
| **EpochSpec** | A protocol object defining the rules of a research game during a fixed interval. Usually includes datasets, metrics, environment requirements, compute policies, thresholds, reward parameters, and challenge windows. |
| **Epoch** | A discrete time period in the protocol used for validator sampling, reward distribution, and challenge windows. |
| **Bond** | Stake posted by proposers, validators, or challengers that can be slashed for misbehavior. |
| **Slashing** | Forfeiture of a participant's bond as penalty for provably false claims or attestations. |
| **Escrow** | Temporary holding of rewards pending challenge-window expiration and confidence settlement. |
| **Data availability** | The requirement that all reward-relevant evidence be publicly retrievable and verifiable. |
| **Fork** | A divergent branch in the recipe history where multiple valid improvements target the same parent. Forks are a first-class protocol feature. |
| **Fork family** | The set of sibling-descended competing branches within a given domain and divergence context that share a common ancestor. |
| **Frontier** | The leading edge of a branch or domain's current search state. |
| **Dominance** | The condition in which one fork or frontier is recognized as superior within its local competition space under protocol rules. |
| **Cross-fork porting** | Importing a useful technique discovered in one fork into a competing fork. Allowed and incentivized by the protocol. |
| **Origin attribution** | Credit for first validated appearance of a useful idea or attributed unit. |
| **Integration attribution** | Credit for successfully porting or combining a useful idea into a stronger or dominant branch. |
| **Frontier attribution** | Credit for moving the best validated frontier forward along the dominant path. |
| **Ancestry farming** | An attack pattern where participants insert trivial blocks into lineage to capture downstream reward without meaningful contribution. |
| **Transfer validation** | Testing whether a local improvement survives at larger scale or under more meaningful conditions. |
| **ProblemDomain** | A protocol-defined research arena. Each domain defines a specific problem participants are trying to improve, with its own codebase root, evaluation logic, fork competition space, canonical frontier, and reward context. |
| **DomainSpec** | The structural specification of a `ProblemDomain`. Defines codebase root, evaluation targets, metrics, modification surface, epoch policy, fork policy, integration rules, canonicalization behavior, and materialization rules. Each domain has exactly one active `DomainSpec` per protocol interval. |
| **Domain type** | A descriptive classification of a `ProblemDomain`. Types include `root`, `model`, `subsystem`, `technique`, `infrastructure`, `integration`, and `experimental`. Types may influence default policy but do not override explicit rules. |
| **Hierarchical domain** | A domain that has a parent domain and/or child domains. Hierarchy allows narrow specialized work to occur without polluting broader frontiers, while still permitting explicit upward integration of successful results. |
| **Domain-local improvement** | A block that improves performance within its own domain under that domain's evaluation logic. Has not necessarily been validated in any parent or sibling domain. |
| **Cross-domain integration** | The act of importing or porting an improvement from one domain into another domain and validating it under the destination domain's rules. |
| **CrossDomainIntegrationBlock** | A block that references a source domain and one or more source artifacts, ports the improvement into a destination domain, and validates it under the destination domain's evaluation rules. |
| **CanonicalFrontierState** | The current protocol-recognized best assembled state of a `ProblemDomain`. Includes or resolves to the dominant frontier block, full source tree, resolved configuration, dependency manifest, environment manifest, evaluation manifest, and content-addressed snapshot reference. This is what participants pull to begin new work. |
| **MaterializedState** | A full assembled working snapshot of a domain's codebase and execution context. Distinguished from a `BlockDiff` (incremental change). Content-addressed and publicly fetchable. Required at fork dominance transitions, scheduled checkpoints, or when diff chains exceed policy thresholds. |
| **CodebaseStateRef** | A protocol-resolvable reference to a full assembled codebase state for a given domain. Resolves to a `CanonicalFrontierState` or a specific historical `MaterializedState`. Allows participants to pull a working codebase without reconstructing it from raw diffs. |
| **Canonicalization** | The process by which the protocol exposes a usable current assembled state rather than only a list of diffs. |
| **Domain reward separation** | The requirement that each domain has its own reward accounting boundaries for proposer, validation, challenge, integration, and transfer rewards. Prevents specialized domains from claiming end-to-end value not yet demonstrated. |
| **Upstream integration rule** | The rule that results discovered in a child or sibling domain do not automatically modify the parent domain's frontier. Upstream movement requires an explicit integration block that is validated and survives challenge in the destination domain. |
| **ResearchTrackStandard (RTS)** | An interface specification that defines the minimum shape a research track must satisfy to participate in the protocol. Defines the protocol-visible structure required for decentralized validation, challenge, and reward settlement without hardcoding any specific benchmark or metric. |
| **RTS-1** | The first research track standard. A single-metric fixed-budget standard for bounded single-node or single-GPU replay, autonomous research-agent loops, and Stage 1 research-discovery markets. |
| **GenesisBlock** | The root block of a new research track. Unlike ordinary blocks, a genesis block is not a claim of improvement over a parent — it is a claim that a new research arena is sufficiently well-defined to be instantiated as a protocol-recognized market. Establishes the rules of the game for all descendant blocks in that track. |
| **Research target declaration** | A human-readable and machine-indexable field in a genesis block declaring what the track is trying to optimize. Supports discovery, track selection, adequacy challenges, and migration logic. |
| **Domain intent** | A protocol-legible declaration of the intended class of value a domain seeks to produce (e.g., `end_to_end_recipe_improvement`, `subsystem_optimization`). Useful for market discovery and governance-neutral metadata. |
| **TrackInitialization** | The lifecycle process through which a proposed genesis block becomes an active research track. Supports permissionless genesis with economic filtering: participants post a seed bond and supply an RTS-conformant package, which must pass conformance checks, seed reproduction, and activation thresholds. |
| **Genesis activation conditions** | The set of conditions a genesis proposal must satisfy to become an active track: RTS conformance, artifact availability, seed score reproduction, minimum validator participation, minimum bonded activation threshold, and no upheld fraud challenge. |
| **TrackTree** | The domain-scoped descendant tree rooted at a single genesis block. Each TrackTree has its own fork families, validator scope, reward context, canonical frontier, and challenge surface. The chain is a forest of independent domain-rooted TrackTrees. |
| **MetricIntegrityPolicy** | A per-track policy defining the integrity requirements for the evaluation metric: immutable metric declaration, immutable direction, frozen evaluation harness, search/frozen surface separation, replay requirements, tolerance rules, and challengeability of invalid evaluation setups. |
| **DatasetIntegrityPolicy** | A per-track policy defining dataset integrity requirements: canonical reference, content-addressed identity, split declaration, availability requirements, preprocessing rules, and declared license status. |
| **Search surface** | The set of files or modules that participants and agents are permitted to modify within a research track. Declared at genesis. |
| **Frozen surface** | The set of files or modules that must remain fixed for the duration of a research track. Typically includes the evaluation harness and dataset preparation logic. Declared at genesis. |
| **Seed bond** | The economic bond posted by a genesis proposer. Returned, partially slashed, or fully slashed depending on whether the genesis proposal succeeds or fails activation. |
| **Successor track** | A new research track that references a prior track and declares an updated metric, harness, or other structural change. Used instead of silently mutating an active track's rules. Preserves historical integrity and public lineage visibility. |
| **Metric validity** | The property that a track's metric is reproducible and conforms to the declared standard. Distinguished from metric adequacy. |
| **Metric adequacy** | The property that a track's metric is actually a good proxy for the stated research target. More subjective than metric validity and not casually mutable inside an active track. |
| **Domain-scoped validator pool** | A validator pool filtered by hardware compatibility, dataset availability, environment support, and track-specific replay requirements. Prevents invalid sampling across heterogeneous research tracks. |
| **Evaluation harness immutability** | The requirement that the logic computing a track's metric is fixed for the life of the track unless a successor track is explicitly created. Essential for score comparability and economic stability. |

---

[Back to docs index](README.md)
