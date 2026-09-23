---
title: Befunde und Berichte
description: Das Befundmodell aus a11y-report – zwei Achsen, vier Zustände, ein Vermerk je Regelkennung und ein schlanker JSON-Vertrag.
order: 2
---

## Zwei Achsen, nicht drei

`Outcome` sagt, *wie sicher* die Aussage ist, `Severity`, *wie schwer* das Problem wiegt.

| `Outcome` | Bedeutung |
|---|---|
| `fail` | automatisch festgestelltes Problem, eindeutig belegt |
| `review` | potenzielles Problem, nur heuristisch vermutet, braucht menschliche Bestätigung |
| `pass` | automatische Prüfung bestanden, wird geführt, aber standardmäßig nicht angezeigt |
| `untested` | automatisiert nicht beurteilbar, grundsätzlich oder weil dem Host die Daten fehlen |

`Severity` reicht von `low` über `medium` und `high` bis `critical`. Für Werkzeuge, die die
axe-Vokabel erwarten, liefert `Severity::as_axe_impact` `minor`, `moderate`, `serious` und
`critical`.

Eine dritte Achse „certainty" gibt es bewusst nicht. Sie wäre weitgehend dieselbe Achse doppelt,
weil `fail` ohnehin automatisch festgestellt heißt, `review` heuristisch und `untested` nur
manuell beurteilbar. Eine Regel, die ihre Aussage nur vermuten kann, liefert deshalb `review`,
nicht `fail` mit niedriger Gewissheit.

## Kein Score

`Summary` zählt Zustände und Schweregrade, mehr nicht. Schweregrade zählen nur über Probleme,
also `fail` und `review`; `Summary::problems` ist deren Summe. `untested` zählt nicht mit, weil es
keine Aussage trifft. Ein aggregierter Prozentwert würde genau die Unterscheidung einebnen, für
die es die vier Zustände gibt.

## Ein Vermerk je Kennung

`Report` hat drei Felder: `findings`, `rule_runs` und `summary`. `RuleRun` beantwortet „lief das
überhaupt?", `Finding` beantwortet „was kam dabei heraus?". Die Vermerke werden je
**Befund**-Kennung geführt, nicht je übergeordneter Regel. Damit benutzen `rule_runs` und
`findings` dieselbe Namensmenge und lassen sich über `rule_id` verbinden.

Wer dieselbe Seite unter mehreren Bedingungen prüft, führt je Durchgang einen Vermerk mit
`viewport`; der Schlüssel ist dann `(rule_id, viewport)`. Ein nicht gelaufener Vermerk kann mit
`wcag` nennen, welches Kriterium ungeprüft blieb.

## Der JSON-Vertrag

Ein Befund trägt vier Pflichtfelder: `rule_id`, `outcome`, `severity` und `message`. Alles andere
fällt weg, wenn es nicht gesetzt ist:

```json
{
  "rule_id": "images/alt-missing",
  "outcome": "fail",
  "severity": "high",
  "message": "Das Bild hat kein alt-Attribut.",
  "location": {
    "node": "15"
  },
  "wcag": [
    "1.1.1"
  ]
}
```

Der Befund stammt aus `examples/teaser.semantik.json`. Die optionalen Felder:

| Feld | Inhalt |
|---|---|
| `rule_name` | menschenlesbarer Name der Regel |
| `location` | `file`, `url`, `selector`, `node`, `source_hint` |
| `role`, `name` | berechnete Rolle und Accessible Name des Elements |
| `wcag`, `wcag_level` | Erfolgskriterien und Stufe `A`, `AA` oder `AAA` |
| `tags` | freie Schlagworte |
| `help`, `help_url`, `suggestion`, `suggested_code` | was zu tun ist |
| `snippet` | das betroffene Markup, soweit der Host es liefern kann |
| `evidence` | Belege mit `source`, `field`, `value` |

## Platz für das, was nicht in den Vertrag gehört

`Finding::extra` ist ein undurchsichtiger Slot für werkzeugspezifische Beigaben, etwa den
Element-Screenshot, den auditmysite für seinen PDF-Bericht zuschneidet. Er reist mit dem Befund,
wird **nie serialisiert** und zählt **nicht zur Identität**: Zwei inhaltlich gleiche Befunde
bleiben gleich, auch wenn an einem ein Screenshot hängt.

```rust
use a11y_report::Finding;

struct Screenshot(Vec<u8>);

let f = Finding::fail("images/alt-missing", "kein alt")
    .with_extra(Screenshot(vec![0x89, 0x50]));

assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(2));
```
