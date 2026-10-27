---
title: "html-conform"
description: "HTML5-Konformanzprüfung ohne JVM, samt der sechs Parser darunter."
order: 7
---

HTML5-Konformanzprüfung als Rust-Abhängigkeit, ohne JVM. Differentiell gegen
den Nu Html Checker (vnu) geprüft. Dazu gehören sechs eigenständige Pakete, die
die Syntax lesen, die die Prüfung braucht.

## Stand

0.2.1 auf crates.io, benutzt von auditmysite (`src/html_conform/`) und
astro-post-audit (`checks/html_validation.rs`). Auf `main` liegen Änderungen
nach 0.2.1: MSRV auf 1.88, zusätzliche Befunde (`img` ohne `alt`, interaktiver
Inhalt in `a`).

**Mit dem Umzug sind die `=`-Pins gefallen.** Vorher hing html-conform auf genaue
Versionen von vier seiner sechs Abhängigkeiten, weil jede Änderung eine
Release-Kaskade über sieben Repositories brauchte. Jetzt stehen sie als
Workspace-Abhängigkeiten mit `path` und `version` — eine Änderung, ein Release.

## Aufbau

```mermaid
flowchart TB
  html["HTML-Quelltext"] --> h5[html5-parser]
  h5 --> info["infoset: normalisierter Baum"]
  info --> cm["Inhaltsmodell, Attribute,\nTabellenintegrität"]
  info --> rng["relax-ng\n(Schema-Validierung)"]
  info --> sch["schematron-engine\n(Regeln über XPath)"]
  sch --> xp[xpath-eval]
  attr["Attributwerte"] --> csp[csp-parse]
  attr --> mq[media-query-parse]
  cm & rng & sch & csp & mq --> f["Befunde (Severity::Error)"]
```

| Paket | Rolle |
|---|---|
| `html5-parser` | WHATWG-Tokenizer und Tree-Construction, meldet Parse-Fehler mit Position |
| `csp-parse` | Content-Security-Policy nach CSP3-Grammatik |
| `media-query-parse` | Media Queries Level 4 |
| `xpath-eval` | XPath-Auswertung |
| `relax-ng` | RELAX NG, für die Schema-Seite der Prüfung |
| `schematron-engine` | Schematron-Regeln über `xpath-eval` |

## Öffentliche Fläche

`html_conform::check(html)` liefert die Befunde. Die sechs Pakete darunter haben
jeweils eine eigene, kleine API und lassen sich einzeln benutzen — `xpath-eval`
und `schematron-engine` zum Beispiel auch außerhalb von HTML.

## Abhängigkeiten

Nach unten: die sechs oben. Nach oben: auditmysite, astro-post-audit, künftig
`web-checks`.

## Grenzen

Kein vollständiger vnu-Ersatz. Die Prüfung ist differentiell gegen vnu
abgeglichen, nicht deckungsgleich; welche Klassen fehlen, steht in
`docs/html-conform/`. Und sie sagt nichts über Zugänglichkeit — Konformanz ist
eine andere Frage als Wahrnehmbarkeit.

Geprüft wird bis zu einer Verschachtelungstiefe von 512 Elementen. Tiefere
Dokumente liefern `CheckError::DocumentTooDeep` statt eines Berichts — die
Prüfschichten nach dem Parser laufen rekursiv über den Baum und würden sonst den
Stack eines 2-MiB-Threads sprengen. Der Parser selbst kennt keine Grenze, weil
die WHATWG-Tree-Construction keine kennt; Blink hört bei 512 auf zu
verschachteln und hängt tiefere Elemente als Geschwister an. Diese Abflachung
übernimmt html-conform bewusst nicht: Befunde gälten dann für einen Baum, den
die Spezifikation nicht baut. Die Grenze nutzt den Spielraum, den der
HTML-Standard für implementierungsspezifische Grenzen bei sonst unbeschränkter
Eingabe lässt, indem sie die Prüfung ablehnt, nicht etwas anderes prüft.
