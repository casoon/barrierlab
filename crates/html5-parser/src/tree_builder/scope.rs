use super::*;

impl TreeBuilder {
    /// True if `node` is an HTML-namespace element with tag name `name`
    /// — the "node is a(n) X element" check that recurs throughout
    /// §13.2.6.4.7 ("in body") and elsewhere.
    pub(super) fn node_has_html_name(&self, node: NodeId, name: &str) -> bool {
        self.node_has_namespace_and_name(node, HTML_NAMESPACE, name)
    }

    /// True if `node` is an element in `namespace` with tag name `name`
    /// — the namespace-generalized form of
    /// [`node_has_html_name`](Self::node_has_html_name), needed for
    /// foreign-content checks (§13.2.6.5) that care about MathML/SVG
    /// elements specifically.
    pub(super) fn node_has_namespace_and_name(
        &self,
        node: NodeId,
        namespace: &str,
        name: &str,
    ) -> bool {
        matches!(
            &self.document.node(node).kind,
            NodeKind::Element { name: node_name, namespace: node_namespace, .. }
                if node_namespace.as_deref() == Some(namespace) && node_name == name
        )
    }

    /// True if `node` is an HTML-namespace element with an attribute
    /// named `attribute_name` (any value, including empty).
    pub(super) fn node_has_html_attribute(&self, node: NodeId, attribute_name: &str) -> bool {
        matches!(
            &self.document.node(node).kind,
            NodeKind::Element { namespace, attributes, .. }
                if namespace.as_deref() == Some(HTML_NAMESPACE)
                    && attributes.iter().any(|attribute| attribute.name == attribute_name)
        )
    }

    /// True if `node` is an element in the HTML namespace, regardless
    /// of tag name — the tree-construction dispatcher's (§13.2.6)
    /// primary foreign-vs-HTML-content test.
    pub(super) fn is_html_namespace_element(&self, node: NodeId) -> bool {
        matches!(
            &self.document.node(node).kind,
            NodeKind::Element { namespace, .. } if namespace.as_deref() == Some(HTML_NAMESPACE)
        )
    }

    /// `node`'s namespace, if it's an element — owned rather than
    /// borrowed so callers can use it across a subsequent `&mut self`
    /// call (e.g. inserting a new element into that same namespace).
    pub(super) fn namespace_of(&self, node: NodeId) -> Option<String> {
        match &self.document.node(node).kind {
            NodeKind::Element { namespace, .. } => namespace.clone(),
            _ => None,
        }
    }

    /// The "adjusted current node" (§13.2.6): the context element
    /// substitution for the HTML fragment parsing algorithm never
    /// applies (this crate never does fragment parsing), so this is
    /// always just the current node.
    pub(super) fn adjusted_current_node(&self) -> Option<NodeId> {
        self.open_elements.current_node()
    }

    /// True if a `template` element is currently on the stack of open
    /// elements — used by the (simplified) `<template>` handling in "in
    /// head" and by [`merge_attributes_onto_html_element`](Self::merge_attributes_onto_html_element).
    pub(super) fn has_template_on_stack(&self) -> bool {
        self.open_elements
            .entries
            .iter()
            .any(|&node| self.node_has_html_name(node, "template"))
    }

    /// Pops elements off the stack of open elements until one whose tag
    /// name is in `names` (HTML namespace) has been popped — the "pop
    /// elements from the stack of open elements until an X element has
    /// been popped from the stack" pattern that recurs throughout "in
    /// body" (§13.2.6.4.7). Callers only invoke this once they've
    /// already confirmed (via a scope check) that a matching element
    /// genuinely exists on the stack, exactly mirroring every one of
    /// "in body"'s own end-tag rules.
    pub(super) fn pop_until_one_of_popped(&mut self, names: &[&str]) {
        loop {
            let popped = self.open_elements.pop();
            let is_match = popped.is_some_and(|node| {
                names
                    .iter()
                    .any(|&name| self.node_has_html_name(node, name))
            });
            if is_match {
                break;
            }
        }
    }

