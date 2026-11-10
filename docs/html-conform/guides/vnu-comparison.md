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

| Result | `html` + `html-aria` | `html-math` | `html-rdfa` | `html-rdfalite` | `html-svg` | All |
| --- | --- | --- | --- | --- | --- | --- |
| true positives (vnu expects findings, html-conform reports some) | 3,745 | 18 | 28 | 2 | 0 | 3,793 |
| true negatives (both clean) | 909 | 30 | 184 | 54 | 5 | 1,182 |
| false positives (vnu clean, html-conform reports something) | 0 | 0 | 0 | 0 | 0 | 0 |
| false negatives (vnu expects findings, html-conform reports none) | 1 | 0 | 0 | 0 | 1 | 2 |
| not comparable (`check()` returned an error) | 0 | 0 | 0 | 0 | 0 | 0 |

CI runs this test on every push (`cargo test --release --test differential -- --ignored`) and
fails if false positives or false negatives of a tree rise above its committed baseline. `html`
and `html-aria` stay at 0 false positives, and so does every other tree. The two false negatives
are pinned by name in the test (CSS below and the nested SVG `<a>` of `html-svg`), so another
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

(Matched plus incidental equals the 3,793 true positives.) The baseline is committed in
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

- **A few tree-construction parse errors.** [`html5-parser`](https://crates.io/crates/html5-parser)
  0.5.0 records the spec's parse errors that earlier versions dropped on tokens it keeps: an end
  tag that closes an element while other elements inside it are still open
  (`<div><span></div>`), content after `</html>`, ignored tokens in the frameset modes. Its
  html5lib harness compares the reported errors with each case's `#errors`: 1,591 of 1,602 cases
  agree on presence; the 11 exceptions are listed in
  `crates/html5-parser/tests/html5lib_errors_known_failures.txt`. Exact error counts and
  positions are not yet compared, so this is presence-level agreement, not parity.
- **CSS inside `<style>`.** vnu hands style sheets to a vendored copy of the W3C CSS
  Validator. html-conform has no CSS parser for this, so a mistyped property such as `colr`
  goes unreported. This is the one false negative
  (`html/elements/style/css-property-error-novalid.html`), and a deliberate limitation:
  CSS correctness is out of scope (decision A of issue #87; checking property names or whole
  values would need its own normative basis and corpus). The test pins this fixture by name
  as the only allowed miss.
- **RDFa.** vnu's default schema includes RDFa 1.1 and so does html-conform (issue #173, see
  [RDFa](#rdfa) below). Both RDFa trees match vnu with no false positive and no false negative.

## Triage of vnu's other test trees

vnu's `tests/` holds more trees than `html` and `html-aria` (issue #86). Each one has one of
three outcomes:

| Tree | Files | Outcome | Why |
| --- | --- | --- | --- |
| `tests/html-math` | 48 | measured | 0 false positives, 0 false negatives |
| `tests/html-rdfa` | 212 | measured | 0 false positives, 0 false negatives |
| `tests/html-rdfalite` | 56 | measured | 0 false positives, 0 false negatives |
| `tests/html-svg` (not in the issue's list) | 522 | 6 vendored and gated; all 522 measured on demand | the 516 embed W3C SVG 1.1 test cases and are not vendored, see below |
| `tests/css` | 16 | out of scope | stand-alone style sheets for vnu's CSS checker; html-conform checks HTML only and has no CSS validation (issue #87) |
| `tests/xhtml` | 129 | out of scope | XML syntax; html-conform parses HTML only and plans no XHTML mode |
| `tests/svg` | 1 | out of scope | a stand-alone `.svg` document, not HTML syntax (inline SVG is `html-svg`) |
| `tests/langdetect` | 11 | out of scope | vnu's language-detection heuristics (character statistics over about 1.4 MB of text); html-conform has no language detector |
| `tests/normalization` | 1 | out of scope | a Unicode NFC warning on one text run; html-conform has no such check |
| `tests/html-its` | 320 (90 `.html`) | not vendored, measured on demand (matches vnu per file) | W3C ITS 2.0 test suite under the W3C Test Suite License, not MIT; no expected-result file; see below |
| `tests/schema-validation` | 3 | out of scope | exercises vnu's `--schema` option on XML files against a remote DocBook schema; see below |

A local run (not part of the test, nothing committed) of the out-of-scope trees that hold
`.html` files gave: `langdetect` 0 true positives, 4 true negatives, 1 false positive, 5 false
negatives; `normalization` 1 false negative. These are not gated.

### `html-its` and `schema-validation` (issue #175)

**`tests/html-its`** holds 320 files: 90 `.html` documents and 230 XML files (ITS rule files and
XML test documents, which html-conform does not read). The files carry no licence header. They
are the input files of the W3C ITS 2.0 test suite
([`w3c/its-2.0-testsuite`](https://github.com/w3c/its-2.0-testsuite)), which that repository
licenses under the W3C Test Suite License, not MIT; vnu's `LICENSE` does not cover them
(vnu has since edited some, for example added headings). vnu's `make` passes
`--ignore=html-its` to its test runner and `tests/messages.json` has no key for the tree, so
there is no expected result per file. Decision: **not vendored.** Taking the files would need a
licence decision for the repository (REUSE entries and a notice for a second licence), and
without an expected-result file the tree would only measure against our own guess that every
document is valid. The tree stays measurable locally from a checkout of the tag.

Local run (nothing committed, not gated; `html-conform` CLI and the vnu 26.8.20 jar on
`tests/html-its` of a shallow clone at the tag): of the 90 `.html` files, vnu reports nothing
but the missing-`lang` warning (46 files, 46 messages). Before issue #188 html-conform got 90
more findings in 25 files, every one `schema.html5` "unexpected attribute `its-…`"
(`its-annotators-ref` 17, `its-loc-quality-issue-type` 6, `its-locale-filter-list` 5, and 17
other `its-*` names). These were false positives: vnu's default schema (`html5-all`) includes
`its20-html5.rnc` (`build/build.py`: `include "its20-html5.rnc"` and
`common.attrs &= its-html-attributes`), which allows the `its-*` attributes in HTML.

Since issue #188 `xtask/vendor-schema.sh` vendors `its20-html5.rnc` and `its20-html5-types.rnc`
(MIT, Jirka Kosek, in vnu's `schema/its2/`) into `schema/its2/`, and `src/schema_source.rs`
includes them the way vnu's driver does. After that, html-conform reports only the
missing-`lang` warning on the same 46 files as vnu and nothing on the other 44: per-file
agreement with vnu is 90 of 90 (clean vs. findings). That compares presence of findings per
file, not their messages.

Decision (issue #175): **fetch on demand into a gitignored cache, nothing foreign-licensed in
the repository.** `xtask/fetch-w3c-trees.sh` downloads the tree at the pinned commit into
`xtask/.cache/w3c-trees/`; the opt-in test `tests/w3c_trees.rs` runs html-conform over it and,
for the per-file verdict of vnu, runs the pinned vnu jar (`xtask/.cache/vnu.jar`). Without the jar
or Java the vnu side is reported as NotRun, without the cache the test prints "not run: cache
missing". Measured with it (vnu 26.8.20, commit `388cb36`): of the 90 `.html` files html-conform
reports findings in 46 and vnu in 46, per-file agreement (clean vs. findings) is **90 of 90**.
Presence of findings per file is compared, not their messages. It is a measurement, not a gate,
and not part of CI.

**`tests/schema-validation`** has 3 files: a README and two XML documents (valid and invalid
DocBook 5), checked by `vnu --schema https://docbook.org/xml/5.1/rng/docbook.rng --xml`. The
test is for vnu's arbitrary-schema option and XML mode. html-conform parses HTML only, has no
schema option and does not vendor the DocBook schema, so the tree is **out of scope**. There is
nothing to measure.

### `html-svg` and the W3C copyright

516 of the 522 files in `tests/html-svg` are SVG 1.1 test cases of the W3C, each with the
header "Copyright 2009 World Wide Web Consortium ... All Rights Reserved". W3C publishes that
suite under its document license, which allows no modification and counts a subset as a
derivative work; this repository is MIT. `xtask/vendor-corpus.sh` therefore copies only the 6
files without that header (written by the vnu authors, MIT). Decision (issue #174): **fetch on demand into a gitignored cache, nothing foreign-licensed in
the repository.** `xtask/fetch-w3c-trees.sh` downloads `tests/html-svg`, `tests/html-its` and
`tests/messages.json` of the pinned commit into `xtask/.cache/w3c-trees/` (843 files, with a
`SHA256SUMS` manifest). The opt-in test `tests/w3c_trees.rs` takes the expectation as
`differential.rs` does (a file listed in `messages.json` expects findings, any other none; the
missing-`lang` warning is ignored for the same harness reason). Measured on all 522 files (the 6
vendored ones included), not gated and not part of CI:

| Result | `html-svg`, 522 files |
| --- | --- |
| true positives | 34 |
| true negatives | 484 |
| false positives | 0 |
| false negatives | 4 |
| not comparable | 0 |

The 4 false negatives, by class:

- **Undeclared `xmlns:*` prefix attributes on an SVG element** (2): `conform-viewers-03-f-novalid`
  (`xmlns:link` with a value other than the XLink namespace) and `fonts-overview-201-t-novalid`
  (`xmlns:xe` not allowed).
- **Unicode Normalization Form C** (1): `struct-cond-02-t-haswarn`; html-conform has no such
  check (see `normalization` above).
- **Nested SVG `<a>`** (1): `svg-a-nested-in-svg-a-novalid`, the one already pinned in the
  differential test.

### RDFa

vnu's default schema (`html5-all`, the one its test runner uses for every tree except `rdfalite`
and `xhtml`) is the HTML5 schema plus `html5-svg-mathml.rnc` and `rdfa.rnc` (vnu's `build.py`
generates `html5full-rdfa.rnc` as the HTML5 modules plus `include "rdfa.rnc"`). So RDFa 1.1
is accepted by default, and html-conform does the same: `rdfa.rnc` is vendored by
`xtask/vendor-schema.sh` and included by the schema entry point in `src/schema_source.rs`.
There is no option to turn it off.

vnu checks `tests/html-rdfalite` against a separate preset, `html5-rdfalite`: the same include
with `nonRDFaLite = notAllowed`, which drops `about`, `content`, `datatype`, `rel`, `rev` and
`inlist` from the RDFa attributes. html-conform offers it behind the `rdfa-lite` feature
(`SchemaPreset::RdfaLite`, `check_with_preset`): `src/schema_source.rs` composes the same entry point
with `include "rdfa.rnc" { nonRDFaLite = notAllowed }` (vnu's `schemaDriverHtml5RDFaLite`), and the
result is `prebuilt/html5-rdfalite.rngc`, a second compiled schema of about 246 KB. The default build
does not contain it. The corpus cannot tell the two presets apart: none of the 56 fixtures is invalid
only because of a non-Lite attribute, so `html-rdfalite` measures 0 false positives and 0 false
negatives under both. With the feature, `tests/differential.rs` runs the tree under the Lite preset
as vnu's `TestRunner` does and gates it with its own baseline.

Differences measured on the two trees (before issue #173: 177 and 50 false positives; before
issue #181: 2 and 0 false positives, 5 and 2 false negatives): none.

- **`rev` on `<a>`** (`html-rdfa/0006`, `0007`). `rdfa.rnc` leaves `rel` and `rev` out of the RDFa
  attributes of `<a>`; vnu accepts `rev` there from `legacy.rnc` (`a.attrs.rev`), a driver html-conform
  does not vendor. Local patch `xtask/schema-patches/0003-a-rev-legacy.patch` copies that definition.
- **`xmlns:prefix` on HTML elements** (`html-rdfa/0147`, `0179`, `0183`, `0258`, `0319` and
  `html-rdfalite/0147`, `0319`). vnu reports "Attribute `xmlns:xmlzzz` not allowed here", and so
  does html-conform: the infoset layer keeps `xmlns:*` on HTML elements and drops it only on SVG and
  MathML elements, where it is a namespace declaration.
- **`link` and `meta`.** `<link property resource>` without `rel`/`href` is valid in the body, and
  `resource` replaces `href` on `<link>`; `rules/elements.sch` accepts both.

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
