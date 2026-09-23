//! `<script type="importmap">` and `<script type="speculationrules">`
//! content validation (Phase 08's largest remaining false-negative
//! cluster, ~51 vnu corpus fixtures under `html/elements/script/`).
//!
//! Neither of these fits `rules/*.sch` (Schematron/XPath 1.0 has no JSON
//! support) nor a `w:*` RELAX NG datatype (those validate a single
//! *attribute* value's grammar; this validates an *element's text
//! content* against a JSON structure that itself depends on that
//! element's own `type` attribute — not expressible as a static content
//! model). It also isn't a Schematron co-constraint in the sense
//! `CLAUDE.md` reserves for `rules/*.sch` (cross-element/attribute
//! relationships) — it's a value/content-format check, the same kind of
//! thing `src/parse.rs`'s `doctype_findings` and the `w:*` datatypes
//! already are, just operating on element text content instead of an
//! attribute value or the parse tree's DOCTYPE node. Walks the raw
//! `html5_parser::Document` directly (same as `parse.rs`), before
//! `infoset::normalize()` — script content nodes are ordinary [`NodeKind::Text`]
//! children, no normalization needed to read them.
//!
//! Structure requirements are ported from the real specs (WICG Speculation
//! Rules API, WHATWG Import Maps), not vnu's Java source (unavailable in
//! this environment) — verified against every corpus fixture's exact
//! JSON shape and expected message, both `-novalid` and `-isvalid`, not
//! just the `-novalid` cases this closes. Checks with no corpus fixture
//! either way (e.g. a non-object `scopes` value at the top level) are
//! deliberately left unvalidated rather than guessed — see the two
//! "not evidenced" comments below.

use html5_parser::{Document, NodeId, NodeKind};
use serde_json::Value;

use crate::{Finding, Severity, SourceLocation};

const IMPORT_MAP_RULE_ID: &str = "scripts.import-map";
const SPECULATION_RULES_RULE_ID: &str = "scripts.speculation-rules";

/// Walks the whole document for `<script type="importmap">`/
/// `<script type="speculationrules">` elements and validates each one's
/// text content.
pub(crate) fn findings(document: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    collect(document, document.root(), &mut findings);
    findings
}

fn collect(document: &Document, id: NodeId, findings: &mut Vec<Finding>) {
    let node = document.node(id);
    if let NodeKind::Element {
        name, attributes, ..
    } = &node.kind
        && name.eq_ignore_ascii_case("script")
    {
        let script_type = attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case("type"))
            .map(|attribute| attribute.value.trim());
        let rule_id = match script_type {
            Some(value) if value.eq_ignore_ascii_case("importmap") => Some(IMPORT_MAP_RULE_ID),
            Some(value) if value.eq_ignore_ascii_case("speculationrules") => {
                Some(SPECULATION_RULES_RULE_ID)
            }
            _ => None,
        };
        if let Some(rule_id) = rule_id {
            let text = script_text_content(document, id);
            let violations = if rule_id == IMPORT_MAP_RULE_ID {
                validate_import_map(&text)
            } else {
                validate_speculation_rules(&text)
            };
            let location = node.position.map(|position| SourceLocation {
                line: position.line,
                column: position.column,
                byte_offset: position.byte_offset,
            });
            findings.extend(violations.into_iter().map(|message| Finding {
                rule_id: rule_id.to_owned(),
                severity: Severity::Error,
                message,
                location,
            }));
        }
    }
    for child in document.children(id) {
        collect(document, child, findings);
    }
}

/// Concatenates every direct `Text` child's content, in order — a
/// `<script>` element's content is ordinarily a single text node, but
/// nothing guarantees that (e.g. a literal `<!---->` comment would split
/// it), so this doesn't assume exactly one.
fn script_text_content(document: &Document, id: NodeId) -> String {
    let mut text = String::new();
    for child in document.children(id) {
        if let NodeKind::Text { content } = &document.node(child).kind {
            text.push_str(content);
        }
    }
    text
}

// ---------------------------------------------------------------------
// Import Maps (https://wicg.github.io/import-maps/)
// ---------------------------------------------------------------------

const IMPORT_MAP_INVALID_JSON_MESSAGE: &str = "A script \"script\" with a \"type\" attribute whose value is \"importmap\" must have valid \
     JSON content.";

