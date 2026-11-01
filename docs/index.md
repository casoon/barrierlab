---
title: "Overview"
description: "Which package does what, and how this documentation is organised."
order: 0
---

One home for all packages in this repository.

| Section | Contents |
|---|---|
| [Packages](packages/index.mdx) | all 15 crates with their dependencies, one page per package |
| [Accessibility in detail](a11y/index.md) | the a11y packages: installation, quickstart, concepts (findings, tiers, accname), reference |
| [Conformance in detail](html-conform/index.md) | html-conform: getting started, guides, reference |

The working documents for the repository itself sit in the root of `docs/` and do not
appear on this site:
[architecture](https://github.com/casoon/barrierlab/blob/main/docs/architecture.md),
[conventions](https://github.com/casoon/barrierlab/blob/main/docs/conventions.md),
[consumers](https://github.com/casoon/barrierlab/blob/main/docs/consumers.md),
[releases](https://github.com/casoon/barrierlab/blob/main/docs/releasing.md) and
[project state](https://github.com/casoon/barrierlab/blob/main/docs/project-state.md).
These are written in German.

Diagrams in the text are Mermaid blocks. GitHub renders them directly; this site turns them
into inline SVG at build time. The source is always the block, never the image. The exception
is the diagrams on the start page and the package overview: chartlet specifications in
`site/diagrams/`.

Most pages of the accessibility section are still in German; the html-conform section is in
English.
