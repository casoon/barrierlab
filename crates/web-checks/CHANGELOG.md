# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [0.4.0] - 2026-09-28

### Added

- Familie `structured_data`, zusammengeführt aus auditmysite (`schema_rules.rs`
  und der JSON-LD-Normalisierung aus `schema.rs`) und den Zusatzregeln aus
  astro-post-audit:
  - `parse_blocks` / `parse_block`: je Script-Text die normalisierten Knoten
    (Wurzel-Listen und `@graph` aufgelöst, jeder `@type`-Eintrag, volle IRIs
    gekürzt) und Strukturprobleme als `StructuralIssue` — `EmptyScript`,
    `InvalidJson`, `InvalidRoot`, `EmptyDocument`, `GraphNotArray`,
    `MissingContext`, `NonSchemaOrgContext`, `GraphWithoutContext`,
    `MissingType`. Als schema.org-Kontext zählt nur `http(s)://schema.org`,
    auch in Listen und als `@vocab`; ein `@graph` vererbt ihn.
  - `assess_node`, `inventory_fields`, `SchemaRuleAssessment` samt Merkmal,
    Verfügbarkeit, Status und Quelle; `RULESET_VERSION` = `2026-09-28`.
    Vorhanden heißt nicht leer: `""`, `null`, `[]` und `{}` fehlen.
  - `duplicate_types`: `@type`-Werte, die in mehr als einem Block derselben
    Seite stehen, mit den Block-Indizes.
  - `ManualReview`: was nur von Hand zu prüfen ist, als Daten statt als Satz.
- Gegenüber auditmysite geändert: Breadcrumb-Einträge dürfen ihren Namen an
  `item.name` tragen; FAQPage verlangt `acceptedAnswer` je Frage
  (`mainEntity[i].acceptedAnswer`); NewsArticle empfiehlt zusätzlich
  `publisher`, WebSite zusätzlich `potentialAction`.
- `SchemaFeature::ALL` — alle Merkmale als Liste, für Inventare und Abdeckung beim Host.

## [0.3.0] - 2026-09-28

### Added

- Familie `meta`: `length` zählt Zeichen nach browserüblicher
  Leerraum-Zusammenfassung, `LengthRange::classify` ordnet zu kurz / ok / zu
  lang ein; `TITLE` (30–60) und `DESCRIPTION` (120–160) als gemeinsame
  Empfehlung. Beide Hosts maßen vorher in Bytes — Umlaute zählten doppelt.
- Familie `social`: Tag-Listen für OpenGraph und Twitter Cards
  (`*_REQUIRED`, `*_FIELDS`), `is_present` (leerer Inhalt gilt als fehlend),
  `is_complete`, `completeness` in Prozent, `is_valid_twitter_card`,
  `is_absolute_url`.

## [0.2.0] - 2026-09-24

### Added

- `RobotsTxt::wildcard_disallows_path` — ist ein Pfad für einen Bot ohne eigene
  Gruppe gesperrt? Mehrere `*`-Gruppen gelten dabei zusammen. Beide Hosts hatten
  sich genau das vorher selbst zusammengebaut; astro-post-audit kann seine
  `wildcard_rules` damit ablegen.

## [0.1.0] - 2026-09-24

### Added

- Erster Release mit der Familie `robots`: Grammatik der robots.txt,
  Bot-Einordnung als Daten und Pfadauswertung nach der Längsten-Regel. Holt
  nichts und formuliert nichts.
