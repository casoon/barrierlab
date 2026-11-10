//! Pattern syntax: XML Schema Part 2, Appendix F, plus the additions of
//! F&O 1.0 section 7.6.1 (anchors, reluctant quantifiers, back-references).

use super::RegexError;
use super::class::{CategoryName, Class, ClassItem, Pred, lookup_category};

/// Groups nested deeper than this are `Unsupported` (the parser and the
/// compiler recurse per level).
pub(super) const MAX_DEPTH: usize = 100;
/// More capture groups than this are `Unsupported` (every thread of the
/// matcher carries a slot per group).
pub(super) const MAX_GROUPS: usize = 100;

#[derive(Debug)]
pub(super) enum Node {
    Empty,
    Char(char),
    Class(Class),
    Any,
    Start,
    End,
    Group(usize, Box<Node>),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
}

pub(super) struct Parsed {
    pub node: Node,
    pub groups: usize,
}

fn invalid(message: impl Into<String>) -> RegexError {
    RegexError::InvalidPattern(message.into())
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    /// `closed[n - 1]`: capture group `n` has been closed.
    closed: Vec<bool>,
    backref: Option<usize>,
}

pub(super) fn parse(pattern: &str) -> Result<Parsed, RegexError> {
    let mut p = Parser {
        chars: pattern.chars().collect(),
        pos: 0,
        closed: Vec::new(),
        backref: None,
    };
    let node = p.alternation(0)?;
    if p.pos < p.chars.len() {
        // Only an unmatched `)` stops the alternation early.
        return Err(invalid("unmatched `)`"));
    }
    if let Some(n) = p.backref {
        return Err(RegexError::Unsupported(format!(
            "the regular-expression back-reference \\{n}"
        )));
    }
    Ok(Parsed {
        node,
        groups: p.closed.len(),
    })
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn alternation(&mut self, depth: usize) -> Result<Node, RegexError> {
        if depth > MAX_DEPTH {
            return Err(RegexError::Unsupported(format!(
                "a regular expression nested deeper than {MAX_DEPTH} levels"
            )));
        }
        let mut branches = vec![self.branch(depth)?];
        while self.peek() == Some('|') {
            self.pos += 1;
            branches.push(self.branch(depth)?);
        }
        Ok(if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Node::Alt(branches)
        })
    }

    fn branch(&mut self, depth: usize) -> Result<Node, RegexError> {
        let mut pieces = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom(depth)?;
            pieces.push(self.quantified(atom)?);
        }
        Ok(match pieces.len() {
            0 => Node::Empty,
            1 => pieces.pop().unwrap(),
            _ => Node::Concat(pieces),
        })
    }

    fn atom(&mut self, depth: usize) -> Result<Node, RegexError> {
        let c = self.peek().expect("checked by the caller");
        self.pos += 1;
        match c {
            '(' => {
                if self.closed.len() >= MAX_GROUPS {
                    return Err(RegexError::Unsupported(format!(
                        "more than {MAX_GROUPS} capture groups in a regular expression"
                    )));
                }
                self.closed.push(false);
                let index = self.closed.len();
                let inner = self.alternation(depth + 1)?;
                if self.peek() != Some(')') {
                    return Err(invalid("unclosed `(`"));
                }
                self.pos += 1;
                self.closed[index - 1] = true;
                Ok(Node::Group(index, Box::new(inner)))
            }
            '.' => Ok(Node::Any),
            '^' => Ok(Node::Start),
            '$' => Ok(Node::End),
            '[' => Ok(Node::Class(self.class(depth)?)),
            '\\' => self.escape_outside_class(),
            '?' | '*' | '+' | '{' | '}' | ']' => {
                Err(invalid(format!("unexpected `{c}` (escape it with `\\`)")))
            }
            c => Ok(Node::Char(c)),
        }
    }

    fn quantified(&mut self, atom: Node) -> Result<Node, RegexError> {
        let (min, max) = match self.peek() {
            Some('?') => (0, Some(1)),
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('{') => {
                self.pos += 1;
                let range = self.counts()?;
                self.pos -= 1; // the closing `}` is consumed below
                range
            }
            _ => return Ok(atom),
        };
        self.pos += 1;
        let greedy = if self.peek() == Some('?') {
            self.pos += 1;
            false
        } else {
            true
        };
        if matches!(self.peek(), Some('?' | '*' | '+' | '{')) {
            return Err(invalid("a quantifier follows a quantifier"));
        }
        Ok(Node::Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    /// After `{`: `n}`, `n,}` or `n,m}`; leaves `pos` on the closing `}`... one past it.
    fn counts(&mut self) -> Result<(u32, Option<u32>), RegexError> {
        let min = self.number()?;
        let max = if self.peek() == Some(',') {
            self.pos += 1;
            if self.peek() == Some('}') {
                None
            } else {
                Some(self.number()?)
            }
        } else {
            Some(min)
        };
        if self.peek() != Some('}') {
            return Err(invalid("malformed `{n,m}` quantifier"));
        }
        self.pos += 1;
        if let Some(max) = max
            && max < min
        {
            return Err(invalid(format!(
                "quantifier {{{min},{max}}}: max below min"
            )));
        }
        Ok((min, max))
    }

    fn number(&mut self) -> Result<u32, RegexError> {
        let start = self.pos;
        let mut value = 0u32;
        while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
            value = value.saturating_mul(10).saturating_add(d);
            self.pos += 1;
        }
        if self.pos == start {
            return Err(invalid("a number is expected in a `{n,m}` quantifier"));
        }
        Ok(value)
    }

    fn escape_outside_class(&mut self) -> Result<Node, RegexError> {
        let Some(c) = self.peek() else {
            return Err(invalid("a pattern cannot end in `\\`"));
        };
        self.pos += 1;
        if let Some(single) = single_escape(c) {
            return Ok(Node::Char(single));
        }
        if let Some(item) = self.multi_escape(c)? {
            return Ok(Node::Class(Class {
                items: vec![item],
                ..Class::default()
            }));
        }
        if let '1'..='9' = c {
            // The first digit is the group; further digits are taken while
            // they still name a group that has been closed (F&O 7.6.1).
            let closed = |n: usize| n >= 1 && self.closed.get(n - 1) == Some(&true);
            let mut n = c.to_digit(10).unwrap() as usize;
            if !closed(n) {
                return Err(invalid(format!(
                    "back-reference \\{n} names no group that is closed"
                )));
            }
            while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
                let longer = n * 10 + d as usize;
                if !closed(longer) {
                    break;
                }
                n = longer;
                self.pos += 1;
            }
            self.backref.get_or_insert(n);
            return Ok(Node::Empty);
        }
        Err(invalid(format!("`\\{c}` is not a valid escape")))
    }

    /// `\s \S \i \I \c \C \d \D \w \W \p{..} \P{..}` after the backslash and
    /// the letter `c`.
    fn multi_escape(&mut self, c: char) -> Result<Option<ClassItem>, RegexError> {
        let (pred, complement) = match c {
            's' => (Pred::Space, false),
            'S' => (Pred::Space, true),
            'i' => (Pred::NameStart, false),
            'I' => (Pred::NameStart, true),
            'c' => (Pred::NameChar, false),
            'C' => (Pred::NameChar, true),
            'd' => (Pred::Digit, false),
            'D' => (Pred::Digit, true),
            'w' => (Pred::Word, false),
            'W' => (Pred::Word, true),
            'p' | 'P' => {
                if self.peek() != Some('{') {
                    return Err(invalid(format!("`\\{c}` needs `{{name}}`")));
                }
                let close = self.chars[self.pos..]
                    .iter()
                    .position(|&x| x == '}')
                    .ok_or_else(|| invalid(format!("`\\{c}{{` is not closed")))?;
                let name: String = self.chars[self.pos + 1..self.pos + close].iter().collect();
                self.pos += close + 1;
                let pred = match lookup_category(&name) {
                    CategoryName::Known(cat) => Pred::Category(cat),
                    CategoryName::Block(lo, hi) => Pred::Block(lo, hi),
                    CategoryName::UnknownBlock => {
                        return Err(RegexError::Unsupported(format!(
                            "the Unicode block `\\{c}{{{name}}}` (only a few blocks are known, see docs)"
                        )));
                    }
                    CategoryName::Invalid => {
                        return Err(invalid(format!("`{name}` is not a category or block")));
                    }
                };
                (pred, c == 'P')
            }
            _ => return Ok(None),
        };
        Ok(Some(ClassItem::Pred(pred, complement)))
    }

    /// After `[`.
    fn class(&mut self, depth: usize) -> Result<Class, RegexError> {
        if depth > MAX_DEPTH {
            return Err(RegexError::Unsupported(format!(
                "a regular expression nested deeper than {MAX_DEPTH} levels"
            )));
        }
        let mut class = Class::default();
        if self.peek() == Some('^') {
            self.pos += 1;
            class.negated = true;
        }
        loop {
            let Some(c) = self.peek() else {
                return Err(invalid("unclosed `[`"));
            };
            match c {
                ']' => {
                    self.pos += 1;
                    break;
                }
                '[' => return Err(invalid("`[` inside a character class must be escaped")),
                '-' if self.peek_at(1) == Some('[') => {
                    if class.items.is_empty() {
                        return Err(invalid("class subtraction without a left operand"));
                    }
                    self.pos += 2;
                    class.subtract = Some(Box::new(self.class(depth + 1)?));
                    if self.peek() != Some(']') {
                        return Err(invalid("a class subtraction must end the class"));
                    }
                }
                _ => {
                    let first = class.items.is_empty();
                    self.class_item(&mut class, first)?;
                }
            }
        }
        if class.items.is_empty() {
            return Err(invalid("empty character class"));
        }
        Ok(class)
    }

    fn class_item(&mut self, class: &mut Class, first: bool) -> Result<(), RegexError> {
        let c = self.peek().unwrap();
        self.pos += 1;
        let lo = if c == '\\' {
            let Some(e) = self.peek() else {
                return Err(invalid("a pattern cannot end in `\\`"));
            };
            self.pos += 1;
            if let Some(single) = single_escape(e) {
                single
            } else if let Some(item) = self.multi_escape(e)? {
                class.items.push(item);
                return self.after_multi_escape();
            } else {
                return Err(invalid(format!(
                    "`\\{e}` is not valid in a character class"
                )));
            }
        } else {
            if c == '-' && !first && self.peek() != Some(']') {
                // A `-` that is neither first, last, a range operator nor
                // the start of a subtraction: the grammar is ambiguous here.
                return Err(RegexError::Unsupported(
                    "a `-` in the middle of a character class (use `\\-`)".into(),
                ));
            }
            c
        };
        if self.peek() == Some('-') && !matches!(self.peek_at(1), Some(']' | '[')) {
            self.pos += 1;
            let h = self.peek().ok_or_else(|| invalid("unclosed `[`"))?;
            self.pos += 1;
            let hi = if h == '\\' {
                let e = self.peek().ok_or_else(|| invalid("unclosed `[`"))?;
                self.pos += 1;
                single_escape(e)
                    .ok_or_else(|| invalid(format!("`\\{e}` cannot end a character range")))?
            } else if matches!(h, '[' | '-') {
                return Err(invalid("a character range ends in a plain character"));
            } else {
                h
            };
            if hi < lo {
                return Err(invalid(format!("the range {lo}-{hi} is reversed")));
            }
            class.items.push(ClassItem::Range(lo, hi));
        } else {
            class.items.push(ClassItem::Range(lo, lo));
        }
        Ok(())
    }

    /// `\d-x` and friends: a range cannot start with a multi-character escape.
    fn after_multi_escape(&self) -> Result<(), RegexError> {
        if self.peek() == Some('-') && !matches!(self.peek_at(1), Some(']' | '[')) {
            return Err(RegexError::Unsupported(
                "a `-` after a multi-character escape inside a character class".into(),
            ));
        }
        Ok(())
    }
}

/// `\n \r \t \\ \| \. \- \^ \? \* \+ \{ \} \( \) \[ \] \$` (XML Schema
/// SingleCharEsc plus `\$` of F&O).
fn single_escape(c: char) -> Option<char> {
    Some(match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' | '|' | '.' | '-' | '^' | '?' | '*' | '+' | '{' | '}' | '(' | ')' | '[' | ']'
        | '$' => c,
        _ => return None,
    })
}
