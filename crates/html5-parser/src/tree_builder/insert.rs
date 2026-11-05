use super::*;

impl TreeBuilder {
    /// "The appropriate place for inserting a node" (§13.2.6.1), given an
    /// optional override target (defaults to the current node).
    pub(super) fn appropriate_place_for_inserting_a_node(
        &self,
        override_target: Option<NodeId>,
    ) -> InsertionLocation {
        let target = override_target.unwrap_or_else(|| {
            self.open_elements
                .current_node()
                .expect("inserting a node requires a current node on the stack")
        });

        let location = if self.foster_parenting && self.is_foster_parenting_target(target) {
            self.foster_parenting_location()
        } else {
            InsertionLocation {
                parent: target,
                before: None,
            }
        };

        self.redirect_into_template_contents(location)
    }

    /// §13.2.6.1's final step: "If [the] adjusted insertion location is
    /// inside a `template` element" — redirect into that element's
    /// template contents instead, applied uniformly after either of the
    /// two branches above (including foster parenting's own "there is a
    /// last template" case, which lands on the template element itself
    /// the same way the plain, non-foster-parenting path can). This
    /// crate has no "insertion target" (part of the `for`-attribute
    /// content-patching feature — out of scope, see README's Known
    /// limitations), so the spec's own `target is not null` branch never
    /// applies: always "inside the template's contents, after its last
    /// child".
    pub(super) fn redirect_into_template_contents(
        &self,
        location: InsertionLocation,
    ) -> InsertionLocation {
        if !self.node_has_html_name(location.parent, "template") {
            return location;
        }
        InsertionLocation {
            parent: self.template_contents(location.parent),
            before: None,
        }
    }

