---
title: "Capability Tiers"
description: "Why the document model consists of several traits and what happens to a rule whose tier the host does not serve."
order: 1
---

The three surfaces differ not in how they present the same data, but in
**which data exists at all**. A flat trait with `Option` return values would lead to
rules silently not running depending on the host.

| | static HTML | Chrome via CDP | In-page WASM |
|---|---|---|---|
| `Document`: structure, tags, attributes, text, hierarchy | ✓ | ✓ | ✓ |
| `Semantics`: role, accessible name | computed | native | computed |
| `Rendering`: styles, geometry | – | ✓ | ✓ |
| `Interaction`: focus, events | – | ✓ | ✓ |

Every rule declares its `Tier`; every host implements the traits it can serve.

## The boundary lies in the type system

Rules are function pointers, registered separately by tier: `StructureRule<D: Document>`,
`SemanticsRule<D: Semantics>`, `RenderingRule<D: Rendering>`. A tier 2 rule cannot even be
called with a host that does not satisfy `Semantics`. No trait objects: this
monomorphizes per host and allocates nothing per rule.

## Not run is not passed

The entry functions run with what the host provides and record the rest. `run` on
a structure-only host lists every tier 2 and tier 3 ID as a `RuleRun` with
`NotRun::CapabilityMissing` and a reason:

```json
{
  "rule_id": "links/name-missing",
  "not_run": "capability_missing",
  "findings": 0,
  "reason": "host provides no role and no accessible name"
}
```

The excerpt comes from `examples/teaser.struktur.json`. Without this record, a
contrast check without rendering access would read like a passed check.

## The tree is DOM-shaped

`Node` models tags, attributes, text and hierarchy, not roles and accessible names. Most
rules need attributes such as `tabindex`, `id`, `role`, `alt` or `for`, and the browser's
native accessibility tree does not provide them: `tabindex` does not appear there. Role
and name are layered on top as a separate capability.

## Requirement on tier 3

`ComputedStyle::background_color` is the **effective** background color, resolved across the
ancestors. If the host cannot determine it, for example with a background image, gradient or
`background-blend-mode`, it returns `None`, and the contrast rule reports
`contrast/text-undetermined` with `untested`. A check against a guessed white would produce a
`pass` that someone relies on.

`Interaction` is defined as a trait, but the 0.26.1 rule set has no rule
on this tier yet.
