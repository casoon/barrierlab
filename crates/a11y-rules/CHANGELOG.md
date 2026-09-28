# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [0.12.1] - 2026-09-28

### Added

- Feature `de` (Vorgabe: an) für die deutschen Befundtexte und Hinweise.
  Mit `default-features = false` kommen die deutschen Vorlagen gar nicht erst
  in den Binärcode; `Locale::De` und `Meta::help_de` gibt es dann nicht —
  wer Deutsch verlangt, bekommt einen Compile-Fehler statt stillem Englisch.
  Rein additiv: Mit den Vorgaben ändert sich nichts.

## [0.12.0] - 2026-09-28

### Added

- **Befundtexte auf Deutsch.** `Locale` (`En`, `De`; Vorgabe `En`) und die
  Einstiege `run_in`, `run_with_semantics_in`, `run_full_in` und
  `run_with_rendering_in`. Übersetzt ist, was ein Werkzeug einem Menschen zeigt:
  `message`, der Grund eines nicht gelaufenen Vermerks und der Hinweis der
  Deklaration (`Meta::help_de`, `Meta::help_in`). Kennungen, Outcomes,
  Schweregrade und WCAG-Bezüge hängen nicht von der Sprache ab. Die bisherigen
  Einstiege liefern unverändert Englisch.

### Changed

- **`ids/duplicate` folgt WCAG 2.2.** 4.1.1 (Parsing) ist gestrichen; gemeldet
  wird nur noch eine doppelte ID, auf die ein IDREF zeigt (`for`, `form`,
  `list`, `headers` und die ARIA-Verweise) — dann ist die Beziehung nicht
  eindeutig auflösbar. WCAG-Bezug jetzt 4.1.2, wie axe-core `duplicate-id-aria`.
  Eine Dopplung, auf die niemand zeigt, erzeugt keinen Befund mehr. Die Regel
  lag bisher als Filter in auditmysite; hier gilt sie für alle Hosts gleich.
- **Bruch:** Regelfunktionen nehmen die Sprache mit
  (`fn(&D, Locale, &mut Vec<Finding>)`), und `Meta` hat ein weiteres Feld. Wer
  nur die `run*`-Einstiege benutzt, merkt davon nichts.

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