    /// Removes `node` from the stack of open elements, wherever it
    /// currently is — not necessarily the current node. Used where the
    /// spec says "remove node from the stack of open elements" rather
    /// than "pop".
    pub(super) fn remove_node_from_open_elements(&mut self, node: NodeId) {
        if let Some(index) = self.open_elements.entries.iter().position(|&n| n == node) {
            self.open_elements.entries.remove(index);
        }
    }

    /// The open-element check shared verbatim by "in body"'s
    /// end-of-file, `</body>` and `</html>` rules (§13.2.6.4.7): "If
    /// there is a node in the stack of open elements that is not either
    /// a dd element, a dt element, an li element, an optgroup element,
    /// an option element, a p element, an rb element, an rp element, an
    /// rt element, an rtc element, a tbody element, a td element, a
    /// tfoot element, a th element, a thead element, a tr element, the
    /// body element, or the html element, then this is a parse error."
    ///
    /// In practice: everything in this list either has an optional end
    /// tag or is one of the two implied document-structure elements, so
    /// anything *else* still being open at this point means the author
    /// left a real element unclosed.
    pub(super) fn has_unexpected_open_elements(&self) -> bool {
        const ALLOWED_UNCLOSED: &[&str] = &[
            "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc", "tbody", "td",
            "tfoot", "th", "thead", "tr", "body", "html",
        ];
        self.open_elements.entries.iter().any(|&node| {
            let NodeKind::Element {
                name, namespace, ..
            } = &self.document.node(node).kind
            else {
                return false;
            };
            namespace.as_deref() != Some(HTML_NAMESPACE)
                || !ALLOWED_UNCLOSED.contains(&name.as_str())
        })
    }

    /// "Close a p element" (§13.2.6.4.7): generate implied end tags
    /// except for `p` elements, then pop until a `p` element has been
    /// popped. Callers are expected to have already checked
    /// `has_element_in_button_scope("p")` first, per every one of this
    /// helper's actual call sites in the spec.
    ///
    /// Between those two steps sits the spec's own "If the current node
    /// is not a p element, then this is a parse error" — i.e. something
    /// other than the `p` (and other than the implied-end-tag set just
    /// popped) is still open and about to be closed implicitly.
    /// `position` is the position of the token that triggered the close.
    pub(super) fn close_a_p_element(&mut self, position: Position) {
        self.open_elements
            .generate_implied_end_tags(&self.document, Some("p"));
        let current_node_is_p = self
            .open_elements
            .current_node()
            .is_some_and(|node| self.node_has_html_name(node, "p"));
        if !current_node_is_p {
            self.error(ParseErrorKind::ImpliedEndTagWithUnclosedElements, position);
        }
        self.pop_until_one_of_popped(&["p"]);
    }

    /// True if `node` — a *specific* element, by identity rather than
    /// by tag name — is reachable from the current node without
    /// crossing a default-scope boundary. Needed by "in body"'s
    /// `</form>` rule (§13.2.6.4.7), which tracks a specific `form`
    /// element via the form element pointer rather than searching by
    /// name (there's normally at most one open `form` at a time, but
    /// the spec phrases the check node-identity-first regardless).
    pub(super) fn has_node_in_scope(&self, node: NodeId) -> bool {
        for &entry in self.open_elements.entries.iter().rev() {
            if entry == node {
                return true;
            }
            let NodeKind::Element {
                name, namespace, ..
            } = &self.document.node(entry).kind
            else {
                continue;
            };
            let namespace = namespace.as_deref().unwrap_or("");
            if element_type_matches(DEFAULT_SCOPE, namespace, name) {
                return false;
            }
        }
        false
    }

    /// "Clear the stack back to a table context" (§13.2.6.4.9) and its
    /// "table body"/"table row" siblings (§13.2.6.4.13/.14) all share
    /// this exact shape: pop while the current node's tag name isn't in
    /// `names`. Each of the three real callers passes its own fixed
    /// boundary list.
    pub(super) fn clear_stack_back_to_context(&mut self, names: &[&str]) {
        while let Some(current) = self.open_elements.current_node() {
            if names
                .iter()
                .any(|&name| self.node_has_html_name(current, name))
            {
                break;
            }
            self.open_elements.pop();
        }
    }

