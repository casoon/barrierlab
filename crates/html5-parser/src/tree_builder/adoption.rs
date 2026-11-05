use super::*;

impl TreeBuilder {
    /// "Any other end tag" (§13.2.6.4.7, "in body" insertion mode's
    /// default end-tag handling) — also the fallback the adoption agency
    /// algorithm itself runs (step 3) when no matching formatting
    /// element exists in the list of active formatting elements. Built
    /// now, ahead of "in body" itself (not yet implemented — see
    /// plan/03-tree-construction.md), because the adoption agency
    /// algorithm cannot be complete without it.
    pub(super) fn any_other_end_tag_in_body(&mut self, tag: &TagToken, position: Position) {
        let mut node = self
            .open_elements
            .current_node()
            .expect("any other end tag requires at least one open element");
        loop {
            let is_match = matches!(
                &self.document.node(node).kind,
                NodeKind::Element { name, namespace, .. }
                    if namespace.as_deref() == Some(HTML_NAMESPACE) && name == &tag.name
            );
            if is_match {
                self.with_option_pop_hook(|this| {
                    this.open_elements
                        .generate_implied_end_tags(&this.document, Some(tag.name.as_str()));
                    // "If node is not the current node, then this is a
                    // parse error" — parse-error-only, no tree-shape
                    // effect.
                    loop {
                        let popped = this.open_elements.pop();
                        if popped == Some(node) {
                            break;
                        }
                    }
                });
                return;
            }
            if is_special(&self.document, node) {
                // "Otherwise, if node is in the special category, then
                // this is a parse error; ignore the token, and return."
                self.error(ParseErrorKind::StrayEndTag, position);
                return;
            }
            node = self.open_elements.element_immediately_above(node).expect(
                "the html element is always in SPECIAL_CATEGORY, so this loop always \
                 returns via the is_special check above before running out of stack \
                 entries to walk above",
            );
        }
    }

