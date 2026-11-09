# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases up to 0.2.1 are described
in the git history only.

## [Unreleased]

### Added

- **API:** feature `serialize` adds `Schema::to_bytes`/`from_bytes`, a stable, versioned binary
  form of the compiled schema that a host can build once and load instead of compiling
  (barrierlab#72). Malformed input is a `LoadError`; no new dependencies.

- **API:** `validate_with_options`/`Schema::validate_with_options` with `ValidationOptions` add an
  opt-in recovery after an unexpected element: its subtree is checked against the element's
  definitions elsewhere in the schema, and the extra errors report `is_recovered()`. Off by
  default; the verdict and the plain error list are unchanged (barrierlab#76).

### Changed

- **Datatypes:** XSD `pattern` facets compile their regular expression once per thread instead of on
  every value check (barrierlab#70); results are unchanged.
- **Validator:** document names, attribute values and namespace bindings are hashed once per lookup
  instead of two to four times (name interning, value memo, table growth), still with a randomly
  seeded SipHash so documents cannot be crafted to collide. Schema validation of large real HTML
  pages is about a fifth faster (barrierlab#70); results are unchanged.

## [0.2.3] - 2026-10-31

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
