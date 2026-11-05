//! Runs the tokenizer against the vendored html5lib-tests tokenizer corpus
//! (`tests/html5lib-tests/tokenizer/*.test`, vendored by
//! `xtask/fetch-html5lib-tests.py`).
//!
//! Lives inside the crate (child module of `tokenizer`) because the
//! tokenizer is `pub(crate)`; an integration test could not reach it.
//!
//! Each test case runs once per entry of its `initialStates` (default:
//! "Data state"). Compared: the token stream (adjacent `Character` tokens
//! merged), not the `errors` list — error codes are covered by the
//! per-kind table test in `lib.rs`.
//!
//! Known failures are listed explicitly in
//! `tests/html5lib_tokenizer_known_failures.txt` (`<file>#<index>@<state>`
//! per line). An unlisted failure fails the test, and so does a listed
//! entry that now passes. Regenerate the list (after investigating each
//! new entry) with `HTML5LIB_BLESS=1 cargo test -p html5-parser
//! tokenizer_conformance`.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::{State, Token, TokenKind, Tokenizer};

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/html5lib-tests/tokenizer"
);
const KNOWN_FAILURES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/html5lib_tokenizer_known_failures.txt"
);
const HEADER_LAST_LINE: &str = "# --- entries below, one per line ---\n";

// ---- minimal JSON reader (no dependencies in this crate) ----

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while matches!(self.bytes.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }
    fn eat(&mut self, b: u8) {
        self.ws();
        assert_eq!(
            self.bytes.get(self.pos),
            Some(&b),
            "json: expected {}",
            b as char
        );
        self.pos += 1;
    }
    fn value(&mut self) -> Json {
        self.ws();
        match self.bytes[self.pos] {
            b'{' => {
                self.pos += 1;
                let mut fields = Vec::new();
                loop {
                    self.ws();
                    if self.bytes[self.pos] == b'}' {
                        self.pos += 1;
                        break;
                    }
                    if !fields.is_empty() {
                        self.eat(b',');
                    }
                    self.ws();
                    let key = self.string();
                    self.eat(b':');
                    fields.push((key, self.value()));
                }
                Json::Obj(fields)
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.bytes[self.pos] == b']' {
                        self.pos += 1;
                        break;
                    }
                    if !items.is_empty() {
                        self.eat(b',');
                    }
                    items.push(self.value());
                }
                Json::Arr(items)
            }
            b'"' => Json::Str(self.string()),
            b't' => {
                self.pos += 4;
                Json::Bool(true)
            }
            b'f' => {
                self.pos += 5;
                Json::Bool(false)
            }
            b'n' => {
                self.pos += 4;
                Json::Null
            }
            _ => {
                let start = self.pos;
                while matches!(
                    self.bytes[self.pos],
                    b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                ) {
                    self.pos += 1;
                }
                let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
                Json::Num(text.parse().expect("json number"))
            }
        }
    }
    fn hex4(&mut self) -> u32 {
        let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).unwrap();
        self.pos += 4;
        u32::from_str_radix(text, 16).expect("json \\u escape")
    }
    /// Decodes a JSON string. A lone surrogate (not representable as a
    /// Rust `char`) is returned as U+FFFD — only `doubleEscaped` tests
    /// carry those, and they are decoded separately by [`unescape_double`].
    fn string(&mut self) -> String {
        assert_eq!(self.bytes[self.pos], b'"');
        self.pos += 1;
        let mut out = String::new();
        loop {
            let b = self.bytes[self.pos];
            self.pos += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let e = self.bytes[self.pos];
                    self.pos += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'/' => out.push('/'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        b'u' => {
                            let mut code = self.hex4();
                            if (0xD800..0xDC00).contains(&code)
                                && self.bytes[self.pos..].starts_with(b"\\u")
                            {
                                let save = self.pos;
                                self.pos += 2;
                                let low = self.hex4();
                                if (0xDC00..0xE000).contains(&low) {
                                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                } else {
                                    self.pos = save;
                                }
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        other => panic!("json: bad escape \\{}", other as char),
                    }
                }
                _ => {
                    // Copy the full UTF-8 sequence starting at this byte.
                    let start = self.pos - 1;
                    let len = match b {
                        0..=0x7F => 1,
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        _ => 4,
                    };
                    self.pos = start + len;
                    out.push_str(std::str::from_utf8(&self.bytes[start..self.pos]).unwrap());
                }
            }
        }
        out
    }
}

