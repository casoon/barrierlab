//! Der Geltungsbereich einer Regel: welche Knoten sie überhaupt sieht.
//!
//! Ein Element in einem versteckten Teilbaum ist für niemanden da — weder
//! wird es gezeichnet, noch steht es im Accessibility-Tree. Lief eine Regel
//! trotzdem darüber, bekam etwa ein Button im versteckten Banner einen leeren
//! Namen (die Namensberechnung überspringt versteckten Inhalt zu Recht) und
//! damit einen `FAIL`. Gemessen am 29.09.2026 auf barrierlab.eu: vier
//! kritische Befunde auf jeder der 237 Seiten, alle an Versteckten.
//!
//! Die Regeln selbst bleiben davon unberührt. Sie laufen über eine [`Visibility`]
//! auf das Dokument, in der die versteckten Teilbäume schlicht fehlen — so
//! kann keine Regel sie vergessen, und keine muss sie einzeln ausnehmen.

use std::collections::HashSet;

use a11y_dom::{
    ComputedStyle, Document, Layout, NameSource, Node, NodeId, NodeKind, Rect, Rendering,
    Semantics, TreeScope,
};

/// Welche Knoten eine Regel betrachtet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Scope {
    /// Was im Accessibility-Tree steht: ohne `aria-hidden="true"` und ohne
    /// Nicht-Dargestelltes. Der Normalfall.
    AccessibilityTree,
    /// Was dargestellt wird, auch unter `aria-hidden`. Für Regeln, bei denen
    /// das Verstecken vor der Assistenztechnik selbst der Befund ist, und für
    /// Fokus und Kontrast, die Sehende und Tastaturnutzer betreffen.
    Rendered,
    /// Das ganze Markup. Für Aussagen über das Dokument und über IDs, auf die
    /// auch versteckte Elemente verweisen dürfen.
    Markup,
}

/// Die Wurzeln der versteckten Teilbäume, je Geltungsbereich.
///
/// Gemerkt werden nur die Wurzeln: Unter einer ausgelassenen Wurzel kommt die
/// Traversierung nie an, die Nachfahren müssen nicht in der Menge stehen.
pub(crate) struct Hidden {
    not_rendered: HashSet<NodeId>,
    skipped: HashSet<NodeId>,
}

impl Hidden {
    /// Ohne berechnete Stile: versteckt ist, was das `hidden`-Attribut trägt.
    pub(crate) fn by_attribute<D: Document>(doc: &D) -> Self {
        Self::determine(doc, |n| n.has_attr("hidden"))
    }

    /// Mit berechneten Stilen: `display: none` und `visibility: hidden`.
    ///
    /// Das `hidden`-Attribut zählt hier nur, wo der Host keinen Stil kennt —
    /// ein Stylesheet kann es überschreiben, und dann ist das Element da.
    /// `visibility` vererbt sich und kann an einem Nachfahren wieder
    /// `visible` sein; ein solcher Nachfahre fällt hier mit weg. Das ist
    /// selten und die bewusste Grenze dieser Sicht.
    pub(crate) fn by_style<D: Rendering>(doc: &D) -> Self {
        Self::determine(doc, |n| match doc.computed_style(n) {
            Some(_) if invisible_by_ua(n) => n.has_attr("hidden"),
            Some(style) => hidden_per_style(&style),
            None => n.has_attr("hidden"),
        })
    }

    fn determine<'d, D: Document>(doc: &'d D, hidden: impl Fn(D::N<'d>) -> bool) -> Self {
        let root = doc.root();
        let collect_where = |also: &dyn Fn(D::N<'d>) -> bool| {
            let mut away = HashSet::new();
            let mut stack: Vec<D::N<'d>> = root.children().collect();
            while let Some(n) = stack.pop() {
                if n.kind() != NodeKind::Element {
                    continue;
                }
                if hidden(n) || also(n) {
                    away.insert(n.id());
                    continue;
                }
                // Ein geschlossenes `<details>` stellt nur seine erste
                // `<summary>` dar (HTML, „The details element"). Chrome blendet
                // den Rest über `content-visibility` aus, sodass berechnetes
                // `display` und `visibility` ihn für sichtbar halten — deshalb
                // hier aus dem Markup und für jeden Host gleich.
                if n.is_element("details") && !n.has_attr("open") {
                    let mut summary_seen = false;
                    for kind in n.children().filter(|k| k.kind() == NodeKind::Element) {
                        if !summary_seen && kind.is_element("summary") {
                            summary_seen = true;
                            stack.push(kind);
                        } else {
                            away.insert(kind.id());
                        }
                    }
                    continue;
                }
                stack.extend(n.children());
            }
            away
        };
        Hidden {
            not_rendered: collect_where(&|_| false),
            skipped: collect_where(&|n| n.attr("aria-hidden") == Some("true")),
        }
    }

    fn for_scope(&self, scope: Scope) -> Option<&HashSet<NodeId>> {
        match scope {
            Scope::AccessibilityTree => Some(&self.skipped),
            Scope::Rendered => Some(&self.not_rendered),
            Scope::Markup => None,
        }
    }
}

