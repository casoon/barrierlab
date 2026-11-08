//! End to end: one XML text against a RELAX NG and a Schematron schema, plus
//! the XPath data-model details the adapter has to get right.

use relax_ng::{
    CompiledSchema, DatatypeRegistry, ResolveError, SchemaResolver, SchemaSource, SchemaSyntax,
    ValidationErrorKind,
};
use schematron_engine::{CheckKind, evaluate};
use xml_adapters::{Position, RngElement, XmlDoc};
use xpath_eval::{Document, Node, NodeKind};

const RNG: &str = r#"
<grammar xmlns="http://relaxng.org/ns/structure/1.0"
         datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
  <start><element name="order">
    <attribute name="date"><data type="date"/></attribute>
    <oneOrMore>
      <element name="item">
        <attribute name="id"/>
        <element name="qty"><data type="positiveInteger"/></element>
      </element>
    </oneOrMore>
  </element></start>
</grammar>"#;

const SCH: &str = r#"
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <pattern>
    <rule context="item">
      <assert test="qty &lt;= 100">quantity of <value-of select="@id"/> is too large</assert>
    </rule>
    <rule context="order">
      <report test="count(item) = 1">single-item order</report>
    </rule>
  </pattern>
</schema>"#;

struct NoIncludes;

impl SchemaResolver for NoIncludes {
    fn resolve(&self, href: &str, _base: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("unexpected include {href}")))
    }
}

fn compile() -> CompiledSchema {
    let source = SchemaSource::new(RNG, "", SchemaSyntax::Xml);
    let schema = relax_ng::parse(&source).expect("schema parses");
    relax_ng::simplify(&schema, &NoIncludes).expect("schema compiles")
}

fn rng_problems(xml: &str) -> Vec<(ValidationErrorKind, Option<Position>)> {
    let document = roxmltree::Document::parse(xml).expect("well-formed");
    relax_ng::validate(
        &compile(),
        &DatatypeRegistry::new(),
        &RngElement::root(&document),
    )
    .expect("datatypes known")
    .into_iter()
    .map(|e| (e.kind().clone(), e.location().copied()))
    .collect()
}

fn sch_messages(xml: &str) -> Vec<(CheckKind, String, Option<Position>)> {
    let tree = XmlDoc::parse(xml).expect("well-formed");
    let schema = schematron_engine::parse(SCH).expect("schema parses");
    evaluate(&schema, &tree)
        .expect("evaluates")
        .into_iter()
        .map(|r| (r.kind, r.message, r.node.position()))
        .collect()
}

#[test]
fn valid_document_passes_both() {
    let xml = r#"<order date="2026-11-08"><item id="a"><qty>2</qty></item><item id="b"><qty>3</qty></item></order>"#;
    assert!(rng_problems(xml).is_empty());
    assert!(sch_messages(xml).is_empty());
}

#[test]
fn relax_ng_reports_problems_with_positions() {
    // The comment and the split text must not disturb the datatype check.
    let xml = "<order date=\"2026-11-08\">\n  <item id=\"a\"><qty>1<!-- c -->0</qty></item>\n  <item id=\"b\">\n    <qty>zero</qty></item>\n</order>";
    let problems = rng_problems(xml);
    // "10" (merged across the comment) is fine; "zero" is not. Every
    // problem sits on the second `qty`.
    assert!(!problems.is_empty());
    for (_, position) in &problems {
        let position = position.expect("location");
        assert_eq!((position.line, position.column), (4, 5));
        assert_eq!(&xml[position.offset..][..4], "<qty");
    }
}

#[test]
fn relax_ng_rejects_a_missing_attribute() {
    let problems = rng_problems(r#"<order><item id="a"><qty>1</qty></item></order>"#);
    assert!(!problems.is_empty());
}

#[test]
fn schematron_reports_with_positions() {
    let xml = "<order date=\"2026-11-08\">\n  <item id=\"a\"><qty>500</qty></item>\n</order>";
    let found = sch_messages(xml);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0].0, CheckKind::Assert);
    assert_eq!(found[0].1, "quantity of a is too large");
    assert_eq!(found[0].2.map(|p| (p.line, p.column)), Some((2, 3)));
    assert_eq!(found[1].0, CheckKind::Report);
    assert_eq!(found[1].1, "single-item order");
    assert_eq!(found[1].2.map(|p| p.line), Some(1));
}

