//! Serialisierung von Token-Folgen zurück in CSS-Text.
//!
//! Grundlage ist CSS Syntax Level 3 §9 („Serialization"): Zwischen zwei
//! Token, die ohne Trennung zu einem anderen Token verschmelzen würden, steht
//! ein leerer Kommentar `/**/`. Bezeichner, Namen, Zeichenketten und URLs
//! werden nach CSSOM §2.1 maskiert, damit erneutes Tokenisieren dieselben
//! Token liefert.
//!
//! Verlustbehaftet — und deshalb in der README festgehalten:
//!
//! - Zahlen erscheinen kompakt (`.5` → `0.5`, `1.0` → `1`, `1e3` → `1000`,
//!   `+1` → `1`); die Typkennung „ganzzahlig" geht bei `1.0` verloren.
//!   Folgt eine nicht negative Zahl unmittelbar auf ein Token, mit dem sie
//!   verschmelzen würde, erhält sie ein `+` (`2n+1` bleibt `2n+1`).
//! - Leerraum schrumpft auf ein Leerzeichen; Kommentare fehlen.
//! - `<bad-string-token>` wird `""`, `<bad-url-token>` wird `url()`.
//! - Zeichenketten erscheinen immer mit `"`.

use media_query_parse::tokenizer::Token;

/// Schließendes Token eines Blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Closer {
    Paren,
    Square,
    Curly,
}

/// Öffnet `token` einen Block, und welches Token schließt ihn?
pub(crate) fn opens(token: &Token) -> Option<Closer> {
    match token {
        Token::OpenParen | Token::Function(_) => Some(Closer::Paren),
        Token::OpenSquare => Some(Closer::Square),
        Token::OpenCurly => Some(Closer::Curly),
        _ => None,
    }
}

/// Ist `token` ein schließendes Token, und welches?
pub(crate) fn closes(token: &Token) -> Option<Closer> {
    match token {
        Token::CloseParen => Some(Closer::Paren),
        Token::CloseSquare => Some(Closer::Square),
        Token::CloseCurly => Some(Closer::Curly),
        _ => None,
    }
}

fn closer_text(closer: Closer) -> char {
    match closer {
        Closer::Paren => ')',
        Closer::Square => ']',
        Closer::Curly => '}',
    }
}

/// Grobe Art eines Tokens für die Tabelle in §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ident,
    Function,
    Url,
    BadUrl,
    AtKeyword,
    Hash,
    Number,
    Percentage,
    Dimension,
    Cdc,
    OpenParen,
    Delim(char),
    Other,
}

fn kind(token: &Token) -> Kind {
    match token {
        Token::Ident(_) => Kind::Ident,
        Token::Function(_) => Kind::Function,
        Token::Url(_) => Kind::Url,
        Token::BadUrl => Kind::BadUrl,
        Token::AtKeyword(_) => Kind::AtKeyword,
        Token::Hash { .. } => Kind::Hash,
        Token::Number { .. } => Kind::Number,
        Token::Percentage { .. } => Kind::Percentage,
        Token::Dimension { .. } => Kind::Dimension,
        Token::Cdc => Kind::Cdc,
        Token::OpenParen => Kind::OpenParen,
        Token::Delim(c) => Kind::Delim(*c),
        _ => Kind::Other,
    }
}

/// CSS Syntax Level 3 §9: Braucht es zwischen `a` und `b` einen Kommentar?
fn needs_separator(a: Kind, b: Kind) -> bool {
    use Kind::*;
    let ident_like = |k: Kind| matches!(k, Ident | Function | Url | BadUrl);
    let numeric = |k: Kind| matches!(k, Number | Percentage | Dimension);
    match a {
        Ident => ident_like(b) || b == Delim('-') || numeric(b) || b == Cdc || b == OpenParen,
        AtKeyword | Hash | Dimension => ident_like(b) || b == Delim('-') || numeric(b) || b == Cdc,
        Delim('#') | Delim('-') => ident_like(b) || b == Delim('-') || numeric(b),
        Number => ident_like(b) || numeric(b) || b == Delim('%'),
        Delim('@') => ident_like(b) || b == Delim('-'),
        Delim('.') | Delim('+') => numeric(b),
        Delim('/') => b == Delim('*'),
        _ => false,
    }
}

/// Fortlaufende Ausgabe: sammelt Token, faltet Leerraum, setzt Trenner und
/// schließt am Ende offene Blöcke.
pub(crate) struct Writer {
    out: String,
    prev: Option<Kind>,
    space: bool,
    open: Vec<Closer>,
}

impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            out: String::new(),
            prev: None,
            space: false,
            open: Vec::new(),
        }
    }

    fn flush_space(&mut self) {
        if self.space && !self.out.is_empty() {
            self.out.push(' ');
            // Leerraum trennt: keine Verschmelzung über ihn hinweg.
            self.prev = None;
        }
        self.space = false;
    }

    /// Fügt fertigen CSS-Text ein (für die Auflösung von `&`).
    pub(crate) fn raw(&mut self, text: &str) {
        self.flush_space();
        self.out.push_str(text);
        self.prev = Some(Kind::Other);
    }

    pub(crate) fn token(&mut self, token: &Token) {
        if matches!(token, Token::Whitespace) {
            self.space = true;
            return;
        }
        if matches!(token, Token::Eof) {
            return;
        }
        self.flush_space();
        let k = kind(token);
        let mut plus = false;
        if self.prev.is_some_and(|prev| needs_separator(prev, k)) {
            match numeric_value(token) {
                Some(v) if v >= 0.0 && v.is_sign_positive() => plus = true,
                _ => self.out.push_str("/**/"),
            }
        }
        if plus {
            self.out.push('+');
        }
        write_token(&mut self.out, token);
        if let Some(c) = opens(token) {
            self.open.push(c);
        } else if closes(token).is_some() && closes(token).as_ref() == self.open.last() {
            self.open.pop();
        }
        self.prev = Some(k);
    }

    /// Bisherige Länge der Ausgabe in Bytes.
    pub(crate) fn len(&self) -> usize {
        self.out.len()
    }

    pub(crate) fn finish(mut self) -> String {
        while let Some(c) = self.open.pop() {
            self.out.push(closer_text(c));
        }
        self.out
    }
}

/// Serialisiert `tokens`: Leerraum gefaltet und an den Rändern entfernt,
/// offene Blöcke geschlossen.
pub(crate) fn serialize(tokens: &[Token]) -> String {
    let mut w = Writer::new();
    for t in tokens {
        w.token(t);
    }
    w.finish()
}

fn numeric_value(token: &Token) -> Option<f64> {
    match token {
        Token::Number { value, .. }
        | Token::Percentage { value }
        | Token::Dimension { value, .. } => Some(*value),
        _ => None,
    }
}

fn write_token(out: &mut String, token: &Token) {
    match token {
        Token::Ident(s) => write_ident(out, s),
        Token::Function(s) => {
            write_ident(out, s);
            out.push('(');
        }
        Token::AtKeyword(s) => {
            out.push('@');
            write_ident(out, s);
        }
        Token::Hash { value, .. } => {
            out.push('#');
            write_name(out, value);
        }
        Token::String(s) => write_string(out, s),
        Token::BadString => out.push_str("\"\""),
        Token::Url(s) => {
            out.push_str("url(");
            write_url(out, s);
            out.push(')');
        }
        Token::BadUrl => out.push_str("url()"),
        // `\` als Delim entsteht nur vor einem Zeilenumbruch (§4.3.1).
        Token::Delim('\\') => out.push_str("\\\n"),
        Token::Delim(c) => out.push(*c),
        Token::Number { value, .. } => write_number(out, *value),
        Token::Percentage { value } => {
            write_number(out, *value);
            out.push('%');
        }
        Token::Dimension { value, unit, .. } => {
            write_number(out, *value);
            write_unit(out, unit);
        }
        Token::Whitespace => out.push(' '),
        Token::Cdo => out.push_str("<!--"),
        Token::Cdc => out.push_str("-->"),
        Token::Colon => out.push(':'),
        Token::Semicolon => out.push(';'),
        Token::Comma => out.push(','),
        Token::OpenSquare => out.push('['),
        Token::CloseSquare => out.push(']'),
        Token::OpenParen => out.push('('),
        Token::CloseParen => out.push(')'),
        Token::OpenCurly => out.push('{'),
        Token::CloseCurly => out.push('}'),
        _ => {}
    }
}

/// Kompakte Zahl: kürzeste Darstellung, die denselben `f64` ergibt, ohne
/// Exponent. Unendlich (aus Literalen jenseits des `f64`-Bereichs) wird auf
/// den größten endlichen Wert begrenzt.
fn write_number(out: &mut String, value: f64) {
    use std::fmt::Write;
    let v = if value.is_infinite() {
        f64::MAX.copysign(value)
    } else {
        value
    };
    let _ = write!(out, "{v}");
}

