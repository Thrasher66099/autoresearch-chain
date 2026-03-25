# Attack Model

## Purpose

AutoResearch Chain is a mechanism-design-heavy protocol.

That means the protocol should be evaluated not only by what honest users do, but by what rational adversarial users will try to do.

This document catalogs the major attacks the protocol expects, along with their failure modes, intended mitigations, and open questions.

---

## 1. Noise Harvesting

### Attack
A proposer runs many experiments and only submits the luckiest apparent improvement.

### Failure mode
The protocol rewards variance instead of real progress.

### Mitigations
- replay-based validation
- confidence scoring
- challenge rights
- scale-stage validation
- staged rewards rather than full immediate payout

### Open risk
If the evaluation metric is weak or variance is too high, noise harvesting remains profitable.

---

## 2. Benchmark Overfitting

### Attack
Participants find changes that improve the benchmark or local metric without improving the broader thing the protocol actually cares about.

### Failure mode
The system pays for narrow hacks rather than durable progress.

### Mitigations
- multi-stage validation
- scale transfer
- domain separation
- destination-domain revalidation for upstream integration
- delayed escrow release

### Open risk
Benchmark design remains one of the deepest risks in the entire project.

---

## 3. Branch Spam

### Attack
A participant floods a fork family or domain with low-quality branches, hoping one gets lucky or simply overwhelming validation bandwidth.

### Failure mode
Validator congestion, low signal density, degraded search quality.

### Mitigations
- submission fees
- bonds
- domain-local fork competition
- possible dynamic fee policy
- validation scarcity protection

### Open risk
Fee tuning is a live economic design problem.

---

## 4. Validator Laziness

### Attack
A validator submits low-effort or careless attestations without doing real replay work.

### Failure mode
False positives, false negatives, decayed trust in validation.

### Mitigations
- validator bonding
- challengeability of attestations
- replay evidence requirements
- performance history
- non-performance penalties

### Open risk
Cheap lazy validation may still be tempting if detection is weak.

---

## 5. Validator Collusion

### Attack
Validators coordinate to falsely confirm a block or protect one another from challenge.

### Failure mode
The validation layer becomes corrupt.

### Mitigations
- permissionless challenger role
- additional replay rounds
- deterministic but anti-correlated assignment
- slashing for rule-legible dishonest conduct
- public evidence availability

### Open risk
Subtle collusion can be hard to prove.

---

## 6. Fraudulent Evidence Submission

### Attack
A proposer submits falsified logs, manipulated artifacts, or impossible environment claims.

### Failure mode
The protocol rewards fabricated progress.

### Mitigations
- evidence bundle schema
- fraud veto logic
- public content-addressed artifacts
- challenger incentives
- validator replay requirements

### Open risk
The evidence schema must be strict enough to make fraud legible.

---

## 7. Ancestry Farming

### Attack
A participant tries to create genealogy rent by inserting themselves into the lineage without adding real value.

### Failure mode
The tree becomes financially optimized instead of scientifically optimized.

### Mitigations
- ancestry decay
- trivial block suppression
- attribution tied to causal contribution rather than genealogy alone
- frontier vs origin vs integration distinction

### Open risk
Attribution remains imperfect, so ancestry games are still a real design pressure.

---

## 8. Attribution Manipulation

### Attack
A participant tries to capture credit for an idea discovered elsewhere by strategically porting, rewriting, or reframing a change.

### Failure mode
Contributors lose trust that useful work will be rewarded fairly.

### Mitigations
- explicit origin vs integration vs frontier attribution
- cross-domain integration logic
- attribution claims
- challengeable attribution
- similarity heuristics

### Open risk
Perfect attribution is unlikely. The goal is to be good enough and hard enough to game.

---

## 9. Fork Manipulation

### Attack
A participant strategically uses forks to:
- dilute rewards
- manipulate dominance timing
- block convergence
- trap attention

### Failure mode
The fork structure becomes a weapon rather than a search process.

### Mitigations
- domain-local fork accounting
- timeout rules
- dominance rules
- reduced immediate payout during unresolved branch competition
- explicit canonical frontier state

### Open risk
Fork-economics tuning is still a design frontier.

---

## 10. Cross-Domain Reward Leakage

### Attack
A participant claims broad value for a win that has only been shown in a narrow subdomain.

### Failure mode
Local optimizations get overpaid as end-to-end improvements.

### Mitigations
- domain-local reward separation
- explicit cross-domain integration blocks
- destination-domain validation
- no automatic upstream contamination

### Open risk
Domain design and integration policy must remain strict.

---

## 11. Domain Pollution / Genesis Spam

### Attack
A participant introduces low-quality or badly scoped domains through permissionless genesis proposals to attract rewards, fragment attention, or waste validator resources.

### Failure mode
The protocol becomes cluttered with low-signal arenas. Validator bandwidth is consumed by worthless track activation attempts.

### Mitigations
- seed bond requirements for genesis proposals
- RTS conformance checking
- seed score reproduction by validators before activation
- minimum validator participation and bonded activation thresholds
- failed genesis bond slashing (proportional to failure reason)
- structured DomainSpec rules
- clear evaluation surfaces
- reward accounting boundaries

