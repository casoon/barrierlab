//! Die Parse-Algorithmen aus CSS Syntax Level 3 §5 (Editor's Draft mit
//! Verschachtelung), über einer Token-Liste mit abschließendem `Eof`.
//!
//! Das Ergebnis ist ein roher Baum ([`Raw`]): Präludien und Werte bleiben
//! Bereiche der Token-Liste. Welche Regel was bedeutet (`@media`,
//! `@keyframes`, Stilregel …), entscheidet erst `lib.rs`.

use std::ops::Range;

use media_query_parse::tokenizer::Token;

use crate::Declaration;
use crate::serialize::{Closer, closes, opens, serialize};

/// Tiefe, ab der ein Block übersprungen und seine Regel verworfen wird.
/// Schützt den Stapel vor Eingaben wie `@media{` hunderttausendfach; echte
/// Stylesheets bleiben weit darunter.
pub(crate) const MAX_DEPTH: usize = 128;

/// Eine Regel vor der Einordnung.
pub(crate) enum Raw {
    /// At-Regel; `block` fehlt bei Anweisungen wie `@import …;`.
    At {
        name: String,
        prelude: Range<usize>,
        block: Option<Block>,
    },
    /// Qualifizierte Regel (Stilregel, Keyframe).
    Qualified { prelude: Range<usize>, block: Block },
}

/// Inhalt eines `{}`-Blocks: Deklarationen und Regeln (§5.4.4).
#[derive(Default)]
pub(crate) struct Block {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) rules: Vec<Raw>,
}

/// Ergebnis von §5.4.6.
enum Decl {
    /// Eine Deklaration.
    Valid(Declaration),
    /// Syntaktisch eine Deklaration, ihr Wert enthält aber fehlerhafte Token
    /// und passt damit auf keine Grammatik: verworfen, Eingabe verbraucht.
    Dropped,
    /// Keine Deklaration; der Aufrufer setzt neu als Regel an.
    NotADeclaration,
}

pub(crate) struct Parser<'t> {
    pub(crate) toks: &'t [Token],
    pos: usize,
    depth: usize,
}

/// Ende der Komponente, die bei `start` beginnt: ein einzelnes Token oder
/// ein ganzer Block samt Funktion (§5.4.7–5.4.9), ohne Rekursion. Ein nicht
/// geschlossener Block reicht bis `Eof` bzw. bis zum Ende von `toks`.
pub(crate) fn component_end(toks: &[Token], start: usize) -> usize {
    let mut stack: Vec<Closer> = Vec::new();
    let mut pos = start;
    while pos < toks.len() {
        let t = &toks[pos];
        if matches!(t, Token::Eof) {
            return pos;
        }
        pos += 1;
        if let Some(c) = opens(t) {
            stack.push(c);
        } else if !stack.is_empty() && closes(t).as_ref() == stack.last() {
            stack.pop();
        }
        if stack.is_empty() {
            return pos;
        }
    }
    pos
}

/// Die Komponenten von `toks[range]` auf oberster Ebene, als Bereiche.
pub(crate) fn components(toks: &[Token], range: Range<usize>) -> Vec<Range<usize>> {
    let slice = &toks[..range.end];
    let mut out = Vec::new();
    let mut pos = range.start;
    while pos < range.end {
        let end = component_end(slice, pos).max(pos + 1);
        out.push(pos..end);
        pos = end;
    }
    out
}

impl<'t> Parser<'t> {
    pub(crate) fn new(toks: &'t [Token]) -> Self {
        Self {
            toks,
            pos: 0,
            depth: 0,
        }
    }

    fn peek(&self) -> &Token {
        // Die Liste endet immer mit `Eof`, und `pos` geht nie darüber hinaus.
        &self.toks[self.pos]
    }

    fn bump(&mut self) {
        if !matches!(self.peek(), Token::Eof) {
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Token::Whitespace) {
            self.pos += 1;
        }
    }

    fn skip_component(&mut self) {
        self.pos = component_end(self.toks, self.pos);
    }

