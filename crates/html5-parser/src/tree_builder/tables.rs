use super::*;

impl TreeBuilder {
    /// "In table"'s "anything else" case (§13.2.6.4.9): enable foster
    /// parenting, process the token using "in body"'s rules, then
    /// disable foster parenting again.
    pub(super) fn in_table_anything_else(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        self.foster_parenting = true;
        let outcome = self.process_token_in_body(kind, position);
        self.foster_parenting = false;
        outcome
    }

    /// The "in table" insertion mode (§13.2.6.4.9).
    pub(super) fn process_token_in_table(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(_)
                if self.open_elements.current_node().is_some_and(|node| {
                    ["table", "tbody", "template", "tfoot", "thead", "tr"]
                        .iter()
                        .any(|&name| self.node_has_html_name(node, name))
                }) =>
            {
                self.pending_table_character_tokens.clear();
                self.original_insertion_mode = Some(self.insertion_mode);
                self.insertion_mode = InsertionMode::InTableText;
                TokenOutcome::Reprocess
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
            TokenKind::StartTag(tag) if tag.name == "caption" => {
                self.clear_stack_back_to_a_table_context();
                self.active_formatting_elements.push_marker();
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InCaption;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "colgroup" => {
                self.clear_stack_back_to_a_table_context();
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InColumnGroup;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "col" => {
                self.clear_stack_back_to_a_table_context();
                let colgroup_tag = TagToken {
                    name: "colgroup".to_owned(),
                    self_closing: false,
                    attributes: vec![],
                };
                self.insert_html_element(&colgroup_tag, None);
                self.insertion_mode = InsertionMode::InColumnGroup;
                TokenOutcome::Reprocess
            }
            TokenKind::StartTag(tag)
                if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") =>
            {
                self.clear_stack_back_to_a_table_context();
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InTableBody;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "td" | "th" | "tr") => {
                self.clear_stack_back_to_a_table_context();
                let tbody_tag = TagToken {
                    name: "tbody".to_owned(),
                    self_closing: false,
                    attributes: vec![],
                };
                self.insert_html_element(&tbody_tag, None);
                self.insertion_mode = InsertionMode::InTableBody;
                TokenOutcome::Reprocess
            }
            TokenKind::StartTag(tag) if tag.name == "table" => {
                // "A start tag whose tag name is 'table': Parse error."
                // — unconditional, ahead of the in-table-scope check
                // that only decides whether the token is *ignored*.
                self.error(ParseErrorKind::StartTagTableInTable, position);
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, "table")
                {
                    return TokenOutcome::Consumed(None);
                }
                self.pop_until_one_of_popped(&["table"]);
                self.reset_the_insertion_mode_appropriately();
                TokenOutcome::Reprocess
            }
            TokenKind::EndTag(tag) if tag.name == "table" => {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, "table")
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.pop_until_one_of_popped(&["table"]);
                self.reset_the_insertion_mode_appropriately();
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body"
                        | "caption"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(tag.name.as_str(), "style" | "script" | "template") =>
            {
                self.process_token_in_head(kind, position)
            }
            TokenKind::EndTag(tag) if tag.name == "template" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::StartTag(tag) if tag.name == "input" => {
                let is_hidden_type = tag
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name == "type")
                    .is_some_and(|attribute| attribute.value.eq_ignore_ascii_case("hidden"));
                if !is_hidden_type {
                    self.error(ParseErrorKind::MisplacedTokenInTable, position);
                    return self.in_table_anything_else(kind, position);
                }
                // The "otherwise" branch is itself a parse error too
                // ("Parse error. Insert an HTML element for the token
                // ..."), reported with the same kind.
                self.error(ParseErrorKind::MisplacedTokenInTable, position);
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "form" => {
                // "Parse error." — unconditional, ahead of the check that
                // only decides whether the token is ignored.
                self.error(ParseErrorKind::MisplacedTokenInTable, position);
                if self.has_template_on_stack() || self.form_element_pointer.is_some() {
                    return TokenOutcome::Consumed(None);
                }
                let form = self.insert_html_element(tag, Some(position));
                self.form_element_pointer = Some(form);
                self.open_elements.pop();
                TokenOutcome::Consumed(None)
            }
            TokenKind::Eof => self.process_token_in_body(kind, position),
            _ => {
                // "Anything else: Parse error. Enable foster parenting,
                // process the token using the rules for the 'in body'
                // insertion mode, and then disable foster parenting."
                self.error(ParseErrorKind::MisplacedTokenInTable, position);
                self.in_table_anything_else(kind, position)
            }
        }
    }

    /// The "in table text" insertion mode (§13.2.6.4.10) — only ever
    /// entered from "in table"'s own character-token rule, buffering
    /// characters until a non-character token arrives to decide whether
    /// they're plain whitespace or need foster parenting.
    pub(super) fn process_token_in_table_text(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if *c == '\0' => TokenOutcome::Consumed(None),
            TokenKind::Character(c) => {
                self.pending_table_character_tokens.push((*c, position));
                TokenOutcome::Consumed(None)
            }
            _ => {
                let tokens = std::mem::take(&mut self.pending_table_character_tokens);
                let has_non_whitespace = tokens.iter().any(|&(c, _)| !is_whitespace(c));
                if has_non_whitespace {
                    // "If any of the tokens in the pending table
                    // character tokens list are character tokens that
                    // are not ASCII whitespace, then this is a parse
                    // error" — once for the run, not once per character
                    // (which is also why "in table"'s own "anything
                    // else" error is emitted at its call sites rather
                    // than inside `in_table_anything_else`).
                    let first = tokens
                        .first()
                        .expect("has_non_whitespace implies a non-empty list")
                        .1;
                    self.error(ParseErrorKind::NonSpaceCharactersInTable, first);
                    for &(c, char_position) in &tokens {
                        self.in_table_anything_else(&TokenKind::Character(c), char_position);
                    }
                } else {
                    for &(c, char_position) in &tokens {
                        self.insert_character(c, Some(char_position));
                    }
                }
                self.insertion_mode = self.original_insertion_mode.take().expect(
                    "\"in table text\" is only entered from \"in table\", which always \
                     records original_insertion_mode first",
                );
                TokenOutcome::Reprocess
            }
        }
    }

