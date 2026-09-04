# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-09-04

### Added
- **Fuzzing:** `fuzz/` (`cargo-fuzz`), a whole-pipeline fuzz target on `check()`. Local dev tool, not run in CI.
- **Benchmarks:** `benches/check.rs` (`criterion`), timing `check()` against a small typical page and a large table-heavy page from `tests/corpus/`. CI only compiles them (`cargo bench --no-run`).

### Changed
- Precision claim in the README narrowed to what the differential test suite actually measures (0 false positives, 99.98 % accuracy across 4,655 fixtures), instead of implying finding-level (rule-ID/message/position) parity with vnu.
- CI now runs the full `cargo-deny check` (licenses, bans, sources, advisories) instead of just `check licenses`, and adds a dedicated MSRV job pinned to `rust-version = "1.85.0"`.

### Fixed
- README install example and CHANGELOG were still on 0.1.0 despite the 0.2.0 release.

## [0.2.0] - 2026-09-01

### Added
- **Table Cell Grid (`tables.integrity`):** 2D grid layout of `colspan`/`rowspan` values to detect overlapping cells, cells spanning past the end of their row group, and columns no cell ever begins in (`src/table_integrity.rs`).
- **Real-World Sanity Tooling:** `xtask/fetch-real-world.sh` and `xtask/compare-real-world.sh` to compare `html-conform` against a locally-run vnu jar on live websites (supplementary, not part of the pinned corpus baseline).

### Changed
- Bumped `html5-parser` to 0.3.0, closing the tree-construction-error false-negative bucket.
- Dropped "pure" from the crate description/README wording.

### Fixed
- Recognize RDFa Lite `<meta property>` (Open Graph) in the RELAX NG schema layer.
- Corrected `dl` duplicate-`dt`, `rb`/`rtc`, and active `role=tab` Schematron rules.
- Implemented the missing-`lang` warning (`elements-html-missing-lang`) and corrected the differential test's harness-artifact comparison for it.
- Fixed an invalid crates.io category slug (`development-tools::validation` doesn't exist).

## [0.1.0] - 2026-08-29

### Added
- **Core Engine:** HTML5 conformance checking library combining 5 validation layers (Parser, RELAX NG Schema, Datatype Microsyntaxes, Schematron Co-constraints, JSON/CSP validators).
- **Schema Layer:** Full vendored W3C RELAX NG schemas for HTML5, SVG 1.1, and MathML 3 via `relax-ng`.
- **Datatype Library:** 50 custom W3C attribute datatype checkers (`w:image-candidate-strings` for `srcset`, `w:content-security-policy`, `w:media-query`, `w:datetime`, `w:iri-ref`, BCP 47 language tags).
- **Assertions:** Schematron rule engine with 100+ co-constraint rules across ARIA 1.2, HTML structural restrictions, element nesting, heading hierarchy, and link/script attribute combinations.
- **Dedicated Script/CSP Validators:** Spec-compliant JSON checkers for `<script type="importmap">` and `<script type="speculationrules">`, plus Content Security Policy enforcement for `<meta http-equiv="Content-Security-Policy">`.
- **Differential Test Suite:** 4,655 W3C/vnu corpus test fixtures with 98.8% accuracy and 0 False Positives.
- **Compliance & Tooling:** REUSE compliance, `deny.toml` for `cargo-deny`, and `THIRD-PARTY-NOTICES.md`.