    /// §5.4.1 „consume a stylesheet's contents" (Liste von Regeln, oberste
    /// Ebene).
    pub(crate) fn stylesheet(&mut self) -> Vec<Raw> {
        let mut rules = Vec::new();
        loop {
            match self.peek() {
                Token::Whitespace | Token::Cdo | Token::Cdc => self.pos += 1,
                Token::Eof => return rules,
                Token::AtKeyword(_) => {
                    if let Some(r) = self.at_rule(false) {
                        rules.push(r);
                    }
                }
                _ => {
                    if let Some(r) = self.qualified_rule(None, false) {
                        rules.push(r);
                    }
                }
            }
        }
    }

    /// §5.4.2 „consume an at-rule".
    fn at_rule(&mut self, nested: bool) -> Option<Raw> {
        let name = match self.peek() {
            Token::AtKeyword(n) => n.to_ascii_lowercase(),
            _ => return None,
        };
        self.pos += 1;
        let start = self.pos;
        loop {
            match self.peek() {
                Token::Semicolon => {
                    let prelude = start..self.pos;
                    self.pos += 1;
                    return Some(Raw::At {
                        name,
                        prelude,
                        block: None,
                    });
                }
                Token::CloseCurly if nested => {
                    return Some(Raw::At {
                        name,
                        prelude: start..self.pos,
                        block: None,
                    });
                }
                Token::Eof => {
                    return Some(Raw::At {
                        name,
                        prelude: start..self.pos,
                        block: None,
                    });
                }
                Token::OpenCurly => {
                    let prelude = start..self.pos;
                    let block = self.block()?;
                    return Some(Raw::At {
                        name,
                        prelude,
                        block: Some(block),
                    });
                }
                // Auch ein `}` auf oberster Ebene: Parse-Fehler, gehört dann
                // zum Präludium.
                _ => self.skip_component(),
            }
        }
    }

    /// §5.4.3 „consume a qualified rule".
    fn qualified_rule(&mut self, stop: Option<&Token>, nested: bool) -> Option<Raw> {
        let start = self.pos;
        loop {
            let t = self.peek();
            if matches!(t, Token::Eof) || stop == Some(t) {
                return None;
            }
            match t {
                Token::CloseCurly => {
                    if nested {
                        return None;
                    }
                    self.pos += 1;
                }
                Token::OpenCurly => {
                    let prelude = start..self.pos;
                    if self.starts_with_custom_property(prelude.clone()) {
                        if nested {
                            self.bad_declaration_remnants(nested);
                        } else {
                            self.skip_component();
                        }
                        return None;
                    }
                    let block = self.block()?;
                    return Some(Raw::Qualified { prelude, block });
                }
                _ => self.skip_component(),
            }
        }
    }

    /// Beginnt das Präludium mit `--name:`? Dann ist es eine verunglückte
    /// Eigenschaft, keine Regel (§5.4.3).
    fn starts_with_custom_property(&self, prelude: Range<usize>) -> bool {
        let mut it = self.toks[prelude]
            .iter()
            .filter(|t| !matches!(t, Token::Whitespace));
        matches!(
            (it.next(), it.next()),
            (Some(Token::Ident(n)), Some(Token::Colon)) if n.starts_with("--")
        )
    }

    /// §5.4.4 „consume a block": steht auf `{`, verbraucht bis einschließlich
    /// `}`. `None`, wenn die Tiefengrenze erreicht ist (Block übersprungen).
    fn block(&mut self) -> Option<Block> {
        if self.depth >= MAX_DEPTH {
            self.skip_component();
            return None;
        }
        self.pos += 1; // `{`
        self.depth += 1;
        let block = self.block_contents();
        self.depth -= 1;
        if matches!(self.peek(), Token::CloseCurly) {
            self.pos += 1;
        }
        Some(block)
    }

    /// §5.4.5 „consume a block's contents".
    fn block_contents(&mut self) -> Block {
        let mut block = Block::default();
        loop {
            match self.peek() {
                Token::Whitespace | Token::Semicolon => self.pos += 1,
                Token::Eof | Token::CloseCurly => return block,
                Token::AtKeyword(_) => {
                    if let Some(r) = self.at_rule(true) {
                        block.rules.push(r);
                    }
                }
                _ => {
                    let mark = self.pos;
                    match self.declaration() {
                        Decl::Valid(d) => block.declarations.push(d),
                        Decl::Dropped => {}
                        Decl::NotADeclaration => {
                            self.pos = mark;
                            if let Some(r) = self.qualified_rule(Some(&Token::Semicolon), true) {
                                block.rules.push(r);
                            }
                        }
                    }
                }
            }
        }
    }