    /// True for the element types whose presence as the insertion target
    /// (while foster parenting is enabled) triggers foster parenting
    /// itself: `table`, `tbody`, `tfoot`, `thead`, `tr` (§13.2.6.1).
    pub(super) fn is_foster_parenting_target(&self, node: NodeId) -> bool {
        let NodeKind::Element {
            name, namespace, ..
        } = &self.document.node(node).kind
        else {
            return false;
        };
        namespace.as_deref() == Some(HTML_NAMESPACE)
            && matches!(name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr")
    }

    /// §13.2.6.1's foster-parenting substeps.
    pub(super) fn foster_parenting_location(&self) -> InsertionLocation {
        let last_template =
            self.open_elements
                .last_matching(&self.document, HTML_NAMESPACE, "template");
        let last_table = self
            .open_elements
            .last_matching(&self.document, HTML_NAMESPACE, "table");

        if let Some(last_template) = last_template {
            let use_template = match last_table {
                None => true,
                Some(last_table) => self.open_elements.is_lower(last_template, last_table),
            };
            if use_template {
                // Targets the template *element* itself here — the
                // caller (`appropriate_place_for_inserting_a_node`)
                // redirects into its template contents afterward, the
                // same generic step it applies to every other branch.
                return InsertionLocation {
                    parent: last_template,
                    before: None,
                };
            }
        }

        let Some(last_table) = last_table else {
            // No table at all: insert inside the first stack element
            // (the `html` element). Spec calls this "the fragment case",
            // but it's reachable in ordinary (non-fragment) parsing too,
            // wherever the algorithm's other branches don't apply.
            let html = self
                .open_elements
                .topmost()
                .expect("foster parenting requires at least the html element on the stack");
            return InsertionLocation {
                parent: html,
                before: None,
            };
        };

        if let Some(parent) = self.document.parent(last_table) {
            return InsertionLocation {
                parent,
                before: Some(last_table),
            };
        }

        // last_table has no parent: unreachable in this single-pass,
        // non-scripting parser (nothing ever removes a node from its
        // parent once inserted), but implemented for spec fidelity.
        let previous_element = self
            .open_elements
            .element_immediately_above(last_table)
            .expect("a table with no parent must still have something above it on the stack");
        InsertionLocation {
            parent: previous_element,
            before: None,
        }
    }

    /// "Create an element for a token" (§13.2.6.1), simplified: no custom
    /// elements, no speculative parsing, no form-association, no `xmlns`
    /// validation — none of those apply without script execution or a
    /// live DOM, which are outside this crate's Step-1 scope. What
    /// remains: build a [`NodeKind::Element`] from the tag's name,
    /// `namespace`, and attributes (converted via
    /// [`document::Attribute`](Attribute)'s `From<tokenizer::Attribute>`
    /// for HTML-namespace elements; via
    /// [`adjust_attributes_for_foreign_element`] for everything else —
    /// "adjust MathML attributes"/"adjust SVG attributes"/"adjust
    /// foreign attributes" (§13.2.6.1/.5), called by every real
    /// caller that inserts a non-HTML element: "in body"'s `<math>`/
    /// `<svg>` start-tag rules and foreign content's "any other start
    /// tag").
    pub(super) fn create_element_for_token(
        &mut self,
        tag: &TagToken,
        namespace: &str,
        position: Option<Position>,
    ) -> NodeId {
        let attributes = if namespace == HTML_NAMESPACE {
            tag.attributes
                .iter()
                .cloned()
                .map(Attribute::from)
                .collect()
        } else {
            adjust_attributes_for_foreign_element(tag, namespace)
        };
        let element = self.document.new_node(
            NodeKind::Element {
                name: tag.name.clone(),
                namespace: Some(namespace.to_owned()),
                attributes,
            },
            position,
        );
        if namespace == HTML_NAMESPACE && tag.name == "template" {
            // Every `template` element gets its "template contents"
            // fragment created alongside it (not deferred to whichever
            // insertion-mode rule happens to insert content into it) —
            // see `NodeKind::DocumentFragment`'s doc comment and
            // `appropriate_place_for_inserting_a_node`'s redirect step,
            // which is what actually routes real children here.
            let content = self.document.new_node(NodeKind::DocumentFragment, None);
            self.document.append_child(element, content);
        }
        element
    }

    /// The `template` element's "template contents" fragment — its sole
    /// real tree child, created alongside it in
    /// [`create_element_for_token`](Self::create_element_for_token).
    pub(super) fn template_contents(&self, template: NodeId) -> NodeId {
        self.document.children(template).next().expect(
            "a template element always has its template-contents fragment as its first (and \
             only, until real content is inserted into it) tree child",
        )
    }

    /// "Insert an element at the adjusted insertion location" (§13.2.6.1),
    /// simplified: no custom-element-reaction-queue bookkeeping (no
    /// custom elements without script execution). Keeps the one
    /// tree-shape-relevant guard: a second root element is never
    /// inserted as a child of the document node itself.
    pub(super) fn insert_element_at_adjusted_insertion_location(
        &mut self,
        element: NodeId,
        location: InsertionLocation,
    ) {
        if location.parent == self.document.root()
            && self.document.children(location.parent).next().is_some()
        {
            return;
        }
        self.document
            .insert_before(location.parent, location.before, element);
    }

    /// "Insert a foreign element" (§13.2.6.1). `only_add_to_element_stack`
    /// corresponds to the spec parameter of the same name — used by
    /// insertion-mode rules that build an element without actually
    /// placing it in the tree (not yet needed by anything implemented so
    /// far, but part of the algorithm's real signature).
    pub(super) fn insert_foreign_element(
        &mut self,
        tag: &TagToken,
        namespace: &str,
        only_add_to_element_stack: bool,
        position: Option<Position>,
    ) -> NodeId {
        let location = self.appropriate_place_for_inserting_a_node(None);
        let element = self.create_element_for_token(tag, namespace, position);
        if !only_add_to_element_stack {
            self.insert_element_at_adjusted_insertion_location(element, location);
        }
        self.open_elements.push(element);
        element
    }

    /// "Insert an HTML element" (§13.2.6.1): insert a foreign element in
    /// the HTML namespace, actually placing it in the tree.
    pub(super) fn insert_html_element(
        &mut self,
        tag: &TagToken,
        position: Option<Position>,
    ) -> NodeId {
        self.insert_foreign_element(tag, HTML_NAMESPACE, false, position)
    }

    /// §13.2.6.2's "generic raw text element parsing algorithm"/"generic
    /// RCDATA element parsing algorithm" — always invoked for a start tag
    /// token (`<title>`/`<textarea>` → RCDATA, `<style>`/`<xmp>`/
    /// `<iframe>`/`<noembed>`/`<noframes>` → RAWTEXT). This crate's
    /// tree-builder never holds the tokenizer itself (kept decoupled —
    /// tree_builder.rs only depends on tokenizer's plain data types
    /// otherwise), so instead of switching it directly, this returns the
    /// [`ExternalState`] the caller must feed to
    /// [`Tokenizer::switch_to`](crate::tokenizer::Tokenizer::switch_to)
    /// right after this returns.
    pub(super) fn generic_text_element_parsing_algorithm(
        &mut self,
        tag: &TagToken,
        kind: GenericTextElementKind,
        position: Option<Position>,
    ) -> (NodeId, ExternalState) {
        let element = self.insert_html_element(tag, position);
        let external_state = match kind {
            GenericTextElementKind::RawText => ExternalState::RawText,
            GenericTextElementKind::Rcdata => ExternalState::RcData,
            GenericTextElementKind::Script => ExternalState::ScriptData,
        };
        self.original_insertion_mode = Some(self.insertion_mode);
        self.insertion_mode = InsertionMode::Text;
        (element, external_state)
    }

    /// Rebuilds an equivalent `TagToken` from an already-inserted
    /// element's current (immutable-once-parsed) name and attributes —
    /// used by "reconstruct the active formatting elements" to recreate
    /// a formatting element via [`insert_html_element`](Self::insert_html_element).
    pub(super) fn tag_token_for(&self, node: NodeId) -> TagToken {
        let NodeKind::Element {
            name, attributes, ..
        } = &self.document.node(node).kind
        else {
            panic!("tag_token_for called on a non-element node")
        };
        TagToken {
            name: name.clone(),
            self_closing: false,
            attributes: attributes
                .iter()
                .map(|attribute| crate::tokenizer::Attribute {
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                })
                .collect(),
        }
    }

    /// "Insert a character" (§13.2.6.1): appends to the last child if
    /// it's already a text node, otherwise creates a new one. Also
    /// implements the "insertion location is inside a Document node"
    /// guard (character data is silently dropped there — the DOM never
    /// allows Text children of the Document node).
    pub(super) fn insert_character(&mut self, c: char, position: Option<Position>) {
        let location = self.appropriate_place_for_inserting_a_node(None);
        if location.parent == self.document.root() {
            return;
        }
        // "If there is a Text node immediately before insertionLocation" —
        // that's the previous sibling of `before` (append case: `before`
        // is None, so it's the parent's last child instead).
        let node_immediately_before = match location.before {
            Some(before) => self.document.prev_sibling(before),
            None => self.document.last_child(location.parent),
        };
        if let Some(node) = node_immediately_before
            && let NodeKind::Text { content } = &mut self.document.node_mut(node).kind
        {
            content.push(c);
            return;
        }
        let text = self.document.new_node(
            NodeKind::Text {
                content: c.to_string(),
            },
            position,
        );
        self.document
            .insert_before(location.parent, location.before, text);
    }

    /// "Insert a comment" (§13.2.6.1).
    pub(super) fn insert_comment(&mut self, data: String, position: Option<Position>) {
        let location = self.appropriate_place_for_inserting_a_node(None);
        let comment = self
            .document
            .new_node(NodeKind::Comment { content: data }, position);
        self.document
            .insert_before(location.parent, location.before, comment);
    }

    /// "Insert a processing instruction" (§13.2.6.1).
    pub(super) fn insert_processing_instruction(
        &mut self,
        target: String,
        data: String,
        position: Option<Position>,
    ) {
        let location = self.appropriate_place_for_inserting_a_node(None);
        let pi = self
            .document
            .new_node(NodeKind::ProcessingInstruction { target, data }, position);
        self.document
            .insert_before(location.parent, location.before, pi);
    }

    /// "Insert a comment as the last child of the Document object"
    /// (used by the "initial" and "before html" insertion modes'
    /// comment-token rules) — distinct from the generic
    /// [`insert_comment`](Self::insert_comment) (§13.2.6.1), which
    /// targets the *current node* instead. Both modes run before any
    /// element (hence no current node) exists on the stack of open
    /// elements, so the generic algorithm doesn't apply here.
    pub(super) fn append_comment_to_document(&mut self, data: String, position: Position) {
        let comment = self
            .document
            .new_node(NodeKind::Comment { content: data }, Some(position));
        let root = self.document.root();
        self.document.append_child(root, comment);
    }

    /// "Insert a processing instruction as the last child of the
    /// Document object" — the PI-token counterpart to
    /// [`append_comment_to_document`](Self::append_comment_to_document).
    pub(super) fn append_processing_instruction_to_document(
        &mut self,
        target: String,
        data: String,
        position: Position,
    ) {
        let pi = self.document.new_node(
            NodeKind::ProcessingInstruction { target, data },
            Some(position),
        );
        let root = self.document.root();
        self.document.append_child(root, pi);
    }

    /// "Insert a comment as the last child of the first element in the
    /// stack of open elements (the html element)" — "after body"'s
    /// (§13.2.6.4.17) comment-token rule.
    pub(super) fn append_comment_to_html_element(&mut self, data: String, position: Position) {
        let html = self
            .open_elements
            .topmost()
            .expect("\"after body\" always runs with html on the stack of open elements");
        let comment = self
            .document
            .new_node(NodeKind::Comment { content: data }, Some(position));
        self.document.append_child(html, comment);
    }

    /// The PI-token counterpart to
    /// [`append_comment_to_html_element`](Self::append_comment_to_html_element).
    pub(super) fn append_processing_instruction_to_html_element(
        &mut self,
        target: String,
        data: String,
        position: Position,
    ) {
        let html = self
            .open_elements
            .topmost()
            .expect("\"after body\" always runs with html on the stack of open elements");
        let pi = self.document.new_node(
            NodeKind::ProcessingInstruction { target, data },
            Some(position),
        );
        self.document.append_child(html, pi);
    }

    /// "In body"'s rule for a start tag whose tag name is "html"
    /// (§13.2.6.4.7): merge any attributes not already present onto the
    /// existing (topmost) `html` element, unless a `template` element
    /// is currently open. Several other insertion modes ("before head",
    /// "after head", and others not yet implemented) delegate `<html>`
    /// start tags to "in body" wholesale — but a token that *is* an
    /// html start tag can only ever hit this one "in body" rule, so
    /// it's implemented standalone here rather than waiting for the
    /// rest of "in body" to exist.
    pub(super) fn merge_attributes_onto_html_element(
        &mut self,
        tag: &TagToken,
        position: Position,
    ) {
        // "Parse error." — unconditional, ahead of the template check
        // that only decides whether the attributes are merged.
        self.error(ParseErrorKind::StrayStartTag, position);
        if self.has_template_on_stack() {
            return;
        }
        let html = self
            .open_elements
            .topmost()
            .expect("this rule never runs before the html element has been pushed");
        self.merge_new_attributes_onto(html, tag);
    }

    /// Adds each of `tag`'s attributes to `node` that isn't already
    /// present there by name — the "for each attribute on the token,
    /// check to see if the attribute is already present [...], if it is
    /// not, add [it]" pattern shared by "in body"'s `<html>` (via
    /// [`merge_attributes_onto_html_element`](Self::merge_attributes_onto_html_element))
    /// and `<body>` start-tag rules (§13.2.6.4.7).
    pub(super) fn merge_new_attributes_onto(&mut self, node: NodeId, tag: &TagToken) {
        let NodeKind::Element { attributes, .. } = &mut self.document.node_mut(node).kind else {
            unreachable!("merge_new_attributes_onto is always called with an element node")
        };
        for attribute in &tag.attributes {
            if !attributes
                .iter()
                .any(|existing| existing.name == attribute.name)
            {
                attributes.push(Attribute::from(attribute.clone()));
            }
        }
    }

    /// The void-element group's shared shape (§13.2.6.4.7:
    /// `area`/`br`/`embed`/`img`/`keygen`/`wbr`, and the `<br>`
    /// end-tag-treated-as-start-tag rewrite): reconstruct the active
    /// formatting elements, insert an HTML element for the token,
    /// immediately pop it back off, acknowledge the token's self-closing
    /// flag, and mark the frameset-ok flag "not ok".
    pub(super) fn insert_void_element(&mut self, tag: &TagToken, position: Option<Position>) {
        self.reconstruct_the_active_formatting_elements();
        self.insert_html_element(tag, position);
        self.open_elements.pop();
        self.acknowledge_self_closing_flag();
        self.frameset_ok = false;
    }
}
