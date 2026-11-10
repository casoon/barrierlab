# XPath 1.0 conformance

## What was tested

`xpath-eval` was run against the test cases of the
[Jaxen](https://github.com/jaxen-xpath/jaxen) XPath library
(`core/xml/test/tests.xml` plus the sample documents it refers to).

| | |
|---|---|
| Upstream | `jaxen-xpath/jaxen`, path `core/xml/` |
| Pinned revision | `e675b18efa486234ed364483184ae8ce493c48d1` |
| Licence | BSD-3-Clause-style (Jaxen Project, upstream `LICENSE.txt`) |
| Checksum of `tests.xml` | SHA-256 `46c5c8f4acf69f1d3ce5e4773fc91ad4901d64e5379ef152a8d8f7d37eef204b` |
| Run on | 2026-11-05, `xpath-eval` 0.2.3 (unreleased state) |
| Runner | `tests/jaxen.rs` (adapter over `roxmltree`, see below) |

Result: **281 case runs, 263 pass, 18 fail**. All 18 failures are expected
and listed below; none is a deviation inside XPath 1.0 core, one is a
deliberate policy.

A "case run" is one `<test>` or `<valueOf>` element evaluated against one
context node (a `<context>` that selects several nodes runs its cases once per
node). The runner follows Jaxen's own generator: a `<context>`'s namespace
declarations become the namespace context, its attributes in the `var`
namespace become string variables; `<test count>` checks the size of the
node-set result, `<test exception>` expects a parse or evaluation error,
`<valueOf>` compares `string(select)`.

## Results by category

The corpus has no categories of its own; the cases were grouped by what the
expression exercises (first matching rule wins, checked by hand).

| Category | Cases | Pass | Fail |
|---|---:|---:|---:|
| Axes (all 13: descendant, ancestor, sibling, preceding, following, parent, self) | 31 | 31 | 0 |
| Child and attribute steps, abbreviated paths (`//`, `.`, `..`, `*`, `@`) | 24 | 24 | 0 |
| Node tests: `comment()`, `processing-instruction()`, `text()` | 18 | 18 | 0 |
| Predicates (boolean, `and`/`or`, `!=`) | 20 | 20 | 0 |
| Positional predicates (`[n]`, `position()`, `last()`, reverse axes) | 10 | 10 | 0 |
| Node-set functions: `count()` | 15 | 15 | 0 |
| Node-set functions: `name()`, `local-name()` | 31 | 31 | 0 |
| String functions (`string`, `concat`, `substring`, `translate`, `normalize-space`) | 33 | 33 | 0 |
| Boolean functions (`lang`) | 8 | 8 | 0 |
| Number functions (`sum`) | 2 | 2 | 0 |
| Arithmetic (associativity of `-`, `div`, `mod`; `+`, `*`) | 5 | 5 | 0 |
| Comparison operators, incl. node-set comparison (existential semantics) | 24 | 24 | 0 |
| Variables | 5 | 5 | 0 |
| Namespaces: name tests with a namespace context | 18 | 17 | 1 |
| Namespaces: the namespace axis | 12 | 12 | 0 |
| Syntax errors that must be rejected | 8 | 8 | 0 |
| Extension functions beyond XPath 1.0 | 17 | 0 | 17 |
| **Total** | **281** | **263** | **18** |

Gaps of the corpus itself (not of the crate): `id()`, `contains`,
`starts-with`, `substring-before/after`, `floor`/`ceiling`/`round`,
`boolean`/`not`, and `|` unions are not exercised at all, `lang` and `sum` only
lightly. These are covered by the own tests below.

## Known failures (18)

Recorded in `tests/jaxen_known_failures.txt` (one line per case, our own file).
The runner checks the list both ways: a new failure and a stale entry both
fail the run.

| Cases (`tests.xml` lines) | Count | Reason |
|---|---:|---|
| 178-183 | 6 | `evaluate()` is a Jaxen extension function, not part of the XPath 1.0 core library. |
| 489, 493, 500, 507 | 4 | `document()` is an XSLT function, not XPath 1.0; `xpath-eval` never loads documents. |
| 564-568 | 5 | `upper-case()` / `lower-case()` are XPath 2.0 / Jaxen functions. |
| 569-570 | 2 | `ends-with()` is XPath 2.0 / Jaxen. |
| 629 | 1 | Unbound namespace prefix (`x:a` with no binding for `x`). XPath 1.0 section 2.3 makes this an error; `xpath-eval` deliberately lets the name test match nothing (documented on `EvaluationContext`). The caller owns the namespace context and a missing binding is a normal state for hosts that evaluate untrusted expressions. |

So 17 of 18 failures are functions outside XPath 1.0 (the crate implements the
27 functions of the core library and reports `UnknownFunction` for anything
else); the 18th is the unbound-prefix policy.

## Own regression tests

Because the corpus is not in the repository, the level reached is locked by
`tests/xpath10_regression.rs`: documents and expressions written for this crate
from the specification, with the spec section named in the comments. It covers
every category above plus the corpus gaps: existential node-set comparison
(`=` and `!=` both true), empty node-set against boolean, NaN in all
comparisons, `position()`/`last()` on reverse axes, string-values of every node
type, `substring` rounding with NaN/infinity, `translate` edge cases, `lang`,
`id()` on documents without ID information (empty node-set, no error), unions
in document order, the namespace axis, the unbound-prefix behaviour, and
rejected syntax and evaluation errors. Run with `cargo test -p xpath-eval`
(also in CI).

## Rerunning the Jaxen suite

```sh
cd crates/xpath-eval
xtask/vendor-jaxen.sh                              # fetch into tests/corpus/jaxen
cargo test -p xpath-eval --test jaxen -- --ignored
```

The script fetches the pinned revision with a sparse checkout, checks the
checksum of `tests.xml` and copies only the documents it uses. After changing
the pinned revision, re-bless the known failures after examining every entry:
`JAXEN_BLESS=1 cargo test -p xpath-eval --test jaxen -- --ignored`.

## Why the corpus is not in the repository

The sample documents (`moreover.xml` news feed data, `nitf.xml`,
`much_ado.xml`, ...) carry no licence notice of their own, so no third-party
corpus file is kept in this repository. `tests/corpus/` is gitignored; the
runner is marked `#[ignore]`, so a plain `cargo test` reports it as *ignored*
(never as a silent pass) and CI does not need the corpus. The files from the
earlier vendoring (PR #119) remain in the git history.

## Test adapter

`tests/common/mod.rs` implements the crate's `Document`/`Node` traits over
`roxmltree` (arena in document order, the `xml` namespace node on every
element, no ID attributes). It is test-only code; the crate itself has no
document model and no XML dependency.

# XPath 2.0 mode

The opt-in XPath 2.0 mode (`XPathVersion::V2`, design in
[`docs/packages/xpath-eval.md`](../../docs/packages/xpath-eval.md)) is checked
two ways.

## Own tests

`tests/xpath20.rs` and the unit tests in `src/xpath2/` are written from XPath 2.0
(Second Edition) and XQuery 1.0 and XPath 2.0 Functions and Operators, with the
section named in the comments: version switch and default, conditional,
quantified, `for` and range expressions, general/value/node comparison,
arithmetic and exact decimals, path steps that are expressions, set operators,
casts and constructors, dates, the function library, lexical forms and
precedence, syntax errors, and the three failure classes (unsupported feature,
static error, dynamic error). The regex functions have their own file,
`tests/xpath20_regex.rs`: every example of F&O 7.6.2 to 7.6.4, each of the
error codes `FORX0001` to `FORX0004`, the grammar of XML Schema Appendix F,
the flags, and pathological patterns and inputs (`(a*)*b` against 5,000 `a`,
`(a|aa)+$`, 600 nested groups, `(a{1000}){1000}`, 200,000-character inputs)
that must finish with a result or a named error. Run with `cargo test -p xpath-eval` (also in CI).

## EN 16931 coverage

Source of expressions: the preprocessed UBL and CII validation Schematron of
`ConnectingEurope/eInvoicing-EN16931`, tag `validation-1.3.17`
(`queryBinding="xslt2"`, EUPL-1.2, fetched into the gitignored
`tests/corpus/en16931` and never committed).

| | |
|---|---|
| Files | `ubl/schematron/preprocessed/EN16931-UBL-validation-preprocessed.sch` SHA-256 `64b93f26acadd19de9792cec9e085628271c6fe3870ecacbf4e7271272f2ef08`, `cii/schematron/preprocessed/EN16931-CII-validation-preprocessed.sch` SHA-256 `74fa94df0513881b9c8e369650bb2282f9c8a30bb52ce4ce43d1219e88ccedc2` |
| Run on | 2026-11-10, `xpath-eval` 0.2.3 plus the XPath 2.0 mode and its regex functions (unreleased) |
| Runner | `tests/en16931.rs` (`#[ignore]`) |

Every `@test` (assert/report) and rule `@context`, 2010 expressions, is parsed
and evaluated against a small own sample invoice per syntax (UBL, CII), the
document element as context node for tests and the root for contexts.
"Evaluates" means it returns a value instead of an error. The sample is not a
conforming invoice, so the results are not judged; a rule run on the wrong
context (for example a `.` comparison on the document element) can legitimately
fail, and in XPath 1.0 mode an unknown function only fails when the evaluation
reaches it (`and`/`or` short-circuit), so the 1.0 evaluate figure is an upper
bound.

| Expressions | 1.0 mode: parse | 1.0 mode: evaluate | 2.0 mode: parse | 2.0 mode: evaluate |
|---|---:|---:|---:|---:|
| UBL (1085) | 981 | 903 | 1085 | 1085 |
| CII (925) | 890 | 877 | 925 | 924 |
| **Total (2010)** | **1871 (93.1 %)** | **1780 (88.6 %)** | **2010 (100 %)** | **2009 (99.95 %)** |

2.0 mode, the 1 that does not evaluate is a `. = ...` amount comparison
evaluated on the document element instead of the amount element it belongs to
(a type error `FORG0001`, which is correct: the harness context, not the
engine). The `matches()` expression that was *unsupported* in slice 1 now
evaluates (own regex engine, slice 2). Run on 2026-11-10 with the regex
functions. 261 of the 2010 expressions contain a construct that exists only
in XPath 2.0; none of them can give a correct result in 1.0 mode.

```sh
cd crates/xpath-eval
xtask/vendor-en16931.sh                            # fetch into tests/corpus/en16931
cargo test -p xpath-eval --test en16931 -- --ignored --nocapture
```
