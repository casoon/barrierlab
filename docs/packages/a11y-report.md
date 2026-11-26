---
title: "a11y-report"
description: "The shared finding model: four outcome states, severity, WCAG mapping, JSON contract."
order: 5
---

The finding model all tools share: four outcome states, severity,
WCAG mapping and a stable JSON contract. It evaluates nothing itself.

## Status

On [crates.io](https://crates.io/crates/a11y-report), versioned in lockstep with the other
three a11y crates. Used by auditmysite, astro-post-audit, Relief and barrierlab-cli directly
and by liveaudit through `a11y-wasm` — the package that is really in use everywhere.

## Structure

```mermaid
flowchart LR
  rule["Rule in a11y-rules\nor in a host"] --> f[Finding]
  f --> ev[Evidence]
  f --> loc[Location]
  f --> ex[Extra]
  f --> rep[Report]
  nr[NotRun] --> rep
  rr[RuleRun] --> rep
  rep --> sum[Summary]
  rep --> json["JSON\n(stable contract)"]
```

## Public surface

| Type | Purpose |
|---|---|
| `Outcome` | the four states; `NotRun` is one of them, not a special case of "passed" |
| `Basis` | criterion or recommendation: a best-practice finding (`Finding::best_practice`) is `REVIEW` with `Basis::BestPractice`, never a WCAG failure |
| `Severity`, `WcagLevel` | classification of a finding; `WcagLevel::of_criterion("1.4.3")` gives the level of a WCAG 2.2 criterion |
| `Finding`, `Evidence`, `Location`, `SourcePosition` | a finding and what it is anchored to; `Location::end` is the exclusive end of a source range, `Finding::id()` the stable id (selector, else file and start position) |
| `Report`, `RuleRun`, `NotRun`, `Summary` | what a run produced, including the rules that did not run |
| `InputNote`, `InputKind` | `Report::inputs`: the host notes that an input was incomplete (stylesheet unavailable or unresolved, partial document). Rules do not read these notes; with `stylesheet_unavailable` present, treat the contrast rules as untested |
| `Extra` | host-specific additional fields, without breaking the contract |
| `FindingId`, `RootCauseId`, `normalize_selector` | stable identifier of a finding (rule + element pattern) and of a cause, the same across pages and across runs |
| `CauseConfidence`, `ImpactArea`, `EvidenceBasis` | how well a shared cause and an impact are supported |
| `CriterionState` | state of an assessment criterion; "not detected" is not "not met"

### Extensibility decisions

- `Report`, `Finding`, `RuleRun`, `Summary`, `Evidence`, `Location`, `InputNote`, `NotRun` (and `Meta` in `a11y-rules`) are `#[non_exhaustive]`: the crate produces them, hosts only read or build them through constructors, so new fields are not breaking.
- `Outcome`, `Severity` and `WcagLevel` stay exhaustive: they are closed vocabularies, and a new member would change what a verdict means, so a `match` that breaks is the intended signal.
- The host-input types `ComputedStyle`, `Layout`, `Rect`, `Color` and `Backdrop` stay exhaustive: hosts fill them completely, and a field added silently would be left at a default instead of being supplied.
- `SourcePosition` stays exhaustive: line, column and byte offset are the complete position a parser reports, and the producer fills all three.
- `Evidence::source` stays a free string: the set of sources is open (`ax_tree`, `dom_attribute`, `computed`, `meta`, `http_header`, and what hosts add), it is part of the JSON contract, and an enum would need an `Other(String)` escape that gives no more safety than the string.

## Dependencies

Downward: only `serde`. Upward: `a11y-rules`, `a11y-static`, `html-conform`, `a11y-wasm`,
`html-conform-wasm`, all hosts.

## Mutations

`tests/mutations/model.toml` lists targeted mutations of the counters, severity and
level order, defaults, wire names and the finding-id keys (including the pinned hash
values); the crate's unit tests must kill each. There is no corpus here, so entries
name their test filter in `killed_by = "lib:<filter>"` (or `"test:<file>"`) and
`xtask/mutate.py crates/a11y-report` runs it. Not part of CI; run it before changing
counting, ordering or id logic.

## Limits

The model says nothing about how a finding was found and whether the
measurement was suitable. Anyone who supplies no evidence gets a finding without
evidence — the model does not enforce it.
