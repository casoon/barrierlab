//! XPath 2.0 mode: `fn:matches`, `fn:replace`, `fn:tokenize` (F&O 1.0 section 7.6,
//! built on XML Schema Part 2 Appendix F). The examples of sections 7.6.2 to 7.6.4
//! are taken verbatim; the rest is written from the grammar of the sections
//! named in the comments.

#[allow(unused_imports)]
mod common;

use common::parse_xml;
use std::time::{Duration, Instant};
use xpath_eval::{
    Document, EvaluationContext, Item, Node, VersionedError, XPathVersion, evaluate_versioned,
    parse_versioned,
};

fn run(expr: &str) -> Result<Vec<String>, VersionedError> {
    let doc: &'static _ = Box::leak(Box::new(parse_xml("<a/>")));
    let element = doc
        .root()
        .children()
        .find(|c| c.expanded_name().is_some())
        .unwrap();
    let expression = parse_versioned(expr, XPathVersion::V2)?;
    let ctx = EvaluationContext::new(element);
    Ok(evaluate_versioned(&expression, &ctx)?
        .into_iter()
        .map(|i| match i {
            Item::Node(n) => n.string_value(),
            Item::Atomic(a) => a.to_xpath_string(),
        })
        .collect())
}

fn one(expr: &str) -> String {
    let mut v = run(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));
    assert_eq!(v.len(), 1, "{expr}");
    v.pop().unwrap()
}

fn error(expr: &str) -> VersionedError {
    match run(expr) {
        Err(e) => e,
        Ok(v) => panic!("{expr}: expected an error, got {v:?}"),
    }
}

fn code(expr: &str) -> String {
    match error(expr) {
        VersionedError::Dynamic { code, .. } => code.to_owned(),
        other => panic!("{expr}: not a dynamic error: {other}"),
    }
}

fn unsupported(expr: &str) -> String {
    let e = error(expr);
    assert!(e.is_unsupported(), "{expr}: {e}");
    e.to_string()
}

const POEM: &str = "'\nKaum hat dies der Hahn gesehen,\nFängt er auch schon an zu krähen:\nKikeriki! Kikikerikih!!\nTak, tak, tak! - da kommen sie.\n'";

// ---- F&O 7.6.2 fn:matches: the examples ----------------------------------------

#[test]
fn matches_spec_examples() {
    assert_eq!(one("matches('abracadabra', 'bra')"), "true");
    assert_eq!(one("matches('abracadabra', '^a.*a$')"), "true");
    assert_eq!(one("matches('abracadabra', '^bra')"), "false");
    assert_eq!(one(&format!("matches({POEM}, 'Kaum.*krähen')")), "false");
    assert_eq!(
        one(&format!("matches({POEM}, 'Kaum.*krähen', 's')")),
        "true"
    );
    assert_eq!(
        one(&format!("matches({POEM}, '^Kaum.*gesehen,$', 'm')")),
        "true"
    );
    assert_eq!(
        one(&format!("matches({POEM}, '^Kaum.*gesehen,$')")),
        "false"
    );
    assert_eq!(one(&format!("matches({POEM}, 'kiki', 'i')")), "true");
    assert_eq!(one(&format!("matches({POEM}, 'KIKI')")), "false");
}

// F&O 7.6.1.1, flag x
#[test]
fn flag_x() {
    assert_eq!(one("matches('helloworld', 'hello world', 'x')"), "true");
    assert_eq!(one("matches('helloworld', 'hello[ ]world', 'x')"), "false");
    assert_eq!(one("matches('hello world', 'hello[ ]world', 'x')"), "true");
    assert_eq!(one("matches('hello world', 'hello\\ sworld', 'x')"), "true");
    assert_eq!(one("matches('ab', 'a\n b', 'x')"), "true");
}

