# Changelog

All notable changes to this crate are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project does not yet promise strict
[SemVer](https://semver.org/) API stability (still `0.x`) but avoids breaking changes without a
version bump.

## [Unreleased]

### Added

- **API:** `evaluate_with_cache` evaluates against a caller-held `xpath_eval::DocumentCache`, so
  several schemas checked against one document share it (barrierlab#69).

### Changed

- **Evaluation:** each expression is parsed once per document instead of once per context node,
  `<let>` scopes are no longer copied per node, and `assert`/`report` tests are evaluated as
  booleans. Reports are unchanged.

## [0.2.0] - 2026-08-29

### Breaking

- **API:** `LetBinding.value` is now `LetValue` (was a raw `String`). These public API changes are
  why this is `0.2.0`, not a patch release.
- **API:** `Schema` gains `properties: Vec<Property>` and `query_binding: Option<String>`;
  `MessagePart` gains a `Name` variant; `ParseError` gains `UnsupportedQueryBinding`,
  `CyclicInclude`, `UnresolvableInclude` and `InvalidInclude`.

### Added

- **API:** New public items `Property`, `parse_with_resolver`, `SchemaResolver`/`SchemaSource`/
  `ResolveError` (new `resolver` module).
- **Includes:** `<include href="...">` resolution (issue #2) via
  `parse_with_resolver(xml, base_uri, resolver)`; plain `parse` is unchanged (still
  `ParseError::UnexpectedElement` for `<include>`). A text-substitution pass (`resolve_includes`)
  splices each `<include>`'s resolved root element in, recursively, before any structural parsing,
  so every structural check applies to the expanded result. Only `href` without `#fragment`.
- **Includes:** `<extends href="...">` is deliberately not wired up: the ISO reference
  implementation (`iso_dsdl_include.xsl`) labels it experimental/non-standard, with child-splicing
  rather than element-splicing substitution semantics.
- **Let:** `<let>` with literal-XML (`foreign-element+`) content (issue #3): `LetValue::Literal(String)`,
  pre-computed to the content's string-value at parse time and bound as `Value::String`, as XSLT
  1.0 treats a variable bound to a result-tree fragment by default (usable as a string, not a
  node-set; no `exsl:node-set()` equivalent, consistent with implementing XPath 1.0 only).
- **Messages:** `<name>` message element (issue #4): `MessagePart::Name(Option<String>)`, rendered
  via `xpath-eval`'s own `name()` (`name()`/`name(path)`), reusing its qualified-name formatting.
- **Properties:** `<properties>`/`<property>` (issue #5, ISO Schematron 2016 extension):
  `Schema.properties: Vec<Property>`. The per-check IDREFS attribute `assert|report/@properties` is
  not modeled, like several other unread `assert`/`report` attributes.
- **Query binding:** `schema/@queryBinding` is read and validated (issue #6):
  `Schema.query_binding: Option<String>`; `"xslt"`/`"xpath"` (case-insensitive) run as XPath 1.0,
  anything else is `ParseError::UnsupportedQueryBinding` instead of silently running as XPath 1.0.

### Changed

- **Conformance:** Closes issues #2–#9 and #11 from the conformance tracking (#10) filed after
  0.1.0's release: broader conformance, external resources, literal-XML `<let>`.
- **Ids:** `id` uniqueness is now document-wide (issue #8), not just `pattern`/`diagnostic`:
  `check_document_wide_id_uniqueness` walks the raw XML tree once, before structural parsing, and
  checks every `xsd:ID`-typed element (`pattern`, `rule`, `assert`, `report`, `phase`,
  `diagnostic`, `property`) against one shared id space, matching XML `ID` semantics.
- **Contributors:** Vendored conformance corpus `tests/corpus/schxslt-testsuite/` (issue #9;
  27 cases, MIT-licensed, checksum-pinned, see its `UPSTREAM.md`), run by
  `tests/schxslt_testsuite.rs`. A handful of cases are skipped with a documented reason each
  (`<extends href>`, `pattern/@documents`, comment/PI node kinds the fixture doesn't model, the
  XSLT-only `document()` function), see its `SKIP` list.
- **Contributors:** The corpus surfaced issue #12, deferred: whether pattern-level `<let>` needs
  document-wide rather than per-pattern scope per ISO Schematron 2016 §5.4.5 clause 1. A plausible
  gap, but fixing it would change shipped, tested scoping, so it needs its own ISO-text verification.
- **Contributors:** The roxmltree `Document`/`Node` adapter moved from `tests/integration.rs` to
  `tests/support/mod.rs`, shared with the corpus test, and now normalizes `xmlns=""` (roxmltree's
  `Some("")`) to `None`; otherwise an unprefixed XPath name test silently fails to match (found via
  the corpus fixtures).

### Fixed

- **Context:** `rule/@context="/"`, ISO's bare root-node pattern, produced an invalid, unparseable
  `descendant-or-self::node()//`; the bare path `/` is now used unprefixed (issue #11, found via
  the corpus).
- **Ids:** Two abstract `pattern/@id`s colliding is now an error; the old, narrower check never
  caught it.
- **Let:** A `<let>` on an abstract rule pulled in via `<extends>` is now applied (issue #7):
  `collect_checks` (`src/parser.rs`) collects `<let>`s the way it inlines checks, in document order
  and recursing into `<extends>` where it appears, so an inherited `<let>` shadows or is shadowed by
  the rule's own `<let>`s depending on where `<extends>` stands.

## [0.1.2] - 2026-08-28

### Changed

- **Contributors:** No test caught the union bug below because every fixture in `engine.rs`'s
  suite was flat (`doc_with_root_children`), where a prefixed and a bare relative match coincide for
  a direct root child. New regression tests: `context_union_matches_through_every_alternative`
  (new `doc_with_nested_item` fixture, two levels down) and `split_top_level_union_ignores_nested_pipes`.

### Fixed

- **Context:** Only the first alternative of a `|`-union `context` ever matched: `context="a | b"`
  was evaluated as `descendant-or-self::node()/a | b` (XPath's `/` binds tighter than `|`), so `b`
  ran as a bare relative path from the root, matching only a direct root child named `b`, in
  practice never. Found via `html-conform`'s Phase 08 corpus work.
- **Context:** `context` is now split on top-level `|` (ignoring `|` inside `[...]`, `(...)` and
  quoted strings; new `split_top_level_union` helper) and each alternative is prefixed. No public
  API change (`evaluate`/`evaluate_with_phase` unchanged); single-alternative contexts are
  unaffected, multi-alternative contexts now match strictly more nodes, never fewer.

## [0.1.1] - 2026-08-22

### Changed

- **Internals:** Deduplication found via `cargo judge`'s duplicate-code analysis (22 findings down
  to 0). No public API or observable-behavior change (same 68 unit + 8 integration tests +
  1 doctest, all green before and after).
- **Internals:** `src/parser.rs`: required-attribute-or-`MissingAttribute` reads go through
  `required_attribute()`; "filter children by tag, parse each" positions (`let*`, `param*`,
  `active*`, `ns*`, `phase*`) through `collect_children()`; `role`/message-content extraction
  (shared by `<diagnostic>` and `<assert>`/`<report>`) through `parse_role_and_message()`.
- **Internals:** `src/engine.rs`: every "parse XPath, build variable/namespace hooks, evaluate"
  block (`<let>` binding, check `test`, `<value-of>`) goes through one `evaluate_xpath()` helper.

## [0.1.0] - 2026-08-21

### Added

- **Diagnostics:** `<diagnostic>`/`<diagnostics>`.
- **Ids:** Referential-integrity checking: duplicate `pattern`/`diagnostic` ids, unknown
  `active/@pattern` and `assert|report/@diagnostics` references are parse errors, not silently dropped.
- **Let:** `<let>` variable bindings (schema/pattern/rule/phase level).
- **Messages:** `<value-of>` message interpolation.
- **Patterns:** Abstract patterns/`is-a` (with parameter substitution) and `<rule>` `<extends>`.
- **Phases:** `phase` selection (`evaluate_with_phase`, `#ALL`/`#DEFAULT` semantics).
- **Schema:** Initial release: both ISO Schematron stages targeted by this crate (see `README.md`,
  "Scope and the name \"engine\"") are implemented. Schema parsing (`pattern`/`rule`/`assert`/
  `report`, `<ns>`) and evaluation against a caller-provided document (via `xpath-eval`).
- **Validation:** Unrecognized or misplaced Schematron-namespace elements are parse errors
  (`ParseError::UnexpectedElement`) rather than silently ignored; `README.md`'s support matrix says
  exactly what is and isn't covered.
