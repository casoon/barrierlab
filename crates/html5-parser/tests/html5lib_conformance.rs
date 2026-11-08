//! Runs this crate's `parse()` against the vendored html5lib-tests
//! tree-construction corpus (`tests/html5lib-tests/*.dat`, see that
//! directory's `README.md`) and compares the resulting tree, dumped via
//! `support::dump`, against each case's expected `#document` section.
//!
//! Per-case applicability (not per-file — a single `.dat` file can mix
//! applicable and inapplicable cases):
//! - `#document-fragment` cases are skipped: this crate has no fragment
//!   parsing.
//! - `#script-on`-only cases are skipped: this crate always models
//!   scripting as disabled.
//! - Everything else runs for real.
//!
//! Known failures (frameset/template gaps — see README.md's "Known
//! limitations" — and any real bugs the corpus surfaces) are tracked
//! explicitly in `tests/html5lib_known_failures.txt` rather than
//! silently ignored: an unlisted failure fails this test (regression
//! detection), and so does a listed entry that unexpectedly starts
//! passing (keeps the list honest). Regenerate it with
//! `HTML5LIB_BLESS=1 cargo test --test html5lib_conformance` after
//! investigating each new entry — see that file's own header.

mod support;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use support::dat::parse_dat_file;
use support::dump::dump_document;

const VENDORED_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/html5lib-tests");
const KNOWN_FAILURES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/html5lib_known_failures.txt"
);
/// Everything up to and including this line in `tests/html5lib_known_failures.txt`
/// is the fixed header (see that file); `HTML5LIB_BLESS` rewrites only
/// what follows it.
const KNOWN_FAILURES_HEADER_LAST_LINE: &str = "# --- entries below, one per line ---\n";

struct Failure {
    key: String,
    detail: String,
}

fn load_known_failures(path: &str) -> HashSet<String> {
    let contents = fs::read_to_string(path).unwrap_or_default();
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split_whitespace().next().unwrap_or(line).to_owned())
        .collect()
}

fn vendored_dat_files() -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(VENDORED_DIR)
        .unwrap_or_else(|error| {
            panic!(
                "reading {VENDORED_DIR}: {error} — run `python3 xtask/fetch-html5lib-tests.py` \
                 to vendor the html5lib-tests corpus first"
            )
        })
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("dat"))
        .collect();
    files.sort();
    files
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rewrites a known-failures file's entry list (keeping
/// its fixed header) from the current run's failures.
fn bless(path: &str, failures: &[Failure]) {
    let mut keys: Vec<&str> = failures
        .iter()
        .map(|failure| failure.key.as_str())
        .collect();
    keys.sort_unstable();

    let existing = fs::read_to_string(path).unwrap_or_default();
    let header_end = existing
        .find(KNOWN_FAILURES_HEADER_LAST_LINE)
        .unwrap_or_else(|| {
            panic!(
                "{path}'s header no longer contains the expected marker line \
                 {KNOWN_FAILURES_HEADER_LAST_LINE:?} — fix the header (or this constant) rather \
                 than blessing, or the entry list below it will be silently dropped"
            )
        })
        + KNOWN_FAILURES_HEADER_LAST_LINE.len();
    let mut out = existing[..header_end].to_owned();
    out.push('\n');
    for key in &keys {
        out.push_str(key);
        out.push('\n');
    }
    fs::write(path, out).expect("writing known-failures file");
    println!(
        "HTML5LIB_BLESS: wrote {} known-failing case(s) to {path}",
        keys.len()
    );
}

#[test]
fn html5lib_tree_construction_conformance() {
    let dat_files = vendored_dat_files();
    assert!(
        !dat_files.is_empty(),
        "no .dat files found in {VENDORED_DIR} — run `python3 xtask/fetch-html5lib-tests.py`"
    );

    let mut total = 0usize;
    let mut skipped_fragment = 0usize;
    let mut skipped_scripting = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<Failure> = Vec::new();

    for path in &dat_files {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("vendored .dat file name is valid UTF-8")
            .to_owned();
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));

        for (index, case) in parse_dat_file(&contents).into_iter().enumerate() {
            total += 1;

            if case.is_fragment {
                skipped_fragment += 1;
                continue;
            }
            if case.script_on_only {
                skipped_scripting += 1;
                continue;
            }

            let document = html5_parser::parse(&case.data).document;
            let actual = dump_document(&document);

            if actual == case.expected_document {
                passed += 1;
            } else {
                let key = format!("{file_name}#{index}");
                let detail = format!(
                    "{key} (input: {:?})\n  expected:\n{}\n  actual:\n{}",
                    case.data,
                    indent_block(&case.expected_document),
                    indent_block(&actual),
                );
                failures.push(Failure { key, detail });
            }
        }
    }

    println!(
        "html5lib conformance: {passed}/{total} passed, {} failing, \
         {skipped_fragment} skipped (fragment), {skipped_scripting} skipped (scripting)",
        failures.len(),
    );

    check_known_failures(KNOWN_FAILURES_PATH, "html5lib_conformance", &failures);
}

