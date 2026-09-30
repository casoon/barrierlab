//! Der Geltungsbereich einer Regel: welche Knoten sie überhaupt sieht.
//!
//! Ein Element in einem versteckten Teilbaum ist für niemanden da — weder
//! wird es gezeichnet, noch steht es im Accessibility-Tree. Lief eine Regel
//! trotzdem darüber, bekam etwa ein Button im versteckten Banner einen leeren
//! Namen (die Namensberechnung überspringt versteckten Inhalt zu Recht) und
//! damit einen `FAIL`. Gemessen am 29.09.2026 auf barrierlab.eu: vier
//! kritische Befunde auf jeder der 237 Seiten, alle an Versteckten.
//!
//! Die Regeln selbst bleiben davon unberührt. Sie laufen über eine [`Sicht`]
//! auf das Dokument, in der die versteckten Teilbäume schlicht fehlen — so
//! kann keine Regel sie vergessen, und keine muss sie einzeln ausnehmen.

use std::collections::HashSet;

use a11y_dom::{
    ComputedStyle, Document, Layout, NameSource, Node, NodeId, NodeKind, Rect, Rendering, Semantics,
};

/// Welche Knoten eine Regel betrachtet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub(crate) struct Verborgen {
    nicht_dargestellt: HashSet<NodeId>,
    ausgelassen: HashSet<NodeId>,
}

impl Verborgen {
    /// Ohne berechnete Stile: versteckt ist, was das `hidden`-Attribut trägt.
    pub(crate) fn nach_attribut<D: Document>(doc: &D) -> Self {
        Self::bestimme(doc, |n| n.has_attr("hidden"))
    }

    /// Mit berechneten Stilen: `display: none` und `visibility: hidden`.
    ///
    /// Das `hidden`-Attribut zählt hier nur, wo der Host keinen Stil kennt —
    /// ein Stylesheet kann es überschreiben, und dann ist das Element da.
    /// `visibility` vererbt sich und kann an einem Nachfahren wieder
    /// `visible` sein; ein solcher Nachfahre fällt hier mit weg. Das ist
    /// selten und die bewusste Grenze dieser Sicht.
    pub(crate) fn nach_stil<D: Rendering>(doc: &D) -> Self {
        Self::bestimme(doc, |n| match doc.computed_style(n) {
            Some(stil) => versteckt_per_stil(&stil),
            None => n.has_attr("hidden"),
        })
    }

    fn bestimme<'d, D: Document>(doc: &'d D, versteckt: impl Fn(D::N<'d>) -> bool) -> Self {
        let wurzel = doc.root();
        let sammle = |auch: &dyn Fn(D::N<'d>) -> bool| {
            let mut weg = HashSet::new();
            let mut stapel: Vec<D::N<'d>> = wurzel.children().collect();
            while let Some(n) = stapel.pop() {
                if n.kind() != NodeKind::Element {
                    continue;
                }
                if versteckt(n) || auch(n) {
                    weg.insert(n.id());
                    continue;
                }
                stapel.extend(n.children());
            }
            weg
        };
        Verborgen {
            nicht_dargestellt: sammle(&|_| false),
            ausgelassen: sammle(&|n| n.attr("aria-hidden") == Some("true")),
        }
    }

    fn fuer(&self, scope: Scope) -> Option<&HashSet<NodeId>> {
        match scope {
            Scope::AccessibilityTree => Some(&self.ausgelassen),
            Scope::Rendered => Some(&self.nicht_dargestellt),
            Scope::Markup => None,
        }
    }
}

fn versteckt_per_stil(stil: &ComputedStyle) -> bool {
    stil.display.as_deref() == Some("none")
        || matches!(stil.visibility.as_deref(), Some("hidden" | "collapse"))
}

/// Das Dokument, wie eine Regel mit einem bestimmten [`Scope`] es sieht.
pub(crate) struct Sicht<'d, D> {
    doc: &'d D,
    weg: Option<&'d HashSet<NodeId>>,
}

impl<'d, D> Sicht<'d, D> {
    pub(crate) fn new(doc: &'d D, verborgen: &'d Verborgen, scope: Scope) -> Self {
        Sicht {
            doc,
            weg: verborgen.fuer(scope),
        }
    }
}

/// Ein Knoten der [`Sicht`]. Seine Kinder sind die des Hosts, ohne die
/// Wurzeln versteckter Teilbäume.
#[derive(Clone, Copy)]
pub(crate) struct SichtNode<'a, N> {
    inner: N,
    weg: Option<&'a HashSet<NodeId>>,
}

impl<N: PartialEq> PartialEq for SichtNode<'_, N> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<N: Eq> Eq for SichtNode<'_, N> {}

impl<'a, N: Node<'a>> SichtNode<'a, N> {
    fn huelle(&self, inner: N) -> Self {
        SichtNode {
            inner,
            weg: self.weg,
        }
    }
}

impl<'a, N: Node<'a>> Node<'a> for SichtNode<'a, N> {
    fn id(self) -> NodeId {
        self.inner.id()
    }

    fn kind(self) -> NodeKind {
        self.inner.kind()
    }

    /// Über einem sichtbaren Knoten ist nichts versteckt — Verstecktheit
    /// vererbt sich nach unten.
    fn parent(self) -> Option<Self> {
        self.inner.parent().map(|p| self.huelle(p))
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let weg = self.weg;
        self.inner
            .children()
            .filter(move |k| weg.is_none_or(|w| !w.contains(&k.id())))
            .map(move |inner| SichtNode { inner, weg })
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

    fn document_order(self, other: Self) -> std::cmp::Ordering {
        self.inner.document_order(other.inner)
    }
}

impl<D: Document> Document for Sicht<'_, D> {
    type N<'a>
        = SichtNode<'a, D::N<'a>>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        let doc: &D = self.doc;
        SichtNode {
            inner: doc.root(),
            weg: self.weg,
        }
    }

    fn node_count(&self) -> Option<usize> {
        self.doc.node_count()
    }
}

/// Rolle und Name kommen vom Host, über den ganzen Baum: Ein Name darf per
/// `aria-labelledby` auf versteckten Inhalt zeigen.
impl<D: Semantics> Semantics for Sicht<'_, D> {
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

impl<D: Rendering> Rendering for Sicht<'_, D> {
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
}
