<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Terminology

Glossary of terms used in the AutoResearch Chain protocol.

---

| Term | Definition |
|------|------------|
| **Block** | A claim that a child training recipe improves on a parent training recipe. Includes a diff, evidence bundle, and bond. |
| **Evidence bundle** | The complete set of artifacts (code diff, config, logs, metrics, environment manifest) required to replay and verify a block. |
| **Proposer** | A participant who submits a candidate recipe improvement as a block. |
| **Validator** | A bonded participant who replays a parent/child transition and submits a signed attestation of the result. |
| **Challenger** | A participant who disputes a block, attestation, attribution claim, or fork dominance declaration. |
| **Scale validator** | A validator who tests whether a Stage 1 improvement transfers to larger models or longer training budgets (Stage 2). |
| **Governor** | A participant who votes on protocol parameter changes. Cannot override validation outcomes. |
| **Fork** | A divergent branch in the recipe history where multiple valid improvements target the same parent. Forks are a first-class protocol feature. |
| **Fork family** | The set of competing branches that share a common ancestor. |
| **Epoch** | A discrete time period in the protocol used for validator sampling, reward distribution, and challenge windows. |
| **Bond** | Stake posted by proposers, validators, or challengers that can be slashed for misbehavior. |
| **Slashing** | Forfeiture of a participant's bond as penalty for provably false claims or attestations. |
| **Escrow** | Temporary holding of rewards pending challenge-window expiration and confidence settlement. |
| **Attestation** | A signed vote by a validator on the outcome of a block replay (`PASS`, `FAIL`, `INCONCLUSIVE`, or `FRAUD_SUSPECTED`). |
| **Training recipe** | The complete specification for a training run: code, configuration, hyperparameters, dataset references, and evaluation procedure. |
| **Metric delta** | The measured difference in the target evaluation metric between a parent and child recipe. |
| **Proof of Useful Work** | The principle that mining work produces useful output (validated training improvements) rather than arbitrary computation. |
| **Ancestry farming** | An attack pattern where participants insert trivial blocks into lineage to capture downstream reward without meaningful contribution. |
| **Cross-fork porting** | Importing a useful technique discovered in one fork into a competing fork. Allowed and incentivized by the protocol. |
| **Confidence settlement** | The final determination of reward eligibility after all challenge windows have closed. |
| **Replay** | Re-executing a training run from the evidence bundle to verify the claimed metric delta. |
| **Stage 1** | Recipe Discovery: short-horizon experiments on consumer GPUs using agent-driven search. The current protocol focus. |
| **Stage 2** | Scale Validation: testing whether Stage 1 improvements transfer to larger models. Partially specified. |
| **Stage 3** | Decentralized Training: long-horizon shared model training using battle-tested recipes. Not yet specified. |
| **Research-discovery layer** | The portion of the protocol that handles recipe submission, validation, challenge, and reward. The current scope. |
| **Data availability** | The requirement that all reward-relevant evidence be publicly retrievable and verifiable. |

---

[Back to docs index](README.md)