#[test]
fn matches_input_and_flags_edge_cases() {
    // an empty sequence is the zero-length string; the empty pattern matches
    assert_eq!(one("matches((), '^$')"), "true");
    assert_eq!(one("matches('abc', '')"), "true");
    // flags may repeat and come in any order; the empty flag string is fine
    assert_eq!(one("matches('A', 'a', 'iisi')"), "true");
    assert_eq!(one("matches('A', 'a', '')"), "false");
    // untyped values are accepted as strings
    assert_eq!(one("matches(/a, '^$')"), "true");
    assert_eq!(code("matches(1, 'a')"), "XPTY0004");
    assert_eq!(code("matches('a', ())"), "XPTY0004");
}

// ---- F&O 7.6.3 fn:replace: the examples ----------------------------------------

#[test]
fn replace_spec_examples() {
    assert_eq!(one("replace('abracadabra', 'bra', '*')"), "a*cada*");
    assert_eq!(one("replace('abracadabra', 'a.*a', '*')"), "*");
    assert_eq!(one("replace('abracadabra', 'a.*?a', '*')"), "*c*bra");
    assert_eq!(one("replace('abracadabra', 'a', '')"), "brcdbr");
    assert_eq!(
        one("replace('abracadabra', 'a(.)', 'a$1$1')"),
        "abbraccaddabbra"
    );
    assert_eq!(code("replace('abracadabra', '.*?', '$1')"), "FORX0003");
    assert_eq!(one("replace('AAAA', 'A+', 'b')"), "b");
    assert_eq!(one("replace('AAAA', 'A+?', 'b')"), "bbbb");
    assert_eq!(one("replace('darted', '^(.*?)d(.*)$', '$1c$2')"), "carted");
    assert_eq!(
        one("replace('abcd', '(ab)|(a)', '[1=$1][2=$2]')"),
        "[1=ab][2=]cd"
    );
}

#[test]
fn replace_replacement_string() {
    assert_eq!(one("replace('abc', 'b', '$0$0')"), "abbc");
    assert_eq!(one(r"replace('abc', 'b', '\$')"), "a$c");
    assert_eq!(one(r"replace('abc', 'b', '\\')"), r"a\c");
    // a group that does not exist inserts nothing
    assert_eq!(one("replace('abc', 'b', '[$3]')"), "a[]c");
    // digits after $ are taken while they name an existing group
    assert_eq!(one("replace('a', '(a)', '$10')"), "a0");
    assert_eq!(
        one("replace('abcdefghijk', '(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)', '$11-$10-$1')"),
        "k-j-a"
    );
    // nothing to replace: the input; empty input stays empty
    assert_eq!(one("replace('abc', 'x', 'y')"), "abc");
    assert_eq!(one("replace((), 'x', 'y')"), "");
    assert_eq!(one("replace('Hello', 'L', 'x', 'i')"), "Hexxo");
    // matches are non-overlapping and found left to right
    assert_eq!(one("replace('aaa', 'aa', 'b')"), "ba");
}

// ---- F&O 7.6.4 fn:tokenize: the examples ---------------------------------------

#[test]
fn tokenize_spec_examples() {
    assert_eq!(
        run("tokenize('The cat sat on the mat', '\\s+')").unwrap(),
        ["The", "cat", "sat", "on", "the", "mat"]
    );
    assert_eq!(
        run("tokenize('1, 15, 24, 50', ',\\s*')").unwrap(),
        ["1", "15", "24", "50"]
    );
    assert_eq!(
        run("tokenize('1,15,,24,50,', ',')").unwrap(),
        ["1", "15", "", "24", "50", ""]
    );
    assert_eq!(code("tokenize('abba', '.?')"), "FORX0003");
    assert_eq!(
        run("tokenize('Some unparsed <br> HTML <BR> text', '\\s*<br>\\s*', 'i')").unwrap(),
        ["Some unparsed", "HTML", "text"]
    );
    assert_eq!(
        run("tokenize('abracadabra', '(ab)|(a)')").unwrap(),
        ["", "r", "c", "d", "r", ""]
    );
}

#[test]
fn tokenize_edge_cases() {
    assert!(run("tokenize('', ',')").unwrap().is_empty());
    assert!(run("tokenize((), ',')").unwrap().is_empty());
    // a bad pattern is an error even when there is nothing to split
    assert_eq!(code("tokenize('', '.?')"), "FORX0003");
    assert_eq!(run("tokenize('abc', 'x')").unwrap(), ["abc"]);
    assert_eq!(run("tokenize('abc', 'abc')").unwrap(), ["", ""]);
}

