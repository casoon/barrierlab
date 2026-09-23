# a11y-report

Gemeinsames Befund- und Berichtsmodell für Accessibility-Werkzeuge. Teil von
[a11y-core](https://github.com/casoon/a11y-core).

## Zwei Achsen, nicht drei

`Outcome` sagt, *wie sicher* die Aussage ist — `Fail`, `Review`, `Pass`,
`Untested`. `Severity` sagt, *wie schwer* das Problem wiegt.

Eine dritte Achse „certainty" gibt es bewusst nicht: Sie wäre weitgehend
dieselbe Achse doppelt, weil `Fail` ohnehin automatisch festgestellt heißt,
`Review` heuristisch und `Untested` nur manuell beurteilbar. Eine Regel, die
ihre Aussage nur vermuten kann, liefert deshalb `Review` — nicht `Fail` mit
niedriger Gewissheit.

Kein aggregierter Score. Ein einzelner Prozentwert würde genau die
Unterscheidung einebnen, für die es die vier Zustände gibt.

```rust
use a11y_report::{Finding, Location, Report, Severity};

let mut report = Report::new();
report.push(
    Finding::fail("images/alt-missing", "Das Bild besitzt kein alt-Attribut.")
        .with_severity(Severity::High)
        .with_wcag(["1.1.1"])
        .at(Location::file("about/index.html").with_selector("main > img")),
);
let report = report.finish();
assert_eq!(report.summary.fail, 1);
```

## Platz fuer das, was nicht in den Vertrag gehoert

Der gemeinsame Vertrag kann nicht jedes Feld jedes Werkzeugs aufnehmen —
auditmysite haengt etwa einen Element-Screenshot an einen Befund, den es fuer
seinen PDF-Bericht zuschneidet. Eine Seitentabelle ueber den Index traegt
nicht: Befunde werden gefiltert, sortiert und aus mehreren Laeufen
zusammengefuehrt, dabei verschieben sich Indizes.

`Extra` ist deshalb ein undurchsichtiger Slot am Befund selbst. Er reist mit,
wird **nie serialisiert** und zaehlt **nicht zur Identitaet** — zwei inhaltlich
gleiche Befunde bleiben gleich, auch wenn an einem ein Screenshot haengt.

```rust
use a11y_report::Finding;

struct Screenshot(Vec<u8>);

let f = Finding::fail("images/alt-missing", "kein alt")
    .with_extra(Screenshot(vec![0x89, 0x50]));

assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(2));
```

Rolle und Accessible Name des betroffenen Elements sind dagegen **keine**
Beigabe, sondern Teil des Vertrags: Wer einen Bericht liest, will wissen, was
das Element fuer die Assistenztechnik war — ein Selektor allein sagt das nicht.

## „Lief nicht" ist nicht „bestanden"

`RuleRun` hält fest, ob eine Regel überhaupt laufen konnte. Ohne das liest sich
eine Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung — der
häufigste Weg, wie ein Bericht mehr verspricht, als er geprüft hat.

## Lizenz

MIT.
