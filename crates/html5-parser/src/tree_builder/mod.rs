// WHATWG HTML5 tree-construction algorithm
// (https://html.spec.whatwg.org/multipage/parsing.html#tree-construction),
// including foreign-content (SVG/MathML) dispatch and attribute/tag-name
// adjustment.
//
// All insertion modes (§13.2.6.4.1-.20, minus the frameset-related ones —
// see plan/03-tree-construction.md's scope decision) and the foreign-
// content dispatcher (§13.2.6.5) are implemented; `TreeBuilder::process_token`
// is the real entry point, driven end to end by `lib.rs::parse()`.

use crate::document::{Attribute, Document, NodeId, NodeKind};
use crate::tokenizer::{
    DoctypeToken, ExternalState, ParseError, ParseErrorKind, Position, TagToken, TokenKind,
};

mod adoption;
mod after;
mod body;
mod elements;
mod foreign;
mod formatting;
mod head;
mod insert;
mod quirks;
mod scope;
mod select;
mod tables;
mod template;
#[cfg(test)]
mod tests;

use self::elements::*;
use self::foreign::*;
use self::formatting::*;
use self::quirks::*;

/// The HTML namespace URI (https://infra.spec.whatwg.org/#html-namespace).
pub(crate) const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
/// The SVG namespace URI.
pub(crate) const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
/// The MathML namespace URI.
pub(crate) const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

/// The insertion mode (§13.2.4.1): determines how the tree-construction
/// dispatcher processes each token. Only the modes this crate's scope
/// decision actually needs are listed (see plan/03-tree-construction.md)
/// — no `InFrameset`/`AfterFrameset`/`AfterAfterFrameset` (frameset
/// documents can never validate against `html-conform`'s schema, so
/// there's nothing to build them for) and no `InTemplate` (`<template>`
/// is treated as a plain element, not given its own inert-content
/// insertion mode). Each mode's actual token-processing rules are a
/// separate, not-yet-implemented step — this enum only tracks *which*
/// mode is active, needed by algorithms like this one (§13.2.6.2) that
/// read/switch it before any mode's rules exist yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum InsertionMode {
    #[default]
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

/// Which variant of §13.2.6.2's "generic raw text/RCDATA element parsing
/// algorithm" to run — the spec text is identical between the two except
/// for which tokenizer state they switch to. `Script` is not literally
/// part of §13.2.6.2 (a `<script>` start tag has its own, much longer,
/// dedicated algorithm in "in head", §13.2.6.4.4) — but once its
/// script-execution bookkeeping is stripped out (parser document,
/// force-async, already-started flags, `document.write()` re-entrancy —
/// none of it applicable, this crate never executes scripts), what's
/// left (create the element, insert it, push it, switch the tokenizer
/// to script data state, remember the original insertion mode, switch
/// to "text") is exactly this same shape, so it's folded in here rather
/// than duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenericTextElementKind {
    RawText,
    Rcdata,
    Script,
}

/// The result of running one insertion-mode handler on a token: either
/// it was fully handled (optionally requesting a tokenizer state
/// switch, same convention as
/// [`generic_text_element_parsing_algorithm`](TreeBuilder::generic_text_element_parsing_algorithm)),
/// or the spec's "reprocess the token" instruction applies — almost
/// always because the handler just switched `insertion_mode` and wants
/// the *new* mode's handler to see the same token next.
enum TokenOutcome {
    Consumed(Option<ExternalState>),
    Reprocess,
}

/// True for the five whitespace characters every insertion mode's
/// "A character token that is one of ..." case lists (tab, LF, FF, CR,
/// space). This tokenizer's input-stream preprocessing (§13.2.3.5)
/// normalizes CR/CRLF to LF before tokenization, so a literal `'\r'`
/// character token can never actually reach here — CR is included
/// anyway for fidelity to the spec's literal wording, the same stance
/// taken for other practically-unreachable-but-spec-mandated checks
/// elsewhere in this file.
fn is_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | '\r' | ' ')
}

/// Where a node should be inserted: as a child of `parent`, immediately
/// before `before` — or, if `before` is `None`, as the last child.
/// §13.2.6.1's "adjusted insertion location", minus the "inside a
/// `template` element" adjustment (this crate treats `<template>` as a
/// plain element, not a separate inert content fragment — see
/// plan/03-tree-construction.md's Template scope decision) and the
/// fragment-parsing "root insertion target" step (this crate doesn't
/// implement the HTML fragment parsing algorithm).
struct InsertionLocation {
    parent: NodeId,
    before: Option<NodeId>,
}

