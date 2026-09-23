---
title: API-Überblick
description: Die wichtigsten Typen und Funktionen je Crate. Die Einzeldokumentation jedes Elements steht auf docs.rs.
order: 2
---

## a11y-rules

| Element | Zweck |
|---|---|
| `run`, `run_with_semantics`, `run_with_rendering`, `run_full` | ein Dokument prüfen, je nachdem, was der Host liefert |
| `structure_rules`, `semantics_rules`, `rendering_rules` | die Regeln eines Tiers, gebunden an einen Host |
| `structure_metas`, `semantics_metas`, `rendering_metas` | die Deklarationen ohne Host, zum Auflisten und Benennen |
| `Meta` | Kennungen, Tier, WCAG-Kriterien, Vorgabeschwere und Hinweis einer Regel |
| `StructureRule`, `SemanticsRule`, `RenderingRule` | eine Regel als Funktionszeiger plus `Meta` |

[docs.rs/a11y-rules](https://docs.rs/a11y-rules)

## a11y-dom

| Element | Zweck |
|---|---|
| `Document`, `Node` | der DOM-förmige Basisbaum |
| `Semantics`, `Rendering`, `Interaction` | die Fähigkeiten über der Struktur |
| `Tier`, `Caps` | welche Datenschicht eine Regel braucht, was ein Host liefert |
| `ComputedStyle`, `Color`, `Rect`, `NameSource` | Daten der Fähigkeiten |
| `Arena`, `ArenaBuilder`, `ArenaNode` | mitgelieferter Baum, Testvehikel und Form für WASM-Hosts |
| `elements`, `descendants`, `ancestors`, `closest`, `subtree_text`, `has_text` | Traversierung |

[docs.rs/a11y-dom](https://docs.rs/a11y-dom)

## a11y-report

| Element | Zweck |
|---|---|
| `Finding`, `Location`, `Evidence` | ein Prüfergebnis mit Verortung und Belegen |
| `Outcome`, `Severity`, `WcagLevel` | die zwei Achsen und die WCAG-Stufe |
| `Report`, `Summary` | Befunde, Vermerke und Zählwerk |
| `RuleRun`, `NotRun` | ob eine Regel lief, und wenn nicht, warum |
| `Extra` | undurchsichtige, nicht serialisierte Beigabe am Befund |

[docs.rs/a11y-report](https://docs.rs/a11y-report)

## accname

| Element | Zweck |
|---|---|
| `name`, `description` | Accessible Name und Beschreibung |
| `role`, `implicit` | wirksame und implizite Rolle |
| `allows_name_from_content`, `name_is_prohibited` | Eigenschaften einer Rolle |
| `IdIndex` | ID-Index für Verweise |

[docs.rs/accname](https://docs.rs/accname)
