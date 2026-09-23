> Historie von `casoon/a11y-core` bis 0.10.1 (Crates im Gleichschritt). Ab 0.10.2
> führt release-plz je Crate eine eigene `CHANGELOG.md` in `crates/<name>/`.


# Changelog

Alle nennenswerten Änderungen an den vier Crates. Sie werden im Gleichschritt versioniert. Das
Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/); die Einträge sind aus der
Git-Historie nachgetragen.

## [Unreleased]

### Fixed

- `headings/empty` meldet eine Überschrift nicht mehr als leer, wenn sie ihren Namen über
  `aria-labelledby` oder ein Bild mit Alternativtext bekommt. Im Repository als 0.10.1 geführt,
  noch nicht auf crates.io.

## [0.10.0] - 2026-09-20

### Changed

- `images/alt-suspicious` erkennt Alt-Texte aus ein oder zwei Zeichen. Der Befund bleibt `review`.

## [0.9.0] - 2026-09-20

### Added

- `links/generic-name`: Linktext, der für sich genommen nichts über das Ziel sagt, als `review`.

### Fixed

- `images/alt-missing` meldet keine Bilder mehr, die per `role="presentation"` oder
  `aria-hidden="true"` aus dem Accessibility-Tree genommen sind.

## [0.8.0] - 2026-09-20

### Added

- `landmarks/main-missing`, `landmarks/main-duplicate`, `landmarks/navigation-missing`,
  `landmarks/banner-missing`, `landmarks/contentinfo-missing`.
- `keyboard/skip-link-missing`, `aria/required-attribute-missing`, `headings/h1-multiple`,
  `zoom/viewport-missing`.

## [0.7.0] - 2026-09-20

### Added

- Tier 3: `RenderingRule`, `contrast/text-insufficient` (WCAG 1.4.3) und
  `contrast/text-undetermined`.
- `run_full` für Hosts mit Semantik und Darstellung, `run_with_rendering` für Hosts ohne Semantik.

### Changed

- `run` und `run_with_semantics` vermerken die Kontrastkennungen mit `CapabilityMissing`.

## [0.6.0] - 2026-09-19

### Added

- `lists/item-outside-list`: ein Listeneintrag ohne umgebende Liste.

## [0.5.0] - 2026-09-19

### Added

- `lists/term-without-definition`, `tables/presentational-with-headers`, `tables/name-missing`
  (als `review`) und `zoom/viewport-scale-limited`.

## [0.4.0] - 2026-09-19

### Added

- `lists/empty`: eine Liste ohne Einträge.

### Changed

- `keyboard/positive-tabindex` hat die Vorgabeschwere `high`.

### Fixed

- `zoom/viewport-locked` vergleicht den `content`-Wert ohne Rücksicht auf Groß- und
  Kleinschreibung.
- `lists/invalid-structure` berücksichtigt `role="list"` und `role="listitem"`; `role="presentation"`
  an der Liste unterdrückt die Listenbefunde.
- `tables/header-missing` erkennt Kopfzellen über `role="columnheader"` und `role="rowheader"` und
  prüft auch `role="table"`.

## [0.3.0] - 2026-09-19

### Added

- `Finding` trägt `role`, `name` und `rule_name`.
- `Extra`: werkzeugspezifische Beigabe am Befund, nie serialisiert und nicht Teil der Identität.
- `RuleRun` trägt `viewport` und `wcag`.

### Breaking

- Neue Felder an `Finding` und `RuleRun`.

## [0.2.0] - 2026-09-18

### Breaking

- `Meta::id` wird zu `Meta::ids` und deklariert alle Befund-Kennungen einer Regel. `RuleRun` wird
  je Kennung geführt, sodass `rule_runs` und `findings` dieselbe Namensmenge benutzen.

## [0.1.0] - 2026-09-18

### Added

- `a11y-report`: Befund- und Berichtsmodell.
- `a11y-dom` und `a11y-rules`: Dokumentmodell mit Fähigkeits-Tiers und die ersten Regeln.
- `accname`: Accessible Name und Role Computation nach accname 1.2 und HTML-AAM 1.0.
