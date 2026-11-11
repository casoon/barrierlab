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

## XPath 2.0 schemas

`queryBinding="xslt2"` or `"xpath2"` (ISO/IEC 19757-3:2016, 5.4.4, Query language
binding) selects the XPath 2.0 mode of `xpath-eval` for every `test`, `let`,
`value-of` and `name` expression of the schema; `xslt`, `xpath` and an absent
binding stay XPath 1.0 and behave as before. Any other binding is a parse error.

- The mode is the Cargo feature `xpath2`, on by default. Without it (the workspace crates
  depend on `schematron-engine` with `default-features = false`, to keep the
  `html-conform-wasm` size budget) `xslt2`/`xpath2` are rejected as before.
- A `<let>` is bound to a sequence with its atomic types, through the
  sequence-valued variable hook of `EvaluationContext`. `value-of` writes the
  atomized items of its sequence joined by one space (XSLT 2.0, 11.4.1).
- A rule `context` is a pattern: the nodes `descendant-or-self::node()/(context)`
  selects from the root. A context that starts with `/` and has no `|` is
  evaluated from the root alone (the result is the same). The 1.0 rewrite into
  per-node tests is not used, so a 2.0 schema is slower to match than a 1.0 one.
- What `xpath-eval` does not implement is reported, not skipped:
  `SchematronEvalError::Query { unsupported, message }` carries the
  message of the `xpath-eval` error, and `is_unsupported()` tells an incomplete run
  from a failed one. Static and dynamic XPath errors use
  the same variant; syntax errors keep `Parse`.
- `xsl:function` declarations are run, see below.
- A rule `context` may read the variables in scope (`cac:Party[$supplierCountry = 'NO']` in the
  Peppol rules) and call the functions of the schema.

Measured end to end with the preprocessed EN 16931 UBL rules (release
validation-1.3.17, fetched on demand by `crates/xpath-eval/xtask/vendor-en16931.sh`,
`cargo test -p schematron-engine --test en16931 -- --ignored`): the 981 checks of
104 rules run on the two UBL example invoices of that release (211 and 193 rule
firings) without any fired check, and a copy without `cbc:IssueDate` and with a
changed payable amount fires `BR-03` and `BR-CO-16`.

## XSLT functions

Peppol BIS Billing 3.0 and XRechnung declare helper functions with `xsl:function` (check sums,
IBAN) and call them from `assert/@test`. With the feature `xpath2` the engine runs them: the
declarations are read into `Schema.functions` (`XsltFunction`; `Schema.xslt_functions` keeps the
names), and `xpath-eval` asks the engine for a function in a namespace other than `fn` and `xs`
(`EvaluationContext::functions`). This is not an XSLT processor, only the subset the inventoried
rule sets use:

| Instruction | Notes |
|---|---|
| `xsl:param` | before every other instruction; `@as` converts the argument |
| `xsl:variable` | `@select`; text content (bound to the text as `xs:untypedAtomic`, not as a document node); a sequence constructor if `@as` is given; visible to the following siblings and their descendants |
| `xsl:sequence` | `@select` |
| `xsl:value-of` | `@select`, `@separator` (no attribute value template); yields an `xs:string` |
| `xsl:text`, literal text | text (white-space-only text is dropped) |
| `xsl:if`, `xsl:choose`/`xsl:when`/`xsl:otherwise` | |

Anything else (`xsl:for-each`, `xsl:copy-of`, literal result elements, `xsl:param` defaults, a
variable with a sequence constructor and no `@as`, ...) leaves the function declared but marked:
`XsltFunction::unsupported` names the first such construct, and a call ends the evaluation with
`SchematronEvalError::Query` for which `is_unsupported()` is `true`. Nothing is skipped and
nothing passes silently.

- **Types.** `@as` on `xsl:param`, `xsl:variable` and `xsl:function` follows the function
  conversion rules (XPath 2.0, 3.1.5.2) for `item()`, `node()` and `xs:string`, `boolean`,
  `integer`, `decimal`, `double`, `date`, `untypedAtomic` with `?`, `*`, `+`: atomization, an
  untyped value cast to the expected type, `xs:integer` promoted to `xs:decimal` and
  `xs:double`, then the occurrence check; a mismatch is `XPTY0004`. Any other declared type
  (`element()`, `xs:dateTime`, ...) is unsupported when the function is called.