// ---- errors: FORX0001 to FORX0004 -----------------------------------------------

#[test]
fn flag_and_pattern_errors() {
    for flags in ["q", "ii!", "S", "i s", "u"] {
        assert_eq!(
            code(&format!("matches('a', 'a', '{flags}')")),
            "FORX0001",
            "{flags}"
        );
        assert_eq!(
            code(&format!("replace('a', 'a', 'b', '{flags}')")),
            "FORX0001"
        );
        assert_eq!(code(&format!("tokenize('a', 'a', '{flags}')")), "FORX0001");
    }
    // clear syntax errors: FORX0002
    for pattern in [
        "(",
        ")",
        "(a",
        "a)",
        "[",
        "[a",
        "]",
        "}",
        "{",
        "*a",
        "+",
        "?",
        "a**",
        "a+*",
        "a?+",
        "a{2,1}",
        "a{,3}",
        "a{x}",
        "a{1",
        "a{1,2",
        "\\q",
        "\\",
        "[z-a]",
        "[]",
        "[^]",
        "\\p{Foo}",
        "\\p",
        "\\p{L",
        "(?:a)",
        "[a-[b]c]",
        "\\0",
        "(a)\\2",
        "\\1(a)",
        "(a\\1)",
        "a{1000}{1000}",
        "[a[b]]",
        "[a-\\d]",
    ] {
        assert_eq!(
            code(&format!("matches('a', '{pattern}')")),
            "FORX0002",
            "{pattern:?}"
        );
    }
}

#[test]
fn zero_length_pattern_errors() {
    for pattern in ["a*", "^", "$", "()", "(a|)", "x?", "^$", "a{0}", "a*?"] {
        assert_eq!(
            code(&format!("replace('b', '{pattern}', 'x')")),
            "FORX0003",
            "{pattern}"
        );
        assert_eq!(
            code(&format!("tokenize('b', '{pattern}')")),
            "FORX0003",
            "{pattern}"
        );
    }
    // matches() accepts them: only replace and tokenize reject
    assert_eq!(one("matches('b', 'a*')"), "true");
}

#[test]
fn replacement_string_errors() {
    for replacement in ["$", "a$", "$x", "$-1", "\\", "\\n", "a\\"] {
        assert_eq!(
            code(&format!("replace('a', 'a', '{replacement}')")),
            "FORX0004",
            "{replacement}"
        );
    }
    // reported even when nothing matches
    assert_eq!(code("replace('b', 'a', '$')"), "FORX0004");
}

// ---- the grammar of XML Schema Part 2, Appendix F --------------------------------

#[test]
fn branches_pieces_and_quantifiers() {
    assert_eq!(one("matches('ab', '^(a|b)*$')"), "true");
    assert_eq!(one("matches('abc', '^(a|b)*$')"), "false");
    assert_eq!(one("matches('aaa', '^a{3}$')"), "true");
    assert_eq!(one("matches('aaaa', '^a{3}$')"), "false");
    assert_eq!(one("matches('aaaa', '^a{2,}$')"), "true");
    assert_eq!(one("matches('a', '^a{2,}$')"), "false");
    assert_eq!(one("matches('aaaa', '^a{2,3}$')"), "false");
    assert_eq!(one("matches('', '^a{0}$')"), "true");
    assert_eq!(one("matches('abab', '^(ab){2}$')"), "true");
    assert_eq!(one("matches('', '^(a|)$')"), "true");
    assert_eq!(one("matches('x', '^a?x$')"), "true");
    assert_eq!(one("replace('aaaa', 'a{2}?', 'b')"), "bb");
    assert_eq!(one("replace('aaaaa', 'a{1,3}?', 'b')"), "bbbbb");
    assert_eq!(one("replace('aaaaa', 'a{1,3}', 'b')"), "bb");
    assert_eq!(one("replace('a{1}', 'a\\{1\\}', 'b')"), "b");
}

