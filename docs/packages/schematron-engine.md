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
