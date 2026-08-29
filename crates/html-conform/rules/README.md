# rules

Schematron rules (`.sch`) expressing the assertion layer's co-constraints
(ARIA combinations, form-label requirements, ID uniqueness, etc.) as
declarative XPath — the only place for this domain logic. No Rust code for
co-constraints; see `assertions.rs` and `CLAUDE.md`.

## Named element tests need the `h:` namespace prefix

`src/infoset.rs` gives every plain HTML element the XHTML namespace
(`http://www.w3.org/1999/xhtml`) — needed so `relax_ng`'s vendored HTML5
schema validates correctly (see `plan/DECISIONS.md`'s adapter-contract
entry). XPath 1.0's unprefixed name tests (`th`, `input`, `self::font`, …)
only ever match nodes with **no** namespace — the `xmlns` default
namespace is explicitly excluded from name-test expansion (XPath 1.0 §2.3).
An unprefixed element name test against this infoset therefore silently
matches nothing, not an error, no diagnostic — easy to miss.

Every rule file that names an element (not just `*`) must declare and use
the XHTML namespace prefix:

```xml
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="...">
    <rule context="h:th[@scope]">
      ...
```

Attribute tests (`@scope`, `@type`, `@aria-hidden`, …) do **not** need the
prefix — ordinary HTML attributes carry no namespace in this crate's
infoset, matching XPath's own unprefixed-attribute expansion rule. Only
*element* name tests need `h:`. The `*` wildcard element test also needs
no prefix (namespace-agnostic by definition). See `rules/aria.sch` (no
prefix needed for its first rule, wildcard-only) vs. `rules/tables.sch`/
`obsolete-elements.sch` (named element tests, prefix required) for worked
examples.
