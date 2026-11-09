# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- **WASM:** `check(html)` returns the `html-conform` result as an `a11y-report` `Report`. A document that is too deeply nested is reported as `rule_runs` entry `html-conform` with `not_run: "errored"` and a reason, never as an empty pass (barrierlab#73).
