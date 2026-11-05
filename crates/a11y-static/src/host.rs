//! The internal host: structure from `html5_parser::Document` (via
//! `a11y-dom`), semantics from `accname`, rendering from `style-cascade`.

use a11y_dom::{
    Backdrop, ComputedStyle, Document, Node, NodeKind, Rect, Rendering, Semantics, ancestors,
};
use accname::IdIndex;
use html5_parser::Document as Html5Document;
use style_cascade::{Cascade, Environment};
use stylesheet_parse::Stylesheet;

type Html5Node<'a> = a11y_dom::Html5Node<'a>;

pub(crate) struct Host<'a> {
    doc: &'a Html5Document,
    ids: IdIndex<'a, Html5Node<'a>>,
    cascade: Cascade,
    /// Styles are known to be incomplete: no colour is claimed.
    incomplete: bool,
}

impl<'a> Host<'a> {
    pub(crate) fn new(
        doc: &'a Html5Document,
        sheets: &[Stylesheet],
        env: Environment,
        missing_sheets: bool,
    ) -> Self {
        let root = <Html5Document as Document>::root(doc);
        let cascade = Cascade::new(sheets, env);
        Host {
            doc,
            ids: IdIndex::build(root),
            incomplete: missing_sheets || !cascade.unresolved_imports().is_empty(),
            cascade,
        }
    }

    pub(crate) fn unresolved_imports(&self) -> &[String] {
        self.cascade.unresolved_imports()
    }
}

impl Document for Host<'_> {
    type N<'n>
        = Html5Node<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        <Html5Document as Document>::root(self.doc)
    }
}

impl Semantics for Host<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name_rendered(self, node, &self.ids)
    }

    /// `aria-hidden="true"` or `inert` on the node or an ancestor. The
    /// `hidden` attribute is a rendering matter (UA `display: none` in the
    /// cascade) and reaches the rules through [`Rendering::is_rendered`].
    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        std::iter::once(node)
            .chain(ancestors(node))
            .any(|n| n.attr("aria-hidden") == Some("true") || n.has_attr("inert"))
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        (node.kind() == NodeKind::Element).then(|| {
            let mut style = self.cascade.computed_style(node);
            if self.incomplete {
                // A missing sheet may set any colour: claim none, so the
                // contrast rules come out undetermined instead of passed.
                // `display` and `visibility` stay: hiding still works.
                style.color = None;
                style.background_color = None;
                style.own_background_color = None;
                style.border_top_color = None;
            }
            style
        })
    }

    /// Without layout there are no boxes.
    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }

    fn sampled_backdrop<'n>(&'n self, node: Self::N<'n>) -> Option<Backdrop> {
        if self.incomplete {
            return None;
        }
        self.cascade
            .backdrop_luminances(node)
            .map(|luminance| Backdrop { luminance })
    }

    /// Neither the node nor an ancestor has `display: none` (which includes
    /// the `hidden` attribute through the UA rules), and the node is not
    /// `visibility: hidden|collapse`.
    fn is_rendered<'n>(&'n self, node: Self::N<'n>) -> bool {
        let visible = self
            .cascade
            .computed_style(node)
            .visibility
            .is_none_or(|v| v != "hidden" && v != "collapse");
        let none =
            |n: Html5Node<'_>| self.cascade.computed_style(n).display.as_deref() == Some("none");
        visible && !std::iter::once(node).chain(ancestors(node)).any(none)
    }
}
