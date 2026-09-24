# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [0.1.1] - 2026-09-24

### Fixed

- Doku-Verweis auf ein privates Item entschärft (`is_layout_only_role`), damit
  `rustdoc -D warnings` durchläuft. Die Korrektur stammt aus auditmysite und ist
  hierher mitgewandert, weil der Code das auch getan hat.

## [0.1.0] - 2026-09-24

### Added

- Erster Release. Herausgezogen aus `casoon/auditmysite`: `AXTree`, `AXSnapshot`,
  `AXTreeDiff` und die Projektion in Lesereihenfolge. Berechnet, erhebt nicht.
