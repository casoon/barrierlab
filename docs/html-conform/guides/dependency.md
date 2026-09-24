---
title: "Using html-conform as a dependency"
description: "The public API, the one option, error handling, and patterns for tests, tools and services."
order: 1
---

The whole public API is two functions and a handful of types, all re-exported from the crate
root:

| Item | Purpose |
| --- | --- |
| `check(html: &str)` | Check a complete document with the default options |
| `check_with_options(html: &str, options: CheckOptions)` | Same, with explicit options |
| `CheckOptions` | `include_parse_errors: bool` (default `true`) |
| `CheckReport` | `findings: Vec<Finding>`, plus `has_errors()` |
| `Finding` | `rule_id`, `severity`, `message`, `location` |
| `Severity` | `Error`, `Warning`, `Info` |
| `SourceLocation` | one-based `line` and `column`, zero-based `byte_offset` |
| `CheckError` | a setup failure of the checker itself |

## Input

`check()` takes the complete document as a `&str`, so decoding bytes is up to the caller. For
UTF-8 input, `std::fs::read_to_string` or `String::from_utf8` is enough. The document is
always parsed with HTML5 error recovery, the same way a browser would build its tree; there
is no separate XML or XHTML mode.

## Options

`CheckOptions` has one field. Set `include_parse_errors` to `false` to drop the recovered
parser diagnostics (`parser.html5`) from the report:

```rust
use html_conform::{CheckOptions, check_with_options};

let options = CheckOptions {
    include_parse_errors: false,
};
let report = check_with_options(html, options)?;
```

Schema, assertion, script, CSP and table findings are always included; the option only
removes parser diagnostics.

## Errors

`CheckError` is returned only when the checker cannot run at all: the embedded HTML schema
failed to compile or the embedded rule set failed to parse. A document that is merely
non-conforming always produces `Ok` with findings. `CheckError` is `#[non_exhaustive]` and
implements `std::error::Error`, so `?` into `Box<dyn Error>` or `anyhow::Error` works.

## Reading a report

- Findings come in parser order, followed by the later layers: schema, assertions, scripts,
  CSP, table grid.
- `has_errors()` is true if at least one finding has `Severity::Error`. Warnings, such as
  CSP violations or a missing `lang` attribute, do not count.
- `location` is `None` when a finding cannot be mapped to a source range, for example a
  schema finding on an element the parser inserted implicitly (a `<head>` that is not in the
  source).
- Match on `rule_id` rather than on `message`. Rule IDs are listed in
  [Findings and rule IDs](../rule-ids/).

## Patterns

### Fail a test on invalid markup

```rust
#[test]
fn rendered_page_is_conforming() {
    let html = render_page(); // your templating
    let report = html_conform::check(&html).expect("checker starts");
    assert!(!report.has_errors(), "{:#?}", report.findings);
}
```

### Only look at errors

```rust
use html_conform::Severity;

let errors: Vec<_> = report
    .findings
    .iter()
    .filter(|finding| finding.severity == Severity::Error)
    .collect();
```

### Emit JSON

`CheckReport`, `Finding`, `Severity` and `SourceLocation` implement serde's `Serialize` and
`Deserialize`. With `serde_json`:

```rust
let json = serde_json::to_string_pretty(&report)?;
```

produces an object with a `findings` array. One entry, the last finding of the table grid
sample in the showcase:

```json
{
  "rule_id": "tables.integrity",
  "severity": "Error",
  "message": "Table cell overlaps an earlier table cell.",
  "location": {
    "line": 12,
    "column": 9,
    "byte_offset": 270
  }
}
```

This is the format of the `examples/showcase/*.json` files behind the
die Beispiele des Crates, written by `examples/findings.rs`.