#[test]
fn single_character_escapes() {
    assert_eq!(one("matches('a\tb', 'a\\tb')"), "true");
    assert_eq!(one("matches('a\nb', 'a\\nb')"), "true");
    assert_eq!(one("matches('a\rb', 'a\\rb')"), "true");
    for (text, pattern) in [
        ("\\", "\\\\"),
        ("|", "\\|"),
        (".", "\\."),
        ("-", "\\-"),
        ("^", "\\^"),
        ("?", "\\?"),
        ("*", "\\*"),
        ("+", "\\+"),
        ("{", "\\{"),
        ("}", "\\}"),
        ("(", "\\("),
        (")", "\\)"),
        ("[", "\\["),
        ("]", "\\]"),
        ("$", "\\$"),
    ] {
        assert_eq!(
            one(&format!("matches('{text}', '^{pattern}$')")),
            "true",
            "{pattern}"
        );
        assert_eq!(
            one(&format!("matches('x', '^{pattern}$')")),
            "false",
            "{pattern}"
        );
        assert_eq!(
            one(&format!("matches('{text}', '^[{pattern}]$')")),
            "true",
            "[{pattern}]"
        );
    }
    assert_eq!(one("matches('a.b', '^a.b$')"), "true");
    assert_eq!(one("matches('axb', '^a\\.b$')"), "false");
}

#[test]
fn dot_and_line_boundaries() {
    assert_eq!(one("matches('a\nb', 'a.b')"), "false");
    assert_eq!(one("matches('a\rb', 'a.b')"), "false");
    assert_eq!(one("matches('a\nb', 'a.b', 's')"), "true");
    assert_eq!(one("matches('a\nb', '^b')"), "false");
    assert_eq!(one("matches('a\nb', '^b', 'm')"), "true");
    assert_eq!(one("matches('a\nb', 'a$', 'm')"), "true");
    assert_eq!(one("matches('a\nb', 'a$')"), "false");
    // $ without m is the end of the string, not before a trailing newline
    assert_eq!(one("matches('a\n', 'a$')"), "false");
    // only LF is a line boundary
    assert_eq!(one("matches('a\rb', '^b', 'm')"), "false");
}

#[test]
fn character_classes() {
    assert_eq!(one("matches('b', '^[abc]$')"), "true");
    assert_eq!(one("matches('d', '^[abc]$')"), "false");
    assert_eq!(one("matches('m', '^[a-z]$')"), "true");
    assert_eq!(one("matches('M', '^[a-z]$')"), "false");
    assert_eq!(one("matches('M', '^[^a-z]$')"), "true");
    assert_eq!(one("matches('-', '^[-a]$')"), "true");
    assert_eq!(one("matches('-', '^[a-]$')"), "true");
    assert_eq!(one("matches('-', '^[^-a]$')"), "false");
    assert_eq!(one("matches('^', '^[a^]$')"), "true");
    assert_eq!(one("matches('.', '^[.]$')"), "true");
    assert_eq!(one("matches('x', '^[.]$')"), "false");
    assert_eq!(one("matches('5', '^[a-c0-9]$')"), "true");
    assert_eq!(one("matches('é', '^[à-ü]$')"), "true");
    // subtraction
    assert_eq!(one("matches('b', '^[a-z-[aeiou]]$')"), "true");
    assert_eq!(one("matches('e', '^[a-z-[aeiou]]$')"), "false");
    assert_eq!(one("matches('e', '^[^a-z-[aeiou]]$')"), "true");
    assert_eq!(one("matches('b', '^[a-z-[aeiou-[b-d]]]$')"), "true");
    assert_eq!(one("matches('a', '^[a-z-[aeiou-[b-d]]]$')"), "false");
    // an ambiguous `-` is named, not guessed
    assert!(unsupported("matches('a', '[a-c-e]')").contains("`-`"));
}