#[test]
fn xpath_data_model() {
    let xml = r#"<?xml version="1.0"?>
<a xmlns:p="urn:p" xmlns="urn:d"><!-- note --><?target data?><b xml:id="x1" p:k="v">t<c/></b></a>"#;
    let tree = XmlDoc::parse(xml).unwrap();
    let root = Document::root(&tree);
    let a = root.children().next().unwrap();
    assert_eq!(a.kind(), NodeKind::Element);

    // Comment and PI are children, in order, with their text.
    let kinds: Vec<_> = a.children().map(|n| n.kind()).collect();
    assert_eq!(
        kinds,
        [
            NodeKind::Comment,
            NodeKind::ProcessingInstruction,
            NodeKind::Element
        ]
    );
    let pi = a.children().nth(1).unwrap();
    assert_eq!(pi.expanded_name().unwrap().local_name, "target");
    assert_eq!(pi.string_value(), "data");

    // Namespace axis: `xml`, the default (empty prefix) and `p`.
    let mut ns: Vec<_> = a
        .namespaces()
        .map(|n| (n.expanded_name().unwrap().local_name, n.string_value()))
        .collect();
    ns.sort();
    assert_eq!(
        ns,
        [
            ("".to_owned(), "urn:d".to_owned()),
            ("p".to_owned(), "urn:p".to_owned()),
            (
                "xml".to_owned(),
                "http://www.w3.org/XML/1998/namespace".to_owned()
            ),
        ]
    );

    // Names are expanded; the string value skips comment and PI.
    let b = a.children().nth(2).unwrap();
    assert!(b.has_expanded_name(Some("urn:d"), "b"));
    assert_eq!(a.string_value(), "t");
    assert_eq!(root.string_value(), "t");

    // `p:k` is namespaced, `xml:id` is an ID attribute and `p:k` is not.
    let attrs: Vec<_> = b.attributes().collect();
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs.iter().filter(|n| n.is_id_attribute()).count(), 1);
    assert!(
        attrs
            .iter()
            .any(|n| n.has_expanded_name(Some("urn:p"), "k"))
    );

    // Document order: element < namespace nodes < attributes < children.
    assert!(a.document_order(b).is_lt());
    assert!(
        b.namespaces()
            .next()
            .unwrap()
            .document_order(attrs[0])
            .is_lt()
    );
    assert!(
        attrs[1]
            .document_order(b.children().next().unwrap())
            .is_lt()
    );
    assert_eq!(b.parent(), Some(a));
    assert_eq!(attrs[0].parent(), Some(b));
    assert_eq!(b.position().map(|p| p.line), Some(2));
}

#[test]
fn xpath_functions_see_the_adapter() {
    // `id()` through `xml:id`, namespace and comment/PI node tests, all
    // through `schematron-engine` so the whole chain is exercised.
    let xml = r#"<r xmlns:p="urn:p"><!--c--><?t d?><e xml:id="k"/></r>"#;
    let sch = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
      <pattern><rule context="/r">
        <assert test="id('k')/self::e">id() finds xml:id</assert>
        <assert test="count(comment()) = 1 and count(processing-instruction('t')) = 1">comment and PI</assert>
        <assert test="count(namespace::*) = 2">namespace axis</assert>
        <assert test="count(e/namespace::*) = 2">inherited namespace nodes</assert>
      </rule></pattern></schema>"#;
    let tree = XmlDoc::parse(xml).unwrap();
    let reports = evaluate(&schematron_engine::parse(sch).unwrap(), &tree).unwrap();
    assert!(
        reports.is_empty(),
        "{:?}",
        reports.iter().map(|r| &r.message).collect::<Vec<_>>()
    );
}
