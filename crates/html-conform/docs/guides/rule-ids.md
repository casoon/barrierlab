---
title: Findings and rule IDs
description: Which layer reports what, under which rule ID and with which severity.
order: 2
---

Every finding carries a `rule_id` that names the layer, and for Schematron rules the
individual assertion. Messages are meant for people and may change; rule IDs are what code
should match on.

| Rule ID | Layer | Severity |
| --- | --- | --- |
| `parser.html5` | HTML5 tokenizer and tree construction | Error |
| `schema.html5` | RELAX NG schema, including attribute datatypes | Error |
| `assertion.<id>` | Schematron rule in `rules/*.sch` | from the rule's `role`, Error by default |
| `scripts.import-map` | JSON content of `<script type="importmap">` | Error |
| `scripts.speculation-rules` | JSON content of `<script type="speculationrules">` | Error |
| `csp.meta-enforcement` | inline content against a meta CSP | Warning |
| `tables.integrity` | table cell grid | Error |

## `parser.html5`

Diagnostics the HTML5 parser records while it recovers, for example an unknown named
character reference or a duplicate attribute. Set `include_parse_errors: false` to leave them
out of the report.

Tokenizer errors are covered completely, tree-construction errors in part: stray end tags
such as `</div>`, ignored start tags and misnested `<b><i>…</b></i>` are reported, but an end
tag that closes an element with others still open inside it (`<div><span></div>`) is not. See
[Comparison with vnu](../vnu-comparison/#not-covered-or-different).

## `schema.html5`

Validation against the vendored W3C RELAX NG schema for HTML, with the SVG 1.1 and MathML 3
subtrees. This covers the content model (which element may appear where), allowed
attributes, and the value of each attribute through custom datatypes: `srcset` and `sizes`,
media queries, dates and times, URLs, BCP 47 language tags, CSP strings and others. Messages
name the offending value, for example ``invalid value `2026-13-01` for attribute `datetime` ``.

## `assertion.<id>`

Co-constraints the schema cannot express, written as declarative XPath rules in `rules/*.sch`
and evaluated with [`schematron-engine`](https://crates.io/crates/schematron-engine). The
rule ID is `assertion.` followed by the Schematron assertion id, for example
`assertion.ids.duplicate` or `assertion.aria.hidden-not-focusable`. The rule files cover ARIA
roles, states and restrictions, attributes, elements, headings, IDs, microdata, obsolete
elements and tables.

A rule's `role` sets the severity: `warning` and `info` map to `Severity::Warning` and
`Severity::Info`, anything else to `Severity::Error`. The missing-`lang` check,
`assertion.elements.html-missing-lang`, is a warning.

## `scripts.*`

The text content of `<script type="importmap">` and `<script type="speculationrules">` must be
valid JSON of the right shape. For example, an import map may only have the keys `imports`,
`scopes` and `integrity`, and the `urls` of a speculation rule must be an array.

## `csp.meta-enforcement`

When a document declares a policy with `<meta http-equiv="Content-Security-Policy">`, its
inline script and style content is checked against `script-src` and `style-src`, each falling
back to `default-src`. Several meta policies apply cumulatively. A violation is a warning:
the document is still conforming HTML, but a browser would block that content.

The model is deliberately narrow. Only `'unsafe-inline'` counts as allowing inline content;
nonce and hash sources are not evaluated, and HTTP response headers are out of reach because
the checker only sees the document.

## `tables.integrity`

Lays each table out over its `colspan` and `rowspan` values and reports overlapping cells,
cells that span past the end of their row group, and columns in which no cell begins. This
needs state carried across cells, so it is Rust code rather than a Schematron rule.

The [showcase](../../../showcase/) has one sample document per layer, with the findings
html-conform reports for it.
