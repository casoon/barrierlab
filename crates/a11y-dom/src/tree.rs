//! Der Basisbaum. Das, was alle drei Oberflächen liefern können.

use std::cmp::Ordering;

/// Stabile Kennung eines Knotens innerhalb eines Dokuments. Der Host bestimmt,
/// was sie bedeutet — Arena-Index, AXTree-Backend-ID, laufende Nummer beim
/// Parsen. Für die Regeln ist sie nur ein Rückbezug für den Befund.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Welche Art von Knoten. Kommentare, Processing Instructions und Doctype
/// kommen nicht vor — für Accessibility-Regeln sind sie ohne Bedeutung, und
/// sie wegzulassen spart bei großen Dokumenten spürbar Arbeit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Element,
    Text,
}

/// Ein Knoten im Baum des Hosts.
///
/// `Copy`, weil die Traversierungs-Iteratoren viele kurzlebige Handles
/// erzeugen — modelliert nach `roxmltree::Node`, nicht nach einem
/// Arena-Index-Schema. Die Methoden nehmen `self` per Wert, damit die
/// zurückgegebenen Iteratoren nur `'a` einfangen und keine zusätzliche
/// Ausleihe eines lokal gehaltenen Handles.
///
/// Das Trait ist bewusst **DOM-förmig**, nicht Accessibility-Tree-förmig: Die
/// Mehrzahl der Regeln braucht Tags und Attribute (`tabindex`, `id`, `role`,
/// `alt`), und der native Accessibility-Tree gibt die gar nicht her. Rolle und
/// Accessible Name kommen stattdessen als eigene Fähigkeit obendrauf — siehe
/// [`crate::Semantics`].
pub trait Node<'a>: Copy + Eq + 'a {
    fn id(self) -> NodeId;
    fn kind(self) -> NodeKind;
    fn parent(self) -> Option<Self>;
    fn children(self) -> impl Iterator<Item = Self> + 'a;

    /// Kleingeschriebener Tagname ohne Namensraum-Präfix. Leer für Textknoten.
    fn local_name(self) -> &'a str;

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a;

    /// Der eigene Text dieses Knotens. Leer für Elemente — deren Text steckt in
    /// ihren Textkindern, siehe [`subtree_text`].
    fn text(self) -> &'a str;

    /// Wert eines Attributs. Der Vorgabepfad geht linear über
    /// [`Node::attributes`]; Hosts mit einer Hashmap überschreiben ihn.
    fn attr(self, name: &str) -> Option<&'a str> {
        self.attributes().find(|(k, _)| *k == name).map(|(_, v)| v)
    }

    fn has_attr(self, name: &str) -> bool {
        self.attr(name).is_some()
    }

    fn is_element(self, local_name: &str) -> bool {
        self.kind() == NodeKind::Element && self.local_name() == local_name
    }

    /// Dokumentreihenfolge. Wird für Regeln gebraucht, die auf Abfolge prüfen —
    /// etwa übersprungene Überschriftenebenen.
    fn document_order(self, other: Self) -> Ordering {
        self.id().cmp(&other.id())
    }
}

/// Ein Dokument: gibt Zugriff auf die Wurzel.
pub trait Document {
    type N<'a>: Node<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_>;

    /// Anzahl der Knoten, wenn der Host sie billig kennt. Nur für Berichte und
    /// Vorabdimensionierung, nie für Korrektheit.
    fn node_count(&self) -> Option<usize> {
        None
    }
}

/// Alle Nachfahren in Dokumentreihenfolge, ohne den Knoten selbst.
pub fn descendants<'a, N: Node<'a>>(node: N) -> impl Iterator<Item = N> + 'a {
    let mut stack: Vec<N> = node.children().collect();
    stack.reverse();
    std::iter::from_fn(move || {
        let n = stack.pop()?;
        let mut kids: Vec<N> = n.children().collect();
        kids.reverse();
        stack.extend(kids);
        Some(n)
    })
}

/// Der Knoten selbst und alle Nachfahren, in Dokumentreihenfolge.
pub fn self_and_descendants<'a, N: Node<'a>>(node: N) -> impl Iterator<Item = N> + 'a {
    std::iter::once(node).chain(descendants(node))
}

/// Alle Vorfahren, vom Elternknoten aufwärts.
pub fn ancestors<'a, N: Node<'a>>(node: N) -> impl Iterator<Item = N> + 'a {
    let mut cur = node.parent();
    std::iter::from_fn(move || {
        let n = cur?;
        cur = n.parent();
        Some(n)
    })
}

/// Der nächste Vorfahre mit diesem Tagnamen, den Knoten selbst eingeschlossen.
pub fn closest<'a, N: Node<'a>>(node: N, local_name: &str) -> Option<N> {
    std::iter::once(node)
        .chain(ancestors(node))
        .find(|n| n.is_element(local_name))
}

/// Der zusammengesetzte Text des gesamten Teilbaums.
///
/// Grundlage für den Accessible-Name-Ersatz, solange ein Host keine
/// [`crate::Semantics`] liefert.
pub fn subtree_text<'a, N: Node<'a>>(node: N) -> String {
    let mut out = String::new();
    for n in self_and_descendants(node) {
        if n.kind() == NodeKind::Text {
            out.push_str(n.text());
        }
    }
    out
}

/// Ob der Teilbaum sichtbaren Text enthält — billiger als [`subtree_text`],
/// weil beim ersten Treffer abgebrochen wird.
pub fn has_text<'a, N: Node<'a>>(node: N) -> bool {
    self_and_descendants(node).any(|n| n.kind() == NodeKind::Text && !n.text().trim().is_empty())
}

/// Alle Elemente des Dokuments in Dokumentreihenfolge.
pub fn elements<'a, D: Document>(doc: &'a D) -> impl Iterator<Item = D::N<'a>> + 'a {
    self_and_descendants(doc.root()).filter(|n| n.kind() == NodeKind::Element)
}
