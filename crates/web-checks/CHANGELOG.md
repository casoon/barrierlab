# Changelog

Alle nennenswerten Änderungen an `web-checks` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.5.0] - 2026-10-03

### Added

- **hreflang:** Familie `hreflang` (casoon/barrierlab#32), zusammengeführt aus auditmysite
  (`seo/technical.rs`: x-default und Verweis auf sich selbst; `page_health`: `hreflang_invalid`)
  und astro-post-audit (`checks/hreflang.rs`: x-default und Verweis auf sich selbst).
  Details: [hreflang](https://casoon.github.io/barrierlab/docs/packages/web-checks/#hreflang-050).
- **hreflang:** `Alternate`, `is_x_default`, `has_x_default`.
- **hreflang:** `is_valid_code`, `invalid_codes`: Sprache (ISO 639), optional Schrift (ISO 15924)
  und Region (ISO 3166-1 Alpha-2), Schreibung beliebig.
- **hreflang:** `has_self_reference`, `same_page`.
- **hreflang:** Gegenüber beiden Hosts geändert: `x-default` zählt nicht als Verweis auf sich
  selbst (Beleg: heise.de nennt seine deutsche Startseite nur so) und gilt ohne Rücksicht auf
  Schreibung. Gegenüber auditmysite sind `de-de` und Schrift-Subtags wie `zh-Hant` gültig,
  gegenüber astro-post-audit unterscheidet die Abfrage Seiten.
- **hreflang:** Beim Host bleiben Rückverweise zwischen Seiten und ob ein Ziel im Build existiert
  (beides nur astro-post-audit, braucht den ganzen Build).

## [0.4.0] - 2026-09-28

### Added

- **structured_data:** Familie `structured_data`, zusammengeführt aus auditmysite
  (`schema_rules.rs` und der JSON-LD-Normalisierung aus `schema.rs`) und den Zusatzregeln aus
  astro-post-audit. Details: [structured_data](https://casoon.github.io/barrierlab/docs/packages/web-checks/#structured_data-040).
- **structured_data:** `parse_blocks` / `parse_block`: je Script-Text die normalisierten Knoten
  (Wurzel-Listen und `@graph` aufgelöst, jeder `@type`-Eintrag, volle IRIs gekürzt).
- **structured_data:** Strukturprobleme als `StructuralIssue`: `EmptyScript`, `InvalidJson`,
  `InvalidRoot`, `EmptyDocument`, `GraphNotArray`, `MissingContext`, `NonSchemaOrgContext`,
  `GraphWithoutContext`, `MissingType`.
- **structured_data:** Als schema.org-Kontext zählt nur `http(s)://schema.org`, auch in Listen und
  als `@vocab`; ein `@graph` vererbt ihn.
- **structured_data:** `assess_node`, `inventory_fields`, `SchemaRuleAssessment` samt Merkmal,
  Verfügbarkeit, Status und Quelle; `RULESET_VERSION` = `2026-09-28`. Vorhanden heißt nicht
  leer: `""`, `null`, `[]` und `{}` fehlen.
- **structured_data:** `duplicate_types`: `@type`-Werte, die in mehr als einem Block derselben
  Seite stehen, mit den Block-Indizes.
- **structured_data:** `ManualReview`: was nur von Hand zu prüfen ist, als Daten statt als Satz.
- **structured_data:** Gegenüber auditmysite geändert: Breadcrumb-Einträge dürfen ihren Namen an
  `item.name` tragen; FAQPage verlangt `acceptedAnswer` je Frage (`mainEntity[i].acceptedAnswer`);
  NewsArticle empfiehlt zusätzlich `publisher`, WebSite zusätzlich `potentialAction`.
- **structured_data:** `SchemaFeature::ALL`: alle Merkmale als Liste, für Inventare und Abdeckung beim Host.

## [0.3.0] - 2026-09-28

### Added

- **meta:** Familie `meta`: `length` zählt Zeichen nach browserüblicher Leerraum-Zusammenfassung,
  `LengthRange::classify` ordnet zu kurz / ok / zu lang ein; `TITLE` (30–60) und `DESCRIPTION`
  (120–160) als gemeinsame Empfehlung. Beide Hosts maßen vorher in Bytes, Umlaute zählten doppelt.
  Details: [meta und social](https://casoon.github.io/barrierlab/docs/packages/web-checks/#meta-und-social-030).
- **social:** Familie `social`: Tag-Listen für OpenGraph und Twitter Cards (`*_REQUIRED`,
  `*_FIELDS`), `is_present` (leerer Inhalt gilt als fehlend), `is_complete`, `completeness` in
  Prozent, `is_valid_twitter_card`, `is_absolute_url`.

## [0.2.0] - 2026-09-24

### Added

- **robots:** `RobotsTxt::wildcard_disallows_path`: ist ein Pfad für einen Bot ohne eigene Gruppe
  gesperrt? Mehrere `*`-Gruppen gelten dabei zusammen. Beide Hosts hatten sich genau das selbst
  gebaut; astro-post-audit kann seine `wildcard_rules` damit ablegen.

## [0.1.0] - 2026-09-24

### Added

- **robots:** Erster Release mit der Familie `robots`: Grammatik der robots.txt, Bot-Einordnung als
  Daten und Pfadauswertung nach der Längsten-Regel. Holt nichts und formuliert nichts.
