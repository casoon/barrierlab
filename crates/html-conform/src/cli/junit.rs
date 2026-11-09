//! JUnit XML: one `testsuite` per file.
//!
//! A `Fail` finding is a `failure`; a `Review` or `Untested` finding, and
//! each content class that was not checked, is a `skipped` testcase with its
//! message; a file that could not be checked is an `error`. A file with
//! nothing decided against it gets one passing testcase, `conformance`.

use std::fmt::Write as _;

use a11y_report::Outcome;

use crate::input::FileResult;
use crate::output::{findings, line_column, not_checked_reason, notes};

/// Escapes text for XML 1.0 content and attribute values. Characters XML 1.0
/// cannot carry at all (most control characters) become U+FFFD.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || matches!(c, '\u{FFFE}' | '\u{FFFF}') => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

#[derive(Default)]
struct Counts {
    tests: usize,
    failures: usize,
    errors: usize,
    skipped: usize,
}

pub fn render(results: &[FileResult]) -> String {
    let mut suites = String::new();
    let mut total = Counts::default();
    for result in results {
        let mut cases = String::new();
        let mut counts = Counts::default();
        let path = escape(&result.path);
        let mut decided_against = false;
        for finding in findings(result) {
            let at = line_column(finding).map_or(String::new(), |(l, c)| format!(" at {l}:{c}"));
            let name = escape(&format!("{}{at}", finding.rule_id));
            let message = escape(&finding.message);
            let rule = escape(&finding.rule_id);
            counts.tests += 1;
            match finding.outcome {
                Outcome::Fail => {
                    decided_against = true;
                    counts.failures += 1;
                    let _ = writeln!(
                        cases,
                        "    <testcase classname=\"{path}\" name=\"{name}\">\n      <failure message=\"{message}\" type=\"{rule}\">{path}{at}: {message}</failure>\n    </testcase>",
                    );
                }
                _ => {
                    if finding.outcome == Outcome::Review {
                        decided_against = true;
                    }
                    counts.skipped += 1;
                    let _ = writeln!(
                        cases,
                        "    <testcase classname=\"{path}\" name=\"{name}\">\n      <skipped message=\"{}{message}\"/>\n    </testcase>",
                        if finding.outcome == Outcome::Review {
                            "warning: "
                        } else {
                            "not evaluated: "
                        },
                    );
                }
            }
        }
        if let Some(reason) = not_checked_reason(result) {
            decided_against = true;
            counts.tests += 1;
            counts.errors += 1;
            let _ = writeln!(
                cases,
                "    <testcase classname=\"{path}\" name=\"check\">\n      <error message=\"not checked: {0}\" type=\"not-checked\">{0}</error>\n    </testcase>",
                escape(reason)
            );
        }
        for note in notes(result) {
            counts.tests += 1;
            counts.skipped += 1;
            let _ = writeln!(
                cases,
                "    <testcase classname=\"{path}\" name=\"not checked: {}\">\n      <skipped message=\"{}\"/>\n    </testcase>",
                escape(note.class),
                escape(note.detail)
            );
        }
        if !decided_against && counts.failures == 0 {
            counts.tests += 1;
            cases.insert_str(
                0,
                &format!("    <testcase classname=\"{path}\" name=\"conformance\"/>\n"),
            );
        }
        let _ = write!(
            suites,
            "  <testsuite name=\"{path}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\">\n{cases}  </testsuite>\n",
            counts.tests, counts.failures, counts.errors, counts.skipped
        );
        total.tests += counts.tests;
        total.failures += counts.failures;
        total.errors += counts.errors;
        total.skipped += counts.skipped;
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"html-conform\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\">\n{suites}</testsuites>\n",
        total.tests, total.failures, total.errors, total.skipped
    )
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_markup_and_drops_control_characters() {
        assert_eq!(escape("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&apos;");
        assert_eq!(escape("x\u{1}y\tz"), "x\u{FFFD}y\tz");
    }
}
