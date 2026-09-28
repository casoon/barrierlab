# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [0.3.0] - 2026-09-28

### Added

- Familie `meta`: `length` zählt Zeichen nach browserüblicher
  Leerraum-Zusammenfassung, `LengthRange::classify` ordnet zu kurz / ok / zu
  lang ein; `TITLE` (30–60) und `DESCRIPTION` (120–160) als gemeinsame
  Empfehlung. Beide Hosts maßen vorher in Bytes — Umlaute zählten doppelt.
- Familie `social`: Tag-Listen für OpenGraph und Twitter Cards
  (`*_REQUIRED`, `*_FIELDS`), `is_present` (leerer Inhalt gilt als fehlend),
  `is_complete`, `completeness` in Prozent, `is_valid_twitter_card`,
  `is_absolute_url`.

## [0.2.0] - 2026-09-24

### Added

- `RobotsTxt::wildcard_disallows_path` — ist ein Pfad für einen Bot ohne eigene
  Gruppe gesperrt? Mehrere `*`-Gruppen gelten dabei zusammen. Beide Hosts hatten
  sich genau das vorher selbst zusammengebaut; astro-post-audit kann seine
  `wildcard_rules` damit ablegen.

## [0.1.0] - 2026-09-24

### Added

- Erster Release mit der Familie `robots`: Grammatik der robots.txt,
  Bot-Einordnung als Daten und Pfadauswertung nach der Längsten-Regel. Holt
  nichts und formuliert nichts.