- **No focus.** The body has no context item: `.`, `/`, a path or `position()` is `XPDY0002`.
- **Names.** The function name and the prefixes in its body resolve through the schema's `ns`
  elements first (the ISO skeleton declares them on the generated stylesheet; XRechnung's `xr`
  prefix is bound there only), then the namespace declarations in scope; `xs`, `fn`, `xml` are
  predefined.
- **Global variables.** A body reads the schema-level `<let>`s and the `<let>`s of every pattern
  (XSLT compiles both to global variables, and XRechnung's `xr:checkIBAN` reads
  `$XR-IBAN-REGEX` from a pattern); it does not see the `<let>`s of a rule or a phase. A pattern
  `<let>` that fails to evaluate before its pattern runs is left unbound, so a body reading it gets
  `XPST0008`.
- **Recursion.** Calls may nest `xpath_eval::MAX_FUNCTION_DEPTH` (64) deep; the next one is the
  dynamic error `FOER0000` instead of a stack overflow (the depth check is in `xpath-eval`, the
  interpreter's own nesting of instructions is limited to 32 levels). `u:addPIVA` recurses once
  per character of an Italian VAT number.
- **Arity.** Functions are found by namespace, local name and number of arguments; no match is the
  static error `XPST0017`.

### Deviations from the standards

- **Variables of a pattern without rules.** ISO/IEC 19757-3:2016 scopes a `<let>` to its
  pattern. XRechnung's `common.sch` puts its constants in a pattern that has `<let>`s and no rule
  ("solely serves for declaring global variables (in XSLT speak)") and reads them from every
  other pattern; that works because the XSLT implementations compile pattern variables to global
  variables. In XPath 2.0 mode such a pattern, and only such a pattern, declares its variables for the whole schema.
  The `<let>`s of a pattern with rules stay scoped to it (schxslt `let-pattern-global` remains a
  known difference).
- **`xsl:variable` text content** is bound as `xs:untypedAtomic`, not as a document node.
- **`xsl:value-of` in a function** yields an `xs:string`, not a text node, and adjacent
  instructions are not merged into one text node.

### Measured

Rule sets and example invoices are fetched on demand by
`crates/xpath-eval/xtask/vendor-en16931.sh` (every file pinned by SHA-256, never committed) and run
by `cargo test -p schematron-engine --test en16931 -- --ignored`:

| Rule set | Functions | Examples | Result |
|---|---:|---|---|
| Peppol BIS Billing 3.0 UBL (OpenPEPPOL `peppol-bis-invoice-3`, `38d693d`) | 13 | `base-example`, `Allowance-example` | nothing fires; a GLN with a wrong check digit fires `PEPPOL-COMMON-R040` (`u:gln`) |
| Peppol BIS Billing 3.0 CII | 12 | EN 16931 `CII_example1` (Peppol publishes no CII example) | not a Peppol invoice, so 10 Peppol rules fire, each for a stated reason (`PEPPOL-EN16931-R120` among them: a returned line with a positive quantity and a negative amount); with the missing electronic addresses added `PEPPOL-EN16931-R010` and `-R020` disappear, and a wrong GLN in them fires `PEPPOL-COMMON-R040` |
| XRechnung 3.0 UBL (KoSIT `xrechnung-schematron`, `7e3ddbe`) | 1 | test suite `01.01a`, `02.01a`, `03.01a` | only the information-level `BR-DE-TMP-32` on `01.01a` (no delivery date); a wrong IBAN check digit fires `BR-DE-19` (`xr:checkIBAN`) |
| XRechnung 3.0 CII | 2 | the same three cases | the same |

The Peppol CII rule set declares the assert id `PEPPOL-EN16931-R043` twice; an id is unique in a
Schematron document and the parser refuses it, so the test renames the second one in memory.

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
