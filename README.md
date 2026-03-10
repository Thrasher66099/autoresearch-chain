<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# AutoResearch Chain

**A fully decentralized, fork-native Proof-of-Useful-Work protocol for mining validated improvements to AI training recipes.**

---

## What This Is

AutoResearch Chain is a protocol for coordinating GPU owners, AI agents, validators, and challengers around a shared competitive research game.

Instead of spending energy on arbitrary hash puzzles, participants use their GPUs to:

- propose improvements to AI training recipes,
- validate improvements discovered by others,
- challenge invalid or fraudulent claims,
- compete across forks,
- and converge on better training code over time.

The useful work being mined is measurable progress in AI training methodology.

Blockchain is **not** the source of value.
The value is the research output.
Blockchain is the mechanism that makes a **trustless, permissionless, auditable market** for that research possible.

## What This Is Not

- Not a generic "AI + blockchain" project. This is a mechanism-design project for trustless AI research markets.
- Not a centralized platform with blockchain branding. Full decentralization is non-negotiable.
- Not a claim that the full decentralized training layer already exists. The current protocol covers research discovery; decentralized long-horizon training is future work.
- Not a wrapper around someone else's code. The protocol is designed around `autoresearch`-style agent loops, but does not vendor or redistribute that code.

## Current Scope

The protocol currently specifies a decentralized **research-discovery layer**:

- Decentralized submission of recipe improvements
- Permissionless bonded validation via replay
- Adversarial challenge and falsification
- Fork-native parallel search
- Escrowed, staged rewards
- Scale-validation hooks
- Agent-driven Stage 1 research (designed around [`karpathy/autoresearch`](https://github.com/karpathy/autoresearch) or similar autonomous research loops)

## Future Scope

The protocol does not yet fully specify:

- Stage 3 decentralized long-horizon swarm training
- Full gradient attestation
- Sustained compute-contribution accounting

These are compatible with the design but require separate formal specification. See [Future: Stage 3 Training](docs/future-stage-3-training.md).

## Relationship to `autoresearch`

The Stage 1 mining loop is explicitly modeled on autonomous research agents like Andrej Karpathy's [`autoresearch`](https://github.com/karpathy/autoresearch). In that paradigm, an AI agent:

1. Reads the current training recipe
2. Modifies the training code
3. Runs a short GPU experiment
4. Measures the result
5. Keeps improvements, discards failures, repeats

AutoResearch Chain wraps this loop in a decentralized protocol: successful diffs become candidate blocks, validators replay transitions, and the chain records validated progress.

No code from `autoresearch` is currently included in this repository. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Design Principles

1. Parallel search is first-class
2. Validation is provisional
3. Claims must be challengeable
4. Rewards follow survival through falsification
5. Decentralization is non-negotiable
6. Governance may tune the game, but not decide truth
7. Forks are native
8. Cross-fork synthesis is desirable
9. Ancestry alone should not produce rent
10. Useful work, not arbitrary hashing, is the core mining primitive

## Status

| Area | Status |
|------|--------|
| Agent-driven Stage 1 recipe search | Specified |
| Block submission and evidence bundles | Specified |
| Replay-based validation | Specified |
| Fork-native competition | Specified |
| Challenge-based falsification | Specified |
| Scale-validation hooks | Specified |
| Stage 3 swarm training protocol | Not yet specified |
| Gradient attestation system | Not yet specified |
| Long-horizon compute accounting | Not yet specified |
| Reference implementation | Not yet started |

## Documentation

| Document | Description |
|----------|-------------|
| [Docs Index](docs/README.md) | Full documentation navigation |
| [Executive Summary](docs/executive-summary.md) | Short polished overview |
| [White Paper](docs/whitepaper.md) | Full protocol thesis and rationale |
| [Protocol v0.2](docs/protocol-v0.2.md) | Technical protocol specification |
| [Project Scope](docs/project-scope.md) | Current vs. future scope |
| [Future: Stage 3 Training](docs/future-stage-3-training.md) | Decentralized training layer (future work) |
| [Terminology](docs/terminology.md) | Glossary of protocol terms |
| [Licensing](docs/licensing.md) | Split-license model explained |

## Repository Structure

```
README.md                          Project landing page
LICENSES.md                        License policy overview
LICENSE-AGPL                       AGPL-3.0-or-later notice
LICENSE-CC-BY-SA                   CC BY-SA 4.0 notice
NOTICE                             Attribution and provenance
THIRD_PARTY_NOTICES.md             Third-party dependencies
CONTRIBUTING.md                    Contribution guide
.gitignore                         Git ignore rules
docs/
  README.md                        Docs index
  executive-summary.md             Short project overview
  whitepaper.md                    Full white paper
  protocol-v0.2.md                 Technical protocol spec
  project-scope.md                 Scope and staging
  future-stage-3-training.md       Stage 3 future work
  licensing.md                     Licensing explanation
  terminology.md                   Glossary
templates/
  code-file-header.txt             AGPL header for source files
  documentation-file-header.txt    CC BY-SA header for docs
```

## License

This repository uses a split-license model:

- **Code** (protocol implementations, validators, clients): [AGPL-3.0-or-later](LICENSE-AGPL)
- **Documentation** (specs, white papers, research writing): [CC BY-SA 4.0](LICENSE-CC-BY-SA)
- **Third-party code**: original upstream licenses apply

See [LICENSES.md](LICENSES.md) for the full policy and [docs/licensing.md](docs/licensing.md) for a plain-language explanation.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

*AutoResearch Chain is an early-stage protocol design. No reference implementation exists yet. The current repository contains the protocol specification, design rationale, and project infrastructure.*