/// `doubleEscaped` tests encode the real text with `\uXXXX` sequences
/// that survive one JSON decode. Unpaired surrogates become U+FFFD (a
/// Rust `&str` cannot hold them, so the tokenizer never sees them).
fn unescape_double(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let hex = |at: usize| -> Option<u32> {
        let s: String = chars.get(at..at + 4)?.iter().collect();
        u32::from_str_radix(&s, 16).ok()
    };
    while i < chars.len() {
        if chars[i] == '\\'
            && chars.get(i + 1) == Some(&'u')
            && let Some(mut code) = hex(i + 2)
        {
            {
                i += 6;
                if (0xD800..0xDC00).contains(&code)
                    && chars.get(i) == Some(&'\\')
                    && chars.get(i + 1) == Some(&'u')
                    && let Some(low) = hex(i + 2)
                    && (0xDC00..0xE000).contains(&low)
                {
                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                    i += 6;
                }
                out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// ---- token comparison ----

/// Canonical text form of a token stream, shared by expected and actual
/// side so a mismatch prints readably.
fn render_actual(tokens: &[Token]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, out: &mut Vec<String>| {
        if !text.is_empty() {
            out.push(format!("Character {text:?}"));
            text.clear();
        }
    };
    for token in tokens {
        match &token.kind {
            TokenKind::Character(c) => text.push(*c),
            TokenKind::Eof => {}
            other => {
                flush(&mut text, &mut out);
                out.push(match other {
                    TokenKind::Doctype(d) => format!(
                        "DOCTYPE {:?} {:?} {:?} correct={}",
                        d.name, d.public_identifier, d.system_identifier, !d.force_quirks
                    ),
                    TokenKind::StartTag(t) => format!(
                        "StartTag {:?} {:?} selfclosing={}",
                        t.name,
                        sorted_attrs(
                            t.attributes
                                .iter()
                                .map(|a| (a.name.clone(), a.value.clone()))
                        ),
                        t.self_closing
                    ),
                    TokenKind::EndTag(t) => format!("EndTag {:?}", t.name),
                    TokenKind::Comment(c) => format!("Comment {c:?}"),
                    TokenKind::ProcessingInstruction(p) => {
                        format!("ProcessingInstruction {:?} {:?}", p.target, p.data)
                    }
                    TokenKind::Character(_) | TokenKind::Eof => unreachable!(),
                });
            }
        }
    }
    flush(&mut text, &mut out);
    out
}

fn sorted_attrs(attrs: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut v: Vec<_> = attrs.collect();
    v.sort();
    v
}

fn render_expected(output: &[Json], double_escaped: bool) -> Vec<String> {
    let dec = |s: &str| {
        if double_escaped {
            unescape_double(s)
        } else {
            s.to_owned()
        }
    };
    let opt = |j: &Json| match j {
        Json::Null => None,
        Json::Str(s) => Some(dec(s)),
        other => panic!("unexpected {other:?}"),
    };
    let mut out: Vec<String> = Vec::new();
    for item in output {
        let Some(parts) = item.as_arr() else {
            // "ParseError" bare strings in old-format files.
            continue;
        };
        let kind = parts[0].as_str().unwrap();
        match kind {
            "Character" => {
                // Adjacent Character entries are one run.
                let text = dec(parts[1].as_str().unwrap());
                if let Some(last) = out.last_mut()
                    && let Some(prev) = last.strip_prefix("Character ")
                {
                    let prev: String = serde_unquote(prev);
                    *last = format!("Character {:?}", prev + &text);
                } else {
                    out.push(format!("Character {text:?}"));
                }
            }
            "Comment" => out.push(format!("Comment {:?}", dec(parts[1].as_str().unwrap()))),
            "ProcessingInstruction" => out.push(format!(
                "ProcessingInstruction {:?} {:?}",
                dec(parts[1].as_str().unwrap()),
                dec(parts[2].as_str().unwrap())
            )),
            "EndTag" => out.push(format!("EndTag {:?}", dec(parts[1].as_str().unwrap()))),
            "StartTag" => {
                let attrs = match &parts[2] {
                    Json::Obj(fields) => sorted_attrs(
                        fields
                            .iter()
                            .map(|(k, v)| (dec(k), dec(v.as_str().unwrap()))),
                    ),
                    other => panic!("attrs {other:?}"),
                };
                let selfclosing = matches!(parts.get(3), Some(Json::Bool(true)));
                out.push(format!(
                    "StartTag {:?} {:?} selfclosing={selfclosing}",
                    dec(parts[1].as_str().unwrap()),
                    attrs
                ));
            }
            "DOCTYPE" => out.push(format!(
                "DOCTYPE {:?} {:?} {:?} correct={}",
                opt(&parts[1]),
                opt(&parts[2]),
                opt(&parts[3]),
                matches!(parts[4], Json::Bool(true))
            )),
            other => panic!("unknown expected token kind {other}"),
        }
    }
    out
}

/// Inverse of `{:?}` for the plain string we just formatted ourselves.
fn serde_unquote(debug: &str) -> String {
    // Re-derive from the Debug form: only `\\`, `\"`, `\n`, `\r`, `\t`,
    // `\0`, `\'` and `\u{..}` can appear.
    let inner = &debug[1..debug.len() - 1];
    let mut out = String::new();
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next().unwrap() {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            'u' => {
                assert_eq!(it.next(), Some('{'));
                let mut hex = String::new();
                for h in it.by_ref() {
                    if h == '}' {
                        break;
                    }
                    hex.push(h);
                }
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
            }
            other => panic!("unexpected escape \\{other}"),
        }
    }
    out
}

fn initial_state(name: &str) -> State {
    match name {
        "Data state" => State::Data,
        "PLAINTEXT state" => State::PlainText,
        "RCDATA state" => State::RcData,
        "RAWTEXT state" => State::RawText,
        "Script data state" => State::ScriptData,
        "CDATA section state" => State::CdataSection,
        other => panic!("unknown initial state {other}"),
    }
}

fn run_case(input: &str, state: &str, last_start_tag: Option<&str>) -> Vec<String> {
    let mut tokenizer = Tokenizer::new(input);
    tokenizer.state = initial_state(state);
    tokenizer.last_start_tag_name = last_start_tag.map(str::to_owned);
    // CDATA sections are only entered from foreign content.
    tokenizer.in_foreign_content = state == "CDATA section state";
    let tokens: Vec<Token> = tokenizer.collect();
    render_actual(&tokens)
}

fn load_known_failures() -> HashSet<String> {
    fs::read_to_string(KNOWN_FAILURES_PATH)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().next().unwrap_or(l).to_owned())
        .collect()
}

#[test]
fn html5lib_tokenizer_conformance() {
    let mut files: Vec<PathBuf> = fs::read_dir(DIR)
        .unwrap_or_else(|e| {
            panic!(
                "reading {DIR}: {e} — run `python3 xtask/fetch-html5lib-tests.py --tokenizer-only`"
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("test"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no tokenizer .test files vendored");

    let known = load_known_failures();
    let mut failing: Vec<(String, String)> = Vec::new();
    let mut total = 0usize;
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = fs::read(path).unwrap();
        let root = JsonParser {
            bytes: &bytes,
            pos: 0,
        }
        .value();
        let tests = root
            .get("tests")
            .and_then(Json::as_arr)
            .expect("tests array");
        for (index, case) in tests.iter().enumerate() {
            let double = matches!(case.get("doubleEscaped"), Some(Json::Bool(true)));
            let raw_input = case.get("input").and_then(Json::as_str).unwrap();
            let input = if double {
                unescape_double(raw_input)
            } else {
                raw_input.to_owned()
            };
            let last = case.get("lastStartTag").and_then(Json::as_str);
            let expected =
                render_expected(case.get("output").and_then(Json::as_arr).unwrap(), double);
            let states: Vec<&str> = match case.get("initialStates").and_then(Json::as_arr) {
                Some(list) => list.iter().filter_map(Json::as_str).collect(),
                None => vec!["Data state"],
            };
            for state in states {
                total += 1;
                let actual = run_case(&input, state, last);
                if actual != expected {
                    let key = format!("{name}#{index}@{}", state.replace(' ', "_"));
                    let desc = case.get("description").and_then(Json::as_str).unwrap_or("");
                    failing.push((
                        key.clone(),
                        format!(
                            "{key} ({desc})\n    input:    {input:?}\n    expected: {expected:?}\n    actual:   {actual:?}"
                        ),
                    ));
                }
            }
        }
    }

    if std::env::var_os("HTML5LIB_BLESS").is_some() {
        let current = fs::read_to_string(KNOWN_FAILURES_PATH).unwrap();
        let header_end = current.find(HEADER_LAST_LINE).expect("header") + HEADER_LAST_LINE.len();
        let mut out = current[..header_end].to_owned();
        for (key, _) in &failing {
            out.push_str(key);
            out.push('\n');
        }
        fs::write(KNOWN_FAILURES_PATH, out).unwrap();
        eprintln!(
            "blessed {} of {total} tokenizer cases as known failures",
            failing.len()
        );
        return;
    }

    let failing_keys: HashSet<&str> = failing.iter().map(|(k, _)| k.as_str()).collect();
    let unexpected: Vec<&str> = failing
        .iter()
        .filter(|(k, _)| !known.contains(k))
        .map(|(_, d)| d.as_str())
        .collect();
    let mut fixed: Vec<&String> = known
        .iter()
        .filter(|k| !failing_keys.contains(k.as_str()))
        .collect();
    fixed.sort();
    assert!(
        unexpected.is_empty() && fixed.is_empty(),
        "{} unexpected failure(s), {} listed entries now pass (of {total} cases)\n{}\nnow passing: {fixed:?}",
        unexpected.len(),
        fixed.len(),
        unexpected.join("\n")
    );
}
