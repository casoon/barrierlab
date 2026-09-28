# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [0.11.1] - 2026-09-28

### Fixed

- Inline-Elemente fügen im Namen aus dem Inhalt (Schritt 2F) und in
  `<label>`-, `<legend>`-, `<figcaption>`- und `<caption>`-Texten kein
  Leerzeichen mehr ein: `<abbr>EU</abbr>-Arktis` ergibt „EU-Arktis" statt
  „EU -Arktis", `Rechen<span>power</span>` „Rechenpower". Blockelemente,
  Inline-Block-Elemente (`img`, `input`, `button`, …), `br` und unbekannte
  Elemente bleiben durch ein Leerzeichen getrennt — wie in WPT
  `accname/name/comp_name_from_content.html`. Mangels berechneter Stile
  entscheidet das Tag. Gefunden im accname-Differentialkorpus von auditmysite.
- Text innerhalb von `<legend>`, `<figcaption>`, `<caption>` und `<option>`
  wird an Blockgrenzen jetzt getrennt statt zusammengeklebt.

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
