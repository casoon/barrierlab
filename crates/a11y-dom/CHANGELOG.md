# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [0.21.0] - 2026-10-05

### Changed

- **`Layout`: jedes Feld ist jetzt `Option`.** `None` heißt „nicht gemessen",
  nicht „nein". Ein Host kann liefern, was er erhebt — etwa nur `obscured` —,
  ohne dass die Heuristiken auf den übrigen Feldern mit Vorgabewerten laufen
  (casoon/barrierlab#47). Wer `Layout { .. }` baut, setzt `Some(..)`.

### Added

- `Backdrop` und `Rendering::sampled_backdrop`: eine Stichprobe der
  relativen Leuchtdichten hinter einem Text, für Verläufe, Bilder und
  Überlagerungen. Vorgabe `None`.
- `Rendering::visually_hidden`: ob der Text eines Knotens optisch verborgen
  ist, obwohl `display` und `visibility` ihn darstellen (`clip`,
  `clip-path`, `text-indent`, Kasten von höchstens 1 px mit
  `overflow: hidden`). Vorgabe `None`.

## [0.20.0] - 2026-10-05

### Added

- `ComputedStyle`: `list_style_type`, `text_decoration_line`, `font_style`,
  `font_family`, `border_bottom_style` (casoon/barrierlab#21). `ComputedStyle`
  implementiert jetzt `Default`; neue Felder lassen sich so mit
  `..Default::default()` auslassen.
- `Rendering::scroll_overflow_px`: Überhang eines Kastens mit `overflow: auto`
  oder `scroll` in CSS-Pixeln, Vorgabe `None` (nicht gemessen). Eine eigene
  Methode statt eines `Layout`-Felds: Ein Host, der nur den Überhang misst,
  liefert kein halbes `Layout`, auf dem die Heuristiken mit Vorgabewerten
  liefen (gefunden bei auditmysite#698).

### Changed

- Wer `ComputedStyle { .. }` von Hand baut, ergänzt die neuen Felder oder
  `..Default::default()`.

## [0.19.1] - 2026-10-04

### Changed

- Gleichschritt mit `a11y-rules` 0.19.1; keine eigene Änderung.

## [0.19.0] - 2026-10-03

### Added

- `Tier::Stylesheets` und `Caps::stylesheets` / `Caps::with_stylesheets`:
  die Stylesheets der Seite als eigene Schicht, unabhängig von Semantik und
  Darstellung (casoon/barrierlab#21). `Caps` bekommt damit ein Feld mehr;
  wer `Caps { .. }` von Hand baut, ergänzt es.

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

### Added

- `Layout` und `Rendering::layout()` (Vorgabe `None`): Layout-Angaben für
  heuristische Regeln — umgekehrte Flex-Richtung, `order`, `min-width`,
  `cursor: pointer`, Endlos-Animation, Verdeckung durch fixierte Elemente,
  Leisten tiefer als `scroll-padding-top`, gemessene Fokus-Sichtbarkeit.
  Nicht brechend: Ein Host ohne diese Daten bekommt die Heuristiken nicht
  (liveaudit#6).

## [0.12.2] - 2026-09-29

### Documentation

- Vertrag für Shadow DOM festgehalten: Der Host liefert den flachen Baum —
  Shadow-Root-Kinder unter dem Host, zugewiesene Knoten unter ihrem `<slot>`,
  nicht zugewiesene Light-DOM-Kinder gar nicht.

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
