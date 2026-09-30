# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Changed

- `RenderArena` berechnet den Namen mit `accname::name_rendered`: Per
  `display: none` versteckte Labels und Teilbäume tragen nichts bei.
- Übernimmt aus `a11y-rules` den Geltungsbereich je Regel: Befunde an
  versteckten Elementen entfallen (liveaudit#1, #3, #4), dazu Checkliste und
  Heuristiken (liveaudit#6, #7).

### Added

- `Scan.withLayout(flags, order, minWidthPx, bounds)`: Layout und Geometrie
  für die heuristischen Regeln; `RenderArena` bedient damit `bounds` und
  `layout`. `withRendering` bleibt unverändert. Größe 168,5 → 197,6 KB roh,
  80,9 → 92,4 KB gzip.

## [0.2.1] - 2026-09-28

### Changed

- Baut `a11y-rules` ohne das Feature `de`: Das Modul trägt keine deutschen
  Texte mehr, die es in der laufenden Seite nie ausgibt. 173,6 → 168,4 KB roh,
  83,3 → 80,9 KB gzip. Befunde und Texte unverändert.

## [0.2.0] - 2026-09-28

### Changed

- Gegen `a11y-*` 0.12.0 gebaut: `ids/duplicate` meldet nur noch doppelte IDs,
  auf die ein IDREF zeigt (WCAG 2.2, Kriterium 4.1.2). Befundtexte bleiben
  englisch — die Sprache wählt der Host, und die laufende Seite spricht
  Englisch.

## [0.1.1] - 2026-09-23

### Changed

- Gegen `a11y-*` 0.11.0 gebaut (englische Befundtexte).
- `bool::then` mit Closure durch `then_some` ersetzt — clippy-Befund, der in
  liveaudit nie auffiel, weil dort nur der Pages-Workflow lief.

## [0.1.0] - 2026-09-23

### Added

- Erster Release. Herausgezogen aus `casoon/liveaudit` (`packages/core`, dort
  `liveaudit-core` mit `publish = false`): Arena-Adapter auf `a11y-dom` und die
  `wasm-bindgen`-Grenze über `a11y-rules`. Keine eigenen Regeln.
