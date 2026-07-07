<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Milestone Plan: Path to a Functioning Proof-of-Useful-Work Chain

**Status:** Active execution plan. Written 2026-07 at `main` commit `d274600`.

This document is the working plan for carrying AutoResearch Chain from its
current state (a locally-complete protocol game) to a functioning, useful,
networked Proof-of-Useful-Work chain. It is written so that a contributor —
human or agent — with no prior context can pick up any milestone and execute
it correctly.

It complements `implementation-plan.md` (architecture and phase definitions)
and `protocol-v0.2.md` (normative protocol behavior). Where this document and
the protocol spec disagree, the spec wins; where implementation reveals a spec
defect, fix the spec explicitly — never silently deform the implementation.

---

## Read This First

1. **The core invariant.** Every decision must preserve a fully decentralized,
   trustless, adversarial market for useful AI research work. No discretionary
   truth, no centralized scheduler, no benchmark-platform drift. See the
   repository `AGENTS.md` and `docs/governance-boundaries.md`.
2. **Ordering principle.** Prove the game is sound before paying for
   decentralization; prove the work is useful before inviting strangers.
   Do not start a later milestone to avoid a hard problem in an earlier one.
3. **Protocol truth discipline.** Protocol truth derives exclusively from
   validator-observed data (`ValidatedBlockOutcome`), never proposer claims.
   `DerivedValidity` (`DirectValid` / `DirectInvalid` / `AncestryInvalid`)
   gates all downstream decisions: settlement, frontier, dominance, escrow.
   Any new mechanism must respect this gate.
4. **Working conventions.** Rust: `cargo build --workspace`,
   `cargo test --workspace`. Python: `cd python && python -m pytest` and
   `python -m ruff check .` (integration tests need a built `arc-node`).
   Land work through feature branches and PRs to `main`. Each protocol change
   lands with tests at every layer it touches (engine, simulator, node CLI,
   Python client). Do not overclaim maturity in docs; update
   `implementation-plan.md` status sections when a milestone lands.
5. **Where things are.** Rust protocol core in `crates/` (10 crates; see
   `implementation-plan.md` for the status table). Python runners in
   `python/arc_runner/`. Real-computation demo:
   `python -m arc_runner.demo` on the QMD domain (`fixtures/qmd/finetune/`).

## Current State (as of milestone plan creation)

Implemented and tested end-to-end locally:

