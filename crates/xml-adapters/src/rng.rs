use relax_ng::{Content, Element, ExpandedName, ExpandedNameRef};
use roxmltree::Node;

use crate::Position;

/// A `roxmltree` element seen as a [`relax_ng::Element`].
///
/// Adjacent text (also across comments and processing instructions, which
/// RELAX NG does not see) is merged into one run. The location of an error
/// is the [`Position`] of the offending element's start tag.
#[derive(Clone, Copy, Debug)]
pub struct RngElement<'a, 'input>(Node<'a, 'input>);

impl<'a, 'input> RngElement<'a, 'input> {
    /// Wraps `node`, which must be an element.
    ///
    /// # Panics
    ///
    /// If `node` is not an element.
    pub fn new(node: Node<'a, 'input>) -> Self {
        assert!(node.is_element(), "RngElement needs an element node");
        RngElement(node)
    }

    /// The root element of `document`.
    pub fn root(document: &'a roxmltree::Document<'input>) -> Self {
        RngElement(document.root_element())
    }

    /// The wrapped node.
    pub fn node(&self) -> Node<'a, 'input> {
        self.0
    }
}

impl Element for RngElement<'_, '_> {
    type Location = Position;

    fn name(&self) -> ExpandedName {
        let tag = self.0.tag_name();
        ExpandedName::new(tag.namespace(), tag.name())
    }

    fn attributes(&self) -> impl Iterator<Item = (ExpandedNameRef<'_>, &str)> {
        // roxmltree keeps `xmlns` declarations apart from attributes.
        self.0.attributes().map(|a| {
            (
                ExpandedNameRef {
                    namespace: a.namespace(),
                    local: a.name(),
                },
                a.value(),
            )
        })
    }

    fn children(&self) -> impl Iterator<Item = Content<Self>> {
        let mut out = Vec::new();
        let mut text = String::new();
        for child in self.0.children() {
            if child.is_element() {
                if !text.is_empty() {
                    out.push(Content::Text(std::mem::take(&mut text)));
                }
                out.push(Content::Element(RngElement(child)));
            } else if child.is_text() {
                text.push_str(child.text().unwrap_or_default());
            }
        }
        if !text.is_empty() {
            out.push(Content::Text(text));
        }
        out.into_iter()
    }

    fn location(&self) -> Option<Position> {
        Some(Position::of(self.0))
    }

    fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
        // `namespaces()` includes those inherited from ancestors.
        self.0
            .namespaces()
            .map(|n| (n.name().unwrap_or("").to_owned(), n.uri().to_owned()))
    }
}
