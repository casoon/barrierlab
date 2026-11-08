# xml-adapters

Lets [`relax-ng`](https://docs.rs/relax-ng), [`xpath-eval`](https://docs.rs/xpath-eval)
and [`schematron-engine`](https://docs.rs/schematron-engine) read an XML tree you
already have. Those crates take documents through traits and parse none themselves;
this crate implements the traits for [`roxmltree`](https://docs.rs/roxmltree), so
validating an XML file against a RELAX NG and a Schematron schema takes a few lines.

```rust
use relax_ng::{DatatypeRegistry, ResolveError, SchemaResolver, SchemaSource, SchemaSyntax};
use xml_adapters::{RngElement, XmlDoc};

struct NoIncludes;
impl SchemaResolver for NoIncludes {
    fn resolve(&self, href: &str, _base: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("no include {href}")))
    }
}

let xml = r#"<order><item id="a1"/><item id="a2"/></order>"#;
let rng = r#"<element name="order" xmlns="http://relaxng.org/ns/structure/1.0">
    <oneOrMore><element name="item"><attribute name="id"/></element></oneOrMore>
</element>"#;
let sch = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
    <pattern><rule context="order">
        <assert test="count(item) &lt;= 5">at most five items</assert>
    </rule></pattern>
</schema>"#;

// RELAX NG: `roxmltree` parses, the adapter wraps the root element.
let document = roxmltree::Document::parse(xml)?;
let schema = relax_ng::parse(&SchemaSource::new(rng, "", SchemaSyntax::Xml))?;
let compiled = relax_ng::simplify(&schema, &NoIncludes)?;
let problems = relax_ng::validate(&compiled, &DatatypeRegistry::new(), &RngElement::root(&document))?;
assert!(problems.is_empty());

// Schematron: the same text as an XPath document.
let tree = XmlDoc::parse(xml)?;
let failed = schematron_engine::evaluate(&schematron_engine::parse(sch)?, &tree)?;
assert!(failed.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## What is covered

- **`RngElement`** (feature `relax-ng`): `relax_ng::Element` over a `roxmltree`
  element. Text runs are merged across comments and processing instructions,
  `xmlns` declarations are not attributes, the prefix bindings in scope (inherited
  ones included) serve `QName` values, and an error carries the `Position` of the
  element's start tag.
- **`XmlDoc` / `XmlNode`** (feature `xpath`): `xpath_eval::Document` and `Node`,
  and with them `schematron_engine::evaluate`. The tree is copied into an arena in
  document order. Every element has the `xml` namespace node and the declared
  ones; attributes are nodes; comments and processing instructions are kept; an
  empty namespace URI means "no namespace". `xml:id` attributes are of type ID, so
  `id()` works. `XmlNode::position()` gives an element's source `Position`, e.g.
  for a Schematron `Report::node`.
- **`Position`**: line and column from 1, byte offset from 0.

## What is not

- Only `roxmltree`. Other trees (`xmltree`, `quick-xml` DOMs) are not adapted.
- No DTD attribute types, so `id()` knows only `xml:id`. `XmlDoc::parse` uses
  `roxmltree`'s defaults (no DTD); for other options parse yourself and call
  `XmlDoc::from_roxmltree`.
- Nothing is parsed, fetched or resolved here: schema loading and `include`
  resolution stay with `relax-ng` and `schematron-engine`.

Both features are on by default; `default-features = false` with one of them
leaves out the other crate. MIT.