/// Compares `failures` against the known-failures file at `path` in both
/// directions (see this file's module doc); `test` names the test binary
/// for the hint in the failure message.
fn check_known_failures(path: &str, test: &str, failures: &[Failure]) {
    if std::env::var_os("HTML5LIB_BLESS").is_some() {
        bless(path, failures);
        return;
    }

    let known_failures = load_known_failures(path);
    let failing_keys: HashSet<&str> = failures
        .iter()
        .map(|failure| failure.key.as_str())
        .collect();

    let unexpected: Vec<&str> = failures
        .iter()
        .filter(|failure| !known_failures.contains(&failure.key))
        .map(|failure| failure.detail.as_str())
        .collect();
    let stale: Vec<&String> = known_failures
        .iter()
        .filter(|key| !failing_keys.contains(key.as_str()))
        .collect();

    assert!(
        unexpected.is_empty(),
        "{} test case(s) failed that are not in {path}:\n\n{}\n\n\
         If these are real, understood gaps (see README.md's \"Known limitations\") or newly \
         found bugs worth tracking rather than fixing immediately, add them via \
         `HTML5LIB_BLESS=1 cargo test --test {test}` — don't hand-edit the list.",
        unexpected.len(),
        unexpected.join("\n\n"),
    );
    assert!(
        stale.is_empty(),
        "{path} lists {} case(s) that now pass (or no longer exist) \
         — regenerate it with `HTML5LIB_BLESS=1 cargo test --test {test}`: {:?}",
        stale.len(),
        stale,
    );
}

const ERRORS_KNOWN_FAILURES_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/html5lib_errors_known_failures.txt"
);

