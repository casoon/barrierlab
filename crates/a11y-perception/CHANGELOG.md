# Changelog

Alle nennenswerten Änderungen an `a11y-perception` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.2.1] - 2026-09-26

### Fixed

- **AXTree:** `AXTree::iter()` und damit `images`, `headings`, `form_controls`, `links`,
  `nodes_with_role` lassen alles unterhalb eines `Video`- oder `Audio`-Knotens aus.
- **AXTree:** Grund: mit `controls` legt Chrome dort die Player-Bedienung aus seinem Shadow DOM
  ab (Wiedergabe-, Stumm-, Vollbild-Button, Zeitleiste als `slider` ohne `valuenow`). ARIA-Regeln
  eines Hosts meldeten diese Zeitleiste auf jeder Seite mit `<video controls>` als Critical
  „fehlendes aria-valuenow", ohne dass ein Autor sie geschrieben hätte oder beheben könnte; in
  auditmysite kappte das den Score auf 49.
- **AXTree:** Der Medien-Knoten selbst bleibt; `iter_all()` und `text_nodes()` sind unverändert.

## [0.2.0] - 2026-09-25

### Added

- **announce:** `announce` und `Announcement`: woraus die Ansage zu einer Leseeinheit besteht,
  Name, Rolle und Zustände in Ansagereihenfolge als benannte Teile (`AnnouncedRole`,
  `AnnouncedState`) statt als Zeichenkette. Die Wörter bringt ein Host mit seiner Lokalisierung mit.
- **announce:** Die Struktur ist überall dieselbe: die Ebene einer Überschrift gehört in die
  Rolle, `expanded=false` sagt „eingeklappt", `required=false` nichts, „fokussierbar" kommt nur
  hinzu, wo die Rolle es nicht schon sagt.
- **announce:** Herkunft: auditmysites `screen_reader/announcer.rs`. Dort waren Struktur und
  Sprache in einer Funktion verschränkt, weshalb das Paket den Renderer bisher nicht übernehmen
  konnte (siehe [a11y-perception](https://casoon.github.io/barrierlab/docs/packages/a11y-perception/), 0.1.0).

### Fixed

- **AXTreeDiff:** meldet jetzt Wechsel an `checked` und `pressed` (Tristate „true"/„false"/„mixed");
  bisher blieb der Diff bei Checkbox, Switch und Toggle-Button leer, obwohl die Bedienung gewirkt
  hatte. Gesehen im Relief-CDP-Spike gegen Chrome: Checkbox angeklickt, `checked` auf `true`, Diff leer.

## [0.1.1] - 2026-09-24

### Fixed

- **Doku:** Verweis auf das private Item `is_layout_only_role` entschärft, damit
  `rustdoc -D warnings` durchläuft. Die Korrektur stammt aus auditmysite und ist mit dem Code mitgewandert.

## [0.1.0] - 2026-09-24

### Added

- **Paket:** Erster Release, herausgezogen aus `casoon/auditmysite`: `AXTree`, `AXSnapshot`,
  `AXTreeDiff` und die Projektion in Lesereihenfolge. Berechnet, erhebt nicht.
