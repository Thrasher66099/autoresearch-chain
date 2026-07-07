<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Funding Economics Design

**Status:** Design, not implemented. Written 2026-07 (pre-testnet). This
document settles the funding-economics decisions that the testnet genesis
format will freeze. Mechanism parameters marked *calibratable* flow into
the Phase 4 adversarial-simulation harness before launch.

## The problem this design must solve

The research output is open (AGPL). Its beneficiaries do not have to pay
for it, which is the classic failure mode of useful-work chains: rewards
funded purely by money-printing, token value sustained purely by
speculation, economics that never close. This design closes the loop by
making **the parties who want research done fund the miners** — the
research-bounty model — while using a strictly bounded emissions subsidy
to solve the cold-start problem.

## Token

- Native token (working name **ARC**), required for every protocol bond,
  fee, and reward. Demand is demand for participation: posting funded
  research domains, bonding blocks and challenges, paying validation.
- **Fair launch.** No premine, no team or investor allocation. Tokens
  enter existence only through (a) the emissions subsidy schedule and
  (b) a one-time genesis auction whose entire proceeds escrow the first
  community bounty pools. Development is unfunded by the protocol —
  stated plainly rather than laundered through a treasury.

## Funding model: hybrid, bounty-first

### Research bounties (the permanent mechanism)

A genesis proposal escrows a **reward pool** alongside its seed bond:

- `reward_pool`: funds all staged block rewards for the domain
  (provisional and survival tranches, and later integration/frontier
  stages). `base_block_reward` becomes a per-domain genesis field paid
  from this pool, not a protocol constant.
- `validation_reserve` (*calibratable*, default 25% of pool): reserved
  for validation and adversarial work — per-attestation validator pay
  and standing challenger incentives on the domain.
- Pools are **permissionlessly top-uppable** by anyone, any time. A
  domain whose pool cannot cover one full block reward plus its
  validation costs enters **Dormant** status: no new block submissions
  accepted (existing lifecycle completes) until topped up. Dormancy is a
  legible arithmetic condition — no discretion.
- Pool funds are spent, never refunded to the creator (refundability
  would enable bounty-griefing: attract miners, withdraw the pool).
  A deprecated domain (§18 challenge) burns its remaining pool.

### Emissions subsidy (the bounded bootstrap)

- Total subsidy is **hard-capped** (*calibratable*; on the order of
  10–20% of intended eventual supply) with a halving-style decay.
- The subsidy **matches bounty funding rather than replacing it**: an
  eligible accepted-and-settled block earns `min(subsidy_rate ×
  bounty_reward_paid, remaining_epoch_subsidy)`. Matching (rather than
  flat per-block emission) means subsidy flows only to domains someone
  already valued enough to fund, which blunts wash-mining.
- **Wash-mining defense** (self-bounty to farm subsidy): matching means
  a self-funder pays ≥ 1/subsidy_rate of every token minted to
  themselves and loses the pool spend to other miners, validation
  reserve, and burns; per-domain and per-epoch subsidy caps
  (*calibratable*) bound residual extraction. This must be a named
  scenario in the adversarial harness before launch.
- End state: when the cap exhausts, the chain is pure-bounty. The design
  target (*calibratable*) is bounty income exceeding subsidy income
  within the first two halvings.

## Validation and challenger compensation

Calibration finding: honest validation is -EV unpaid, and fees alone pay
lazy validators equally. Therefore:

- **Proposer fees** (already a block field) are paid to the block's
  assigned validators, contingent on truth-bearing attestations.
- The domain's **validation reserve** funds standing audit incentives:
  per-epoch challenger retainers and/or challenge-bond matching
  (*mechanism calibratable*), so quiet domains still have adversarial
  eyes on them.
- **Attestation slashing is a hard prerequisite**: validators bond at
  registration; an upheld attestation challenge slashes the attester.
  Without this, any fee scheme rewards laziness (specified in
  protocol-v0.2 challenge targets; implementation required before
  testnet rewards are real).
- Bond-at-submission (flagged in the calibration report) ships with
  this: block and challenge bonds commit when submitted, so rejected
  fraud is no longer free.

## Slashed funds and fee residuals: burn

The challenger's payout share is unchanged (50% default, calibratable).
All residuals — slash remainders, deprecated-domain pools, dormancy
round-off — are **burned**. No protocol treasury: a treasury is a
discretionary honeypot, and discretionary spending is the same failure
mode as discretionary truth. Public-goods funding, if the community
wants it, belongs in voluntarily funded bounty domains, not protocol
plumbing.

## Failure modes considered

- **Pool exhaustion**: legible dormancy + permissionless top-up; no
  stranded miners (submissions refused upfront, not unpaid afterward).
- **Bounty griefing**: pools are non-refundable; seed bond and RTS
  conformance already gate garbage domains; §18 challenge kills
  degenerate ones and burns their pools.
- **Subsidy addiction**: hard cap + matching structure; the handoff is
  an explicit design target measured on testnet, not a hope.
- **Speculation dominance**: cannot be prevented, only made
  non-load-bearing — every protocol function prices in ARC against real
  work costs, so utility demand exists at any price.
- **Wash-mining**: see matching + caps above; named adversarial-sim
  scenario before launch.

## Genesis format changes (freeze list for testnet)

`GenesisBlock` gains: `reward_pool: TokenAmount`,
`validation_reserve_bps: u16`, `base_block_reward: TokenAmount`.
Protocol state gains per-domain pool accounting (balance, reserve
balance, dormancy flag) and global subsidy accounting (remaining cap,
epoch schedule). `RewardConfig`'s global `base_block_reward` becomes the
default for unfunded test states only.

## Implementation order (feeds Milestone E4)

1. Per-domain reward pools + dormancy — **implemented** (`DomainPool`
   in the simulator: reserve split at activation, debit at acceptance,
   arithmetic dormancy gate at submission, permissionless
   `top-up-pool`; unfunded domains keep legacy global-config behavior).
2. Bond-at-submission + attestation slashing — **implemented** (bond escrow committed in `submit_block`, released on rejection — slashing requires adjudication, never a vote tally; validators post registration bonds when `validator_bond > 0`, slashed by upheld attestation challenges with the standard challenger payout split, leaving the block itself governed by its own challenges).
3. Proposer-fee distribution — **implemented** (at evaluation, the block fee splits equally among validators who actually attested, pass or fail — the fee pays replay work, not agreement; laziness is deterred by attestation slashing. Division remainder burned; recorded as auditable `FeePayout` entries).
4. Emissions subsidy — **implemented** (settled blocks on funded domains mint `subsidy_rate x base_block_reward` to the proposer; rate halves every `subsidy_halving_epochs`; per-epoch and lifetime hard caps; zero cap = disabled/pure-bounty; unfunded domains never earn subsidy, so matching structurally requires a bounty).
5. Adversarial-sim scenarios: wash-mining, pool exhaustion, validator
   fee/slash equilibrium; recalibrate defaults.
6. Genesis auction mechanics (testnet: faucet stands in).
