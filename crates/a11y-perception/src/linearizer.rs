use std::collections::HashSet;

use crate::tree::{AXNode, AXTree, AXValue};

use crate::reading::{IgnoredReadingNode, ReadingItem};

/// Build the default screen reader reading order from an AXTree.
///
/// Ignored nodes and layout-only nodes (see `is_layout_only_role`) are
/// traversed for their visible descendants, but are not emitted as reading
/// items.
pub fn linearize(tree: &AXTree) -> Vec<ReadingItem> {
    linearize_with_ignored(tree).items
}

/// Build reading order and retain ignored nodes for diagnostics.
pub fn linearize_with_ignored(tree: &AXTree) -> LinearizedReadingOrder {
    let mut order = LinearizedReadingOrder::default();
    let Some(root_id) = tree.root_id.as_deref() else {
        return order;
    };

    let mut visited = HashSet::new();
    visit_node(tree, root_id, 0, &mut visited, &mut order);
    order
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinearizedReadingOrder {
    pub items: Vec<ReadingItem>,
    pub ignored: Vec<IgnoredReadingNode>,
}

fn visit_node(
    tree: &AXTree,
    node_id: &str,
    depth: usize,
    visited: &mut HashSet<String>,
    order: &mut LinearizedReadingOrder,
) {
    if !visited.insert(node_id.to_string()) {
        return;
    }

    let Some(node) = tree.get_node(node_id) else {
        return;
    };

    let child_depth = if node.ignored {
        order.ignored.push(ignored_node(node, depth));
        depth
    } else if is_layout_only_role(node.role.as_deref()) {
        // Layout-only node: traversed for descendants, never emitted.
        depth
    } else {
        order
            .items
            .push(reading_item(node, order.items.len(), depth));
        depth + 1
    };

    for child_id in &node.child_ids {
        visit_node(tree, child_id, child_depth, visited, order);
    }
}

/// Blink layout artifacts that Chrome exposes in the AXTree but never hands to
/// an assistive technology. `InlineTextBox` nodes are the per-line fragments a
/// `StaticText` is broken into by line wrapping — the same text a second (and
/// third, and fourth) time, once per rendered line. They exist so the AX API can
/// answer character-level bounds queries (caret, text selection), not so anything
/// announces them.
///
/// Emitting them inflated every count derived from the reading order. Confirmed
/// live on www.sachsen-anhalt.de (2026-09-17): 67 of 237 reading items were
/// `InlineTextBox`, and all four "long section without a landmark, heading or
/// focus target" findings on that page were produced purely by line wrapping —
/// the worst segment reported 23 entries where a screen reader announces 4.
fn is_layout_only_role(role: Option<&str>) -> bool {
    matches!(role, Some("InlineTextBox"))
}

fn reading_item(node: &AXNode, seq: usize, depth: usize) -> ReadingItem {
    ReadingItem {
        seq,
        role: node.role.clone(),
        name: node.name.clone(),
        description: node.description.clone(),
        value: node.value.clone(),
        states: states(node),
        tab_stop: node.is_focusable(),
        depth,
        node_id: node.node_id.clone(),
    }
}

fn ignored_node(node: &AXNode, depth: usize) -> IgnoredReadingNode {
    IgnoredReadingNode {
        node_id: node.node_id.clone(),
        role: node.role.clone(),
        name: node.name.clone(),
        depth,
        reasons: node
            .ignored_reasons
            .iter()
            .map(|property| match property.value.as_str() {
                Some(value) if !value.trim().is_empty() => {
                    format!("{}={}", property.name, value)
                }
                _ => property.name.clone(),
            })
            .collect(),
    }
}

fn states(node: &AXNode) -> Vec<String> {
    node.properties
        .iter()
        .filter_map(|property| match &property.value {
            AXValue::Bool(value) if is_state_property(&property.name) => {
                if *value {
                    Some(property.name.clone())
                } else {
                    Some(format!("{}=false", property.name))
                }
            }
            AXValue::String(value) if is_state_property(&property.name) && !value.is_empty() => {
                Some(format!("{}={}", property.name, value))
            }
            AXValue::Int(value) if is_state_property(&property.name) => {
                Some(format!("{}={}", property.name, value))
            }
            _ => None,
        })
        .collect()
}

fn is_state_property(name: &str) -> bool {
    matches!(
        name,
        "expanded"
            | "checked"
            | "selected"
            | "required"
            | "invalid"
            | "disabled"
            | "pressed"
            | "level"
            | "autocomplete"
            | "haspopup"
    )
}

#[cfg(test)]
mod tests {
    use crate::tree::{AXNode, AXProperty, AXTree, AXValue};

    use super::{linearize, linearize_with_ignored};

    fn node(id: &str, role: &str, name: Option<&str>, child_ids: Vec<&str>) -> AXNode {
        AXNode {
            node_id: id.to_string(),
            ignored: false,
            ignored_reasons: vec![],
            role: Some(role.to_string()),
            name: name.map(String::from),
            name_source: None,
            description: None,
            value: None,
            properties: vec![],
            child_ids: child_ids.into_iter().map(String::from).collect(),
            parent_id: None,
            backend_dom_node_id: None,
        }
    }

    fn property(name: &str, value: AXValue) -> AXProperty {
        AXProperty {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn linearize_uses_dfs_order_and_skips_ignored_nodes() {
        let mut ignored = node("2", "generic", None, vec!["3"]);
        ignored.ignored = true;
        ignored.ignored_reasons = vec![property("ariaHidden", AXValue::Bool(true))];

        let mut heading = node("3", "heading", Some("Willkommen"), vec![]);
        heading.properties = vec![property("level", AXValue::Int(1))];

        let mut button = node("4", "button", Some("Mehr erfahren"), vec![]);
        button.properties = vec![
            property("focusable", AXValue::Bool(true)),
            property("expanded", AXValue::Bool(false)),
            property("required", AXValue::Bool(true)),
        ];

        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", Some("Example"), vec!["2", "4"]),
            ignored,
            heading,
            button,
        ]);

        let items = linearize(&tree);

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].node_id, "1");
        assert_eq!(items[0].depth, 0);
        assert_eq!(items[0].seq, 0);
        assert_eq!(items[1].node_id, "3");
        assert_eq!(items[1].depth, 1);
        assert_eq!(items[1].states, vec!["level=1"]);
        assert_eq!(items[2].node_id, "4");
        assert_eq!(items[2].depth, 1);
        assert_eq!(items[2].seq, 2);
        assert!(items[2].tab_stop);
        assert_eq!(items[2].states, vec!["expanded=false", "required"]);
    }

    #[test]
    fn linearize_with_ignored_retains_diagnostics() {
        let mut ignored = node("2", "none", None, vec![]);
        ignored.ignored = true;
        ignored.ignored_reasons = vec![property("presentational", AXValue::String("true".into()))];

        let tree = AXTree::from_nodes(vec![node("1", "WebArea", None, vec!["2"]), ignored]);
        let order = linearize_with_ignored(&tree);

        assert_eq!(order.items.len(), 1);
        assert_eq!(order.ignored.len(), 1);
        assert_eq!(order.ignored[0].node_id, "2");
        assert_eq!(order.ignored[0].depth, 1);
        assert_eq!(order.ignored[0].reasons, vec!["presentational=true"]);
    }

    #[test]
    fn linearize_drops_inline_text_boxes_but_keeps_their_static_text() {
        // Chrome splits a wrapped `StaticText` into one `InlineTextBox` per
        // rendered line. Those are layout artifacts, never announcements, and
        // emitting them inflated every count derived from the reading order.
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2"]),
            node(
                "2",
                "StaticText",
                Some("Ein langer Satz uber zwei Zeilen"),
                vec!["3", "4"],
            ),
            node("3", "InlineTextBox", Some("Ein langer Satz"), vec![]),
            node("4", "InlineTextBox", Some("uber zwei Zeilen"), vec![]),
        ]);

        let items = linearize(&tree);

        assert_eq!(items.len(), 2);
        assert_eq!(items[1].role.as_deref(), Some("StaticText"));
        assert!(
            items
                .iter()
                .all(|item| item.role.as_deref() != Some("InlineTextBox"))
        );
    }

    #[test]
    fn linearize_still_traverses_descendants_of_a_layout_node() {
        // A layout node is skipped, not pruned -- anything below it must still
        // reach the reading order, at the depth the layout node would have had.
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2"]),
            node("2", "InlineTextBox", Some("Zeile"), vec!["3"]),
            node("3", "link", Some("Weiterlesen"), vec![]),
        ]);

        let items = linearize(&tree);

        assert_eq!(items.len(), 2);
        assert_eq!(items[1].node_id, "3");
        assert_eq!(items[1].depth, 1);
    }

    #[test]
    fn linearize_returns_empty_for_empty_tree() {
        assert!(linearize(&AXTree::new()).is_empty());
    }

    // --- Mutationskatalog tests/mutations/linearizer.toml (Refs #257) ---

    #[test]
    fn items_carry_seq_depth_description_value_and_tab_stop() {
        let mut field = node("2", "textbox", Some("Name"), vec![]);
        field.description = Some("Pflichtfeld".into());
        field.value = Some("Ada".into());
        field.properties = vec![property("focusable", AXValue::Bool(true))];
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2", "3"]),
            field,
            node("3", "paragraph", None, vec![]),
        ]);

        let items = linearize(&tree);

        assert_eq!(items.len(), 3);
        assert_eq!(
            items.iter().map(|i| (i.seq, i.depth)).collect::<Vec<_>>(),
            vec![(0, 0), (1, 1), (2, 1)]
        );
        assert_eq!(items[1].description.as_deref(), Some("Pflichtfeld"));
        assert_eq!(items[1].value.as_deref(), Some("Ada"));
        assert!(items[1].tab_stop && !items[2].tab_stop && !items[0].tab_stop);
    }

    #[test]
    fn depth_grows_only_below_emitted_items() {
        // item > ignored > layout > item: weder der ignorierte noch der
        // Layout-Knoten verschiebt die Tiefe.
        let mut hidden = node("2", "none", None, vec!["3"]);
        hidden.ignored = true;
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2"]),
            hidden,
            node("3", "InlineTextBox", None, vec!["4"]),
            node("4", "link", Some("Tief"), vec!["5"]),
            node("5", "image", Some("Bild"), vec![]),
        ]);

        let result = linearize_with_ignored(&tree);

        assert_eq!(
            result
                .items
                .iter()
                .map(|i| (i.node_id.as_str(), i.depth))
                .collect::<Vec<_>>(),
            vec![("1", 0), ("4", 1), ("5", 2)]
        );
        assert_eq!(result.ignored[0].depth, 1);
    }

    #[test]
    fn an_ignored_layout_role_is_reported_as_ignored() {
        let mut hidden = node("2", "InlineTextBox", Some("x"), vec![]);
        hidden.ignored = true;
        let tree = AXTree::from_nodes(vec![node("1", "WebArea", None, vec!["2"]), hidden]);
        let result = linearize_with_ignored(&tree);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.ignored.len(), 1);
    }

    #[test]
    fn only_inline_text_boxes_are_layout_only() {
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2", "3"]),
            node("2", "LineBreak", None, vec![]),
            node("3", "StaticText", Some("Text"), vec![]),
        ]);
        let ids: Vec<_> = linearize(&tree).into_iter().map(|i| i.node_id).collect();
        assert_eq!(ids, vec!["1", "2", "3"]);
    }

    #[test]
    fn children_are_visited_in_child_id_order() {
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["c", "a", "b"]),
            node("a", "heading", Some("A"), vec![]),
            node("b", "heading", Some("B"), vec![]),
            node("c", "heading", Some("C"), vec![]),
        ]);
        let ids: Vec<_> = linearize(&tree).into_iter().map(|i| i.node_id).collect();
        assert_eq!(ids, vec!["1", "c", "a", "b"]);
    }

    #[test]
    fn a_shared_or_cyclic_child_is_visited_once() {
        let tree = AXTree::from_nodes(vec![
            node("1", "WebArea", None, vec!["2", "2"]),
            node("2", "group", None, vec!["1"]),
        ]);
        assert_eq!(linearize(&tree).len(), 2);
    }

    #[test]
    fn ignored_reasons_are_name_equals_value_or_bare_name() {
        let mut hidden = node("2", "none", None, vec![]);
        hidden.ignored = true;
        hidden.ignored_reasons = vec![
            property("notRendered", AXValue::Bool(true)),
            property("ariaHiddenElement", AXValue::String("true".into())),
            property("blank", AXValue::String("  ".into())),
            property("empty", AXValue::String(String::new())),
        ];
        let tree = AXTree::from_nodes(vec![node("1", "WebArea", None, vec!["2"]), hidden]);
        let result = linearize_with_ignored(&tree);
        assert_eq!(
            result.ignored[0].reasons,
            vec!["notRendered", "ariaHiddenElement=true", "blank", "empty"]
        );
    }

    #[test]
    fn states_use_name_value_forms_in_node_order() {
        let mut n = node("2", "checkbox", Some("AGB"), vec![]);
        n.properties = vec![
            property("required", AXValue::Bool(true)),
            property("checked", AXValue::String("mixed".into())),
            property("disabled", AXValue::Bool(false)),
            property("level", AXValue::Int(3)),
            property("invalid", AXValue::String(String::new())),
            property("posinset", AXValue::Int(4)),
            property("focusable", AXValue::Bool(true)),
            property("haspopup", AXValue::String("menu".into())),
        ];
        let tree = AXTree::from_nodes(vec![node("1", "WebArea", None, vec!["2"]), n]);
        assert_eq!(
            linearize(&tree)[1].states,
            vec![
                "required",
                "checked=mixed",
                "disabled=false",
                "level=3",
                "haspopup=menu"
            ]
        );
    }

    #[test]
    fn every_state_property_is_listed() {
        let names = [
            "expanded",
            "checked",
            "selected",
            "required",
            "invalid",
            "disabled",
            "pressed",
            "level",
            "autocomplete",
            "haspopup",
        ];
        for name in names {
            let mut n = node("2", "generic", None, vec![]);
            n.properties = vec![property(name, AXValue::Bool(true))];
            let tree = AXTree::from_nodes(vec![node("1", "WebArea", None, vec!["2"]), n]);
            assert_eq!(linearize(&tree)[1].states, vec![name], "{name}");
        }
    }
}
