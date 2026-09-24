---
title: "Konventionen"
description: "Befundmodell, Aussagegrenzen und API-Stil — was für jedes Paket gilt."
order: 2
---

Was für jedes Paket in diesem Repository gilt.

## Befunde

- Ein Befundmodell für alle: `a11y-report` (`Outcome`, `Evidence`, `NotRun`,
  `RuleRun`, `Severity`). Kein Paket definiert ein eigenes.
- **„Lief nicht" ist nicht „bestanden".** Fehlt eine Voraussetzung — kein
  AX-Baum, kein Fokus, abgebrochene Aufnahme, uneindeutige Knotenidentität —,
  dann `NotRun` mit Grund. Niemals still ein Erfolg.
- Jede Regel nennt ihre Quelle: nativ vom Browser gemeldet oder hier berechnet.
  Beides im Bericht unterscheidbar.
- Jede Regel zeigt auf einen Fall aus einem echten Korpus, sonst kommt sie nicht
  hinein. Das hält Sonderfallsammlungen klein.

## Aussagegrenzen

- Browserdaten belegen den **Anlass** zu einer Ansage, nicht die Ansage selbst.
  Kein Paket behauptet, was ein Screenreader gesagt hat.
- Näherungen werden als solche gekennzeichnet (Lesereihenfolge, Linearisierung),
  nicht als virtueller Puffer ausgegeben.
- Fähigkeits-Tiers aus `a11y-dom` gelten unverändert: ohne die nötigen Daten
  keine validierte Aussage.

## API-Stil

- Keine Browser- oder CDP-Typen in öffentlichen Signaturen.
- Fehlende Werte explizit (`Option`), nicht durch Standardwerte ersetzt.
- Serialisierung über `serde`, Feature-gesteuert, wo sie nicht der Kern ist.
- Öffentliche Fläche klein halten: erst veröffentlichen, wenn ein zweiter
  Konsument sie tatsächlich braucht.

## Sprache

- Code, Bezeichner, Commit-Titel und veröffentlichte READMEs: Englisch.
- Doku in `docs/` und Planung: Deutsch.
