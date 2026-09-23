# a11y-dom

Dokumentmodell-Abstraktion für Accessibility-Regeln. Teil von
[a11y-core](https://github.com/casoon/a11y-core).

Regeln werden einmal geschrieben und laufen über verschiedene Substrate:
statisches HTML aus einem Build, ein per CDP ferngesteuerter Chrome, der DOM
einer laufenden Seite. Dieses Crate definiert, was diese gemeinsam haben — und,
wichtiger, wie sie sich unterscheiden.

## Der Baum ist DOM-förmig

`Node` bildet Tags, Attribute, Text und Hierarchie ab, nicht Rollen und
Accessible Names. Das ist bewusst: Die Mehrzahl der Regeln braucht Attribute
(`tabindex`, `id`, `role`, `alt`, `for`), und der native Accessibility-Tree des
Browsers gibt die gar nicht her — `tabindex` taucht dort nicht auf.

## Fähigkeiten statt Optionen

| | statisches HTML | Chrome via CDP | In-Page WASM |
|---|---|---|---|
| `Document` — Struktur | ✓ | ✓ | ✓ |
| `Semantics` — Rolle, Name | berechnet | nativ | berechnet |
| `Rendering` — Stile, Geometrie | — | ✓ | ✓ |
| `Interaction` — Fokus, Ereignisse | — | ✓ | ✓ |

Ein flaches Trait mit `Option`-Rückgaben würde dazu führen, dass Regeln je nach
Host stillschweigend nicht laufen. Stattdessen implementiert ein Host die
Traits, die er bedienen kann — und eine Regel, deren Tier nicht erfüllt ist,
wird als `Untested` vermerkt statt zu schweigen.

```rust
use a11y_dom::{elements, subtree_text, Arena, Document, Node};

let doc = Arena::builder()
    .open("html").attr("lang", "de")
        .open("body").open("h1").text("Bericht").close().close()
    .close()
    .build();

let h1 = elements(&doc).find(|n| n.local_name() == "h1").unwrap();
assert_eq!(subtree_text(h1), "Bericht");
```

Die mitgelieferte `Arena` erfüllt die Traits — als Testvehikel und als die Form,
die ein WASM-Host braucht, weil dort nicht pro Trait-Methode nach JavaScript
zurückgerufen werden kann.

## Lizenz

MIT.
