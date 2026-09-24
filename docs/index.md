---
title: "Überblick"
description: "Welches Paket wofür da ist und wie diese Dokumentation aufgebaut ist."
order: 0
---

Ein Bereich für alle Pakete dieses Repositorys.

| Seite | Inhalt |
|---|---|
| [architecture.md](architecture.md) | Schichten, Abhängigkeitsgraph, Regeln für neue Pakete |
| [conventions.md](conventions.md) | was für jedes Paket gilt: Befundmodell, Aussagegrenzen, API-Stil |
| [releasing.md](releasing.md) | Tags, release-plz, crates.io, npm, WASM |
| [consumers.md](consumers.md) | die externen Werkzeuge und was sie von hier benutzen |
| [project-state.md](project-state.md) | Ist-Zustand des Repositorys |
| `packages/<name>.md` | eine Seite je Paket, Vorlage in [packages/README.md](packages/README.md) |
| [a11y/](a11y/) | die ausführliche Doku der a11y-Pakete: Einstieg, Konzepte (Befunde, Tiers, accname), Referenz |
| [html-conform/](html-conform/) | die ausführliche Doku der Konformanzprüfung: Einstieg, Guides, Referenz |

Diagramme stehen als Mermaid-Block im Text. GitHub rendert sie direkt; die
Pages-Seite erzeugt daraus zur Bauzeit ein Inline-SVG. Die Quelle ist immer der
Block, nie das Bild.