/// Elemente, denen schon das UA-Stylesheet `display: none` gibt, ohne dass
/// sie verborgen wären: `<area>` wirkt über das Bild seiner Imagemap, ein
/// `<audio>` ohne `controls` spielt trotzdem (HTML, „Rendering"). Für sie
/// zählt nur das `hidden`-Attribut — sonst sähen `images/area-alt-missing`
/// und `media/audio-autoplay` mit berechneten Stilen nie etwas
/// (auditmysite-Korpus `misc_content_checks`, `media_and_visual`). Ebenso der
/// Ersatzinhalt eines `<object>`: Lädt das Objekt, stellt der Browser ihn
/// nicht dar, er bleibt aber dessen Textalternative (H53).
fn invisible_by_ua<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("area")
        || (n.is_element("audio") && !n.has_attr("controls"))
        || n.parent().is_some_and(|p| p.is_element("object"))
}

fn hidden_per_style(style: &ComputedStyle) -> bool {
    style.display.as_deref() == Some("none")
        || matches!(style.visibility.as_deref(), Some("hidden" | "collapse"))
}

/// Das Dokument, wie eine Regel mit einem bestimmten [`Scope`] es sieht.
pub(crate) struct Visibility<'d, D> {
    doc: &'d D,
    away: Option<&'d HashSet<NodeId>>,
}

impl<'d, D> Visibility<'d, D> {
    pub(crate) fn new(doc: &'d D, hidden: &'d Hidden, scope: Scope) -> Self {
        Visibility {
            doc,
            away: hidden.for_scope(scope),
        }
    }
}

/// Ein Knoten der [`Visibility`]. Seine Kinder sind die des Hosts, ohne die
/// Wurzeln versteckter Teilbäume.
#[derive(Clone, Copy)]
pub(crate) struct VisibilityNode<'a, N> {
    inner: N,
    away: Option<&'a HashSet<NodeId>>,
}

impl<N: PartialEq> PartialEq for VisibilityNode<'_, N> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<N: Eq> Eq for VisibilityNode<'_, N> {}

impl<'a, N: Node<'a>> VisibilityNode<'a, N> {
    fn wrapper(&self, inner: N) -> Self {
        VisibilityNode {
            inner,
            away: self.away,
        }
    }
}

impl<'a, N: Node<'a>> Node<'a> for VisibilityNode<'a, N> {
    fn id(self) -> NodeId {
        self.inner.id()
    }

    fn kind(self) -> NodeKind {
        self.inner.kind()
    }

    /// Über einem sichtbaren Knoten ist nichts versteckt — Verstecktheit
    /// vererbt sich nach unten.
    fn parent(self) -> Option<Self> {
        self.inner.parent().map(|p| self.wrapper(p))
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let away = self.away;
        self.inner
            .children()
            .filter(move |k| away.is_none_or(|w| !w.contains(&k.id())))
            .map(move |inner| VisibilityNode { inner, away })
    }

    fn local_name(self) -> &'a str {
        self.inner.local_name()
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.inner.attributes()
    }

    fn text(self) -> &'a str {
        self.inner.text()
    }

    fn attr(self, name: &str) -> Option<&'a str> {
        self.inner.attr(name)
    }

    fn tree_scope(self) -> TreeScope {
        self.inner.tree_scope()
    }

    fn document_order(self, other: Self) -> std::cmp::Ordering {
        self.inner.document_order(other.inner)
    }
}

impl<D: Document> Document for Visibility<'_, D> {
    type N<'a>
        = VisibilityNode<'a, D::N<'a>>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        let doc: &D = self.doc;
        VisibilityNode {
            inner: doc.root(),
            away: self.away,
        }
    }

    fn node_count(&self) -> Option<usize> {
        self.doc.node_count()
    }
}

/// Rolle und Name kommen vom Host, über den ganzen Baum: Ein Name darf per
/// `aria-labelledby` auf versteckten Inhalt zeigen.
impl<D: Semantics> Semantics for Visibility<'_, D> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        let doc: &'n D = self.doc;
        doc.role(node.inner)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        let doc: &'n D = self.doc;
        doc.accessible_name(node.inner)
    }

    fn name_source<'n>(&'n self, node: Self::N<'n>) -> Option<NameSource> {
        let doc: &'n D = self.doc;
        doc.name_source(node.inner)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        let doc: &'n D = self.doc;
        doc.is_ignored(node.inner)
    }
}

