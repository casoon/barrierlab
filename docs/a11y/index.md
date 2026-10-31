---
title: "Overview"
description: "What a11y-core is, which crates belong to it and how this documentation is organized."
order: 0
---

a11y-core is the shared set of building blocks of the CASOON accessibility tools in Rust. One
rule set serves three surfaces with identical rule IDs and identical JSON:

| Surface | Tool | Substrate |
|---|---|---|
| Build time | [astro-post-audit](https://github.com/casoon/astro-post-audit) | static HTML from `dist/` |
| CI / crawl | [auditmysite](https://github.com/casoon/auditmysite) | Chrome via CDP, native accessibility tree |
| Running page | [liveaudit](https://casoon.github.io/liveaudit/) | the page's DOM, via WASM |

## Crates

| Crate | Purpose |
|---|---|
| [`a11y-report`](https://docs.rs/a11y-report) | Finding and report model, JSON contract |
| [`a11y-dom`](https://docs.rs/a11y-dom) | Document model abstraction plus capability tiers |
| [`accname`](https://docs.rs/accname) | WAI-ARIA accessible name and role computation |
| [`a11y-rules`](https://docs.rs/a11y-rules) | The rules, generic over the document model |

All four are on crates.io at version 0.26.1 and are versioned in lockstep. Planned but not yet
available is `a11y-conformance`: a shared fixture corpus that keeps the three surfaces from
drifting apart.

## Where it stops

a11y-core provides no HTML parser, no crawler and no browser control. The host builds the tree:
astro-post-audit from the build files, auditmysite from Chrome, liveaudit from the page's DOM. The
bundled `Arena` from `a11y-dom` is the test vehicle and the form a WASM host needs.

## How this documentation is organized

- **Getting started:** add the crates and check a first document.
- **Concepts:** capability tiers, the finding model and name computation.
- **Reference:** all rule IDs and an overview of the public API. The detailed documentation of
  each type is on [docs.rs](https://docs.rs/a11y-rules).
