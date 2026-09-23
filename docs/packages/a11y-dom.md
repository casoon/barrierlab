# a11y-dom

Die Abstraktion über das Dokument: ein DOM-förmiger Baum als Trait, plus
Fähigkeits-Tiers. Damit läuft dieselbe Regel über einen geparsten HTML-Baum,
über einen Browser-AX-Baum und über die laufende Seite.

## Stand

0.10.1 auf crates.io. Benutzt von `accname`, `a11y-rules` und allen Hosts;
auditmysite baut daraus in `dom_document.rs` einen Baum aus CDP-Daten.

## Aufbau

```mermaid
flowchart TB
  subgraph tiers["Fähigkeits-Tiers"]
    d["Document\n(Struktur: Elemente, Attribute, Kinder)"]
    s["Semantics\n(berechnete Rolle, Name, Zustand)"]
    r["Rendering\n(Layout, Sichtbarkeit, Farben)"]
  end
  d --> s --> r
  arena["Arena / ArenaBuilder\n(eigene Baumhaltung für Tests und Parser)"] -.implementiert.-> d
  host["Host-Adapter\n(CDP, scraper, laufende Seite)"] -.implementiert.-> d
```

## Öffentliche Fläche

| Typ | Zweck |
|---|---|
| `Document` | das Minimum: Baum, Elemente, Attribute |
| `Semantics`, `Rendering` | Tiers, die ein Host nur dann anbietet, wenn er die Daten wirklich hat |
| `Arena`, `ArenaBuilder`, `ArenaNode` | fertige Implementierung für Tests und parserbasierte Hosts |

## Abhängigkeiten

Nach unten: nichts außer `serde` (optional). Nach oben: `accname`,
`a11y-rules`, Host-Adapter.

## Grenzen

Ein Tier ist eine Zusage über **Verfügbarkeit**, nicht über Qualität. Wer
`Rendering` anbietet, verspricht Layoutdaten — nicht, dass sie stimmen. Fehlt
ein Tier, liefern die darauf aufbauenden Regeln `NotRun`, nicht „bestanden".
