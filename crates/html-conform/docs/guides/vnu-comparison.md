---
title: Comparison with vnu
description: What the differential test against the Nu Html Checker measures, what html-conform covers, and where it differs.
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
| **Total** | **4,655** |

vnu's expected results say, per fixture, whether it should produce findings. The test counts
agreement:

| Result | Count |
| --- | --- |
| true positives (vnu expects findings, html-conform reports some) | 3,745 |
| true negatives (both clean) | 909 |
| false positives (vnu clean, html-conform reports something) | 0 |
| false negatives (vnu expects findings, html-conform reports none) | 1 |
| not comparable (`check()` returned an error) | 0 |

CI runs this test on every push (`cargo test --release --test differential -- --ignored`) and
fails if false positives or false negatives rise above those numbers.

### What the numbers do not say

The comparison is per fixture: does the document produce findings or not. It does not compare
rule IDs, message text or positions. A fixture counts as a true positive when html-conform
reports anything, even if it reports a different problem than vnu does. Treat 0 false
positives as "html-conform does not flag documents vnu considers clean, on this corpus", not as
finding-by-finding parity.

## Covered

- **The same schema.** The RELAX NG schema in `schema/` is vnu's, vendored unchanged,
  including SVG 1.1 and MathML 3.
- **Parser diagnostics, microsyntaxes, co-constraints**: see
  [Findings and rule IDs](../rule-ids/) for each layer.
- **Import maps, speculation rules and meta CSP**: checked by dedicated Rust code.
- **Table integrity**: the 2D cell grid, which was once the largest group of false
  negatives.

## Not covered, or different

- **CSS inside `<style>`.** vnu hands style sheets to a vendored copy of the W3C CSS
  Validator. html-conform has no CSS parser for this, so a mistyped property such as `colr`
  goes unreported. This is the one false negative
  (`html/elements/style/css-property-error-novalid.html`), and a deliberate limitation.
- **Parts of vnu's test suite that are not vendored.** `tests/css`, `tests/xhtml`,
  `tests/svg`, `tests/html-rdfa`, `tests/html-rdfalite`, `tests/html-math`, `tests/html-its`,
  `tests/langdetect`, `tests/normalization` and `tests/schema-validation` are excluded from the
  corpus, so nothing is measured for them.
- **HTML syntax only.** The input is parsed as HTML with browser-style recovery; there is no
  XHTML (XML syntax) mode.
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
