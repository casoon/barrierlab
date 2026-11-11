# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.3.0] - 2026-11-11

### Changed

- **Dependencies:** requires `html-conform` 0.7.0, so findings follow that release: `help_url` on every finding, `xmlns:*` on SVG and MathML elements and a nested SVG `a` are reported. Details: [html-conform changelog](https://github.com/casoon/barrierlab/blob/main/crates/html-conform/CHANGELOG.md).

## [0.2.0] - 2026-11-10

### Changed

- **Dependencies:** requires `html-conform` 0.6.0, so findings follow that release: RDFa 1.1 and the ITS 2.0 attributes are accepted, `xmlns:prefix` on an HTML element is reported. Details: [html-conform changelog](https://github.com/casoon/barrierlab/blob/main/crates/html-conform/CHANGELOG.md).

## [0.1.1] - 2026-11-10

### Changed

- **npm:** `@casoon/html-conform-wasm` is published from CI with npm provenance; 0.1.0 was a one-time manual publish that created the package. No code change.

## [0.1.0] - 2026-11-10

### Added

- **WASM:** `check(html)` returns the `html-conform` result as an `a11y-report` `Report`. A document that is too deeply nested is reported as `rule_runs` entry `html-conform` with `not_run: "errored"` and a reason, never as an empty pass (barrierlab#73).
