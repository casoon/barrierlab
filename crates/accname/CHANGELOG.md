# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [Unreleased]

### Fixed

- `name_rendered`: Ein `<label>`, das nicht dargestellt wird (`display: none`
  am Label oder einem Vorfahren), trägt nichts zum Namen bei — wie im Browser
  (Chrome per CDP: Name `""`). Per `aria-labelledby` zählt verstecktes Label
  weiterhin. Ohne Stil (`name`) unverändert (liveaudit#3).

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

### Added

- `name_rendered(doc, node, ids)` für Hosts mit `a11y_dom::Rendering`: Im
  Inhaltsdurchlauf (2F) und in `<label>`-, `<legend>`-, `<figcaption>`- und
  `<caption>`-Texten schließen Kinder mit `display: inline`/`contents` direkt
  an, alle anderen und ersetzte Elemente werden abgesetzt, `<br>` trennt immer.
  `display: none` (auch an einem Vorfahren) und `visibility: hidden`/`collapse`
  blenden aus (2A), außer bei Verweis. Ohne Stil für ein Element gilt das
  Verhalten von `name()`. Im Differentialkorpus von auditmysite (35 Seiten, mit
  Stilen aus CDP `DOMSnapshot`) bleibt ein echtes Abweichungsmuster statt
  sechs; „EU-Arktis", „Rechenpower", „abholen*" stimmen jetzt mit Chrome
  überein. `name()` ist unverändert.

## [0.11.2] - 2026-09-28

### Fixed

- Rücknahme der Leerzeichen-Regel aus 0.11.1: Zwischen den Teilen eines Namens
  steht wieder immer ein Leerzeichen. Die Entscheidung nach dem Tag traf echte
  Seiten falsch, auf denen `<span>` per CSS Block-Elemente sind
  (`<a><span>Cloud & Hosting</span><span>Edge-Hosting</span></a>` ergab
  „Cloud & HostingEdge-Hosting"). Im Differentialkorpus von auditmysite
  (35 Seiten) standen 5 behobenen Fällen 450 neue Abweichungen gegenüber.
  Ohne berechnetes `display` vom Host bleibt das Leerzeichen die bessere
  Näherung.

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
