---
title: "schematron-engine"
description: "ISO Schematron engine, generic over the caller's document model, with SVRL output."
order: 14
---

Parses an ISO/IEC 19757-3 schema and evaluates it against a document the caller
provides through `xpath_eval::Document`/`Node`. It parses no instance XML or
HTML itself; XPath goes only through `xpath-eval`. Support matrix and usage:
[`README.md`](https://github.com/casoon/barrierlab/blob/main/crates/schematron-engine/README.md).

## `<let>` scope

A `<let>` is scoped to the element that declares it. ISO/IEC 19757-3:2016
defines a variable (3.26) as "scoped within the parent schema, phase, pattern or
rule", and 5.4.5 makes it an error to reference a variable "not defined in the
current schema, phase, pattern, or rule". A pattern-level `<let>` is therefore
visible to that pattern's rules only, not to sibling patterns. The vendored
schxslt case `let-pattern-global` expects the opposite; that is the suite's
reading (SchXslt compiles pattern variables as global XSLT variables, which its
own README lists as a scoping limitation), not the standard's text. The case
stays in the known-failure list.

## XPath 2.0 schemas

`queryBinding="xslt2"` or `"xpath2"` (ISO/IEC 19757-3:2016, 5.4.4, Query language
binding) selects the XPath 2.0 mode of `xpath-eval` for every `test`, `let`,
`value-of` and `name` expression of the schema; `xslt`, `xpath` and an absent
binding stay XPath 1.0 and behave as before. Any other binding is a parse error.

- The mode is the Cargo feature `xpath2`, on by default. Without it (the workspace crates
  depend on `schematron-engine` with `default-features = false`, to keep the
  `html-conform-wasm` size budget) `xslt2`/`xpath2` are rejected as before.
- A `<let>` is bound to a sequence with its atomic types, through the
  sequence-valued variable hook of `EvaluationContext`. `value-of` writes the
  atomized items of its sequence joined by one space (XSLT 2.0, 11.4.1).
- A rule `context` is a pattern: the nodes `descendant-or-self::node()/(context)`
  selects from the root. A context that starts with `/` and has no `|` is
  evaluated from the root alone (the result is the same). The 1.0 rewrite into
  per-node tests is not used, so a 2.0 schema is slower to match than a 1.0 one.
- What `xpath-eval` does not implement is reported, not skipped:
  `SchematronEvalError::Query` wraps the `VersionedError`, and `is_unsupported()`
  tells an incomplete run from a failed one. Static and dynamic XPath errors use
  the same variant; syntax errors keep `Parse`.
- `xsl:function` declarations are not run (no XSLT). Their names are in
  `Schema.xslt_functions`; a test that calls one ends the run as unsupported with
  the function named. Peppol BIS Billing 3.0 declares 12 to 13 such functions
  (check digits, `u:mod11`, `u:checkCF`, ...), XRechnung one or two
  (`xr:checkIBAN`); EN 16931 none.

Measured end to end with the preprocessed EN 16931 UBL rules (release
validation-1.3.17, fetched on demand by `crates/xpath-eval/xtask/vendor-en16931.sh`,
`cargo test -p schematron-engine --test en16931 -- --ignored`): the 981 checks of
104 rules run on the two UBL example invoices of that release (211 and 193 rule
firings) without any fired check, and a copy without `cbc:IssueDate` and with a
changed payable amount fires `BR-03` and `BR-CO-16`.

## SVRL output

`evaluate_trace` / `evaluate_trace_with_phase` return the evaluation as a list of
`Event`s: `ActivePattern`, `FiredRule` (one per matched node) and `Check` (a
fired `assert` or `report`, with its `test` and the ids of its diagnostics).
`to_svrl(schema, phase, &events)` writes them as an SVRL document string;
`to_svrl_with` takes a caller-supplied function for the `location` of a node.

Written: `schematron-output` (`phase`), `ns-prefix-in-attribute-values`,
`active-pattern` (`id`), `fired-rule` (`context`), `failed-assert` and
`successful-report` (`id`, `role`, `test`, `location`), `diagnostic-reference`
and `text`. Not written: `title`, `schemaVersion`, the `document`, `name`,
`flag` and `diagnostic` attributes, and `property-reference`, because the schema
model does not carry them. The structure follows the informative Annex D.

`location_path` builds an XPath 1.0 path from the `Node` trait, for example
`/a[1]/*[local-name()='b' and namespace-uri()='urn:x'][2]/@id`. Names in a
namespace are written with `local-name()`/`namespace-uri()` so the path needs no
prefix bindings. A test evaluates every generated path and checks that it
selects exactly the node it names.
