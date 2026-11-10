use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A source location in the checked HTML input.
///
/// Units, as the parser counts them: `line` is one-based and counts line
/// feeds (a CR or CRLF counts as one line break, as in HTML input
/// preprocessing); `column` is one-based and counts Unicode scalar values
/// (chars) since the start of the line, so a tab or a character outside
/// ASCII is one column and a character that takes two UTF-16 code units is
/// still one column; `byte_offset` is zero-based and counts UTF-8 bytes in
/// the `&str` passed to the checker. Hosts that need UTF-16 columns (the
/// language server protocol) convert from `byte_offset` against their text.
/// A finding has no location when it cannot be mapped back to a concrete
/// position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// One-based line number.
    pub line: u32,
    /// One-based column number.
    pub column: u32,
    /// Zero-based byte offset.
    pub byte_offset: usize,
}

/// The range a finding covers: `start` is where [`Finding::location`] points,
/// `end` is the position just past the range (exclusive), in the units of
/// [`SourceLocation`]. Only present where a layer knows the end; see
/// [`RangedReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SourceRange {
    /// First position of the range, equal to the finding's location.
    pub start: SourceLocation,
    /// Position just past the last character of the range.
    pub end: SourceLocation,
}

/// A [`CheckReport`] plus, per finding, the source range where known.
///
/// Returned by [`crate::check_with_ranges`]. The ranges are kept beside the
/// report, parallel to `report.findings`, so [`CheckReport`] and [`Finding`]
/// stay as they are. Which layers know an end:
///
/// - Findings of the schema, assertion, scripts, CSP and table layers are
///   anchored at an element; their range is always that element's start tag,
///   also for a finding about the element's content or end. Findings anchored at a text node, and findings about an element the
///   parser synthesized, have no range.
/// - `parser.html5` findings have no range: the parser reports a point, not
///   a token span.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangedReport {
    report: CheckReport,
    /// Always as long as `report.findings`; both are private so it stays so.
    ranges: Vec<Option<SourceRange>>,
}

impl RangedReport {
    pub(crate) fn new(report: CheckReport, ranges: Vec<Option<SourceRange>>) -> Self {
        debug_assert_eq!(report.findings.len(), ranges.len());
        Self { report, ranges }
    }

    /// The report [`crate::check_with_options`] would return.
    #[must_use]
    pub fn report(&self) -> &CheckReport {
        &self.report
    }

    /// The report without the ranges.
    #[must_use]
    pub fn into_report(self) -> CheckReport {
        self.report
    }

    /// The ranges, parallel to `report().findings`: `None` where the layer
    /// that reported the finding does not know an end.
    #[must_use]
    pub fn ranges(&self) -> &[Option<SourceRange>] {
        &self.ranges
    }

    /// Each finding with its range, in report order.
    pub fn findings(&self) -> impl Iterator<Item = (&Finding, Option<SourceRange>)> {
        self.report.findings.iter().zip(self.ranges.iter().copied())
    }

    /// Converts like [`CheckReport::to_report`], and fills `location.end` of
    /// each finding whose layer knows a range. The findings keep their
    /// order, so [`RangedReport::ranges`] stays parallel to the result's
    /// findings.
    #[must_use]
    pub fn to_report(&self) -> a11y_report::Report {
        let mut report = self.report.to_report();
        for (finding, range) in report.findings.iter_mut().zip(&self.ranges) {
            if let Some(range) = range {
                finding.location.end = Some(a11y_report::SourcePosition::new(
                    range.end.line,
                    range.end.column,
                    range.end.byte_offset,
                ));
            }
        }
        report
    }
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
///
/// Deprecated in favour of `a11y_report`: see [`CheckReport::to_report`].
#[deprecated(
    note = "html-conform now reports through the shared model: use `CheckReport::to_report()` and the `a11y_report` types"
)]
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
///
/// Deprecated in favour of `a11y_report`: see [`CheckReport::to_report`].
#[deprecated(
    note = "html-conform now reports through the shared model: use `CheckReport::to_report()` and the `a11y_report` types"
)]
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
    /// Machine-readable form of `message`, for hosts that localize the text.
    /// Only schema findings carry one so far; `message` stays the English
    /// text either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<FindingDetail>,
}

