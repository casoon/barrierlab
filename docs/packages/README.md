# Vorlage für eine Paketseite

Jedes Paket bekommt `docs/packages/<name>.md` mit diesem Aufbau. Kurz halten;
die API-Referenz steht auf docs.rs, nicht hier.

```markdown
# <name>

Ein Satz: was das Paket berechnet und was nicht.

## Stand

Version, Registry, wer es benutzt. Ehrlich über halb Gebautes.

## Aufbau

Ein Mermaid-Diagramm: Datenfluss oder Module. Keine Wiederholung des Graphen
aus architecture.md, sondern das Innere dieses Pakets.

## Öffentliche Fläche

Die wenigen Typen und Funktionen, die ein Konsument braucht, mit einem Satz je
Eintrag. Link auf docs.rs für den Rest.

## Abhängigkeiten

Nach unten: welche Pakete dieses hier braucht. Nach oben: wer es benutzt.

## Grenzen

Was das Paket nicht entscheidet, nicht beweist, nicht sieht. Dieser Abschnitt
ist Pflicht — er verhindert Behauptungen, die die Daten nicht tragen.
```
