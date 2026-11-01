---
title: "Quick Start"
description: "Build a document, check it with and without semantics, and read the report."
order: 2
---

## Structure only

The simplest host is the `Arena` from `a11y-dom`. `run` checks everything that is decidable without role, name and
rendering:

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

// Not assessed: the tier 2 and tier 3 rules. They are not missing from the report,
// they appear in it with `NotRun::CapabilityMissing`.
assert_eq!(report.summary.rules_not_run, 7);
```

The example is the doctest from `a11y-rules` and runs with `cargo test`.

## With semantics

Rules for links, buttons and SVG need a real accessible name. A host gets it
by implementing `Semantics`, here with `accname`. The ID index is built once and
kept, not built per call:

```rust
use a11y_dom::{Arena, ArenaNode, Document, Node, Semantics};
use a11y_rules::run_with_semantics;
use accname::IdIndex;

struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl Document for MitSemantik<'_> {
    type N<'n> = ArenaNode<'n> where Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

// `doc` as in the first example.
let host = MitSemantik { ids: IdIndex::build(doc.root()), doc: &doc };
let report = run_with_semantics(&host);
```

The same host is used in the rule test (`crates/a11y-rules/tests/rules.rs`) and in the generator of
this page's examples (`examples/src/main.rs`).

## Which entry point

| Host provides | Entry point |
|---|---|
| structure only | `run` |
| + semantics | `run_with_semantics` |
| + rendering | `run_with_rendering` |
| both | `run_full` |

## Reading the report

`Report` is serializable with `serde`. What the JSON looks like is shown by
the examples in [`examples/a11y/`](https://github.com/casoon/barrierlab/tree/main/examples/a11y): a teaser document checked once with structure only and once with
semantics. What the fields mean is described under
[Findings and Reports](../../concepts/befunde/).