    /// "Close the caption" — not a named spec algorithm, but the four
    /// steps shared by "in caption"'s (§13.2.6.4.11) `</caption>` rule
    /// and its "close it implicitly and reprocess" group (the start-tag
    /// table-structure group and `</table>`). Returns whether it
    /// actually closed one (false only when `caption` isn't in table
    /// scope — every one of this helper's real call sites treats that
    /// the same way: ignore the token, don't reprocess).
    pub(super) fn close_caption(&mut self) -> bool {
        if !self
            .open_elements
            .has_element_in_table_scope(&self.document, "caption")
        {
            return false;
        }
        self.open_elements
            .generate_implied_end_tags(&self.document, None);
        self.pop_until_one_of_popped(&["caption"]);
        self.active_formatting_elements.clear_up_to_last_marker();
        self.insertion_mode = InsertionMode::InTable;
        true
    }

    /// The "in caption" insertion mode (§13.2.6.4.11).
    pub(super) fn process_token_in_caption(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::EndTag(tag) if tag.name == "caption" => {
                if !self.close_caption() {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                if self.close_caption() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::MisplacedTokenInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag) if tag.name == "table" => {
                if self.close_caption() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.process_token_in_body(kind, position),
        }
    }

    /// The "in column group" insertion mode (§13.2.6.4.12).
    pub(super) fn process_token_in_column_group(
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
                self.process_token_in_body(kind, position)
            }
            TokenKind::StartTag(tag) if tag.name == "col" => {
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "colgroup" => {
                if !self
                    .open_elements
                    .current_node()
                    .is_some_and(|node| self.node_has_html_name(node, "colgroup"))
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "col" => {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "template" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::EndTag(tag) if tag.name == "template" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::Eof => self.process_token_in_body(kind, position),
            _ => {
                if !self
                    .open_elements
                    .current_node()
                    .is_some_and(|node| self.node_has_html_name(node, "colgroup"))
                {
                    // "...then this is a parse error; ignore the token."
                    let error_kind = if matches!(kind, TokenKind::EndTag(_)) {
                        ParseErrorKind::StrayEndTagInTable
                    } else {
                        ParseErrorKind::MisplacedTokenInTable
                    };
                    self.error(error_kind, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                TokenOutcome::Reprocess
            }
        }
    }

    /// "Close the table body" — not a named spec algorithm, but the
    /// steps shared by "in table body"'s (§13.2.6.4.13) `</tbody>`/
    /// `</thead>`/`</tfoot>` rule and its "close it implicitly and
    /// reprocess" group. Returns whether it actually closed one (false
    /// only when none of tbody/thead/tfoot is in table scope).
    pub(super) fn close_table_body(&mut self) -> bool {
        let in_scope = ["tbody", "thead", "tfoot"].iter().any(|&name| {
            self.open_elements
                .has_element_in_table_scope(&self.document, name)
        });
        if !in_scope {
            return false;
        }
        self.clear_stack_back_to_a_table_body_context();
        self.open_elements.pop();
        self.insertion_mode = InsertionMode::InTable;
        true
    }

    /// The "in table body" insertion mode (§13.2.6.4.13).
    pub(super) fn process_token_in_table_body(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::StartTag(tag) if tag.name == "tr" => {
                self.clear_stack_back_to_a_table_body_context();
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InRow;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_stack_back_to_a_table_body_context();
                let tr_tag = TagToken {
                    name: "tr".to_owned(),
                    self_closing: false,
                    attributes: vec![],
                };
                self.insert_html_element(&tr_tag, None);
                self.insertion_mode = InsertionMode::InRow;
                TokenOutcome::Reprocess
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, &tag.name)
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.clear_stack_back_to_a_table_body_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTable;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                if self.close_table_body() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::MisplacedTokenInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag) if tag.name == "table" => {
                if self.close_table_body() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
                ) =>
            {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.process_token_in_table(kind, position),
        }
    }

    /// "Close the row" — not a named spec algorithm, but the steps
    /// shared by "in row"'s (§13.2.6.4.14) `</tr>` rule and its "close
    /// it implicitly and reprocess" group. Returns whether it actually
    /// closed one (false only when `tr` isn't in table scope).
    pub(super) fn close_row(&mut self) -> bool {
        if !self
            .open_elements
            .has_element_in_table_scope(&self.document, "tr")
        {
            return false;
        }
        self.clear_stack_back_to_a_table_row_context();
        self.open_elements.pop();
        self.insertion_mode = InsertionMode::InTableBody;
        true
    }

    /// The "in row" insertion mode (§13.2.6.4.14).
    pub(super) fn process_token_in_row(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_stack_back_to_a_table_row_context();
                self.insert_html_element(tag, Some(position));
                self.insertion_mode = InsertionMode::InCell;
                self.active_formatting_elements.push_marker();
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "tr" => {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, "tr")
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.clear_stack_back_to_a_table_row_context();
                self.open_elements.pop();
                self.insertion_mode = InsertionMode::InTableBody;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if self.close_row() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::MisplacedTokenInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag) if tag.name == "table" => {
                if self.close_row() {
                    TokenOutcome::Reprocess
                } else {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, &tag.name)
                {
                    // "...this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                // "If the stack of open elements does not have a tr
                // element in table scope, ignore the token." — no parse
                // error on this second check.
                if self.close_row() {
                    TokenOutcome::Reprocess
                } else {
                    TokenOutcome::Consumed(None)
                }
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
                ) =>
            {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            _ => self.process_token_in_table(kind, position),
        }
    }

    /// "Close the cell" (§13.2.6.4.15).
    pub(super) fn close_the_cell(&mut self) {
        self.open_elements
            .generate_implied_end_tags(&self.document, None);
        self.pop_until_one_of_popped(&["td", "th"]);
        self.active_formatting_elements.clear_up_to_last_marker();
        self.insertion_mode = InsertionMode::InRow;
    }

    /// The "in cell" insertion mode (§13.2.6.4.15).
    pub(super) fn process_token_in_cell(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, &tag.name)
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.open_elements
                    .generate_implied_end_tags(&self.document, None);
                self.pop_until_one_of_popped(&[tag.name.as_str()]);
                self.active_formatting_elements.clear_up_to_last_marker();
                self.insertion_mode = InsertionMode::InRow;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                self.close_the_cell();
                TokenOutcome::Reprocess
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html"
                ) =>
            {
                // "Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTagInTable, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "table" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if !self
                    .open_elements
                    .has_element_in_table_scope(&self.document, &tag.name)
                {
                    // "...then this is a parse error; ignore the token."
                    self.error(ParseErrorKind::StrayEndTagInTable, position);
                    return TokenOutcome::Consumed(None);
                }
                self.close_the_cell();
                TokenOutcome::Reprocess
            }
            _ => self.process_token_in_body(kind, position),
        }
    }
}