- Deterministic protocol state machine (`simulator` composing all engines):
  genesis activation, block lifecycle, truth-bearing validation, challenge
  adjudication (open → review → upheld/rejected), fork/frontier settlement,
  minimal escrow (create/release/slash on the proposer's block bond).
- Node CLI (`arc-node`): 17 write commands, 5 read commands, file-based
  state persistence.
- Python runners: proposer, validator, challenger; BLAKE3 evidence bundling
  with verified Python↔Rust hash agreement; autoresearch adapter with
  frozen/search-surface enforcement; a real-computation fraud-detection test
  (challenger replay catches an inflated score; upheld challenge invalidates
  the block, slashes escrow, reverts the frontier).

Not implemented (the gaps this plan closes):

- Challenger bond economics and staged reward release (Milestone A)
- Frontier materialization beyond genesis workspaces (Milestone B)
- Multi-actor adversarial simulation and parameter calibration (Milestone C)
- A genuinely useful research domain (Milestone D)
- Cryptographic identity, networking, consensus, testnet (Milestone E)
- Token/funding economics design (parallel track)

---

## Milestone A — Complete the Incentive Spine

**Status: COMPLETE (2026-07).** Challenger bond escrow with payout on upheld
(50% of slashed funds by default, residual burned, recorded as a
`SlashDistribution`), forfeiture on rejected, return on expired; staged
provisional/survival reward tranches with the fraud-exposure invariant
(`bond >= provisional`) enforced at acceptance. Exercised end-to-end in
Rust scenario/integration tests and Python integration tests, including the
real-computation fraud scenario asserting proposer net loss and challenger
net gain. Defaults are placeholders pending Milestone C calibration.

**Goal:** make the adversarial market economically real. Challenging must be
profitable when correct and costly when wrong; rewards must pay for survival
through falsification, not for claiming.

**Why first:** every later milestone (especially the adversarial simulation)
tests incentives that do not currently exist. Closes out Phase 0.

### A1. Challenge bond economics

Current gap: `ChallengeRecord.bond` is recorded but has no lifecycle. An
upheld challenge slashes the proposer's escrow, but the challenger receives
nothing; a rejected challenge costs the challenger nothing. The spec
(`protocol-v0.2.md` § Challenges) requires: failed challenge → challenger
loses bond; successful challenge → protocol may redirect escrow.

Deliverables:

- Challenger bond escrow: created when a challenge opens, held during
  adjudication.
  - **Upheld:** challenger bond returned; a configured fraction of the
    slashed proposer escrow is redirected to the challenger as payout; the
    residual is recorded as burned (no treasury exists in Phase 0 — record
    the residual explicitly so the accounting is auditable).
  - **Rejected:** challenger bond slashed (forfeited).
  - **Expired:** challenger bond returned. Rationale: expiry means the
    protocol failed to adjudicate, not that the challenge was wrong;
    punishing unresolved challenges would chill challenging. Document this
    in the spec when implementing.
- Slash distribution record (who was slashed, payout recipient, payout
  amount, burned residual) persisted in simulator state and queryable via
  the node CLI.
- Configuration in `RewardConfig` (e.g. challenger payout fraction), not
  hardcoded — Milestone C calibrates the numbers.

### A2. Staged reward release

Current gap: only the proposer's bond is escrowed. There is no reward, and
no staging. The spec defines five stages; only the first two are
implementable now:

- **Provisional reward** (on acceptance) and **survival reward** (on
  settlement after the challenge window) — implement now. A clean approach:
  a per-block reward amount (configurable `base_block_reward`) escrowed as
  two tranches with different release epochs, reusing the existing
  `EscrowRecord` machinery. Both tranches slash if a challenge is upheld
  before release.
- **Integration / frontier / transfer rewards** — deferred: they depend on
  cross-fork porting mechanics, attribution, and Stage 2 scale validation
  respectively. Leave the enum/config surface open for them; do not fake
  them.

### A3. Wiring and surfaces

- `reward-engine`: new escrow constructors and transitions, with unit tests.
- `simulator`: wire challenge escrow into `open_challenge` /
  `uphold_challenge` / `reject_challenge` / expiry; wire staged tranches into
  acceptance and settlement. Scenario tests for every path.
- `arc-node`: expose new state via queries (escrows, slash distributions);
  adjust write commands only if new inputs are genuinely required.
- Python client/runners + integration tests: challenger economics visible
  end-to-end (fraud scenario should now show the challenger being paid).

**Definition of done:** a dishonest proposer loses bond and both reward
tranches; the successful challenger nets a positive payout; a frivolous
challenger nets a loss; every flow is asserted in Rust scenario tests and
Python integration tests; all existing tests still pass.

---

## Milestone B — Frontier Materialization (Phase 3)

**Status: COMPLETE (2026-07).** Implemented in `arc_runner/materialize`:
content-addressed state manifests (sorted path → BLAKE3 hash, per-file
storage), structured state diffs, verified assembly (a workspace that does
not hash back to its reference raises `MaterializationError`), and
diff-chain resolution from the genesis seed. Genesis seeds are packaged as
manifests; `pull_frontier(state_ref)` materializes any block's
`child_state_ref`; `capture_result` snapshots the post-experiment codebase
and computes the parent diff. The QMD demo and tests run two generations
where generation 2 builds on generation 1's verified state with frozen
surfaces intact and the full diff chain resolving to the tip.

Two deliberate decisions recorded:

1. **Structured diffs, not unified-diff patching.** Diffs are canonical
   JSON objects (parent ref, child ref, changed path→hash map, deleted
   paths). Application is deterministic (no context matching or fuzz),
   every step verifies against the declared child state, and unchanged
   content deduplicates in the store. Textual diffs remain in evidence
   bundles for human review only. This supersedes this plan's earlier
   suggestion of unified diffs.
2. **Protocol bug found and fixed:** frontier selection and dominance
   compared per-block validated deltas, which are relative to each block's
   own parent and therefore incomparable across generations — no child
   could displace a parent with a larger delta. Both now compare
   cumulative validated improvement from the domain seed
   (`SimulatorState::cumulative_validated_delta`). Found by the first
   two-generation run; the simulator-first discipline working as intended.

**Goal:** make multi-generation research real. A proposer must be able to
pull the canonical frontier of a domain — not just the genesis seed — as an
assembled workspace, so block N+1 builds on block N's accepted state.

Current gap: `python/arc_runner/materialize/` is a stub;
`AutoresearchAdapter.pull_frontier()` only extracts genesis-based workspaces.
The Rust `storage-model` stores content-addressed artifacts but does not
resolve reference chains into assembled states.

Deliverables:

- Diff-chain resolution: given a frontier block, resolve
  `seed_codebase_state_ref` + the chain of `diff_ref`s into a materialized
  workspace. Decide and document the diff format (unified diff applied with
  strict context matching is the simplest defensible choice).
- Full-snapshot support (`child_state_ref` as complete state) as the
  fallback/checkpoint path, per `protocol-v0.2.md` § MaterializedState.
- Materialization verification: the assembled workspace must hash-verify
  against the block's `child_state_ref` commitment. A workspace that does
  not verify is a protocol violation, not a warning.
- Frozen-surface enforcement post-materialization (a diff chain must not be
  able to smuggle changes into the frozen surface).
- Multi-generation integration test: genesis → block 1 accepted → proposer
  pulls block-1 frontier → block 2 builds on it → accepted → frontier is
  block 2. Extend the QMD demo to two generations.

**Definition of done:** the QMD demo runs two generations of improvement
where generation 2's workspace is materialized from generation 1's accepted
block, verified against its commitment, with frozen surfaces intact.

---

## Milestone C — Adversarial Simulation and Calibration (Phase 4)

**Status: COMPLETE (2026-07).** Harness in `crates/adversarial-sim`
(strategy-parameterized actors over the real `SimulatorState`,
deterministic seeds, EV accounting from protocol escrows + slash
distributions + compute costs); 11 scenario tests; calibration report in
`simulations/calibration-report.md`. Key outcomes:

- Fraud is +EV at naive parameters (bond 500 vs reward 1000 needs ~80%
  audit coverage); calibrated set (bond ≥ 2× reward, provisional 10%)
  brings break-even coverage to ~35%. `recommended_world()` encodes it.
- The challenge game is the binding security layer: an auditor at 50%
  coverage defeats fraud even with a fully lazy validator pool.
- **Noise mining forced a protocol change**: sub-tolerance claims were
  unfalsifiable free money; `ValidationConfig::min_accepted_delta` now
  gates acceptance on validated improvement clearing a threshold
  calibrated above the tolerance band.
- Flagged gaps for later milestones: validator compensation (fees alone
  reward laziness; attestation-level slashing needed) and
  bond-at-submission (rejected fraud currently costs nothing).
- §18 mechanism specified (see below); abstract waste model confirms it
  bounds honest waste. Economic stress tests and successor-track
  scenarios remain future Phase 4 work.

The §18 **evaluation-surface challenge** is specified in
`protocol-v0.2.md` § Evaluation-Surface Challenges and `attack-model.md`
§18: a bonded challenge carrying a budget-bounded demonstration generator
(RTS-declared source/compute limits), adjudicated by pure validator
replay — upheld ⇒ domain Deprecated, seed bond slashed with challenger
share; rejected ⇒ challenger bond forfeited. No discretionary truth.
Implementation is future work.

**Goal:** subject the incentive system to scripted adversaries and calibrate
protocol parameters. This is the project's actual scientific test: does
honest play dominate under the implemented economics?

Deliverables (build in `simulations/`, driving the simulator directly in
Rust or via the Python client — choose per-scenario; Rust is faster for
thousands of episodes):

- Actor framework: strategy-parameterized proposers, validators, and
  challengers with per-participant balance accounting across episodes.
- Scenario library keyed to `docs/attack-model.md`, at minimum:
  - lazy validators (rubber-stamp Pass without replay)
  - proposer–validator collusion rings
  - noise miners (many cheap submissions hoping tolerance-band luck)
  - bond-griefing / frivolous challengers
  - ancestry farming (trivial intermediate blocks)
  - degenerate evaluation surface (§18) — see below
- Measurement: expected value per strategy per episode. The output that
  matters: parameter regions where every dishonest strategy has negative
  expected value while honest participation is profitable.
- Calibration: bond sizes, challenger payout fraction, tolerance bands,
  validators per block, challenge window length. Record chosen values and
  the evidence in a `simulations/` report; update defaults in code.
- **§18 mechanism design:** the degenerate-evaluation-surface attack has no
  mitigation mechanism yet (acknowledged in `attack-model.md` §18). Use the
  simulation harness to evaluate candidate designs (validator-quorum domain
  review, stake-weighted deprecation, participation decay). Hard constraint:
  legible, auditable criteria only — no discretionary truth. Specify the
  chosen mechanism in the protocol spec before implementing.

**Definition of done:** a reproducible simulation suite demonstrating
honest-play dominance under calibrated defaults, a written calibration
report, and a specified (not necessarily implemented) §18 mechanism.

---

## Milestone D — A Genuinely Useful Research Domain

**Goal:** prove the chain mines something valuable, before networking.
QMD query-expansion is a correctness toy; this milestone runs a real ML
research track.

Recommended track: a nanoGPT-speedrun-style RTS-1 domain — language-model
loss (e.g. `val_bpb`) on a fixed public dataset, fixed time budget, single
consumer GPU class. This shape has an existence proof of value (the speedrun
community produced optimizer and recipe improvements that transferred to
serious training runs).

Deliverables:

- Domain fixture under `fixtures/` with real `train.py` / `eval.py` /
  frozen harness; genesis packaged via the existing RTS-1 flow.
- Confront replay nondeterminism honestly: GPU nondeterminism, floating
  point, hardware variance. Deliverables here are empirical: measured replay
  variance on real hardware, and tolerance bands set from that measurement
  (feeding back into Milestone C calibration). If exact-replay determinism
  is achievable (fixed seeds, deterministic kernels), prefer it and document
  the constraints it imposes on the search surface.
- Run the full loop on 2–3 real machines operated by the project (proposer,
  validators, challenger) for multiple generations. This is a single-operator
  rehearsal, not decentralization — say so plainly in any write-up.
- An honest assessment document: did the track produce improvements a
  competent ML practitioner would consider real? What was the noise rate?

**Definition of done:** a multi-generation run on real hardware in a real ML
domain, with measured replay tolerances and an honest utility assessment.

---

## Milestone E — Identity, Then Network (Phase 5)

**Goal:** a minimal real testnet. Everything before this milestone is
single-operator; this is where trustlessness becomes real.

**Identity comes first.** The current implementation has no cryptography on
transactions: `ParticipantId` is raw bytes, nothing is signed, anyone can
submit anything as anyone. Retrofitting signatures after networking is far
harder than before it. Sub-milestone E1 (signing) can be built directly
after Milestone A if contributor bandwidth allows — it has no dependency on
B–D.

Deliverables:

- **E1 — Signing and identity: COMPLETE (2026-07).** Ed25519 via
  `crates/identity`: a participant's ID **is** their public key, so
  verification is self-contained. Signatures are computed over
  deterministic domain-separated message strings (versioned type tag +
  `|`-joined fields; floats as f64-bit hex) — never over serialized JSON,
  whose float formatting differs across languages. Actor-bearing
  submissions (genesis, block, attestation, seed validation, challenge)
  carry a sibling `signature` field verified at the node boundary;
  enforcement is opt-in per state (`arc-node init --require-signatures`,
  intended ON for any testnet genesis) and any signature present is
  verified even in legacy mode. `arc-node keygen` generates keypairs;
  `arc_runner.identity` mirrors the message builders (pinned
  cross-language test vectors + a Python-signs/Rust-verifies integration
  test). Remaining for later: signing adjudication/registration commands
  (no actor field yet) and key management beyond flat files.
- **E2/E3 — Ordering + networking: COMPLETE (2026-07), as explicit scaffolding.**
  Single-sequencer PoA over plain HTTP (`arc-node serve` / `follow`):
  every state mutation flows through one `apply_tx` dispatcher and a
  hash-chained, authority-signed ordering log (JSONL beside the state
  file). Followers fetch `/log`, verify chain + authority signature +
  per-actor signatures on replay, and cross-check a canonical BLAKE3
  state hash against the sequencer — divergence is a hard error.
  Artifacts serve content-addressed via `/artifact`. Trust model stated
  plainly: the authority can censor but cannot forge, reorder
  undetectably, or fabricate actor signatures. libp2p/gossip and
  permissionless ordering are deliberately deferred; the transaction and
  state model do not change when the ordering layer is replaced.
  Follow-ups: Python HTTP client, follower serving read endpoints,
  sequencer rotation.

- **E2 — Ordering layer (original goal):** the useful-work game does not order the chain; a
  deliberately boring mechanism does. Start with the simplest thing that is
  honest about its trust model (a small permissioned validator set / PoA
  federation), explicitly documented as temporary scaffolding to be replaced
  by permissionless bonded ordering. Do not let ordering-layer ambition
  block the testnet.
- **E3 — Networking:** gossip (libp2p is the default choice in the Rust
  ecosystem), transaction mempool, state sync for new nodes, and the
  artifact/DA layer serving evidence bundles between real machines
  (content-addressed fetch by hash; availability challenges per the spec).
- **E4 — Testnet:** valueless faucet tokens, public genesis, at least one
  domain from Milestone D, external participants explicitly invited to
  attack it. Bugs found here are the point.

**Definition of done:** strangers on machines the project does not control
can propose, validate, and successfully challenge fraud, with signatures
enforced, on a persistent multi-node network.

---

## Parallel Track — Funding Economics (Design, Not Code)

The unresolved existential question: who funds rewards, and why does the
token have demand beyond speculation? This is spec work that should proceed
alongside Milestones A–D and must be settled before Milestone E4.

The leading candidate is the **research-bounty model**: domain creators
escrow a reward pool when opening a track, so the parties who want the
research done fund the miners. This makes the chain a decentralized
research-bounty market with adversarial QA — a real demand loop — and it
changes genesis mechanics (reward-pool escrow alongside the seed bond), so
it must be designed before testnet genesis formats freeze.

Deliverable: a design document (`docs/`) covering reward-pool escrow in
genesis, emission policy (if any) alongside bounties, fee flows, and an
honest analysis of failure modes (pool exhaustion, bounty griefing,
speculation dominance). Cross-check every choice against
`docs/governance-boundaries.md`.

---

## Explicitly Deferred (Do Not Build Yet)

- Stage 2 scale-validation markets and transfer rewards
- Stage 3 decentralized long-horizon training
- Cross-domain integration effects, successor tracks, attribution-weighted
  reward distribution
- Governance implementation, explorers, dashboards, wallets, EVM/bridges
- Any hosted/centralized convenience service that would deform the protocol

## Maintenance of This Document

When a milestone lands: update its status here, update
`implementation-plan.md`'s status sections, and record any spec changes in
`protocol-v0.2.md` (or successor). If implementation experience invalidates
part of this plan, rewrite the plan — an execution doc that no longer
matches reality is worse than none.
