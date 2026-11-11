---
title: "relax-ng"
description: "RELAX NG in pure Rust: reads compact and XML schemas, simplifies them and validates instances with Brzozowski derivatives, from a tree or from events."
order: 11
---

Reads a RELAX NG schema (compact and XML syntax), simplifies it and validates instances with
Brzozowski derivatives. Norm: the RELAX NG Specification. The core contains no XML parser;
instances come in through the `Element` trait (tree) or as events (`StreamValidator`).

## Streaming validation

`StreamValidator` validates a document from events, without the caller building an `Element`
tree (barrierlab#77). The tree path (`validate`, `Schema::validate`) is built on it: it turns the
`Element` tree into the same events, so there is one implementation of the derivation steps.

```rust
let mut v = schema.stream_validator::<()>(&registry)?; // or StreamValidator::new
v.start_element(name, attributes, namespace_bindings, location)?;
v.text("…")?;           // any number of times, joined up to the next tag
v.end_element()?;
let findings = v.finish()?; // Vec<ValidationError<L>>, empty = valid
```

### Event model

| Event | Derivative step |
|---|---|
| `start_element(name, attributes, bindings, location)` | `startTagOpen` (derivative of the parent pattern by the name; determines the content of the child), then `attDeriv` per attribute (unordered); `startTagClose` is implicit: the child content starts with the remaining pattern |
| `text(&str)` | collected in the frame of the open element |
| next `start_element` / `end_element` | the collected text is derived as **one** text node (`textDeriv`) |
| `end_element` | derive remaining text, then the nullable check (`endTag`); otherwise `MissingContent` |
| `finish` | checks that exactly one root element was complete |

Attributes arrive bundled with `start_element`, not as separate events: a pull parser delivers
them with the start tag anyway, and so there is no state "tag open, attributes incomplete".

### Text

- `data`, `value` and `list` need the complete text of a text node. Text is therefore buffered
  per open element and derived only at a child boundary (start or end tag). Consecutive `text`
  calls are one run; joining text across comments stays with the caller (as with
  `Content::Text`).
- Whitespace only (§6.2.7, weak match): next to other children it is not derived. "Next to other
  children" means that an earlier child existed or that a child starts right after it. This is
  the tree path's criterion `child count > 1`, only without looking ahead to the end of the
  element. An element whose only child is a whitespace run is offered that run as before.
- Text that does not match is `UnexpectedText`, except for whitespace when nothing in the pattern
  asks for text (`wants_text`), identical to the tree path.

### State

One frame per open element: remaining pattern (interned, `Id`), datatype context (namespace
bindings), location, buffered text, child counter. In addition the path of element names, the
list of findings and a counter for skipped subtrees. Memory grows with nesting depth, findings,
the length of the longest undivided text run and the pattern interner of the schema, not with the
size of the document. (The interner grows with the number of distinct patterns that occur; the
memo of attribute derivatives keeps at most 1,024 distinct values per pattern and attribute name,
so unique ids or URLs do not add up.)

### Measured

`cargo run --release --example stream_vs_tree -- 1000000 [tree|stream]` validates one generated
document of 1,000,000 `<item id="…">entry …</item>` elements with unique ids against
`element list { element item { attribute id { text }, text }* }`, in a process of its own per
path, and reports the peak heap (counting allocator) and the time.

| Path | Peak heap | Time |
|---|---:|---:|
| Tree, building it included | 185 MiB | 0.40 s |
| Push (`StreamValidator`) | 0.3 MiB | 0.33 s |

The tree path pays for the tree the caller has to build; the push path holds only the open
elements. (Before the attribute memo was capped, the same run peaked at 487 MiB and 302 MiB.)

Moving the tree path onto the push core changed neither number: before and after, median of 10
runs on one machine, the tree run took 0.35 s (building the tree included) and peaked at 185 MiB,
the push run 0.25 s.

### Positions

The caller passes `location: Option<L>` per element; `L: Clone`. A finding carries the location
of the element it arises on (attributes, text, `MissingContent`) or of the unexpected child
(`UnexpectedElement`), as in the tree path. The paths are the same `ExpandedName` lists.

### Unexpected elements

An unexpected element yields `UnexpectedElement` and its subtree is skipped (events inside it are
counted, not checked), as in the tree path. The pattern of the parent does not advance.

### Errors in the event stream

An invalid event stream (end tag without start, second root, text outside the root other than
whitespace, `finish` before the end of the root) is a `StreamError`, never a finding and never a
silent success. After it the state is undefined; the validator is discarded. A schema with an
unsupported datatype fails in `new` with `DatatypeError`, as in the tree path.

### Limits

- A single text run is buffered whole.
- A `StreamValidator` validates one document; the next one needs a new validator (the schema
  interner is not shared).
- No recovery after an unexpected element in the public push API:
  `ValidationOptions::recover_after_unexpected_element` (#76) is offered only through the tree
  path (`validate_with_options`). Internally the recovery lives in the shared core and is
  switched on by the tree path; `StreamValidator` always runs with it off, and without the option
  both paths report the same.
- No parser: callers pass names already expanded, without `xmlns` attributes.

### Equivalence

There is one implementation of the derivation steps, `StreamValidator` (`src/validator/stream.rs`).
The tree path (`drive` in `src/validator.rs`) walks the `Element` tree iteratively and feeds it the
events; the opt-in recovery (`ValidationOptions`) is a mode of that core, so findings, their order
and the recovery are the same by construction. Two things stay with the tree path: it does not
descend into a subtree the core skips (no `children()` call there), and it reads a finding's
`Element::Location` from the element while it is still open (the core only carries a serial
number), so `Element::Location` needs no `Clone`. Tests check that the two drivers agree:
the spec test (`tests/spectest.rs`) drives every `valid`/`invalid` instance through
both paths, with text runs split in two, and requires **the same findings in the same order**, not
only the same verdict. An ignored test in `html-conform` checks the same over the whole vnu corpus
(`cargo test --release -p html-conform --lib stream_and_tree -- --ignored`).
