# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [Unreleased]

### Added

- `FindingId` und `RootCauseId`: stabile Kennungen für einen Befund (Regel und Menge der normalisierten Selektoren) und eine Ursache (Regel und Selektor-Muster). Seitenübergreifend gleich, unabhängig von Reihenfolge und Knoten-IDs, über Versionen festgehalten (FNV-1a).
- `normalize_selector` und `PAGEWIDE_SELECTOR`: die Form, in der zwei Seiten desselben Templates übereinstimmen.
- `CauseConfidence`, `ImpactArea`, `EvidenceBasis`, `CriterionState`: Belegstärke einer Ursache und einer Auswirkung, Zustand eines Bewertungskriteriums.

## [0.24.1] - 2026-10-25

### Changed

- Gleichschritt mit `a11y-rules` 0.24.1; keine eigene Änderung.

## [0.24.0] - 2026-10-24

### Changed

- Gleichschritt mit `a11y-rules` 0.24.0; keine eigene Änderung.

## [0.23.0] - 2026-10-23

### Changed

- Gleichschritt mit `a11y-rules` 0.23.0; keine eigene Änderung.

## [0.22.0] - 2026-10-05

### Changed

- Gleichschritt mit `a11y-rules` 0.22.0; keine eigene Änderung.

## [0.21.0] - 2026-10-05

### Changed

- Gleichschritt mit `a11y-rules` 0.21.0; keine eigene Änderung.

## [0.20.0] - 2026-10-05

### Changed

- Gleichschritt mit `a11y-rules` 0.20.0; keine eigene Änderung.

## [0.19.1] - 2026-10-04

### Changed

- Gleichschritt mit `a11y-rules` 0.19.1; keine eigene Änderung.

## [0.19.0] - 2026-10-03

### Changed

- Gleichschritt mit `a11y-rules` 0.19.0; keine eigene Änderung.

## [0.18.0] - 2026-10-03

### Changed

- Gleichschritt mit `a11y-rules` 0.18.0; keine eigene Änderung.

## [0.17.0] - 2026-10-01

### Changed

- Gleichschritt mit `a11y-rules` 0.17.0; keine eigene Änderung.

## [0.16.0] - 2026-10-01

### Changed

- Gleichschritt mit `a11y-rules` 0.16.0; keine eigene Änderung.

## [0.15.0] - 2026-10-01

### Changed

- Gleichschritt mit `a11y-rules` 0.15.0; keine eigene Änderung.

## [0.14.2] - 2026-10-01

### Changed

- Gleichschritt mit `a11y-rules` 0.14.2; keine eigene Änderung.

## [0.14.1] - 2026-10-01

### Changed

- Gleichschritt mit `a11y-rules` 0.14.1; keine eigene Änderung.

## [0.14.0] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.14.0; keine eigene Änderung.

## [0.13.5] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.5; keine eigene Änderung.

## [0.13.4] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.4; keine eigene Änderung.

## [0.13.3] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.3; keine eigene Änderung.

## [0.13.2] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.2; keine eigene Änderung.

## [0.13.1] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.1; keine eigene Änderung.

## [0.13.0] - 2026-09-30

### Changed

- Gleichschritt mit `a11y-rules` 0.13.0; keine eigene Änderung.

## [0.12.2] - 2026-09-29

### Changed

- Gleichschritt mit `a11y-rules` 0.12.2; keine eigene Änderung.

## [0.12.1] - 2026-09-28

### Changed

- Gleichschritt mit `a11y-rules` 0.12.1; keine eigene Änderung.

## [0.12.0] - 2026-09-28

### Changed

- Gleichschritt mit `a11y-rules` 0.12.0; keine eigene Änderung.

## [0.11.3] - 2026-09-28

### Changed

- Gleichschritt mit `accname` 0.11.3; keine eigene Änderung.

## [0.11.2] - 2026-09-28

### Changed

- Gleichschritt mit `accname` 0.11.2; keine eigene Änderung.

## [0.11.1] - 2026-09-28

### Changed

- Edition 2024 statt 2021, Edition und MSRV werden jetzt vom Workspace geerbt.
  Rein intern; die öffentliche Fläche ist unverändert.

## [0.11.0] - 2026-09-23

### Changed

- Befundtexte und Regel-Hinweise auf Englisch. Entstanden im Repository
  `casoon/a11y-core` und hier nachgezogen; ab jetzt ist dieses Monorepo die
  einzige Quelle.

## [0.10.2] - 2026-09-23

### Changed

- Das Crate liegt jetzt im Monorepo `casoon/barrierlab`; `repository` und
  `homepage` zeigen dorthin. Keine Änderung an Code oder öffentlicher Fläche.