    /// The adoption agency algorithm (§13.2.6.4.7) — handles misnested
    /// formatting element end tags (the canonical `<b><i>x</b>y</i>`
    /// case). `tag` is the end tag token being processed. Implemented as
    /// its own isolated step per plan/03-tree-construction.md (notorious
    /// for subtle bugs in other implementations), ahead of "in body"
    /// itself (not yet implemented) actually calling it.
    pub(super) fn adoption_agency_algorithm(&mut self, tag: &TagToken, position: Position) {
        let subject = tag.name.as_str();

        // Step 2: the common, non-misnested case — the current node
        // already *is* the element being closed, and isn't itself an
        // active formatting element, so there's nothing to adopt.
        if let Some(current) = self.open_elements.current_node() {
            let is_subject = matches!(
                &self.document.node(current).kind,
                NodeKind::Element { name, namespace, .. }
                    if namespace.as_deref() == Some(HTML_NAMESPACE) && name == subject
            );
            let is_active_formatting_element =
                self.active_formatting_elements.entries.iter().any(
                    |entry| matches!(entry, FormattingEntry::Element(node) if *node == current),
                );
            if is_subject && !is_active_formatting_element {
                self.open_elements.pop();
                return;
            }
        }

        // Steps 3-4: the outer loop, capped at 8 iterations.
        for _ in 0..8 {
            // Step 4.3: the last formatting element with this tag name,
            // searching only entries after the last marker (or from the
            // start of the list, if there is none).
            let marker_boundary = self
                .active_formatting_elements
                .entries
                .iter()
                .rposition(|entry| matches!(entry, FormattingEntry::Marker))
                .map_or(0, |marker_index| marker_index + 1);
            let formatting_element = self.active_formatting_elements.entries[marker_boundary..]
                .iter()
                .rev()
                .find_map(|entry| match entry {
                    FormattingEntry::Element(node) => {
                        let NodeKind::Element { name, .. } = &self.document.node(*node).kind else {
                            unreachable!("active formatting elements are always elements")
                        };
                        (name == subject).then_some(*node)
                    }
                    FormattingEntry::Marker => None,
                });
            let Some(formatting_element) = formatting_element else {
                self.any_other_end_tag_in_body(tag, position);
                return;
            };

            // Step 4.4: "If formattingElement is not in the stack of open
            // elements, then this is a parse error; remove the element
            // from the list, and return." — the element was already
            // closed implicitly, so this end tag has nothing left to close.
            if !self.open_elements.contains(formatting_element) {
                self.error(ParseErrorKind::StrayEndTag, position);
                let index = self
                    .active_formatting_elements
                    .entries
                    .iter()
                    .position(|entry| {
                        matches!(entry, FormattingEntry::Element(node) if *node == formatting_element)
                    })
                    .expect("formattingElement was just found in this list");
                self.active_formatting_elements.entries.remove(index);
                return;
            }

            // Step 4.5
            let NodeKind::Element {
                name: formatting_element_name,
                ..
            } = &self.document.node(formatting_element).kind
            else {
                unreachable!("formattingElement is always an element")
            };
            if !self
                .open_elements
                .has_element_in_scope(&self.document, formatting_element_name)
            {
                // "...then this is a parse error; return."
                self.error(ParseErrorKind::FormattingElementNotInScope, position);
                return;
            }

            // Step 4.6: "If formattingElement is not the current node,
            // this is a parse error." No tree-shape effect.
            if self.open_elements.current_node() != Some(formatting_element) {
                self.error(ParseErrorKind::MisnestedFormattingElement, position);
            }

            // Step 4.7
            let furthest_block = self
                .open_elements
                .topmost_special_element_below(&self.document, formatting_element);
            let Some(furthest_block) = furthest_block else {
                // Step 4.8: no furthest block — pop up to and including
                // formattingElement, drop it from the active formatting
                // elements list, and stop.
                loop {
                    let popped = self.open_elements.pop();
                    if popped == Some(formatting_element) {
                        break;
                    }
                }
                let index = self
                    .active_formatting_elements
                    .entries
                    .iter()
                    .position(|entry| {
                        matches!(entry, FormattingEntry::Element(node) if *node == formatting_element)
                    })
                    .expect("formattingElement was just found in this list");
                self.active_formatting_elements.entries.remove(index);
                return;
            };

            // Step 4.9
            let common_ancestor = self
                .open_elements
                .element_immediately_above(formatting_element)
                .expect("formattingElement always has an element above it (at minimum html)");

            // Step 4.10: the bookmark, tracked as a running index into
            // the active formatting elements list, kept in sync with
            // every removal/insertion this algorithm performs on that
            // list below.
            let mut bookmark = self
                .active_formatting_elements
                .entries
                .iter()
                .position(|entry| {
                    matches!(entry, FormattingEntry::Element(node) if *node == formatting_element)
                })
                .expect("formattingElement was just found in this list");

            // Steps 4.11-4.13: the inner loop.
            let mut node;
            let mut last_node = furthest_block;
            let mut node_above = self.open_elements.element_immediately_above(furthest_block);
            let mut inner_loop_counter = 0u32;
            loop {
                inner_loop_counter += 1;

                // Step 4.13.2: advance to the element above the previous
                // `node`, cached *before* this iteration's own possible
                // stack removal below (once removed, its neighbor can no
                // longer be looked up positionally).
                node = node_above.expect(
                    "walking upward from furthestBlock always reaches formattingElement, \
                     which sits strictly above it, before running out of stack entries",
                );
                node_above = self.open_elements.element_immediately_above(node);

                // Step 4.13.3
                if node == formatting_element {
                    break;
                }

                // Step 4.13.4
                if inner_loop_counter > 3
                    && let Some(index) = self.active_formatting_elements.entries.iter().position(
                        |entry| matches!(entry, FormattingEntry::Element(afe_node) if *afe_node == node),
                    )
                {
                    self.active_formatting_elements.entries.remove(index);
                    if index < bookmark {
                        bookmark -= 1;
                    }
                }

                // Step 4.13.5
                let node_afe_index = self.active_formatting_elements.entries.iter().position(
                    |entry| matches!(entry, FormattingEntry::Element(afe_node) if *afe_node == node),
                );
                let Some(node_afe_index) = node_afe_index else {
                    let stack_index = self
                        .open_elements
                        .entries
                        .iter()
                        .position(|&entry| entry == node)
                        .expect("node is currently on the stack of open elements");
                    self.open_elements.entries.remove(stack_index);
                    continue;
                };

                // Step 4.13.6
                let node_tag = self.tag_token_for(node);
                let new_node = self.create_element_for_token(&node_tag, HTML_NAMESPACE, None);
                self.active_formatting_elements.entries[node_afe_index] =
                    FormattingEntry::Element(new_node);
                let stack_index = self
                    .open_elements
                    .entries
                    .iter()
                    .position(|&entry| entry == node)
                    .expect("node is currently on the stack of open elements");
                self.open_elements.entries[stack_index] = new_node;

                // Step 4.13.7
                if last_node == furthest_block {
                    bookmark = node_afe_index + 1;
                }

                // Steps 4.13.8-4.13.9 — `append_child` detaches lastNode
                // from wherever it's currently attached first (see
                // `Document::insert_before`'s doc comment).
                self.document.append_child(new_node, last_node);
                last_node = new_node;
            }

            // Steps 4.14-4.17: "the adjusted insertion location given
            // commonAncestor" is exactly `appropriate_place_for_inserting_a_node`
            // (§13.2.6.1) — the same helper `insert_html_element` uses,
            // foster parenting included.
            let location = self.appropriate_place_for_inserting_a_node(Some(common_ancestor));

            // Step 4.18
            self.document.remove(last_node);

            // Step 4.19: all four conditions transcribed literally, even
            // though in this single-pass parser `target` is never the
            // Document node and `lastNode` can't practically end up an
            // ancestor of `target` — matching this crate's established
            // fidelity-over-pruning stance for spec-mandated but
            // practically unreachable guards (see foster parenting's
            // "table has no parent" fallback).
            let last_node_parent_is_null = self.document.parent(last_node).is_none();
            let last_node_is_not_ancestor_of_target = !self
                .document
                .is_inclusive_ancestor(last_node, location.parent);
            let target_is_not_document_with_element_child =
                !(matches!(self.document.node(location.parent).kind, NodeKind::Document)
                    && self.document.children(location.parent).any(|child| {
                        matches!(self.document.node(child).kind, NodeKind::Element { .. })
                    }));
            let ref_node_is_null_or_its_parent_is_target = match location.before {
                Some(reference) => self.document.parent(reference) == Some(location.parent),
                None => true,
            };
            if last_node_parent_is_null
                && last_node_is_not_ancestor_of_target
                && target_is_not_document_with_element_child
                && ref_node_is_null_or_its_parent_is_target
            {
                self.document
                    .insert_before(location.parent, location.before, last_node);
            }

            // Step 4.20
            let formatting_element_tag = self.tag_token_for(formatting_element);
            let new_formatting_element =
                self.create_element_for_token(&formatting_element_tag, HTML_NAMESPACE, None);

            // Step 4.21
            let furthest_block_children: Vec<_> = self.document.children(furthest_block).collect();
            for child in furthest_block_children {
                self.document.append_child(new_formatting_element, child);
            }

            // Step 4.22
            self.document
                .append_child(furthest_block, new_formatting_element);

            // Step 4.23
            let formatting_element_afe_index = self
                .active_formatting_elements
                .entries
                .iter()
                .position(|entry| {
                    matches!(entry, FormattingEntry::Element(node) if *node == formatting_element)
                })
                .expect("formattingElement's own entry is never touched by the inner loop");
            self.active_formatting_elements
                .entries
                .remove(formatting_element_afe_index);
            if formatting_element_afe_index < bookmark {
                bookmark -= 1;
            }
            self.active_formatting_elements
                .entries
                .insert(bookmark, FormattingEntry::Element(new_formatting_element));

            // Step 4.24
            let formatting_element_stack_index = self
                .open_elements
                .entries
                .iter()
                .position(|&entry| entry == formatting_element)
                .expect("formattingElement is still on the stack of open elements");
            self.open_elements
                .entries
                .remove(formatting_element_stack_index);
            let furthest_block_stack_index = self
                .open_elements
                .entries
                .iter()
                .position(|&entry| entry == furthest_block)
                .expect("furthestBlock is still on the stack of open elements");
            self.open_elements
                .entries
                .insert(furthest_block_stack_index + 1, new_formatting_element);
        }
    }
}
