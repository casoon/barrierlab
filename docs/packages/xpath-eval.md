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
fetched corpora: [`CONFORMANCE.md`](https://github.com/casoon/barrierlab/blob/main/crates/xpath-eval/CONFORMANCE.md).

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
for functions and values (a regex back-reference, `xs:dateTime(...)`, date arithmetic).
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
- Variables: the 2.0 evaluator reads `EvaluationContext::sequence_variables` (a variable bound to a sequence with its atomic types) first, then the 1.0 `variables` hook (a 1.0 value becomes a sequence of one item). `sequence_to_string` renders a sequence the way `value-of` does: atomized, joined by one space.
- Nesting (both modes): the parser rejects an expression whose syntax tree would be
  deeper than 256 levels with a parse error ("nested or chained too deeply"),
  never a stack overflow. A parenthesis, function argument or predicate counts 8
  levels (about 31 nested levels); in 2.0 an `if`/`for` branch counts 8 as
  well. Each link of a left-leaning `+`, `*`, `=`, `<`, `|` chain and each unary
  `-` counts 1, and in 1.0 each `or`/`and` link too (the sum of nesting and chain
  length is bounded: 248 links of `1+1+...` fit, 249 do not). The 2.0 evaluator
  walks `or`/`and` chains without recursion and limits them to 4096 operands. This
  keeps the recursive parser, evaluator and drop of the tree inside a debug-build
  stack. A location path is a flat list of steps and has no length limit in 1.0.
  The evaluator relies on this: a tree built by hand instead of through `parse`
  is not depth-checked.

### Regular expressions

`matches`, `replace` and `tokenize` (all arities; F&O 1.0 section 7.6 on XML
Schema Part 2 Appendix F) run on an own engine in `src/xpath2/regex/`, no
dependency. The pattern is parsed (`parse.rs`), compiled to a Thompson NFA
program (`vm.rs`, counted quantifiers expanded) and run as a Pike VM with
capture slots: all threads advance over the input in lock step in priority
order, so a search is O(program x input), nothing recurses on the input, and
leftmost / first-alternative / greedy-or-reluctant semantics follow from the
thread order. `matches` tracks no captures, `tokenize` only the match span,
`replace` only the groups its replacement string uses.

Supported: branches, `? * + {n} {n,} {n,m}` and the reluctant forms, groups
(capturing only), `.`, anchors `^ $`, classes with ranges, negation and
subtraction, the single-character escapes of the grammar and `\$`,
`\s \S \d \D \w \W`, `\p{..}`/`\P{..}` as below, flags `s m i x` in any order
and repetition, `$N` (F&O 7.6.3 digit rule), `\\` and `\$` in replacements, and the errors
`FORX0001` (a flag other than `s m i x`), `FORX0002` (invalid pattern,
including a quantifier after a quantifier such as `a{2}{3}`, an empty class, a
reversed range, an unknown category), `FORX0003` (`replace`/`tokenize` with a
pattern that matches the empty string, also for an empty input), `FORX0004`
(replacement with a stray `$` or `\`; checked even when nothing matches). Flag
`x` removes space, tab, CR and LF outside classes, also after a backslash (as
the F&O example `hello\ sworld` requires).

Unicode, exactly what is implemented. There is no Unicode database in this
crate. Every question about a character answers yes, no or "cannot decide",
and "cannot decide" for a character the match actually tests is `Unsupported`
naming that code point (a character the match never tests, or one an earlier
class member already accepts, costs nothing):

- Decided for every character: `\s` (space, tab, CR, LF); the categories `Cc`,
  `Z`, `Zs`, `Zl`, `Zp` (fixed sets); ranges and literals; the blocks
  `IsBasicLatin`, `IsLatin-1Supplement`, `IsLatinExtended-A`,
  `IsLatinExtended-B`, `IsGreek`/`IsGreekandCoptic`, `IsCyrillic`, `IsHebrew`,
  `IsArabic`, `IsGeneralPunctuation`, `IsCurrencySymbols`, `IsHiragana`,
  `IsKatakana`, `IsCJKUnifiedIdeographs`.
- Decided for U+0000..=U+00FF only (a table of the Unicode General_Category of
  those 256 code points, checked by a unit test): every other category and the
  one-letter groups (`\p{L}`, `\p{Lu}`, `\p{N}`, `\p{P}`, `\p{S}`, `\p{C}`, ...),
  `\d` (`\p{Nd}`), `\w` (everything except `P`, `Z`, `C`, so `_` is not a word
  character and `+` is). Beyond U+00FF: `Unsupported`.
- `\i` and `\c` (name characters): ASCII only (`[A-Za-z_:]` and
  `[A-Za-z0-9_:.-]`); the non-ASCII tables differ between XML 1.0 editions.
  Beyond ASCII: `Unsupported`.
- Any other `\p{Is...}` block: `Unsupported` (it may or may not exist). A name
  that is neither a category nor `Is...`, and `Cs`, is `FORX0002`.
- `-` in a class follows XML Schema Part 2 F.1: it is not an `XmlChar`, so it is no
  endpoint of `s-e` (`[--a]`, `[+--]`, `[a--]` are `FORX0002`), and it is valid
  only first, last, as the range operator or before a subtraction (`[a-c-e]` is
  `FORX0002`); `\-` is a single-character escape and may start a range
  (`[a\--z]`).

Flag `i` (F&O 7.6.1.1): C2 is a case-variant of C1 iff `lower-case(C1) eq
lower-case(C2)` or `upper-case(C1) eq upper-case(C2)` on the one-character
strings, with the full Unicode mappings std provides (the same ones the
function library uses). A literal or a range stands for its members and their
case-variants; this is applied to the ranges before a class is negated or
subtracted from, as the specification's examples require (`[^Q]` excludes `Q`
and `q`, `[A-Z-[IO]]` excludes `I O i o`). So `ſ` (U+017F) matches `s`, `ı`
(U+0131) matches `i` and `I`, `İ` (U+0130) matches neither (its lower case is
two characters), the Kelvin sign matches `k`. Escapes (`\w`, `\p{Lu}`, ...) are
not affected by the flag. A range class spanning more than 131,072 characters
under `i` is `Unsupported` (the case-variants are enumerated).

Limits, each a named `Unsupported`, never a wrong result:

- Back-references `\N` (they need backtracking, which this engine does not
  do). A reference to a group that does not exist or is not closed is
  `FORX0002`.
- The compiled program holds at most 10,000 instructions: `{n,m}` is expanded,
  so `a{100000}` and `(a{1000}){1000}` are refused, `a{2000}` is not.
- At most 100 capture groups, and class subtractions nested at most 100 deep
  (groups are bounded by the group cap, so 600 nested groups is refused as
  "more than 100 capture groups").
- Step budget: one call (all matches of a `replace` or `tokenize` together)
  may spend 8 thread steps per program instruction and input character (plus
  100,000), at most 2,000,000,000. A search costs about two steps per instruction
  and character, so this is linear work with margin (`a{50}b` over a megabyte
  fits); it is the protection against quadratic re-scanning (`a.*z|a` over many
  `a`). Beyond it the call stops with an error naming the budget.
- `replace` produces at most 16 Mi characters, or twice its input if that is
  more (`$1` over many matches); `tokenize` returns at most 1,000,000 tokens.

Follow-ups, not in this slice: back-references with a step budget; a vendored
Unicode table (categories, name characters, blocks) so that
`\p{L}`, `\d`, `\w`, `\i` and `\c` decide every character.

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
| 2 | **Done, with gaps (see "Regular expressions").** `matches`, `replace`, `tokenize` with flags on an own engine (maintainer decision: no dependency). Open: back-references, a Unicode database for categories, `\i`/`\c`, blocks | 1 of 261 here, more in XRechnung/Peppol |
| 3 | **Partly done.** `string-to-codepoints` and `codepoints-to-string` (Peppol check digits, see "Inventory: Peppol and XRechnung"). Not implemented, and not used by any inventoried rule set: `round-half-to-even`, `compare`, `codepoint-equal`, `subsequence`, `index-of`, `insert-before`, `remove`, `exactly-one`/`zero-or-one`/`one-or-more`, `deep-equal`, `root`, `lang`, `id`, `encode-for-uri`, `error`, `trace` (all `Unsupported`) | 0 here; Peppol only through `xsl:function` bodies |
| 4 | Date and time: `xs:dateTime`, `xs:time`, durations, timezones for `xs:date`, `current-date()`, component functions, date arithmetic | 0 here (`xs:date` comparison without timezone is in slice 1) |
| 5 | Sequence types: `instance of`, `treat as`, kind tests with names (`element(a)`), `*:name`, user-callable `xsl:function` bridge | 0 here |
| 6 | **Done.** `schematron-engine` accepts `xslt2`/`xpath2`, runs the schema in 2.0 mode and reports `Unsupported` as an error that says so (`SchematronEvalError::is_unsupported`; the engine has no `NotRun` model, that is `a11y-report`). Needs the sequence-valued variable hook of `EvaluationContext` (`sequence_variables`) and `sequence_to_string` from this crate | EN 16931 UBL end to end: see `schematron-engine` |

### Inventory: Peppol and XRechnung

Fetched on 2026-11-11, not committed: `OpenPEPPOL/peppol-bis-invoice-3`
`rules/sch/PEPPOL-EN16931-UBL.sch` and `-CII.sch` (master, commit
`38d693d71d28d813781323a6ca97324edcc4d239`), and `itplr-kosit/xrechnung-schematron`
`src/validation/schematron/{ubl/XRechnung-UBL-validation.sch,cii/XRechnung-CII-validation.sch,common.sch}`
(master, commit `7e3ddbe224674427fd174a8406635708b0d50ed1`). Every `test`,
`context`, `let/@value` and `select` attribute (including the `xsl:function`
bodies):

| File | Expressions | Parse in 2.0 mode |
|---|---:|---:|
| Peppol UBL | 346 | 346 |
| Peppol CII | 207 | 207 |
| XRechnung UBL | 97 | 97 |
| XRechnung CII | 90 | 90 |
| XRechnung `common.sch` | 20 | 20 |

Functions called beyond the EN 16931 set: `tokenize`, `replace`, `matches`,
`translate`, `reverse`, `string-join`, `substring-after`, `codepoints-to-string`,
`string-to-codepoints`, `round`, `last`, `xs:integer`, `xs:string`, `xs:date`, plus the
constructs `if`, `for`, `to`, `castable as`. All but the two codepoint functions
were already implemented; those two are now.

The part that is not reachable from this crate: the rule sets declare helper
functions with `xsl:function` (Peppol: 12 in CII and 13 in UBL, for example
`u:mod11`, `u:gln`, `u:checkCF`, recursive `u:addPIVA`; XRechnung: `xr:checkIBAN`)
and call them from `assert/@test`. Running them needs an XSLT function bridge
(`xsl:param`, `xsl:variable`, `xsl:sequence`, `xsl:choose`, recursion) in
`schematron-engine`. That is not done; the engine names the function and reports
the run as unsupported (see its documentation).

Out of scope for `xpath-eval` altogether: XSD validation, XSLT, XQuery.
