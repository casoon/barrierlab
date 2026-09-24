//! Der ID-Index.
//!
//! Der Namensalgorithmus löst `aria-labelledby`, `aria-describedby` und
//! `label[for]` über IDs auf. Diesen Index je Knoten neu aufzubauen wäre
//! quadratisch — er wird deshalb einmal je Dokument gebaut und
//! weitergereicht.

use std::collections::HashMap;

use a11y_dom::{Node, NodeKind, self_and_descendants};

/// Nachschlagewerk von ID auf Knoten, plus die `label[for]`-Zuordnung.
pub struct IdIndex<'a, N: Node<'a>> {
    nach_id: HashMap<&'a str, N>,
    labels: HashMap<&'a str, Vec<N>>,
}

impl<'a, N: Node<'a>> IdIndex<'a, N> {
    /// Baut den Index über den Teilbaum ab `root` auf.
    ///
    /// Bei doppelten IDs gewinnt das erste Vorkommen — so verhält sich auch
    /// `getElementById`.
    pub fn build(root: N) -> Self {
        let mut nach_id: HashMap<&'a str, N> = HashMap::new();
        let mut labels: HashMap<&'a str, Vec<N>> = HashMap::new();

        for n in self_and_descendants(root) {
            if n.kind() != NodeKind::Element {
                continue;
            }
            if let Some(id) = n.attr("id") {
                if !id.is_empty() {
                    nach_id.entry(id).or_insert(n);
                }
            }
            if n.is_element("label") {
                if let Some(target) = n.attr("for") {
                    if !target.is_empty() {
                        labels.entry(target).or_default().push(n);
                    }
                }
            }
        }

        IdIndex { nach_id, labels }
    }

    pub fn get(&self, id: &str) -> Option<N> {
        self.nach_id.get(id).copied()
    }

    /// Alle `<label for="…">`, die auf diese ID zeigen, in Dokumentreihenfolge.
    pub fn labels_for(&self, id: &str) -> &[N] {
        self.labels.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.nach_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nach_id.is_empty()
    }
}
