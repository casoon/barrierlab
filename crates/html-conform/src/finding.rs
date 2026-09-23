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
}

impl CheckReport {
    /// Returns whether the report contains an error-level finding.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
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
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization { message } => {
                write!(formatter, "checker initialization failed: {message}")
            }
        }
    }
}

impl Error for CheckError {}
