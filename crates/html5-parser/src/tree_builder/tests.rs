#[cfg(test)]
mod open_elements_tests {
    use crate::document::{Document, NodeKind};
    use crate::tree_builder::{HTML_NAMESPACE, OpenElementsStack};

    /// Builds a stack containing one HTML-namespace element per given
    /// name, pushed in order (so the last name is the current node).
    fn stack_of(document: &mut Document, names: &[&str]) -> OpenElementsStack {
        let mut stack = OpenElementsStack::new();
        for &name in names {
            let node = document.new_node(
                NodeKind::Element {
                    name: name.to_owned(),
                    namespace: Some(HTML_NAMESPACE.to_owned()),
                    attributes: vec![],
                },
                None,
            );
            stack.push(node);
        }
        stack
    }

    #[test]
    fn empty_stack_has_no_current_node() {
        let stack = OpenElementsStack::new();
        assert_eq!(stack.current_node(), None);
    }

    #[test]
    fn push_and_pop_and_current_node() {
        let mut document = Document::new();
        let mut stack = stack_of(&mut document, &["html", "body"]);
        let body = stack.current_node().unwrap();
        assert_eq!(
            document.node(body).kind,
            NodeKind::Element {
                name: "body".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        stack.pop();
        let html = stack.current_node().unwrap();
        assert_eq!(
            document.node(html).kind,
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn has_element_in_scope_finds_a_matching_ancestor() {
        let mut document = Document::new();
        let stack = stack_of(&mut document, &["html", "body", "div", "p"]);
        assert!(stack.has_element_in_scope(&document, "div"));
        assert!(stack.has_element_in_scope(&document, "p"));
        assert!(!stack.has_element_in_scope(&document, "span"));
    }

    #[test]
    fn has_element_in_scope_stops_at_a_boundary_element() {
        // A `p` above a `table` is out of scope from inside the table,
        // per the default scope's boundary list including `table` — and
        // `td` itself is *also* a boundary element, so the search for
        // `table` from inside `td` stops at `td` before ever reaching it.
        let mut document = Document::new();
        let stack = stack_of(&mut document, &["html", "body", "p", "table", "tr", "td"]);
        assert!(!stack.has_element_in_scope(&document, "p"));
        assert!(!stack.has_element_in_scope(&document, "table"));
        // `td` matches immediately since it's the current node itself —
        // a node matching the target is checked *before* the boundary
        // check, per spec step order.
        assert!(stack.has_element_in_scope(&document, "td"));
    }

    #[test]
    fn has_element_in_list_item_scope_stops_at_ul_but_default_scope_does_not() {
        // `body` sits above a `ul`; list item scope's boundary list
        // includes `ul`/`ol` (on top of the default scope list), default
        // scope's does not.
        let mut document = Document::new();
        let stack = stack_of(&mut document, &["html", "body", "ul", "li", "span"]);
        assert!(!stack.has_element_in_list_item_scope(&document, "body"));
        assert!(stack.has_element_in_scope(&document, "body"));
    }

    #[test]
    fn has_element_in_button_scope_stops_at_button() {
        let mut document = Document::new();
        let stack = stack_of(&mut document, &["html", "body", "p", "button"]);
        assert!(!stack.has_element_in_button_scope(&document, "p"));
        assert!(stack.has_element_in_scope(&document, "p"));
    }

    #[test]
    fn has_element_in_table_scope_uses_its_own_short_boundary_list() {
        // `td` is a boundary element for default scope but *not* for
        // table scope (table scope's boundary list is just
        // html/table/template) — so table scope can see past it to the
        // enclosing `table`, while default scope cannot.
        let mut document = Document::new();
        let stack = stack_of(&mut document, &["html", "table", "td", "div"]);
        assert!(stack.has_element_in_table_scope(&document, "table"));
        assert!(!stack.has_element_in_scope(&document, "table"));
    }

    #[test]
    fn generate_implied_end_tags_pops_while_current_node_is_in_the_list() {
        let mut document = Document::new();
        let mut stack = stack_of(&mut document, &["html", "body", "ul", "li", "p"]);
        stack.generate_implied_end_tags(&document, None);
        // `p` and `li` are both in the list, `ul` is not: popping stops
        // there and `ul` remains the current node.
        let current = stack.current_node().unwrap();
        assert_eq!(
            document.node(current).kind,
            NodeKind::Element {
                name: "ul".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn generate_implied_end_tags_excludes_the_given_element() {
        let mut document = Document::new();
        let mut stack = stack_of(&mut document, &["html", "body", "dl", "dt", "li"]);
        // `li` is in the list but excluded, so popping stops immediately
        // without popping anything.
        stack.generate_implied_end_tags(&document, Some("li"));
        let current = stack.current_node().unwrap();
        assert_eq!(
            document.node(current).kind,
            NodeKind::Element {
                name: "li".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn generate_implied_end_tags_thoroughly_pops_table_elements_too() {
        let mut document = Document::new();
        let mut stack = stack_of(&mut document, &["html", "table", "tbody", "tr", "td"]);
        // `td` is only in the "thoroughly" list, not the plain list.
        stack.generate_implied_end_tags(&document, None);
        assert_eq!(stack.entries.len(), 5);
        stack.generate_implied_end_tags_thoroughly(&document);
        let current = stack.current_node().unwrap();
        assert_eq!(
            document.node(current).kind,
            NodeKind::Element {
                name: "table".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }
}

#[cfg(test)]
mod quirks_mode_tests {
    use crate::tokenizer::DoctypeToken;
    use crate::tree_builder::{QuirksMode, determine_quirks_mode};

    fn doctype(
        name: Option<&str>,
        public_identifier: Option<&str>,
        system_identifier: Option<&str>,
        force_quirks: bool,
    ) -> DoctypeToken {
        DoctypeToken {
            name: name.map(str::to_owned),
            public_identifier: public_identifier.map(str::to_owned),
            system_identifier: system_identifier.map(str::to_owned),
            force_quirks,
        }
    }

    #[test]
    fn html5_doctype_is_no_quirks() {
        let token = doctype(Some("html"), None, None, false);
        assert_eq!(determine_quirks_mode(&token), QuirksMode::NoQuirks);
    }

    #[test]
    fn force_quirks_flag_wins_regardless_of_everything_else() {
        let token = doctype(Some("html"), None, None, true);
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }

    #[test]
    fn missing_doctype_name_is_quirks() {
        let token = doctype(None, None, None, false);
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }

    #[test]
    fn exact_public_identifier_match_is_quirks_case_insensitively() {
        let token = doctype(Some("html"), Some("html"), None, false);
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }

    #[test]
    fn html4_transitional_without_system_identifier_is_quirks() {
        let token = doctype(
            Some("html"),
            Some("-//W3C//DTD HTML 4.01 Transitional//EN"),
            None,
            false,
        );
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }

    #[test]
    fn html4_transitional_with_system_identifier_is_limited_quirks() {
        let token = doctype(
            Some("html"),
            Some("-//W3C//DTD HTML 4.01 Transitional//EN"),
            Some("http://www.w3.org/TR/html4/loose.dtd"),
            false,
        );
        assert_eq!(determine_quirks_mode(&token), QuirksMode::LimitedQuirks);
    }

    #[test]
    fn xhtml_transitional_is_always_limited_quirks() {
        let token = doctype(
            Some("html"),
            Some("-//W3C//DTD XHTML 1.0 Transitional//EN"),
            Some("http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"),
            false,
        );
        assert_eq!(determine_quirks_mode(&token), QuirksMode::LimitedQuirks);
    }

    #[test]
    fn empty_system_identifier_counts_as_missing_for_the_html4_split() {
        let token = doctype(
            Some("html"),
            Some("-//W3C//DTD HTML 4.01 Frameset//EN"),
            Some(""),
            false,
        );
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }

    #[test]
    fn ibm_system_identifier_is_quirks() {
        let token = doctype(
            Some("html"),
            None,
            Some("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd"),
            false,
        );
        assert_eq!(determine_quirks_mode(&token), QuirksMode::Quirks);
    }
}

#[cfg(test)]
mod insertion_algorithm_tests {
    use crate::document::{NodeId, NodeKind};
    use crate::tokenizer::TagToken;
    use crate::tree_builder::{HTML_NAMESPACE, InsertionLocation, TreeBuilder};

    fn tag(name: &str) -> TagToken {
        TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: vec![],
        }
    }

    /// Mirrors what the "before html" insertion mode (not yet
    /// implemented) does for the very
    /// first `<html>` element: append it directly to the Document node
    /// and push it onto the stack, bypassing the generic insertion
    /// algorithm under test here (which requires a current node to
    /// already exist, since "the appropriate place for inserting a node"
    /// defaults its target to the current node).
    fn bootstrap_html(builder: &mut TreeBuilder) -> NodeId {
        let root = builder.document.root();
        let html = builder.document.new_node(
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            },
            None,
        );
        builder.document.append_child(root, html);
        builder.open_elements.push(html);
        html
    }

    fn only_child_kind(builder: &TreeBuilder, parent: NodeId) -> NodeKind {
        let children: Vec<_> = builder.document.children(parent).collect();
        assert_eq!(children.len(), 1, "expected exactly one child");
        builder.document.node(children[0]).kind.clone()
    }

    #[test]
    fn insert_html_element_nests_under_the_current_node() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);

        assert_eq!(
            builder.document.children(html).collect::<Vec<_>>(),
            vec![body]
        );
        assert_eq!(builder.open_elements.current_node(), Some(body));
        assert_eq!(
            builder.document.node(body).kind,
            NodeKind::Element {
                name: "body".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn generic_rcdata_element_parsing_algorithm() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        builder.insertion_mode = crate::tree_builder::InsertionMode::InBody;

        let (title, external_state) = builder.generic_text_element_parsing_algorithm(
            &tag("title"),
            crate::tree_builder::GenericTextElementKind::Rcdata,
            None,
        );

        assert_eq!(external_state, crate::tokenizer::ExternalState::RcData);
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![title]
        );
        assert_eq!(builder.open_elements.current_node(), Some(title));
        assert_eq!(
            builder.original_insertion_mode,
            Some(crate::tree_builder::InsertionMode::InBody)
        );
        assert_eq!(
            builder.insertion_mode,
            crate::tree_builder::InsertionMode::Text
        );
    }

    #[test]
    fn generic_rawtext_element_parsing_algorithm() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("head"), None);
        builder.insertion_mode = crate::tree_builder::InsertionMode::InHead;

        let (style, external_state) = builder.generic_text_element_parsing_algorithm(
            &tag("style"),
            crate::tree_builder::GenericTextElementKind::RawText,
            None,
        );

        assert_eq!(external_state, crate::tokenizer::ExternalState::RawText);
        assert_eq!(builder.open_elements.current_node(), Some(style));
        assert_eq!(
            builder.original_insertion_mode,
            Some(crate::tree_builder::InsertionMode::InHead)
        );
        assert_eq!(
            builder.insertion_mode,
            crate::tree_builder::InsertionMode::Text
        );
    }

    #[test]
    fn a_second_element_is_never_appended_directly_to_the_document_root() {
        // "insert an element at the adjusted insertion location"'s guard:
        // a Document node with an element child already must not get a
        // second one. Exercised directly against the private method,
        // since none of the insertion-mode rules that would ever trigger
        // this (not yet implemented) exist yet — normal calls through
        // `insert_html_element` always target the current node, never the
        // document root itself.
        let mut builder = TreeBuilder::new();
        let root = builder.document.root();
        bootstrap_html(&mut builder);
        let second_html = builder.document.new_node(
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            },
            None,
        );

        builder.insert_element_at_adjusted_insertion_location(
            second_html,
            InsertionLocation {
                parent: root,
                before: None,
            },
        );

        assert_eq!(builder.document.children(root).count(), 1);
    }

    #[test]
    fn insert_character_merges_into_an_adjacent_text_node() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        builder.insert_character('a', None);
        builder.insert_character('b', None);

        assert_eq!(
            only_child_kind(&builder, html),
            NodeKind::Text {
                content: "ab".to_owned()
            }
        );
    }

    #[test]
    fn insert_character_after_an_element_creates_a_new_text_node() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("body"), None);
        builder.open_elements.pop();
        builder.insert_character('x', None);

        let children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(children.len(), 2);
        assert_eq!(
            builder.document.node(children[1]).kind,
            NodeKind::Text {
                content: "x".to_owned()
            }
        );
    }

    #[test]
    fn insert_comment_and_processing_instruction() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        builder.insert_comment("hi".to_owned(), None);
        builder.insert_processing_instruction("target".to_owned(), "data".to_owned(), None);

        let children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Comment {
                content: "hi".to_owned()
            }
        );
        assert_eq!(
            builder.document.node(children[1]).kind,
            NodeKind::ProcessingInstruction {
                target: "target".to_owned(),
                data: "data".to_owned(),
            }
        );
    }

    #[test]
    fn foster_parenting_inserts_before_the_table_not_inside_it() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        let table = builder.insert_html_element(&tag("table"), None);
        builder.foster_parenting = true;
        // A character token processed while the current node is `table`
        // (a foster-parenting target) must land *before* the table, as a
        // sibling under `html` — not as a child of the table.
        builder.insert_character('x', None);

        assert_eq!(builder.document.children(table).count(), 0);
        let html_children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(html_children.len(), 2);
        assert_eq!(html_children[1], table);
        assert_eq!(
            builder.document.node(html_children[0]).kind,
            NodeKind::Text {
                content: "x".to_owned()
            }
        );
    }

