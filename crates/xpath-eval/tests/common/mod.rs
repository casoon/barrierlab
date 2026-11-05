//! Test-only adapter: an arena document model over `roxmltree`, so the tests can
//! feed XML text to `xpath-eval`. Shared by `jaxen.rs` and
//! `xpath10_regression.rs`.

#![allow(dead_code)]

use std::cmp::Ordering;

use xpath_eval::{Document, ExpandedName, Node as XNode, NodeKind};

pub struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    attributes: Vec<usize>,
    namespaces: Vec<usize>,
}

/// Arena in document order, so `document_order` is an index comparison.
pub struct Doc(pub Vec<Data>);

#[derive(Clone, Copy)]
pub struct N<'a>(pub &'a Doc, pub usize);

impl PartialEq for N<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for N<'_> {}

impl Document for Doc {
    type N<'a> = N<'a>;
    fn root(&self) -> N<'_> {
        N(self, 0)
    }
}

impl<'a> XNode<'a> for N<'a> {
    fn kind(self) -> NodeKind {
        self.0.0[self.1].kind
    }
    fn parent(self) -> Option<Self> {
        self.0.0[self.1].parent.map(|p| N(self.0, p))
    }
    fn children(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1].children.iter().map(move |&c| N(self.0, c))
    }
    fn attributes(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1]
            .attributes
            .iter()
            .map(move |&c| N(self.0, c))
    }
    fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1]
            .namespaces
            .iter()
            .map(move |&c| N(self.0, c))
    }
    fn expanded_name(self) -> Option<ExpandedName> {
        self.0.0[self.1].name.clone()
    }
    fn string_value(self) -> String {
        let d = &self.0.0[self.1];
        match d.kind {
            NodeKind::Root | NodeKind::Element => {
                let mut out = String::new();
                collect_text(self.0, self.1, &mut out);
                out
            }
            _ => d.text.clone().unwrap_or_default(),
        }
    }
    fn document_order(self, other: Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

fn collect_text(doc: &Doc, idx: usize, out: &mut String) {
    for &c in &doc.0[idx].children {
        let d = &doc.0[c];
        match d.kind {
            NodeKind::Text => out.push_str(d.text.as_deref().unwrap_or_default()),
            NodeKind::Element => collect_text(doc, c, out),
            _ => {}
        }
    }
}

/// XPath knows "in a namespace" or "in none", never the empty URI.
fn ns(uri: Option<&str>) -> Option<String> {
    uri.filter(|u| !u.is_empty()).map(str::to_owned)
}

fn push(
    nodes: &mut Vec<Data>,
    parent: usize,
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
) -> usize {
    let idx = nodes.len();
    nodes.push(Data {
        kind,
        name,
        text,
        parent: Some(parent),
        children: Vec::new(),
        attributes: Vec::new(),
        namespaces: Vec::new(),
    });
    idx
}

fn add(nodes: &mut Vec<Data>, parent: usize, x: roxmltree::Node<'_, '_>) {
    for c in x.children() {
        match c.node_type() {
            roxmltree::NodeType::Element => {
                let name = ExpandedName {
                    namespace_uri: ns(c.tag_name().namespace()),
                    local_name: c.tag_name().name().to_owned(),
                };
                let idx = push(nodes, parent, NodeKind::Element, Some(name), None);
                nodes[parent].children.push(idx);
                // XPath 1.0 (5.4): every element has the `xml` namespace node;
                // roxmltree does not report it.
                let xml = ExpandedName {
                    namespace_uri: None,
                    local_name: "xml".to_owned(),
                };
                let uri = "http://www.w3.org/XML/1998/namespace".to_owned();
                let i = push(nodes, idx, NodeKind::Namespace, Some(xml), Some(uri));
                nodes[idx].namespaces.push(i);
                for n in c.namespaces() {
                    let name = ExpandedName {
                        namespace_uri: None,
                        local_name: n.name().unwrap_or_default().to_owned(),
                    };
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Namespace,
                        Some(name),
                        Some(n.uri().to_owned()),
                    );
                    nodes[idx].namespaces.push(i);
                }
                for a in c.attributes() {
                    let name = ExpandedName {
                        namespace_uri: ns(a.namespace()),
                        local_name: a.name().to_owned(),
                    };
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Attribute,
                        Some(name),
                        Some(a.value().to_owned()),
                    );
                    nodes[idx].attributes.push(i);
                }
                add(nodes, idx, c);
            }
            roxmltree::NodeType::Text => {
                let i = push(
                    nodes,
                    parent,
                    NodeKind::Text,
                    None,
                    Some(c.text().unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Comment => {
                let i = push(
                    nodes,
                    parent,
                    NodeKind::Comment,
                    None,
                    Some(c.text().unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::PI => {
                let pi = c.pi().expect("PI node");
                let name = ExpandedName {
                    namespace_uri: None,
                    local_name: pi.target.to_owned(),
                };
                let i = push(
                    nodes,
                    parent,
                    NodeKind::ProcessingInstruction,
                    Some(name),
                    Some(pi.value.unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Root => {}
        }
    }
}

/// Parses `xml` (DTD allowed) into the arena model.
pub fn parse_xml(xml: &str) -> Doc {
    let x = roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut nodes = vec![Data {
        kind: NodeKind::Root,
        name: None,
        text: None,
        parent: None,
        children: Vec::new(),
        attributes: Vec::new(),
        namespaces: Vec::new(),
    }];
    add(&mut nodes, 0, x.root());
    Doc(nodes)
}
