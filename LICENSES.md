# Licensing

This repository uses different licenses for different classes of material.

The goal is to keep the core protocol and network-facing software in the commons, while making the written specification and research materials broadly shareable.

## Summary

### Core protocol and network-facing software
Licensed under **GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)**.

This includes, unless otherwise noted:

- protocol node implementations
- validator software
- challenger software
- reference execution clients
- network-facing APIs and services that are part of the protocol
- orchestration code used to operate the protocol
- protocol-critical web applications and dashboards
- reference agent integration code
- smart-contract or on-chain protocol logic, if included in this repository

### Documentation, white papers, specifications, and written materials
Licensed under **Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0)**.

This includes, unless otherwise noted:

- `README.md`
- white papers
- executive summaries
- protocol specifications
- design notes
- governance documents
- diagrams
- architecture explanations
- website copy stored in this repository
- non-code research writing

### Third-party code and dependencies
Third-party code remains under its original license.

Nothing in this repository relicenses third-party software.  
If a file, directory, submodule, vendored component, or dependency includes its own license, that license controls that material.

---

## Why this repo uses split licensing

AutoResearch Chain is intended to be a fully decentralized, trustless protocol for Proof of Useful Work in AI research.

Because of that, the project wants to prevent silent proprietary capture of the core protocol and network-facing implementations. For that reason, the core software is licensed under AGPL-3.0-or-later.

At the same time, the project wants the written ideas, protocol descriptions, and public-facing documentation to be widely shareable and remixable, provided attribution is preserved and derivatives remain open. For that reason, documentation is licensed under CC BY-SA 4.0.

---

## File-level default rule

Unless a file or directory explicitly states otherwise:

- **Code** is licensed under **AGPL-3.0-or-later**
- **Documentation and non-code text/media** are licensed under **CC BY-SA 4.0**

If there is any conflict between a file header and this document, the file header or local license notice takes precedence.

---

## Suggested repository layout

The following convention is recommended:

- `src/`, `node/`, `validator/`, `challenger/`, `contracts/`, `client/`, `server/`  
  → **AGPL-3.0-or-later**

- `docs/`, `whitepaper/`, `spec/`, `governance/`, `research/`  
  → **CC BY-SA 4.0**

- `third_party/`, `vendor/`, `external/`  
  → licensed according to their included upstream licenses

---

## Contributor intent

By contributing code to this repository, you agree that your contribution is licensed under **AGPL-3.0-or-later**, unless explicitly stated otherwise in the relevant path or file.

By contributing documentation, writing, diagrams, or other non-code materials to this repository, you agree that your contribution is licensed under **CC BY-SA 4.0**, unless explicitly stated otherwise in the relevant path or file.

---

## Third-party and upstream components

This repository may incorporate ideas from, integrate with, or be inspired by third-party projects, including autonomous research tooling such as `karpathy/autoresearch` or similar systems.

All third-party code, assets, and materials remain subject to their original licenses and notices.

If this repository includes any copied, modified, vendored, or derived third-party material, the maintainers should:

1. preserve original copyright notices
2. preserve original license notices
3. identify the upstream source
4. comply with all applicable attribution and redistribution requirements

Nothing in this repository should be interpreted as replacing or removing any third-party license obligations.

---

## Important note on `karpathy/autoresearch` or similar upstream tools

If this project uses, modifies, vendors, or derives code from `karpathy/autoresearch` or another upstream project:

- you must preserve the upstream copyright notice
- you must preserve the upstream license text
- you must clearly mark modified files
- you must comply with the terms of the upstream license for that code

Project inspiration alone does not trigger code-license obligations.  
Copying, modifying, vendoring, or redistributing code does.

Maintain a clear record in `NOTICE`, `THIRD_PARTY_NOTICES.md`, or equivalent if upstream code is incorporated.

---

## No implicit patent grant beyond the applicable license

This repository does not provide any patent rights beyond those granted, if any, by the applicable license governing the relevant material.

If stronger explicit patent language is desired for some parts of the ecosystem, those parts should be placed under a license that provides it and clearly marked.

---

## Trademark notice

The project name, logos, and branding, if any, are not automatically licensed for unrestricted trademark use by these copyright licenses.

Copyright licenses govern copying, modification, and distribution of creative works and software.  
Trademark rights, if any, are separate.

---

## How to apply the licenses in the repo

The repository should include at minimum:

- `LICENSE-AGPL`
- `LICENSE-CC-BY-SA`
- `LICENSES.md`

Optionally also include:

- `NOTICE`
- `THIRD_PARTY_NOTICES.md`

Recommended root-level structure:

```text
LICENSE-AGPL
LICENSE-CC-BY-SA
LICENSES.md
NOTICE
THIRD_PARTY_NOTICES.md