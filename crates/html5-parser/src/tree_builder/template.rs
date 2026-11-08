use super::*;

impl TreeBuilder {
    /// Pops and re-pushes the current template insertion mode as `mode`,
    /// switches the insertion mode to it, and reprocesses — the shared
    /// shape of "in template"'s (§13.2.6.4.16) several
    /// "pop/push/switch/reprocess" arms.
    pub(super) fn switch_from_in_template_to(&mut self, mode: InsertionMode) -> TokenOutcome {
        self.stack_of_template_insertion_modes.pop();
        self.stack_of_template_insertion_modes.push(mode);
        self.insertion_mode = mode;
        TokenOutcome::Reprocess
    }

    /// The "in template" insertion mode (§13.2.6.4.16).
    pub(super) fn process_token_in_template(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(_)
            | TokenKind::Comment(_)
            | TokenKind::ProcessingInstruction(_)
            | TokenKind::Doctype(_) => self.process_token_in_body(kind, position),
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
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                self.switch_from_in_template_to(InsertionMode::InTable)
            }
            TokenKind::StartTag(tag) if tag.name == "col" => {
                self.switch_from_in_template_to(InsertionMode::InColumnGroup)
            }
            TokenKind::StartTag(tag) if tag.name == "tr" => {
                self.switch_from_in_template_to(InsertionMode::InTableBody)
            }
            TokenKind::StartTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
                self.switch_from_in_template_to(InsertionMode::InRow)
            }
            TokenKind::StartTag(_) => self.switch_from_in_template_to(InsertionMode::InBody),
            TokenKind::EndTag(_) => {
                // "Any other end tag: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayEndTag, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::Eof => {
                // "If there is no template element on the stack of open
                // elements, stop parsing" is the fragment case (never
                // reached: this crate never does fragment parsing, and
                // this mode is only ever entered via a genuine
                // `<template>` start tag, which always pushes a
                // template element first). The "otherwise, this is a
                // parse error" path is therefore always taken.
                self.error(ParseErrorKind::EofWithUnclosedElements, position);
                self.pop_until_one_of_popped(&["template"]);
                self.active_formatting_elements.clear_up_to_last_marker();
                self.stack_of_template_insertion_modes.pop();
                self.reset_the_insertion_mode_appropriately();
                TokenOutcome::Reprocess
            }
        }
    }
}
