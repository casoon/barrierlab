use super::*;

impl TreeBuilder {
    /// The "initial" insertion mode (§13.2.6.4.1).
    pub(super) fn process_token_initial(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if is_whitespace(*c) => TokenOutcome::Consumed(None),
            TokenKind::Comment(data) => {
                self.append_comment_to_document(data.clone(), position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.append_processing_instruction_to_document(
                    pi.target.clone(),
                    pi.data.clone(),
                    position,
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Doctype(doctype) => {
                // The parse-error checks (wrong name/public-identifier/
                // system-identifier shape) have no tree-shape effect —
                // omitted.
                let root = self.document.root();
                let has_doctype_or_element_child = self.document.children(root).any(|child| {
                    matches!(
                        self.document.node(child).kind,
                        NodeKind::Doctype { .. } | NodeKind::Element { .. }
                    )
                });
                if !has_doctype_or_element_child {
                    let doctype_node = self.document.new_node(
                        NodeKind::Doctype {
                            name: Some(doctype.name.clone().unwrap_or_default()),
                            public_identifier: Some(
                                doctype.public_identifier.clone().unwrap_or_default(),
                            ),
                            system_identifier: Some(
                                doctype.system_identifier.clone().unwrap_or_default(),
                            ),
                        },
                        Some(position),
                    );
                    self.document.append_child(root, doctype_node);
                }
                self.quirks_mode = determine_quirks_mode(doctype);
                self.insertion_mode = InsertionMode::BeforeHtml;
                TokenOutcome::Consumed(None)
            }
            _ => {
                // "Anything else": not an iframe srcdoc document (this
                // crate never parses srcdoc content), so set quirks
                // mode unconditionally, then switch and reprocess.
                self.quirks_mode = QuirksMode::Quirks;
                self.insertion_mode = InsertionMode::BeforeHtml;
                TokenOutcome::Reprocess
            }
        }
    }

    /// The "before html" insertion mode (§13.2.6.4.2).
    pub(super) fn process_token_before_html(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::Comment(data) => {
                self.append_comment_to_document(data.clone(), position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.append_processing_instruction_to_document(
                    pi.target.clone(),
                    pi.data.clone(),
                    position,
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Character(c) if is_whitespace(*c) => TokenOutcome::Consumed(None),
            TokenKind::StartTag(tag) if tag.name == "html" => {
                let element = self.create_element_for_token(tag, HTML_NAMESPACE, Some(position));
                let root = self.document.root();
                self.document.append_child(root, element);
                self.open_elements.push(element);
                self.insertion_mode = InsertionMode::BeforeHead;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(tag.name.as_str(), "head" | "body" | "html" | "br") =>
            {
                self.before_html_anything_else()
            }
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.before_html_anything_else(),
        }
    }

    /// "Before html"'s "anything else" case: synthesize an `html`
    /// element directly (bypassing the generic insertion algorithm,
    /// which requires a current node that doesn't exist yet), push it,
    /// switch to "before head", and reprocess.
    pub(super) fn before_html_anything_else(&mut self) -> TokenOutcome {
        let html = self.document.new_node(
            NodeKind::Element {
                name: "html".to_owned(),
                namespace: Some(HTML_NAMESPACE.to_owned()),
                attributes: vec![],
            },
            None,
        );
        let root = self.document.root();
        self.document.append_child(root, html);
        self.open_elements.push(html);
        self.insertion_mode = InsertionMode::BeforeHead;
        TokenOutcome::Reprocess
    }

    /// The "before head" insertion mode (§13.2.6.4.3).
    pub(super) fn process_token_before_head(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if is_whitespace(*c) => TokenOutcome::Consumed(None),
            TokenKind::Comment(data) => {
                self.insert_comment(data.clone(), Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.insert_processing_instruction(
                    pi.target.clone(),
                    pi.data.clone(),
                    Some(position),
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.merge_attributes_onto_html_element(tag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "head" => {
                let head = self.insert_html_element(tag, Some(position));
                self.head_element_pointer = Some(head);
                self.insertion_mode = InsertionMode::InHead;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(tag.name.as_str(), "head" | "body" | "html" | "br") =>
            {
                self.before_head_anything_else()
            }
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.before_head_anything_else(),
        }
    }

    /// "Before head"'s "anything else" case: insert a synthesized
    /// `head` start tag (no attributes), remember it via the head
    /// element pointer, switch to "in head", and reprocess.
    pub(super) fn before_head_anything_else(&mut self) -> TokenOutcome {
        let head_tag = TagToken {
            name: "head".to_owned(),
            self_closing: false,
            attributes: vec![],
        };
        let head = self.insert_html_element(&head_tag, None);
        self.head_element_pointer = Some(head);
        self.insertion_mode = InsertionMode::InHead;
        TokenOutcome::Reprocess
    }

    /// The "in head" insertion mode (§13.2.6.4.4). `<template>`'s
    /// classic content-fragment model is implemented (marker on the
    /// active formatting elements list, the stack of template insertion
    /// modes, `NodeKind::DocumentFragment` template contents — see
    /// plan/04-frameset.md's "Normative Grundlage" for where this was
    /// researched and plan/05-template.md for this piece specifically);
    /// declarative shadow DOM (`shadowrootmode`) and content patching
    /// (`for`) are not — see README's Known limitations, both require
    /// modeling shadow roots/custom element registries this crate has
    /// no other use for. Character-encoding sniffing (`<meta charset>`)
    /// and script-execution bookkeeping (parser document, force-async,
    /// already-started, `document.write()` re-entrancy) are entirely
    /// out of scope — this crate never decodes bytes itself or executes
    /// scripts.
    pub(super) fn process_token_in_head(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.insert_character(*c, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::Comment(data) => {
                self.insert_comment(data.clone(), Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.insert_processing_instruction(
                    pi.target.clone(),
                    pi.data.clone(),
                    Some(position),
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.merge_attributes_onto_html_element(tag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "base" | "basefont" | "bgsound" | "link" | "meta"
                ) =>
            {
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "title" => {
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::Rcdata,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "noscript" => {
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InHeadNoscript;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "noframes" | "style") => {
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::RawText,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "script" => {
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::Script,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::EndTag(tag) if tag.name == "head" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::AfterHead;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "body" | "html" | "br") => {
                self.in_head_anything_else()
            }
            TokenKind::StartTag(tag) if tag.name == "template" => {
                self.active_formatting_elements.push_marker();
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTemplate;
                self.stack_of_template_insertion_modes
                    .push(InsertionMode::InTemplate);
                // The `shadowrootmode`/`for`-attribute branches
                // (declarative shadow DOM, content-patching — out of
                // scope, see this function's doc comment) never apply:
                // always the "otherwise, insert an HTML element" path.
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "template" => {
                if !self.has_template_on_stack() {
                    // Parse error, ignore the token.
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags_thoroughly(&self.document);
                // The parse-error check ("current node is not a
                // template element") has no tree-shape effect. The
                // "insertion target"/marker cleanup is content-
                // patching's `for`-attribute bookkeeping — out of
                // scope, this crate never sets those in the first
                // place.
                self.pop_until_one_of_popped(&["template"]);
                self.active_formatting_elements.clear_up_to_last_marker();
                self.stack_of_template_insertion_modes.pop();
                self.reset_the_insertion_mode_appropriately();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "head" => {
                // "A start tag whose tag name is 'head': Parse error.
                // Ignore the token."
                self.error(ParseErrorKind::StrayStartTag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.in_head_anything_else(),
        }
    }

    /// "In head"'s "anything else" case: pop the head element, switch
    /// to "after head", and reprocess.
    pub(super) fn in_head_anything_else(&mut self) -> TokenOutcome {
        self.open_elements.pop();
        self.insertion_mode = InsertionMode::AfterHead;
        TokenOutcome::Reprocess
    }

    /// The "in head noscript" insertion mode (§13.2.6.4.5). Only
    /// reachable via "in head"'s `<noscript>` handling, which this
    /// crate always takes with scripting modeled as disabled (this
    /// crate never executes scripts) — matching typical validator
    /// behavior (e.g. `vnu`), the same posture `html-conform` documents
    /// elsewhere as its default.
    pub(super) fn process_token_in_head_noscript(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.merge_attributes_onto_html_element(tag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "noscript" => {
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InHead;
                TokenOutcome::Consumed(None)
            }
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::Comment(_) | TokenKind::ProcessingInstruction(_) => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style"
                ) =>
            {
                self.process_token_in_head(kind, position)
            }
            TokenKind::EndTag(tag) if tag.name == "br" => self.in_head_noscript_anything_else(),
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "head" | "noscript") => {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayStartTag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.in_head_noscript_anything_else(),
        }
    }

    /// "In head noscript"'s "anything else" case (and its explicit
    /// `</br>` entry, which acts the same way): pop the `noscript`
    /// element, switch back to "in head", and reprocess.
    pub(super) fn in_head_noscript_anything_else(&mut self) -> TokenOutcome {
        self.open_elements.pop();
        self.insertion_mode = InsertionMode::InHead;
        TokenOutcome::Reprocess
    }

    /// The "after head" insertion mode (§13.2.6.4.6). The `frameset`
    /// start tag case is intentionally absent: frameset documents are
    /// out of this crate's scope entirely (see the scope decision in
    /// plan/03-tree-construction.md) — falls through to "anything
    /// else" instead of the (unimplemented) "in frameset" mode.
    pub(super) fn process_token_after_head(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.insert_character(*c, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::Comment(data) => {
                self.insert_comment(data.clone(), Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.insert_processing_instruction(
                    pi.target.clone(),
                    pi.data.clone(),
                    Some(position),
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.merge_attributes_onto_html_element(tag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "body" => {
                self.insert_html_element(tag, Some(position));
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InFrameset;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                let Some(head) = self.head_element_pointer else {
                    unreachable!(
                        "the head element pointer is always set by the time \"after head\" runs"
                    )
                };
                self.open_elements.push(head);
                let outcome = self.process_token_in_head(kind, position);
                let head_index = self
                    .open_elements
                    .entries
                    .iter()
                    .position(|&node| node == head);
                if let Some(head_index) = head_index {
                    self.open_elements.entries.remove(head_index);
                }
                outcome
            }
            TokenKind::EndTag(tag) if tag.name == "template" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "body" | "html" | "br") => {
                self.after_head_anything_else()
            }
            TokenKind::StartTag(tag) if tag.name == "head" => {
                // "A start tag whose tag name is 'head': Parse error.
                // Ignore the token."
                self.error(ParseErrorKind::StrayStartTag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.after_head_anything_else(),
        }
    }

    /// "After head"'s "anything else" case: insert a synthesized `body`
    /// start tag (no attributes), switch to "in body", and reprocess.
    pub(super) fn after_head_anything_else(&mut self) -> TokenOutcome {
        let body_tag = TagToken {
            name: "body".to_owned(),
            self_closing: false,
            attributes: vec![],
        };
        self.insert_html_element(&body_tag, None);
        self.insertion_mode = InsertionMode::InBody;
        TokenOutcome::Reprocess
    }
}
