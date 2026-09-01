# html-conform

A pure Rust library for HTML5 specification conformance checking — comparable in precision and output to the [Nu Html Checker (vnu)](https://validator.github.io/validator/), but without a JVM, subprocesses, or HTTP network requests. Embeddable directly into any Rust application, CLI, or web service.

---

## 🎯 Conformance Profile & Metrics

`html-conform` is validated continuously against the official W3C/vnu differential test suite (**4,655 test fixtures** vendored from [`validator/validator@388cb36`](https://github.com/validator/validator/commit/388cb36)).

- **Precision Floor:** **0 False Positives (`BASELINE_FALSE_POSITIVE = 0`)** — zero false alarms across all 4,655 test cases.
- **Accuracy:** **99.7 % overall accuracy** across the entire corpus (**3,734 True Positives**, **909 True Negatives**).
- **Residual False Negatives:** **12 / 4,655** — 5 need a full 2D table grid model with `colspan`/`rowspan` overlap detection (not reasonably expressible in the Schematron/XPath 1.0 layer), and 7 are smaller individually-researched gaps (an ARIA tab/tabpanel heuristic, `dl` duplicate-term edge cases, ruby markup advisories, inline CSS property validation). None of these are false alarms — each is a documented, deliberate limitation, not a silent gap. (Real tree-construction-error tracking, formerly the largest gap at 43 fixtures, landed via [`html5-parser`](https://crates.io/crates/html5-parser) 0.3.0.)

---

## 🔍 Validation Layers

`html-conform` combines **five independent finding sources** into a single, unified `CheckReport`:

1. **HTML5 Tree Construction (`parser.html5`)** — Spec-compliant, error-tolerant tree parsing via [`html5-parser`](https://crates.io/crates/html5-parser). Emits tokenizer, DOCTYPE, and tree-construction parse findings with line, column, and byte offset locations.
2. **Grammar & Content Model (`schema.html5`)** — Validation against the full vendored W3C RELAX NG schema ([`relax-ng`](https://crates.io/crates/relax-ng)), including SVG 1.1 and MathML 3 subtrees.
3. **Custom Datatype Micro-Syntaxes (`w:*`)** — Full spec-compliant datatype validation for 50 custom W3C attribute microsyntaxes (`w:image-candidate-strings` for `srcset`, `w:content-security-policy`, `w:media-query`, `w:datetime`, `w:iri-ref`, BCP 47 language tags, etc.).
4. **Schematron Co-Constraints (`rules/*.sch`)** — High-precision assertion rules via [`schematron-engine`](https://crates.io/crates/schematron-engine) and [`xpath-eval`](https://crates.io/crates/xpath-eval) (ARIA 1.2 constraints, structural HTML restrictions, heading hierarchy, link/script attribute combinations).
5. **Script & CSP Validation (`scripts.import-map`, `scripts.speculation-rules`, `csp.meta-enforcement`)** — Dedicated JSON validation for `<script type="importmap">` / `<script type="speculationrules">` contents, and Content Security Policy (`<meta http-equiv="Content-Security-Policy">`) enforcement against inline scripts/styles via [`csp-parse`](https://crates.io/crates/csp-parse).

---

## 🚀 Usage

Add `html-conform` to your `Cargo.toml`:

```toml
[dependencies]
html-conform = "0.1.0"
```

### Basic Check

```rust
use html_conform::check;

fn main() {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head><title>Test Document</title></head>
<body><p>Hello world</p></body>
</html>"#;

    let report = check(html).expect("checker execution succeeded");

    for finding in &report.findings {
        println!(
            "[{:?}] {} at line {:?}: {}",
            finding.severity, finding.rule_id, finding.location, finding.message
        );
    }

    if report.has_errors() {
        eprintln!("Document has conformance errors!");
    }
}
```

### Fine-Grained Options

```rust
use html_conform::{CheckOptions, check_with_options};

let options = CheckOptions {
    include_parse_errors: false, // exclude tokenizer/parser diagnostics
};

let report = check_with_options(html, options).unwrap();
```

---

## 🏗️ Architecture

```
                 HTML Source String / Document
                               │
                               ▼
                    ┌─────────────────────┐
                    │  1. html5-parser    │  WHATWG Tree Construction
                    └─────────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                  ▼
  ┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
  │ 2. relax-ng      │ │ 3. Schematron │ │ 4. JSON / CSP    │
  │    (Schema &     │ │    (Co-Con-   │ │    (Import-Maps, │
  │    Datatypes)    │ │    straints)  │ │    Speculation,  │
  └──────────────────┘ └───────────────┘ │    CSP)          │
            │                  │         └──────────────────┘
            └──────────────────┼──────────────────┘
                               │
                               ▼
                   ┌───────────────────────┐
                   │ CheckReport           │
                   │  Vec<Finding>         │
                   └───────────────────────┘
```

---

## 🔄 Maintenance & Refinement Loops

`html-conform` enforces quality through structured maintenance loops:

- **Loop A (Schema Sync):** Mechanical updates when W3C RELAX NG schemas or vnu upstream specifications update (`xtask/vendor-corpus.sh`).
- **Loop B (Assertion Refinement Loop):** Iterative refinement of Schematron rules and datatype checkers against the 4,655-fixture differential test suite, strictly maintaining the **0 False Positive floor**.
- **Loop C (Real-World Sanity Check):** Supplementary, manual/non-CI signal that fetches real websites and diffs `html-conform`'s findings against a locally-run Nu Html Checker (vnu) jar (`xtask/fetch-real-world.sh` + `xtask/compare-real-world.sh`, see [`xtask/README.md`](xtask/README.md)). Not part of the pinned 4,655-fixture corpus baseline, and not expected to hit 0 false positives — real pages carry their own unrelated markup errors.

---

## 📜 License & Attributions

- **Code:** MIT License — see [`LICENSE`](LICENSE) or [`LICENSES/MIT.txt`](LICENSES/MIT.txt). This is a [REUSE](https://reuse.software/)-compliant project.
- **Third-Party & Vendored Assets:** the RELAX NG schema and test corpus (`schema/`, `tests/corpus/`) are vendored from [`validator/validator`](https://github.com/validator/validator) under the MIT License, not authored by this project — see [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).
