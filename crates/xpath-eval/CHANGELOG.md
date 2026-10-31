# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases up to 0.2.2 are described
in the git history only.

## [Unreleased]

## [0.2.3] - 2026-10-31

### Added

- **API:** `DocumentCache`, handed in through the new `EvaluationContext::cache` field, memoises
  per document which nodes a name test selects from the root and which of them carry an
  attribute value; `//X[@a = 'v']` and `ancestor::*[@a = 'v']` are then answered from that
  index (barrierlab#69). Results are the same with and without it.
- **API:** `evaluate_boolean` evaluates an expression straight to a boolean and stops a location
  path at its first node.
- **API:** `Node::has_expanded_name` and `Node::with_string_value`, default methods a tree can
  override to compare names and values without copying them.

### Changed

- **Evaluation:** axes walk the tree lazily, `//X[p]` runs as `descendant::X[p]` when `p` does
  not depend on position, namespace prefixes are resolved once per call, and boolean contexts
  stop at the first node. Results are unchanged; in boolean contexts an error from a node after
  the first match is no longer raised, as with the existing `and`/`or` short-circuit.
