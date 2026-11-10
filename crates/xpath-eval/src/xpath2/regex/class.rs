//! Character classes and the Unicode questions they ask. Every question has
//! three answers: yes, no, or "cannot decide" (`None`): the crate carries no
//! Unicode database, so a general category outside what is known exactly is
//! reported as undecided, and the matcher turns that into `Unsupported` for the
//! character at hand instead of guessing.

/// General categories of U+0000..=U+00FF (Unicode `UnicodeData.txt`, two
/// letters per code point, generated once from the Unicode Character Database).
const LATIN1: &str = "CcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcZsPoPoPoScPoPoPoPsPePoSmPoPdPoPoNdNdNdNdNdNdNdNdNdNdPoPoSmSmSmPoPoLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuPsPoPeSkPcSkLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlPsSmPeSmCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcCcZsPoScScScScSoPoSkSoLoPiSmCfSoSkSoSmNoNoSkLlPoPoSkNoLoPfNoNoNoPoLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuLuSmLuLuLuLuLuLuLuLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlLlSmLlLlLlLlLlLlLlLl";

/// The categories a `\p{..}` may name (XML Schema Part 2, Appendix F, `IsCategory`;
/// `Cs` is not a category of XML characters).
const CATEGORIES: &[&str] = &[
    "L", "Lu", "Ll", "Lt", "Lm", "Lo", "M", "Mn", "Mc", "Me", "N", "Nd", "Nl", "No", "P", "Pc",
    "Pd", "Ps", "Pe", "Pi", "Pf", "Po", "Z", "Zs", "Zl", "Zp", "S", "Sm", "Sc", "Sk", "So", "C",
    "Cc", "Cf", "Co", "Cn",
];

/// Blocks whose range is known exactly (name as in XML Schema, without `Is`).
const BLOCKS: &[(&str, u32, u32)] = &[
    ("BasicLatin", 0x0000, 0x007F),
    ("Latin-1Supplement", 0x0080, 0x00FF),
    ("LatinExtended-A", 0x0100, 0x017F),
    ("LatinExtended-B", 0x0180, 0x024F),
    ("Greek", 0x0370, 0x03FF),
    ("GreekandCoptic", 0x0370, 0x03FF),
    ("Cyrillic", 0x0400, 0x04FF),
    ("Hebrew", 0x0590, 0x05FF),
    ("Arabic", 0x0600, 0x06FF),
    ("GeneralPunctuation", 0x2000, 0x206F),
    ("CurrencySymbols", 0x20A0, 0x20CF),
    ("Hiragana", 0x3040, 0x309F),
    ("Katakana", 0x30A0, 0x30FF),
    ("CJKUnifiedIdeographs", 0x4E00, 0x9FFF),
];

