# accname

WAI-ARIA Accessible Name and Description Computation in Rust, generic over the
document model from [`a11y-dom`](https://crates.io/crates/a11y-dom). Part of
[a11y-core](https://github.com/casoon/a11y-core).

An implementation of [accname 1.2](https://w3c.github.io/accname/) and the naming
rules of [HTML-AAM 1.0](https://www.w3.org/TR/html-aam-1.0/) — written against
the specification, not against an existing implementation. The step numbers in
the source match those of the normative text.

## Why this cannot be done in passing

An approximation of "subtree text plus `aria-label`" is wrong in exactly the
cases where it matters:

- `aria-labelledby` resolves chains of references and may pull in nodes that are
  otherwise hidden
- an embedded control contributes its *value* within a recursion, not its label
- for roles such as `generic` or `paragraph`, a name is simply forbidden

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

Besides `name` there are `description` and `role` — the latter as role
computation per HTML-AAM, including the context-dependent cases: `header` and
`footer` are landmarks only outside sectioning elements, `section` is a `region`
only when named, and `img` with an empty `alt` is `presentation`.

## Limit without rendering

Step 2A of the specification excludes hidden nodes. Without rendering data only
`aria-hidden="true"` and the `hidden` attribute are detectable — `display: none`
and `visibility: hidden` are not. A host with `a11y_dom::Rendering` can be more
precise here; this crate deliberately stays with what can be decided without
layout.

## License

MIT.