    #[test]
    fn foster_parenting_with_no_table_falls_back_to_the_html_element() {
        let mut builder = TreeBuilder::new();
        let html = bootstrap_html(&mut builder);
        // No table on the stack at all, but foster parenting is somehow
        // enabled and the current node is a foster-parenting target
        // (`tbody`) — falls back to inserting inside the topmost stack
        // element.
        builder.insert_html_element(&tag("tbody"), None);
        builder.foster_parenting = true;
        builder.insert_character('x', None);

        let html_children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(
            builder.document.node(html_children[1]).kind,
            NodeKind::Text {
                content: "x".to_owned()
            }
        );
    }
}

#[cfg(test)]
mod active_formatting_elements_tests {
    use crate::document::{Attribute, Document, NodeId, NodeKind};
    use crate::tokenizer::TagToken;
    use crate::tree_builder::{
        ActiveFormattingElements, FormattingEntry, HTML_NAMESPACE, TreeBuilder,
    };

    fn font_with_color(document: &mut Document, color: &str) -> NodeId {
        document.new_node(
            NodeKind::Element {
                name: "font".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![Attribute {
                    name: "color".to_owned(),
                    value: color.to_owned(),
                    namespace: None,
                }],
            },
            None,
        )
    }

    fn elements(afe: &ActiveFormattingElements) -> Vec<NodeId> {
        afe.entries
            .iter()
            .filter_map(|entry| match entry {
                FormattingEntry::Element(node) => Some(*node),
                FormattingEntry::Marker => None,
            })
            .collect()
    }

    #[test]
    fn noahs_ark_clause_removes_the_earliest_of_four_identical_elements() {
        let mut document = Document::new();
        let mut afe = ActiveFormattingElements::new();
        let nodes: Vec<_> = (0..4)
            .map(|_| font_with_color(&mut document, "red"))
            .collect();
        for &node in &nodes {
            afe.push(&document, node);
        }

        assert_eq!(elements(&afe), vec![nodes[1], nodes[2], nodes[3]]);
    }

    #[test]
    fn noahs_ark_clause_ignores_elements_with_different_attributes() {
        let mut document = Document::new();
        let mut afe = ActiveFormattingElements::new();
        let red: Vec<_> = (0..3)
            .map(|_| font_with_color(&mut document, "red"))
            .collect();
        let blue = font_with_color(&mut document, "blue");
        for &node in &red {
            afe.push(&document, node);
        }
        afe.push(&document, blue);

        // Four entries total: the three "red" ones don't collide with the
        // differently-attributed "blue" one, so nothing is removed.
        assert_eq!(elements(&afe).len(), 4);
    }

    #[test]
    fn noahs_ark_clause_only_counts_entries_after_the_last_marker() {
        let mut document = Document::new();
        let mut afe = ActiveFormattingElements::new();
        for _ in 0..3 {
            let node = font_with_color(&mut document, "red");
            afe.push(&document, node);
        }
        afe.push_marker();
        let node = font_with_color(&mut document, "red");
        afe.push(&document, node);

        // The marker resets the Noah's Ark count — the 3 pre-marker
        // entries plus the marker plus the 1 post-marker entry, none
        // removed.
        assert_eq!(afe.entries.len(), 5);
    }

    #[test]
    fn clear_up_to_last_marker_removes_the_marker_and_everything_after_it() {
        let mut document = Document::new();
        let mut afe = ActiveFormattingElements::new();
        let before = font_with_color(&mut document, "red");
        afe.push(&document, before);
        afe.push_marker();
        let after = font_with_color(&mut document, "blue");
        afe.push(&document, after);

        afe.clear_up_to_last_marker();

        assert_eq!(elements(&afe), vec![before]);
    }

