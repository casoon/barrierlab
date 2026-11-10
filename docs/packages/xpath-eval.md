---
title: "xpath-eval"
description: "XPath 1.0 parser and evaluator with an opt-in XPath 2.0 mode, generic over the caller's document model."
order: 13
---

Parses and evaluates XPath expressions against a document the caller provides
through the `Document`/`Node` traits. It builds no tree and parses no XML or
HTML itself. Used by `schematron-engine`.

Two languages, selected explicitly:

| Mode | Entry points | Normative |
|---|---|---|
| XPath 1.0 (default) | `parse`, `evaluate`, `evaluate_boolean` | W3C XPath 1.0 |
| XPath 2.0 (subset) | `parse_versioned(src, XPathVersion::V2)`, `evaluate_versioned`, `evaluate_boolean_versioned` | W3C XPath 2.0 (Second Edition), XQuery 1.0 and XPath 2.0 Functions and Operators |

## Conformance

XPath 1.0: measured against the Jaxen XPath test cases (281 case runs: 263
pass, the 18 failures are functions outside XPath 1.0 and the unbound-prefix
policy) and locked by own tests in `tests/xpath10_regression.rs`.

XPath 2.0: locked by `tests/xpath20.rs` (spec section named per test) and
measured against the XPath expressions of the EN 16931 validation Schematron.
Categories, known failures, coverage numbers and how to rerun the externally
fetched corpora: [`CONFORMANCE.md`](../../crates/xpath-eval/CONFORMANCE.md).

## XPath 2.0 mode

### Why a separate mode

The e-invoice rule sets (EN 16931, XRechnung, Peppol BIS) are Schematron with
`queryBinding="xslt2"`. XPath 2.0 is not 1.0 plus operators: the value model
(sequences of atomic values and nodes instead of four object types), the
comparison rules (`'1' = 1` is true in 1.0 and a type error in 2.0), the number
types (exact `xs:decimal` instead of IEEE doubles) and the function rules
differ. So the 1.0 parser, AST and evaluator are untouched and 2.0 lives in its
own module (`src/xpath2/`), sharing only the axes, the `Node`/`Document`
traits and the `EvaluationContext` hooks (namespaces, variables).

- **1.0 is the default and stays normative for 1.0.** `XPathVersion::default()`
  is `V1`; `parse`/`evaluate` keep their signature and behaviour; the existing
  test suites pass unchanged.
- **The mode is a value, not a feature flag.** `parse_versioned` returns an
  `Expression` that remembers its version, so one binary evaluates 1.0 and 2.0
  rule sets side by side. A 2.0-only construct in 1.0 mode is a parse error.
- **`schematron-engine` keeps using XPath only through `xpath-eval`** and
  chooses the mode from the schema: `XPathVersion::from_query_binding` maps
  `xslt`/`xpath`/absent to `V1`, `xslt2`/`xpath2` to `V2` and everything else to
  `None` (stays `UnsupportedQueryBinding`). Wiring that into the engine is a
  follow-up (it also needs `Expression` in `Rule`/`LetBinding` and the SVRL
  path), not part of the first slice.

### Value model

`Sequence<N> = Vec<Item<N>>`, `Item<N> = Node(N) | Atomic(Atomic)`. No nested
sequences, the empty sequence is the empty vector. Atomic types, chosen by what
the rule sets use:

| Type | Representation | Notes |
|---|---|---|
| `xs:untypedAtomic` | `String` | what atomizing a node gives: there is no schema, no node has a type annotation |
| `xs:string`, `xs:boolean` | `String`, `bool` | |
| `xs:integer` | `i64` | overflow raises `FOAR0002` instead of wrapping |
| `xs:decimal` | own `Decimal`, `i128` mantissa and scale | exact, 38 digits, no new dependency; `div` keeps 18 fractional digits (F&O §6.2.4); an operation that does not fit raises `FOAR0002` |
| `xs:double` | `f64` | |
| `xs:date` | year, month, day | no timezone: a date with timezone is *unsupported*, not silently compared |

Types outside this table (`xs:dateTime`, durations, `xs:float`, ...) are
reported as unsupported. Dynamic errors carry the standard code (`XPTY0004`,
`FORG0001`, `FOAR0001`, ...), static ones too (`XPST0017` unknown function,
`XPST0008` unbound variable, `XPST0081` unbound prefix). An unbound prefix is a
static error in 2.0 mode; the 1.0 evaluator keeps its "matches nothing" policy.

### Never a silent success

Anything valid XPath 2.0 that is not implemented fails with
`VersionedError::Unsupported("<feature>")` (`is_unsupported()`), at parse time
where the syntax is recognised (`instance of`, `treat as`, `element()` tests,
`*:name`, the namespace axis, unsupported atomic types) and at evaluation time
for functions and values (`fn:matches`, `xs:dateTime(...)`, date arithmetic).
`schematron-engine` reports such a check as `NotRun` with that reason. Genuine
errors (`XPTY0004`, ...) are errors, not "false".

`VersionedError` is the one error type of the versioned entry points for both
versions (it also wraps the 1.0 `EvalError`). A function that F&O 1.0 or XSLT 2.0
defines, at an arity it defines, but that is not implemented (including
`key`, `document`, `current`, `format-number`, collation arguments) is
`Unsupported`; `XPST0017` is only for names and arities in neither (an explicit
table in `src/xpath2/functions.rs`).

