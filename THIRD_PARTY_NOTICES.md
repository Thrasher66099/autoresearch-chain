# THIRD-PARTY NOTICES

This file documents third-party software, content, and related notices relevant to the AutoResearch Chain repository.

## Purpose

The purpose of this file is to:

- identify third-party code or materials included in the repository,
- preserve required attribution and license information,
- distinguish inspiration from direct inclusion,
- help maintainers stay compliant with upstream licenses.

---

## Current Status

At the current repository stage, the project may reference external tools, papers, concepts, or repositories for inspiration and compatibility planning.

**Unless explicitly noted below, reference to a third-party project does not mean that code from that project is included in this repository.**

If code is later copied, modified, vendored, or redistributed from any third-party source, maintainers should add an entry here with:

- project name,
- upstream URL,
- copyright holder,
- applicable license,
- paths in this repository where the material appears,
- notes on modifications.

---

## Referenced Project: `karpathy/autoresearch`

**Project:** `autoresearch`  
**Upstream URL:** `https://github.com/karpathy/autoresearch`

### Relevance

AutoResearch Chain explicitly contemplates users running an autonomous research loop similar to `karpathy/autoresearch` or a closely related derivative.

The protocol design assumes a Stage 1 workflow in which an AI agent may:

- inspect a training recipe,
- modify training code such as `train.py`,
- run a short GPU experiment,
- measure the resulting metric,
- and submit improvements to the protocol.

### Important Clarification

At this stage, **conceptual inspiration or compatibility planning does not by itself mean that `autoresearch` code is included in this repository**.

If this repository does **not** contain copied, modified, or vendored code from `autoresearch`, then no code-level relicensing or embedded third-party source notice is required beyond this informational reference.

If this repository **does** later include code copied or derived from `autoresearch`, maintainers must:

1. preserve the upstream copyright notice,
2. preserve the upstream license text,
3. document the affected paths in this file,
4. clearly mark modified files where appropriate.

### Placeholder Entry Format for Future Inclusion

If `autoresearch`-derived code is later added, use a section like this:

```text
Project: autoresearch
Upstream URL: https://github.com/karpathy/autoresearch
License: <insert upstream license here>
Repository paths:
- path/to/file1
- path/to/file2
Modification status:
- modified / unmodified
Notes:
- <any important compliance notes>