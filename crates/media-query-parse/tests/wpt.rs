//! Run the vendored web-platform-tests files `test_media_queries.html` and
//! `match-media-parsing.html` (`tests/corpus/wpt`, see `UPSTREAM.md` there).
//!
//! Only calls whose arguments are plain string literals are cases; calls built
//! from loops or template strings need a JavaScript engine and are not run.
//! Whether a query *matches* is a device question outside this crate, so only
//! the grammar verdict is compared:
//!
//! - `query_should_be_parseable(q)` / `query_should_not_be_parseable(q)`: `q`
//!   parses as one `<media-query>` or does not.
//! - `expression_should_be_known(e)` / `expression_should_be_unknown(e)`: both
//!   need `(e)` to parse (an unknown feature parses as `<general-enclosed>`).
//! - `test_parsing(q, expected)` (matchMedia): an entry of the comma-separated
//!   `q` is invalid exactly when the matching entry of `expected` is
//!   `not all`.
//!
//! Failing cases are listed in `tests/wpt_known_failures.txt`, keyed
//! `<file>:<line>`; the list is checked in both directions. Re-bless after
//! examining every entry with `WPT_BLESS=1 cargo test -p media-query-parse
//! --test wpt`.

use std::collections::HashSet;
use std::fs;

use media_query_parse::{parse_media_query, parse_media_query_list};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/wpt");
const KNOWN_FAILURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/wpt_known_failures.txt");
const HEADER_END: &str = "# --- entries below, one per line ---\n";
const FILES: [&str; 2] = ["test_media_queries.html", "match-media-parsing.html"];
const CALLS: [&str; 5] = [
    "query_should_be_parseable",
    "query_should_not_be_parseable",
    "expression_should_be_known",
    "expression_should_be_unknown",
    "test_parsing",
];

/// Reads one JavaScript string literal at the start of `s`; returns the value
/// and the rest after the closing quote.
fn literal(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    let quote = s.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let mut out = String::new();
    let mut chars = s[1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            c if c == quote => return Some((out, &s[1 + i + 1..])),
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                o => out.push(o),
            },
            c => out.push(c),
        }
    }
    None
}

/// `name(lit)` or `name(lit, lit)` at the start of a trimmed line.
fn call(line: &str) -> Option<(&'static str, Vec<String>)> {
    let line = line.trim_start();
    let name = CALLS
        .iter()
        .find(|n| line.strip_prefix(**n).is_some_and(|r| r.starts_with('(')))?;
    let mut rest = &line[name.len() + 1..];
    let mut args = Vec::new();
    loop {
        let (v, r) = literal(rest)?;
        args.push(v);
        let r = r.trim_start();
        if let Some(r) = r.strip_prefix(',') {
            rest = r;
        } else {
            return r.starts_with(')').then_some((name, args));
        }
    }
}

fn verdict(name: &str, args: &[String]) -> Result<(), String> {
    match name {
        "query_should_be_parseable"
        | "expression_should_be_known"
        | "expression_should_be_unknown" => {
            let q = if name.starts_with("expression") {
                format!("({})", args[0])
            } else {
                args[0].clone()
            };
            parse_media_query(&q)
                .map(|_| ())
                .map_err(|e| format!("{q:?} should parse: {e:?}"))
        }
        "query_should_not_be_parseable" => match parse_media_query(&args[0]) {
            Err(_) => Ok(()),
            Ok(_) => Err(format!("{:?} should not parse", args[0])),
        },
        "test_parsing" => {
            let expected = args.get(1).unwrap_or(&args[0]);
            let want: Vec<bool> = if expected.is_empty() {
                Vec::new()
            } else {
                expected.split(", ").map(|e| e != "not all").collect()
            };
            let got: Vec<bool> = if args[0].trim().is_empty() {
                Vec::new()
            } else {
                parse_media_query_list(&args[0])
                    .iter()
                    .map(Result::is_ok)
                    .collect()
            };
            if got == want {
                Ok(())
            } else {
                Err(format!(
                    "{:?} -> expected {expected:?} (valid: {want:?}), got {got:?}",
                    args[0]
                ))
            }
        }
        _ => unreachable!(),
    }
}

fn bekannte() -> HashSet<String> {
    fs::read_to_string(KNOWN_FAILURES)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn wpt_media_query_parsing() {
    let mut cases = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    for file in FILES {
        let text = fs::read_to_string(format!("{CORPUS}/{file}"))
            .unwrap_or_else(|e| panic!("{file}: {e} - run xtask/vendor-wpt-mediaqueries.sh"));
        for (i, line) in text.lines().enumerate() {
            let Some((name, args)) = call(line) else {
                continue;
            };
            cases += 1;
            if let Err(d) = verdict(name, &args) {
                failures.push((format!("{file}:{}", i + 1), d));
            }
        }
    }
    assert!(cases > 0, "no cases found in the corpus");

    if std::env::var_os("WPT_BLESS").is_some() {
        let old = fs::read_to_string(KNOWN_FAILURES).unwrap_or_default();
        let head = old
            .find(HEADER_END)
            .unwrap_or_else(|| panic!("header of {KNOWN_FAILURES} lacks the marker line"))
            + HEADER_END.len();
        let mut out = old[..head].to_owned();
        out.push('\n');
        for (k, d) in &failures {
            out.push_str(&format!("{k}  {d}\n"));
        }
        fs::write(KNOWN_FAILURES, out).unwrap();
        println!("WPT_BLESS: {} of {cases} cases listed", failures.len());
        return;
    }

    let known = bekannte();
    let keys: HashSet<&str> = failures.iter().map(|(k, _)| k.as_str()).collect();
    let new: Vec<String> = failures
        .iter()
        .filter(|(k, _)| !known.contains(k))
        .map(|(k, d)| format!("{k}: {d}"))
        .collect();
    let stale: Vec<&String> = known
        .iter()
        .filter(|k| !keys.contains(k.as_str()))
        .collect();
    assert!(
        new.is_empty(),
        "{} case(s) not in tests/wpt_known_failures.txt:\n{}",
        new.len(),
        new.join("\n")
    );
    assert!(
        stale.is_empty(),
        "tests/wpt_known_failures.txt lists passing cases {stale:?} - re-bless with WPT_BLESS=1"
    );
}
