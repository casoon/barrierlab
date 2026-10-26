# Changelog

Alle nennenswerten Änderungen an `stylesheet-parse` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-03

### Added

- **Parser:** Erste Fassung (casoon/barrierlab#21): `parse_stylesheet` nach CSS Syntax Level 3 §5
  (Editor's Draft, mit Verschachtelung), tokenisiert mit `media-query-parse`.
  Details: [stylesheet-parse](https://casoon.github.io/barrierlab/docs/packages/stylesheet-parse/).
- **Parser:** Fehlerbehandlung wie spezifiziert; zusätzlich fallen Deklarationen und Selektorlisten
  weg, auf die keine Grammatik passen kann (fehlerhafte Token, leere Listeneinträge).
- **Parser:** Grenzen gegen bösartige Eingaben: Blocktiefe 128, aufgelöster Selektor 64 KiB.
- **Parser:** Geprüft an 337 Stylesheets von 48 Seiten sowie Bootstrap 5.3.3 und Tailwind 2.2.19:
  kein Panic, rund 0,1 s je MB im Release-Build.
- **Regeln:** `Style`, `Media`, `Keyframes` (auch mit Herstellerpräfix), `Group` (`@supports`,
  `@layer {}`, `@container`, `@scope`, `@document`, `@-moz-document`, `@starting-style`), `Other`
  für alle übrigen At-Regeln.
- **Serialisierung:** Präludien, Selektoren und Werte serialisiert nach CSS Syntax §9 und
  CSSOM §2.1; `!important` als eigenes Feld.
- **Stylesheet:** `Stylesheet::style_rules` und `Stylesheet::keyframes` ebnen durch `@media`,
  Gruppenregeln und CSS Nesting ein und liefern den Kontext von außen nach innen. `&` am
  Selektoranfang wird durch den Elternselektor ersetzt, sonst durch `:is(…)`.