/// How a category name is looked up.
pub(super) enum CategoryName {
    Known(&'static str),
    Block(u32, u32),
    /// A block name that is not in the exact list (may or may not exist).
    UnknownBlock,
    /// Neither a category nor an `Is...` name: a syntax error.
    Invalid,
}

pub(super) fn lookup_category(name: &str) -> CategoryName {
    if let Some(c) = CATEGORIES.iter().find(|c| **c == name) {
        return CategoryName::Known(c);
    }
    match name.strip_prefix("Is") {
        Some(block) => match BLOCKS.iter().find(|(n, ..)| *n == block) {
            Some(&(_, lo, hi)) => CategoryName::Block(lo, hi),
            None => CategoryName::UnknownBlock,
        },
        None => CategoryName::Invalid,
    }
}

fn is_space_separator(c: char) -> bool {
    matches!(
        c as u32,
        0x20 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

/// Is `c` in the general category `cat` (one or two letters)? Exact for
/// `Cc`, `Z`, `Zs`, `Zl`, `Zp` everywhere (fixed sets) and for every category
/// on U+0000..=U+00FF; `None` otherwise.
fn in_category(cat: &str, c: char) -> Option<bool> {
    let cp = c as u32;
    match cat {
        "Cc" => return Some(c.is_control()),
        "Zs" => return Some(is_space_separator(c)),
        "Zl" => return Some(cp == 0x2028),
        "Zp" => return Some(cp == 0x2029),
        "Z" => return Some(is_space_separator(c) || cp == 0x2028 || cp == 0x2029),
        _ => {}
    }
    if cp > 0xFF {
        return None;
    }
    let actual = &LATIN1[2 * cp as usize..2 * cp as usize + 2];
    Some(if cat.len() == 1 {
        actual.starts_with(cat)
    } else {
        actual == cat
    })
}

/// `\w`: every character except those of the categories `P`, `Z`, `C`.
fn is_word(c: char) -> Option<bool> {
    if c as u32 > 0xFF {
        return None;
    }
    Some(!matches!(
        LATIN1.as_bytes()[2 * c as usize],
        b'P' | b'Z' | b'C'
    ))
}

/// `\i`: ASCII only (`[A-Za-z_:]`); the name-character tables differ between
/// the editions of XML 1.0, so anything non-ASCII is undecided.
fn is_name_start(c: char) -> Option<bool> {
    c.is_ascii()
        .then(|| c.is_ascii_alphabetic() || c == '_' || c == ':')
}

/// `\c`: ASCII only (`[A-Za-z0-9_:.-]`), see [`is_name_start`].
fn is_name_char(c: char) -> Option<bool> {
    c.is_ascii()
        .then(|| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.' | '-'))
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Pred {
    /// `\s`: space, tab, CR, LF (XML Schema).
    Space,
    NameStart,
    NameChar,
    /// `\d` = `\p{Nd}`.
    Digit,
    Word,
    Category(&'static str),
    Block(u32, u32),
}

impl Pred {
    fn eval(&self, c: char) -> Option<bool> {
        match self {
            Pred::Space => Some(matches!(c, ' ' | '\t' | '\n' | '\r')),
            Pred::NameStart => is_name_start(c),
            Pred::NameChar => is_name_char(c),
            Pred::Digit => in_category("Nd", c),
            Pred::Word => is_word(c),
            Pred::Category(cat) => in_category(cat, c),
            Pred::Block(lo, hi) => Some((*lo..=*hi).contains(&(c as u32))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ClassItem {
    Range(char, char),
    /// A predicate, complemented for `\S \I \C \D \W \P{..}`.
    Pred(Pred, bool),
}

/// `[...]`, `[^...]`, `[...-[...]]`, and the single-character escapes.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct Class {
    pub negated: bool,
    pub items: Vec<ClassItem>,
    pub subtract: Option<Box<Class>>,
}

impl Class {
    pub fn literal(c: char) -> Class {
        Class {
            items: vec![ClassItem::Range(c, c)],
            ..Class::default()
        }
    }

    pub fn eval(&self, c: char) -> Option<bool> {
        let mut any_unknown = false;
        let mut found = false;
        for item in &self.items {
            let answer = match item {
                ClassItem::Range(lo, hi) => Some((*lo..=*hi).contains(&c)),
                ClassItem::Pred(p, complement) => p.eval(c).map(|b| b != *complement),
            };
            match answer {
                Some(true) => {
                    found = true;
                    break;
                }
                None => any_unknown = true,
                Some(false) => {}
            }
        }
        let positive = if found {
            Some(true)
        } else if any_unknown {
            None
        } else {
            Some(false)
        };
        let positive = match (&self.subtract, positive) {
            (_, Some(false)) | (None, _) => positive,
            (Some(sub), base) => match sub.eval(c) {
                Some(true) => Some(false),
                Some(false) => base,
                None => None,
            },
        };
        positive.map(|b| b != self.negated)
    }

    /// The `(lo, hi)` ranges of this class and of its subtrahends.
    pub fn has_ranges(&self) -> bool {
        self.items.iter().any(|i| matches!(i, ClassItem::Range(..)))
            || self.subtract.as_ref().is_some_and(|s| s.has_ranges())
    }
}

/// The simple case mapping the `i` flag rests on (F&O 1.0 section 7.6.1.1).
/// std gives full mappings; where those are one character they are the simple
/// mappings. Where they are several, the simple mapping is the character itself
/// (`ß`, ligatures) except for U+0130 (simple lower case `i`) and the Greek
/// Extended characters with iota subscript (U+1F80..=U+1FFF), which have a
/// simple upper case mapping std does not expose: `None` there.
pub(super) fn simple_upper(c: char) -> Option<char> {
    let mut it = c.to_uppercase();
    match (it.next(), it.next()) {
        (Some(u), None) => Some(u),
        _ if (0x1F80..=0x1FFF).contains(&(c as u32)) => None,
        _ => Some(c),
    }
}

pub(super) fn simple_lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ if c == '\u{130}' => 'i',
        _ => c,
    }
}

/// A class under the `i` flag: a character matches if it is in the class, or
/// if its simple upper or lower case equals that of a member of a range of the
/// class (the sets `upper` and `lower`). Escapes (`\w`, `\p{Lu}`, ...) are not
/// affected by the flag. Only built for classes whose ranges are not negated.
#[derive(Debug, Clone)]
pub(super) struct FoldClass {
    pub class: Class,
    upper: Vec<char>,
    lower: Vec<char>,
    /// A range member has no known simple upper case mapping.
    fuzzy: bool,
}

/// Most characters an `i`-flag class may span in its ranges.
const FOLD_SPAN_LIMIT: u32 = 0x2_0000;

impl FoldClass {
    /// `None` if the ranges are too large to enumerate.
    pub fn new(class: Class) -> Option<FoldClass> {
        let mut span = 0u32;
        for item in &class.items {
            if let ClassItem::Range(lo, hi) = item {
                span = span.saturating_add(*hi as u32 - *lo as u32 + 1);
            }
        }
        if span > FOLD_SPAN_LIMIT {
            return None;
        }
        let (mut upper, mut lower, mut fuzzy) = (Vec::new(), Vec::new(), false);
        for item in &class.items {
            if let ClassItem::Range(lo, hi) = item {
                for cp in *lo as u32..=*hi as u32 {
                    let Some(c) = char::from_u32(cp) else {
                        continue;
                    };
                    match simple_upper(c) {
                        Some(u) => upper.push(u),
                        None => fuzzy = true,
                    }
                    lower.push(simple_lower(c));
                }
            }
        }
        upper.sort_unstable();
        upper.dedup();
        lower.sort_unstable();
        lower.dedup();
        Some(FoldClass {
            class,
            upper,
            lower,
            fuzzy,
        })
    }

    pub fn eval(&self, c: char) -> Option<bool> {
        let direct = self.class.eval(c);
        if direct == Some(true) {
            return direct;
        }
        match simple_upper(c) {
            None => None,
            Some(u) => {
                if self.upper.binary_search(&u).is_ok()
                    || self.lower.binary_search(&simple_lower(c)).is_ok()
                {
                    Some(true)
                } else if self.fuzzy {
                    None
                } else {
                    direct
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_table_has_two_letters_per_code_point() {
        assert_eq!(LATIN1.len(), 512);
        assert_eq!(in_category("Ll", 'ß'), Some(true));
        assert_eq!(in_category("Sm", '÷'), Some(true));
        assert_eq!(in_category("Lu", 'Ö'), Some(true));
        assert_eq!(in_category("Cc", '\u{9f}'), Some(true));
        assert_eq!(in_category("Zs", '\u{a0}'), Some(true));
        assert_eq!(in_category("Lu", 'ÿ'), Some(false));
    }
}
