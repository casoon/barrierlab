---
title: "Schnellstart"
description: "Ein Dokument aufbauen, mit und ohne Semantik prüfen und den Bericht lesen."
order: 2
---

## Nur Struktur

Der einfachste Host ist die `Arena` aus `a11y-dom`. `run` prüft alles, was ohne Rolle, Name und
Darstellung entscheidbar ist:

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

// Gefunden: kein lang, kein title, kein alt.
assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));
assert!(report.findings.iter().any(|f| f.rule_id == "document/lang-missing"));

// Nicht beurteilt: die Tier-2- und Tier-3-Regeln. Sie fehlen nicht im Bericht,
// sie stehen mit `NotRun::CapabilityMissing` darin.
assert_eq!(report.summary.rules_not_run, 7);
```

Das Beispiel ist der Doctest aus `a11y-rules` und läuft mit `cargo test`.

## Mit Semantik

Regeln für Links, Buttons und SVG brauchen einen echten Accessible Name. Ein Host bekommt ihn,
indem er `Semantics` implementiert, hier mit `accname`. Der Index der IDs wird einmal gebaut und
gehalten, nicht je Aufruf:

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

// `doc` wie im ersten Beispiel.
let host = MitSemantik { ids: IdIndex::build(doc.root()), doc: &doc };
let report = run_with_semantics(&host);
```

Derselbe Host steckt im Regeltest (`crates/a11y-rules/tests/rules.rs`) und im Generator der
Beispiele dieser Seite (`examples/src/main.rs`).

## Welcher Einstieg

| Host liefert | Einstieg |
|---|---|
| nur Struktur | `run` |
| + Semantik | `run_with_semantics` |
| + Darstellung | `run_with_rendering` |
| beides | `run_full` |

## Den Bericht lesen

`Report` ist mit `serde` serialisierbar. Wie das JSON aussieht, zeigt der
[Showcase](../../../showcase/): ein Teaser-Dokument einmal nur mit Struktur und einmal mit
Semantik geprüft. Was die Felder bedeuten, steht unter
[Befunde und Berichte](../../concepts/befunde/).