fn validate_import_map(text: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return vec![IMPORT_MAP_INVALID_JSON_MESSAGE.to_owned()];
    };
    let Value::Object(root) = &value else {
        // Not evidenced by any corpus fixture (every "not valid JSON
        // content" case is a genuine syntax error, not e.g. a JSON
        // array) — reusing the same message is the closest evidenced
        // fallback rather than inventing a new one.
        return vec![IMPORT_MAP_INVALID_JSON_MESSAGE.to_owned()];
    };

    let mut violations = Vec::new();
    for key in root.keys() {
        if key != "imports" && key != "scopes" && key != "integrity" {
            violations.push(
                "A \"script\" element with a \"type\" attribute whose value is \"importmap\" \
                 must contain a JSON object with no properties other than \"imports\", \
                 \"scopes\", and \"integrity\"."
                    .to_owned(),
            );
        }
    }

    if let Some(imports) = root.get("imports") {
        match imports {
            Value::Object(map) => {
                for (key, value) in map {
                    if key.is_empty() {
                        violations.push(specifier_map_message(
                            "imports",
                            "must only contain non-empty keys",
                        ));
                    }
                    match value {
                        Value::String(address) => {
                            if key.ends_with('/') && !address.ends_with('/') {
                                violations.push(specifier_map_message(
                                    "imports",
                                    "must have values that end with \"/\" when its corresponding key ends with \"/\"",
                                ));
                            }
                        }
                        _ => violations.push(specifier_map_message(
                            "imports",
                            "must only contain string values",
                        )),
                    }
                }
            }
            _ => violations.push(
                "The value of the \"imports\" property within the content of a \"script\" \
                 element with a \"type\" attribute whose value is \"importmap\" must be a JSON \
                 object."
                    .to_owned(),
            ),
        }
    }

    if let Some(Value::Object(scopes)) = root.get("scopes") {
        for (scope_key, scope_value) in scopes {
            if !is_url_like_specifier(scope_key) {
                violations.push(
                    "The value of the \"scopes\" property within the content of a \"script\" \
                     element with a \"type\" attribute whose value is \"importmap\" must be a \
                     JSON object whose keys are valid URL strings."
                        .to_owned(),
                );
            }
            match scope_value {
                Value::Object(inner) => {
                    for value in inner.values() {
                        if let Value::String(address) = value
                            && !is_url_like_specifier(address)
                        {
                            violations.push(specifier_map_message(
                                "scopes",
                                "must only contain valid URL values",
                            ));
                        }
                        // A non-string value here isn't evidenced by any
                        // corpus fixture — `imports`' "must only contain
                        // string values" check isn't duplicated for
                        // `scopes`' inner maps without evidence it applies
                        // the same way.
                    }
                }
                _ => violations.push(
                    "The value of the \"scopes\" property within the content of a \"script\" \
                     element with a \"type\" attribute whose value is \"importmap\" must be a \
                     JSON object whose values are also JSON objects."
                        .to_owned(),
                ),
            }
        }
    }
    // `scopes` present but not itself a JSON object: not evidenced by any
    // corpus fixture, left unvalidated rather than guessed.

    violations
}

fn specifier_map_message(property: &str, requirement: &str) -> String {
    format!(
        "A specifier map defined in a \"{property}\" property within the content of a \"script\" \
         element with a \"type\" attribute whose value is \"importmap\" {requirement}."
    )
}

/// WHATWG Import Maps' own `isURLLikeSpecifier` check — verified against
/// `scopes-value-not-url-novalid.html` (address `"..."`, invalid: no
/// scheme, doesn't start with `/`/`./`/`../`) and `scopes-isvalid.html`
/// (`"./value1"`, `"http://www.example.com/value2"`, both valid).
fn is_url_like_specifier(specifier: &str) -> bool {
    specifier.starts_with('/')
        || specifier.starts_with("./")
        || specifier.starts_with("../")
        || url::Url::parse(specifier).is_ok()
}

// ---------------------------------------------------------------------
// Speculation Rules API (https://wicg.github.io/nav-speculation/speculation-rules.html)
// ---------------------------------------------------------------------

fn validate_speculation_rules(text: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return vec![speculation_rules_message("must have valid JSON content")];
    };
    let Value::Object(root) = &value else {
        return vec![speculation_rules_message("must contain a JSON object")];
    };

    let mut violations = Vec::new();
    let mut has_known_key = false;
    for key in root.keys() {
        if key == "prefetch" || key == "prerender" {
            has_known_key = true;
        } else {
            violations.push(speculation_rules_message(
                "must contain a JSON object with only \"prefetch\" and/or \"prerender\" as properties",
            ));
        }
    }
    if !has_known_key {
        violations.push(speculation_rules_message(
            "must contain a JSON object with at least one of the properties \"prefetch\" or \"prerender\"",
        ));
        return violations;
    }

    for list_key in ["prefetch", "prerender"] {
        if let Some(list_value) = root.get(list_key) {
            match list_value {
                Value::Array(items) => {
                    for item in items {
                        validate_speculation_rule(item, list_key, &mut violations);
                    }
                }
                _ => violations.push(format!(
                    "The \"{list_key}\" property within the content of a \"script\" element with \
                     a \"type\" attribute whose value is \"speculationrules\" must be a JSON array."
                )),
            }
        }
    }
    violations
}