impl<D: Rendering> Rendering for Visibility<'_, D> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        let doc: &'n D = self.doc;
        doc.computed_style(node.inner)
    }

    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect> {
        let doc: &'n D = self.doc;
        doc.bounds(node.inner)
    }

    fn is_rendered<'n>(&'n self, node: Self::N<'n>) -> bool {
        let doc: &'n D = self.doc;
        doc.is_rendered(node.inner)
    }

    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        let doc: &'n D = self.doc;
        doc.layout(node.inner)
    }

    fn scroll_overflow_px<'n>(&'n self, node: Self::N<'n>) -> Option<f32> {
        let doc: &'n D = self.doc;
        doc.scroll_overflow_px(node.inner)
    }

    fn visually_hidden<'n>(&'n self, node: Self::N<'n>) -> Option<bool> {
        let doc: &'n D = self.doc;
        doc.visually_hidden(node.inner)
    }

    fn sampled_backdrop<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::Backdrop> {
        let doc: &'n D = self.doc;
        doc.sampled_backdrop(node.inner)
    }

    fn frame_reachable<'n>(&'n self, node: Self::N<'n>) -> Option<bool> {
        let doc: &'n D = self.doc;
        doc.frame_reachable(node.inner)
    }

    fn box_transform<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::BoxTransform> {
        let doc: &'n D = self.doc;
        doc.box_transform(node.inner)
    }

    fn paste_probe<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::PasteProbe> {
        let doc: &'n D = self.doc;
        doc.paste_probe(node.inner)
    }

    fn focus_cover<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::FocusCover> {
        let doc: &'n D = self.doc;
        doc.focus_cover(node.inner)
    }

    fn text_spacing_clip<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::SpacingClip> {
        let doc: &'n D = self.doc;
        doc.text_spacing_clip(node.inner)
    }
}

#[cfg(test)]
mod tests {
    //! Die Sicht reicht jede Methode des Hosts durch. Eine Trait-Methode mit
    //! Vorgabe, die hier fehlt, liefert sonst still `None` — so geschehen mit
    //! `scroll_overflow_px` vor 0.20.0. Wer `Rendering` erweitert, ergänzt
    //! diesen Test.

    use super::*;
    use a11y_dom::{Arena, ArenaNode, ComputedStyle, Layout, Rect, Rendering};

    struct Full<'a>(&'a Arena);

    impl Document for Full<'_> {
        type N<'n>
            = ArenaNode<'n>
        where
            Self: 'n;

        fn root(&self) -> Self::N<'_> {
            self.0.root()
        }
    }

    impl Rendering for Full<'_> {
        fn computed_style<'n>(&'n self, _: Self::N<'n>) -> Option<ComputedStyle> {
            Some(ComputedStyle::default())
        }

        fn bounds<'n>(&'n self, _: Self::N<'n>) -> Option<Rect> {
            Some(Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            })
        }

        fn is_rendered<'n>(&'n self, _: Self::N<'n>) -> bool {
            true
        }

        fn layout<'n>(&'n self, _: Self::N<'n>) -> Option<Layout> {
            Some(Layout::default())
        }

        fn scroll_overflow_px<'n>(&'n self, _: Self::N<'n>) -> Option<f32> {
            Some(7.0)
        }

        fn visually_hidden<'n>(&'n self, _: Self::N<'n>) -> Option<bool> {
            Some(true)
        }

        fn sampled_backdrop<'n>(&'n self, _: Self::N<'n>) -> Option<a11y_dom::Backdrop> {
            Some(a11y_dom::Backdrop {
                luminance: vec![0.5],
            })
        }

        fn frame_reachable<'n>(&'n self, _: Self::N<'n>) -> Option<bool> {
            Some(false)
        }

        fn box_transform<'n>(&'n self, _: Self::N<'n>) -> Option<a11y_dom::BoxTransform> {
            Some(a11y_dom::BoxTransform::default())
        }

        fn text_spacing_clip<'n>(&'n self, _: Self::N<'n>) -> Option<a11y_dom::SpacingClip> {
            Some(a11y_dom::SpacingClip {
                before: false,
                after: true,
            })
        }

        fn focus_cover<'n>(&'n self, _: Self::N<'n>) -> Option<a11y_dom::FocusCover> {
            Some(a11y_dom::FocusCover {
                points: 9,
                covered: 3,
                blocker: None,
            })
        }

        fn paste_probe<'n>(&'n self, _: Self::N<'n>) -> Option<a11y_dom::PasteProbe> {
            Some(a11y_dom::PasteProbe {
                cancelled: true,
                inserted: false,
            })
        }
    }

    #[test]
    fn visibility_forwards_every_rendering_method() {
        let arena = Arena::builder().open("html").close().build();
        let host = Full(&arena);
        let v = Hidden::by_attribute(&host);
        let visibility = Visibility::new(&host, &v, Scope::Markup);
        let n = visibility.root();
        assert!(visibility.computed_style(n).is_some());
        assert!(visibility.bounds(n).is_some());
        assert!(visibility.is_rendered(n));
        assert!(visibility.layout(n).is_some());
        assert_eq!(visibility.scroll_overflow_px(n), Some(7.0));
        assert_eq!(visibility.visually_hidden(n), Some(true));
        assert!(visibility.sampled_backdrop(n).is_some());
        assert_eq!(visibility.frame_reachable(n), Some(false));
        assert!(visibility.box_transform(n).is_some());
        assert_eq!(visibility.focus_cover(n).map(|c| c.covered), Some(3));
        assert_eq!(visibility.text_spacing_clip(n).map(|c| c.after), Some(true));
        assert_eq!(
            visibility.paste_probe(n),
            Some(a11y_dom::PasteProbe {
                cancelled: true,
                inserted: false
            })
        );
    }
}
