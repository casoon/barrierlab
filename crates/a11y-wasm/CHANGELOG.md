# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

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