fn write_hex_escape(out: &mut String, c: char) {
    use std::fmt::Write;
    let _ = write!(out, "\\{:x} ", c as u32);
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

/// CSSOM §2.1 „serialize an identifier".
pub(crate) fn write_ident(out: &mut String, s: &str) {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() == 1 && chars[0] == '-' {
        out.push_str("\\-");
        return;
    }
    for (i, &c) in chars.iter().enumerate() {
        if c == '\0' {
            out.push('\u{FFFD}');
        } else if matches!(c, '\u{1}'..='\u{1F}' | '\u{7F}')
            || (i == 0 && c.is_ascii_digit())
            || (i == 1 && c.is_ascii_digit() && chars[0] == '-')
        {
            write_hex_escape(out, c);
        } else if is_name_char(c) {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
}

/// Wie [`write_ident`], aber ohne Sonderregeln am Anfang (Wert eines
/// `<hash-token>`).
fn write_name(out: &mut String, s: &str) {
    for c in s.chars() {
        if c == '\0' {
            out.push('\u{FFFD}');
        } else if matches!(c, '\u{1}'..='\u{1F}' | '\u{7F}') {
            write_hex_escape(out, c);
        } else if is_name_char(c) {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
}

/// Einheit einer Dimension. Beginnt sie mit `e` und folgt eine Ziffer
/// (oder Vorzeichen und Ziffer), läse ein Tokenizer einen Exponenten; das
/// `e` wird dann maskiert.
fn write_unit(out: &mut String, unit: &str) {
    let mut chars = unit.chars();
    let first = chars.next();
    let rest: Vec<char> = chars.clone().take(2).collect();
    let looks_like_exponent = matches!(first, Some('e' | 'E'))
        && match rest.as_slice() {
            [d, ..] if d.is_ascii_digit() => true,
            ['+' | '-', d, ..] => d.is_ascii_digit(),
            _ => false,
        };
    if looks_like_exponent {
        if let Some(e) = first {
            write_hex_escape(out, e);
        }
        write_name(out, chars.as_str());
    } else {
        write_ident(out, unit);
    }
}

/// CSSOM §2.1 „serialize a string", immer mit `"`.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\0' => out.push('\u{FFFD}'),
            '\u{1}'..='\u{1F}' | '\u{7F}' => write_hex_escape(out, c),
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}

/// Wert eines unquotierten `url(...)`: maskiert, was dort nicht stehen darf
/// (§4.3.6).
fn write_url(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '\0' => out.push('\u{FFFD}'),
            '\u{1}'..='\u{20}' | '\u{7F}' => write_hex_escape(out, c),
            '"' | '\'' | '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_query_parse::tokenizer::{NumericType, tokenize};

    /// Token zum Vergleich: Leerraum gefaltet, Typkennung von Zahlen und
    /// Hashes weggelassen (die erste geht bei `1.0` absichtlich verloren).
    fn normalize(toks: Vec<Token>) -> Vec<Token> {
        let mut out: Vec<Token> = Vec::new();
        for t in toks {
            let t = match t {
                Token::Number { value, .. } => Token::Number {
                    value,
                    type_flag: NumericType::Number,
                },
                Token::Dimension { value, unit, .. } => Token::Dimension {
                    value,
                    unit,
                    type_flag: NumericType::Number,
                },
                other => other,
            };
            if t == Token::Whitespace && out.last().is_none_or(|l| *l == Token::Whitespace) {
                continue;
            }
            out.push(t);
        }
        while out.last() == Some(&Token::Eof) || out.last() == Some(&Token::Whitespace) {
            out.pop();
        }
        out
    }

    #[test]
    fn serialization_retokenizes_to_the_same_tokens() {
        for css in [
            r".md\:flex .w-1\/2 .\31 0 #\#x -\-a \-",
            "a/**/b 1/**/-2 a/**/1 -/**/-1 #/**/a @/**/b x/**/(y) 1/**/% ./**/5 //**/*",
            "2n+1 -n+3 1.5.5 1+2 1e3 1e-3 .5em 1\\65 3px 1e\\33 x",
            "\"a\\\"b\\\\c\" 'it''s' \"\\a\" url(a\\ b\\(c\\)) url(\"x\")",
            "rgba(0,0,0,.5) calc(100% - 2*1rem) <!-- --> a!important",
            "\u{1}\u{7f} é ü-ß",
        ] {
            let toks = tokenize(css);
            let again = tokenize(&serialize(&toks));
            assert_eq!(normalize(again), normalize(toks), "{css}");
        }
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(serialize(&tokenize(".5 1.0 1e3 +2 -0.25 1e999")), {
            let max = f64::MAX;
            format!("0.5 1 1000 2 -0.25 {max}")
        });
    }
}
