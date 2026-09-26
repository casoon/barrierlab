# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.2.1] - 2026-09-26

### Fixed

- `AXTree::iter()` — und damit `images`, `headings`, `form_controls`, `links`,
  `nodes_with_role` — lässt alles unterhalb eines `Video`- oder `Audio`-Knotens
  aus. Mit `controls` legt Chrome dort die Bedienoberfläche des Players aus dem
  Shadow DOM des Browsers ab: Wiedergabe-, Stumm- und Vollbild-Button und eine
  Zeitleiste als `slider` ohne `valuenow`. Die ARIA-Regeln eines Hosts meldeten
  diese Zeitleiste als Critical „fehlendes aria-valuenow" — auf jeder Seite mit
  `<video controls>`, ohne dass ein Autor sie geschrieben hätte oder beheben
  könnte. In auditmysite kappte das den Score einer solchen Seite auf 49. Der
  Medien-Knoten selbst bleibt; `iter_all()` und `text_nodes()` sind unverändert.

## [0.2.0] - 2026-09-25

### Added

- `announce` und `Announcement`: woraus die Ansage zu einer Leseeinheit besteht
  — Name, Rolle, Zustände, in Ansagereihenfolge und als benannte Teile
  (`AnnouncedRole`, `AnnouncedState`), nicht als Zeichenkette. Die Struktur ist
  überall dieselbe: dass die Ebene einer Überschrift in die Rolle gehört, dass
  `expanded=false` „eingeklappt" zu sagen hat und `required=false` nichts, dass
  „fokussierbar" nur dort etwas hinzufügt, wo die Rolle es nicht schon sagt. Die
  Wörter bringt ein Host mit seiner Lokalisierung mit.

  Herkunft: auditmysites `screen_reader/announcer.rs`. Dort waren Struktur und
  Sprache in einer Funktion verschränkt, weshalb das Paket den Renderer bisher
  nicht übernehmen konnte (siehe `docs/packages/a11y-perception.md`, 0.1.0).

### Fixed

- `AXTreeDiff` meldet jetzt Wechsel an `checked` und `pressed` (Tristate-Token
  „true"/„false"/„mixed"). Bisher blieb der Diff bei Checkbox, Switch und
  Toggle-Button leer, obwohl die Bedienung gewirkt hatte. Gesehen im
  Relief-CDP-Spike gegen Chrome: Checkbox angeklickt, `checked` wechselte auf
  `true`, der Diff zeigte nichts.

## [0.1.1] - 2026-09-24

### Fixed

- Doku-Verweis auf ein privates Item entschärft (`is_layout_only_role`), damit
  `rustdoc -D warnings` durchläuft. Die Korrektur stammt aus auditmysite und ist
  hierher mitgewandert, weil der Code das auch getan hat.

## [0.1.0] - 2026-09-24

### Added

- Erster Release. Herausgezogen aus `casoon/auditmysite`: `AXTree`, `AXSnapshot`,
  `AXTreeDiff` und die Projektion in Lesereihenfolge. Berechnet, erhebt nicht.