const RULE_KEYS: [&str; 4] = ["source", "urls", "where", "eagerness"];

fn validate_speculation_rule(item: &Value, list_key: &str, violations: &mut Vec<String>) {
    let Value::Object(rule) = item else {
        violations.push(format!(
            "Each item in the \"{list_key}\" array within the content of a \"script\" element \
             with a \"type\" attribute whose value is \"speculationrules\" must be a JSON object."
        ));
        return;
    };

    for key in rule.keys() {
        if !RULE_KEYS.contains(&key.as_str()) {
            violations.push(format!(
                "Each rule in the \"{list_key}\" array must only contain the properties \
                 \"source\", \"urls\", \"where\", and \"eagerness\"."
            ));
        }
    }

    let explicit_source = match rule.get("source") {
        Some(Value::String(source)) => Some(source.as_str()),
        Some(_) => {
            violations
                .push("The \"source\" property in a speculation rule must be a string.".to_owned());
            None
        }
        None => None,
    };
    let has_urls = rule.contains_key("urls");
    let has_where = rule.contains_key("where");

    let effective_source = match explicit_source {
        Some("list") => Some("list"),
        Some("document") => Some("document"),
        Some(_) => {
            violations.push(
                "The \"source\" property in a speculation rule must be either \"list\" or \
                 \"document\"."
                    .to_owned(),
            );
            None
        }
        // No explicit "source": inferred from which of urls/where is
        // present, same as a real browser's speculation-rules parser.
        None if has_urls && !has_where => Some("list"),
        None if has_where && !has_urls => Some("document"),
        None if !has_urls && !has_where => {
            violations.push(
                "A speculation rule must have a \"source\" property, or a \"urls\" property \
                 (for list rules), or a \"where\" property (for document rules)."
                    .to_owned(),
            );
            None
        }
        // Both `urls` and `where` present with no explicit `source`:
        // ambiguous, not evidenced by any corpus fixture either way.
        None => None,
    };

    match effective_source {
        Some("list") => {
            if !has_urls {
                violations.push(
                    "A speculation rule with \"source\" set to \"list\" must have a \"urls\" \
                     property."
                        .to_owned(),
                );
            }
            if has_where {
                violations.push(
                    "A speculation rule with \"source\" set to \"list\" must not have a \
                     \"where\" property."
                        .to_owned(),
                );
            }
        }
        Some("document") => {
            if !has_where {
                violations.push(
                    "A speculation rule with \"source\" set to \"document\" must have a \
                     \"where\" property."
                        .to_owned(),
                );
            }
            if has_urls {
                violations.push(
                    "A speculation rule with \"source\" set to \"document\" must not have a \
                     \"urls\" property."
                        .to_owned(),
                );
            }
        }
        _ => {}
    }

    if let Some(urls) = rule.get("urls") {
        validate_urls(urls, violations);
    }
    if let Some(where_value) = rule.get("where") {
        match where_value {
            Value::Object(predicate) => validate_predicate(predicate, violations),
            _ => violations.push(
                "The \"where\" property in a speculation rule must be a JSON object.".to_owned(),
            ),
        }
    }
    if let Some(eagerness) = rule.get("eagerness") {
        match eagerness {
            Value::String(value)
                if value == "eager" || value == "moderate" || value == "conservative" => {}
            Value::String(_) => violations.push(
                "The \"eagerness\" property in a speculation rule must be one of \"eager\", \
                 \"moderate\", or \"conservative\"."
                    .to_owned(),
            ),
            _ => violations.push(
                "The \"eagerness\" property in a speculation rule must be a string.".to_owned(),
            ),
        }
    }
}

fn validate_urls(urls: &Value, violations: &mut Vec<String>) {
    match urls {
        Value::Array(items) => {
            if items.is_empty() {
                violations.push(
                    "The \"urls\" property in a speculation rule must contain at least one URL."
                        .to_owned(),
                );
            }
            for item in items {
                match item {
                    Value::String(url) if url.is_empty() => violations.push(
                        "Each URL in the \"urls\" array must be a non-empty string.".to_owned(),
                    ),
                    Value::String(_) => {}
                    _ => violations
                        .push("Each item in the \"urls\" array must be a string.".to_owned()),
                }
            }
        }
        _ => violations
            .push("The \"urls\" property in a speculation rule must be a JSON array.".to_owned()),
    }
}

