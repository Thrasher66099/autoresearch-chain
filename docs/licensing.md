<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Licensing

This document explains the AutoResearch Chain split-license model in plain language.

For the formal policy, see [LICENSES.md](../LICENSES.md) in the repository root.

---

## Why Split Licensing?

AutoResearch Chain uses two different licenses for two different kinds of material:

1. **Code** is licensed under AGPL-3.0-or-later to keep the core protocol software in the commons.
2. **Documentation** is licensed under CC BY-SA 4.0 to make the ideas widely shareable while requiring attribution.

This split reflects the project's goals: the protocol software should remain open and forkable, and the written ideas should be freely remixable.

## What Each License Means

### AGPL-3.0-or-later (Code)

The [GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.html) applies to:

- Protocol node implementations
- Validator software
- Challenger software
- Reference execution clients
- Network-facing APIs and services
- Orchestration code
- Smart-contract or on-chain logic
- Reference agent integration code

**In plain terms:** You can use, modify, and redistribute the code. If you modify the code and make it available over a network, you must also make your modified source code available under the AGPL. This prevents proprietary capture of the core protocol.

### CC BY-SA 4.0 (Documentation)

The [Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/) license applies to:

- White papers
- Protocol specifications
- Executive summaries
- Design notes
- Research writing
- Diagrams and architecture explanations
- This documentation

**In plain terms:** You can copy, redistribute, remix, and build on the written materials for any purpose, including commercially. You must give attribution. If you create derivative works, you must distribute them under the same license.

### Third-Party Code

Third-party code, dependencies, vendored components, and external assets remain under their original licenses. Nothing in this repository relicenses third-party software. See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

## Default Rules

Unless a file or directory explicitly states otherwise:

- **Code files** are licensed under **AGPL-3.0-or-later**
- **Documentation and non-code files** are licensed under **CC BY-SA 4.0**

If a file header and the repository-level policy conflict, the file header takes precedence.

## How to Apply Headers

File header templates are provided in the `templates/` directory:

- `templates/code-file-header.txt` — for source code files
- `templates/documentation-file-header.txt` — for documentation files

## Contributing

By contributing code, you agree it is licensed under AGPL-3.0-or-later unless explicitly marked otherwise. By contributing documentation, you agree it is licensed under CC BY-SA 4.0 unless explicitly marked otherwise.

## Note on License Texts

The `LICENSE-AGPL` and `LICENSE-CC-BY-SA` files in the repository root are currently short-form placeholder notices that state the licensing intent and link to the official texts.

**Before public release**, maintainers should consider replacing or supplementing these files with the full canonical license texts:

- AGPL-3.0: [https://www.gnu.org/licenses/agpl-3.0.txt](https://www.gnu.org/licenses/agpl-3.0.txt)
- CC BY-SA 4.0: [https://creativecommons.org/licenses/by-sa/4.0/legalcode](https://creativecommons.org/licenses/by-sa/4.0/legalcode)

The short-form notices are sufficient for development and clearly communicate intent, but the full texts provide the authoritative legal terms.

---

[Back to docs index](README.md)
