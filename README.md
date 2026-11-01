# BarrierLab

**One rule set for web audits.** Accessibility rules (WCAG 2.2), HTML conformance,
perception and web checks as pure-Rust libraries, with the accessibility rules also
shipped as WebAssembly. A finding means the same thing whether the page was checked at
build time, in a terminal audit or live in the browser.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/a11y-rules.svg?label=a11y-rules)](https://crates.io/crates/a11y-rules)
[![npm](https://img.shields.io/npm/v/@casoon/a11y-wasm.svg?label=%40casoon%2Fa11y-wasm)](https://www.npmjs.com/package/@casoon/a11y-wasm)

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/img/architecture.dark.svg">
    <img src="docs/img/architecture.light.svg" width="900" alt="Flow chart: parsers and shared models feed a11y-rules and html-conform; those and a11y-perception and web-checks ship as 15 Rust crates on crates.io, a11y-rules additionally as WebAssembly on npm; auditmysite, astro-post-audit and Relief use the crates, liveaudit the npm package.">
  </picture>
</p>

**This repository contains libraries only.** The tools that use them live in their own
repositories and pull the packages from crates.io and npm.

## Why

Two audit tools that check the same page tend to grow two copies of the same check, and
the copies drift. When `web-checks` was extracted, auditmysite classified `GPTBot` as a
training bot and astro-post-audit as a citation bot, so they would have reported opposite
things about the same `robots.txt`. BarrierLab is where those checks live once.

- **One finding model.** Every package reports through `a11y-report`: rule ID, outcome,
  severity, WCAG criteria, location, stable JSON.
- **Pure libraries, no browser.** A package takes data and computes. It never opens a
  page, fetches a URL or speaks CDP, so the same code runs in a CLI, a build step and a
  browser tab.
- **"Did not run" is not "passed".** If a rule's prerequisite is missing (no accessible
  names, no computed styles, no focus), it appears in the report as `NotRun` with a
  reason. There is no silent success.
- **Dependencies point down.** Four layers, no cycles, no host code in the repository.

## What is in here

| Layer | Crates | What they do |
|---|---|---|
| **L0** parsers | `html5-parser` · `csp-parse` · `media-query-parse` · `xpath-eval` · `relax-ng` · `schematron-engine` | WHATWG HTML tree construction, CSP3 and Media Queries grammars, XPath 1.0, RELAX NG, ISO Schematron |
| **L1** models | `a11y-dom` · `a11y-report` · `stylesheet-parse` · `html-conform` | document model as a trait, the shared finding model, CSS with `@media` context, HTML5 conformance without a JVM |
| **L2** computation | `accname` · `a11y-rules` · `a11y-perception` · `web-checks` | accessible name and description, 152 rule IDs, reading order and snapshot diffs, robots / meta / OpenGraph / JSON-LD / hreflang |
| **L3** delivery | `a11y-wasm` (crate + npm `@casoon/a11y-wasm`) | the rule set inside a live page |

All 15 crates are on crates.io. One page per package, with dependencies and limits:
[package overview](https://casoon.github.io/barrierlab/docs/packages/).

## Examples

### Check a document against the accessibility rules

`a11y-rules` is generic over a document model. `Arena` from `a11y-dom` is the simplest
host: structure only.

```rust
use a11y_dom::Arena;
use a11y_rules::run;

let doc = Arena::builder()
    .open("html")
        .open("body")
            .open("img").attr("src", "logo.png").close()
        .close()
    .close()
    .build();

let report = run(&doc);

// Found: no lang, no title, no alt.
assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));
assert!(report.findings.iter().any(|f| f.rule_id == "document/lang-missing"));

// Not judged: the rules that need semantics or rendering. They are in the
// report as `NotRun::CapabilityMissing`, not silently absent.
assert_eq!(report.summary.rules_not_run, 7);
```

This is the doctest from `a11y-rules`; it runs with `cargo test`.

### Read the report

A host that supplies structure only gets the rules that need nothing else. The same page
with accessible names added gets more, and says so. From
[`examples/a11y/`](examples/a11y), a small teaser page:

| Host supplies | fail | review | untested | rules not run |
|---|---:|---:|---:|---:|
| structure | 2 | 4 | 8 | 57 |
| structure + semantics | 3 | 14 | 8 | 22 |

```json
{
  "rule_id": "images/alt-missing",
  "outcome": "fail",
  "severity": "high",
  "message": "The image has no alt attribute.",
  "location": { "node": "15" },
  "wcag": ["1.1.1"]
}
```

Four outcomes exist: `fail`, `review` (needs a human decision), `pass` and `untested`
(manual checks the machine cannot make). The rule IDs are stable across every surface.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/img/rules-by-tier.dark.svg">
    <img src="docs/img/rules-by-tier.light.svg" width="720" alt="Bar chart: 95 structure rules, 35 semantics rules, 17 rendering rules and 5 stylesheet rules, 152 rule IDs in total.">
  </picture>
</p>

Each rule declares what it needs. A rule that needs rendering cannot even be called with a
host that does not provide it; the boundary is in the type system, not in a runtime
`Option`.

### Check HTML conformance, no JVM

```rust
let html = std::fs::read_to_string("index.html")?;
let report = html_conform::check(&html)?;

for finding in &report.findings {
    let at = finding.location.map(|l| l.to_string()).unwrap_or_default();
    println!("{:?} {at} {} {}", finding.severity, finding.rule_id, finding.message);
}
```

For a document with `Fish &chips; for dinner.` and a duplicated `class` attribute it prints:

```text
Error 8:17 parser.html5 an unknown named character reference
Error 9:25 parser.html5 a duplicate attribute on a tag
```

`html-conform` combines parser diagnostics, the W3C RELAX NG schema, attribute
microsyntaxes, Schematron co-constraints, `importmap` / `speculationrules` / CSP checks
and a table-grid check. It is measured against the Nu Html Checker's own test suite:

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/img/vnu-agreement.dark.svg">
    <img src="docs/img/vnu-agreement.light.svg" width="720" alt="Bar chart of html-conform against 4,655 vnu fixtures: 3,745 true positives, 909 true negatives, 1 false negative, 0 false positives.">
  </picture>
</p>

CI fails if either error count rises. What the comparison measures, and what it does not,
is spelled out in the [vnu comparison](docs/html-conform/guides/vnu-comparison.md).

### Run the same rules in a page

```sh
npm i @casoon/a11y-wasm
```

`@casoon/a11y-wasm` is the rule set compiled to WebAssembly. A collector in the page hands
it the tree as columns, it returns the report as JSON; which rules run depends on the
columns supplied (structure, semantics, rendering). Releases are built in CI and published
with npm provenance (`npm audit signatures`). See the
[package page](https://casoon.github.io/barrierlab/docs/packages/a11y-wasm/).

### Use the parsers on their own

The L0 crates are independent of the audit code. `xpath-eval` and `schematron-engine` are
generic over the document model and work outside HTML; `csp-parse` and `media-query-parse`
parse a single grammar and nothing else.

```sh
cargo add xpath-eval
cargo add csp-parse
```

## Tools built on this

| Tool | What it does | Uses |
|---|---|---|
| [auditmysite](https://github.com/casoon/auditmysite) | audits rendered pages in the terminal | `a11y-rules`, `a11y-dom`, `a11y-report`, `accname`, `html-conform` |
| [astro-post-audit](https://github.com/casoon/astro-post-audit) | audits a finished Astro build | the same, plus `web-checks` |
| [liveaudit](https://github.com/casoon/liveaudit) | audits the running page in the browser | `@casoon/a11y-wasm` |

Who uses what, and why a package lives here at all:
[docs/consumers.md](docs/consumers.md).

## Ground rules

- Dependencies only point down; no package imports one of its own or a higher layer.
- No browser up to L3: no `chromiumoxide`, no CDP types in a public API.
- No host format in the core: documents arrive through a trait, not a built-in HTML or XML parser.
- No `unsafe` without a reason and a comment; `#![forbid(unsafe_code)]` where possible.
- Spec first: each package names its normative source (WHATWG HTML, CSP3, Media Queries 4,
  XPath 1.0, RELAX NG, ISO Schematron, WCAG 2.2, accname 1.2). Other implementations are a
  comparison point, not the norm.
- A new rule points at a case from a real corpus. Without evidence it does not go in.

Architecture and conventions: [`docs/`](docs/). Releases: one tag per package
(`<package>-vX.Y.Z`), a changelog per crate; the four a11y crates are versioned together
([docs/releasing.md](docs/releasing.md)).

## Documentation

**<https://casoon.github.io/barrierlab/>**: overview, package pages, the accessibility
guide and the html-conform guide.

## License

MIT
