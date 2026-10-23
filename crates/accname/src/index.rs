//! Der ID-Index.
//!
//! Der Namensalgorithmus löst `aria-labelledby`, `aria-describedby` und
//! `label[for]` über IDs auf. Diesen Index je Knoten neu aufzubauen wäre
//! quadratisch — er wird deshalb einmal je Dokument gebaut und
//! weitergereicht.

use std::collections::HashMap;

use a11y_dom::{Node, NodeKind, ScopedId, scoped_id, self_and_descendants};

/// Nachschlagewerk von ID auf Knoten, plus die `label[for]`-Zuordnung.
///
/// IDs gelten je Baumbereich ([`a11y_dom::TreeScope`]): Ein Verweis trifft
/// nur ein Ziel im Bereich des verweisenden Knotens, und ein `<label for>`
/// nur ein Feld im eigenen Bereich (barrierlab#23).
pub struct IdIndex<'a, N: Node<'a>> {
    nach_id: HashMap<ScopedId<'a>, N>,
    labels: HashMap<ScopedId<'a>, Vec<N>>,
}

impl<'a, N: Node<'a>> IdIndex<'a, N> {
    /// Baut den Index über den Teilbaum ab `root` auf.
    ///
    /// Bei doppelten IDs im selben Bereich gewinnt das erste Vorkommen — so
    /// verhält sich auch `getElementById`.
    pub fn build(root: N) -> Self {
        let mut nach_id: HashMap<ScopedId<'a>, N> = HashMap::new();
        let mut labels: HashMap<ScopedId<'a>, Vec<N>> = HashMap::new();

        for n in self_and_descendants(root) {
            if n.kind() != NodeKind::Element {
                continue;
            }
            if let Some(id) = n.attr("id") {
                if !id.is_empty() {
                    nach_id.entry(scoped_id(n, id)).or_insert(n);
                }
            }
            if n.is_element("label") {
                if let Some(target) = n.attr("for") {
                    if !target.is_empty() {
                        labels.entry(scoped_id(n, target)).or_default().push(n);
                    }
                }
            }
        }

        IdIndex { nach_id, labels }
    }

    /// Das Ziel der ID `id`, gesehen von `from` aus — im Bereich von `from`.
    pub fn get(&self, from: N, id: &str) -> Option<N> {
        self.nach_id
            .get(&ScopedId {
                scope: from.tree_scope(),
                id,
            })
            .copied()
    }

    /// Alle `<label for="…">` im Bereich von `control`, die auf dessen ID
    /// zeigen, in Dokumentreihenfolge.
    pub fn labels_for(&self, control: N) -> &[N] {
        control
            .attr("id")
            .and_then(|id| self.labels.get(&scoped_id(control, id)))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.nach_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nach_id.is_empty()
    }
}
