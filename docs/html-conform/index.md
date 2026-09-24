---
title: "Overview"
description: "What html-conform checks, how it is measured against vnu, and how this documentation is organised."
order: 0
---

html-conform is a Rust library for HTML5 conformance checking. You pass it a document as a
string and get back a `CheckReport`: a list of findings, each with a rule ID, a severity, a
message and, where it can be established, a source position. It runs in-process, with no JVM,
no subprocess and no network requests.

## What it checks

One call to `check()` combines six finding sources:

1. **HTML5 parser diagnostics** from [`html5-parser`](https://crates.io/crates/html5-parser),
   with browser-style error recovery.
2. **The W3C RELAX NG schema** for HTML, vendored from the Nu Html Checker (vnu), including
   the SVG 1.1 and MathML 3 subtrees.
3. **Attribute microsyntaxes** such as `srcset`, media queries, dates, URLs and language
   tags, checked as schema datatypes.
4. **Schematron co-constraints** from `rules/*.sch`: ARIA, IDs, headings, tables, obsolete
   elements and more.
5. **Script contents and CSP**: JSON checks for `<script type="importmap">` and
   `<script type="speculationrules">`, and enforcement of a
   `<meta http-equiv="Content-Security-Policy">` against inline scripts and styles.
6. **The table cell grid**: overlapping cells, cells spanning past their row group, and
   columns no cell begins in.

## How it is measured

The repository vendors 4,655 fixtures from vnu's own test suite. A differential test runs
each of them through `check()` and compares the result with vnu's expectation. The current
result is 0 false positives and 1 false negative; CI fails if either number rises. The
[comparison with vnu](guides/vnu-comparison/) explains exactly what that measures and what it
does not.

## Where to go next

- [Installation](getting-started/installation/) and [Quickstart](getting-started/quickstart/)
  get a first report on screen.
- [Using html-conform as a dependency](guides/dependency/) covers options, errors and
  typical integrations.
- [Findings and rule IDs](guides/rule-ids/) lists what each layer reports.
- The [API overview](reference/api/) links to the item-level documentation on docs.rs.
