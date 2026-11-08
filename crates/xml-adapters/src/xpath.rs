use std::cmp::Ordering;

use xpath_eval::{Document, ExpandedName, Node, NodeKind};

use crate::Position;

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    attributes: Vec<usize>,
    namespaces: Vec<usize>,
    position: Option<Position>,
}

impl Data {
    fn new(kind: NodeKind, parent: Option<usize>) -> Self {
        Data {
            kind,
            name: None,
            text: None,
            parent,
            children: Vec::new(),
            attributes: Vec::new(),
            namespaces: Vec::new(),
            position: None,
        }
    }
}

/// An XPath 1.0 document model built from a `roxmltree` document, usable
/// with [`xpath_eval`] and, through it, `schematron-engine`.
///
/// The tree is copied into an arena in document order, so the document owns
/// its data and does not borrow the `roxmltree` document. What XPath adds
/// to the XML tree and `roxmltree` does not report is supplied here: every
/// element has the `xml` namespace node (XPath 1.0 §5.4), attributes and
/// namespace declarations are nodes of their own, comments and processing
/// instructions are kept. An empty namespace URI counts as "no namespace".
///
/// Attributes named `xml:id` are of type ID, so `id()` finds their element.
/// `roxmltree` does not expose DTD attribute types, so no other attribute is.
pub struct XmlDoc(Vec<Data>);

impl XmlDoc {
    /// Parses `xml` with `roxmltree`'s default options (no DTD) and builds
    /// the document.
    pub fn parse(xml: &str) -> Result<Self, roxmltree::Error> {
        Ok(Self::from_roxmltree(&roxmltree::Document::parse(xml)?))
    }

    /// Builds the document from an already parsed `roxmltree` document,
    /// for hosts that need other parsing options.
    pub fn from_roxmltree(document: &roxmltree::Document<'_>) -> Self {
        let mut nodes = vec![Data::new(NodeKind::Root, None)];
        add_children(&mut nodes, 0, document.root());
        XmlDoc(nodes)
    }
}

impl Document for XmlDoc {
    type N<'a> = XmlNode<'a>;

    fn root(&self) -> XmlNode<'_> {
        XmlNode(self, 0)
    }
}

/// A node of an [`XmlDoc`]. Cheap to copy; equal when it is the same node.
#[derive(Clone, Copy)]
pub struct XmlNode<'a>(&'a XmlDoc, usize);

impl XmlNode<'_> {
    /// Where the element starts in the source text. `None` for everything
    /// but elements.
    pub fn position(&self) -> Option<Position> {
        self.data().position
    }

    fn data(&self) -> &Data {
        &(self.0).0[self.1]
    }
}

impl PartialEq for XmlNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for XmlNode<'_> {}

impl std::fmt::Debug for XmlNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlNode")
            .field("index", &self.1)
            .field("name", &self.data().name)
            .finish()
    }
}

impl<'a> Node<'a> for XmlNode<'a> {
    fn kind(self) -> NodeKind {
        self.data().kind
    }

    fn parent(self) -> Option<Self> {
        self.data().parent.map(|p| XmlNode(self.0, p))
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let doc = self.0;
        doc.0[self.1].children.iter().map(move |&c| XmlNode(doc, c))
    }

    fn attributes(self) -> impl Iterator<Item = Self> + 'a {
        let doc = self.0;
        (doc.0[self.1].attributes.iter()).map(move |&c| XmlNode(doc, c))
    }

    fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
        let doc = self.0;
        (doc.0[self.1].namespaces.iter()).map(move |&c| XmlNode(doc, c))
    }

    fn expanded_name(self) -> Option<ExpandedName> {
        self.data().name.clone()
    }

    fn string_value(self) -> String {
        self.with_string_value(str::to_owned)
    }

    fn with_string_value<R>(self, f: impl FnOnce(&str) -> R) -> R {
        let d = self.data();
        match d.kind {
            NodeKind::Root | NodeKind::Element => {
                let mut out = String::new();
                collect_text(self.0, self.1, &mut out);
                f(&out)
            }
            _ => f(d.text.as_deref().unwrap_or_default()),
        }
    }

    fn has_expanded_name(self, namespace_uri: Option<&str>, local_name: &str) -> bool {
        self.data().name.as_ref().is_some_and(|n| {
            n.local_name == local_name && n.namespace_uri.as_deref() == namespace_uri
        })
    }

    fn document_order(self, other: Self) -> Ordering {
        // The arena is filled in document order.
        self.1.cmp(&other.1)
    }

    fn is_id_attribute(self) -> bool {
        self.data().name.as_ref().is_some_and(|n| {
            n.local_name == "id" && n.namespace_uri.as_deref() == Some(XML_NAMESPACE)
        })
    }
}

fn collect_text(doc: &XmlDoc, idx: usize, out: &mut String) {
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
fn name(namespace: Option<&str>, local: &str) -> ExpandedName {
    ExpandedName {
        namespace_uri: namespace.filter(|u| !u.is_empty()).map(str::to_owned),
        local_name: local.to_owned(),
    }
}

fn push(
    nodes: &mut Vec<Data>,
    parent: usize,
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
) -> usize {
    let mut data = Data::new(kind, Some(parent));
    data.name = name;
    data.text = text;
    nodes.push(data);
    nodes.len() - 1
}

fn add_children(nodes: &mut Vec<Data>, parent: usize, x: roxmltree::Node<'_, '_>) {
    for c in x.children() {
        match c.node_type() {
            roxmltree::NodeType::Element => {
                let tag = c.tag_name();
                let idx = push(
                    nodes,
                    parent,
                    NodeKind::Element,
                    Some(name(tag.namespace(), tag.name())),
                    None,
                );
                nodes[idx].position = Some(Position::of(c));
                nodes[parent].children.push(idx);
                // XPath 1.0 §5.4: every element has the `xml` namespace
                // node; roxmltree does not report it.
                let i = push(
                    nodes,
                    idx,
                    NodeKind::Namespace,
                    Some(name(None, "xml")),
                    Some(XML_NAMESPACE.to_owned()),
                );
                nodes[idx].namespaces.push(i);
                for n in c.namespaces() {
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Namespace,
                        Some(name(None, n.name().unwrap_or_default())),
                        Some(n.uri().to_owned()),
                    );
                    nodes[idx].namespaces.push(i);
                }
                for a in c.attributes() {
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Attribute,
                        Some(name(a.namespace(), a.name())),
                        Some(a.value().to_owned()),
                    );
                    nodes[idx].attributes.push(i);
                }
                add_children(nodes, idx, c);
            }
            roxmltree::NodeType::Text => {
                let t = c.text().unwrap_or_default().to_owned();
                let i = push(nodes, parent, NodeKind::Text, None, Some(t));
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Comment => {
                let t = c.text().unwrap_or_default().to_owned();
                let i = push(nodes, parent, NodeKind::Comment, None, Some(t));
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::PI => {
                let pi = c.pi().expect("a PI node has a PI");
                let i = push(
                    nodes,
                    parent,
                    NodeKind::ProcessingInstruction,
                    Some(name(None, pi.target)),
                    Some(pi.value.unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Root => {}
        }
    }
}