const PREDICATE_KEYS: [&str; 5] = ["and", "or", "not", "href_matches", "selector_matches"];

fn validate_predicate(predicate: &serde_json::Map<String, Value>, violations: &mut Vec<String>) {
    let present = PREDICATE_KEYS
        .iter()
        .filter(|key| predicate.contains_key(**key))
        .count();
    if present == 0 {
        violations.push(
            "A document rule predicate must have one of the properties \"and\", \"or\", \
             \"not\", \"href_matches\", or \"selector_matches\"."
                .to_owned(),
        );
    } else if present > 1 {
        violations.push(
            "A document rule predicate must have only one of the properties \"and\", \"or\", \
             \"not\", \"href_matches\", or \"selector_matches\"."
                .to_owned(),
        );
    }

    for name in ["and", "or"] {
        if let Some(value) = predicate.get(name) {
            validate_and_or(value, name, violations);
        }
    }
    if let Some(Value::Object(inner)) = predicate.get("not") {
        validate_predicate(inner, violations);
    }
    // "not" present but not a JSON object: not evidenced by any corpus
    // fixture, left unvalidated rather than guessed.
    for name in ["href_matches", "selector_matches"] {
        if let Some(value) = predicate.get(name) {
            validate_match_pattern(value, name, violations);
        }
    }
}

fn validate_and_or(value: &Value, name: &str, violations: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                violations.push(format!(
                    "The \"{name}\" property in a document rule must contain at least one item."
                ));
            }
            for item in items {
                if let Value::Object(inner) = item {
                    validate_predicate(inner, violations);
                }
                // A non-object item isn't evidenced by any corpus fixture.
            }
        }
        _ => violations.push(format!(
            "The \"{name}\" property in a document rule must be a JSON array."
        )),
    }
}

fn validate_match_pattern(value: &Value, name: &str, violations: &mut Vec<String>) {
    match value {
        Value::String(pattern) if pattern.is_empty() => violations.push(format!(
            "The \"{name}\" property in a document rule must be a non-empty string."
        )),
        Value::String(_) => {}
        Value::Array(items) if items.is_empty() => violations.push(format!(
            "The \"{name}\" property in a document rule must contain at least one pattern."
        )),
        Value::Array(_) => {
            // Per-item type-checking within the array isn't evidenced by
            // any corpus fixture.
        }
        _ => violations.push(format!(
            "The \"{name}\" property in a document rule must be a string or an array of strings."
        )),
    }
}

