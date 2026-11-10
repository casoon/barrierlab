//! The `text` and `json` formats, and what all formats share.

use std::fmt::Write as _;
use std::io::IsTerminal;

use a11y_report::{Finding, InputKind, InputNote, Outcome, Report};

use crate::args::ColorMode;
use crate::input::{FileResult, State};

/// A class of content in a checked document that html-conform did not look at.
pub struct Note<'a> {
    pub class: &'a str,
    pub detail: &'a str,
}

pub fn notes(result: &FileResult) -> Vec<Note<'_>> {
    result.report().map_or_else(Vec::new, |report| {
        report
            .inputs
            .iter()
            .map(|n| Note {
                class: n.subject.as_deref().unwrap_or("unchecked"),
                detail: n.detail.as_deref().unwrap_or(""),
            })
            .collect()
    })
}

pub fn not_checked_reason(result: &FileResult) -> Option<&str> {
    match &result.state {
        State::NotChecked(reason) => Some(reason),
        State::Checked(_) => None,
    }
}

pub fn findings(result: &FileResult) -> &[Finding] {
    result.report().map_or(&[], |r| r.findings.as_slice())
}

/// Where each finding of `findings(result)` ends, when known.
pub fn ranges(result: &FileResult) -> &[Option<html_conform::SourceRange>] {
    &result.ranges
}

/// `error`, `warning` or `info`, as the text format and JUnit name them.
pub fn level_word(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Fail => "error",
        Outcome::Review => "warning",
        Outcome::Pass | Outcome::Untested => "info",
    }
}

pub fn line_column(finding: &Finding) -> Option<(u32, u32)> {
    finding.location.position.map(|p| (p.line, p.column))
}

/// One JSON document: the serialised `a11y_report::Report` over all files.
/// Findings carry `location.file`; a finding whose layer knows where it ends
/// also carries `location.end` (`line`, `column`, `byte_offset`; exclusive,
/// in the units of `location.position`). A note on unchecked content or a
/// file that could not be checked is an input note whose `subject` is the
/// file.
pub fn json(results: &[FileResult]) -> String {
    let mut merged = Report::new();
    let mut ends = Vec::new();
    for result in results {
        match &result.state {
            State::Checked(report) => {
                merged.extend(report.findings.iter().cloned());
                ends.extend(ranges(result).iter().map(|r| r.map(|r| r.end)));
                for note in &report.inputs {
                    merged.note_input(
                        InputNote::new(InputKind::DocumentIncomplete)
                            .with_subject(&result.path)
                            .with_detail(format!(
                                "{}: {}",
                                note.subject.as_deref().unwrap_or("unchecked"),
                                note.detail.as_deref().unwrap_or("")
                            )),
                    );
                }
            }
            State::NotChecked(reason) => merged.note_input(
                InputNote::new(InputKind::DocumentIncomplete)
                    .with_subject(&result.path)
                    .with_detail(format!("not checked: {reason}")),
            ),
        }
    }
    let merged = merged.finish();
    let positioned_ends: Vec<_> = merged
        .findings
        .iter()
        .zip(&ends)
        .filter(|(finding, _)| finding.location.position.is_some())
        .map(|(_, end)| *end)
        .collect();
    let pretty = serde_json::to_string_pretty(&merged).expect("report serialises");
    let mut text = add_ends(&pretty, &positioned_ends);
    text.push('\n');
    text
}

/// The `position` key of a finding's location in the pretty-printed `Report`.
const POSITION: &str = "        \"position\": {";
/// The line that closes that object.
const CLOSE: &str = "        }";

/// Adds `location.end` after the `position` of each finding that has one, in
/// the pretty-printed `Report`. `ends` has one entry per finding with a
/// position, in order. Done on the text because `a11y_report::SourcePosition`
/// has no end and a `serde_json::Value` round trip would sort the keys,
/// changing every existing line. The only line equal to the `position` key
/// at this indent is the structural one: strings never contain raw newlines.
fn add_ends(pretty: &str, ends: &[Option<html_conform::SourceLocation>]) -> String {
    let ends_total = ends.iter().flatten().count();
    let mut inserted = 0;
    let mut ends = ends.iter();
    let mut pending = None;
    let mut lines: Vec<String> = Vec::new();
    for line in pretty.lines() {
        if line == POSITION {
            pending = ends.next().copied().flatten();
        } else if line == CLOSE
            && let Some(end) = pending.take()
        {
            lines.push(format!("{CLOSE},"));
            lines.push("        \"end\": {".to_owned());
            lines.push(format!("          \"line\": {},", end.line));
            lines.push(format!("          \"column\": {},", end.column));
            lines.push(format!("          \"byte_offset\": {}", end.byte_offset));
            lines.push(CLOSE.to_owned());
            inserted += 1;
            continue;
        }
        lines.push(line.to_owned());
    }
    debug_assert_eq!(
        inserted, ends_total,
        "a11y_report's JSON layout changed: not every end found its `position`"
    );
    lines.join("\n")
}

