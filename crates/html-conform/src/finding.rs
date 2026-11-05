use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A source location in the checked HTML input.
///
/// Positions are one-based; `byte_offset` is zero-based. A finding has no
/// location when it cannot be mapped back to a concrete source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// One-based line number.
    pub line: u32,
    /// One-based column number.
    pub column: u32,
    /// Zero-based byte offset.
    pub byte_offset: usize,
}

impl fmt::Display for SourceLocation {
    /// `"line:column"` — the same shape `src/infoset.rs`'s old, pre-Phase-08
    /// `relax_ng::Element::location()` string used to format by hand,
    /// kept for message-text compatibility now that this struct itself is
    /// that `Element` impl's `Location` type
    /// (`relax_ng::ValidationError<L>`'s `Display` impl needs `L: Display`
    /// to include the location in a finding's message text).
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// The severity of a conformance finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// The document violates a required conformance rule.
    Error,
    /// The document is valid but should be reviewed.
    Warning,
    /// Informational diagnostic.
    Info,
}

/// A single conformance or parser finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Stable identifier for the rule that emitted this finding.
    pub rule_id: String,
    /// Severity assigned by the reporting layer.
    pub severity: Severity,
    /// Human-readable explanation of the finding.
    pub message: String,
    /// Source location when the reporting layer can establish one.
    pub location: Option<SourceLocation>,
}

/// The result of checking one HTML document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckReport {
    /// Findings in parser order followed by the later validation layers.
    pub findings: Vec<Finding>,
    /// Classes of content in this document that the checker did not
    /// validate. An empty list means the document contains none of the
    /// known unchecked constructs, not that it is fully covered.
    pub skipped: Vec<Skipped>,
}

/// A class of content the checker does not validate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SkippedClass {
    /// The document has `<style>` elements; their CSS is not validated
    /// (barrierlab#87).
    StyleElementCss,
    /// A `<meta>` Content-Security-Policy carries hash sources, which are
    /// not evaluated against inline content.
    CspHashSource,
}

impl SkippedClass {
    fn subject(self) -> &'static str {
        match self {
            Self::StyleElementCss => "style-element-css",
            Self::CspHashSource => "csp-hash-source",
        }
    }
}

/// One unchecked class that occurs in the checked document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skipped {
    /// Which class was not checked.
    pub class: SkippedClass,
    /// Why, and what the host should not conclude from the silence.
    pub detail: String,
}

impl CheckReport {
    /// Returns whether the report contains an error-level finding.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }

    /// Converts the report into the shared `a11y_report` model.
    ///
    /// Severity mapping (`a11y_report` separates certainty, `Outcome`, from
    /// weight, `Severity`): an `Error` is a violated conformance rule, so
    /// `Fail` with `Medium` weight; it is a spec violation, not necessarily
    /// an accessibility blocker. A `Warning` is valid markup that needs a
    /// look, which is what `Review` means: `Review` with `Low`. An `Info`
    /// is a statement the checker could not decide (such as unevaluated
    /// hash sources), so `Untested` with `Low`. The location becomes
    /// `source_hint` as `"line:column"`, since `a11y_report::Location` has
    /// no line or column field. Each [`Skipped`] becomes an
    /// `InputKind::DocumentIncomplete` note, so unchecked content stays
    /// visible. No `RuleRun` entries are recorded: html-conform has no rule
    /// registry to say which rules could have run.
    #[must_use]
    pub fn to_report(&self) -> a11y_report::Report {
        let mut report = a11y_report::Report::new();
        for finding in &self.findings {
            let (mut mapped, severity) = match finding.severity {
                Severity::Error => (
                    a11y_report::Finding::fail(&finding.rule_id, &finding.message),
                    a11y_report::Severity::Medium,
                ),
                Severity::Warning => (
                    a11y_report::Finding::review(&finding.rule_id, &finding.message),
                    a11y_report::Severity::Low,
                ),
                Severity::Info => (
                    a11y_report::Finding::untested(&finding.rule_id, &finding.message),
                    a11y_report::Severity::Low,
                ),
            };
            mapped = mapped.with_severity(severity);
            if let Some(location) = finding.location {
                mapped.location.source_hint = Some(location.to_string());
            }
            report.push(mapped);
        }
        for skipped in &self.skipped {
            report.note_input(
                a11y_report::InputNote::new(a11y_report::InputKind::DocumentIncomplete)
                    .with_subject(skipped.class.subject())
                    .with_detail(&skipped.detail),
            );
        }
        report.finish()
    }
}

/// A technical failure that prevented a document from being checked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckError {
    /// A checker component could not be initialized.
    Initialization {
        /// Description of the failed component initialization.
        message: String,
    },
    /// The document nests elements deeper than the checker validates, so it
    /// was not checked at all. Not a conformance verdict: the input may be
    /// conforming HTML (the HTML syntax has no depth limit).
    DocumentTooDeep {
        /// The deepest nesting level found in the parsed tree.
        depth: usize,
        /// The deepest nesting level the checker validates.
        limit: usize,
    },
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization { message } => {
                write!(formatter, "checker initialization failed: {message}")
            }
            Self::DocumentTooDeep { depth, limit } => write!(
                formatter,
                "document not checked: element nesting depth {depth} exceeds the checker's limit of {limit}"
            ),
        }
    }
}

impl Error for CheckError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(severity: Severity, location: Option<SourceLocation>) -> Finding {
        Finding {
            rule_id: "schema.html5".to_owned(),
            severity,
            message: "m".to_owned(),
            location,
        }
    }

    #[test]
    fn severities_map_to_outcome_and_severity() {
        use a11y_report::{Outcome, Severity as S};
        let report = CheckReport {
            findings: vec![
                finding(Severity::Error, None),
                finding(Severity::Warning, None),
                finding(Severity::Info, None),
            ],
            skipped: Vec::new(),
        }
        .to_report();
        let pairs: Vec<_> = report
            .findings
            .iter()
            .map(|f| (f.outcome, f.severity))
            .collect();
        assert_eq!(
            pairs,
            [
                (Outcome::Fail, S::Medium),
                (Outcome::Review, S::Low),
                (Outcome::Untested, S::Low)
            ]
        );
        assert_eq!(report.findings[0].rule_id, "schema.html5");
        assert_eq!(report.summary.fail, 1);
    }

    #[test]
    fn location_becomes_line_column_source_hint() {
        let location = SourceLocation {
            line: 3,
            column: 7,
            byte_offset: 20,
        };
        let report = CheckReport {
            findings: vec![finding(Severity::Error, Some(location))],
            skipped: Vec::new(),
        }
        .to_report();
        assert_eq!(
            report.findings[0].location.source_hint.as_deref(),
            Some("3:7")
        );
    }

    #[test]
    fn skipped_become_document_incomplete_notes_in_json() {
        let report = CheckReport {
            findings: vec![finding(Severity::Error, None)],
            skipped: vec![Skipped {
                class: SkippedClass::StyleElementCss,
                detail: "d".to_owned(),
            }],
        }
        .to_report();
        let json = serde_json::to_value(&report).expect("serializes");
        assert_eq!(json["findings"][0]["outcome"], "fail");
        assert_eq!(json["findings"][0]["severity"], "medium");
        assert_eq!(json["inputs"][0]["kind"], "document_incomplete");
        assert_eq!(json["inputs"][0]["subject"], "style-element-css");
        assert_eq!(json["inputs"][0]["detail"], "d");
    }
}
