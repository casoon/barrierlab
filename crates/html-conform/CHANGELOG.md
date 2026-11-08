# Changelog

All notable changes to this project will be documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Features:** `runtime-compile` compiles the vendored schema on first use instead of loading
  the prebuilt artefact.

### Changed

- **Start-up:** the first `check()` in a process loads the schema from `prebuilt/html5.rngc`
  instead of compiling the `.rnc` files: about 3.7 billion instructions and 146 ms become 0.12
  billion and 7 ms (barrierlab#72). Findings are unchanged. Regenerate with `xtask/README.md`.

## [0.4.2] - 2026-11-07

### Added

- **API:** `CheckReport::to_report()` hands a finding's `FindingDetail` on in `Finding::extra`
  (`finding.extra.get::<FindingDetail>()`), so hosts can localize schema messages through the
  shared model instead of the deprecated `Finding` type. `extra` is never serialized; the JSON is
  unchanged.

## [0.4.1] - 2026-11-06

### Added

- **API:** `CheckReport::to_report()` fills `Location::position` (line, column, byte offset) from
  the finding's source location, so hosts that re-read the checked markup at the reported offset
  can move off the deprecated `Finding` type. `source_hint` keeps its `"line:column"` text.

## [0.4.0] - 2026-11-06

### Added

- **API:** `CheckReport::to_report()` converts a report into the shared `a11y_report::Report`
  (Error to Fail/Medium, Warning to Review/Low, Info to Untested/Low), so hosts can switch from
  the crate's own `Finding` types to the shared model (barrierlab#106). The old types stay.
- **API:** `CheckReport::skipped` lists classes the run did not check but the document contains
  (`SkippedClass::StyleElementCss`, `SkippedClass::CspHashSource`). `to_report()` turns each into
  an `InputKind::DocumentIncomplete` note. New public field: code that builds `CheckReport` with
  a struct literal must add it.
- **API:** `Finding::detail: Option<FindingDetail>` carries the structured arguments of a schema finding (`UnexpectedElement`, `UnexpectedText`, `UnexpectedAttribute`, `InvalidAttributeValue`, `MissingContent`) with element and attribute as local names, without the XHTML namespace, so hosts can localize the message without parsing it. `message` stays the unchanged English text. `detail` is `None` for every other layer and left out of the serialized form when `None` (casoon/barrierlab#96).

### Changed

- **API (breaking):** `Finding` has the new public field `detail`; code that builds a `Finding` with a struct literal must set it (`detail: None`).

### Deprecated

- **API:** `Finding` and `Severity` (the crate's own types) are `#[deprecated]` in favour of
  `CheckReport::to_report()` and the `a11y_report` model (barrierlab#106). They keep working;
  `CheckReport::findings` still carries them.

### Fixed

- **Check:** `csp.meta-enforcement` no longer flags inline scripts and styles whose `nonce`
  matches a `'nonce-…'` source of the policy (barrierlab#104). `'unsafe-inline'` is ignored when
  the source list carries a nonce or hash source, as in CSP3. Where a hash source is present the
  finding is an Info ("hash matching is not evaluated") instead of a warning.

## [0.3.4] - 2026-10-31

### Changed

- **Check:** schema validation does about half the work per page, through the faster
  `relax-ng` validator (astro-post-audit#109). Findings are unchanged.
- **Check:** the assertion stage runs about 10 to 20 times faster on typical pages and no longer
  grows quadratically with the number of `id` attributes, with one XPath cache shared by all rule
  files (barrierlab#69). Findings are unchanged.

## [0.3.3] - 2026-10-27

### Added

- **API:** `CheckError::DocumentTooDeep { depth, limit }`: a document nesting elements deeper
  than 512 levels is not checked and returns this error instead of a report. Hosts report it as
  "not run", never as passed. Details:
  [Errors](https://casoon.github.io/barrierlab/docs/html-conform/guides/dependency/#errors).

### Fixed

- **Check:** A deeply nested document (a few thousand `div`s) no longer aborts the process with a
  stack overflow on a 2 MiB thread such as a tokio worker (auditmysite#783).

## [0.3.2] - 2026-10-23

### Added

- **Assertions:** `assertion.elements.track-multiple-default`: more than one `track` with `default`
  in the same `audio`/`video` (vnu's message). The differential corpus caught this case before only
  by accident, through the whitespace bug below.

### Fixed

- **Schema:** Inter-element whitespace before a leading optional child no longer breaks the content
  model: `<fieldset> <legend>`, `<figure> <figcaption>`, `<details> <summary>` and
  `<table> <caption>` were reported as "unexpected element" (barrierlab#41, auditmysite#738).
- **Schema:** The cause was in `relax-ng` 0.2.0, which offered whitespace-only text to the pattern next to other
  children instead of stripping it (RELAX NG §6.2.7). Bumped `relax-ng` to 0.2.1.

## [0.3.1] - 2026-09-29

### Fixed

- **Schema:** `<meta name="…" property="…" content="…">` is no longer an "unexpected attribute
  `property`" error. HTML+RDFa 1.1 only drops the `name` requirement when `property` is present; it
  does not forbid `name`. Seen on gov.uk (`<meta name="title" property="og:title">`).

## [0.3.0] - 2026-09-23

### Added

- **Assertions:** `assertion.elements.img-missing-alt`: an `img` without `alt` is now an error
  outside `figure`, with vnu's exemptions (`aria-label`, `aria-labelledby`, a non-empty `title`;
  `role` and other `aria-*` attributes keep their more specific accessible-name rules). Previously
  only the `figure`-without-`figcaption` case was checked.
- **Assertions:** `assertion.elements.*-in-a`: interactive content inside `a` (`button`, `select`,
  `textarea`, `label`, `details`, `dialog`, `embed`, `iframe`, `a`, non-hidden `input`,
  `video`/`audio` with `controls`, `img`/`object` with `usemap`, any element with `tabindex` or an
  interactive `role`), matching vnu's list.

### Changed

- **Dependencies:** Bumped `html5-parser` to 0.4.0. `parser.html5` now also reports stray end tags
  with no matching element in scope (`</div>`, `</li>`, `</h2>`, …), start tags the parser ignores
  (a second `<body>`, `<td>` outside a table, …) and misnested formatting elements
  (`<b><i>…</b></i>`), all of which 0.3.0 dropped silently.
- **Dependencies:** Documents containing them get additional `parser.html5` errors from
  `html5-parser` 0.4.0; the differential corpus result is unchanged (0 false positives).
- **Docs:** README and docs no longer imply complete tree-construction error coverage; "What's not
  covered" lists the parse errors `html5-parser` 0.4.0 still does not record (e.g. `<div><span></div>`).
- **Packaging:** The crate now lives in the `casoon/barrierlab` monorepo, together with
  `html5-parser`, `csp-parse`, `media-query-parse`, `xpath-eval`, `relax-ng` and
  `schematron-engine`. The `=` pins on these packages are gone; they are workspace dependencies.
  `repository` and `homepage` point to the monorepo.
- **Packaging:** MSRV raised from 1.85 to 1.88 (`rust-version = "1.88.0"`). 1.85 was never actually
  buildable: `html5-parser` 0.4.0, `csp-parse` 0.1.0 and `xpath-eval` use `let` chains (stable
  since 1.88), and `url` → `idna` → `idna_adapter` 1.2.2 pulls in the `icu_*` 2.3 crates, which
  require 1.88.
- **Contributors:** The CI MSRV job now builds with 1.88.0.

## [0.2.1] - 2026-09-04

### Added

- **Contributors:** Fuzzing: `fuzz/` (`cargo-fuzz`), a whole-pipeline fuzz target on `check()`.
  Local dev tool, not run in CI.
- **Contributors:** Benchmarks: `benches/check.rs` (`criterion`), timing `check()` against a small
  typical page and a large table-heavy page from `tests/corpus/`. CI only compiles them
  (`cargo bench --no-run`).

### Changed

- **Docs:** Precision claim in the README narrowed to what the differential test suite actually
  measures (0 false positives, 99.98 % accuracy across 4,655 fixtures), instead of implying
  finding-level (rule-ID/message/position) parity with vnu.
- **Contributors:** CI now runs the full `cargo-deny check` (licenses, bans, sources, advisories)
  instead of just `check licenses`, and adds a dedicated MSRV job pinned to `rust-version = "1.85.0"`.

### Fixed

- **Docs:** README install example and CHANGELOG were still on 0.1.0 despite the 0.2.0 release.

## [0.2.0] - 2026-09-01

### Added

- **Tables:** Table cell grid (`tables.integrity`): 2D grid layout of `colspan`/`rowspan` values to
  detect overlapping cells, cells spanning past the end of their row group, and columns no cell
  ever begins in (`src/table_integrity.rs`).
- **Contributors:** Real-world sanity tooling: `xtask/fetch-real-world.sh` and
  `xtask/compare-real-world.sh` compare `html-conform` against a locally-run vnu jar on live
  websites (supplementary, not part of the pinned corpus baseline).

### Changed

- **Dependencies:** Bumped `html5-parser` to 0.3.0, closing the tree-construction-error
  false-negative bucket.
- **Docs:** Dropped "pure" from the crate description/README wording.

### Fixed

- **Assertions:** Corrected `dl` duplicate-`dt`, `rb`/`rtc`, and active `role=tab` Schematron rules.
- **Assertions:** Implemented the missing-`lang` warning (`elements-html-missing-lang`) and
  corrected the differential test's harness-artifact comparison for it.
- **Packaging:** Fixed an invalid crates.io category slug (`development-tools::validation` doesn't exist).
- **Schema:** Recognize RDFa Lite `<meta property>` (Open Graph) in the RELAX NG schema layer.

## [0.1.0] - 2026-08-29

### Added

- **Assertions:** Schematron rule engine with 100+ co-constraint rules across ARIA 1.2, HTML
  structural restrictions, element nesting, heading hierarchy, and link/script attribute combinations.
- **Core:** HTML5 conformance checking library combining 5 validation layers (Parser, RELAX NG
  Schema, Datatype Microsyntaxes, Schematron Co-constraints, JSON/CSP validators).
- **Datatypes:** 50 custom W3C attribute datatype checkers (`w:image-candidate-strings` for
  `srcset`, `w:content-security-policy`, `w:media-query`, `w:datetime`, `w:iri-ref`, BCP 47
  language tags).
- **Schema:** Full vendored W3C RELAX NG schemas for HTML5, SVG 1.1, and MathML 3 via `relax-ng`.
- **Validators:** Spec-compliant JSON checkers for `<script type="importmap">` and
  `<script type="speculationrules">`, plus Content Security Policy enforcement for
  `<meta http-equiv="Content-Security-Policy">`.
- **Contributors:** Differential test suite: 4,655 W3C/vnu corpus test fixtures with 98.8 %
  accuracy and 0 false positives.
- **Contributors:** Compliance and tooling: REUSE compliance, `deny.toml` for `cargo-deny`, and
  `THIRD-PARTY-NOTICES.md`.
