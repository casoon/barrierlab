---
title: Überblick
description: Was a11y-core ist, welche Crates dazugehören und wie diese Dokumentation aufgebaut ist.
order: 0
---

a11y-core sind die gemeinsamen Bausteine der CASOON-Accessibility-Werkzeuge in Rust. Ein
Regelbestand bedient drei Oberflächen mit identischen Regelkennungen und identischem JSON:

| Oberfläche | Werkzeug | Substrat |
|---|---|---|
| Build-Zeit | [astro-post-audit](https://github.com/casoon/astro-post-audit) | statisches HTML aus `dist/` |
| CI / Crawl | [auditmysite](https://github.com/casoon/auditmysite) | Chrome via CDP, nativer Accessibility-Tree |
| laufende Seite | [liveaudit](https://liveaudit.casoon.de) | DOM der Seite, über WASM |

## Crates

| Crate | Zweck |
|---|---|
| [`a11y-report`](https://docs.rs/a11y-report) | Befund- und Berichtsmodell, JSON-Vertrag |
| [`a11y-dom`](https://docs.rs/a11y-dom) | Dokumentmodell-Abstraktion plus Fähigkeits-Tiers |
| [`accname`](https://docs.rs/accname) | WAI-ARIA Accessible Name und Role Computation |
| [`a11y-rules`](https://docs.rs/a11y-rules) | die Regeln, generisch über das Dokumentmodell |

Alle vier stehen auf crates.io in Version 0.10.0 und werden im Gleichschritt versioniert. Geplant,
aber noch nicht vorhanden, ist `a11y-conformance`: ein geteiltes Fixture-Korpus, das die drei
Oberflächen gegen Auseinanderlaufen absichert.

## Wo es aufhört

a11y-core liefert keinen HTML-Parser, keinen Crawler und keine Browsersteuerung. Den Baum baut
der Host: astro-post-audit aus den Build-Dateien, auditmysite aus Chrome, liveaudit aus dem DOM
der Seite. Die mitgelieferte `Arena` aus `a11y-dom` ist das Testvehikel und die Form, die ein
WASM-Host braucht.

## Aufbau dieser Dokumentation

- **Einstieg:** die Crates einbinden und ein erstes Dokument prüfen.
- **Konzepte:** Fähigkeits-Tiers, das Befundmodell und die Namensberechnung.
- **Referenz:** alle Regelkennungen und der Überblick über die öffentliche API. Die
  Einzeldokumentation jedes Typs steht auf [docs.rs](https://docs.rs/a11y-rules).
