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
