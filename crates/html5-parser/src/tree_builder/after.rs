use super::*;

impl TreeBuilder {
    /// The "after body" insertion mode (§13.2.6.4.17).
    pub(super) fn process_token_after_body(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.process_token_in_body(kind, position)
            }
            TokenKind::Comment(data) => {
                self.append_comment_to_html_element(data.clone(), position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.append_processing_instruction_to_html_element(
                    pi.target.clone(),
                    pi.data.clone(),
                    position,
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
            TokenKind::EndTag(tag) if tag.name == "html" => {
                // The fragment-case check never applies — this crate
                // never does HTML fragment parsing.
                let _ = tag;
                self.insertion_mode = InsertionMode::AfterAfterBody;
                TokenOutcome::Consumed(None)
            }
            TokenKind::Eof => {
                // "Stop parsing" is a driver-loop concern, not yet
                // built.
                TokenOutcome::Consumed(None)
            }
            _ => {
                // "Anything else: Parse error. Switch the insertion
                // mode to 'in body' and reprocess the token."
                self.error(ParseErrorKind::TokenAfterBody, position);
                self.insertion_mode = InsertionMode::InBody;
                TokenOutcome::Reprocess
            }
        }
    }

    /// The "in frameset" insertion mode (§13.2.6.4.18).
    pub(super) fn process_token_in_frameset(
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
            TokenKind::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(tag, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "frameset" => {
                // "If the current node is the root html element" is
                // only reachable in the fragment case (this crate never
                // does fragment parsing, same simplification as "after
                // body"'s `</html>` handling above).
                let _ = tag;
                self.open_elements.pop();
                let current_is_frameset = self
                    .open_elements
                    .current_node()
                    .is_some_and(|node| self.node_has_html_name(node, "frameset"));
                if !current_is_frameset {
                    self.insertion_mode = InsertionMode::AfterFrameset;
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "frame" => {
                self.insert_html_element(tag, Some(position));
                self.open_elements.pop();
                self.acknowledge_self_closing_flag();
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "noframes" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::Eof => {
                // "If the current node is not the root html element,
                // then this is a parse error." Always true here: the
                // root-html case is the fragment case (never reached).
                self.error(ParseErrorKind::EofWithUnclosedElements, position);
                // "Stop parsing" itself is a driver-loop concern, not
                // yet built.
                TokenOutcome::Consumed(None)
            }
            _ => {
                // "Anything else: Parse error. Ignore the token."
                self.report_stray_token(kind, position);
                TokenOutcome::Consumed(None)
            }
        }
    }

    /// Records the parse error for a token a mode's "anything else" rule
    /// rejects ("Parse error. Ignore the token." in the frameset family,
    /// "Parse error. Pop the noscript" in "in head noscript"): the kind
    /// depends only on the token's shape.
    pub(super) fn report_stray_token(&mut self, kind: &TokenKind, position: Position) {
        let error = match kind {
            TokenKind::StartTag(_) => ParseErrorKind::StrayStartTag,
            TokenKind::EndTag(_) => ParseErrorKind::StrayEndTag,
            TokenKind::Eof => ParseErrorKind::EofWithUnclosedElements,
            _ => ParseErrorKind::StrayCharacter,
        };
        self.error(error, position);
    }

    /// The "after frameset" insertion mode (§13.2.6.4.19).
    pub(super) fn process_token_after_frameset(
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
            TokenKind::EndTag(tag) if tag.name == "html" => {
                let _ = tag;
                self.insertion_mode = InsertionMode::AfterAfterFrameset;
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag) if tag.name == "noframes" => {
                self.process_token_in_head(kind, position)
            }
            TokenKind::Eof => TokenOutcome::Consumed(None),
            _ => {
                // "Anything else: Parse error. Ignore the token."
                self.report_stray_token(kind, position);
                TokenOutcome::Consumed(None)
            }
        }
    }

    /// The "after after body" insertion mode (§13.2.6.4.20).
    pub(super) fn process_token_after_after_body(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
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
            TokenKind::Doctype(_) => self.process_token_in_body(kind, position),
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.process_token_in_body(kind, position)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.process_token_in_body(kind, position)
            }
            TokenKind::Eof => TokenOutcome::Consumed(None),
            _ => {
                // "Anything else: Parse error. Switch the insertion
                // mode to 'in body' and reprocess the token."
                self.error(ParseErrorKind::TokenAfterBody, position);
                self.insertion_mode = InsertionMode::InBody;
                TokenOutcome::Reprocess
            }
        }
    }

    /// The "after after frameset" insertion mode (§13.2.6.4.21) — the
    /// very last insertion mode this crate implements.
    pub(super) fn process_token_after_after_frameset(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
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
            TokenKind::Doctype(_) => self.process_token_in_body(kind, position),
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.process_token_in_body(kind, position)
            }
            TokenKind::StartTag(tag) if tag.name == "html" => {
                self.process_token_in_body(kind, position)
            }
            TokenKind::Eof => TokenOutcome::Consumed(None),
            TokenKind::StartTag(tag) if tag.name == "noframes" => {
                self.process_token_in_head(kind, position)
            }
            _ => {
                // "Anything else: Parse error. Ignore the token."
                self.report_stray_token(kind, position);
                TokenOutcome::Consumed(None)
            }
        }
    }
}
