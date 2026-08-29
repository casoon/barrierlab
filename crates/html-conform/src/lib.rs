//! HTML5 conformance checking with browser-style error recovery.
//!
//! `check`/`check_with_options` combine five finding sources, in order:
//! HTML parser diagnostics, RELAX NG schema (content-model) validation,
//! Schematron-style assertion (co-constraint) checking,
//! `<script type="importmap"|"speculationrules">` JSON content validation
//! (`src/scripts.rs` — a value/content-format check like the RELAX NG
//! datatypes, just on element text content instead of an attribute), and
//! `<meta http-equiv="Content-Security-Policy">` enforcement against
//! inline script/style content elsewhere in the document
//! (`src/csp_enforcement.rs` — a genuine cross-element check needing a
//! real CSP source-list parser, not expressible as a `rules/*.sch` rule
//! or a `w:*` datatype).

mod assertions;
mod csp_enforcement;
mod datatypes;
mod finding;
mod infoset;
mod parse;
mod schema;
mod scripts;

use assertions::SchematronEngine;

pub use finding::{CheckError, CheckReport, Finding, Severity, SourceLocation};

/// Options that control which diagnostics are included in a check report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckOptions {
    /// Include recoverable HTML parser diagnostics in the report.
    pub include_parse_errors: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            include_parse_errors: true,
        }
    }
}

/// Checks a complete HTML document with the default options.
///
/// Findings are returned in parser order. Technical initialization failures are
/// returned as [`CheckError`] rather than being represented as findings.
pub fn check(html: &str) -> Result<CheckReport, CheckError> {
    check_with_options(html, CheckOptions::default())
}

/// Checks a complete HTML document with explicit options.
///
/// This function always uses HTML5 error recovery. `include_parse_errors`
/// controls whether recovered parser diagnostics become report findings.
/// Schema and assertion findings are always included — the checker only
/// omits parser diagnostics on request, not the conformance findings that
/// are the point of running it at all.
///
/// # Errors
///
/// Only for a genuine setup failure in this checker itself (the embedded
/// HTML5 schema failed to compile, or the embedded assertion rule set
/// failed to parse) — never for a document that is merely non-conformant,
/// which is reported through [`CheckReport::findings`] instead.
pub fn check_with_options(html: &str, options: CheckOptions) -> Result<CheckReport, CheckError> {
    let parsed = parse::parse(html);
    let document = infoset::normalize(parsed.document(), parsed.source());

    let mut findings = if options.include_parse_errors {
        parse::findings(&parsed)
    } else {
        Vec::new()
    };

    let schema_errors =
        schema::validate_document(&document).map_err(|message| CheckError::Initialization {
            message: format!("schema validation setup failed: {message}"),
        })?;
    findings.extend(schema::findings(&schema_errors));

    let assertion_failures = assertions::RuleSetEngine
        .check(&document)
        .map_err(|error| CheckError::Initialization {
            message: format!("assertion engine setup failed: {error}"),
        })?;
    findings.extend(assertions::findings(&assertion_failures));

    findings.extend(scripts::findings(parsed.document()));
    findings.extend(csp_enforcement::findings(parsed.document()));

    Ok(CheckReport { findings })
}

#[cfg(test)]
mod tests {
    use super::{CheckOptions, SourceLocation, check, check_with_options};

    #[test]
    fn valid_html_has_no_parser_findings() {
        let report = check("<!doctype html><title>Example</title><p>Hello</p>")
            .expect("HTML5 parsing should recover");

        assert!(report.findings.is_empty());
        assert!(!report.has_errors());
    }

    #[test]
    fn parser_diagnostics_can_be_excluded() {
        // Otherwise fully schema-conformant (has a <title>, per
        // schema/html5/meta.rnc's required head.inner) so that excluding
        // the recoverable parser diagnostic (an unknown entity reference)
        // leaves no findings at all — isolates this test to what it's
        // actually about (the `include_parse_errors` toggle), rather than
        // also depending on schema/assertion behavior.
        let report = check_with_options(
            "<!doctype html><title>Example</title><p>&notAnEntity;</p>",
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert!(report.findings.is_empty());
    }

    #[test]
    fn parser_diagnostics_are_included_by_default() {
        let report = check("<!doctype html><title>Example</title><p>&notAnEntity;</p>")
            .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "parser.html5");
    }

    #[test]
    fn schema_violation_is_reported_as_a_finding() {
        // No <title> — schema/html5/meta.rnc's head.inner requires one.
        // Fires against the *synthesized* implicit `<head>` (no explicit
        // `<head>` tag in this input), so `location` is `None` here — see
        // `schema_violation_location_is_populated_for_an_explicit_element`
        // below for the populated case.
        let report = check_with_options(
            "<!doctype html><p>Hello</p>",
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "schema.html5");
        assert_eq!(report.findings[0].location, None);
        assert!(report.has_errors());
    }

    #[test]
    fn schema_violation_location_is_populated_for_an_explicit_element() {
        // Phase 08: `relax_ng::Element::Location` became generic
        // (`src/infoset.rs` sets it to `crate::finding::SourceLocation`
        // directly) — a schema.html5 finding against an *explicit*
        // element now carries a real, structured position, not `None`.
        let report = check_with_options(
            r#"<!doctype html><title>x</title><p bogus="1">hi</p>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "schema.html5");
        assert_eq!(
            report.findings[0].location,
            Some(SourceLocation {
                line: 1,
                column: 32,
                byte_offset: 31,
            })
        );
    }

    #[test]
    fn assertion_violation_is_reported_as_a_finding() {
        let report = check_with_options(
            r#"<!doctype html><title>Example</title><div aria-hidden="true" tabindex="0">x</div>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].rule_id,
            "assertion.aria.hidden-not-focusable"
        );
    }
}
