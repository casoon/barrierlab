# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

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
