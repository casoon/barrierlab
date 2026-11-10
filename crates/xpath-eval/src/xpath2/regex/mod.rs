//! The regular expressions of `fn:matches`, `fn:replace` and `fn:tokenize`:
//! XQuery 1.0 and XPath 2.0 Functions and Operators section 7.6, on top of
//! XML Schema Part 2 Appendix F. Own engine, no dependency: a Thompson NFA run
//! as a Pike VM ([`vm`]), so no pattern or input can cause backtracking blow-up
//! or deep recursion. What it cannot decide exactly is a [`RegexError::Unsupported`],
//! never a guess; limits and the supported Unicode subset are listed in
//! `docs/packages/xpath-eval.md`.

mod class;
mod parse;
mod vm;

use vm::{Flags, Program, STEP_BUDGET};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum RegexError {
    /// `FORX0001`
    InvalidFlags(String),
    /// `FORX0002`
    InvalidPattern(String),
    /// `FORX0003`
    ZeroLength,
    /// `FORX0004`
    InvalidReplacement(String),
    /// A valid pattern or input this engine does not decide.
    Unsupported(String),
}

pub(super) struct Regex {
    program: Program,
}

impl Regex {
    pub fn new(pattern: &str, flags: &str) -> Result<Regex, RegexError> {
        let mut f = Flags {
            dot_all: false,
            multi_line: false,
            case_insensitive: false,
        };
        let mut strip = false;
        for c in flags.chars() {
            match c {
                's' => f.dot_all = true,
                'm' => f.multi_line = true,
                'i' => f.case_insensitive = true,
                'x' => strip = true,
                other => {
                    return Err(RegexError::InvalidFlags(format!(
                        "`{other}` is not one of the flags s, m, i, x"
                    )));
                }
            }
        }
        let stripped;
        let pattern = if strip {
            stripped = strip_whitespace(pattern);
            &stripped
        } else {
            pattern
        };
        let program = vm::compile(parse::parse(pattern)?, &f)?;
        Ok(Regex { program })
    }

    pub fn groups(&self) -> usize {
        self.program.groups
    }

    /// `fn:matches`: does any substring match?
    pub fn is_match(&self, text: &str) -> Result<bool, RegexError> {
        let text: Vec<char> = text.chars().collect();
        let mut budget = STEP_BUDGET;
        Ok(self
            .program
            .search(&text, 0, 0, true, &mut budget)?
            .is_some())
    }

    /// F&O 7.6.3/7.6.4: a pattern that matches the zero-length string is `FORX0003`.
    fn reject_empty_match(&self) -> Result<(), RegexError> {
        let mut budget = STEP_BUDGET;
        match self.program.search(&[], 0, 0, true, &mut budget)? {
            Some(_) => Err(RegexError::ZeroLength),
            None => Ok(()),
        }
    }

    /// `fn:tokenize` for a non-empty `text`.
    pub fn tokenize(&self, text: &str) -> Result<Vec<String>, RegexError> {
        self.reject_empty_match()?;
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let text: Vec<char> = text.chars().collect();
        let mut budget = STEP_BUDGET;
        let mut tokens = Vec::new();
        let (mut last, mut pos) = (0, 0);
        while pos <= text.len() {
            let Some(m) = self.program.search(&text, pos, 2, false, &mut budget)? else {
                break;
            };
            tokens.push(text[last..m[0]].iter().collect());
            last = m[1];
            pos = m[1];
        }
        tokens.push(text[last..].iter().collect());
        Ok(tokens)
    }

    /// `fn:replace`.
    pub fn replace(&self, text: &str, replacement: &str) -> Result<String, RegexError> {
        let pieces = parse_replacement(replacement, self.groups())?;
        self.reject_empty_match()?;
        let needed = pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Group(n) => Some(*n),
                Piece::Text(_) => None,
            })
            .max()
            .unwrap_or(0);
        let slots = 2 * (needed + 1);
        let text: Vec<char> = text.chars().collect();
        let mut budget = STEP_BUDGET;
        let mut out = String::new();
        let (mut last, mut pos) = (0, 0);
        while pos <= text.len() {
            let Some(m) = self.program.search(&text, pos, slots, false, &mut budget)? else {
                break;
            };
            out.extend(&text[last..m[0]]);
            for piece in &pieces {
                match piece {
                    Piece::Text(t) => out.push_str(t),
                    Piece::Group(n) => {
                        let (s, e) = (m[2 * n], m[2 * n + 1]);
                        if s != usize::MAX && e != usize::MAX {
                            out.extend(&text[s..e]);
                        }
                    }
                }
            }
            last = m[1];
            pos = m[1];
        }
        out.extend(&text[last..]);
        Ok(out)
    }
}

enum Piece {
    Text(String),
    Group(usize),
}

/// F&O 7.6.3: `$N` is group `N` (`$0` the whole match), `\$` and `\\` are the
/// literal characters, any other `$` or `\` is `FORX0004`. After `$` the first
/// digit is always part of the number; further digits are taken while the
/// number still names an existing group. A group that does not exist inserts
/// nothing.
fn parse_replacement(replacement: &str, groups: usize) -> Result<Vec<Piece>, RegexError> {
    let invalid = |m: &str| RegexError::InvalidReplacement(m.to_owned());
    let chars: Vec<char> = replacement.chars().collect();
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => match chars.get(i + 1) {
                Some(&c @ ('\\' | '$')) => {
                    text.push(c);
                    i += 2;
                }
                _ => return Err(invalid("`\\` must be followed by `\\` or `$`")),
            },
            '$' => {
                let Some(first) = chars.get(i + 1).and_then(|c| c.to_digit(10)) else {
                    return Err(invalid("`$` must be followed by a digit (write `\\$`)"));
                };
                let mut n = first as usize;
                i += 2;
                while let Some(d) = chars.get(i).and_then(|c| c.to_digit(10)) {
                    let longer = n * 10 + d as usize;
                    if longer > groups {
                        break;
                    }
                    n = longer;
                    i += 1;
                }
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                if n <= groups {
                    pieces.push(Piece::Group(n));
                }
            }
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    Ok(pieces)
}

/// Flag `x` (F&O 7.6.1.1): whitespace characters (space, tab, LF, CR) in the
/// pattern are removed, except inside a character class. A backslash does not
/// protect a following whitespace character (`hello\ sworld` is `hello\sworld`).
fn strip_whitespace(pattern: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' | '\r' if depth == 0 => {}
            '\\' => {
                out.push(c);
                if let Some(next) = chars.next() {
                    if depth == 0 && matches!(next, ' ' | '\t' | '\n' | '\r') {
                        continue;
                    }
                    out.push(next);
                }
            }
            '[' => {
                depth += 1;
                out.push(c);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}
