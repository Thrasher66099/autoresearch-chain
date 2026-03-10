# Protocol Open Questions

## Purpose

This document lists the major unresolved or partially resolved questions in the AutoResearch Chain design.

The project is ambitious and should be honest about what remains open.

---

## 1. Validation Thresholds

How should the protocol choose:
- validator count
- pass thresholds
- confidence scoring
- replay depth
- fraud veto conditions

These are central to whether the system rewards real gains or noise.

---

## 2. Benchmark Quality

How do we ensure the benchmark or evaluation logic remains aligned enough with what we actually care about?

This is one of the deepest unsolved issues.

A weak benchmark can corrupt the entire market.

---

## 3. Attribution Quality

How accurate does origin/integration/frontier attribution need to be for contributors to trust the system?

Perfect attribution may be impossible.
But “too weak to be trusted” is also not acceptable.

---

## 4. Genesis and Domain Creation Policy

The protocol specifies permissionless genesis with economic filtering through seed bonds, RTS conformance, and validator reproduction. Key open questions remain:

- What is the right seed bond size? Too low encourages spam; too high suppresses experimentation.
- What are the right activation thresholds for minimum validator participation and bonded commitment?
- How should seed bond slashing scale with failure reason (unreproducible score vs. missing artifacts vs. insufficient participation)?
- Should there be cooldown periods or rate limits on genesis proposals from the same proposer?

---

## 5. Cross-Domain Reward Policy

How much reward should stay local to a source domain versus flow to integrators or downstream parent domains?

This is a live design question.

---

## 6. Fork Economics

What is the best economic structure for:
- unresolved branch competition
- fork timeout
- convergence timing
- dominance margins

Forks are essential, but poorly tuned fork economics can create chaos.

---

## 7. Challenge Market Tuning

How large should challenge bonds be?

Too small:
- griefing becomes cheap

Too large:
- valid challenges may be suppressed

Challenge economics are critical to truth-seeking.

---

## 8. Validator Collusion Resistance

How much collusion can the system tolerate before validation legitimacy breaks down?

This depends on:
- validator sampling
- bond size
- evidence legibility
- challenger depth

---

## 9. Scale-Stage Provisioning

How do we guarantee enough participation in Stage 2 larger-scale validation?

Everyone benefits from transfer testing, but the cost is concentrated.

This is both a mechanism-design and provisioning problem.

---

## 10. Canonical Frontier Materialization

What policies should trigger materialization of full code states?

Possible triggers:
- dominance
- depth thresholds
- time intervals
- manual epoch boundaries

The choice affects usability and reproducibility.

---

## 11. Agent Policy Surface and Search/Frozen Surface Boundaries

How much freedom should autonomous research agents have inside a domain?

The protocol specifies that each genesis block declares a search surface and frozen surface. Open questions include:

- How granular should surface declarations be (file-level, module-level, function-level)?
- How should edge cases between search and frozen surfaces be adjudicated?
- Should the protocol support dynamic surface expansion through governance or successor tracks?

This affects search breadth, validation complexity, and attack surface.

---

## 12. Long-Horizon Training Layer

How should the eventual Stage 3 shared-training protocol work?

This includes open questions around:
- contribution proofs
- synchronization
- checkpoint control
- node dropout
- training fraud
- live recipe updates

This is future work, but it matters to the long-term vision.

---

## 13. Governance Constitution

What governance parameters should be mutable and which should be constitutionally hard to change?

This question matters for preserving decentralization over time.

---

## 14. Practical User Experience

How should real users discover domains, pull frontier states, run agents, validate blocks, and file challenges in a way that is practical enough to attract participation?

Protocol elegance is not enough.
Usability matters.

---

## 15. Economic Sustainability

How should rewards be funded and sustained across:
- domain-local work
- integration work
- validation
- challenge
- scale-stage testing

This is one of the core economic design questions.

---

## Why This File Exists

A protocol can be ambitious without pretending to be finished.

This file exists so the project remains intellectually honest:
- some parts are well framed
- some parts are still live research problems
- and acknowledging that is a strength, not a weakness