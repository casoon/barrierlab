use super::*;

/// An entry in the [`ActiveFormattingElements`] list (§13.2.4.3): either a
/// marker, or a formatting element. Stores only the `NodeId`, not a
/// separate copy of the creating token — the token needed to recreate an
/// equivalent element during "reconstruct the active formatting
/// elements" is re-derived from the node's current name/attributes
/// (immutable once parsed), avoiding duplicated data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FormattingEntry {
    Marker,
    Element(NodeId),
}

/// The list of active formatting elements (§13.2.4.3): used to handle
/// mis-nested formatting element tags (`a`/`b`/`i`/... — the adoption
/// agency algorithm, not yet implemented) by "reopening" formatting
/// elements that got implicitly closed.
#[derive(Debug, Default)]
pub(crate) struct ActiveFormattingElements {
    pub(super) entries: Vec<FormattingEntry>,
}

impl ActiveFormattingElements {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Markers are inserted when entering `applet`/`object`/`marquee`/
    /// `template`/`td`/`th`/`caption` elements, to stop formatting from
    /// "leaking" into them — not called by anything yet (none of those
    /// elements' insertion-mode handling is implemented), but part of
    /// the list's real interface.
    pub(crate) fn push_marker(&mut self) {
        self.entries.push(FormattingEntry::Marker);
    }

    /// "Push onto the list of active formatting elements" (§13.2.4.3),
    /// including the Noah's Ark clause: if three elements with the same
    /// tag name, namespace, and attributes already exist after the last
    /// marker (or anywhere, if there is none), the earliest is removed
    /// first.
    pub(crate) fn push(&mut self, document: &Document, element: NodeId) {
        let boundary = self
            .entries
            .iter()
            .rposition(|entry| matches!(entry, FormattingEntry::Marker))
            .map_or(0, |marker_index| marker_index + 1);
        let matching: Vec<usize> = self.entries[boundary..]
            .iter()
            .enumerate()
            .filter_map(|(offset, entry)| match entry {
                FormattingEntry::Element(node)
                    if elements_match_for_noahs_ark(document, *node, element) =>
                {
                    Some(boundary + offset)
                }
                _ => None,
            })
            .collect();
        if matching.len() >= 3 {
            self.entries.remove(matching[0]);
        }
        self.entries.push(FormattingEntry::Element(element));
    }

    /// "Clear the list of active formatting elements up to the last
    /// marker" (§13.2.4.3) — not called by anything yet (nothing that
    /// would trigger it, e.g. table-related insertion modes, is
    /// implemented), but part of the list's real interface.
    pub(crate) fn clear_up_to_last_marker(&mut self) {
        while let Some(entry) = self.entries.pop() {
            if matches!(entry, FormattingEntry::Marker) {
                break;
            }
        }
    }
}

/// §13.2.4.3's "two elements have the same attributes if all their parsed
/// attributes can be paired such that the two attributes in each pair
/// have identical names, namespaces, and values (the order of the
/// attributes does not matter)" — plus the tag name/namespace match the
/// same clause requires.
pub(super) fn elements_match_for_noahs_ark(document: &Document, a: NodeId, b: NodeId) -> bool {
    let NodeKind::Element {
        name: name_a,
        namespace: namespace_a,
        attributes: attributes_a,
    } = &document.node(a).kind
    else {
        return false;
    };
    let NodeKind::Element {
        name: name_b,
        namespace: namespace_b,
        attributes: attributes_b,
    } = &document.node(b).kind
    else {
        return false;
    };
    if name_a != name_b || namespace_a != namespace_b || attributes_a.len() != attributes_b.len() {
        return false;
    }
    let mut sorted_a: Vec<_> = attributes_a
        .iter()
        .map(|attribute| (&attribute.name, &attribute.namespace, &attribute.value))
        .collect();
    let mut sorted_b: Vec<_> = attributes_b
        .iter()
        .map(|attribute| (&attribute.name, &attribute.namespace, &attribute.value))
        .collect();
    sorted_a.sort();
    sorted_b.sort();
    sorted_a == sorted_b
}

impl TreeBuilder {
    /// "Reconstruct the active formatting elements" (§13.2.4.3): reopens
    /// whichever formatting elements (`a`/`b`/`i`/...) were implicitly
    /// closed since they were last active, by recreating each as a fresh
    /// element via [`insert_html_element`](Self::insert_html_element) —
    /// which is why this had to wait until that existed (see
    /// plan/03-tree-construction.md's reordering note). Recreated
    /// elements get `position: None`: they aren't literally parsed at
    /// this point in the source, this crate's own extension over the
    /// spec (which has no position concept at all) treats them like any
    /// other synthesized node.
    pub(super) fn reconstruct_the_active_formatting_elements(&mut self) {
        let Some(&last) = self.active_formatting_elements.entries.last() else {
            return;
        };
        let is_already_active = match last {
            FormattingEntry::Marker => true,
            FormattingEntry::Element(node) => self.open_elements.contains(node),
        };
        if is_already_active {
            return;
        }

        let last_index = self.active_formatting_elements.entries.len() - 1;
        let mut index = last_index;
        let create_index = loop {
            if index == 0 {
                break 0;
            }
            index -= 1;
            let is_stop = match self.active_formatting_elements.entries[index] {
                FormattingEntry::Marker => true,
                FormattingEntry::Element(node) => self.open_elements.contains(node),
            };
            if is_stop {
                break index + 1;
            }
        };

        let mut index = create_index;
        loop {
            let FormattingEntry::Element(old_node) = self.active_formatting_elements.entries[index]
            else {
                unreachable!(
                    "rewinding stops at a marker or in-stack element, so the Create step's \
                     entry (one position later) is always a formatting element"
                )
            };
            let tag = self.tag_token_for(old_node);
            let new_node = self.insert_html_element(&tag, None);
            self.active_formatting_elements.entries[index] = FormattingEntry::Element(new_node);
            if index == last_index {
                break;
            }
            index += 1;
        }
    }

    /// Removes `node`'s entry from the list of active formatting
    /// elements, if it has one.
    pub(super) fn remove_node_from_active_formatting_elements(&mut self, node: NodeId) {
        if let Some(index) = self.active_formatting_elements.entries.iter().position(
            |entry| matches!(entry, FormattingEntry::Element(afe_node) if *afe_node == node),
        ) {
            self.active_formatting_elements.entries.remove(index);
        }
    }
}