    /// §5.4.6 „consume a declaration" (immer `nested`, da nur aus Blöcken
    /// heraus gerufen).
    fn declaration(&mut self) -> Decl {
        let raw_name = match self.peek() {
            Token::Ident(n) => n.clone(),
            _ => return Decl::NotADeclaration,
        };
        self.pos += 1;
        self.skip_whitespace();
        if !matches!(self.peek(), Token::Colon) {
            return Decl::NotADeclaration;
        }
        self.pos += 1;
        self.skip_whitespace();
        let start = self.pos;
        loop {
            match self.peek() {
                Token::Semicolon | Token::CloseCurly | Token::Eof => break,
                _ => self.skip_component(),
            }
        }
        let mut comps = components(self.toks, start..self.pos);
        let is_ws = |r: &Range<usize>, toks: &[Token]| {
            r.len() == 1 && matches!(toks[r.start], Token::Whitespace)
        };

        // `!important`, auch mit Leerraum dazwischen.
        let non_ws: Vec<usize> = (0..comps.len())
            .filter(|&i| !is_ws(&comps[i], self.toks))
            .collect();
        let mut important = false;
        if let [.., bang, imp] = non_ws.as_slice() {
            if comps[*bang].len() == 1
                && comps[*imp].len() == 1
                && matches!(self.toks[comps[*bang].start], Token::Delim('!'))
                && matches!(&self.toks[comps[*imp].start],
                    Token::Ident(s) if s.eq_ignore_ascii_case("important"))
            {
                important = true;
                comps.truncate(*bang);
            }
        }
        while comps.last().is_some_and(|r| is_ws(r, self.toks)) {
            comps.pop();
        }

        let custom = raw_name.starts_with("--");
        let toks = self.toks;
        let has_curly = comps
            .iter()
            .any(|r| matches!(toks[r.start], Token::OpenCurly));
        let has_other = comps
            .iter()
            .any(|r| !is_ws(r, toks) && !matches!(toks[r.start], Token::OpenCurly));
        if !custom && has_curly && has_other {
            return Decl::NotADeclaration;
        }

        let value_start = comps.first().map_or(start, |f| f.start);
        let value_end = comps.last().map_or(start, |l| l.end);
        let value_range = value_start..value_end;
        let value_toks = &toks[value_range];
        // Ein Wert mit fehlerhaften Token passt auf keine Grammatik, auch
        // nicht `<declaration-value>` einer Custom Property: verworfen.
        if contains_bad_tokens(value_toks) {
            return Decl::Dropped;
        }
        let name = if custom {
            raw_name
        } else {
            raw_name.to_ascii_lowercase()
        };
        Decl::Valid(Declaration {
            name,
            value: serialize(value_toks),
            important,
        })
    }

    /// §5.4.6 „consume the remnants of a bad declaration".
    fn bad_declaration_remnants(&mut self, nested: bool) {
        loop {
            match self.peek() {
                Token::Eof => return,
                Token::Semicolon => {
                    self.bump();
                    return;
                }
                Token::CloseCurly if nested => return,
                _ => self.skip_component(),
            }
        }
    }
}

/// `<bad-string-token>`, `<bad-url-token>` oder ein schließendes Token ohne
/// öffnendes auf oberster Ebene.
pub(crate) fn contains_bad_tokens(toks: &[Token]) -> bool {
    let mut stack: Vec<Closer> = Vec::new();
    for t in toks {
        if matches!(t, Token::BadString | Token::BadUrl) {
            return true;
        }
        if let Some(c) = opens(t) {
            stack.push(c);
        } else if let Some(c) = closes(t) {
            if stack.last() == Some(&c) {
                stack.pop();
            } else if stack.is_empty() {
                return true;
            }
        }
    }
    false
}
