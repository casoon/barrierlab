//! Co-constraint (Schematron-style) assertion checking — ARIA combinations,
//! form-label requirements, table semantics, obsolete elements — behind the
//! [`SchematronEngine`] trait, so this module never talks to a concrete
//! Schematron backend directly outside [`RuleSetEngine`]'s single
//! implementation of it (`CLAUDE.md`, "Feste Regeln").
//!
//! Rule files (`rules/*.sch`) are the only place for this domain's fachliche
//! Logik — declarative XPath, no Rust co-constraint code (`CLAUDE.md`). This
//! module only loads, runs, and maps their results; see `rules/README.md`
//! for the rule-authoring contract (notably: named element tests need the
//! `h:` namespace prefix against this crate's XHTML-namespaced infoset).
//!
//! [`findings`]/[`RuleSetEngine`] are wired into
//! [`crate::check`]/[`crate::check_with_options`], combined there with
//! schema (RELAX NG) and parser findings into one [`crate::CheckReport`].

use std::error::Error;
use std::fmt;
use std::sync::LazyLock;

use crate::finding::{Finding, Severity};
use crate::infoset::NormalizedHtmlDocument;

/// A structured Schematron assertion failure — [`SchematronEngine`]'s only
/// output shape, engine-agnostic (no `schematron_engine::Report` or other
/// backend type leaks through this module's public surface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssertionFailure {
    /// The fired check's stable id (`assert`/`report`'s `@id` in the rule
    /// file). Every rule in `rules/*.sch` must have one — a fired check
    /// with no id is a rule-authoring bug, surfaced as an [`EngineError`]
    /// rather than silently producing an empty/placeholder id (see
    /// [`RuleSetEngine::check`]).
    pub(crate) rule_id: String,
    /// Derived from the rule's `@role` attribute — see
    /// [`severity_from_role`] for the convention this crate defines (ISO
    /// Schematron's `role` is free text with no standardized severity
    /// meaning).
    pub(crate) severity: Severity,
    pub(crate) message: String,
    /// The fired check's context node's position, if any (`None` for a
    /// node html5-parser itself synthesizes — an implied `<html>`/
    /// `<head>`/`<body>`, say — see `src/infoset.rs`'s `normalize` doc
    /// comment). Sourced from `schematron_engine::Report::node`, not
    /// computed here — see `RuleSetEngine::check`.
    pub(crate) location: Option<crate::finding::SourceLocation>,
}

/// An assertion-engine evaluation failure — a bug in the rule set or its
/// loading (a malformed `test`/`context` expression, a missing `@id`), not
/// a property of the checked document. Distinct from a `Vec<AssertionFailure>`
/// being empty (which just means no rule fired).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineError(String);

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "assertion engine error: {}", self.0)
    }
}

impl Error for EngineError {}

/// Abstraction over a Schematron rule-evaluation backend. See this module's
/// doc comment for why `assertions.rs` is required to go through this trait
/// rather than a concrete engine type directly.
pub(crate) trait SchematronEngine {
    /// Runs this engine's rule set against `document`, in rule-file/pattern/
    /// rule order. `Err` only for a genuine evaluation failure (the rule
    /// set itself is broken), never for "no rule fired" (that's `Ok(vec![])`).
    fn check(
        &self,
        document: &NormalizedHtmlDocument,
    ) -> Result<Vec<AssertionFailure>, EngineError>;
}

/// This crate's own convention for turning ISO Schematron's free-text
/// `role` attribute into a [`Severity`] — ISO Schematron itself assigns no
/// standardized meaning to `role`. `"warning"`/`"info"` (case-sensitive,
/// matching this crate's own `rules/*.sch` authoring convention) map to the
/// matching [`Severity`]; anything else (including no `role` at all, or
/// literally `"error"`) defaults to [`Severity::Error`] — a conformance
/// checker's assertions are errors unless a rule explicitly opts into a
/// softer severity.
fn severity_from_role(role: Option<&str>) -> Severity {
    match role {
        Some("warning") => Severity::Warning,
        Some("info") => Severity::Info,
        _ => Severity::Error,
    }
}

