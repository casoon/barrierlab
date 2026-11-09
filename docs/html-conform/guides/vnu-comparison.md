---
title: "Comparison with vnu"
description: "What the differential test against the Nu Html Checker measures, what html-conform covers, and where it differs."
order: 3
---

html-conform reuses the HTML schema of the [Nu Html Checker](https://validator.github.io/validator/)
(vnu) and measures itself against vnu's test suite. It is not a port of vnu and does not
produce the same messages.

## What is measured

`tests/differential.rs` runs html-conform against fixtures vendored from
[`validator/validator`](https://github.com/validator/validator) at tag 26.8.20 (commit
`388cb36`):

| Corpus tree | Fixtures |
| --- | --- |
| `tests/html` | 3,845 |
| `tests/html-aria` | 810 |
| `tests/html-math` | 48 |
| `tests/html-rdfa` | 212 |
| `tests/html-rdfalite` | 56 |
| `tests/html-svg` (6 of 522, see below) | 6 |
| **Total** | **4,977** |

vnu's expected results say, per fixture, whether it should produce findings. The test counts
agreement:

| Result | Count |
| --- | --- |
| Result | `html` + `html-aria` | `html-math` | `html-rdfa` | `html-rdfalite` | `html-svg` | All |
| --- | --- | --- | --- | --- | --- | --- |
| true positives (vnu expects findings, html-conform reports some) | 3,745 | 18 | 28 | 2 | 0 | 3,793 |
| true negatives (both clean) | 909 | 30 | 7 | 4 | 5 | 955 |
| false positives (vnu clean, html-conform reports something) | 0 | 0 | 177 | 50 | 0 | 227 |
| false negatives (vnu expects findings, html-conform reports none) | 1 | 0 | 0 | 0 | 1 | 2 |
| not comparable (`check()` returned an error) | 0 | 0 | 0 | 0 | 0 | 0 |

CI runs this test on every push (`cargo test --release --test differential -- --ignored`) and
fails if false positives or false negatives of a tree rise above its committed baseline. `html`
and `html-aria` stay at 0 false positives; the 227 false positives are the measured gap of the
two RDFa trees (see below), so they cannot hide a regression elsewhere. The two false negatives
are pinned by name in the test (CSS below, and the nested SVG `<a>` of `html-svg`), so another
missed fixture cannot hide behind them.

### Finding classes

The same test also compares the *class* of finding for the 3,795 fixtures where vnu expects
findings. `tests/vnu-class-map.tsv` maps each vnu message (with quoted parts replaced by `“X”`)
to a class (parser, schema, assertion, scripts, csp, tables, css) and to the html-conform
`rule_id` prefixes that count as the same class. A fixture is **matched** if html-conform
reports a finding of vnu's class, **incidental** if it reports findings but none of that
class, and **missed** if it reports none. vnu messages without a row in the map are listed by
the test and counted as **unmapped**; there are none at present.

| Corpus tree | matched | incidental | missed | unmapped |
| --- | --- | --- | --- | --- |
| `tests/html` | 3,389 | 100 | 1 | 0 |
| `tests/html-aria` | 253 | 3 | 0 | 0 |
| `tests/html-math` | 18 | 0 | 0 | 0 |
| `tests/html-rdfa` | 28 | 0 | 0 | 0 |
| `tests/html-rdfalite` | 2 | 0 | 0 | 0 |
| `tests/html-svg` | 0 | 0 | 1 | 0 |

(Matched plus incidental equals the 3,793 true positives. The 30 matches in the two RDFa trees
carry little weight: html-conform rejects every RDFa attribute, so each of those fixtures
produces a schema finding whether or not it contains the error vnu expects.) The baseline is committed in
`tests/vnu-class-baseline.json`; CI fails if *incidental* rises, *matched* falls, or the
number of unmapped messages changes.

The class figures still do not say that html-conform reports the *same* problem, only the
same *kind*: a schema finding on a fixture whose vnu message is a different schema error
counts as matched. They ignore message text, positions and how many findings vnu expects per
fixture (the expectation file holds one message per fixture). The class map is coarse and was
written from vnu's message shapes, not tuned to the results; a wrong row shifts fixtures
between matched and incidental. The `css` class has no html-conform counterpart.

### What the numbers do not say

The comparison is per fixture: does the document produce findings or not. It does not compare
rule IDs, message text or positions. A fixture counts as a true positive when html-conform
reports anything, even if it reports a different problem than vnu does. Treat 0 false
positives as "html-conform does not flag documents vnu considers clean, on this corpus", not as
finding-by-finding parity.

## Covered

- **The same schema.** The RELAX NG schema in `schema/` is vnu's, vendored unchanged,
  including SVG 1.1 and MathML 3. MathML is measured with 48 fixtures (no disagreement); SVG
  with only 6 (see the triage below).
- **Parser diagnostics, microsyntaxes, co-constraints**: see
  [Findings and rule IDs](../rule-ids/) for each layer.
- **Import maps, speculation rules and meta CSP**: checked by dedicated Rust code.
- **Table integrity**: the 2D cell grid, which was once the largest group of false
  negatives.

## Not covered, or different

- **Some tree-construction parse errors.** [`html5-parser`](https://crates.io/crates/html5-parser)
  0.4.0 records every tokenizer error, and a tree-construction error wherever it ignores a
  token (a stray `</div>`, `</li>` or `</h2>`, a second `<body>`, `<td>` outside a table, …)
  or repairs misnested formatting elements such as `<b><i>…</b></i>` with the adoption agency
  algorithm. It still records nothing for the spec's parse errors on tokens it keeps: an end
  tag that closes an element while other elements inside it are still open
  (`<div><span></div>`), content after `</html>`, and the obsolete frameset modes. vnu reports
  these; the repaired tree carries no trace of them, so html-conform cannot find them after
  parsing. The differential test did not move with 0.4.0: the fixtures vnu flags for a stray
  `</header>` or `</td>` already counted as true positives through unrelated heading and table
  findings.
- **CSS inside `<style>`.** vnu hands style sheets to a vendored copy of the W3C CSS
  Validator. html-conform has no CSS parser for this, so a mistyped property such as `colr`
  goes unreported. This is the one false negative
  (`html/elements/style/css-property-error-novalid.html`), and a deliberate limitation:
  CSS correctness is out of scope (decision A of issue #87; checking property names or whole
  values would need its own normative basis and corpus). The test pins this fixture by name
  as the only allowed miss.
- **RDFa.** vnu's default schema includes RDFa (`rdfa.rnc`), html-conform's vendored schema
  does not, and html-conform also reports `<link>`/`<meta>` elements in `<body>` that carry only
  RDFa attributes. Every RDFa attribute (`prefix`, `property`, `typeof`, `about`, `resource`, `vocab`, ...) is reported as
  unexpected: 177 false positives in `tests/html-rdfa` and 50 in `tests/html-rdfalite`
  (measured, see the triage below). The gate pins these numbers; it does not accept more.

## Triage of vnu's other test trees

vnu's `tests/` holds more trees than `html` and `html-aria` (issue #86). Each one has one of
three outcomes:

| Tree | Files | Outcome | Why |
| --- | --- | --- | --- |
| `tests/html-math` | 48 | measured | 0 false positives, 0 false negatives |
| `tests/html-rdfa` | 212 | measured, gap | 177 false positives: no RDFa in the schema |
| `tests/html-rdfalite` | 56 | measured, gap | 50 false positives: same cause |
| `tests/html-svg` (not in the issue's list) | 522 | 6 measured, 516 not vendored | the 516 embed W3C SVG 1.1 test cases, see below |
| `tests/css` | 16 | out of scope | stand-alone style sheets for vnu's CSS checker; html-conform checks HTML only and has no CSS validation (issue #87) |
| `tests/xhtml` | 129 | out of scope | XML syntax; html-conform parses HTML only and plans no XHTML mode |
| `tests/svg` | 1 | out of scope | a stand-alone `.svg` document, not HTML syntax (inline SVG is `html-svg`) |
| `tests/langdetect` | 11 | out of scope | vnu's language-detection heuristics (character statistics over about 1.4 MB of text); html-conform has no language detector |
| `tests/normalization` | 1 | out of scope | a Unicode NFC warning on one text run; html-conform has no such check |
| `tests/html-its` | 320 | needs a decision | HTML plus external ITS rule files in XML; vnu's own `make` skips it (`--ignore=html-its`), so there is no expected-result file to measure against |
| `tests/schema-validation` | 3 | needs a decision | exercises vnu's `--schema` option on XML files with an arbitrary schema (DocBook); html-conform's command line has no schema option |

A local run (not part of the test, nothing committed) of the out-of-scope trees that hold
`.html` files gave: `langdetect` 0 true positives, 4 true negatives, 1 false positive, 5 false
negatives; `normalization` 1 false negative. These are not gated.

### `html-svg` and the W3C copyright

516 of the 522 files in `tests/html-svg` are SVG 1.1 test cases of the W3C, each with the
header "Copyright 2009 World Wide Web Consortium ... All Rights Reserved". W3C publishes that
suite under its document license, which allows no modification and counts a subset as a
derivative work; this repository is MIT. `xtask/vendor-corpus.sh` therefore copies only the 6
files without that header (written by the vnu authors, MIT). The full tree is measurable
locally by checking out vnu at the same tag; the 479 false positives above come from such a
run, not from the committed corpus. Whether to vendor the W3C files, fetch them on demand into
a gitignored cache (like the real-world pages), or leave SVG at 6 fixtures is open.

### RDFa

vnu checks `tests/html-rdfa` against its default schema (`html5-all`, which includes RDFa) and
`tests/html-rdfalite` against `html5-rdfalite`. html-conform vendors only the part of the
schema reachable from `html5.rnc`, so these two trees measure the absence of RDFa, not
differences in how it is checked. They stay in the gated test, with the false positives pinned
per tree, so that the gap is visible and cannot grow unnoticed.

- **HTML syntax only.** The input is parsed as HTML with browser-style recovery; there is no
  XHTML (XML syntax) mode, and none is planned.
- **A library, not a service.** html-conform checks the string you pass it, in-process. It
  makes no HTTP requests, needs no JVM and ships no command-line tool.
- **Messages and rule IDs are its own.** Wording differs from vnu, and there is no mapping from
  html-conform rule IDs to vnu messages.
- **Missing `lang` is a warning, on purpose.** vnu's test runner switches the missing-`lang`
  check off for every fixture except one, so 752 fixtures without `lang` are recorded as clean.
  Production vnu warns on them, and so does html-conform
  (`assertion.elements.html-missing-lang`). The differential test ignores that one rule when
  comparing, instead of the checker shipping less than vnu.

## Real-world pages

Besides the corpus, `xtask/fetch-real-world.sh` and `xtask/compare-real-world.sh` compare
html-conform with a locally running vnu jar on live websites. This is a manual check, not part
of CI, and not expected to reach 0 false positives: real pages carry their own markup errors.
See `xtask/README.md` in the repository.
