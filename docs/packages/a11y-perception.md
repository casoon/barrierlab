---
title: "a11y-perception"
description: "Aufnahme, Lesereihenfolge und Differenz zweier Aufnahmen. Berechnet, erhebt nicht."
order: 5
---

Was von einer Seite **wahrnehmbar** ist: eine Aufnahme, ihre Lesereihenfolge und
die Differenz zweier Aufnahmen. Das Paket berechnet, es erhebt nicht.

## Stand

0.1.0, neu in barrierlab. Herausgezogen aus auditmysite
(`accessibility/{tree,diff,snapshot}.rs`, `screen_reader/{linearizer,types}.rs`)
nach dem Umbau, der an 133 Seiten mit 692 Journey-Instanzen gemessen wurde.
27 Tests sind mitgekommen und laufen ohne Browser.

Noch **nicht** hier: der Renderer, der aus der Struktur einen lesbaren Satz
macht (`announcer.rs`). Er hängt an der Lokalisierung des Hosts; die
Ansage-Struktur von ihm zu trennen, ist ein eigener Schritt.

## Aufbau

```mermaid
flowchart LR
  host["Host: CDP, Collector, Parser\n(nicht in diesem Paket)"] --> tree["AXTree · AXNode"]
  host --> focus["FocusSnapshot"]
  tree & focus --> snap["AXSnapshot\nLabel · URL · Titel · Zeit"]
  snap --> lin["linearize()"]
  lin --> ri["ReadingItem-Folge"]
  lin --> ign["IgnoredReadingNode\nmit Grund"]
  snap -- vorher --> diff["AXTreeDiff"]
  snap2["AXSnapshot nachher"] --> diff
  diff --> fm["Fokusbewegung"]
  diff --> add["wahrnehmbar geworden / verschwunden"]
  diff --> chg["geänderte Eigenschaften je Knoten"]
  diff --> unk["unklare Identität\n(fehlende Backend-ID, Frame, Navigation)"]
```

## Öffentliche Fläche

| Eintrag | Zweck |
|---|---|
| `AXTree`, `AXNode`, `AXValue`, `AXProperty` | der Baum und seine Knoten; `node_by_backend_id`, `is_within` für Zugehörigkeit |
| `AXSnapshot`, `FocusSnapshot`, `FocusIndicatorStatus`, `Rect` | ein aufgenommener Zeitpunkt samt Fokuslage |
| `AXTreeDiff` | die Differenz zweier Aufnahmen |
| `linearize`, `ReadingItem`, `IgnoredReadingNode` | die Projektion in Lesereihenfolge, mit dem, was sie übergeht |

## Abhängigkeiten

Nach unten: nur `serde`. Nach oben: auditmysite; künftig der Reader-Host.
Bewusst **nicht** `a11y-report`: dieses Paket erzeugt keine Befunde.

## Grenzen

Drei Dinge, die die Messung gelehrt hat und die hier als Regel gelten:

1. **„Hinzugekommen" heißt wahrnehmbar geworden.** Chrome entfernt einen
   verborgenen Knoten nicht — er bleibt mit derselben Backend-ID als `ignored`
   mit Grund `notRendered` stehen. Über Anwesenheit definiert, sah der Diff die
   häufigste Art nicht, Inhalt zu öffnen.
2. **Identität über die Backend-ID, nicht über die Position.** Positionsvergleich
   über zwei Arrays war eine der drei Ursachen, warum 44 % der High-Befunde
   einer JavaScript-Sonde falsch waren.
3. **Unklare Identität wird gemeldet, nicht geraten.** Fehlende Backend-ID,
   Knotenersetzung, Frame-Wechsel, Navigation.

Und die Grenze des Verfahrens selbst: Der Diff belegt keine Ansage. Flüchtige
Änderungen zwischen zwei Aufnahmen braucht ein Host als Ereignisspur — im
Ursprungsprojekt ist das `interaction::live_regions`, und das ist Erhebung, also
nicht hier.
