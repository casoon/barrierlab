//! The internal host: structure from `html5_parser::Document` (via
//! `a11y-dom`), semantics from `accname`, rendering from `style-cascade`.

use a11y_dom::{
    ColorPairs, ComputedStyle, Document, Layout, Node, NodeKind, Rect, Rendering, Semantics,
    ancestors,
};
use accname::IdIndex;
use html5_parser::Document as Html5Document;
use style_cascade::{Cascade, Environment, FocusIndicator, Specified};
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

    /// Der angegebene Wert, klein geschrieben: `Some(None)` für den
    /// Anfangswert, `None`, wenn er sich nicht bestimmen lässt.
    fn value(&self, node: Html5Node<'_>, prop: &str) -> Option<Option<String>> {
        match self.cascade.specified(node, prop) {
            Specified::Set(v) => Some(Some(v.trim().to_ascii_lowercase())),
            Specified::Unset => Some(None),
            Specified::Unknown => None,
        }
    }

    /// Ob auf dem Element eine Animation endlos läuft: Name nicht `none`,
    /// Dauer über 0, `infinite` und nicht `paused`. Die Listen der
    /// `animation-*`-Werte gehören paarweise zusammen; eine kürzere
    /// wiederholt sich wie in CSS.
    fn infinite_animation(&self, node: Html5Node<'_>) -> Option<bool> {
        let list = |p: &str, initial: &str| -> Option<Vec<String>> {
            let v = self.value(node, p)?.unwrap_or_else(|| initial.to_string());
            Some(v.split(',').map(|s| s.trim().to_string()).collect())
        };
        let names = list("animation-name", "none")?;
        let durations = list("animation-duration", "0s")?;
        let counts = list("animation-iteration-count", "1")?;
        let states = list("animation-play-state", "running")?;
        let at = |l: &[String], i: usize| l[i % l.len()].clone();
        Some(names.iter().enumerate().any(|(i, name)| {
            name != "none"
                && at(&counts, i) == "infinite"
                && at(&states, i) != "paused"
                && seconds(&at(&durations, i)).is_some_and(|d| d > 0.0)
        }))
    }
}

fn seconds(value: &str) -> Option<f32> {
    match value.strip_suffix("ms") {
        Some(ms) => ms.parse::<f32>().ok().map(|ms| ms / 1000.0),
        None => value.strip_suffix('s')?.parse().ok(),
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

    fn accessible_description<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::description(node, &self.ids)
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

    /// Gradients, `opacity` and `text-shadow`: the exact colour pairs, not
    /// a pixel sample (the text lies over one part of a gradient, which part
    /// a static host does not know).
    fn color_pairs<'n>(&'n self, node: Self::N<'n>) -> Option<ColorPairs> {
        if self.incomplete {
            return None;
        }
        self.cascade.color_pairs(node)
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

    /// Was die Kaskade über die Anordnung sagt: `order`, umgekehrte
    /// Flex-Richtung, `cursor: pointer` und endlose Animationen — nur an
    /// dargestellten Elementen, wie ein Browser sie nur an Elementen mit
    /// Layout-Objekt misst. Ein Wert, den die Kaskade nicht bestimmen kann,
    /// bleibt `None`; fehlen Stylesheets, ist nichts gemessen. Was ein Layout
    /// braucht (`min-width` in Pixeln, Überdeckung, Fokus), misst er nicht.
    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        if self.incomplete || node.kind() != NodeKind::Element || !self.is_rendered(node) {
            return None;
        }
        let flex = matches!(
            self.value(node, "display"),
            Some(Some(d)) if d == "flex" || d == "inline-flex"
        );
        let flex_reversed = if flex {
            self.value(node, "flex-direction")
                .map(|d| d.is_some_and(|d| d.ends_with("-reverse")))
        } else {
            Some(false)
        };
        let order = self
            .value(node, "order")
            .and_then(|o| o.map_or(Some(0), |o| o.parse().ok()));
        let cursor_pointer = self
            .value(node, "cursor")
            .map(|c| c.as_deref() == Some("pointer"));
        // Nur wo die Tabtaste hinkommt: die Kaskade wird dafür je Element ein
        // zweites Mal gerechnet.
        let (focus_visible, focus_outline_removed) = if reachable_by_tab(node) {
            match self.cascade.focus_indicator(node) {
                FocusIndicator::Shown => (Some(true), Some(false)),
                FocusIndicator::Removed => (Some(false), Some(true)),
                FocusIndicator::Unknown => (None, None),
            }
        } else {
            (None, None)
        };
        Some(Layout {
            flex_reversed,
            order,
            cursor_pointer,
            infinite_animation: self.infinite_animation(node),
            focus_visible,
            focus_outline_removed,
            ..Layout::default()
        })
    }
}

/// Per Tab erreichbar, wie `a11y-rules` es für die Fokusregeln zählt: ein
/// `tabindex` ab 0, sonst ein Link mit `href`, ein Feld, eine Schaltfläche,
/// `summary` oder `iframe`.
fn reachable_by_tab<'a, N: Node<'a>>(n: N) -> bool {
    if let Some(t) = n
        .attr("tabindex")
        .and_then(|t| t.trim().parse::<i32>().ok())
    {
        return t >= 0;
    }
    (match n.local_name() {
        "a" | "area" => n.has_attr("href"),
        "input" => !n
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        "button" | "select" | "textarea" | "summary" | "iframe" => true,
        _ => false,
    }) && !(n.has_attr("disabled")
        && matches!(n.local_name(), "button" | "input" | "select" | "textarea"))
}
