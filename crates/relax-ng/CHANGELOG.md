# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases up to 0.2.1 are described
in the git history only.

## [Unreleased]

### Changed

- **Validator:** attribute and child-element events skip every `ref` whose definition cannot
  lead to a matching name, instead of walking it each time. On real HTML pages validation does
  about half the work (astro-post-audit#109); results and their order are unchanged.
- **Validator:** patterns are interned during validation, so equal patterns are shared and
  compared by number; nullability and element and attribute derivatives are computed once per
  pattern, and `ref` is resolved by position instead of by name. On top of the change above,
  schema validation of real HTML pages runs 20–60 times faster (barrierlab#65); results are
  unchanged.

## [0.2.2] - 2026-10-27

### Fixed

- **Validator:** `validate` no longer recurses once per element nesting level; open elements are
  kept on a heap stack, so arbitrarily deep documents cannot overflow the call stack
  (auditmysite#783). Results and their order are unchanged.