/// `ParseErrorKind::EofInTag` -> `eof-in-tag`: the WHATWG error code
/// spelling used by the corpus's `#new-errors` sections.
fn kebab(kind: impl std::fmt::Debug) -> String {
    let mut out = String::new();
    for (index, c) in format!("{kind:?}").chars().enumerate() {
        if c.is_ascii_uppercase() {
            if index > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `(1:16) incorrectly-closed-comment` -> `(1, 16, "incorrectly-closed-comment")`;
/// a range `(1:44-1:49)` keeps its start.
fn parse_new_error(line: &str) -> (u32, u32, String) {
    let (position, code) = line
        .split_once(' ')
        .unwrap_or_else(|| panic!("malformed #new-errors line {line:?}"));
    let (line_no, column) = position
        .trim_matches(['(', ')'])
        // A range `(1:44-1:49)` is compared by its start.
        .split('-')
        .next()
        .unwrap_or_default()
        .split_once(':')
        .unwrap_or_else(|| panic!("malformed #new-errors position {line:?}"));
    (
        line_no.parse().expect("line number"),
        column.parse().expect("column number"),
        code.trim().to_owned(),
    )
}

/// html5lib's errors about a missing or non-conforming DOCTYPE
/// (`expected-doctype-but-got-*`, `unknown-doctype`, and the free-text forms older files use). This crate does not
/// record them: html-conform reports DOCTYPE problems itself, and a
/// parser-level error would duplicate its finding. They are left out of the
/// presence and count comparison below.
fn is_doctype_shape_error(line: &str) -> bool {
    // Some files predate the error codes and use free text instead.
    // "Unexpected DOCTYPE." is the stray-DOCTYPE error, which is recorded.
    let line = line.to_ascii_lowercase().replace("unexpected doctype", "");
    [
        "expected-doctype-but-got-",
        "unknown-doctype",
        "doctype-has-public-identifier",
        "no-doctype",
        "no doctype",
        "missing doctype",
        "expected doctype.",
        "expected a doctype",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

/// Compares the parse errors `parse()` reports against each case's
/// `#errors`/`#new-errors` sections. Two gated checks per case:
///
/// 1. **Presence** — the case has expected errors if and only if `parse()`
///    reports at least one. The `#errors` section is html5lib's own
///    vocabulary and counting (it reports some conditions once per
///    character, and its positions are not the spec's), so an exact count
///    or code comparison would only measure html5lib's quirks.
/// 2. **Spec codes** — `#new-errors` lists the tokenizer errors with their
///    WHATWG code and `(line:column)`. `parse()`'s errors, restricted to
///    codes that occur in any `#new-errors` section of the corpus, must
///    equal that list as a multiset of codes.
///
/// Reported but not gated: agreement including positions (the spec does
/// not fix where an error is reported, and for character references the
/// corpus reports the end of the reference while this crate reports its
/// `&`), and the exact count against `#errors`.
///
/// `processing-instructions.dat` is skipped: its cases exercise this
/// crate's processing-instruction extension, and the corpus has no error
/// expectations for them (every `#errors` section is empty).
///
/// Failures are tracked in `tests/html5lib_errors_known_failures.txt`,
/// in both directions, like the tree comparison above.
#[test]
fn html5lib_parse_errors() {
    let mut cases = Vec::new();
    let mut skipped_no_expectations = 0usize;
    for path in &vendored_dat_files() {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("vendored .dat file name is valid UTF-8")
            .to_owned();
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        for (index, case) in parse_dat_file(&contents).into_iter().enumerate() {
            if case.is_fragment || case.script_on_only {
                continue;
            }
            if file_name == "processing-instructions.dat" {
                skipped_no_expectations += 1;
                continue;
            }
            cases.push((format!("{file_name}#{index}"), case));
        }
    }

    let spec_codes: HashSet<String> = cases
        .iter()
        .flat_map(|(_, case)| &case.expected_new_errors)
        .map(|line| parse_new_error(line).2)
        .collect();

    let total = cases.len();
    let mut presence_ok = 0usize;
    let mut codes_ok = 0usize;
    let mut positions_ok = 0usize;
    let mut count_ok = 0usize;
    let mut failures: Vec<Failure> = Vec::new();

    for (key, case) in &cases {
        let errors = html5_parser::parse(&case.data).errors;

        let expected_errors: Vec<&String> = case
            .expected_errors
            .iter()
            .filter(|line| !is_doctype_shape_error(line))
            .collect();
        let expects_errors = !expected_errors.is_empty() || !case.expected_new_errors.is_empty();
        let presence = expects_errors == !errors.is_empty();
        if presence {
            presence_ok += 1;
        }
        if errors.len() == expected_errors.len() {
            count_ok += 1;
        }

        let mut expected: Vec<(u32, u32, String)> = case
            .expected_new_errors
            .iter()
            .map(|line| parse_new_error(line))
            .collect();
        let mut actual: Vec<(u32, u32, String)> = errors
            .iter()
            .map(|error| {
                (
                    error.position.line,
                    error.position.column,
                    kebab(error.kind),
                )
            })
            .filter(|(_, _, code)| spec_codes.contains(code))
            .collect();
        expected.sort();
        actual.sort();
        if expected == actual {
            positions_ok += 1;
        }
        let codes_of = |list: &[(u32, u32, String)]| {
            let mut codes: Vec<String> = list.iter().map(|entry| entry.2.clone()).collect();
            codes.sort();
            codes
        };
        let codes = codes_of(&expected) == codes_of(&actual);
        if codes {
            codes_ok += 1;
        }

        if !presence || !codes {
            failures.push(Failure {
                key: key.clone(),
                detail: format!(
                    "{key} (input: {:?})\n  presence ok: {presence}, spec codes ok: {codes}\n  \
                     expected (#errors): {:?}\n  expected (#new-errors): {expected:?}\n  \
                     actual (spec codes only): {actual:?}\n  all actual kinds: {:?}",
                    case.data,
                    expected_errors,
                    errors
                        .iter()
                        .map(|error| kebab(error.kind))
                        .collect::<Vec<_>>(),
                ),
            });
        }
    }

    println!(
        "html5lib parse errors: presence {presence_ok}/{total}, spec codes {codes_ok}/{total}, \
         with positions (reported only) {positions_ok}/{total}, exact count (reported only) \
         {count_ok}/{total}, {} failing, {skipped_no_expectations} skipped (no error expectations)",
        failures.len(),
    );

    check_known_failures(
        ERRORS_KNOWN_FAILURES_PATH,
        "html5lib_conformance",
        &failures,
    );
}