#[test]
fn multi_character_escapes() {
    // \s is space, tab, CR, LF and nothing else
    for c in [" ", "\t", "\n", "\r"] {
        assert_eq!(one(&format!("matches('{c}', '^\\s$')")), "true");
        assert_eq!(one(&format!("matches('{c}', '^\\S$')")), "false");
    }
    assert_eq!(one("matches('\u{a0}', '^\\s$')"), "false");
    assert_eq!(one("matches('x', '^\\S$')"), "true");
    assert_eq!(one("matches('x y', '^\\S\\s\\S$')"), "true");
    assert_eq!(one("matches(' ', '^[\\s]$')"), "true");
    assert_eq!(one("matches('a', '^[\\S]$')"), "true");
    assert_eq!(one("matches('5', '^\\d$')"), "true");
    assert_eq!(one("matches('a', '^\\d$')"), "false");
    assert_eq!(one("matches('a', '^\\D$')"), "true");
    assert_eq!(one("matches('é', '^\\w$')"), "true");
    assert_eq!(one("matches('_', '^\\w$')"), "false"); // Pc is punctuation
    // symbols are word characters in XSD (everything but P, Z, C)
    assert_eq!(one("matches('+', '^\\w$')"), "true");
    assert_eq!(one("matches(' ', '^\\w$')"), "false");
    assert_eq!(one("matches('-', '^\\w$')"), "false");
    assert_eq!(one("matches(' ', '^\\W$')"), "true");
    assert_eq!(one("matches('a1', '^\\i\\c$')"), "true");
    assert_eq!(one("matches('1a', '^\\i\\c$')"), "false");
    assert_eq!(one("matches('-', '^\\i$')"), "false");
    assert_eq!(one("matches('-', '^\\c$')"), "true");
    assert_eq!(one("matches('-', '^\\C$')"), "false");
    assert_eq!(one("matches(':', '^\\I$')"), "false");
    assert_eq!(one("matches('a-b.c_d:e', '^\\i\\c*$')"), "true");
}

#[test]
fn category_and_block_escapes() {
    assert_eq!(one("matches('A', '^\\p{Lu}$')"), "true");
    assert_eq!(one("matches('a', '^\\p{Lu}$')"), "false");
    assert_eq!(one("matches('a', '^\\P{Lu}$')"), "true");
    assert_eq!(one("matches('a', '^\\p{L}$')"), "true");
    assert_eq!(one("matches('é', '^\\p{Ll}$')"), "true");
    assert_eq!(one("matches('ß', '^\\p{Ll}$')"), "true");
    assert_eq!(one("matches('£', '^\\p{Sc}$')"), "true");
    assert_eq!(one("matches('7', '^\\p{N}$')"), "true");
    assert_eq!(one("matches('²', '^\\p{No}$')"), "true");
    assert_eq!(one("matches('!', '^\\p{P}$')"), "true");
    assert_eq!(one("matches(',', '^\\p{Po}$')"), "true");
    assert_eq!(one("matches('a', '^[\\p{Lu}\\p{Nd}]$')"), "false");
    assert_eq!(one("matches('9', '^[\\p{Lu}\\p{Nd}]$')"), "true");
    // categories decided exactly everywhere
    assert_eq!(one("matches('\u{2003}', '^\\p{Zs}$')"), "true");
    assert_eq!(one("matches('\u{2028}', '^\\p{Zl}$')"), "true");
    assert_eq!(one("matches('\u{2029}', '^\\p{Zp}$')"), "true");
    assert_eq!(one("matches('\u{3000}', '^\\p{Z}$')"), "true");
    assert_eq!(one("matches('\u{4E00}', '^\\p{Z}$')"), "false");
    assert_eq!(one("matches('\u{85}', '^\\p{Cc}$')"), "true");
    assert_eq!(one("matches('\u{4E00}', '^\\p{Cc}$')"), "false");
    // blocks
    assert_eq!(one("matches('a', '^\\p{IsBasicLatin}$')"), "true");
    assert_eq!(one("matches('é', '^\\p{IsBasicLatin}$')"), "false");
    assert_eq!(one("matches('é', '^\\p{IsLatin-1Supplement}$')"), "true");
    assert_eq!(one("matches('é', '^\\P{IsBasicLatin}$')"), "true");
    assert_eq!(one("matches('\u{3b1}', '^\\p{IsGreek}$')"), "true");
    assert_eq!(
        one("matches('\u{4E00}', '^\\p{IsCJKUnifiedIdeographs}$')"),
        "true"
    );
}

