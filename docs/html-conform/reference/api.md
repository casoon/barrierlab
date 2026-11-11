---
title: "API overview"
description: "The public items of the crate at a glance. Item-level documentation lives on docs.rs."
order: 1
---

The full, versioned API documentation is on
[docs.rs/html-conform](https://docs.rs/html-conform). This page lists what is there.

## Functions

### `check`

```rust
pub fn check(html: &str) -> Result<CheckReport, CheckError>
```

Checks a complete HTML document with `CheckOptions::default()`.

### `check_with_options`

```rust
pub fn check_with_options(html: &str, options: CheckOptions) -> Result<CheckReport, CheckError>
```

Checks a complete HTML document with explicit options. Always uses HTML5 error recovery.

## Types

### `CheckOptions`

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `include_parse_errors` | `bool` | `true` | Include recovered parser diagnostics (`parser.html5`) |

### `CheckReport`

| Member | Meaning |
| --- | --- |
| `findings: Vec<Finding>` | Parser findings first, then the later validation layers |
| `skipped: Vec<Skipped>` | Unchecked classes that occur in this document (`SkippedClass::StyleElementCss`, `SkippedClass::CspHashSource`); empty for a document without them |
| `has_errors() -> bool` | Whether any finding has `Severity::Error` |
| `to_report() -> a11y_report::Report` | The same result in the shared finding model |

`to_report` maps `Error` to `Fail`/`Medium`, `Warning` to `Review`/`Low` and `Info` to
`Untested`/`Low`, keeps `rule_id` and `message`, and writes the location as `line:column` into
`source_hint`. Each `Skipped` becomes an `InputKind::DocumentIncomplete` note, so content the
checker did not look at is not read as passed. `SkippedClass` is `#[non_exhaustive]`.

### `RangedReport` and `check_with_ranges`

```rust
pub fn check_with_ranges(html: &str, options: CheckOptions) -> Result<RangedReport, CheckError>
```

Like `check_with_options`, plus the source range of each finding where its layer knows the end.
`RangedReport` keeps its fields private so findings and ranges cannot get out of step: `report()`,
`into_report()`, `ranges() -> &[Option<SourceRange>]` (parallel to the findings),
`findings()` (an iterator of `(&Finding, Option<SourceRange>)`) and `to_report()`. `SourceRange` is
`#[non_exhaustive]`. A `SchemaPreset` cannot be combined with ranges yet. `SourceRange { start, end }` holds two
`SourceLocation`s; `end` is exclusive. `RangedReport::to_report()` carries it as `location.end` of the shared
`a11y_report` model.

### Rule catalogue

```rust
pub fn rules() -> Result<Vec<RuleInfo>, CheckError>
```

Every rule id the checker can report, with `layer` (`RuleLayer`), `outcome`, a one-line
`description`, `normative_source` and `help_url` (the rule layer's section of the rule ID guide). The assertion rules are read from the parsed
`rules/*.sch` files the checker runs, the other ids from the constants their findings use, so
the list cannot drift. `parser.html5` and `schema.html5` are one id each for many conditions.
Assertion rules have no `normative_source`: the rule files carry none. `html-conform --list-rules`
prints the catalogue as JSON.

### `Finding`

| Field | Type |
| --- | --- |
| `rule_id` | `String`, see [Findings and rule IDs](../../guides/rule-ids/) |
| `severity` | `Severity` |
| `message` | `String` |
| `location` | `Option<SourceLocation>` |

### `Severity`

`Error`, `Warning`, `Info`.

### `SourceLocation`

| Field | Type | Meaning |
| --- | --- | --- |
| `line` | `u32` | one-based |
| `column` | `u32` | one-based |
| `byte_offset` | `usize` | zero-based |

Displays as `line:column`.

#### Positions and ranges

- `line` counts line breaks; LF, CR and CRLF each count once.
- `column` counts Unicode scalar values (chars) from the start of the line. A tab, `é` and an
  emoji are one column each; this is neither bytes nor UTF-16 code units. A host that needs
  UTF-16 columns (the language server protocol) converts from `byte_offset` against its text.
- `byte_offset` counts UTF-8 bytes in the `&str` that was checked.
- A range's `end` is the position just past the last character.

Which layers fill the end (`check_with_ranges`, counted over the vendored vnu corpus in
`tests/finding_ranges.rs`): `schema.html5`, `assertion.*`, `scripts.*`, `tables.integrity` and
`csp.meta-enforcement` findings anchored at an element get that element's start tag as range, always, also when the finding is about the element's content
or end (the part from `<` to `>`; not the end tag or content, and not an attribute's own span, which the
parser does not record). Findings about an element the parser synthesized, or anchored at text,
have no end. **`parser.html5` has no end position**: the parser reports a point, not a token
span.

### `CheckError`

`#[non_exhaustive]` enum; the document was not checked. Implements `std::error::Error`.

| Variant | Meaning |
| --- | --- |
| `DocumentTooDeep { depth: usize, limit: usize }` | the parsed tree nests elements `depth` levels deep, more than `limit` (512); no findings are produced |
| `Initialization { message: String }` | a checker component could not be set up |

## Serialization

`CheckReport`, `Finding`, `Severity` and `SourceLocation` derive serde's `Serialize` and
`Deserialize`. `serde` is a regular dependency, not a feature flag.
