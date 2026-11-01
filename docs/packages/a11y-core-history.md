---
title: "a11y-core up to 0.11.0"
description: "The changelog history of the four crates from their time in the a11y-core repository."
order: 90
---

> History of `casoon/a11y-core` up to 0.10.1 (crates versioned in lockstep). From 0.10.2
> on, release-plz keeps a separate `CHANGELOG.md` per crate in `crates/<name>/`.


# Changelog

All notable changes to the four crates. They are versioned in lockstep. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the entries were
backfilled from the Git history.

## [0.11.0] - 2026-09-23

### Changed

- **All finding texts and rule hints are in English.** `message` and `help` come from a rule
  core that serves three surfaces and arbitrary third-party pages; German sentences in an
  English interface were a visible break. Rule IDs, outcomes and severities are
  unchanged — anyone checking on identifiers will notice nothing. Anyone comparing **message texts**
  (snapshots in `astro-post-audit` or `auditmysite`) must re-approve them. Hence 0.11.0 and not 0.10.2.
- Comments, tests and `docs/` stay German. What a tool outputs is translated.

## [0.10.2] - 2026-09-23

### Fixed

- `headings/empty` no longer reports a heading as empty when it gets its name via
  `aria-labelledby` or an image with alternative text. The same source is also on crates.io
  as 0.10.1; 0.10.2 is the version dependencies should rely on.

## [0.10.0] - 2026-09-20

### Changed

- `images/alt-suspicious` detects alt texts of one or two characters. The finding stays `review`.

## [0.9.0] - 2026-09-20

### Added

- `links/generic-name`: link text that says nothing about the target on its own, as `review`.

### Fixed

- `images/alt-missing` no longer reports images that are removed from the accessibility tree
  via `role="presentation"` or `aria-hidden="true"`.

## [0.8.0] - 2026-09-20

### Added

- `landmarks/main-missing`, `landmarks/main-duplicate`, `landmarks/navigation-missing`,
  `landmarks/banner-missing`, `landmarks/contentinfo-missing`.
- `keyboard/skip-link-missing`, `aria/required-attribute-missing`, `headings/h1-multiple`,
  `zoom/viewport-missing`.

## [0.7.0] - 2026-09-20

### Added

- Tier 3: `RenderingRule`, `contrast/text-insufficient` (WCAG 1.4.3) and
  `contrast/text-undetermined`.
- `run_full` for hosts with semantics and rendering, `run_with_rendering` for hosts without semantics.

### Changed

- `run` and `run_with_semantics` record the contrast identifiers as `CapabilityMissing`.

## [0.6.0] - 2026-09-19

### Added

- `lists/item-outside-list`: a list item without a surrounding list.

## [0.5.0] - 2026-09-19

### Added

- `lists/term-without-definition`, `tables/presentational-with-headers`, `tables/name-missing`
  (as `review`) and `zoom/viewport-scale-limited`.

## [0.4.0] - 2026-09-19

### Added

- `lists/empty`: a list without items.

### Changed

- `keyboard/positive-tabindex` has the default severity `high`.

### Fixed

- `zoom/viewport-locked` compares the `content` value case-insensitively.
- `lists/invalid-structure` takes `role="list"` and `role="listitem"` into account; `role="presentation"`
  on the list suppresses the list findings.
- `tables/header-missing` recognizes header cells via `role="columnheader"` and `role="rowheader"` and
  also checks `role="table"`.

## [0.3.0] - 2026-09-19

### Added

- `Finding` carries `role`, `name` and `rule_name`.
- `Extra`: tool-specific addition to a finding, never serialized and not part of its identity.
- `RuleRun` carries `viewport` and `wcag`.

### Breaking

- New fields on `Finding` and `RuleRun`.

## [0.2.0] - 2026-09-18

### Breaking

- `Meta::id` becomes `Meta::ids` and declares all finding identifiers of a rule. `RuleRun` is kept
  per identifier, so that `rule_runs` and `findings` use the same set of names.

## [0.1.0] - 2026-09-18

### Added

- `a11y-report`: finding and report model.
- `a11y-dom` and `a11y-rules`: document model with capability tiers and the first rules.
- `accname`: Accessible Name and Role Computation per accname 1.2 and HTML-AAM 1.0.