#[test]
fn undecidable_unicode_is_unsupported_not_a_value() {
    // no Unicode database: the character is named, nothing is guessed
    for expr in [
        "matches('\u{436}', '\\p{L}')",
        "matches('\u{663}', '\\d')",
        "matches('\u{436}', '\\w')",
        "matches('\u{436}', '\\i')",
        "matches('\u{e9}\u{436}', '^\\c*$')",
        "matches('\u{436}', '[^\\p{Lu}]')",
    ] {
        let e = unsupported(expr);
        assert!(e.contains("U+"), "{e}");
    }
    // a position the match never reaches does not matter
    assert_eq!(one("matches('a\u{436}', '^a')"), "true");
    // a character the class already accepts needs no category data
    assert_eq!(one("matches('\u{436}', '[\u{436}\\p{L}]')"), "true");
    // unknown block names may or may not exist: unsupported
    assert!(unsupported("matches('a', '\\p{IsTibetan}')").contains("IsTibetan"));
    // wrong category and block names are syntax errors
    assert_eq!(code("matches('a', '\\p{Xx}')"), "FORX0002");
    assert_eq!(code("matches('a', '\\p{Cs}')"), "FORX0002");
}

#[test]
fn back_references_are_unsupported() {
    let e = unsupported("matches('aa', '(a)\\1')");
    assert!(e.contains("back-reference"), "{e}");
    assert!(unsupported("replace('aa', '(a)\\1', 'b')").contains("back-reference"));
    // but a reference to a group that cannot be referred to is a syntax error
    assert_eq!(code("matches('a', '(a)\\2')"), "FORX0002");
    assert_eq!(code("matches('a', '\\1(a)')"), "FORX0002");
}

// ---- flag i (F&O 7.6.1.1) ------------------------------------------------------

#[test]
fn flag_i_case_mapping() {
    assert_eq!(one("matches('ABC', '^abc$', 'i')"), "true");
    assert_eq!(one("matches('abc', '^ABC$', 'i')"), "true");
    assert_eq!(one("matches('abc', '^ABD$', 'i')"), "false");
    assert_eq!(one("matches('B', '^[a-c]$', 'i')"), "true");
    assert_eq!(one("matches('b', '^[A-C]$', 'i')"), "true");
    assert_eq!(one("matches('D', '^[a-c]$', 'i')"), "false");
    assert_eq!(one("matches('Ä', '^ä$', 'i')"), "true");
    assert_eq!(one("matches('\u{3A3}', '^\u{3C3}$', 'i')"), "true");
    // digits and punctuation are unaffected
    assert_eq!(one("matches('1', '^1$', 'i')"), "true");
    // the Kelvin sign has the lower case mapping k: same lower case, so it matches
    assert_eq!(one("matches('\u{212A}', '^k$', 'i')"), "true");
    assert_eq!(one("matches('\u{212A}', '^k$')"), "false");
    // escapes are not affected by the flag
    assert_eq!(one("matches('a', '^\\p{Lu}$', 'i')"), "false");
    assert_eq!(one("matches('A', '^\\p{Lu}$', 'i')"), "true");
    // a negated or subtracted class over ranges is not decided
    assert!(unsupported("matches('A', '[^a]', 'i')").contains("`i` flag"));
    assert!(unsupported("matches('A', '[a-z-[x]]', 'i')").contains("`i` flag"));
    // ... but a negated escape is plain
    assert_eq!(one("matches('A', '^[^\\s]$', 'i')"), "true");
    // a character without a known simple mapping is named
    assert!(unsupported("matches('\u{1F88}', '^a$', 'i')").contains("U+1F88"));
}

// ---- limits: pathological patterns and inputs ----------------------------------------

fn quickly(expr: &str) -> Result<Vec<String>, VersionedError> {
    let start = Instant::now();
    let r = run(expr);
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?} for a pathological pattern",
        start.elapsed()
    );
    r
}

