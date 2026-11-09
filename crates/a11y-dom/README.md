# a11y-dom

Document model abstraction for accessibility rules. Part of
[a11y-core](https://github.com/casoon/a11y-core).

Rules are written once and run over different substrates: static HTML from a
build, a Chrome driven over CDP, the DOM of a live page. This crate defines what
they have in common — and, more importantly, how they differ.

## The tree is DOM-shaped

`Node` models tags, attributes, text and hierarchy, not roles and accessible
names. This is deliberate: most rules need attributes (`tabindex`, `id`, `role`,
`alt`, `for`), and the browser's native accessibility tree does not provide them
— `tabindex` does not appear there.

## Capabilities instead of options

| | static HTML | Chrome via CDP | in-page WASM |
|---|---|---|---|
| `Document` — structure | ✓ | ✓ | ✓ |
| `Semantics` — role, name | computed | native | computed |
| `Rendering` — styles, geometry | — | ✓ | ✓ |
| `Interaction` — focus, events | — | ✓ | ✓ |

A flat trait with `Option` return values would make rules silently not run on
some hosts. Instead a host implements the traits it can serve — and a rule whose
tier is not met is recorded as `NotRun::CapabilityMissing` rather than staying silent.

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

The bundled `Arena` implements the traits — as a test vehicle and as the shape a
WASM host needs, because there it is not possible to call back into JavaScript
for every trait method.

## License

MIT.
