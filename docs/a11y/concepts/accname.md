---
title: "Accessible Name"
description: "What the accname crate computes, what it is written against and where it stops without rendering data."
order: 3
---

`accname` implements [Accessible Name and Description Computation 1.2](https://w3c.github.io/accname/)
and the naming rules from [HTML-AAM 1.0](https://www.w3.org/TR/html-aam-1.0/), generic over
the document model from `a11y-dom`. It is written against the specification, not against an
existing implementation; the step numbers in the source match those of the normative text.

```rust
use a11y_dom::{elements, Arena, Document, Node};
use accname::{name, IdIndex};

let doc = Arena::builder()
    .open("form")
        .open("span").attr("id", "l").text("First name").close()
        .open("input").attr("id", "v").attr("aria-labelledby", "l").close()
    .close()
    .build();

let ids = IdIndex::build(doc.root());
let field = elements(&doc).find(|n| n.local_name() == "input").unwrap();
assert_eq!(name(field, &ids).as_deref(), Some("First name"));
```

## Why this cannot be done in passing

An approximation made of "subtree text plus `aria-label`" is wrong in exactly the cases where it
matters:

- `aria-labelledby` resolves reference chains and may draw on nodes that are otherwise hidden.
- An embedded control contributes its *value* within a recursion, not its label.
- For roles such as `generic` or `paragraph`, a name is prohibited.

## What is covered

Steps 2A to 2I: `aria-labelledby` with reference chains and cycle protection, `aria-label`, the
native HTML text alternatives (`alt`, `label` via `for` and wrapping, `legend`, `caption`,
`figcaption`, `title` in SVG, the special cases of `input`), embedded controls, name from
content, `title` as the last fallback.

Plus role computation per HTML-AAM with the context-dependent cases: `header` and `footer`
are landmarks only outside sectioning elements, `section` is a `region` only when named,
an `img` with empty `alt` is `presentation`.

## The public API

| Function | Purpose |
|---|---|
| `name`, `description` | Accessible name and description of a node |
| `role` | the effective role, explicit or implicit |
| `implicit` | the implicit role per HTML-AAM |
| `allows_name_from_content`, `name_is_prohibited` | properties of a role |
| `IdIndex` | ID index for references, built once per document |

The detailed documentation is on [docs.rs/accname](https://docs.rs/accname).

## Limit without rendering

Step 2A excludes hidden nodes. Without rendering data, only `aria-hidden="true"` and
the `hidden` attribute are detectable, `display: none` and `visibility: hidden` are not. A host with
`Rendering` can be more precise here; the crate deliberately stays with what is decidable without
layout.