/// The rule files this crate's assertion layer embeds, in the fixed order
/// they're evaluated and findings are reported in. Each is a complete,
/// independently valid ISO Schematron `<schema>` document — see
/// `rules/README.md`.
const RULE_FILES: &[(&str, &str)] = &[
    ("aria.sch", include_str!("../rules/aria.sch")),
    ("tables.sch", include_str!("../rules/tables.sch")),
    (
        "obsolete-elements.sch",
        include_str!("../rules/obsolete-elements.sch"),
    ),
    ("ids.sch", include_str!("../rules/ids.sch")),
    ("microdata.sch", include_str!("../rules/microdata.sch")),
    ("headings.sch", include_str!("../rules/headings.sch")),
    ("roles.sch", include_str!("../rules/roles.sch")),
    ("elements.sch", include_str!("../rules/elements.sch")),
    (
        "aria-constraints.sch",
        include_str!("../rules/aria-constraints.sch"),
    ),
    (
        "aria-html-restrictions.sch",
        include_str!("../rules/aria-html-restrictions.sch"),
    ),
    ("attributes.sch", include_str!("../rules/attributes.sch")),
];

/// Parses every embedded [`RULE_FILES`] entry once, in order, failing loudly
/// (rather than skipping) if any of them doesn't parse — a broken rule file
/// is a build-time bug, not a per-document condition.
fn parse_rule_files() -> Result<Vec<schematron_engine::Schema>, EngineError> {
    RULE_FILES
        .iter()
        .map(|(name, xml)| {
            schematron_engine::parse(xml)
                .map_err(|error| EngineError(format!("rules/{name}: {error}")))
        })
        .collect()
}

/// Compiled once (see `src/schema.rs`'s `HTML5_SCHEMA` for the same
/// pattern) and cached for the process lifetime.
static RULE_SCHEMAS: LazyLock<Result<Vec<schematron_engine::Schema>, EngineError>> =
    LazyLock::new(parse_rule_files);

/// The [`SchematronEngine`] implementation backed by `schematron-engine`
/// and this crate's embedded [`RULE_FILES`]. The only place in this crate
/// that names `schematron_engine` types outside this module's own
/// [`SchematronEngine`]-trait boundary.
pub(crate) struct RuleSetEngine;

impl SchematronEngine for RuleSetEngine {
    fn check(
        &self,
        document: &NormalizedHtmlDocument,
    ) -> Result<Vec<AssertionFailure>, EngineError> {
        let schemas = RULE_SCHEMAS.as_ref().map_err(Clone::clone)?;

        let mut failures = Vec::new();
        for schema in schemas {
            let reports = schematron_engine::evaluate(schema, document)
                .map_err(|error| EngineError(error.to_string()))?;
            for report in reports {
                let Some(rule_id) = report.check_id else {
                    return Err(EngineError(format!(
                        "fired check in pattern {:?} has no @id — every rules/*.sch assert/report must declare one",
                        report.pattern_id
                    )));
                };
                failures.push(AssertionFailure {
                    rule_id,
                    severity: severity_from_role(report.role.as_deref()),
                    message: report.message,
                    location: report.node.position().copied(),
                });
            }
        }
        Ok(failures)
    }
}

