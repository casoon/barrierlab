//! Test glue: `roxmltree` text into the `xml-adapters` document model, which
//! `jaxen.rs` and `xpath10_regression.rs` share. (The dev-dependency loops
//! back to this crate; Cargo allows that for dev-dependencies, and the
//! path-only entry is dropped when the crate is packaged.)

#![allow(dead_code)]

pub use xml_adapters::{XmlDoc as Doc, XmlNode as N};

/// Parses `xml` (DTD allowed) into the document model.
pub fn parse_xml(xml: &str) -> Doc {
    let x = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    Doc::from_roxmltree(&x)
}