/// Drives tree construction: owns the [`Document`] being built and the
/// [`OpenElementsStack`], and implements the shared insertion algorithms
/// (§13.2.6.1) that every insertion mode (not yet implemented — see
/// plan/03-tree-construction.md) is built on.
pub(crate) struct TreeBuilder {
    document: Document,
    open_elements: OpenElementsStack,
    active_formatting_elements: ActiveFormattingElements,
    /// Whether foster parenting (§13.2.6.1) is currently enabled. Set by
    /// insertion-mode rules (not yet implemented) while processing
    /// certain tokens in table-related contexts.
    foster_parenting: bool,
    /// The current insertion mode (§13.2.4.1). Defaults to `Initial`,
    /// though nothing switches away from it yet — the "initial" mode's
    /// own token-processing rules aren't implemented (see
    /// plan/03-tree-construction.md).
    insertion_mode: InsertionMode,
    /// The insertion mode saved by algorithms (like this one, §13.2.6.2)
    /// that temporarily switch to another mode and need to restore it
    /// afterward — restoring it is "text" mode's own job (not yet
    /// implemented) once it sees the RCDATA/RAWTEXT element's end tag.
    original_insertion_mode: Option<InsertionMode>,
    /// The document's quirks-mode flag (§13.2.6.4.1). Defaults to
    /// `NoQuirks`; set by the "initial" insertion mode via
    /// [`determine_quirks_mode`].
    quirks_mode: QuirksMode,
    /// The "head element pointer" (§13.2.4.1) — the `head` element once
    /// "before head" has inserted one, used by "after head" to reopen it
    /// temporarily even after it's been popped off the stack of open
    /// elements.
    head_element_pointer: Option<NodeId>,
    /// The "form element pointer" (§13.2.4.1) — the most recently
    /// opened `form` element that isn't inside a `template`, used by
    /// "in body" (§13.2.6.4.7) to find the right element to close for a
    /// `</form>` end tag even if it isn't the current node.
    form_element_pointer: Option<NodeId>,
    /// The "frameset-ok flag" (§13.2.4.1). Defaults to "ok" (`true`);
    /// most of "in body"'s content-inserting rules set it to "not ok".
    /// Read by "in body"'s `<frameset>` start-tag rule (§13.2.6.4.7,
    /// plan/04-frameset.md) to decide whether a misplaced `<frameset>`
    /// may still replace `<body>`.
    frameset_ok: bool,
    /// Sentinel for "if the next token is a U+000A LINE FEED (LF)
    /// character token, then ignore that token" (§13.2.6.4.7's `pre`/
    /// `listing`/`textarea` start-tag rules — an authoring convenience
    /// that trims one leading newline). Set to `true` right after
    /// inserting one of those elements; the next character-token check
    /// consumes it unconditionally (suppressing output only if that
    /// character actually is `'\n'`).
    skip_next_line_feed: bool,
    /// The "pending table character tokens" list (§13.2.4.1), used by
    /// "in table text" (§13.2.6.4.10) to buffer character tokens until
    /// a non-character token decides whether they're plain whitespace
    /// (inserted normally) or need foster parenting (reprocessed via
    /// "in table"'s "anything else" rule). Each entry keeps its own
    /// original position, matching how the tokenizer would have
    /// produced these as separate character tokens.
    pending_table_character_tokens: Vec<(char, Position)>,
    /// The "stack of template insertion modes" (§13.2.4.1) — the "current
    /// template insertion mode" is its last entry. Pushed to by `<template>`
    /// start tags (§13.2.6.4.4) and "in template" mode's own rules
    /// (§13.2.6.4.16), popped by `</template>` end tags and "in
    /// template"'s EOF rule. Consulted by "in body"'s EOF rule (whether
    /// to defer to "in template" instead) and by
    /// `reset_the_insertion_mode_appropriately` (a `template` current
    /// node switches to this stack's top, not a fixed mode).
    stack_of_template_insertion_modes: Vec<InsertionMode>,
    /// Tree-construction-stage parse errors (§13.2.6), collected exactly
    /// the way [`crate::tokenizer::Tokenizer`] collects its own
    /// (`plan/07-parse-errors.md`) and merged with them by
    /// `lib.rs::parse`. See `plan/08-tree-construction-errors.md` for
    /// which conditions are covered and which are deliberately left out.
    errors: Vec<ParseError>,
    /// "Acknowledge the token's self-closing flag" (§13.2.5): a start tag
    /// token emitted with its self-closing flag set is a
    /// `non-void-html-element-start-tag-with-trailing-solidus` parse
    /// error *unless* the tree-construction rule that handles it
    /// acknowledges the flag. Reset before each start tag token in
    /// [`process_token`](Self::process_token), set by the rules the spec
    /// says acknowledge it, and checked afterwards.
    self_closing_flag_acknowledged: bool,
}