/// Maps [`AssertionFailure`]s onto this crate's public [`Finding`] model.
/// `rule_id` is prefixed with `assertion.` (plan/06-assertions-engine.md)
/// to namespace it against the `parser.*`/future `schema.*` layers'
/// findings — e.g. `assertion.aria.hidden-not-focusable`.
pub(crate) fn findings(failures: &[AssertionFailure]) -> Vec<Finding> {
    failures
        .iter()
        .map(|failure| Finding {
            rule_id: format!("assertion.{}", failure.rule_id),
            severity: failure.severity,
            message: failure.message.clone(),
            location: failure.location,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{AssertionFailure, EngineError, RuleSetEngine, SchematronEngine, findings};
    use crate::finding::Severity;
    use crate::infoset::normalize;
    use crate::parse::parse;

    /// Wraps `body` in a minimal document carrying `lang` on `<html>`, so
    /// that `rules/elements.sch`'s `elements-html-missing-lang` — which
    /// fires on *every* document without one, by design — doesn't add a
    /// warning to every rule test below. Tests that are about that rule
    /// call [`check_html`] directly.
    fn check_body(body: &str) -> Vec<AssertionFailure> {
        check_html(&format!(r#"<html lang="en">{body}"#))
    }

    fn check_html(html: &str) -> Vec<AssertionFailure> {
        let parsed = parse(html);
        let document = normalize(parsed.document(), parsed.source());
        RuleSetEngine
            .check(&document)
            .expect("rule set should evaluate without an engine error")
    }

    #[test]
    fn all_embedded_rule_files_parse() {
        // Mirrors src/schema.rs's `embedded_html5_schema_compiles` — a
        // broken rules/*.sch file should fail a test, not surface only at
        // first real use.
        super::parse_rule_files().expect("every embedded rule file should parse");
    }

    #[test]
    fn aria_hidden_with_tabindex_fires() {
        let failures = check_body(r#"<div aria-hidden="true" tabindex="0">x</div>"#);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].rule_id, "aria.hidden-not-focusable");
        assert_eq!(failures[0].severity, Severity::Error);
    }

    #[test]
    fn aria_hidden_without_tabindex_is_clean() {
        let failures = check_body(r#"<div aria-hidden="true">x</div>"#);
        assert!(failures.is_empty());
    }

    #[test]
    fn th_scope_enum_valid_and_invalid() {
        assert!(check_body(r#"<table><tr><th scope="col">A</th></tr></table>"#).is_empty());
        let failures = check_body(r#"<table><tr><th scope="column">A</th></tr></table>"#);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].rule_id, "tables.th-scope-enum");
    }

    #[test]
    fn obsolete_elements_fire_and_ordinary_elements_dont() {
        let failures = check_body("<font>x</font><center>y</center>");
        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .all(|f| f.rule_id == "obsolete-elements.deprecated")
        );

        assert!(check_body("<p>ordinary</p>").is_empty());
    }

    /// `rules/elements.sch`'s `elements-html-missing-lang`. vnu's
    /// `LanguageDetectingChecker.warnIfMissingLang()` keys off the mere
    /// *presence* of a `lang` attribute on `html`, so an empty value is
    /// silent too — asserted here so the distinction can't erode.
    #[test]
    fn html_missing_lang_warns_and_any_lang_value_is_silent() {
        let failures = check_html("<html><title>x</title>");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].rule_id, "elements.html-missing-lang");
        assert_eq!(failures[0].severity, Severity::Warning);

        assert!(check_html(r#"<html lang="en"><title>x</title>"#).is_empty());
        assert!(check_html(r#"<html lang=""><title>x</title>"#).is_empty());
    }

    #[test]
    fn multiple_rule_files_fire_independently_in_one_document() {
        let failures = check_body(
            r#"<div aria-hidden="true" tabindex="0">x</div><table><tr><th scope="column">A</th></tr></table><font>x</font>"#,
        );
        let rule_ids: Vec<&str> = failures.iter().map(|f| f.rule_id.as_str()).collect();
        assert!(rule_ids.contains(&"aria.hidden-not-focusable"));
        assert!(rule_ids.contains(&"tables.th-scope-enum"));
        assert!(rule_ids.contains(&"obsolete-elements.deprecated"));
    }

    #[test]
    fn findings_prefixes_rule_id_and_carries_severity_and_message() {
        let failures = vec![AssertionFailure {
            rule_id: "aria.hidden-not-focusable".to_owned(),
            severity: Severity::Error,
            message: "boom".to_owned(),
            location: None,
        }];
        let mapped = findings(&failures);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].rule_id, "assertion.aria.hidden-not-focusable");
        assert_eq!(mapped[0].severity, Severity::Error);
        assert_eq!(mapped[0].message, "boom");
        assert_eq!(mapped[0].location, None);
    }

    /// Demonstrates the `SchematronEngine` trait's whole point: a caller
    /// (or a future test elsewhere in this crate) can swap in any
    /// implementation, not just `RuleSetEngine`/`schematron-engine` — this
    /// mock never touches `schematron_engine` or `rules/*.sch` at all.
    struct MockEngine {
        result: Result<Vec<AssertionFailure>, EngineError>,
    }

    impl SchematronEngine for MockEngine {
        fn check(
            &self,
            _document: &crate::infoset::NormalizedHtmlDocument,
        ) -> Result<Vec<AssertionFailure>, EngineError> {
            self.result.clone()
        }
    }

    #[test]
    fn schematron_engine_trait_is_swappable_for_a_mock_implementation() {
        let parsed = parse("<p>irrelevant — the mock ignores the document</p>");
        let document = normalize(parsed.document(), parsed.source());

        let canned = vec![AssertionFailure {
            rule_id: "mock.rule".to_owned(),
            severity: Severity::Warning,
            message: "mock failure".to_owned(),
            location: None,
        }];
        let engine = MockEngine {
            result: Ok(canned.clone()),
        };
        assert_eq!(engine.check(&document).unwrap(), canned);

        let failing_engine = MockEngine {
            result: Err(EngineError("mock engine error".to_owned())),
        };
        assert!(failing_engine.check(&document).is_err());
    }
}
