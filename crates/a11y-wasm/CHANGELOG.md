# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.14.0] - 2026-10-24

### Changed

- **Größe:** 154,4 KB gzip.
- **Regeln:** Gebaut auf `a11y-rules` 0.24.0: `links/generic-name` mit der Liste aus
  auditmysite (15 Sprachen) und Kontextauswertung, neu `links/url-name`,
  `links/single-character-name`, `links/new-window-unannounced` und
  `links/purpose-link-only`. Keine Änderung an der Schnittstelle.

## [0.13.0] - 2026-10-23

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.23.0: sieben neue `manual/*`-Punkte und
  `contrast/control-*` (1.4.11). Keine Änderung an der Schnittstelle.
- **Rendering:** Baumbereiche (Shadow DOM) liefert dieses Paket noch nicht; alle Knoten liegen
  im Dokumentbereich, die ID-Auflösung bleibt damit wie bisher. Die neuen Stilfelder für
  1.4.11 sind `None`.

## [0.12.0] - 2026-10-05

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.22.0: `targets/size-enhanced` (2.5.5) und die
  überarbeiteten Heuristiken. Keine Änderung an der Schnittstelle.
- **Rendering:** `pointer_events_none` und `animating` liefert dieses Paket noch nicht (`None`).

## [0.11.0] - 2026-10-05

### Changed

- **Größe:** Größenbudget auf 170 KB gzip angehoben (gemessen 150,0 KB).
- **Regeln:** Gebaut auf `a11y-rules` 0.21.0: `contrast/text-enhanced` (1.4.6), Kontrast ohne
  optisch verborgenen und mit überdecktem Text, Messwerte als Evidence.
- **Rendering:** `Layout` ist feldweise optional; dieses Paket füllt jedes Feld. Abtastung und
  `visually_hidden` liefert es noch nicht.

## [0.10.0] - 2026-10-05

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.20.0: die Regeln `lists/role-redundant`,
  `color/link-indistinct` und `keyboard/scrollable-region-not-focusable`. Keine Änderung an der Schnittstelle.
- **Rendering:** Die Stilfelder und den Scroll-Überhang, die diese Regeln brauchen, liefert
  dieses Paket noch nicht; sie stehen als `UNTESTED` im Bericht, sobald es etwas zu prüfen
  gäbe.

## [0.9.1] - 2026-10-03

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.19.1. Keine Änderung an der Schnittstelle.

## [0.9.0] - 2026-10-03

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.19.0. Keine Änderung an der Schnittstelle.
- **Rendering:** Die neuen Regeln über Stylesheets laufen hier noch nicht — das Paket gibt
  keine Stylesheets weiter; sie stehen als nicht gelaufen im Bericht.

## [0.8.0] - 2026-10-03

### Changed

