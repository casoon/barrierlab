//! The vendored jing spectest corpus (`relax-ng/tests/corpus/relaxng`), run
//! through `RngElement`. Every `<valid>`/`<invalid>` instance must get the
//! verdict the corpus gives it, which is also what `relax-ng`'s own
//! `spectest.rs` (with its private roxmltree glue) asserts.
//!
//! Cases with `<resource>`/`<dir>` (include/externalRef) are skipped: they
//! need a resolver that is not part of the adapter.

#![cfg(feature = "relax-ng")]

use std::ops::Range;

use relax_ng::{
    DatatypeRegistry, ResolveError, SchemaResolver, SchemaSource, SchemaSyntax, parse, simplify,
    validate,
};
use roxmltree::Node;
use xml_adapters::RngElement;

const SUITE: &str = "../relax-ng/tests/corpus/relaxng/spectest.xml";

struct NoResolver;

impl SchemaResolver for NoResolver {
    fn resolve(&self, href: &str, _base: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("no resolver for `{href}`")))
    }
}

/// roxmltree refuses any DTD; the suite's only entity is `&dii;`.
fn suite_text() -> String {
    let raw = std::fs::read_to_string(SUITE).expect("spectest.xml is vendored");
    let start = raw.find("<!DOCTYPE").expect("suite has a DOCTYPE");
    let end = start + raw[start..].find("]>").expect("internal subset") + 2;
    let dii = format!(
        "<{}{}/>",
        char::from_u32(0xE14).unwrap(),
        char::from_u32(0xE35).unwrap()
    );
    format!("{}{}", &raw[..start], &raw[end..]).replace("&dii;", &dii)
}

fn children<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(Node::is_element)
}

fn slice(text: &str, r: Range<usize>) -> String {
    text[r].to_owned()
}

#[test]
fn spectest_instances_through_the_adapter() {
    let text = suite_text();
    let suite = roxmltree::Document::parse(&text).expect("suite is well-formed");
    let registry = DatatypeRegistry::new();
    let (mut checked, mut located, mut failures) = (0usize, 0usize, Vec::new());

    for (i, case) in suite
        .descendants()
        .filter(|n| n.has_tag_name("testCase"))
        .enumerate()
    {
        let kids: Vec<_> = children(case).collect();
        if kids
            .iter()
            .any(|k| k.has_tag_name("resource") || k.has_tag_name("dir"))
        {
            continue;
        }
        let Some(schema) = kids
            .iter()
            .find(|k| k.has_tag_name("correct"))
            .and_then(|k| children(*k).next())
        else {
            continue;
        };
        let source = SchemaSource::new(slice(&text, schema.range()), "", SchemaSyntax::Xml);
        let compiled = parse(&source)
            .ok()
            .and_then(|d| simplify(&d, &NoResolver).ok())
            .expect("a correct schema compiles");

        for (tag, expected) in [("valid", true), ("invalid", false)] {
            for root in kids
                .iter()
                .filter(|k| k.has_tag_name(tag))
                .filter_map(|k| children(*k).next())
            {
                checked += 1;
                let xml = slice(&text, root.range());
                let doc = roxmltree::Document::parse(&xml).expect("instance is well-formed");
                let errors = validate(&compiled, &registry, &RngElement::root(&doc));
                let accepted = matches!(&errors, Ok(e) if e.is_empty());
                if accepted != expected {
                    failures.push(format!("jing-spectest-{:04} {tag}", i + 1));
                }
                if let Ok(errors) = errors {
                    located += errors.iter().filter(|e| e.location().is_some()).count();
                }
            }
        }
    }

    assert!(checked > 500, "only {checked} instances found");
    assert_eq!(failures, Vec::<String>::new());
    assert!(located > 0, "no error carried a position");
    eprintln!("adapter spectest: {checked} instances, {located} located errors");
}
