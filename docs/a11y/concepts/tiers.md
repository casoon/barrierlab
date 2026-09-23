---
title: Fähigkeits-Tiers
description: Warum das Dokumentmodell aus mehreren Traits besteht und was mit einer Regel passiert, deren Tier der Host nicht bedient.
order: 1
---

Die drei Oberflächen unterscheiden sich nicht darin, wie sie dieselben Daten darstellen, sondern
darin, **welche Daten es überhaupt gibt**. Ein flaches Trait mit `Option`-Rückgaben würde dazu
führen, dass Regeln je nach Host stillschweigend nicht laufen.

| | statisches HTML | Chrome via CDP | In-Page WASM |
|---|---|---|---|
| `Document`: Struktur, Tags, Attribute, Text, Hierarchie | ✓ | ✓ | ✓ |
| `Semantics`: Rolle, Accessible Name | berechnet | nativ | berechnet |
| `Rendering`: Stile, Geometrie | – | ✓ | ✓ |
| `Interaction`: Fokus, Ereignisse | – | ✓ | ✓ |

Jede Regel deklariert ihren `Tier`, jeder Host implementiert die Traits, die er bedienen kann.

## Die Grenze liegt im Typsystem

Regeln sind Funktionszeiger, nach Tier getrennt registriert: `StructureRule<D: Document>`,
`SemanticsRule<D: Semantics>`, `RenderingRule<D: Rendering>`. Eine Tier-2-Regel kann gar nicht
erst mit einem Host aufgerufen werden, der `Semantics` nicht erfüllt. Keine Trait-Objekte: Das
monomorphisiert pro Host und allokiert nichts pro Regel.

## Nicht gelaufen ist nicht bestanden

Die Einstiegsfunktionen laufen mit dem, was der Host liefert, und vermerken den Rest. `run` auf
einem reinen Struktur-Host führt jede Tier-2- und Tier-3-Kennung als `RuleRun` mit
`NotRun::CapabilityMissing` und einem Grund:

```json
{
  "rule_id": "links/name-missing",
  "not_run": "capability_missing",
  "findings": 0,
  "reason": "Host liefert keine Rolle und keinen Accessible Name"
}
```

Der Auszug stammt aus `examples/teaser.struktur.json`. Ohne diesen Vermerk läse sich eine
Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung.

## Der Baum ist DOM-förmig

`Node` bildet Tags, Attribute, Text und Hierarchie ab, nicht Rollen und Accessible Names. Die
Mehrzahl der Regeln braucht Attribute wie `tabindex`, `id`, `role`, `alt` oder `for`, und der
native Accessibility-Tree des Browsers gibt die nicht her: `tabindex` taucht dort nicht auf. Rolle
und Name kommen als eigene Fähigkeit obendrauf.

## Auflage an Tier 3

`ComputedStyle::background_color` ist die **effektive** Hintergrundfarbe, über die Vorfahren
aufgelöst. Kann der Host sie nicht bestimmen, etwa bei Hintergrundbild, Verlauf oder
`background-blend-mode`, liefert er `None`, und die Kontrastregel meldet
`contrast/text-undetermined` mit `untested`. Eine Prüfung gegen geratenes Weiß würde ein `pass`
erzeugen, auf das sich jemand verlässt.

`Interaction` ist als Trait definiert, im Regelbestand von 0.10.0 gibt es aber noch keine Regel
auf diesem Tier.