#[test]
fn pathological_patterns_finish_quickly() {
    let a = "a".repeat(5000);
    // catastrophic in a backtracking engine
    assert_eq!(
        quickly(&format!("matches('{a}', '(a*)*b')")).unwrap(),
        ["false"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}b', '(a*)*b')")).unwrap(),
        ["true"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}b', '^(a|aa)+$')")).unwrap(),
        ["false"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}', '(a|aa)+$')")).unwrap(),
        ["true"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}', '^(a+)+$')")).unwrap(),
        ["true"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}c', '^(a+)+$')")).unwrap(),
        ["false"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}', '(x+x+)+y')")).unwrap(),
        ["false"]
    );
    assert_eq!(
        quickly(&format!("matches('{a}', '(.*)*(.*)*(.*)*z')")).unwrap(),
        ["false"]
    );
    // an input far longer than any stack: the machine does not recurse on it
    let long = "a".repeat(200_000);
    assert_eq!(
        quickly(&format!("matches('{long}', 'a*b')")).unwrap(),
        ["false"]
    );
    assert_eq!(
        quickly(&format!("string-length(replace('{long}', 'a', 'bb'))")).unwrap(),
        ["400000"]
    );
    assert_eq!(
        quickly(&format!("count(tokenize('{long}', 'a'))")).unwrap(),
        ["200001"]
    );
}

#[test]
fn nesting_and_expansion_are_capped_with_a_named_error() {
    let deep = format!("{}a{}", "(".repeat(600), ")".repeat(600));
    let e = quickly(&format!("matches('a', '{deep}')")).unwrap_err();
    assert!(
        e.is_unsupported() && e.to_string().contains("capture groups"),
        "{e}"
    );
    let deep = format!("[{}x{}", "a-[".repeat(600), "]".repeat(601));
    let e = quickly(&format!("matches('a', '{deep}')")).unwrap_err();
    assert!(
        e.is_unsupported() && e.to_string().contains("nested deeper"),
        "{e}"
    );
    // the quantifier after a quantifier is not in the grammar
    assert_eq!(code("matches('a', 'a{1000}{1000}')"), "FORX0002");
    // expanded counted quantifiers hit the program cap
    for pattern in [
        "(a{1000}){1000}",
        "a{100000}",
        "a{0,100000}",
        "((a{100}){100}){100}",
        "(a{1,99999999999})",
    ] {
        let e = quickly(&format!("matches('a', '{pattern}')")).unwrap_err();
        assert!(
            e.is_unsupported() && e.to_string().contains("instructions"),
            "{pattern}: {e}"
        );
    }
    // within the cap they work
    assert_eq!(one("matches('aaaaaaaaaa', '^a{10}$')"), "true");
    let many = "a".repeat(2000);
    assert_eq!(one(&format!("matches('{many}', '^a{{2000}}$')")), "true");
    let groups = "()".repeat(101);
    assert!(unsupported(&format!("matches('a', '{groups}')")).contains("capture groups"));
    let alternatives = vec!["x"; 3000].join("|");
    assert_eq!(one(&format!("matches('x', '^({alternatives})$')")), "true");
}

#[test]
fn many_matches_stay_linear() {
    let text = "ab ".repeat(20_000);
    assert_eq!(
        quickly(&format!("count(tokenize('{text}', '\\s+'))")).unwrap(),
        ["20001"]
    );
    assert_eq!(
        quickly(&format!(
            "string-length(replace('{text}', '(a)(b)', '$2$1'))"
        ))
        .unwrap(),
        ["60000"]
    );
}

// ---- static behaviour ----------------------------------------------------------------

#[test]
fn arity_is_checked() {
    assert!(matches!(
        error("matches('a')"),
        VersionedError::Static { .. }
    ));
    assert!(matches!(
        error("replace('a', 'a')"),
        VersionedError::Static { .. }
    ));
    assert!(matches!(
        error("tokenize('a')"),
        VersionedError::Static { .. }
    ));
    assert!(matches!(
        error("matches('a', 'a', 'i', 'x')"),
        VersionedError::Static { .. }
    ));
}
