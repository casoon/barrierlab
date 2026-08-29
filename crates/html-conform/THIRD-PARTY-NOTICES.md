# Third-Party Notices & Licenses

This project contains third-party components and vendored assets from external projects.

---

## 1. W3C / Nu Html Checker (vnu) Schema & Test Corpus

- **Source Repository:** [validator/validator](https://github.com/validator/validator)
- **Commit / Tag:** `388cb36`
- **Vendored Locations:**
  - `schema/` (RelaxNG schemas: `html5/`, `svg11/`, `mathml3-inc.rnc`, `html5-all.rnc`)
  - `tests/corpus/` (W3C HTML / HTML-ARIA differential test suite)
- **License:** MIT License (Copyright (c) 2007-2026 Mozilla Foundation, Henri Sivonen, and W3C)

---

## 2. Rust Dependencies

`html-conform` relies on open-source crates published on crates.io:

| Crate | Version | License | Source / Purpose |
|---|---|---|---|
| `html5-parser` | `0.2.0` | MIT | WHATWG HTML5 tokenizer/tree-construction parser |
| `relax-ng` | `0.2.0` | MIT | ISO/IEC 19757-2 RELAX NG schema engine |
| `schematron-engine` | `0.2.0` | MIT | ISO/IEC 19757-3 Schematron assertion engine |
| `xpath-eval` | `0.2.2` | MIT | W3C XPath 1.0 evaluation engine |
| `media-query-parse` | `0.1.0` | MIT | CSS Media Queries Level 4 parser |
| `csp-parse` | `0.1.0` | MIT | W3C Content Security Policy Level 3 parser |
| `encoding_rs` | `0.8.35` | MIT / Apache-2.0 | Character encoding detection/decoding |
| `url` | `2.5.8` | MIT / Apache-2.0 | WHATWG URL parsing |
| `serde` | `1.0.229` | MIT / Apache-2.0 | Serialization framework |
| `serde_json` | `1.0.151` | MIT / Apache-2.0 | JSON parsing/serialization |

`html5-parser`, `relax-ng`, `schematron-engine`, `xpath-eval`, `media-query-parse`,
and `csp-parse` are sister projects by the same author (`casoon`), each MIT-licensed
and published independently on crates.io.
