# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in der
[a11y-core-Historie](https://casoon.github.io/barrierlab/docs/packages/a11y-core-history/) — die vier Crates lagen bis dahin im
Repository `casoon/a11y-core`.

## [Unreleased]

### Added

- **API:** `Location::end` (mit `Location::with_end`): Ende der Quelltextstelle, exklusiv und in
  den Einheiten von `position`; im JSON nur vorhanden, wo eine Prüfschicht ein Ende kennt.
- **API:** `Finding::id()` berechnet die stabile Kennung eines Befunds: mit Selektor aus Regel und
  Element-Muster, sonst aus Regel, Datei und Startposition (neu: `FindingId::at`).

## [0.29.0] - 2026-11-09

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.29.0; keine eigene Änderung.

## [0.28.3] - 2026-11-08

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.28.3; keine eigene Änderung.

## [0.28.2] - 2026-11-07

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.28.2; keine eigene Änderung.

## [0.28.1] - 2026-11-06

### Added

- **Ort:** `Location::position` mit `SourcePosition` (Zeile, Spalte ab 1, Byte-Versatz ab 0) und
  `Location::with_position`: die Stelle im geprüften Quelltext für Prüfungen auf Markup. Der Host
  kann mit dem Versatz im selben Text nachlesen. Das Feld entfällt im JSON, solange es leer ist.

## [0.28.0] - 2026-11-06

### Added

- **Bericht:** `Report::schema` und `SCHEMA_VERSION` (1): Version des JSON-Vertrags, wird immer
  serialisiert; JSON ohne das Feld bleibt lesbar.
- **Builder:** `Location::with_node`, `Location::with_source_hint`, `Evidence::with_field` und
  `Evidence::with_value`, damit Hosts die nun `#[non_exhaustive]`en Typen ohne Struct-Literal bauen.
- **Eingaben:** `Report::inputs` mit `InputNote` und `InputKind` (`stylesheet_unavailable`,
  `stylesheet_unresolved`, `document_incomplete`, `other`) sowie `Report::note_input`: der Host
  vermerkt unvollständige Eingaben. Regeln lesen die Vermerke nicht. Das Feld entfällt im JSON,
  solange es leer ist.

### Changed

- **Breaking, Semver:** `Report`, `Finding`, `RuleRun`, `Summary`, `Evidence`, `Location`,
  `InputNote` und `NotRun` sind `#[non_exhaustive]`. Struct-Literale und erschöpfende Matches
  außerhalb des Crates entfallen; neue Felder sind künftig keine Breaking Changes mehr. Bauen über
  `Finding::fail` und Verwandte, `RuleRun::ran`/`not_run`, `Location::file`/`node`, `Evidence::new`.
- **Doku:** `docs/packages/a11y-report.md` hält fest, welche Typen `#[non_exhaustive]` sind und welche bewusst nicht (Host-Eingabetypen, geschlossene Enums, `Evidence::source` als freier String); casoon/barrierlab#110.

## [0.27.0] - 2026-11-03

### Added

- **Befundmodell:** `Basis` (`Criterion`, `BestPractice`) und `Finding::basis` für Empfehlungen ohne Erfolgskriterium; `Finding::best_practice(..)` liefert `Review` mit `Basis::BestPractice` und dem Schlagwort `best-practice`, `Finding::is_best_practice()` fragt es ab. Im JSON erscheint `basis` (`best_practice`) nur bei Empfehlungen; älteres JSON bleibt lesbar (casoon/barrierlab#81).

## [0.26.1] - 2026-10-27

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.26.1; keine eigene Änderung.

## [0.26.0] - 2026-10-26

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.26.0; keine eigene Änderung.

## [0.25.0] - 2026-10-25

### Added

- **Bewertung:** `CauseConfidence`, `ImpactArea`, `EvidenceBasis`, `CriterionState`:
  Belegstärke einer Ursache und einer Auswirkung, Zustand eines Bewertungskriteriums.
- **Kennungen:** `FindingId` und `RootCauseId`: stabile Kennungen für einen Befund (Regel und
  Menge der normalisierten Selektoren) und eine Ursache (Regel und Selektor-Muster).
  Seitenübergreifend gleich, unabhängig von Reihenfolge und Knoten-IDs, über Versionen
  festgehalten (FNV-1a).
- **Kennungen:** `normalize_selector` und `PAGEWIDE_SELECTOR`: die Form, in der zwei Seiten
  desselben Templates übereinstimmen.

## [0.24.1] - 2026-10-24

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.24.1; keine eigene Änderung.

## [0.24.0] - 2026-10-24

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.24.0; keine eigene Änderung.

## [0.23.0] - 2026-10-23

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.23.0; keine eigene Änderung.

## [0.22.0] - 2026-10-05

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.22.0; keine eigene Änderung.

## [0.21.0] - 2026-10-05

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.21.0; keine eigene Änderung.

## [0.20.0] - 2026-10-05

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.20.0; keine eigene Änderung.

## [0.19.1] - 2026-10-03

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.19.1; keine eigene Änderung.

## [0.19.0] - 2026-10-03

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.19.0; keine eigene Änderung.

## [0.18.0] - 2026-10-03

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.18.0; keine eigene Änderung.

## [0.17.0] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.17.0; keine eigene Änderung.

## [0.16.0] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.16.0; keine eigene Änderung.

## [0.15.0] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.15.0; keine eigene Änderung.

## [0.14.2] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.14.2; keine eigene Änderung.

## [0.14.1] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.14.1; keine eigene Änderung.

## [0.14.0] - 2026-10-01

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.14.0; keine eigene Änderung.

## [0.13.5] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.5; keine eigene Änderung.

## [0.13.4] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.4; keine eigene Änderung.

## [0.13.3] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.3; keine eigene Änderung.

## [0.13.2] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.2; keine eigene Änderung.

## [0.13.1] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.1; keine eigene Änderung.

## [0.13.0] - 2026-09-30

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.13.0; keine eigene Änderung.

## [0.12.2] - 2026-09-29

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.12.2; keine eigene Änderung.

## [0.12.1] - 2026-09-28

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.12.1; keine eigene Änderung.

## [0.12.0] - 2026-09-28

### Changed

- **Gleichschritt:** mit `a11y-rules` 0.12.0; keine eigene Änderung.

## [0.11.3] - 2026-09-28

### Changed

- **Gleichschritt:** mit `accname` 0.11.3; keine eigene Änderung.

## [0.11.2] - 2026-09-28

### Changed

- **Gleichschritt:** mit `accname` 0.11.2; keine eigene Änderung.

## [0.11.1] - 2026-09-28

### Changed

- **Build:** Edition 2024 statt 2021, Edition und MSRV werden jetzt vom Workspace geerbt.
  Rein intern; die öffentliche Fläche ist unverändert.

## [0.11.0] - 2026-09-23

### Changed

- **Texte:** Befundtexte und Regel-Hinweise auf Englisch. Entstanden im Repository
  `casoon/a11y-core` und hier nachgezogen; ab jetzt ist dieses Monorepo die
  einzige Quelle.

## [0.10.2] - 2026-09-23

### Changed

- **Repository:** Das Crate liegt jetzt im Monorepo `casoon/barrierlab`; `repository` und
  `homepage` zeigen dorthin. Keine Änderung an Code oder öffentlicher Fläche.
