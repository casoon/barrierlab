//! The catalogue of rule ids this crate can report, for hosts that explain a
//! finding (hover text, SARIF `tool.driver.rules`, an agent's rule lookup).
//!
//! Nothing here is a second list: the assertion rules are read from the
//! parsed `rules/*.sch` files the checker runs, and the other layers use the
//! constants their findings are built with.

use a11y_report::Outcome;
use serde::Serialize;

use crate::finding::{CheckError, Severity};
use crate::{assertions, csp_enforcement, parse, schema, scripts, table_integrity};

/// The layer that reports a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuleLayer {
    /// HTML5 tokenizer and tree-construction parse errors.
    Parser,
    /// RELAX NG schema validation.
    Schema,
    /// Schematron rule in `rules/*.sch`.
    Assertion,
    /// JSON content of `<script type="importmap"|"speculationrules">`.
    Scripts,
    /// Inline content against a `<meta>` Content-Security-Policy.
    Csp,
    /// Table cell grid.
    Tables,
}

/// One rule id html-conform can report in `Finding::rule_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RuleInfo {
    /// The id as it appears in findings, e.g. `assertion.ids.duplicate`.
    pub id: String,
    /// The layer that reports it.
    pub layer: RuleLayer,
    /// The outcome of its findings in the shared model: `Fail` for an
    /// error, `Review` for a warning, `Untested` for an info. A layer-wide
    /// id such as `csp.meta-enforcement` can also report `Untested` (hash
    /// sources).
    pub outcome: Outcome,
    /// One line on what the rule reports. For an assertion rule it is the
    /// message text of its Schematron check, computed parts shown as `…`.
    pub description: String,
    /// The specification the rule rests on, where the code states one. The
    /// Schematron rule files carry no source reference, so assertion rules
    /// have `None`.
    pub normative_source: Option<String>,
}

fn outcome(severity: Severity) -> Outcome {
    match severity {
        Severity::Error => Outcome::Fail,
        Severity::Warning => Outcome::Review,
        Severity::Info => Outcome::Untested,
    }
}

fn layer_rule(
    id: &str,
    layer: RuleLayer,
    outcome: Outcome,
    description: &str,
    normative_source: &str,
) -> RuleInfo {
    RuleInfo {
        id: id.to_owned(),
        layer,
        outcome,
        description: description.to_owned(),
        normative_source: Some(normative_source.to_owned()),
    }
}

/// Every rule id html-conform can report: the layer-wide ids first, then
/// the Schematron assertion rules in evaluation order.
///
/// `parser.html5` and `schema.html5` are single ids covering many
/// conditions (the parser's error kinds, the schema's content-model and
/// datatype errors); the finding message names the condition.
///
/// # Errors
///
/// [`CheckError::Initialization`] when the embedded rule files do not parse,
/// as for [`crate::check`].
///
/// ```
/// let rules = html_conform::rules().unwrap();
/// assert!(rules.iter().any(|rule| rule.id == "schema.html5"));
/// assert!(rules.iter().any(|rule| rule.id == "assertion.ids.duplicate"));
/// ```
pub fn rules() -> Result<Vec<RuleInfo>, CheckError> {
    let mut catalogue = vec![
        layer_rule(
            parse::RULE_ID,
            RuleLayer::Parser,
            Outcome::Fail,
            "HTML5 tokenizer or tree-construction parse error",
            "WHATWG HTML Standard, 13.2 Parsing HTML documents (parse errors)",
        ),
        layer_rule(
            schema::RULE_ID,
            RuleLayer::Schema,
            Outcome::Fail,
            "Content model, attribute or attribute value rejected by the HTML schema",
            "W3C RELAX NG schema for HTML (vnu), after the WHATWG HTML Standard",
        ),
        layer_rule(
            scripts::IMPORT_MAP_RULE_ID,
            RuleLayer::Scripts,
            Outcome::Fail,
            "Invalid JSON content of a script of type importmap",
            "WHATWG HTML Standard, import maps",
        ),
        layer_rule(
            scripts::SPECULATION_RULES_RULE_ID,
            RuleLayer::Scripts,
            Outcome::Fail,
            "Invalid JSON content of a script of type speculationrules",
            "WHATWG HTML Standard, speculation rules",
        ),
        layer_rule(
            csp_enforcement::RULE_ID,
            RuleLayer::Csp,
            Outcome::Review,
            "Inline content a meta Content-Security-Policy would block",
            "Content Security Policy Level 3",
        ),
        layer_rule(
            table_integrity::RULE_ID,
            RuleLayer::Tables,
            Outcome::Fail,
            "Inconsistent table cell grid",
            "WHATWG HTML Standard, processing model of tables",
        ),
    ];
    let declared = assertions::declared_rules().map_err(|error| CheckError::Initialization {
        message: format!("assertion rules unavailable: {error}"),
    })?;
    catalogue.extend(declared.into_iter().map(|rule| RuleInfo {
        id: format!("assertion.{}", rule.id),
        layer: RuleLayer::Assertion,
        outcome: outcome(rule.severity),
        description: rule.description,
        normative_source: None,
    }));
    Ok(catalogue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_assertions_come_from_the_rule_files() {
        let catalogue = rules().expect("rules load");
        let mut ids: Vec<_> = catalogue.iter().map(|rule| rule.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "duplicate rule id");
        assert!(
            catalogue
                .iter()
                .filter(|rule| rule.layer == RuleLayer::Assertion)
                .count()
                > 250
        );
        let missing_lang = catalogue
            .iter()
            .find(|rule| rule.id == "assertion.elements.html-missing-lang")
            .expect("declared in rules/elements.sch");
        assert_eq!(missing_lang.outcome, Outcome::Review);
        assert!(catalogue.iter().all(|rule| !rule.description.is_empty()));
    }
}