/// Whether text output is coloured. An explicit `always`/`never` wins;
/// `auto` needs a terminal, an unset-or-empty `NO_COLOR` and `TERM` not `dumb`.
pub fn use_color(
    mode: ColorMode,
    is_terminal: bool,
    no_color: Option<&str>,
    term: Option<&str>,
) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            is_terminal && no_color.is_none_or(str::is_empty) && term != Some("dumb")
        }
    }
}

pub fn color_enabled(mode: ColorMode) -> bool {
    use_color(
        mode,
        std::io::stdout().is_terminal(),
        std::env::var("NO_COLOR").ok().as_deref(),
        std::env::var("TERM").ok().as_deref(),
    )
}

fn paint(color: bool, code: &str, text: &str) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// `path:line:column: level[rule] message`, grouped by file, then a summary.
pub fn text(results: &[FileResult], quiet: bool, color: bool) -> String {
    let mut out = String::new();
    let (mut errors, mut warnings, mut infos, mut unchecked) = (0, 0, 0, 0);
    for result in results {
        let list = findings(result);
        for finding in list {
            let (code, counter) = match finding.outcome {
                Outcome::Fail => ("1;31", &mut errors),
                Outcome::Review => ("1;33", &mut warnings),
                Outcome::Pass | Outcome::Untested => ("1;36", &mut infos),
            };
            *counter += 1;
            let at = line_column(finding).map_or(String::new(), |(l, c)| format!(":{l}:{c}"));
            let level = paint(color, code, level_word(finding.outcome));
            let _ = writeln!(
                out,
                "{}{at}: {level}[{}] {}",
                result.path, finding.rule_id, finding.message
            );
        }
        let notes = notes(result);
        if let Some(reason) = not_checked_reason(result) {
            unchecked += 1;
            let _ = writeln!(
                out,
                "{}: {}: {reason}",
                result.path,
                paint(color, "1;31", "not checked")
            );
        } else if list.is_empty() && !quiet {
            let _ = writeln!(
                out,
                "{}: {}",
                result.path,
                paint(color, "32", "no findings")
            );
        }
        for note in notes {
            let _ = writeln!(
                out,
                "{}: {} ({}): {}",
                result.path,
                paint(color, "33", "not fully checked"),
                note.class,
                note.detail
            );
        }
    }
    if !quiet {
        let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
        let _ = write!(
            out,
            "checked {}: {}, {}, {infos} info",
            plural(results.len() - unchecked, "file"),
            plural(errors, "error"),
            plural(warnings, "warning"),
        );
        if unchecked > 0 {
            let _ = write!(out, "; {} not checked", plural(unchecked, "file"));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_ends_relies_on_the_serialised_layout_of_a_finding() {
        use a11y_report::SourcePosition;
        let mut finding = Finding::fail("r", "m");
        finding.location.position = Some(SourcePosition::new(2, 3, 10));
        let mut report = Report::new();
        report.push(finding);
        let pretty = serde_json::to_string_pretty(&report.finish()).unwrap();
        // If a11y-report changes how a finding is serialised, these fail
        // here instead of ends silently disappearing from the output.
        assert_eq!(
            pretty.lines().filter(|l| *l == POSITION).count(),
            1,
            "{pretty}"
        );
        assert_eq!(
            pretty.lines().filter(|l| *l == CLOSE).count(),
            1,
            "{pretty}"
        );
        let end = html_conform::SourceLocation {
            line: 2,
            column: 9,
            byte_offset: 16,
        };
        let out = add_ends(&pretty, &[Some(end)]);
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        let location = &value["findings"][0]["location"];
        assert_eq!(location["position"]["column"], 3);
        assert_eq!(location["end"]["line"], 2);
        assert_eq!(location["end"]["column"], 9);
        assert_eq!(location["end"]["byte_offset"], 16);
        assert_eq!(add_ends(&pretty, &[None]), pretty);
    }

    #[test]
    fn color_rules() {
        let auto = |tty, no, term| use_color(ColorMode::Auto, tty, no, term);
        assert!(auto(true, None, Some("xterm")));
        assert!(
            auto(true, Some(""), Some("xterm")),
            "empty NO_COLOR is unset"
        );
        assert!(!auto(true, Some("1"), Some("xterm")));
        assert!(!auto(true, None, Some("dumb")));
        assert!(!auto(false, None, None));
        assert!(use_color(ColorMode::Always, false, Some("1"), Some("dumb")));
        assert!(!use_color(ColorMode::Never, true, None, None));
    }
}
