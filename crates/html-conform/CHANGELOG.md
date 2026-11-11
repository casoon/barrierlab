# Changelog

All notable changes to this project will be documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-11-11

### Added

- **Assertion rules:** `assertion.elements.svg-a-in-svg-a` reports an SVG `a` inside another SVG
  `a` (barrierlab#198).
- **API:** `RuleInfo::help_url` links each rule to its layer's section of the "Findings and rule
  IDs" guide (barrierlab#194).

### Changed

- **API:** `CheckReport::to_report` and `RangedReport::to_report` fill `help_url` of each
  finding with that link, so the CLI's JSON output now carries `help_url` (barrierlab#194).
- **Schema check:** `xmlns:*` attributes on SVG and MathML elements are reported unless they are
  `xmlns:xlink` bound to the XLink namespace, as vnu does (HTML, "Attributes"; barrierlab#198).

- **API:** `RangedReport::to_report` now fills `location.end` of the shared model for every
  finding that has a range, and the CLI's JSON output takes it from there instead of patching it
  into the text; the output is unchanged.

## [0.6.0] - 2026-11-10

### Added

- **API:** `check_with_ranges` returns a `RangedReport` with the source range (`SourceRange`, end
  exclusive) of each finding whose layer knows the end: the start tag of the element a schema,
  assertion, script, CSP or table finding is anchored at. `parser.html5` findings have none.
  `SourceLocation` documents its units: `column` counts Unicode scalar values, `byte_offset`
  UTF-8 bytes (barrierlab#194).
- **API:** `rules()` lists every rule id the checker can report with layer, outcome, description
  and source, read from the parsed `rules/*.sch` files and the layer constants (barrierlab#194).
- **CLI:** JSON findings carry `location.end` where known; SARIF regions carry
  `endLine`/`endColumn`, the run sets `columnKind` to `unicodeCodePoints`, and
  `tool.driver.rules` take their descriptions from the catalogue; `--list-rules` prints the
  catalogue as JSON (barrierlab#194).
- **Schema check:** feature `rdfa-lite` (off by default) adds `SchemaPreset` and
  `check_with_preset` for vnu's `html5-rdfalite` preset, which allows only the RDFa Lite 1.1
  attributes. The second compiled schema (`prebuilt/html5-rdfalite.rngc`, about 246 KB) is compiled
  in only with the feature; the default behaviour is unchanged (barrierlab#181).
- **Schema check:** RDFa 1.1 is accepted, as in vnu's default schema: `property`, `typeof`,
  `prefix`, `about`, `resource` and the other RDFa attributes, `<link property resource>` and
  `<meta property content>` included. False positives on vnu's `html-rdfa` and `html-rdfalite`
  trees fall from 177 and 50 to 2 and 0 (barrierlab#173).
- **CLI:** files and stdin are decoded by WHATWG sniffing (byte order mark, `<meta charset>`,
  else UTF-8); bytes invalid in that encoding are reported as not checked in every format. A
  non-UTF-8 file is decoded as declared and still gets `assertion.elements.meta-charset-not-utf8`.
  Details: [Encodings](https://casoon.github.io/barrierlab/docs/html-conform/guides/cli/#encodings)
  (barrierlab#74).
- **CLI:** `--disable <rule>` and `--only <rule>` select findings by rule ID or dotted prefix, and
  `html-conform.toml` (working directory) or `--config <file>` sets `format`, `fail-on`, `color`,
  `quiet`, `disable` and `only`; `--no-config` ignores it. Hidden findings are counted on standard
  error, unknown rule IDs and malformed config files are exit code 2 (barrierlab#74).
- **Release:** `html-conform` binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64) and
  Windows (x86_64) are attached to the release with `SHA256SUMS` and a build provenance
  attestation (barrierlab#74).

### Changed

- **Dependencies:** requires `html5-parser` 0.5.1 (`Node::end_position`) and `relax-ng` 0.2.5
  (XSD name escapes in `pattern` facets, used by the RDFa schema).
- **CLI:** the SARIF `shortDescription` of an `assertion.*` rule is now that rule's own message
  instead of the generic sentence "Co-constraint the schema cannot express", and the run declares
  `columnKind` as `unicodeCodePoints` (barrierlab#194).
- **Rules:** a `<link>` may carry `resource` instead of `href`, and a `<link>` with `property`
  is allowed in the body.

### Fixed

- **Schema check:** the ITS 2.0 attributes (`its-loc-note`, `its-translate`, `its-annotators-ref`
  and the others) are accepted on HTML elements, as in vnu's default schema. On the 90 `.html`
  files of vnu's `tests/html-its` the 90 false `schema.html5` findings in 25 files are gone;
  html-conform and vnu 26.8.20 now agree per file (barrierlab#188).
- **Schema check:** `xmlns:prefix` on an HTML element is reported (it stays visible to the schema;
  on SVG and MathML elements it is still a namespace declaration), and `rev` on `<a>` is accepted
  as in vnu's `legacy.rnc` (patch `0003-a-rev-legacy`). Differences to vnu on the `html-rdfa` and
  `html-rdfalite` trees fall from 2 false positives and 7 false negatives to none (barrierlab#181).
- **Schema check:** `xlink:href` and the other XLink attributes on SVG elements were rejected as
  "unexpected attribute"; they now match the schema's `{xlink}href` (barrierlab#172).

## [0.5.0] - 2026-11-09

### Added

- **CLI:** the `cli` feature (off by default, no new dependency) builds an `html-conform` binary
  that checks files, directories and stdin and writes text, JSON, SARIF 2.1.0 or JUnit; exit code
  0/1/2 by `--fail-on`. Unchecked content and unreadable files appear in every format, never as a
  pass. Install with `cargo install html-conform --features cli` (barrierlab#74).
- **Features:** `runtime-compile` compiles the vendored schema on first use instead of loading
  the prebuilt artefact.

### Changed

- **Dependencies:** requires `html5-parser` 0.5 (more tree-construction and tokenizer parse errors
  are reported, so parse-error findings can differ), `relax-ng` 0.2.4 with its `serialize`
  feature, and `a11y-report` 0.29.
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