impl TreeBuilder {
    pub(crate) fn new() -> Self {
        TreeBuilder {
            document: Document::new(),
            open_elements: OpenElementsStack::new(),
            active_formatting_elements: ActiveFormattingElements::new(),
            foster_parenting: false,
            insertion_mode: InsertionMode::default(),
            original_insertion_mode: None,
            quirks_mode: QuirksMode::default(),
            head_element_pointer: None,
            form_element_pointer: None,
            frameset_ok: true,
            skip_next_line_feed: false,
            pending_table_character_tokens: Vec::new(),
            stack_of_template_insertion_modes: Vec::new(),
            errors: Vec::new(),
            self_closing_flag_acknowledged: false,
        }
    }

    /// Records a tree-construction parse error at `position`.
    fn error(&mut self, kind: ParseErrorKind, position: Position) {
        self.errors.push(ParseError { kind, position });
    }

    /// Drains and returns every tree-construction [`ParseError`] recorded
    /// so far. Called once by `lib.rs::parse` after parsing finishes.
    pub(crate) fn take_errors(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.errors)
    }

    /// "Acknowledge the token's self-closing flag" (§13.2.5) — called by
    /// every rule the spec says does so, suppressing the
    /// `non-void-html-element-start-tag-with-trailing-solidus` check in
    /// [`process_token`](Self::process_token).
    fn acknowledge_self_closing_flag(&mut self) {
        self.self_closing_flag_acknowledged = true;
    }

    /// The tree-construction dispatcher (§13.2.6): for each token,
    /// first decides — via [`should_process_as_foreign_content`](Self::should_process_as_foreign_content) —
    /// whether it's processed "in HTML content" (routed to the handler
    /// for the current insertion mode) or "in foreign content"
    /// (§13.2.6.5), looping to reprocess it whenever a handler switches
    /// modes/context without otherwise consuming the token — the spec's
    /// "reprocess the token" instruction, which appears throughout both.
    /// Returns the tokenizer state switch (if any) the caller must
    /// apply via `Tokenizer::switch_to`, same convention as
    /// [`generic_text_element_parsing_algorithm`](Self::generic_text_element_parsing_algorithm).
    pub(crate) fn process_token(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> Option<ExternalState> {
        // §13.2.5: "When a start tag token is emitted with its
        // self-closing flag set, if the flag is not acknowledged when it
        // is processed by the tree construction stage, that is a
        // non-void-html-element-start-tag-with-trailing-solidus parse
        // error." Only meaningful for start tags, and only decidable
        // here — the tokenizer cannot know whether an element is void.
        let self_closing_start_tag = matches!(
            kind,
            TokenKind::StartTag(tag) if tag.self_closing
        );
        if self_closing_start_tag {
            self.self_closing_flag_acknowledged = false;
        }

        let state = loop {
            let outcome = if self.should_process_as_foreign_content(kind) {
                self.process_token_foreign_content(kind, position)
            } else {
                self.process_token_by_insertion_mode(kind, position)
            };
            match outcome {
                TokenOutcome::Consumed(state) => break state,
                TokenOutcome::Reprocess => continue,
            }
        };

        if self_closing_start_tag && !self.self_closing_flag_acknowledged {
            self.error(
                ParseErrorKind::NonVoidHtmlElementStartTagWithTrailingSolidus,
                position,
            );
        }
        state
    }

    /// Dispatches to the handler for the current insertion mode — "process
    /// the token according to the rules given in the section corresponding
    /// to the current insertion mode in HTML content" (§13.2.6.5's "any
    /// other end tag" rule quotes this exact phrase). Factored out of
    /// [`process_token`](Self::process_token) so that rule can call it
    /// directly instead of returning `TokenOutcome::Reprocess`: a
    /// `Reprocess` result sends the token back through
    /// [`should_process_as_foreign_content`](Self::should_process_as_foreign_content),
    /// which for that rule specifically re-examines the very node
    /// (adjusted current node) whose foreign-namespace-ness triggered
    /// foreign-content dispatch in the first place — since that rule
    /// doesn't itself pop or otherwise change the current node before
    /// deferring to HTML content, nothing about that re-check would ever
    /// come out different, so `Reprocess` would loop forever instead of
    /// making progress.
    fn process_token_by_insertion_mode(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match self.insertion_mode {
            InsertionMode::Initial => self.process_token_initial(kind, position),
            InsertionMode::BeforeHtml => self.process_token_before_html(kind, position),
            InsertionMode::BeforeHead => self.process_token_before_head(kind, position),
            InsertionMode::InHead => self.process_token_in_head(kind, position),
            InsertionMode::InHeadNoscript => self.process_token_in_head_noscript(kind, position),
            InsertionMode::AfterHead => self.process_token_after_head(kind, position),
            InsertionMode::InBody => self.process_token_in_body(kind, position),
            InsertionMode::Text => self.process_token_text(kind, position),
            InsertionMode::InTable => self.process_token_in_table(kind, position),
            InsertionMode::InTableText => self.process_token_in_table_text(kind, position),
            InsertionMode::InCaption => self.process_token_in_caption(kind, position),
            InsertionMode::InColumnGroup => self.process_token_in_column_group(kind, position),
            InsertionMode::InTableBody => self.process_token_in_table_body(kind, position),
            InsertionMode::InRow => self.process_token_in_row(kind, position),
            InsertionMode::InCell => self.process_token_in_cell(kind, position),
            InsertionMode::InTemplate => self.process_token_in_template(kind, position),
            InsertionMode::AfterBody => self.process_token_after_body(kind, position),
            InsertionMode::InFrameset => self.process_token_in_frameset(kind, position),
            InsertionMode::AfterFrameset => self.process_token_after_frameset(kind, position),
            InsertionMode::AfterAfterBody => self.process_token_after_after_body(kind, position),
            InsertionMode::AfterAfterFrameset => {
                self.process_token_after_after_frameset(kind, position)
            }
        }
    }

    /// Whether `Tokenizer::set_in_foreign_content` should currently be
    /// `true` — the adjusted current node exists and isn't an HTML-
    /// namespace element (matches that flag's documented contract
    /// exactly: "whether it is currently in a non-HTML namespace",
    /// nothing about integration points). The driver loop
    /// (`lib.rs::parse`) calls this after every token and forwards the
    /// result, since this crate's `TreeBuilder` never holds the
    /// `Tokenizer` itself to call it directly.
    pub(crate) fn is_in_foreign_content(&self) -> bool {
        self.adjusted_current_node()
            .is_some_and(|node| !self.is_html_namespace_element(node))
    }

    /// "Stop parsing" (§13.2.7 "The end"), run by the driver loop
    /// (`lib.rs::parse`) once the tokenizer has no more tokens, right
    /// before [`into_document`](Self::into_document). Almost every real
    /// step here (readiness-state changes, `DOMContentLoaded`/`load`
    /// events, running deferred scripts, speculative-parser teardown)
    /// has no tree-shape effect and isn't implemented (no scripting,
    /// single-pass parser) — except "pop all the nodes off the stack of
    /// open elements", which for real: popping an `option` element can
    /// itself run "maybe clone an option into selectedcontent" (see
    /// `with_option_pop_hook`'s doc comment), so a `<select>` left open
    /// at EOF still needs its final selected `<option>`'s content
    /// mirrored.
    pub(crate) fn stop_parsing(&mut self) {
        self.with_option_pop_hook(|this| while this.open_elements.pop().is_some() {});
    }

    /// Consumes the builder and returns the finished [`Document`] — the
    /// driver loop's (`lib.rs::parse`) final step once the tokenizer
    /// has no more tokens.
    pub(crate) fn into_document(self) -> Document {
        self.document
    }
}
