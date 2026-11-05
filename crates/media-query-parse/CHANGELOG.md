# Changelog

Alle nennenswerten Änderungen an `media-query-parse` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/). Die Fassung 0.1.0 steht nur in der
Git-Historie.

## [Unreleased]

### Added

- **Konformanz:** Die WPT-Dateien `test_media_queries.html` und `match-media-parsing.html` laufen als
  Korpus (`tests/wpt.rs`, vendort über `xtask/vendor-wpt-mediaqueries.sh`); die Liste
  bekannter Fehlschläge `tests/wpt_known_failures.txt` ist leer (casoon/barrierlab#111).

### Changed

- **Tokenizer:** Der CSS-Syntax-Level-3-Tokenizer liegt jetzt im Paket `css-tokenize` (L0).
  `media_query_parse::tokenizer` bleibt als ausgeblendeter Re-Export erhalten, damit bestehender
  Code weiter baut; neuer Code nimmt `css-tokenize` direkt.
- **Verhältnis (`<ratio>`):** `MfValue::Ratio` speichert `f64` statt `u32` (Änderung der
  öffentlichen API). `<ratio>` folgt Media Queries Level 4: Dezimalzahlen sind erlaubt
  (`1.5/1`), negative Werte und Nicht-Zahlen werden abgelehnt, ein Nenner von 0 ist
  syntaktisch gültig. Eine einzelne Zahl bleibt `MfValue::Number`.

### Fixed

- **Parser:** Ein am Eingabeende offener Block wird nach CSS Syntax 3 §5.4 implizit geschlossen;
  `(color` ist damit die gültige Abfrage `(color)` statt `UnbalancedParens`
  (casoon/barrierlab#111).
- **Changelog:** Diese Datei (casoon/barrierlab#116).
