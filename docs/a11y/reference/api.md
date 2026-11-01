---
title: "API Overview"
description: "The most important types and functions per crate. The detailed documentation of each item is on docs.rs."
order: 2
---

## a11y-rules

| Item | Purpose |
|---|---|
| `run`, `run_with_semantics`, `run_with_rendering`, `run_full` | check a document, depending on what the host provides |
| `run_in`, `run_with_semantics_in`, `run_with_rendering_in`, `run_full_in` | the same, with finding texts in the chosen `Locale` (`En`, `De`) |
| `structure_rules`, `semantics_rules`, `rendering_rules` | the rules of a tier, bound to a host |
| `structure_metas`, `semantics_metas`, `rendering_metas` | the declarations without a host, for listing and naming |
| `Meta` | IDs, tier, WCAG criteria, default severity and hint of a rule |
| `StructureRule`, `SemanticsRule`, `RenderingRule` | a rule as a function pointer plus `Meta` |

[docs.rs/a11y-rules](https://docs.rs/a11y-rules)

## a11y-dom

| Item | Purpose |
|---|---|
| `Document`, `Node` | the DOM-shaped base tree |
| `Semantics`, `Rendering`, `Interaction` | the capabilities on top of the structure |
| `Tier`, `Caps` | which data layer a rule needs, what a host provides |
| `ComputedStyle`, `Color`, `Rect`, `NameSource` | data of the capabilities |
| `Arena`, `ArenaBuilder`, `ArenaNode` | bundled tree, test vehicle and form for WASM hosts |
| `elements`, `descendants`, `ancestors`, `closest`, `subtree_text`, `has_text` | traversal |

[docs.rs/a11y-dom](https://docs.rs/a11y-dom)

## a11y-report

| Item | Purpose |
|---|---|
| `Finding`, `Location`, `Evidence` | a check result with location and evidence |
| `Outcome`, `Severity`, `WcagLevel` | the two axes and the WCAG level |
| `Report`, `Summary` | findings, records and counts |
| `RuleRun`, `NotRun` | whether a rule ran, and if not, why |
| `Extra` | opaque, non-serialized extra attached to a finding |

[docs.rs/a11y-report](https://docs.rs/a11y-report)

## accname

| Item | Purpose |
|---|---|
| `name`, `description` | accessible name and description |
| `role`, `implicit` | effective and implicit role |
| `allows_name_from_content`, `name_is_prohibited` | properties of a role |
| `IdIndex` | ID index for references |

[docs.rs/accname](https://docs.rs/accname)