/// The structured arguments behind a schema finding's English message, so a
/// host can write the message in its own language without parsing prose.
/// Element and attribute names are local names without the XHTML namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FindingDetail {
    /// An element is not allowed at this position.
    UnexpectedElement {
        /// Local name of the offending element.
        element: String,
    },
    /// Text is not allowed at this position.
    UnexpectedText {
        /// Local name of the element that contains the text.
        parent: String,
    },
    /// An attribute is not allowed on its element.
    UnexpectedAttribute {
        /// Local name of the offending attribute.
        attribute: String,
        /// Local name of the element carrying it.
        element: String,
    },
    /// An allowed attribute has a value the schema rejects.
    InvalidAttributeValue {
        /// Local name of the attribute.
        attribute: String,
        /// The rejected value.
        value: String,
        /// Local name of the element carrying it.
        element: String,
    },
    /// An element ended while required content was still missing.
    MissingContent {
        /// Local name of the incomplete element.
        element: String,
    },
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
    /// `Location::position` (line, column, byte offset) and, for readers of
    /// older reports, `source_hint` as `"line:column"`. A [`FindingDetail`]
    /// travels in `Finding::extra` (read it with
    /// `finding.extra.get::<FindingDetail>()`); like every `extra` it is not
    /// serialized, so the JSON contract is unchanged. Each [`Skipped`] becomes an
    /// `InputKind::DocumentIncomplete` note, so unchecked content stays
    /// visible. No `RuleRun` entries are recorded: html-conform has no rule
    /// registry to say which rules could have run.
    ///
    /// ```
    /// let report = html_conform::check("<!DOCTYPE html><title>t</title>")
    ///     .unwrap()
    ///     .to_report();
    /// assert!(report.findings.iter().all(|f| !f.rule_id.is_empty()));
    /// ```
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
            if let Some(detail) = &finding.detail {
                mapped = mapped.with_extra(detail.clone());
            }
            if let Some(location) = finding.location {
                mapped.location.source_hint = Some(location.to_string());
                mapped.location.position = Some(a11y_report::SourcePosition::new(
                    location.line,
                    location.column,
                    location.byte_offset,
                ));
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
            detail: None,
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
        assert_eq!(
            report.findings[0].location.position,
            Some(a11y_report::SourcePosition::new(3, 7, 20))
        );
    }

    #[test]
    fn checked_document_carries_byte_offset_into_report() {
        let html = "<!DOCTYPE html><html lang=en><title>t</title><ul><div>x</div></ul></html>";
        let checked = crate::check(html).unwrap();
        let report = checked.to_report();
        let positioned: Vec<_> = report
            .findings
            .iter()
            .filter_map(|f| f.location.position)
            .collect();
        assert!(!positioned.is_empty(), "{:?}", report.findings);
        for position in positioned {
            assert!(position.byte_offset < html.len());
            assert!(position.line >= 1 && position.column >= 1);
        }
        assert!(positioned_points_at(html, &report, "<div>"));
    }

    #[test]
    fn schema_detail_travels_in_extra_and_stays_out_of_json() {
        let html = "<!DOCTYPE html><html lang=en><title>t</title><ul><div>x</div></ul></html>";
        let checked = crate::check(html).unwrap();
        let with_detail = checked
            .findings
            .iter()
            .filter(|f| f.detail.is_some())
            .count();
        assert!(with_detail > 0, "{:?}", checked.findings);
        let report = checked.to_report();
        let details: Vec<&FindingDetail> = report
            .findings
            .iter()
            .filter_map(|f| f.extra.get::<FindingDetail>())
            .collect();
        assert_eq!(details.len(), with_detail);
        assert!(details.iter().any(|d| matches!(
            d,
            FindingDetail::UnexpectedElement { element } if element == "div"
        )));
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("UnexpectedElement"), "{json}");
    }

    fn positioned_points_at(html: &str, report: &a11y_report::Report, tag: &str) -> bool {
        report
            .findings
            .iter()
            .filter_map(|f| f.location.position)
            .any(|p| html[p.byte_offset..].starts_with(tag))
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
