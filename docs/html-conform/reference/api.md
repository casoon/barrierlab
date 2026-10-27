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
| `has_errors() -> bool` | Whether any finding has `Severity::Error` |

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

### `CheckError`

`#[non_exhaustive]` enum; the document was not checked. Implements `std::error::Error`.

| Variant | Meaning |
| --- | --- |
| `DocumentTooDeep { depth: usize, limit: usize }` | the parsed tree nests elements `depth` levels deep, more than `limit` (512); no findings are produced |
| `Initialization { message: String }` | a checker component could not be set up |

## Serialization

`CheckReport`, `Finding`, `Severity` and `SourceLocation` derive serde's `Serialize` and
`Deserialize`. `serde` is a regular dependency, not a feature flag.