    fn tag(name: &str) -> TagToken {
        TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: vec![],
        }
    }

    fn bootstrap_html(builder: &mut TreeBuilder) -> NodeId {
        let root = builder.document.root();
        let html = builder.document.new_node(
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            },
            None,
        );
        builder.document.append_child(root, html);
        builder.open_elements.push(html);
        html
    }

    #[test]
    fn reconstruct_does_nothing_when_the_list_is_empty() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("body"), None);
        let stack_before = builder.open_elements.current_node();

        builder.reconstruct_the_active_formatting_elements();

        assert_eq!(builder.open_elements.current_node(), stack_before);
    }

    #[test]
    fn reconstruct_does_nothing_when_the_last_entry_is_still_open() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);

        builder.reconstruct_the_active_formatting_elements();

        // `b` is still on the stack, so there's nothing to reconstruct —
        // no new node created.
        assert_eq!(builder.open_elements.current_node(), Some(b));
    }

    #[test]
    fn reconstruct_reopens_a_single_implicitly_closed_formatting_element() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);
        // Simulate `b` having been implicitly popped off the stack (e.g.
        // by a misnested close) while staying in the formatting-elements
        // list.
        builder.open_elements.pop();

        builder.reconstruct_the_active_formatting_elements();

        let new_b = builder
            .open_elements
            .current_node()
            .expect("reconstruct should have pushed a new element");
        assert_ne!(new_b, b, "must be a freshly created node, not the old one");
        // The original `b` stays in the tree exactly where it was parsed
        // — reconstruction never removes/moves anything, it only adds a
        // new sibling to carry the formatting forward.
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![b, new_b]
        );
        assert_eq!(
            builder.document.node(new_b).kind,
            NodeKind::Element {
                name: "b".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            crate::tree_builder::FormattingEntry::Element(new_b),
            *builder.active_formatting_elements.entries.last().unwrap()
        );
    }

    #[test]
    fn reconstruct_reopens_nested_formatting_elements_in_the_original_order() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);
        let i = builder.insert_html_element(&tag("i"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, i);
        // Both implicitly closed, in order.
        builder.open_elements.pop();
        builder.open_elements.pop();

        builder.reconstruct_the_active_formatting_elements();

        // The original b>i subtree stays exactly where it was parsed...
        let body_children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(body_children.len(), 2);
        assert_eq!(body_children[0], b);
        assert_eq!(builder.document.children(b).collect::<Vec<_>>(), vec![i]);
        // ...and reconstruction adds a fresh, equally-nested b>i sibling
        // structure to carry both formattings forward.
        let new_b = body_children[1];
        assert_eq!(
            builder.document.node(new_b).kind,
            NodeKind::Element {
                name: "b".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        let new_b_children: Vec<_> = builder.document.children(new_b).collect();
        assert_eq!(new_b_children.len(), 1);
        assert_eq!(
            builder.document.node(new_b_children[0]).kind,
            NodeKind::Element {
                name: "i".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.open_elements.current_node(),
            Some(new_b_children[0])
        );
    }
}

#[cfg(test)]
mod adoption_agency_tests {
    use crate::document::{NodeId, NodeKind};
    use crate::tokenizer::{Position, TagToken};
    use crate::tree_builder::TreeBuilder;

    fn pos() -> Position {
        Position {
            line: 1,
            column: 1,
            byte_offset: 0,
        }
    }

    fn tag(name: &str) -> TagToken {
        TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: vec![],
        }
    }

    fn bootstrap_html(builder: &mut TreeBuilder) -> NodeId {
        let root = builder.document.root();
        let html = builder.document.new_node(
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            },
            None,
        );
        builder.document.append_child(root, html);
        builder.open_elements.push(html);
        html
    }

    #[test]
    fn current_node_matching_and_not_active_is_just_popped() {
        // Step 2's fast path: `<b>` was inserted but never registered as
        // an active formatting element (e.g. it was already dropped from
        // that list by an earlier Noah's Ark/marker-clearing step) — so
        // there's nothing to reconstruct, just pop it like any other end
        // tag would.
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);

        builder.adoption_agency_algorithm(&tag("b"), pos());

        assert_eq!(builder.open_elements.current_node(), Some(body));
        // The element itself is untouched in the tree, just off the
        // stack.
        assert_eq!(builder.document.children(body).collect::<Vec<_>>(), vec![b]);
    }

    #[test]
    fn no_furthest_block_pops_up_to_and_including_the_formatting_element() {
        // `<p><b>bold</p>` (no misnesting content between `<b>` and the
        // current node) — step 8's simple case.
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let p = builder.insert_html_element(&tag("p"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);

        builder.adoption_agency_algorithm(&tag("b"), pos());

        assert_eq!(builder.open_elements.current_node(), Some(p));
        assert!(!builder.open_elements.contains(b));
        assert!(builder.active_formatting_elements.entries.is_empty());
        // The tree shape itself is untouched — `b` remains `p`'s child.
        assert_eq!(builder.document.children(p).collect::<Vec<_>>(), vec![b]);
        assert_eq!(builder.document.children(body).collect::<Vec<_>>(), vec![p]);
    }

    #[test]
    fn not_in_scope_returns_without_changing_the_tree() {
        // `formattingElement` (`b`) is still on the stack, but a `table`
        // — a default-scope boundary — sits between it and the current
        // node, so `has_element_in_scope` stops at `table` before ever
        // reaching `b`. Step 5 bails out entirely, parse error only.
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);
        let table = builder.insert_html_element(&tag("table"), None);

        builder.adoption_agency_algorithm(&tag("b"), pos());

        // Nothing changed: no pop, no tree mutation.
        assert_eq!(builder.open_elements.current_node(), Some(table));
        assert!(builder.open_elements.contains(b));
        assert_eq!(
            builder.document.children(b).collect::<Vec<_>>(),
            vec![table]
        );
    }

    #[test]
    fn relocates_special_content_out_of_the_misnested_formatting_element() {
        // The canonical case: `<b><div>inner</div></b>` — `div` (a
        // special-category element) is still open when `</b>` arrives,
        // so it becomes the "furthest block" that gets relocated: `b`
        // ends up split into an empty original and a fresh clone that
        // wraps `div`'s former content, nested *inside* `div`.
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let b = builder.insert_html_element(&tag("b"), None);
        builder
            .active_formatting_elements
            .push(&builder.document, b);
        let div = builder.insert_html_element(&tag("div"), None);
        let inner_text = builder.document.new_node(
            NodeKind::Text {
                content: "inner".to_owned(),
            },
            None,
        );
        builder.document.append_child(div, inner_text);

        builder.adoption_agency_algorithm(&tag("b"), pos());

        // Stack: html, body, div — both `b` (the original) and its
        // clone are fully popped off by the end (the clone itself hits
        // the outer loop's own no-furthest-block fast path on its
        // second pass).
        assert_eq!(builder.open_elements.current_node(), Some(div));
        assert!(!builder.open_elements.contains(b));
        assert!(builder.active_formatting_elements.entries.is_empty());

        let body_children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(body_children, vec![b, div]);
        assert_eq!(builder.document.children(b).count(), 0);

        let div_children: Vec<_> = builder.document.children(div).collect();
        assert_eq!(div_children.len(), 1);
        let new_b = div_children[0];
        assert_ne!(new_b, b);
        assert_eq!(
            builder.document.node(new_b).kind,
            NodeKind::Element {
                name: "b".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.document.children(new_b).collect::<Vec<_>>(),
            vec![inner_text]
        );
    }

    #[test]
    fn any_other_end_tag_pops_up_to_a_matching_ancestor() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        let body = builder.insert_html_element(&tag("body"), None);
        let div = builder.insert_html_element(&tag("div"), None);
        builder.insert_html_element(&tag("span"), None);

        builder.any_other_end_tag_in_body(&tag("div"), pos());

        assert_eq!(builder.open_elements.current_node(), Some(body));
        assert!(!builder.open_elements.contains(div));
    }

    #[test]
    fn any_other_end_tag_gives_up_at_the_first_special_element() {
        let mut builder = TreeBuilder::new();
        bootstrap_html(&mut builder);
        builder.insert_html_element(&tag("body"), None);
        builder.insert_html_element(&tag("div"), None); // special: blocks the search
        let span = builder.insert_html_element(&tag("span"), None);

        // No `x` element exists anywhere on the stack.
        builder.any_other_end_tag_in_body(&tag("x"), pos());

        // Nothing was popped: the search hit `div` (special) before
        // finding a match and gave up.
        assert_eq!(builder.open_elements.current_node(), Some(span));
    }
}

#[cfg(test)]
mod insertion_mode_tests {
    use crate::document::{NodeId, NodeKind};
    use crate::tokenizer::{Attribute, DoctypeToken, ExternalState, Position, TagToken, TokenKind};
    use crate::tree_builder::{FormattingEntry, InsertionMode, QuirksMode, TreeBuilder};

    fn pos() -> Position {
        Position {
            line: 1,
            column: 1,
            byte_offset: 0,
        }
    }

    fn doctype(
        name: Option<&str>,
        public_identifier: Option<&str>,
        system_identifier: Option<&str>,
    ) -> TokenKind {
        TokenKind::Doctype(DoctypeToken {
            name: name.map(str::to_owned),
            public_identifier: public_identifier.map(str::to_owned),
            system_identifier: system_identifier.map(str::to_owned),
            force_quirks: false,
        })
    }