### Open risk
Genesis bond calibration must balance openness with anti-spam protection. Too low and spam dominates; too high and legitimate experimentation is suppressed.

---

## 12. Scale-Stage Freeloading

### Attack
Everyone wants Stage 2 validation to exist, but too few actors want to pay the real compute cost.

### Failure mode
The protocol underproduces the larger-scale evidence needed to filter Stage 1 noise.

### Mitigations
- explicit scale-stage role
- transfer multipliers
- reserved reward weight for scale validation
- specialized validator compensation

### Open risk
This remains an economic provisioning problem.

---

## 13. Governance Capture

### Attack
A concentrated governance bloc tries to steer the protocol toward favored branches, favored actors, or favorable economic rules.

### Failure mode
The protocol recentralizes politically.

### Mitigations
- governance scope constraints
- no manual scientific truth selection
- timelocks
- constitutional boundaries

### Open risk
Bootstrap concentration is always a risk in governance-bearing systems.

---

## 14. Sybil Behavior

### Attack
One actor uses many identities to:
- spam,
- challenge-grief,
- fake decentralization,
- influence validator dynamics.

### Failure mode
The market appears more pluralistic than it is.

### Mitigations
- bonding requirements
- task assignment rules
- reputation history
- economic cost to participation

### Open risk
Sybil resistance is never free.

---

## 15. Canonical Frontier Poisoning

### Attack
A participant tries to get a fragile or misleading assembled state recognized as the canonical frontier.

### Failure mode
Downstream participants pull and build on a poisoned or misleading codebase.

### Mitigations
- domain-local challenge rights
- materialized state rules
- evidence-backed dominance
- canonical frontier as a protocol result, not a social default

### Open risk
If frontier settlement is weak, poison spreads downstream quickly.

---

## 16. Evaluation Harness Manipulation

### Attack
A participant attempts to improve the metric by modifying the evaluation harness or dataset preparation logic rather than the actual training recipe.

### Failure mode
The protocol rewards changes to the measurement machinery rather than genuine research progress.

### Mitigations
- frozen surface declaration at genesis (evaluation harness and dataset logic are frozen)
- search surface / frozen surface separation enforced by validators
- MetricIntegrityPolicy per track
- evaluation harness immutability for the life of the track
- challengeability of blocks that violate surface constraints

### Open risk
Subtle boundary cases between search and frozen surface may require careful initial scoping in each genesis block.

---

## 17. Long-Horizon Overclaiming

### Attack
The project itself or its community overstates what is already specified or implemented, especially around Stage 3.

### Failure mode
Loss of credibility.

### Mitigations
- clear roadmap
- explicit scope docs
- separate future-stage documentation
- disciplined repo language

### Open risk
Narrative drift is always possible in ambitious projects.

---

## 18. Degenerate Evaluation Surface

### Attack
A domain creator designs a reward function with a hidden exploit — a degenerate input class that scores artificially high — or constructs an evaluation surface where high scores are achievable through shortcut strategies (memorization, pattern matching, trivial reformulations) rather than genuine research improvement. The domain passes conformance checks and seed validation, but the metric does not track useful progress.

A variant: the creator colludes with a proposer who knows the exploit, allowing them to claim rewards that honest proposers cannot match.

### Failure mode
The protocol pays for metric improvement that does not correspond to real research value. Honest proposers waste compute on a domain where the game is rigged. The canonical frontier state is technically "best" by the metric but scientifically worthless.

### Mitigations
- seed bond makes frivolous or malicious domain creation expensive
- seed validation provides early scrutiny of the evaluation surface by independent validators
- challenge games allow anyone to contest blocks or domain integrity
- fork-native competition allows clean competing domains to absorb participants
- permissionless genesis keeps the cost of launching a replacement domain low

### Open risk
The current protocol has no explicit mechanism for community-driven domain deprecation or flagging. A domain with a degenerate evaluation surface that passes conformance and seed validation can persist indefinitely, consuming validator attention and misleading participants.

**This is an unspecified mechanism.** A domain health or deprecation mechanism should exist but does not yet have a complete design. Candidates under consideration include:

- validator-initiated domain review (quorum of validators can flag a domain for re-evaluation)
- stake-weighted deprecation signals (participants signal low confidence, triggering review)
- automatic deprecation based on participation decay (domains with sustained low proposer/validator activity lose priority)
- challenge-based domain invalidation (a bonded challenge targeting the evaluation surface itself, not just individual blocks)

The design must avoid introducing discretionary truth selection — deprecation should be based on legible, auditable criteria, not subjective quality judgments. This is a hard constraint given the project's core invariant.

---

## Closing View

The project is built on the assumption that attacks are not anomalies. They are the normal condition of any reward-bearing system.

The protocol succeeds only if:
- false claims are visible,
- challenge is cheap enough,
- honest replay is worth doing,
- exploit strategies are costly to sustain,
- and robust improvements survive pressure better than weak ones.

That is the attack model in one sentence.