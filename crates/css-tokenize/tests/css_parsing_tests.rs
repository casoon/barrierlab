//! Runs the tokenizer over `component_value_list.json` from
//! `SimonSapin/css-parsing-tests` (CC0), pinned in
//! `tests/corpus/css-parsing-tests/UPSTREAM.md`.
//!
//! The suite expresses nested component values, so tokens are folded into
//! blocks and functions here. It compares what the tokenizer exposes: number
//! representations (the string in position 1) are dropped from the expected
//! side, and `["error", "eof-in-*"]` entries are dropped because the tokenizer
//! reports no errors, and the percentage type flag, which `Token::Percentage`
//! does not carry. Cases the tokenizer does not satisfy are listed in
//! `css_parsing_tests_known_failures.txt` (by case index); the list is checked
//! both ways: a listed case that passes fails the test, and so does an
//! unlisted one that fails. Re-bless with `CSS_TESTS_BLESS=1`.

use css_tokenize::{HashType, NumericType, Token, tokenize};
use serde_json::{Value, json};

fn kind(t: NumericType) -> &'static str {
    if t == NumericType::Integer {
        "integer"
    } else {
        "number"
    }
}

/// Folds tokens into the suite's nested representation until `close`.
fn fold(tokens: &mut std::slice::Iter<Token>, close: Option<&Token>) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some(token) = tokens.next() {
        if Some(token) == close {
            break;
        }
        out.push(match token {
            Token::Eof => break,
            Token::Ident(v) => json!(["ident", v]),
            Token::AtKeyword(v) => json!(["at-keyword", v]),
            Token::Hash { value, type_flag } => {
                json!([
                    "hash",
                    value,
                    if *type_flag == HashType::Id {
                        "id"
                    } else {
                        "unrestricted"
                    }
                ])
            }
            Token::String(v) => json!(["string", v]),
            Token::BadString => json!(["error", "bad-string"]),
            Token::Url(v) => json!(["url", v]),
            Token::BadUrl => json!(["error", "bad-url"]),
            Token::Delim(c) => json!(c.to_string()),
            Token::Number { value, type_flag } => json!(["number", value, kind(*type_flag)]),
            Token::Percentage { value } => json!(["percentage", value]),
            Token::Dimension {
                value,
                type_flag,
                unit,
            } => {
                json!(["dimension", value, kind(*type_flag), unit])
            }
            Token::Whitespace => json!(" "),
            Token::Cdo => json!("<!--"),
            Token::Cdc => json!("-->"),
            Token::Colon => json!(":"),
            Token::Semicolon => json!(";"),
            Token::Comma => json!(","),
            Token::CloseSquare => json!(["error", "]"]),
            Token::CloseParen => json!(["error", ")"]),
            Token::CloseCurly => json!(["error", "}"]),
            Token::Function(name) => {
                let mut v = vec![json!("function"), json!(name)];
                v.extend(fold(tokens, Some(&Token::CloseParen)));
                Value::Array(v)
            }
            Token::OpenSquare => block("[]", tokens, &Token::CloseSquare),
            Token::OpenParen => block("()", tokens, &Token::CloseParen),
            Token::OpenCurly => block("{}", tokens, &Token::CloseCurly),
            _ => json!(["unknown"]),
        });
    }
    out
}

fn block(name: &str, tokens: &mut std::slice::Iter<Token>, close: &Token) -> Value {
    let mut v = vec![json!(name)];
    v.extend(fold(tokens, Some(close)));
    Value::Array(v)
}

/// Brings the expected side to what the tokenizer can express.
fn normalize(value: &Value) -> Value {
    match value {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(items) => {
            let head = items.first().and_then(Value::as_str);
            let mut items: Vec<Value> = items
                .iter()
                .filter(|i| {
                    !matches!(i.as_array().map(Vec::as_slice),
                        Some([Value::String(a), Value::String(b)]) if a == "error" && b.starts_with("eof-"))
                })
                .map(normalize)
                .collect();
            if matches!(head, Some("number" | "percentage" | "dimension")) {
                items.remove(1); // representation string
                if head == Some("percentage") {
                    items.pop(); // type flag: `Token::Percentage` has none
                }
            }
            Value::Array(items)
        }
        other => other.clone(),
    }
}

#[test]
fn component_value_list() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/corpus/css-parsing-tests/component_value_list.json"
    );
    let suite: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let failures_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/css_parsing_tests_known_failures.txt"
    );

    let mut failing = Vec::new();
    for (index, pair) in suite.chunks(2).enumerate() {
        let input = pair[0].as_str().unwrap();
        let tokens = tokenize(input);
        let actual = Value::Array(fold(&mut tokens.iter(), None));
        if actual != normalize(&pair[1]) {
            failing.push(index);
        }
    }

    if std::env::var_os("CSS_TESTS_BLESS").is_some() {
        let lines: Vec<String> = failing.iter().map(usize::to_string).collect();
        std::fs::write(failures_path, lines.join("\n") + "\n").unwrap();
        return;
    }
    let known: Vec<usize> = std::fs::read_to_string(failures_path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.trim().parse().unwrap())
        .collect();
    let unexpected: Vec<_> = failing.iter().filter(|i| !known.contains(i)).collect();
    let fixed: Vec<_> = known.iter().filter(|i| !failing.contains(i)).collect();
    assert!(
        unexpected.is_empty(),
        "newly failing case indexes: {unexpected:?}"
    );
    assert!(
        fixed.is_empty(),
        "listed but passing (remove from the list): {fixed:?}"
    );
}
