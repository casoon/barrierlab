---
title: "Accessible Name"
description: "Was das Crate accname berechnet, wogegen es geschrieben ist und wo es ohne Rendering-Daten aufhört."
order: 3
---

`accname` setzt [Accessible Name and Description Computation 1.2](https://w3c.github.io/accname/)
und die Namensregeln aus [HTML-AAM 1.0](https://www.w3.org/TR/html-aam-1.0/) um, generisch über
das Dokumentmodell aus `a11y-dom`. Es ist gegen die Spezifikation geschrieben, nicht gegen eine
vorhandene Implementierung; die Schrittnummern im Quelltext entsprechen denen des Normtexts.

```rust
use a11y_dom::{elements, Arena, Document, Node};
use accname::{name, IdIndex};

let doc = Arena::builder()
    .open("form")
        .open("span").attr("id", "l").text("Vorname").close()
        .open("input").attr("id", "v").attr("aria-labelledby", "l").close()
    .close()
    .build();

let ids = IdIndex::build(doc.root());
let feld = elements(&doc).find(|n| n.local_name() == "input").unwrap();
assert_eq!(name(feld, &ids).as_deref(), Some("Vorname"));
```

## Warum das nicht nebenbei geht

Eine Näherung aus „Teilbaumtext plus `aria-label`" liegt in genau den Fällen falsch, in denen es
darauf ankommt:

- `aria-labelledby` löst Verweisketten auf und darf dabei sonst versteckte Knoten heranziehen.
- Ein eingebettetes Steuerelement steuert innerhalb einer Rekursion seinen *Wert* bei, nicht seine
  Beschriftung.
- Für Rollen wie `generic` oder `paragraph` ist ein Name verboten.

## Was abgedeckt ist

Die Schritte 2A bis 2I: `aria-labelledby` mit Verweisketten und Zyklenschutz, `aria-label`, die
HTML-eigenen Textalternativen (`alt`, `label` über `for` und umschließend, `legend`, `caption`,
`figcaption`, `title` in SVG, die Sonderfälle von `input`), eingebettete Steuerelemente, Name aus
dem Inhalt, `title` als letzte Rückfallebene.

Dazu die Rollenberechnung nach HTML-AAM mit den kontextabhängigen Fällen: `header` und `footer`
sind nur außerhalb sektionierender Elemente Landmarks, `section` ist nur benannt eine `region`,
ein `img` mit leerem `alt` ist `presentation`.

## Die öffentliche API

| Funktion | Zweck |
|---|---|
| `name`, `description` | Accessible Name und Beschreibung eines Knotens |
| `role` | die wirksame Rolle, explizit oder implizit |
| `implicit` | die implizite Rolle nach HTML-AAM |
| `allows_name_from_content`, `name_is_prohibited` | Eigenschaften einer Rolle |
| `IdIndex` | ID-Index für Verweise, einmal je Dokument gebaut |

Die Einzeldokumentation steht auf [docs.rs/accname](https://docs.rs/accname).

## Grenze ohne Rendering

Schritt 2A schließt versteckte Knoten aus. Ohne Rendering-Daten sind nur `aria-hidden="true"` und
das `hidden`-Attribut erkennbar, `display: none` und `visibility: hidden` nicht. Ein Host mit
`Rendering` kann hier genauer sein; das Crate bleibt bewusst bei dem, was ohne Layout entscheidbar
ist.