    /// "Clear the stack back to a table context" (§13.2.6.4.9).
    pub(super) fn clear_stack_back_to_a_table_context(&mut self) {
        self.clear_stack_back_to_context(&["table", "template", "html"]);
    }

    /// "Clear the stack back to a table body context" (§13.2.6.4.13).
    pub(super) fn clear_stack_back_to_a_table_body_context(&mut self) {
        self.clear_stack_back_to_context(&["tbody", "tfoot", "thead", "template", "html"]);
    }

    /// "Clear the stack back to a table row context" (§13.2.6.4.14).
    pub(super) fn clear_stack_back_to_a_table_row_context(&mut self) {
        self.clear_stack_back_to_context(&["tr", "template", "html"]);
    }

    /// "Reset the insertion mode appropriately" (§13.2.4.1): walks the
    /// stack of open elements from the current node upward, switching
    /// to whichever insertion mode that node's tag name implies. Used
    /// after popping back out of table-related structure (e.g. a
    /// misnested `<table>` end tag) to figure out where parsing
    /// continues.
    ///
    /// Two branches are deliberately omitted, both per this crate's
    /// scope decisions: the "node is a template element" case (no
    /// template insertion modes stack exists — `<template>` is a plain
    /// element — so this just falls
    /// through and keeps walking past it, same as any other
    /// unrecognized element) and the "node is a frameset element" case
    /// (this crate never creates a frameset element at all, so it can
    /// never match). The "fragment case" context-element substitution
    /// never applies either — this crate never does HTML fragment
    /// parsing.
    pub(super) fn reset_the_insertion_mode_appropriately(&mut self) {
        let mut last = false;
        let mut node = self.open_elements.current_node();
        loop {
            let current = node.expect(
                "the stack of open elements always contains at least html by the time this runs",
            );
            if self.open_elements.topmost() == Some(current) {
                last = true;
            }
            if !last
                && (self.node_has_html_name(current, "td")
                    || self.node_has_html_name(current, "th"))
            {
                self.insertion_mode = InsertionMode::InCell;
                return;
            }
            if self.node_has_html_name(current, "tr") {
                self.insertion_mode = InsertionMode::InRow;
                return;
            }
            if ["tbody", "thead", "tfoot"]
                .iter()
                .any(|&name| self.node_has_html_name(current, name))
            {
                self.insertion_mode = InsertionMode::InTableBody;
                return;
            }
            if self.node_has_html_name(current, "caption") {
                self.insertion_mode = InsertionMode::InCaption;
                return;
            }
            if self.node_has_html_name(current, "colgroup") {
                self.insertion_mode = InsertionMode::InColumnGroup;
                return;
            }
            if self.node_has_html_name(current, "table") {
                self.insertion_mode = InsertionMode::InTable;
                return;
            }
            if self.node_has_html_name(current, "template") {
                self.insertion_mode = *self
                    .stack_of_template_insertion_modes
                    .last()
                    .expect("a template element on the stack always has a matching entry here");
                return;
            }
            if !last && self.node_has_html_name(current, "head") {
                self.insertion_mode = InsertionMode::InHead;
                return;
            }
            if self.node_has_html_name(current, "body") {
                self.insertion_mode = InsertionMode::InBody;
                return;
            }
            // Only reachable in the fragment case per spec (this crate
            // never does fragment parsing) — a non-fragment document
            // can't nest anything requiring a stack walk (a `<table>`,
            // `<template>`, etc.) *inside* `<frameset>` content in the
            // first place, "in frameset"'s own rules reject it. Kept
            // for spec fidelity regardless, same stance as other
            // practically-unreachable-but-transcribed branches (e.g.
            // the adoption agency algorithm's step 4.19 guards).
            if self.node_has_html_name(current, "frameset") {
                self.insertion_mode = InsertionMode::InFrameset;
                return;
            }
            if self.node_has_html_name(current, "html") {
                self.insertion_mode = if self.head_element_pointer.is_none() {
                    InsertionMode::BeforeHead
                } else {
                    InsertionMode::AfterHead
                };
                return;
            }
            if last {
                self.insertion_mode = InsertionMode::InBody;
                return;
            }
            node = self.open_elements.element_immediately_above(current);
        }
    }
}
