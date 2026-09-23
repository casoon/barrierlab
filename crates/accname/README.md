# accname

WAI-ARIA Accessible Name and Description Computation in Rust, generisch über das
Dokumentmodell aus [`a11y-dom`](https://crates.io/crates/a11y-dom). Teil von
[a11y-core](https://github.com/casoon/a11y-core).

Umsetzung von [accname 1.2](https://w3c.github.io/accname/) und den Namensregeln
aus [HTML-AAM 1.0](https://www.w3.org/TR/html-aam-1.0/) — gegen die
Spezifikation geschrieben, nicht gegen eine vorhandene Implementierung. Die
Schrittnummern im Quelltext entsprechen denen des Normtexts.

## Warum das nicht nebenbei geht

Eine Näherung aus „Teilbaumtext plus `aria-label`" liegt in genau den Fällen
falsch, in denen es darauf ankommt:

- `aria-labelledby` löst Verweisketten auf und darf dabei Knoten heranziehen,
  die sonst versteckt sind
- ein eingebettetes Steuerelement steuert innerhalb einer Rekursion seinen
  *Wert* bei, nicht seine Beschriftung
- für Rollen wie `generic` oder `paragraph` ist ein Name schlicht verboten

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

Neben `name` gibt es `description` und `role` — Letzteres als Rollenberechnung
nach HTML-AAM, inklusive der kontextabhängigen Fälle: `header` und `footer` sind
nur außerhalb sektionierender Elemente Landmarks, `section` nur benannt eine
`region`, `img` mit leerem `alt` ist `presentation`.

## Grenze ohne Rendering

Schritt 2A der Spezifikation schließt versteckte Knoten aus. Ohne
Rendering-Daten sind nur `aria-hidden="true"` und das `hidden`-Attribut
erkennbar — `display: none` und `visibility: hidden` nicht. Ein Host mit
`a11y_dom::Rendering` kann hier genauer sein; dieses Crate bleibt bewusst bei
dem, was ohne Layout entscheidbar ist.

## Lizenz

MIT.