    fn start_tag(name: &str) -> TokenKind {
        TokenKind::StartTag(TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: vec![],
        })
    }

    fn start_tag_with_attrs(name: &str, attrs: &[(&str, &str)]) -> TokenKind {
        TokenKind::StartTag(TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: attrs
                .iter()
                .map(|&(name, value)| Attribute {
                    name: name.to_owned(),
                    value: value.to_owned(),
                })
                .collect(),
        })
    }

    fn end_tag(name: &str) -> TokenKind {
        TokenKind::EndTag(TagToken {
            name: name.to_owned(),
            self_closing: false,
            attributes: vec![],
        })
    }

    #[test]
    fn initial_mode_ignores_whitespace() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&TokenKind::Character(' '), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::Initial);
        assert_eq!(
            builder.document.children(builder.document.root()).count(),
            0
        );
    }

    #[test]
    fn initial_mode_inserts_comment_as_a_document_child() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&TokenKind::Comment("hi".to_owned()), pos());

        let root = builder.document.root();
        let children: Vec<_> = builder.document.children(root).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Comment {
                content: "hi".to_owned()
            }
        );
        assert_eq!(builder.insertion_mode, InsertionMode::Initial);
    }

    #[test]
    fn initial_mode_doctype_inserts_a_doctype_node_and_switches_mode() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&doctype(Some("html"), None, None), pos());

        let root = builder.document.root();
        let children: Vec<_> = builder.document.children(root).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Doctype {
                name: Some("html".to_owned()),
                public_identifier: Some(String::new()),
                system_identifier: Some(String::new()),
            }
        );
        assert_eq!(builder.quirks_mode, QuirksMode::NoQuirks);
        assert_eq!(builder.insertion_mode, InsertionMode::BeforeHtml);
    }

    #[test]
    fn initial_mode_doctype_records_quirks_mode() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&doctype(Some("html"), Some("HTML"), None), pos());

        assert_eq!(builder.quirks_mode, QuirksMode::Quirks);
    }

    #[test]
    fn end_to_end_doctype_then_html_then_head_reaches_in_head() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&doctype(Some("html"), None, None), pos());
        builder.process_token(&start_tag_with_attrs("html", &[("lang", "en")]), pos());
        builder.process_token(&start_tag("head"), pos());

        assert_eq!(builder.quirks_mode, QuirksMode::NoQuirks);
        assert_eq!(builder.insertion_mode, InsertionMode::InHead);

        let root = builder.document.root();
        let root_children: Vec<_> = builder.document.children(root).collect();
        // [doctype, html]
        assert_eq!(root_children.len(), 2);
        let html = root_children[1];
        assert_eq!(
            builder.document.node(html).kind,
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![crate::tree_builder::Attribute {
                    name: "lang".to_owned(),
                    value: "en".to_owned(),
                    namespace: None,
                }],
            }
        );
        let head = builder.document.children(html).next().unwrap();
        assert_eq!(
            builder.document.node(head).kind,
            NodeKind::Element {
                name: "head".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.head_element_pointer, Some(head));
        assert_eq!(builder.open_elements.current_node(), Some(head));
    }

    #[test]
    fn end_to_end_without_a_doctype_sets_quirks_mode_and_synthesizes_html_and_head() {
        // No DOCTYPE token at all: a single `process_token` call cascades
        // Initial -> BeforeHtml -> BeforeHead (all via "anything
        // else"/reprocess) before finally being consumed by BeforeHead's
        // own "start tag head" rule.
        let mut builder = TreeBuilder::new();
        builder.process_token(&start_tag("head"), pos());

        assert_eq!(builder.quirks_mode, QuirksMode::Quirks);
        assert_eq!(builder.insertion_mode, InsertionMode::InHead);

        let root = builder.document.root();
        let html = builder.document.children(root).next().unwrap();
        assert_eq!(
            builder.document.node(html).kind,
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        let head = builder.document.children(html).next().unwrap();
        assert_eq!(builder.head_element_pointer, Some(head));
        assert!(builder.open_elements.contains(html));
    }

    #[test]
    fn before_html_mode_anything_else_synthesizes_html_and_reprocesses() {
        let mut builder = TreeBuilder::new();
        builder.insertion_mode = InsertionMode::BeforeHtml;

        let outcome = builder.process_token_before_html(&end_tag("br"), pos());

        assert!(matches!(
            outcome,
            crate::tree_builder::TokenOutcome::Reprocess
        ));
        assert_eq!(builder.insertion_mode, InsertionMode::BeforeHead);
        let root = builder.document.root();
        let html = builder.document.children(root).next().unwrap();
        assert_eq!(
            builder.document.node(html).kind,
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.open_elements.current_node(), Some(html));
    }

    #[test]
    fn before_head_mode_html_start_tag_merges_new_attributes_only() {
        let mut builder = TreeBuilder::new();
        builder.insertion_mode = InsertionMode::BeforeHtml;
        builder.process_token(&start_tag_with_attrs("html", &[("lang", "en")]), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::BeforeHead);

        builder.process_token_before_head(
            &start_tag_with_attrs("html", &[("lang", "de"), ("dir", "ltr")]),
            pos(),
        );

        let root = builder.document.root();
        let html = builder.document.children(root).next().unwrap();
        let NodeKind::Element { attributes, .. } = &builder.document.node(html).kind else {
            unreachable!()
        };
        // "lang" already existed (value untouched), "dir" is new.
        assert_eq!(attributes.len(), 2);
        assert!(
            attributes
                .iter()
                .any(|a| a.name == "lang" && a.value == "en")
        );
        assert!(
            attributes
                .iter()
                .any(|a| a.name == "dir" && a.value == "ltr")
        );
    }

    /// Drives a fresh builder through Initial -> BeforeHtml -> BeforeHead
    /// -> InHead by processing a `<head>` start tag (already exercised
    /// end-to-end above) and returns `(html, head)`.
    fn bootstrap_in_head(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        builder.process_token(&start_tag("head"), pos());
        let root = builder.document.root();
        let html = builder.document.children(root).next().unwrap();
        let head = builder.open_elements.current_node().unwrap();
        (html, head)
    }

    #[test]
    fn in_head_inserts_whitespace_characters() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);

        builder.process_token(&TokenKind::Character(' '), pos());

        let children: Vec<_> = builder.document.children(head).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: " ".to_owned()
            }
        );
    }

    #[test]
    fn in_head_void_like_elements_are_inserted_then_immediately_popped() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);

        builder.process_token(&start_tag("meta"), pos());

        let children: Vec<_> = builder.document.children(head).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "meta".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.open_elements.current_node(), Some(head));
    }

    #[test]
    fn in_head_title_switches_to_text_mode_with_rcdata() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);

        let state = builder.process_token(&start_tag("title"), pos());

        assert_eq!(state, Some(ExternalState::RcData));
        assert_eq!(builder.insertion_mode, InsertionMode::Text);
        assert_eq!(builder.original_insertion_mode, Some(InsertionMode::InHead));
        let title = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(head).collect::<Vec<_>>(),
            vec![title]
        );
    }

    #[test]
    fn in_head_script_switches_to_text_mode_with_script_data() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);

        let state = builder.process_token(&start_tag("script"), pos());

        assert_eq!(state, Some(ExternalState::ScriptData));
        assert_eq!(builder.insertion_mode, InsertionMode::Text);
    }

    #[test]
    fn in_head_noscript_start_tag_switches_mode() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);

        builder.process_token(&start_tag("noscript"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InHeadNoscript);
    }

    #[test]
    fn in_head_end_tag_head_pops_and_switches_to_after_head() {
        let mut builder = TreeBuilder::new();
        let (html, head) = bootstrap_in_head(&mut builder);

        builder.process_token(&end_tag("head"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::AfterHead);
        assert_eq!(builder.open_elements.current_node(), Some(html));
        assert!(!builder.open_elements.contains(head));
    }

    #[test]
    fn in_head_template_round_trips_through_the_stack() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);

        builder.process_token(&start_tag("template"), pos());
        let template = builder.open_elements.current_node().unwrap();
        assert_ne!(template, head);

        builder.process_token(&end_tag("template"), pos());

        assert_eq!(builder.open_elements.current_node(), Some(head));
        assert!(!builder.open_elements.contains(template));
        assert_eq!(
            builder.document.children(head).collect::<Vec<_>>(),
            vec![template]
        );
    }

    #[test]
    fn in_head_template_start_tag_pushes_marker_and_switches_to_in_template() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);
        builder.frameset_ok = true;

        builder.process_token(&start_tag("template"), pos());

        let template = builder.open_elements.current_node().unwrap();
        assert!(matches!(
            builder.active_formatting_elements.entries.last(),
            Some(FormattingEntry::Marker)
        ));
        assert!(!builder.frameset_ok);
        assert_eq!(builder.insertion_mode, InsertionMode::InTemplate);
        assert_eq!(
            builder.stack_of_template_insertion_modes,
            vec![InsertionMode::InTemplate]
        );
        // The template's own child is its content fragment, not
        // whatever gets inserted "inside" it afterward.
        let content: Vec<_> = builder.document.children(template).collect();
        assert_eq!(content.len(), 1);
        assert_eq!(
            builder.document.node(content[0]).kind,
            NodeKind::DocumentFragment
        );
    }

    #[test]
    fn in_head_template_end_tag_resets_insertion_mode_via_the_stack() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);

        builder.process_token(&start_tag("template"), pos());
        builder.process_token(&end_tag("template"), pos());

        assert!(builder.stack_of_template_insertion_modes.is_empty());
        // `reset_the_insertion_mode_appropriately` recomputes from the
        // actual current node (`head`) rather than leaving `InTemplate`
        // stale — same reasoning as the `</template>` fix in
        // `process_token_in_head`'s sibling arm above.
        assert_eq!(builder.insertion_mode, InsertionMode::InHead);
    }

    #[test]
    fn content_inserted_under_a_template_lands_in_its_content_fragment() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);
        builder.process_token(&start_tag("template"), pos());
        let template = builder.open_elements.current_node().unwrap();
        let content = builder.document.children(template).next().unwrap();

        // "In template" mode's "any other start tag" arm: pop/push/switch
        // to "in body", reprocess.
        builder.process_token(&start_tag("p"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        assert_eq!(
            builder.stack_of_template_insertion_modes,
            vec![InsertionMode::InBody]
        );
        let p = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.document.parent(p), Some(content));
        assert_eq!(
            builder.document.children(template).collect::<Vec<_>>(),
            vec![content]
        );
    }

    #[test]
    fn in_template_tr_cascades_through_in_table_body_and_in_row() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);
        builder.process_token(&start_tag("template"), pos());
        let template = builder.open_elements.current_node().unwrap();
        let content = builder.document.children(template).next().unwrap();

        // "In template"'s own `tr` arm just switches to "in table body"
        // and reprocesses — which is what actually inserts `tr` and
        // moves on to "in row" (§13.2.6.4.13). `process_token` drives
        // that whole cascade to completion before returning.
        builder.process_token(&start_tag("tr"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::InRow);
        // The template-insertion-modes stack only reflects the mode
        // "in template" itself switched to — the "in table body" ->
        // "in row" transition is an ordinary table-mode cascade, not a
        // template-specific one, so it doesn't touch this stack.
        assert_eq!(
            builder.stack_of_template_insertion_modes,
            vec![InsertionMode::InTableBody]
        );
        let tr = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.document.parent(tr), Some(content));

        builder.process_token(&start_tag("td"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::InCell);
        let td = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.document.parent(td), Some(tr));
    }

    #[test]
    fn in_template_any_other_end_tag_is_ignored() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);
        builder.process_token(&start_tag("template"), pos());
        let current_before = builder.open_elements.current_node();

        builder.process_token(&end_tag("p"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InTemplate);
        assert_eq!(builder.open_elements.current_node(), current_before);
    }

    #[test]
    fn in_head_noscript_delegates_link_to_in_head() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_head(&mut builder);
        builder.process_token(&start_tag("noscript"), pos());
        let noscript = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("link"), pos());

        // `link` is inserted+popped as a child of `noscript` (the current
        // node while in "in head noscript") — delegation runs "in
        // head"'s rules verbatim, including its own insertion target.
        assert_eq!(builder.open_elements.current_node(), Some(noscript));
        let children: Vec<_> = builder.document.children(noscript).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "link".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn in_head_noscript_end_tag_noscript_returns_to_in_head() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);
        builder.process_token(&start_tag("noscript"), pos());

        builder.process_token(&end_tag("noscript"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InHead);
        assert_eq!(builder.open_elements.current_node(), Some(head));
    }

    #[test]
    fn after_head_body_start_tag_switches_to_in_body() {
        let mut builder = TreeBuilder::new();
        let (html, head) = bootstrap_in_head(&mut builder);
        builder.process_token(&end_tag("head"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::AfterHead);

        builder.process_token(&start_tag("body"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        let body = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(html).collect::<Vec<_>>(),
            vec![head, body]
        );
    }

    #[test]
    fn after_head_delegates_link_by_temporarily_repushing_head() {
        let mut builder = TreeBuilder::new();
        let (_, head) = bootstrap_in_head(&mut builder);
        builder.process_token(&end_tag("head"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::AfterHead);

        builder.process_token(&start_tag("link"), pos());

        // `head` is not left on the stack afterward ("it might not be
        // the current node at this point" — here it's not on the stack
        // *at all* anymore, matching before this call too)...
        assert!(!builder.open_elements.contains(head));
        // ...but the link element really did get inserted as its child.
        let children: Vec<_> = builder.document.children(head).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "link".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.insertion_mode, InsertionMode::AfterHead);
    }

    #[test]
    fn after_head_anything_else_synthesizes_body_and_switches_mode() {
        let mut builder = TreeBuilder::new();
        let (html, head) = bootstrap_in_head(&mut builder);
        builder.process_token(&end_tag("head"), pos());

        // Called directly (not via the public `process_token` loop):
        // "Reprocess" would otherwise hand the same token to "in body",
        // not yet implemented.
        let outcome = builder.process_token_after_head(&TokenKind::Character('x'), pos());

        assert!(matches!(
            outcome,
            crate::tree_builder::TokenOutcome::Reprocess
        ));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        let body = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(body).kind,
            NodeKind::Element {
                name: "body".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.document.children(html).collect::<Vec<_>>(),
            vec![head, body]
        );
    }

    /// Drives a fresh builder all the way into "in body" via a single
    /// `<body>` start tag (cascading Initial -> BeforeHtml -> BeforeHead
    /// -> InHead -> AfterHead, ending at AfterHead's own explicit
    /// "start tag body" rule — already exercised end-to-end above).
    /// Returns `(html, body)`.
    fn bootstrap_in_body(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        builder.process_token(&start_tag("body"), pos());
        let root = builder.document.root();
        let html = builder.document.children(root).next().unwrap();
        let body = builder.open_elements.current_node().unwrap();
        (html, body)
    }

    #[test]
    fn in_body_character_sets_frameset_ok_and_ignores_nul() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        // `bootstrap_in_body` goes through "after head"'s real `<body>`
        // start-tag rule, which itself sets frameset-ok to "not ok" —
        // reset it here to isolate this test's actual subject, "in
        // body"'s own character-token rule.
        builder.frameset_ok = true;

        builder.process_token(&TokenKind::Character('\0'), pos());
        assert_eq!(builder.document.children(body).count(), 0);

        builder.process_token(&TokenKind::Character('x'), pos());
        assert!(!builder.frameset_ok);
        let children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: "x".to_owned()
            }
        );
    }

    #[test]
    fn in_body_div_closes_an_open_p_in_button_scope() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("p"), pos());
        let p = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("div"), pos());

        assert!(!builder.open_elements.contains(p));
        let div = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![p, div]
        );
    }

    #[test]
    fn in_body_heading_pops_a_currently_open_heading() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("h1"), pos());
        let h1 = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("h2"), pos());

        assert!(!builder.open_elements.contains(h1));
        let h2 = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![h1, h2]
        );
    }

    #[test]
    fn in_body_formatting_start_tag_pushes_onto_active_formatting_elements() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("b"), pos());

        let b = builder.open_elements.current_node().unwrap();
        assert!(matches!(
            builder.active_formatting_elements.entries.last(),
            Some(crate::tree_builder::FormattingEntry::Element(node)) if *node == b
        ));
    }

    #[test]
    fn in_body_end_tag_b_runs_the_adoption_agency_algorithm() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("b"), pos());
        let b = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("b"), pos());

        // Simple case (no misnesting): adoption agency's step 2 fast
        // path just pops it.
        assert!(!builder.open_elements.contains(b));
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    #[test]
    fn in_body_second_a_start_tag_closes_the_first_via_adoption_agency() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("a"), pos());
        let a1 = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("a"), pos());

        let a2 = builder.open_elements.current_node().unwrap();
        assert_ne!(a1, a2);
        assert!(!builder.open_elements.contains(a1));
        assert!(matches!(
            builder.active_formatting_elements.entries.last(),
            Some(crate::tree_builder::FormattingEntry::Element(node)) if *node == a2
        ));
    }

    #[test]
    fn in_body_void_element_is_inserted_then_immediately_popped() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("br"), pos());

        let children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "br".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.open_elements.current_node(), Some(body));
        assert!(!builder.frameset_ok);
    }

    #[test]
    fn in_body_br_end_tag_is_treated_as_a_start_tag() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);

        builder.process_token(&end_tag("br"), pos());

        let children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "br".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn in_body_li_closes_a_previous_open_li() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("ul"), pos());
        builder.process_token(&start_tag("li"), pos());
        let li1 = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("li"), pos());

        assert!(!builder.open_elements.contains(li1));
        let ul = builder.document.children(body).next().unwrap();
        let ul_children: Vec<_> = builder.document.children(ul).collect();
        assert_eq!(ul_children.len(), 2);
        assert_eq!(ul_children[0], li1);
    }

    #[test]
    fn in_body_form_sets_pointer_and_ignores_a_second_form() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("form"), pos());
        let form1 = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.form_element_pointer, Some(form1));

        builder.process_token(&start_tag("form"), pos());

        // Second form is ignored entirely: no new element, pointer
        // unchanged, current node unchanged.
        assert_eq!(builder.open_elements.current_node(), Some(form1));
        assert_eq!(builder.form_element_pointer, Some(form1));
    }

    #[test]
    fn in_body_table_switches_insertion_mode() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("table"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InTable);
        assert!(!builder.frameset_ok);
    }

    #[test]
    fn in_body_plaintext_returns_plaintext_state_without_switching_mode() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        let state = builder.process_token(&start_tag("plaintext"), pos());

        assert_eq!(state, Some(ExternalState::PlainText));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
    }

    #[test]
    fn in_body_textarea_switches_to_text_mode_and_skips_next_lf() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        let state = builder.process_token(&start_tag("textarea"), pos());

        assert_eq!(state, Some(ExternalState::RcData));
        assert_eq!(builder.insertion_mode, InsertionMode::Text);
        assert!(builder.skip_next_line_feed);
        assert!(!builder.frameset_ok);

        // Unlike `<pre>`/`<listing>` (which stay in "in body"), the flag
        // set above is only ever consulted once the driver has actually
        // switched to "text" mode's own Character handling — regression
        // test for a bug the html5lib-tests conformance corpus caught
        // (tests3.dat's `<textarea>\nfoo</textarea>` cases), where
        // "text" mode never checked it at all and the leading LF ended
        // up in the tree.
        let textarea = builder.open_elements.current_node().unwrap();
        builder.process_token(&TokenKind::Character('\n'), pos());
        assert_eq!(builder.document.children(textarea).count(), 0);
        assert!(!builder.skip_next_line_feed);

        builder.process_token(&TokenKind::Character('x'), pos());
        assert_eq!(builder.document.children(textarea).count(), 1);
    }

    #[test]
    fn in_body_pre_ignores_one_leading_line_feed() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("pre"), pos());
        let pre = builder.open_elements.current_node().unwrap();
        assert!(builder.skip_next_line_feed);

        builder.process_token(&TokenKind::Character('\n'), pos());
        assert_eq!(builder.document.children(pre).count(), 0);
        assert!(!builder.skip_next_line_feed);

        builder.process_token(&TokenKind::Character('x'), pos());
        assert_eq!(builder.document.children(pre).count(), 1);
    }

    #[test]
    fn in_body_math_inserts_with_mathml_namespace() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("math"), pos());

        let math = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(math).kind,
            NodeKind::Element {
                name: "math".to_owned(),
                namespace: Some(crate::tree_builder::MATHML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn in_body_any_other_start_tag_is_inserted_as_an_ordinary_element() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("span"), pos());

        let span = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(span).kind,
            NodeKind::Element {
                name: "span".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![span]
        );
    }

    #[test]
    fn in_body_stray_table_section_start_tags_are_ignored() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("tbody"), pos());

        assert_eq!(builder.document.children(body).count(), 0);
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    #[test]
    fn in_body_delegates_meta_to_in_head_rules() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);

        builder.process_token(&start_tag("meta"), pos());

        let children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "meta".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    #[test]
    fn in_body_any_other_end_tag_pops_through_a_matching_ancestor() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("custom-element"), pos());
        let custom = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("custom-element"), pos());

        assert!(!builder.open_elements.contains(custom));
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    #[test]
    fn text_mode_inserts_characters_into_the_rcdata_element() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("title"), pos());
        let title = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.insertion_mode, InsertionMode::Text);

        builder.process_token(&TokenKind::Character('h'), pos());
        builder.process_token(&TokenKind::Character('i'), pos());

        let children: Vec<_> = builder.document.children(title).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: "hi".to_owned()
            }
        );
    }

    #[test]
    fn text_mode_end_tag_pops_and_restores_original_insertion_mode() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("title"), pos());
        let title = builder.open_elements.current_node().unwrap();

        // Any end tag closes it — even one that doesn't match "title",
        // matching "text" mode's "any other end tag" rule.
        builder.process_token(&end_tag("nonsense"), pos());

        assert!(!builder.open_elements.contains(title));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        assert_eq!(builder.open_elements.current_node(), Some(body));
        assert_eq!(builder.original_insertion_mode, None);
    }

    #[test]
    fn text_mode_eof_pops_and_reprocesses_in_the_original_mode() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("title"), pos());
        let title = builder.open_elements.current_node().unwrap();

        builder.process_token(&TokenKind::Eof, pos());

        assert!(!builder.open_elements.contains(title));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    /// Drives a fresh builder into "in table" via `bootstrap_in_body`
    /// plus a `<table>` start tag (already exercised above). Returns
    /// `(body, table)`.
    fn bootstrap_in_table(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        let (_, body) = bootstrap_in_body(builder);
        builder.process_token(&start_tag("table"), pos());
        let table = builder.open_elements.current_node().unwrap();
        (body, table)
    }

    #[test]
    fn in_table_caption_switches_mode_and_pushes_a_marker() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);

        builder.process_token(&start_tag("caption"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InCaption);
        let caption = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(table).collect::<Vec<_>>(),
            vec![caption]
        );
        assert!(matches!(
            builder.active_formatting_elements.entries.last(),
            Some(crate::tree_builder::FormattingEntry::Marker)
        ));
    }

    #[test]
    fn in_table_col_synthesizes_a_colgroup_and_reprocesses() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);

        builder.process_token(&start_tag("col"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InColumnGroup);
        let colgroup = builder.document.children(table).next().unwrap();
        assert_eq!(
            builder.document.node(colgroup).kind,
            NodeKind::Element {
                name: "colgroup".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        let col = builder.document.children(colgroup).next().unwrap();
        assert_eq!(
            builder.document.node(col).kind,
            NodeKind::Element {
                name: "col".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        // The synthesized colgroup was popped back off after inserting
        // col; current node is the colgroup itself (still open).
        assert_eq!(builder.open_elements.current_node(), Some(colgroup));
    }

    #[test]
    fn in_table_tr_synthesizes_tbody_then_cascades_into_in_row() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);

        // A single process_token call cascades InTable -> InTableBody
        // (synthesize tbody, reprocess) -> InRow (insert the real tr).
        builder.process_token(&start_tag("tr"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InRow);
        let tbody = builder.document.children(table).next().unwrap();
        assert_eq!(
            builder.document.node(tbody).kind,
            NodeKind::Element {
                name: "tbody".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        let tr = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(tbody).collect::<Vec<_>>(),
            vec![tr]
        );
    }

    #[test]
    fn in_table_nested_table_closes_the_first_and_starts_a_sibling() {
        let mut builder = TreeBuilder::new();
        let (body, table1) = bootstrap_in_table(&mut builder);

        builder.process_token(&start_tag("table"), pos());

        // The nested <table> does NOT nest: it implicitly closes the
        // first one (reset_the_insertion_mode_appropriately lands back
        // in "in body"), then "in body"'s own table rule creates a
        // fresh sibling table.
        assert_eq!(builder.insertion_mode, InsertionMode::InTable);
        let table2 = builder.open_elements.current_node().unwrap();
        assert_ne!(table1, table2);
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![table1, table2]
        );
        assert_eq!(builder.document.children(table1).count(), 0);
    }

    #[test]
    fn in_table_whitespace_character_is_buffered_and_inserted_normally() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);

        builder.process_token(&TokenKind::Character(' '), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::InTableText);

        // A non-character token (a start tag) flushes the buffer and
        // reprocesses itself.
        builder.process_token(&start_tag("caption"), pos());

        let children: Vec<_> = builder.document.children(table).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: " ".to_owned()
            }
        );
        assert_eq!(builder.insertion_mode, InsertionMode::InCaption);
    }

    #[test]
    fn in_table_non_whitespace_character_is_foster_parented_before_the_table() {
        let mut builder = TreeBuilder::new();
        let (body, table) = bootstrap_in_table(&mut builder);

        builder.process_token(&TokenKind::Character('x'), pos());
        builder.process_token(&start_tag("caption"), pos());

        // Foster-parented: the text lands as body's child immediately
        // before the table, not inside it.
        let body_children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(body_children.len(), 2);
        assert_eq!(
            builder.document.node(body_children[0]).kind,
            NodeKind::Text {
                content: "x".to_owned()
            }
        );
        assert_eq!(body_children[1], table);
        assert_eq!(builder.document.children(table).count(), 1); // just the caption
    }

    #[test]
    fn in_caption_end_tag_closes_and_returns_to_in_table() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("caption"), pos());
        let caption = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("caption"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InTable);
        assert!(!builder.open_elements.contains(caption));
        assert_eq!(builder.open_elements.current_node(), Some(table));
    }

    #[test]
    fn in_column_group_non_whitespace_closes_and_reprocesses_in_table() {
        let mut builder = TreeBuilder::new();
        let (_, table) = bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("colgroup"), pos());
        let colgroup = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("tbody"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InTableBody);
        assert!(!builder.open_elements.contains(colgroup));
        let tbody = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.children(table).collect::<Vec<_>>(),
            vec![colgroup, tbody]
        );
    }

    #[test]
    fn in_table_body_end_tag_table_closes_tbody_and_the_table() {
        let mut builder = TreeBuilder::new();
        let (body, table) = bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("tbody"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::InTableBody);

        builder.process_token(&end_tag("table"), pos());

        assert!(!builder.open_elements.contains(table));
        assert_eq!(builder.open_elements.current_node(), Some(body));
    }

    #[test]
    fn in_row_td_switches_to_in_cell_and_pushes_a_marker() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("tr"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::InRow);
        let tr = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("td"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InCell);
        let td = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.document.children(tr).collect::<Vec<_>>(), vec![td]);
        assert!(matches!(
            builder.active_formatting_elements.entries.last(),
            Some(crate::tree_builder::FormattingEntry::Marker)
        ));
    }

    #[test]
    fn in_row_end_tag_tr_returns_to_in_table_body() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("tr"), pos());
        let tr = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("tr"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InTableBody);
        assert!(!builder.open_elements.contains(tr));
    }

    #[test]
    fn in_cell_end_tag_td_closes_and_returns_to_in_row() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("tr"), pos());
        let tr = builder.open_elements.current_node().unwrap();
        builder.process_token(&start_tag("td"), pos());
        let td = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("td"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InRow);
        assert!(!builder.open_elements.contains(td));
        assert_eq!(builder.open_elements.current_node(), Some(tr));
    }

    #[test]
    fn in_cell_next_cell_start_tag_closes_the_current_one_and_reprocesses() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_table(&mut builder);
        builder.process_token(&start_tag("tr"), pos());
        let tr = builder.open_elements.current_node().unwrap();
        builder.process_token(&start_tag("td"), pos());
        let td1 = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("td"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InCell);
        let td2 = builder.open_elements.current_node().unwrap();
        assert_ne!(td1, td2);
        assert!(!builder.open_elements.contains(td1));
        assert_eq!(
            builder.document.children(tr).collect::<Vec<_>>(),
            vec![td1, td2]
        );
    }

    /// Drives a fresh builder into "after body" via `bootstrap_in_body`
    /// plus a `</body>` end tag. Returns `(html, body)`.
    fn bootstrap_after_body(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        let (html, body) = bootstrap_in_body(builder);
        builder.process_token(&end_tag("body"), pos());
        (html, body)
    }

    #[test]
    fn after_body_comment_is_appended_to_the_html_element() {
        let mut builder = TreeBuilder::new();
        let (html, body) = bootstrap_after_body(&mut builder);
        assert_eq!(builder.insertion_mode, InsertionMode::AfterBody);

        builder.process_token(&TokenKind::Comment("hi".to_owned()), pos());

        // html's children are [head, body, comment] — the comment is
        // appended last, after the already-present head and body.
        let html_children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(html_children.len(), 3);
        assert_eq!(html_children[1], body);
        assert_eq!(
            builder.document.node(html_children[2]).kind,
            NodeKind::Comment {
                content: "hi".to_owned()
            }
        );
    }

    #[test]
    fn after_body_end_tag_html_switches_to_after_after_body() {
        let mut builder = TreeBuilder::new();
        bootstrap_after_body(&mut builder);

        builder.process_token(&end_tag("html"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::AfterAfterBody);
    }

    #[test]
    fn after_body_whitespace_character_is_delegated_to_in_body() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_after_body(&mut builder);

        builder.process_token(&TokenKind::Character(' '), pos());

        let children: Vec<_> = builder.document.children(body).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: " ".to_owned()
            }
        );
        assert_eq!(builder.insertion_mode, InsertionMode::AfterBody);
    }

    #[test]
    fn after_body_anything_else_switches_to_in_body_and_reprocesses() {
        let mut builder = TreeBuilder::new();
        let (_, body) = bootstrap_after_body(&mut builder);

        builder.process_token(&start_tag("p"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        let p = builder.open_elements.current_node().unwrap();
        assert_eq!(builder.document.children(body).collect::<Vec<_>>(), vec![p]);
    }

    /// Drives a fresh builder into "after after body" via
    /// `bootstrap_after_body` plus a `</html>` end tag. Returns the
    /// document root.
    fn bootstrap_after_after_body(builder: &mut TreeBuilder) -> NodeId {
        bootstrap_after_body(builder);
        builder.process_token(&end_tag("html"), pos());
        builder.document.root()
    }

    #[test]
    fn after_after_body_comment_is_appended_to_the_document() {
        let mut builder = TreeBuilder::new();
        let root = bootstrap_after_after_body(&mut builder);

        builder.process_token(&TokenKind::Comment("hi".to_owned()), pos());

        let children: Vec<_> = builder.document.children(root).collect();
        let last = *children.last().unwrap();
        assert_eq!(
            builder.document.node(last).kind,
            NodeKind::Comment {
                content: "hi".to_owned()
            }
        );
    }

    #[test]
    fn after_after_body_whitespace_character_is_delegated_to_in_body() {
        let mut builder = TreeBuilder::new();
        bootstrap_after_after_body(&mut builder);

        builder.process_token(&TokenKind::Character(' '), pos());

        // Whitespace in "after after body" stays put (delegates to "in
        // body", which inserts it into the still-open body element) —
        // the insertion mode itself doesn't change for whitespace.
        assert_eq!(builder.insertion_mode, InsertionMode::AfterAfterBody);
    }

    #[test]
    fn after_after_body_anything_else_switches_to_in_body_and_reprocesses() {
        let mut builder = TreeBuilder::new();
        bootstrap_after_after_body(&mut builder);

        builder.process_token(&start_tag("p"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
        let p = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(p).kind,
            NodeKind::Element {
                name: "p".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn in_body_frameset_replaces_body_when_frameset_ok() {
        let mut builder = TreeBuilder::new();
        let (html, body) = bootstrap_in_body(&mut builder);
        // "after head"'s own `<body>` rule already set frameset-ok to
        // "not ok" (§13.2.6.4.6) — reset it to exercise the case this
        // test is actually about, matching
        // `in_body_character_sets_frameset_ok_and_ignores_nul`'s same
        // reasoning above.
        builder.frameset_ok = true;
        let p = builder.insert_html_element(
            &TagToken {
                name: "p".to_owned(),
                self_closing: false,
                attributes: vec![],
            },
            None,
        );

        builder.process_token(&start_tag("frameset"), pos());

        // `body` (with its `p` child) is detached entirely; `html`'s
        // children are now [head, frameset] — `head` is untouched, only
        // the *second* element (`body`) is ever removed.
        let html_children: Vec<_> = builder.document.children(html).collect();
        assert_eq!(html_children.len(), 2);
        let frameset = html_children[1];
        assert_eq!(
            builder.document.node(frameset).kind,
            NodeKind::Element {
                name: "frameset".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.document.parent(body), None);
        assert_eq!(builder.document.parent(p), Some(body));
        assert_eq!(builder.open_elements.current_node(), Some(frameset));
        assert_eq!(builder.insertion_mode, InsertionMode::InFrameset);
    }

    #[test]
    fn in_body_frameset_ignored_when_frameset_ok_is_not_ok() {
        let mut builder = TreeBuilder::new();
        // Real, unmodified frameset-ok state: a `<body>` start tag
        // (driven through "after head", inside `bootstrap_in_body`)
        // already sets it to "not ok" per spec — the common real-world
        // case (any body content before `<frameset>` disqualifies it).
        let (_, body) = bootstrap_in_body(&mut builder);
        assert!(!builder.frameset_ok);

        builder.process_token(&start_tag("frameset"), pos());

        assert_eq!(builder.open_elements.current_node(), Some(body));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
    }

    #[test]
    fn after_head_frameset_inserts_and_switches_mode() {
        let mut builder = TreeBuilder::new();
        builder.process_token(&start_tag("head"), pos());
        builder.process_token(&end_tag("head"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::AfterHead);

        builder.process_token(&start_tag("frameset"), pos());

        let frameset = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(frameset).kind,
            NodeKind::Element {
                name: "frameset".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.insertion_mode, InsertionMode::InFrameset);
    }

    /// Drives a fresh builder into "in frameset" via a `<head></head>`
    /// then `<frameset>` — the "after head" path exercised just above.
    /// Returns the frameset element.
    fn bootstrap_in_frameset(builder: &mut TreeBuilder) -> NodeId {
        builder.process_token(&start_tag("head"), pos());
        builder.process_token(&end_tag("head"), pos());
        builder.process_token(&start_tag("frameset"), pos());
        builder.open_elements.current_node().unwrap()
    }

    #[test]
    fn in_frameset_nested_frameset_start_tag_is_inserted() {
        let mut builder = TreeBuilder::new();
        let outer = bootstrap_in_frameset(&mut builder);

        builder.process_token(&start_tag("frameset"), pos());

        let inner = builder.open_elements.current_node().unwrap();
        assert_ne!(inner, outer);
        assert_eq!(builder.document.parent(inner), Some(outer));
        assert_eq!(builder.insertion_mode, InsertionMode::InFrameset);
    }

    #[test]
    fn in_frameset_frame_start_tag_is_inserted_and_immediately_popped() {
        let mut builder = TreeBuilder::new();
        let frameset = bootstrap_in_frameset(&mut builder);

        builder.process_token(&start_tag("frame"), pos());

        // Popped immediately: `frameset` (not `frame`) is current again.
        assert_eq!(builder.open_elements.current_node(), Some(frameset));
        let children: Vec<_> = builder.document.children(frameset).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Element {
                name: "frame".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn in_frameset_end_tag_pops_and_switches_to_after_frameset() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_frameset(&mut builder);

        builder.process_token(&end_tag("frameset"), pos());

        // Current node (`html`) is no longer a `frameset` element, so
        // the mode switches — the outermost `</frameset>` case.
        assert_eq!(builder.insertion_mode, InsertionMode::AfterFrameset);
    }

    #[test]
    fn in_frameset_nested_end_tag_stays_in_frameset_mode() {
        let mut builder = TreeBuilder::new();
        let outer = bootstrap_in_frameset(&mut builder);
        builder.process_token(&start_tag("frameset"), pos());

        builder.process_token(&end_tag("frameset"), pos());

        // Current node is `outer`, still a `frameset` element — mode
        // stays put per §13.2.6.4.18's own condition on this.
        assert_eq!(builder.open_elements.current_node(), Some(outer));
        assert_eq!(builder.insertion_mode, InsertionMode::InFrameset);
    }

    #[test]
    fn in_frameset_whitespace_is_inserted_non_whitespace_is_ignored() {
        let mut builder = TreeBuilder::new();
        let frameset = bootstrap_in_frameset(&mut builder);

        builder.process_token(&TokenKind::Character(' '), pos());
        builder.process_token(&TokenKind::Character('x'), pos());

        let children: Vec<_> = builder.document.children(frameset).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: " ".to_owned()
            }
        );
    }

    #[test]
    fn after_frameset_end_tag_html_switches_to_after_after_frameset() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_frameset(&mut builder);
        builder.process_token(&end_tag("frameset"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::AfterFrameset);

        builder.process_token(&end_tag("html"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::AfterAfterFrameset);
    }

    #[test]
    fn after_after_frameset_comment_is_appended_to_the_document() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_frameset(&mut builder);
        builder.process_token(&end_tag("frameset"), pos());
        builder.process_token(&end_tag("html"), pos());
        assert_eq!(builder.insertion_mode, InsertionMode::AfterAfterFrameset);
        let root = builder.document.root();

        builder.process_token(&TokenKind::Comment("hi".to_owned()), pos());

        let children: Vec<_> = builder.document.children(root).collect();
        assert_eq!(
            builder.document.node(*children.last().unwrap()).kind,
            NodeKind::Comment {
                content: "hi".to_owned()
            }
        );
    }

    #[test]
    fn after_after_frameset_anything_else_is_ignored_not_reprocessed() {
        // Unlike "after after body"'s catch-all (switches to "in body"
        // and reprocesses), "after after frameset"'s is just "parse
        // error, ignore the token" — no mode switch, no reprocessing.
        let mut builder = TreeBuilder::new();
        bootstrap_in_frameset(&mut builder);
        builder.process_token(&end_tag("frameset"), pos());
        builder.process_token(&end_tag("html"), pos());
        let current_before = builder.open_elements.current_node();
        let stack_len_before = builder.open_elements.entries.len();

        builder.process_token(&start_tag("p"), pos());

        assert_eq!(builder.insertion_mode, InsertionMode::AfterAfterFrameset);
        assert_eq!(builder.open_elements.current_node(), current_before);
        assert_eq!(builder.open_elements.entries.len(), stack_len_before);
    }

    /// Drives a fresh builder into foreign content via `bootstrap_in_body`
    /// plus an `<svg>` start tag. Returns `(body, svg)`.
    fn bootstrap_in_svg(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        let (_, body) = bootstrap_in_body(builder);
        builder.process_token(&start_tag("svg"), pos());
        let svg = builder.open_elements.current_node().unwrap();
        (body, svg)
    }

    #[test]
    fn foreign_content_ordinary_svg_element_gets_svg_namespace() {
        let mut builder = TreeBuilder::new();
        let (_, svg) = bootstrap_in_svg(&mut builder);

        builder.process_token(&start_tag("circle"), pos());

        let circle = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(circle).kind,
            NodeKind::Element {
                name: "circle".to_owned(),
                namespace: Some(crate::tree_builder::SVG_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.document.children(svg).collect::<Vec<_>>(),
            vec![circle]
        );
    }

    #[test]
    fn foreign_content_svg_tag_name_is_case_fixed() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_svg(&mut builder);

        builder.process_token(&start_tag("foreignobject"), pos());

        let node = builder.open_elements.current_node().unwrap();
        let NodeKind::Element { name, .. } = &builder.document.node(node).kind else {
            unreachable!()
        };
        assert_eq!(name, "foreignObject");
    }

    #[test]
    fn foreign_content_svg_attribute_is_case_fixed() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_svg(&mut builder);

        builder.process_token(
            &start_tag_with_attrs("rect", &[("viewbox", "0 0 1 1")]),
            pos(),
        );

        let node = builder.open_elements.current_node().unwrap();
        let NodeKind::Element { attributes, .. } = &builder.document.node(node).kind else {
            unreachable!()
        };
        assert_eq!(attributes[0].name, "viewBox");
    }

    #[test]
    fn foreign_content_xlink_attribute_gets_namespaced() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_svg(&mut builder);

        builder.process_token(&start_tag_with_attrs("use", &[("xlink:href", "#a")]), pos());

        let node = builder.open_elements.current_node().unwrap();
        let NodeKind::Element { attributes, .. } = &builder.document.node(node).kind else {
            unreachable!()
        };
        assert_eq!(attributes[0].name, "xlink:href");
        assert_eq!(
            attributes[0].namespace.as_deref(),
            Some(crate::tree_builder::XLINK_NAMESPACE)
        );
    }

    #[test]
    fn math_definitionurl_attribute_is_case_fixed() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);

        builder.process_token(
            &start_tag_with_attrs("math", &[("definitionurl", "x")]),
            pos(),
        );

        let math = builder.open_elements.current_node().unwrap();
        let NodeKind::Element { attributes, .. } = &builder.document.node(math).kind else {
            unreachable!()
        };
        assert_eq!(attributes[0].name, "definitionURL");
    }

    #[test]
    fn foreign_content_html_integration_point_allows_html_rules_inside() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_svg(&mut builder);
        builder.process_token(&start_tag("foreignobject"), pos());
        let foreign_object = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("div"), pos());

        let div = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(div).kind,
            NodeKind::Element {
                name: "div".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder
                .document
                .children(foreign_object)
                .collect::<Vec<_>>(),
            vec![div]
        );
    }

    #[test]
    fn foreign_content_mathml_text_integration_point_allows_html_start_tags() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("math"), pos());
        builder.process_token(&start_tag("mi"), pos());
        let mi = builder.open_elements.current_node().unwrap();

        builder.process_token(&start_tag("b"), pos());

        let b = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(b).kind,
            NodeKind::Element {
                name: "b".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(builder.document.children(mi).collect::<Vec<_>>(), vec![b]);
    }

    #[test]
    fn foreign_content_mathml_text_integration_point_still_treats_mglyph_as_foreign() {
        let mut builder = TreeBuilder::new();
        bootstrap_in_body(&mut builder);
        builder.process_token(&start_tag("math"), pos());
        builder.process_token(&start_tag("mi"), pos());

        builder.process_token(&start_tag("mglyph"), pos());

        let mglyph = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(mglyph).kind,
            NodeKind::Element {
                name: "mglyph".to_owned(),
                namespace: Some(crate::tree_builder::MATHML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    #[test]
    fn foreign_content_block_element_start_tag_pops_out_and_reprocesses_as_html() {
        let mut builder = TreeBuilder::new();
        let (body, svg) = bootstrap_in_svg(&mut builder);

        builder.process_token(&start_tag("b"), pos());

        // <b> is in the escape list: pops back out of <svg> entirely
        // and gets processed as ordinary HTML content instead.
        assert!(!builder.open_elements.contains(svg));
        let b = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(b).kind,
            NodeKind::Element {
                name: "b".to_owned(),
                namespace: Some(crate::tree_builder::HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
        assert_eq!(
            builder.document.children(body).collect::<Vec<_>>(),
            vec![svg, b]
        );
    }

    #[test]
    fn foreign_content_end_tag_p_pops_out_of_foreign_content() {
        let mut builder = TreeBuilder::new();
        let (_, svg) = bootstrap_in_svg(&mut builder);

        builder.process_token(&end_tag("p"), pos());

        assert!(!builder.open_elements.contains(svg));
        assert_eq!(builder.insertion_mode, InsertionMode::InBody);
    }

    #[test]
    fn foreign_content_nul_character_becomes_replacement_character() {
        let mut builder = TreeBuilder::new();
        let (_, svg) = bootstrap_in_svg(&mut builder);

        builder.process_token(&TokenKind::Character('\0'), pos());

        let children: Vec<_> = builder.document.children(svg).collect();
        assert_eq!(
            builder.document.node(children[0]).kind,
            NodeKind::Text {
                content: "\u{FFFD}".to_owned()
            }
        );
    }

    #[test]
    fn foreign_content_any_other_end_tag_pops_through_a_matching_element() {
        let mut builder = TreeBuilder::new();
        let (_, svg) = bootstrap_in_svg(&mut builder);
        builder.process_token(&start_tag("circle"), pos());
        let circle = builder.open_elements.current_node().unwrap();

        builder.process_token(&end_tag("circle"), pos());

        assert!(!builder.open_elements.contains(circle));
        assert_eq!(builder.open_elements.current_node(), Some(svg));
    }

    #[test]
    fn foreign_content_math_inside_svg_keeps_the_svg_namespace() {
        // <math> is only special-cased by "in body"'s own dispatch,
        // which never runs while already inside foreign content — a
        // literal <math> tag nested inside <svg> (not at an
        // integration point) is just "any other start tag", inserted
        // into the *current* (SVG) namespace like anything else.
        let mut builder = TreeBuilder::new();
        bootstrap_in_svg(&mut builder);

        builder.process_token(&start_tag("math"), pos());

        let math = builder.open_elements.current_node().unwrap();
        assert_eq!(
            builder.document.node(math).kind,
            NodeKind::Element {
                name: "math".to_owned(),
                namespace: Some(crate::tree_builder::SVG_NAMESPACE.to_owned()),
                attributes: vec![],
            }
        );
    }

    /// Drives a fresh builder through `<select><button><selectedcontent>
    /// </button>` — the common prefix of every `<selectedcontent>` test
    /// below (mirrors html5lib-tests' `webkit02.dat`#44-47). Returns
    /// `(select, selectedcontent)`.
    fn bootstrap_select_with_selectedcontent(builder: &mut TreeBuilder) -> (NodeId, NodeId) {
        bootstrap_in_body(builder);
        builder.process_token(&start_tag("select"), pos());
        let select = builder.open_elements.current_node().unwrap();
        builder.process_token(&start_tag("button"), pos());
        builder.process_token(&start_tag("selectedcontent"), pos());
        let selectedcontent = builder.open_elements.current_node().unwrap();
        builder.process_token(&end_tag("button"), pos());
        (select, selectedcontent)
    }

    fn text_content_of(builder: &TreeBuilder, node: NodeId) -> String {
        builder
            .document
            .children(node)
            .map(|child| match &builder.document.node(child).kind {
                NodeKind::Text { content } => content.clone(),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn second_option_start_tag_clones_the_default_selected_first_option() {
        // html5lib-tests webkit02.dat#44/#46: neither `<option>` has
        // `selected` — the first one is selected by default, and gets
        // mirrored into `<selectedcontent>` when the second `<option>`
        // start tag implicitly closes (pops) it.
        let mut builder = TreeBuilder::new();
        let (_, selectedcontent) = bootstrap_select_with_selectedcontent(&mut builder);

        builder.process_token(&start_tag("option"), pos());
        builder.process_token(&TokenKind::Character('X'), pos());
        builder.process_token(&start_tag("option"), pos());
        builder.process_token(&TokenKind::Character('Y'), pos());

        assert_eq!(text_content_of(&builder, selectedcontent), "X");
    }

    #[test]
    fn stop_parsing_reclones_a_later_option_that_becomes_selected() {
        // html5lib-tests webkit02.dat#47: the second (`selected`)
        // `<option>` is still open when parsing ends — never popped by
        // a following sibling, only by `stop_parsing`'s final
        // pop-everything step, which must still run the clone hook.
        let mut builder = TreeBuilder::new();
        let (_, selectedcontent) = bootstrap_select_with_selectedcontent(&mut builder);

        builder.process_token(&start_tag("option"), pos());
        builder.process_token(&TokenKind::Character('X'), pos());
        builder.process_token(&start_tag_with_attrs("option", &[("selected", "")]), pos());
        builder.process_token(&TokenKind::Character('Y'), pos());
        // Before EOF: the second `<option>` was never popped, so the
        // first clone (from popping the first `<option>`) is still the
        // last thing that happened.
        assert_eq!(text_content_of(&builder, selectedcontent), "X");

        builder.stop_parsing();

        assert_eq!(text_content_of(&builder, selectedcontent), "Y");
    }

    #[test]
    fn selectedcontent_clone_is_a_deep_copy_not_a_reference() {
        let mut builder = TreeBuilder::new();
        let (_, selectedcontent) = bootstrap_select_with_selectedcontent(&mut builder);

        builder.process_token(&start_tag("option"), pos());
        let option = builder.open_elements.current_node().unwrap();
        builder.process_token(&TokenKind::Character('X'), pos());

        builder.stop_parsing();

        let cloned_text = builder.document.children(selectedcontent).next().unwrap();
        let original_text = builder.document.children(option).next().unwrap();
        assert_ne!(cloned_text, original_text);
        assert_eq!(
            builder.document.node(cloned_text).kind,
            builder.document.node(original_text).kind
        );
    }
}
