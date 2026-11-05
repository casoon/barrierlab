use super::*;

impl TreeBuilder {
    /// The "in body" insertion mode (§13.2.6.4.7) — by far the largest
    /// rule set in the whole spec. Transcribed arm by arm from the raw
    /// spec markup (parsed into its actual nested-list structure, not
    /// the flattened prose — flattening lost at least one real
    /// conditional boundary during transcription, e.g. "option"'s
    /// "reconstruct the active formatting elements"/"insert an HTML
    /// element" steps are unconditional, not nested under its
    /// `if`/`else`).
    ///
    /// A few pieces are genuinely out of this crate's scope or deferred
    /// to a separate step, each noted at its arm:
    /// - `<frameset>` never creates a frameset element (frameset
    ///   documents are excluded entirely, see
    ///   plan/03-tree-construction.md's scope decision).
    /// - An end-of-file token's "stop parsing" is a driver-loop concern
    ///   (not yet built).
    /// - `<math>`/`<svg>` insert the foreign-namespaced root element,
    ///   but skip the MathML/SVG/foreign attribute-adjustment tables —
    ///   that's §13.2.6.5's job (Foreign-Content-Dispatch, a separate
    ///   plan/03-tree-construction.md step), as is actually dispatching
    ///   *content inside* a foreign element through different rules.
    /// - `<noscript>` is absent from the raw-text group here (unlike
    ///   "in head"): it only belongs there "if scripting mode is not
    ///   Disabled", and this crate always models scripting as disabled
    ///   (see "in head noscript"'s doc comment) — so a stray
    ///   `<noscript>` in body position falls through to "any other
    ///   start tag" instead, exactly as intended.
    pub(super) fn process_token_in_body(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) => {
                if self.skip_next_line_feed {
                    self.skip_next_line_feed = false;
                    if *c == '\n' {
                        return TokenOutcome::Consumed(None);
                    }
                }
                if *c == '\0' {
                    return TokenOutcome::Consumed(None);
                }
                self.reconstruct_the_active_formatting_elements();
                self.insert_character(*c, Some(position));
                if !is_whitespace(*c) {
                    self.frameset_ok = false;
                }
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
                self.process_token_in_head(kind, position)
            }
            TokenKind::EndTag(tag) if tag.name == "template" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::StartTag(tag) if tag.name == "body" => {
                // "Parse error." — unconditional, ahead of the checks
                // that decide whether the token is ignored or merged.
                self.error(ParseErrorKind::StrayStartTag, position);
                let second = self.open_elements.entries.get(1).copied();
                let ignore = self.open_elements.entries.len() == 1
                    || second.is_none_or(|node| !self.node_has_html_name(node, "body"))
                    || self.has_template_on_stack();
                if !ignore {
                    self.frameset_ok = false;
                    self.merge_new_attributes_onto(second.expect("checked by `ignore` above"), tag);
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "frameset" => {
                // "Parse error." — unconditional, like `<body>` above.
                self.error(ParseErrorKind::StrayStartTag, position);
                // Note the ignore condition here is genuinely only two
                // clauses (unlike the `<body>` start-tag rule just
                // above, which has a third, explicit "or if there is a
                // template element on the stack" clause) — verified
                // against the raw spec markup, not assumed from the
                // rules' surface similarity.
                let second = self.open_elements.entries.get(1).copied();
                let ignore = self.open_elements.entries.len() == 1
                    || second.is_none_or(|node| !self.node_has_html_name(node, "body"));
                if ignore || !self.frameset_ok {
                    return TokenOutcome::Consumed(None);
                }
                self.document
                    .remove(second.expect("checked by `ignore` above"));
                let html = self
                    .open_elements
                    .topmost()
                    .expect("the html element is always on the stack by the time \"in body\" runs");
                while self.open_elements.current_node() != Some(html) {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InFrameset;
                TokenOutcome::Consumed(None)
            }
            TokenKind::Eof => {
                if !self.stack_of_template_insertion_modes.is_empty() {
                    return self.process_token_in_template(kind, position);
                }
                if self.has_unexpected_open_elements() {
                    self.error(ParseErrorKind::EofWithUnclosedElements, position);
                }
                // "Stop parsing" itself is a driver-loop concern, not
                // yet built.
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "body" => {
                if !self
                    .open_elements
                    .has_element_in_scope(&self.document, "body")
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                if self.has_unexpected_open_elements() {
                    self.error(ParseErrorKind::EofWithUnclosedElements, position);
                }
                self.insertion_mode = InsertionMode::AfterBody;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "html" => {
                if !self
                    .open_elements
                    .has_element_in_scope(&self.document, "body")
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                if self.has_unexpected_open_elements() {
                    self.error(ParseErrorKind::EofWithUnclosedElements, position);
                }
                self.insertion_mode = InsertionMode::AfterBody;
                TokenOutcome::Reprocess
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "address"
                        | "article"
                        | "aside"
                        | "blockquote"
                        | "center"
                        | "details"
                        | "dialog"
                        | "dir"
                        | "div"
                        | "dl"
                        | "fieldset"
                        | "figcaption"
                        | "figure"
                        | "footer"
                        | "header"
                        | "hgroup"
                        | "main"
                        | "menu"
                        | "nav"
                        | "ol"
                        | "p"
                        | "search"
                        | "section"
                        | "summary"
                        | "ul"
                ) =>
            {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(tag.name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
            {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                let current_is_heading = self.open_elements.current_node().is_some_and(|node| {
                    ["h1", "h2", "h3", "h4", "h5", "h6"]
                        .iter()
                        .any(|&heading| self.node_has_html_name(node, heading))
                });
                if current_is_heading {
                    self.open_elements.pop();
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "pre" | "listing") => {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                self.skip_next_line_feed = true;
                self.frameset_ok = false;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "form" => {
                if self.form_element_pointer.is_some() && !self.has_template_on_stack() {
                    // "If the form element pointer is not null, and
                    // there is no template element on the stack of open
                    // elements, then this is a parse error; ignore the
                    // token."
                    self.error(ParseErrorKind::NestedForm, position);
                    return TokenOutcome::Consumed(None);
                }
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                let form = self.insert_html_element(tag, Some(position));
                if !self.has_template_on_stack() {
                    self.form_element_pointer = Some(form);
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "li" => {
                self.frameset_ok = false;
                let mut node = self.open_elements.current_node();
                while let Some(current) = node {
                    if self.node_has_html_name(current, "li") {
                        self.open_elements
                            .generate_implied_end_tags(&self.document, Some("li"));
                        self.pop_until_one_of_popped(&["li"]);
                        break;
                    }
                    if is_special(&self.document, current)
                        && !["address", "div", "p"]
                            .iter()
                            .any(|&name| self.node_has_html_name(current, name))
                    {
                        break;
                    }
                    node = self.open_elements.element_immediately_above(current);
                }
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "dd" | "dt") => {
                self.frameset_ok = false;
                let mut node = self.open_elements.current_node();
                while let Some(current) = node {
                    if self.node_has_html_name(current, "dd") {
                        self.open_elements
                            .generate_implied_end_tags(&self.document, Some("dd"));
                        self.pop_until_one_of_popped(&["dd"]);
                        break;
                    }
                    if self.node_has_html_name(current, "dt") {
                        self.open_elements
                            .generate_implied_end_tags(&self.document, Some("dt"));
                        self.pop_until_one_of_popped(&["dt"]);
                        break;
                    }
                    if is_special(&self.document, current)
                        && !["address", "div", "p"]
                            .iter()
                            .any(|&name| self.node_has_html_name(current, name))
                    {
                        break;
                    }
                    node = self.open_elements.element_immediately_above(current);
                }
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "plaintext" => {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(Some(ExternalState::PlainText))
            }
            TokenKind::StartTag(tag) if tag.name == "button" => {
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "button")
                {
                    self.open_elements
                        .generate_implied_end_tags(&self.document, None);
                    self.pop_until_one_of_popped(&["button"]);
                }
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                self.frameset_ok = false;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "address"
                        | "article"
                        | "aside"
                        | "blockquote"
                        | "button"
                        | "center"
                        | "details"
                        | "dialog"
                        | "dir"
                        | "div"
                        | "dl"
                        | "fieldset"
                        | "figcaption"
                        | "figure"
                        | "footer"
                        | "header"
                        | "hgroup"
                        | "listing"
                        | "main"
                        | "menu"
                        | "nav"
                        | "ol"
                        | "pre"
                        | "search"
                        | "section"
                        | "select"
                        | "summary"
                        | "ul"
                ) =>
            {
                if !self
                    .open_elements
                    .has_element_in_scope(&self.document, &tag.name)
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, None);
                self.pop_until_one_of_popped(&[tag.name.as_str()]);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "form" => {
                if self.has_template_on_stack() {
                    if !self
                        .open_elements
                        .has_element_in_scope(&self.document, "form")
                    {
                        // "...this is a parse error; ignore the token."
                        self.error(ParseErrorKind::StrayEndTag, position);
                        return TokenOutcome::Consumed(None);
                    }
                    self.open_elements
                        .generate_implied_end_tags(&self.document, None);
                    self.pop_until_one_of_popped(&["form"]);
                } else {
                    // "If node is null or [...] the stack of open elements
                    // does not have node in scope, then this is a parse
                    // error; return and ignore the token."
                    let Some(node) = self.form_element_pointer.take() else {
                        self.error(ParseErrorKind::StrayEndTag, position);
                        return TokenOutcome::Consumed(None);
                    };
                    if !self.has_node_in_scope(node) {
                        self.error(ParseErrorKind::StrayEndTag, position);
                        return TokenOutcome::Consumed(None);
                    }
                    self.open_elements
                        .generate_implied_end_tags(&self.document, None);
                    self.remove_node_from_open_elements(node);
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "p" => {
                if !self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    // "If the stack of open elements does not have a p
                    // element in button scope, then this is a parse
                    // error; insert an HTML element for a 'p' start tag
                    // token with no attributes."
                    self.error(ParseErrorKind::EndTagPWithoutPInButtonScope, position);
                    let p_tag = TagToken {
                        name: "p".to_owned(),
                        self_closing: false,
                        attributes: vec![],
                    };
                    self.insert_html_element(&p_tag, None);
                }
                self.close_a_p_element(position);
                let _ = tag;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "li" => {
                if !self
                    .open_elements
                    .has_element_in_list_item_scope(&self.document, "li")
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, Some("li"));
                self.pop_until_one_of_popped(&["li"]);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "dd" | "dt") => {
                if !self
                    .open_elements
                    .has_element_in_scope(&self.document, &tag.name)
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, Some(tag.name.as_str()));
                self.pop_until_one_of_popped(&[tag.name.as_str()]);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(tag.name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
            {
                let headings = ["h1", "h2", "h3", "h4", "h5", "h6"];
                if !headings.iter().any(|&heading| {
                    self.open_elements
                        .has_element_in_scope(&self.document, heading)
                }) {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, None);
                self.pop_until_one_of_popped(&headings);
                let _ = tag;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "a" => {
                let marker_boundary = self
                    .active_formatting_elements
                    .entries
                    .iter()
                    .rposition(|entry| matches!(entry, FormattingEntry::Marker))
                    .map_or(0, |marker_index| marker_index + 1);
                let existing_a = self.active_formatting_elements.entries[marker_boundary..]
                    .iter()
                    .find_map(|entry| match entry {
                        FormattingEntry::Element(node) if self.node_has_html_name(*node, "a") => {
                            Some(*node)
                        }
                        _ => None,
                    });
                if let Some(existing_a) = existing_a {
                    // "...then this is a parse error; run the adoption
                    // agency algorithm for the token, ..."
                    self.error(ParseErrorKind::NestedFormattingElement, position);
                    self.adoption_agency_algorithm(tag, position);
                    self.remove_node_from_active_formatting_elements(existing_a);
                    self.remove_node_from_open_elements(existing_a);
                }
                self.reconstruct_the_active_formatting_elements();
                let element = self.insert_html_element(tag, Some(position));
                self.active_formatting_elements
                    .push(&self.document, element);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "b" | "big"
                        | "code"
                        | "em"
                        | "font"
                        | "i"
                        | "s"
                        | "small"
                        | "strike"
                        | "strong"
                        | "tt"
                        | "u"
                ) =>
            {
                self.reconstruct_the_active_formatting_elements();
                let element = self.insert_html_element(tag, Some(position));
                self.active_formatting_elements
                    .push(&self.document, element);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "nobr" => {
                self.reconstruct_the_active_formatting_elements();
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "nobr")
                {
                    // "...then this is a parse error; run the adoption
                    // agency algorithm for the token, ..."
                    self.error(ParseErrorKind::NestedFormattingElement, position);
                    self.adoption_agency_algorithm(tag, position);
                    self.reconstruct_the_active_formatting_elements();
                }
                let element = self.insert_html_element(tag, Some(position));
                self.active_formatting_elements
                    .push(&self.document, element);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "a" | "b"
                        | "big"
                        | "code"
                        | "em"
                        | "font"
                        | "i"
                        | "nobr"
                        | "s"
                        | "small"
                        | "strike"
                        | "strong"
                        | "tt"
                        | "u"
                ) =>
            {
                self.adoption_agency_algorithm(tag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(tag.name.as_str(), "applet" | "marquee" | "object") =>
            {
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                self.active_formatting_elements.push_marker();
                self.frameset_ok = false;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(tag.name.as_str(), "applet" | "marquee" | "object") =>
            {
                if !self
                    .open_elements
                    .has_element_in_scope(&self.document, &tag.name)
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTag, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, None);
                self.pop_until_one_of_popped(&[tag.name.as_str()]);
                self.active_formatting_elements.clear_up_to_last_marker();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "table" => {
                if self.quirks_mode != QuirksMode::Quirks
                    && self
                        .open_elements
                        .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.insert_html_element(tag, Some(position));
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTable;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "br" => {
                // "Parse error. Drop the attributes from the token, and
                // act as described in the next entry; i.e. act as if
                // this was a 'br' start tag token with no attributes."
                self.error(ParseErrorKind::EndTagBr, position);
                let br_tag = TagToken {
                    name: "br".to_owned(),
                    self_closing: false,
                    attributes: vec![],
                };
                self.insert_void_element(&br_tag, Some(position));
                let _ = tag;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "area" | "br" | "embed" | "img" | "keygen" | "wbr"
                ) =>
            {
                self.insert_void_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "input" => {
                // The fragment-case check never applies — this crate
                // never does HTML fragment parsing.
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "select")
                {
                    self.pop_until_one_of_popped(&["select"]);
                }
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                let is_hidden_type = tag
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name == "type")
                    .is_some_and(|attribute| attribute.value.eq_ignore_ascii_case("hidden"));
                if !is_hidden_type {
                    self.frameset_ok = false;
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(tag.name.as_str(), "param" | "source" | "track") =>
            {
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "hr" => {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "select")
                {
                    self.open_elements
                        .generate_implied_end_tags(&self.document, None);
                    // parse-error-only checks (option/optgroup in
                    // scope) omitted, no tree-shape effect.
                }
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                self.frameset_ok = false;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "image" => {
                // §13.2.6.4.7: "Parse error. Change the token's tag name
                // to 'img' and reprocess it. (Don't ask.)"
                self.error(ParseErrorKind::StartTagImage, position);
                let img_tag = TagToken {
                    name: "img".to_owned(),
                    ..tag.clone()
                };
                self.process_token_in_body(&TokenKind::StartTag(img_tag), position)
            }
            TokenKind::StartTag(tag) if tag.name == "textarea" => {
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::Rcdata,
                    Some(position),
                );
                self.skip_next_line_feed = true;
                self.frameset_ok = false;
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "xmp" => {
                if self
                    .open_elements
                    .has_element_in_button_scope(&self.document, "p")
                {
                    self.close_a_p_element(position);
                }
                self.reconstruct_the_active_formatting_elements();
                self.frameset_ok = false;
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::RawText,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "iframe" => {
                self.frameset_ok = false;
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::RawText,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "noembed" => {
                let (_, state) = self.generic_text_element_parsing_algorithm(
                    tag,
                    GenericTextElementKind::RawText,
                    Some(position),
                );
                TokenOutcome::Consumed(Some(state))
            }
            TokenKind::StartTag(tag) if tag.name == "select" => {
                // The fragment-case check never applies.
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "select")
                {
                    // "...then this is a parse error; pop elements [...]
                    // until a select element has been popped" — the
                    // token itself is never inserted.
                    self.error(ParseErrorKind::StrayStartTag, position);
                    self.pop_until_one_of_popped(&["select"]);
                } else {
                    self.reconstruct_the_active_formatting_elements();
                    self.insert_html_element(tag, Some(position));
                    self.frameset_ok = false;
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "option" => {
                self.with_option_pop_hook(|this| {
                    if this
                        .open_elements
                        .has_element_in_scope(&this.document, "select")
                    {
                        this.open_elements
                            .generate_implied_end_tags(&this.document, Some("optgroup"));
                    } else if this
                        .open_elements
                        .current_node()
                        .is_some_and(|node| this.node_has_html_name(node, "option"))
                    {
                        this.open_elements.pop();
                    }
                });
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "optgroup" => {
                self.with_option_pop_hook(|this| {
                    if this
                        .open_elements
                        .has_element_in_scope(&this.document, "select")
                    {
                        this.open_elements
                            .generate_implied_end_tags(&this.document, None);
                    } else if this
                        .open_elements
                        .current_node()
                        .is_some_and(|node| this.node_has_html_name(node, "option"))
                    {
                        this.open_elements.pop();
                    }
                });
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "rb" | "rtc") => {
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "ruby")
                {
                    self.open_elements
                        .generate_implied_end_tags(&self.document, None);
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "rp" | "rt") => {
                if self
                    .open_elements
                    .has_element_in_scope(&self.document, "ruby")
                {
                    self.open_elements
                        .generate_implied_end_tags(&self.document, Some("rtc"));
                }
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "math" => {
                self.reconstruct_the_active_formatting_elements();
                self.insert_foreign_element(tag, MATHML_NAMESPACE, false, Some(position));
                if tag.self_closing {
                    self.open_elements.pop();
                    self.acknowledge_self_closing_flag();
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "svg" => {
                self.reconstruct_the_active_formatting_elements();
                self.insert_foreign_element(tag, SVG_NAMESPACE, false, Some(position));
                if tag.self_closing {
                    self.open_elements.pop();
                    self.acknowledge_self_closing_flag();
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "frame"
                        | "head"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                // "Parse error. Ignore the token."
                let _ = tag;
                self.error(ParseErrorKind::StrayStartTag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) => {
                self.reconstruct_the_active_formatting_elements();
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) => {
                self.any_other_end_tag_in_body(tag, position);
                TokenOutcome::Consumed(None)
            }
        }
    }

    /// The "text" insertion mode (§13.2.6.4.8) — the counterpart to
    /// [`generic_text_element_parsing_algorithm`](Self::generic_text_element_parsing_algorithm),
    /// which is the only way this crate ever enters it (RCDATA/RAWTEXT/
    /// script data; `<plaintext>` deliberately never switches to this
    /// mode at all, see "in body"'s own `plaintext` arm).
    pub(super) fn process_token_text(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) => {
                // "In body"'s `<textarea>` start-tag rule sets
                // `skip_next_line_feed` (same flag `<pre>`/`<listing>`
                // use) *before* switching to this very mode — so the
                // character token it's meant to catch (a leading LF,
                // ignored "as an authoring convenience") always arrives
                // here, never back in "in body". Same check, same
                // unconditional clear (checked once, for the immediate
                // next token only) as "in body"'s own Character arm.
                if self.skip_next_line_feed {
                    self.skip_next_line_feed = false;
                    if *c == '\n' {
                        return TokenOutcome::Consumed(None);
                    }
                }
                self.insert_character(*c, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::Eof => {
                // "An end-of-file token: Parse error. If the current
                // node is a script element, then set its already
                // started to true. Pop the current node off the stack
                // of open elements. ..." (the already-started flag has
                // no effect here — this crate executes no scripts).
                self.error(ParseErrorKind::EofInTextMode, position);
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode.take().expect(
                    "\"text\" mode is only entered via \
                     generic_text_element_parsing_algorithm, which always records \
                     original_insertion_mode first",
                );
                TokenOutcome::Reprocess
            }
            TokenKind::EndTag(_) => {
                // Both "an end tag whose tag name is 'script'" and "any
                // other end tag" reduce to the same tree-relevant steps
                // once script-execution machinery (microtask
                // checkpoints, the speculative parser, document.write()
                // re-entrancy, script nesting level) is stripped out —
                // this crate never executes scripts.
                self.open_elements.pop();
                self.insertion_mode = self.original_insertion_mode.take().expect(
                    "\"text\" mode is only entered via \
                     generic_text_element_parsing_algorithm, which always records \
                     original_insertion_mode first",
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(_)
            | TokenKind::Comment(_)
            | TokenKind::ProcessingInstruction(_)
            | TokenKind::Doctype(_) => unreachable!(
                "the tokenizer's RCDATA/RAWTEXT/script-data states only ever produce \
                 character and end-tag tokens (plus EOF) — never start tags, comments, \
                 processing instructions, or DOCTYPEs"
            ),
        }
    }
}
