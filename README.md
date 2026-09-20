# a11y-core

Gemeinsame Bausteine für Accessibility-Werkzeuge in Rust. Ein Regelbestand
bedient drei Oberflächen mit identischen Regelkennungen und identischem JSON:

| Oberfläche | Werkzeug | Substrat |
|---|---|---|
| Build-Zeit | [astro-post-audit](https://github.com/casoon/astro-post-audit) | statisches HTML aus `dist/` |
| CI / Crawl | [auditmysite](https://github.com/casoon/auditmysite) | Chrome via CDP, nativer Accessibility-Tree |
| laufende Seite | [liveaudit](https://github.com/casoon/liveaudit) | DOM der Seite, über WASM |

## Crates

| Crate | Zweck | Stand |
|---|---|---|
| [`a11y-report`](crates/a11y-report) | Befund- und Berichtsmodell, JSON-Vertrag | 0.10.1 |
| [`a11y-dom`](crates/a11y-dom) | Dokumentmodell-Abstraktion plus Fähigkeits-Tiers | 0.10.1 |
| [`accname`](crates/accname) | WAI-ARIA Accessible Name und Role Computation | 0.10.1 |
| [`a11y-rules`](crates/a11y-rules) | die Regeln, generisch über das Dokumentmodell | 0.10.1, 42 Kennungen über drei Tiers |
| `a11y-conformance` | geteiltes Fixture-Korpus gegen Auseinanderlaufen | geplant |

## Fähigkeiten statt Optionen

Die drei Oberflächen unterscheiden sich nicht darin, wie sie dieselben Daten
darstellen, sondern darin, **welche Daten es überhaupt gibt**. Ein flaches Trait
mit `Option`-Rückgaben würde dazu führen, dass Regeln je nach Host
stillschweigend nicht laufen.

| | statisches HTML | Chrome via CDP | In-Page WASM |
|---|---|---|---|
| Struktur — Tags, Attribute, Text, Hierarchie | ✓ | ✓ | ✓ |
| Semantik — Rolle, Accessible Name | berechnet | nativ | berechnet |
| Rendering — Stile, Geometrie | — | ✓ | ✓ |
| Interaktion — Fokus, Ereignisse | — | ✓ | ✓ |

Jede Regel deklariert ihren Tier, jeder Host implementiert die Traits, die er
bedienen kann — und eine Regel, deren Tier nicht erfüllt ist, wird als
`UNTESTED` vermerkt statt zu schweigen.

Der Basisbaum ist bewusst **DOM-förmig**, nicht Accessibility-Tree-förmig: Die
Mehrzahl der Regeln braucht Attribute (`tabindex`, `id`, `role`, `alt`), und der
native Accessibility-Tree gibt die gar nicht her — `tabindex` taucht dort nicht
auf. Rolle und Name kommen als eigene Fähigkeit obendrauf.

## `accname` steht für sich

Die Accessible-Name-Berechnung ist als eigenes Crate gebaut, gegen
[accname 1.2](https://w3c.github.io/accname/) und
[HTML-AAM](https://www.w3.org/TR/html-aam-1.0/), nicht gegen eine vorhandene
Implementierung. Auf crates.io gab es bislang keine eigenständige
Rust-Umsetzung.

Eine Näherung aus „Teilbaumtext plus `aria-label`" reicht dafür nicht:
`aria-labelledby` löst Verweisketten auf und darf dabei sonst versteckte Knoten
heranziehen, ein eingebettetes Steuerelement steuert innerhalb einer Rekursion
seinen *Wert* bei statt seiner Beschriftung, und für Rollen wie `generic` oder
`paragraph` ist ein Name schlicht verboten.

## Zwei Achsen, nicht drei

`Outcome` sagt, *wie sicher* die Aussage ist — `Fail`, `Review`, `Pass`,
`Untested`. `Severity` sagt, *wie schwer* das Problem wiegt — `Low` bis
`Critical`.

Eine dritte Achse „certainty" gibt es bewusst nicht: Sie wäre weitgehend
dieselbe Achse doppelt, weil `Fail` ohnehin automatisch festgestellt heißt,
`Review` heuristisch und `Untested` nur manuell beurteilbar. Eine Regel, die
ihre Aussage nur vermuten kann, liefert deshalb `Review` — nicht `Fail` mit
niedriger Gewissheit.

Kein aggregierter Score. Ein einzelner Prozentwert würde genau die
Unterscheidung einebnen, für die es die vier Zustände gibt.

## „Lief nicht" ist nicht „bestanden"

`RuleRun` hält fest, ob eine Regel überhaupt laufen konnte. Ohne das liest sich
eine Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung — der
häufigste Weg, wie ein Bericht mehr verspricht, als er geprüft hat.

```rust
use a11y_report::{NotRun, Report, RuleRun};

let mut report = Report::new();
report.record(
    RuleRun::not_run("contrast/text", NotRun::CapabilityMissing)
        .with_reason("statische Analyse liefert keine Rendering-Werte"),
);
```

## Lizenz

MIT.