### Limits

- `xs:decimal` holds at most 38 digits (fraction digits included). A literal,
  cast or constructor beyond that is `Unsupported`; an operation whose result
  does not fit is `FOAR0002`; `div` keeps 18 fractional digits, fewer if the
  scaled dividend does not fit.
- `xs:integer` is 64-bit; overflow is `FOAR0002`.
- Nesting: the 2.0 parser rejects an expression whose syntax tree would be
  deeper than 256 levels with a parse error. A parenthesis, function argument,
  predicate or `if`/`for` branch counts 8 levels (about 32 nested levels), each
  link of a left-leaning `+`, `*`, `|`, `/` chain counts 1 (the sum of nesting
  and chain length is bounded); `or`/`and` chains are evaluated without
  recursion and limited to 4096 operands. This keeps the recursive parser and
  evaluator inside a debug-build stack. (The 1.0 parser has no such limit.)

### Inventory: EN 16931 validation artefacts 1.3.17

Source: `ConnectingEurope/eInvoicing-EN16931`, tag `validation-1.3.17`, the
preprocessed `ubl` and `cii` `EN16931-*-validation-preprocessed.sch` (both
`queryBinding="xslt2"`, EUPL-1.2, fetched on demand and never committed:
`crates/xpath-eval/xtask/vendor-en16931.sh`). Every `@test` of an
`assert`/`report` and every rule `@context`:

- 2010 expressions (1791 tests, 219 contexts), 1763 distinct; UBL 1085, CII 925.
  There are no `let` bindings in the preprocessed files.
- 261 expressions (13 %) contain a construct that exists only in XPath 2.0 (UBL
  185, CII 76). The other 1749 are plain 1.0 syntax.
- Constructs and functions, by number of expressions using them (overlapping):

| Construct or function | Expressions |
|---|---:|
| path step that is a function call or parenthesised expression (`cac:TaxScheme/normalize-space(cbc:ID)`, `//(a\|b)/c`) | 161 |
| `upper-case()` | 133 |
| `exists()` | 115 |
| `xs:decimal()` constructor (exact amount arithmetic: `round(sum(...) * 10 * 10) div 100`) | 61 |
| `every`/`some ... satisfies` | 9 |
| `ends-with()` | 8 |
| `abs()` | 6 |
| `xs:date()` and date comparison | 2 |
| `distinct-values()` | 2 |
| `matches()` (regular expression) | 1 |

Not used at all: `if/then/else`, `for`, `to`, value comparisons (one `eq`),
`castable as`, `tokenize`, `replace`, `string-join`, `round-half-to-even`,
date/duration arithmetic, regex flags. Remaining 1.0 functions (`not`, `count`,
`normalize-space`, `sum`, `round`, ...) behave differently in 2.0 only in their
argument and result types, which is why they are part of the 2.0 library too.

The inventory covers one of the three rule sets named in issue #62. XRechnung
(KoSIT) and Peppol BIS Billing 3.0 are not inventoried yet; they are expected
to add `matches()` and `replace()`/`tokenize()` and string-to-codepoints for
check digits, and possibly `xsl:function` helpers (a `schematron-engine`
matter).

### Slices

Share = expressions of the EN 16931 inventory that need 2.0 and become
parseable and evaluable (of 261).

| # | Slice | Unlocks |
|---|---|---|
| 1 | **Done.** Mode switch; sequences, `,`, `to`; atomic types above; exact decimal arithmetic; general, value and node comparisons; `if`, `for`, `some`/`every`; `cast as`/`castable as`; `xs:` constructors; path steps that are expressions; `union`/`intersect`/`except`; ~45 functions (`exists`, `empty`, `upper-case`, `lower-case`, `ends-with`, `string-join`, `abs`, `round`/`floor`/`ceiling`, `sum`/`avg`/`min`/`max`, `distinct-values`, `concat` and the string library, ...) | 260 of 261 (99.6 %) |
| 2 | Regular expressions: `matches`, `replace`, `tokenize` with flags. Needs the XPath/XSD regex dialect: translate to the `regex` crate (MIT/Apache-2.0) or a small own engine; maintainer decision | 1 of 261 here, more in XRechnung/Peppol |
| 3 | Rest of the F&O 1.0 string and sequence library: `round-half-to-even`, `string-to-codepoints`, `codepoints-to-string`, `compare`, `subsequence`, `index-of`, `insert-before`, `remove`, `root`, `lang`, `id` | 0 here (Peppol check digits expected) |
| 4 | Date and time: `xs:dateTime`, `xs:time`, durations, timezones for `xs:date`, `current-date()`, component functions, date arithmetic | 0 here (`xs:date` comparison without timezone is in slice 1) |
| 5 | Sequence types: `instance of`, `treat as`, kind tests with names (`element(a)`), `*:name`, user-callable `xsl:function` bridge | 0 here |
| 6 | `schematron-engine`: accept `xslt2`/`xpath2` bindings, run in 2.0 mode, report `Unsupported` as `NotRun` | enables the rule sets end to end |

Out of scope for `xpath-eval` altogether: XSD validation, XSLT, XQuery.
