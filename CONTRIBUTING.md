<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contributing to AutoResearch Chain

## Project Status

AutoResearch Chain is at the protocol-design stage. There is no reference implementation yet. Contributions at this stage are primarily to the protocol specification, documentation, and project infrastructure.

## How to Contribute

### Protocol Design and Specification

- Open an issue to discuss a proposed change to the protocol before submitting a pull request.
- Protocol changes should be precise and should address specific mechanism-design questions.
- Avoid hand-waving. If a mechanism is proposed, describe how it works, what it prevents, and what trade-offs it introduces.

### Documentation

- Corrections, clarifications, and improvements to existing documents are welcome.
- New documents should be placed in `docs/` and added to the docs index (`docs/README.md`).

### Future: Code Contributions

When code exists in this repository:

- Follow the coding standards established in the project.
- Include tests where applicable.
- Apply the appropriate license header from `templates/`.

## Licensing of Contributions

By contributing to this repository:

- **Code contributions** are licensed under **AGPL-3.0-or-later** unless explicitly marked otherwise in the relevant file or directory.
- **Documentation contributions** are licensed under **CC BY-SA 4.0** unless explicitly marked otherwise.

See [LICENSES.md](LICENSES.md) and [docs/licensing.md](docs/licensing.md) for details.

## File Headers

When creating new files, apply the appropriate header:

- **Source code files**: Use the template in `templates/code-file-header.txt`
- **Documentation files**: Use the SPDX identifier `<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->` or the template in `templates/documentation-file-header.txt`

## Third-Party Code

If your contribution includes code from a third-party source:

1. Ensure the upstream license is compatible.
2. Preserve the original copyright and license notices.
3. Add an entry to `THIRD_PARTY_NOTICES.md` documenting the source, license, and affected paths.

Do not add third-party code without documenting it.

## Standards

- Write in clear technical prose. Avoid hype.
- Do not make claims about implementation maturity that are not yet true.
- Maintain the distinction between what the protocol specifies today and what is future work.
- Keep the trustless-market framing sharp. Vague "AI + blockchain" language weakens the project.

## Issues and Discussion

- Use GitHub Issues for bug reports, design questions, and feature proposals.
- Be specific. "The challenge mechanism should handle X" is useful. "We should add AI" is not.