- **Größe:** Das Größenbudget liegt seitdem bei 150 KB gzip (gemessen 142,8 KB).
- **Regeln:** Gebaut auf `a11y-rules` 0.18.0: die Regeln aus B5–B7 (barrierlab#18–#20) —
  Links und Zeiger, Bilder und Medien, Tabellen, Dokument, Sprache und Rollen — und die
  statischen `viz/*`- und `display/*`-Regeln der Darstellungskonvention (barrierlab#22).
  Keine Änderung an der Schnittstelle.

## [0.7.0] - 2026-10-01

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.17.0: die Landmark- und Strukturregeln aus B4
  (barrierlab#17) — eindeutige, oberste und doppelte Landmarks, Inhalt außerhalb von
  Landmarks, Seite ohne Überschriften, Tastaturerreichbarkeit, Dialog ohne Fokusziel,
  Akkordeon. Keine Änderung an der Schnittstelle.

## [0.6.0] - 2026-10-01

### Changed

- **Größe:** Das Größenbudget liegt seitdem bei 130 KB gzip.
- **Regeln:** Gebaut auf `a11y-rules` 0.16.0: die Formularregeln aus B3 (barrierlab#16) —
  autocomplete, Zweck, Fehlerkennzeichnung, Gruppen, Pflichtfelder, Anweisungen, Absenden,
  wiederholte Eingabe, Kontextwechsel, CAPTCHA. Keine Änderung an der Schnittstelle.

## [0.5.0] - 2026-10-01

### Changed

- **Größe:** 111,6 KB gzip bei 110 KB Budget (112.640 Byte).
- **Regeln:** Gebaut auf `a11y-rules` 0.15.0: die Namensregeln aus B2 (barrierlab#15) —
  Pflichtnamen, Dialoge, `<summary>`, Live-Regionen, Label in Name. Keine Änderung an der Schnittstelle.

## [0.4.0] - 2026-10-01

### Changed

- **Größe:** Das Größenbudget liegt seitdem bei 110 KB gzip.
- **Regeln:** Gebaut auf `a11y-rules` 0.14.0: die ARIA-Attributregeln aus B1 (barrierlab#14)
  laufen jetzt auch in der Seite — erlaubte/verbotene Attribute, Werte, Pflicht-Eltern und
  -Kinder, Tabs, Combobox, popover/inert. Keine Änderung an der Schnittstelle.

## [0.3.1] - 2026-09-30

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.13.5. Keine Änderung an der Schnittstelle; übernimmt
  die Fehlalarm-Korrekturen aus 0.13.1–0.13.5 (Viewport-Trenner, Pflichtattribute nach
  ARIA 1.2, `aria-activedescendant`, Landmark-Grenze über Rollen, Sprunglink am Ziel und zum
  Hauptinhalt, `img` mit `aria-label`/`title`, SVG im benannten Link, geschlossenes
  `<details>`, eingeklapptes `aria-controls`, main-missing hinter offenem Dialog).

## [0.3.0] - 2026-09-30

### Added

- **Scan:** `Scan.withLayout(flags, order, minWidthPx, bounds)`: Layout und Geometrie für die
  heuristischen Regeln; `RenderArena` bedient damit `bounds` und `layout`. `withRendering`
  bleibt unverändert. Größe 168,5 → 197,6 KB roh, 80,9 → 92,4 KB gzip.

### Changed

- **Regeln:** Gebaut auf `a11y-rules` 0.13.0. Übernimmt den Geltungsbereich je Regel: Befunde an
  versteckten Elementen entfallen (liveaudit#1, #3, #4), dazu Checkliste und Heuristiken
  (liveaudit#6, #7).
- **RenderArena:** `RenderArena` berechnet den Namen mit `accname::name_rendered`: Per
  `display: none` versteckte Labels und Teilbäume tragen nichts bei.

## [0.2.1] - 2026-09-28

### Changed

- **Größe:** Baut `a11y-rules` ohne das Feature `de`: Das Modul trägt keine deutschen Texte
  mehr, die es in der laufenden Seite nie ausgibt. 173,6 → 168,4 KB roh, 83,3 → 80,9 KB gzip.
  Befunde und Texte unverändert.

## [0.2.0] - 2026-09-28

### Changed

- **Regeln:** Gegen `a11y-*` 0.12.0 gebaut: `ids/duplicate` meldet nur noch doppelte IDs, auf
  die ein IDREF zeigt (WCAG 2.2, Kriterium 4.1.2). Befundtexte bleiben englisch — die
  Sprache wählt der Host, und die laufende Seite spricht Englisch.

## [0.1.1] - 2026-09-23

### Changed

- **Regeln:** Gegen `a11y-*` 0.11.0 gebaut (englische Befundtexte).
- **Contributors:** `bool::then` mit Closure durch `then_some` ersetzt — clippy-Befund, der in
  liveaudit nie auffiel, weil dort nur der Pages-Workflow lief.

## [0.1.0] - 2026-09-23

### Added

- **Paket:** Erster Release. Herausgezogen aus `casoon/liveaudit` (`packages/core`, dort
  `liveaudit-core` mit `publish = false`): Arena-Adapter auf `a11y-dom` und die
  `wasm-bindgen`-Grenze über `a11y-rules`. Keine eigenen Regeln.
