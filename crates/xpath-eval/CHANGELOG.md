# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases up to 0.2.2 are described
in the git history only.

## [Unreleased]

### Added

- **API:** user-defined functions in the XPath 2.0 mode: `EvaluationContext::functions`
  (`FunctionContext`, `FunctionLookup`), nesting limited to `MAX_FUNCTION_DEPTH`; a function body can
  be evaluated without a context item (casoon/barrierlab#62).

### Changed

- **XPath 2.0:** `xs:integer` holds 128 bits (39 digits) instead of 64; a longer value is
  reported as unsupported instead of `FOAR0002`. The IBAN check of XRechnung reads a 24-digit
  number. `Atomic::Integer` carries an `i128`, so code that matches it needs a minor version bump
  (casoon/barrierlab#62).

## [0.3.0] - 2026-11-11

### Added

- **API:** Opt-in XPath 2.0 mode next to the unchanged 1.0 default (`XPathVersion`,
  `parse_versioned`, `evaluate_versioned`, `evaluate_boolean_versioned`, `VersionedError`); slice 1
  of the e-invoice rule sets, unsupported features are named errors. Design and remaining slices
  in `docs/packages/xpath-eval.md` (casoon/barrierlab#62).
- **API:** `EvaluationContext::sequence_variables` (`SequenceVariableLookup`) binds a variable to a
  sequence with its atomic types for XPath 2.0 expressions, consulted before the 1.0 `variables`
  hook; `sequence_to_string` renders a sequence as `value-of` does (atomized, joined by a space).
  `VariableLookup` is exported. Used by the XPath 2.0 mode of `schematron-engine`
  (casoon/barrierlab#62).
- **XPath 2.0:** `fn:string-to-codepoints` and `fn:codepoints-to-string` (error `FOCH0001` for a
  code point that is no XML character), used by the check-digit functions of the Peppol BIS Billing
  3.0 rules. Inventory of Peppol and XRechnung in `docs/packages/xpath-eval.md`
  (casoon/barrierlab#62).
- **XPath 2.0:** `fn:matches`, `fn:replace` and `fn:tokenize` (all arities, flags `s m i x`, errors
  `FORX0001` to `FORX0004`) on an own regular-expression engine without a dependency: an NFA run as
  a Pike VM, so matching is O(pattern x input) and cannot backtrack catastrophically or overflow
  the stack. Back-references, Unicode categories and name characters beyond U+00FF, unknown blocks
  and programs over 10,000 instructions are named `Unsupported`, never a guess. Supported subset
  and limits in `docs/packages/xpath-eval.md` (casoon/barrierlab#62).
- **Conformance:** XPath 2.0 own tests and an opt-in measurement against the EN 16931 validation
  Schematron (`tests/en16931.rs`, `xtask/vendor-en16931.sh`; not kept in the repository): 2009 of
  2010 expressions evaluate in 2.0 mode, 1780 in 1.0 mode. Details in `CONFORMANCE.md`
  (casoon/barrierlab#62).
- **Conformance:** The Jaxen XPath test cases run as an opt-in, externally fetched corpus
  (`tests/jaxen.rs`, `#[ignore]`, `xtask/vendor-jaxen.sh`; not kept in the repository): 263 of 281
  case runs pass, the 18 known failures are in `tests/jaxen_known_failures.txt`. Details in
  `CONFORMANCE.md` (casoon/barrierlab#111).
- **Conformance:** Own XPath 1.0 regression tests (`tests/xpath10_regression.rs`) written from the
  specification cover every category of that table, so the level stays checked without the external
  corpus (casoon/barrierlab#111).

### Fixed

- **XPath 1.0:** The parser now limits nesting and operator chains (256 weighted levels, the same
  limit as the 2.0 parser) and returns a `ParseError` instead of overflowing the stack on
  expressions such as 600 nested parentheses, `1+1+...` with 2000 terms or deeply nested `not(...)`,
  reachable through untrusted Schematron files. Expressions within the limit parse as before
  (casoon/barrierlab#192).

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
