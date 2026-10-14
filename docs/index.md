---
title: "Überblick"
description: "Welches Paket wofür da ist und wie diese Dokumentation aufgebaut ist."
order: 0
---

Ein Bereich für alle Pakete dieses Repositorys.

| Bereich | Inhalt |
|---|---|
| [Pakete](packages/index.mdx) | alle 15 Crates mit ihren Abhängigkeiten, eine Seite je Paket |
| [Zugänglichkeit im Detail](a11y/index.md) | die a11y-Pakete: Installation, Schnellstart, Konzepte (Befunde, Tiers, accname), Referenz |
| [Konformanz im Detail](html-conform/index.md) | html-conform: Einstieg, Guides, Referenz |

Für die Arbeit am Repository selbst liegen im Wurzelverzeichnis von `docs/` die
lebenden Projektdokumente, die nicht auf der Doku-Seite erscheinen:
[Architektur](https://github.com/casoon/barrierlab/blob/main/docs/architecture.md),
[Konventionen](https://github.com/casoon/barrierlab/blob/main/docs/conventions.md),
[Konsumenten](https://github.com/casoon/barrierlab/blob/main/docs/consumers.md),
[Releases](https://github.com/casoon/barrierlab/blob/main/docs/releasing.md) und
[Stand](https://github.com/casoon/barrierlab/blob/main/docs/project-state.md).

Diagramme stehen als Mermaid-Block im Text. GitHub rendert sie direkt; die
Pages-Seite erzeugt daraus zur Bauzeit ein Inline-SVG. Die Quelle ist immer der
Block, nie das Bild. Ausnahme sind die Diagramme der Startseite und der
Paketübersicht: chartlet-Spezifikationen in `site/diagrams/`.
