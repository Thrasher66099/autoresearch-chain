# Repository Guidelines

## Project Structure & Module Organization

AutoResearch Chain is a mixed Rust and Python workspace with documentation-first development. Most current content lives in `docs/`; read `docs/protocol-v0.2.md`, `docs/implementation-plan.md`, `docs/project-scope.md`, and `docs/attack-model.md` before changing protocol behavior. Rust crates live in `crates/` and are wired through the root `Cargo.toml`. Python runner code lives under `python/arc_runner/`. Use `fixtures/` for example inputs, `spec/` for normative specs, `simulations/` for scenario notes, and `templates/` for file headers.

## Core Engineering Rule

Do not let implementation convenience rewrite the protocol without noticing. Changes must preserve the project’s goal: a fully decentralized, trustless, adversarial market for useful AI research work. Reject shortcuts that turn the system into a benchmark platform, centralized scheduler, hosted leaderboard, or chain with off-chain discretionary truth.

## Spec Discipline

Use canonical terminology from `docs/terminology.md`. Prefer protocol terms such as `ProblemDomain`, `DomainSpec`, `CanonicalFrontierState`, `MaterializedState`, `ResearchTrackStandard`, `GenesisBlock`, `TrackTree`, `search surface`, and `frozen surface` over ad hoc synonyms. For spec-affecting edits, cross-check `docs/protocol-v0.2.md`, `docs/project-scope.md`, `docs/governance-boundaries.md`, `docs/attack-model.md`, and `docs/protocol-open-questions.md`.

## Build, Test, and Development Commands

Run commands from the repository root unless noted.

- `make check`: runs formatting, linting, and tests for Rust and Python.
- `make build`: builds the Rust workspace with `cargo build --workspace`.
- `make test`: runs `cargo test --workspace` and `cd python && python -m pytest`.
- `make fmt`: formats Rust with `cargo fmt --all` and Python with `ruff format`.
- `make lint`: runs `cargo clippy --workspace -- -D warnings` and `ruff check`.
- `cd python && pip install -e ".[dev]"`: installs Python dev tools locally.

## Coding Style & Naming Conventions

Follow Rust 2021 defaults and let `cargo fmt` decide layout. Python targets 3.10, uses Ruff, and keeps a 100-character line length. Use `snake_case` for Rust modules, files, and Python packages, and `CamelCase` for Rust types. Write serious technical prose: no hype, no vague blockchain language, and no overstatements about maturity. Docs should stay in plain Markdown with no front matter; use the SPDX comment for docs and the AGPL header template for code.

## Scope, Governance, and Integrity Rules

Label work accurately as `specified`, `partially specified`, `planned`, or `implemented`. Stage 1 recipe discovery is the current focus; Stage 2 is partial; Stage 3 decentralized training is future work and must not be implied as complete. Governance may tune parameters, but must not decide scientific truth, winning branches, payouts, or validation outcomes outside protocol-visible rules.

Preserve domain-local reward separation and the upstream integration rule: results from child or sibling domains do not automatically change broader domains. Upstream movement requires explicit cross-domain integration plus destination-domain validation. If you touch genesis, metrics, datasets, or evaluation harnesses, preserve immutable metric definitions, dataset availability and license clarity, and search-surface versus frozen-surface boundaries. Structural changes should follow successor-track logic rather than silent mutation.

## Testing Guidelines

Rust tests should live beside the crate they validate, either inline or under each crate's `tests/` directory. Python tests belong in `python/tests/`, matching the package or feature under test. No coverage gate is defined yet, but new logic should ship with deterministic tests. Run `make test` before opening a PR.

## Commit & Pull Request Guidelines

Recent history uses short, imperative subjects like `Add Phase 0 scaffold` and `Update implementation plan`. Keep commits focused and descriptive. For pull requests, include a concise problem statement, affected paths or crates, linked issues for protocol changes, and notes on tests run. If a change alters specs or docs structure, update `docs/README.md` in the same PR. Keep architecture changes aligned with the current direction: Rust-native custom chain first, operator interfaces later, and heavy artifacts off-chain with on-chain commitments.

## Review Heuristics

Evaluate protocol and economic changes against the attack model, especially benchmark overfitting, validator laziness or collusion, fraudulent evidence, ancestry farming, branch spam, domain spam, governance capture, and canonical frontier poisoning. Preserve the usability invariant from `docs/user-workflows.md`: for any active domain, participants should be able to discover it, inspect metadata, pull a canonical frontier, reproduce work, and submit, validate, challenge, or integrate results.

## Security & Contribution Notes

Do not add undocumented third-party code. Preserve upstream notices and record them in `THIRD_PARTY_NOTICES.md`. Keep claims about implementation maturity accurate: this repository is still early-stage, so distinguish clearly between implemented behavior, scaffolding, and future work.