fn speculation_rules_message(requirement: &str) -> String {
    format!(
        "A \"script\" element with a \"type\" attribute whose value is \"speculationrules\" \
         {requirement}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_map_violations(json: &str) -> Vec<String> {
        validate_import_map(json)
    }

    fn speculation_rules_violations(json: &str) -> Vec<String> {
        validate_speculation_rules(json)
    }

    #[test]
    fn import_map_invalid_json_is_a_single_violation() {
        assert_eq!(import_map_violations("not json at all").len(), 1);
    }

    #[test]
    fn import_map_empty_content_is_invalid_json() {
        assert_eq!(import_map_violations("").len(), 1);
    }

    #[test]
    fn import_map_forbidden_top_level_property_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"forbidden_property": {}}"#).len(),
            1
        );
    }

    #[test]
    fn import_map_valid_imports_scopes_integrity_are_clean() {
        assert!(import_map_violations(
            r#"{"imports": {"app": "/a.js", "dir/": "/dir/"}, "scopes": {"/path/": {"key": "./value"}}, "integrity": {"/a.js": "sha384-x"}}"#
        )
        .is_empty());
    }

    #[test]
    fn import_map_imports_not_object_is_flagged() {
        assert_eq!(import_map_violations(r#"{"imports": "nope"}"#).len(), 1);
    }

    #[test]
    fn import_map_imports_empty_key_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"imports": {"": "/a.js"}}"#).len(),
            1
        );
    }

    #[test]
    fn import_map_imports_non_string_value_is_flagged() {
        assert_eq!(import_map_violations(r#"{"imports": {"app": 1}}"#).len(), 1);
    }

    #[test]
    fn import_map_imports_slash_mismatch_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"imports": {"dir/": "/path/to/dir"}}"#).len(),
            1
        );
    }

    #[test]
    fn import_map_scopes_key_not_url_like_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"scopes": {"not_a_url": {"a": "./b"}}}"#).len(),
            1
        );
    }

    #[test]
    fn import_map_scopes_value_not_object_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"scopes": {"/scope/": "nope"}}"#).len(),
            1
        );
    }

    #[test]
    fn import_map_scopes_inner_value_not_url_like_is_flagged() {
        assert_eq!(
            import_map_violations(r#"{"scopes": {"/scope/": {"a": "..."}}}"#).len(),
            1
        );
    }

    #[test]
    fn speculation_rules_invalid_json_is_a_single_violation() {
        assert_eq!(speculation_rules_violations("{ invalid").len(), 1);
    }

    #[test]
    fn speculation_rules_not_object_is_flagged() {
        assert_eq!(
            speculation_rules_violations(r#"["array", "not", "object"]"#).len(),
            1
        );
    }

    #[test]
    fn speculation_rules_missing_prefetch_and_prerender_is_flagged() {
        assert_eq!(speculation_rules_violations("{}").len(), 1);
    }

    #[test]
    fn speculation_rules_valid_explicit_list_is_clean() {
        assert!(
            speculation_rules_violations(
                r#"{"prefetch": [{"source": "list", "urls": ["https://example.com"]}]}"#
            )
            .is_empty()
        );
    }

    #[test]
    fn speculation_rules_valid_inferred_list_is_clean() {
        assert!(
            speculation_rules_violations(r#"{"prefetch": [{"urls": ["https://example.com"]}]}"#)
                .is_empty()
        );
    }

    #[test]
    fn speculation_rules_valid_inferred_document_is_clean() {
        assert!(
            speculation_rules_violations(
                r#"{"prefetch": [{"eagerness": "moderate", "where": {"href_matches": "/*"}}]}"#
            )
            .is_empty()
        );
    }

    #[test]
    fn speculation_rules_list_with_where_is_flagged() {
        assert_eq!(
            speculation_rules_violations(
                r#"{"prefetch": [{"source": "list", "urls": ["https://example.com"], "where": {"href_matches": "/*"}}]}"#
            )
            .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_document_with_urls_is_flagged() {
        assert_eq!(
            speculation_rules_violations(
                r#"{"prefetch": [{"source": "document", "where": {"href_matches": "/*"}, "urls": ["https://example.com"]}]}"#
            )
            .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_document_missing_where_is_flagged() {
        assert_eq!(
            speculation_rules_violations(r#"{"prefetch": [{"source": "document"}]}"#).len(),
            1
        );
    }

    #[test]
    fn speculation_rules_invalid_eagerness_is_flagged() {
        assert_eq!(
            speculation_rules_violations(
                r#"{"prefetch": [{"urls": ["https://example.com"], "eagerness": "nope"}]}"#
            )
            .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_rule_forbidden_property_is_flagged() {
        assert_eq!(
            speculation_rules_violations(
                r#"{"prefetch": [{"source": "list", "urls": ["https://example.com"], "extra": true}]}"#
            )
            .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_urls_empty_array_is_flagged() {
        assert_eq!(
            speculation_rules_violations(r#"{"prefetch": [{"urls": []}]}"#).len(),
            1
        );
    }

    #[test]
    fn speculation_rules_urls_empty_string_item_is_flagged() {
        assert_eq!(
            speculation_rules_violations(r#"{"prefetch": [{"urls": [""]}]}"#).len(),
            1
        );
    }

    #[test]
    fn speculation_rules_where_multiple_predicates_is_flagged() {
        assert_eq!(
            speculation_rules_violations(
                r#"{"prefetch": [{"source": "document", "where": {"href_matches": "*", "selector_matches": "a"}}]}"#
            )
            .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_where_empty_predicate_is_flagged() {
        assert_eq!(
            speculation_rules_violations(r#"{"prefetch": [{"source": "document", "where": {}}]}"#)
                .len(),
            1
        );
    }

    #[test]
    fn speculation_rules_nested_and_not_is_clean() {
        assert!(speculation_rules_violations(
            r#"{"prefetch": [{"source": "document", "where": {"and": [{"href_matches": "/*"}, {"not": {"selector_matches": ".no-prefetch"}}]}}]}"#
        )
        .is_empty());
    }

    #[test]
    fn speculation_rules_href_matches_array_is_clean() {
        assert!(speculation_rules_violations(
            r#"{"prefetch": [{"source": "document", "where": {"href_matches": ["/blog/*", "/news/*"]}}]}"#
        )
        .is_empty());
    }

    #[test]
    fn speculation_rules_prerender_is_validated_same_as_prefetch() {
        assert_eq!(
            speculation_rules_violations(r#"{"prerender": [{"urls": []}]}"#).len(),
            1
        );
    }
}
