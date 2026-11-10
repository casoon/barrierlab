//! SARIF 2.1.0 for GitHub Code Scanning.
//!
//! Only findings that were decided become `results` (`Fail` is `error`,
//! `Review` is `warning`). An `Untested` finding, a content class that was
//! not checked, and a file that could not be checked are never results: they
//! are `toolExecutionNotifications` of the single invocation, so a viewer
//! cannot read them as a pass or as an alert.
//!
//! `partialFingerprints["htmlConform/v1"]` is `<hash>:<n>`: `<hash>` is the
//! 64-bit FNV-1a of `rule id NUL normalised path NUL message` in 16 hex
//! digits, `<n>` counts, in position order, the findings of the same file that
//! share rule and message. Line numbers are left out on purpose, so
//! the fingerprint survives edits above a finding.

use std::collections::BTreeMap;

use a11y_report::{Finding, Outcome};
use serde_json::{Value, json};

use crate::input::{FileResult, STDIN_NAME};
use crate::output::{findings, line_column, not_checked_reason, notes, ranges};

const SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json";
const HELP_URI: &str = "https://casoon.github.io/html-conform/docs/guides/rule-ids/";

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub fn fingerprint(rule_id: &str, path: &str, message: &str, occurrence: usize) -> String {
    let key = format!("{rule_id}\0{path}\0{message}");
    format!("{:016x}:{occurrence}", fnv1a(key.as_bytes()))
}

fn describe(catalogue: &[html_conform::RuleInfo], rule_id: &str) -> String {
    catalogue
        .iter()
        .find(|rule| rule.id == rule_id)
        .map_or_else(
            || "html-conform finding".to_owned(),
            |rule| rule.description.clone(),
        )
}

/// Percent-encodes a path for use as a URI reference (`/` is kept).
fn uri(path: &str) -> String {
    let mut out = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    if path.starts_with('/') {
        format!("file://{out}")
    } else {
        out
    }
}

/// `end` is the exclusive end: SARIF's `endColumn` is one past the last
/// column of the region, and `endLine` the line of its last character.
fn location(path: &str, position: Option<(u32, u32)>, end: Option<(u32, u32)>) -> Value {
    let mut physical = json!({
        "artifactLocation": { "uri": uri(path), "uriBaseId": "%SRCROOT%" }
    });
    if path.starts_with('/') {
        physical["artifactLocation"]
            .as_object_mut()
            .expect("object")
            .remove("uriBaseId");
    }
    if let Some((line, column)) = position {
        physical["region"] = json!({ "startLine": line, "startColumn": column });
        if let Some((end_line, end_column)) = end {
            physical["region"]["endLine"] = json!(end_line);
            physical["region"]["endColumn"] = json!(end_column);
        }
    }
    json!({ "physicalLocation": physical })
}

pub fn render(results: &[FileResult]) -> String {
    let mut rule_ids: BTreeMap<&str, usize> = BTreeMap::new();
    for result in results {
        for finding in findings(result).iter().filter(|f| is_result(f)) {
            rule_ids.entry(&finding.rule_id).or_default();
        }
    }
    for (index, slot) in rule_ids.values_mut().enumerate() {
        *slot = index;
    }
    // Findings exist only if the embedded rule files loaded for the check, so
    // the catalogue cannot fail here; without findings it is not needed.
    let catalogue = if rule_ids.is_empty() {
        Vec::new()
    } else {
        html_conform::rules().expect("the rule files loaded for the check that produced findings")
    };
    let rules: Vec<Value> = rule_ids
        .keys()
        .map(|id| {
            json!({
                "id": id,
                "shortDescription": { "text": describe(&catalogue, id) },
                "helpUri": HELP_URI,
            })
        })
        .collect();

    let mut sarif_results = Vec::new();
    let mut notifications = Vec::new();
    let mut successful = true;
    for result in results {
        let mut seen: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for (finding, range) in findings(result).iter().zip(ranges(result)) {
            let position = line_column(finding);
            let end = range.map(|range| (range.end.line, range.end.column));
            if !is_result(finding) {
                notifications.push(json!({
                    "level": "note",
                    "message": { "text": format!("not evaluated: {}", finding.message) },
                    "descriptor": { "id": finding.rule_id },
                    "locations": [location(&result.path, position, end)],
                }));
                continue;
            }
            let count = seen
                .entry((&finding.rule_id, &finding.message))
                .or_default();
            *count += 1;
            sarif_results.push(json!({
                "ruleId": finding.rule_id,
                "ruleIndex": rule_ids[finding.rule_id.as_str()],
                "level": if finding.outcome == Outcome::Fail { "error" } else { "warning" },
                "message": { "text": finding.message },
                "locations": [location(&result.path, position, end)],
                "partialFingerprints": {
                    "htmlConform/v1": fingerprint(&finding.rule_id, &result.path, &finding.message, *count)
                },
            }));
        }
        if let Some(reason) = not_checked_reason(result) {
            successful = false;
            let at = (result.path != STDIN_NAME).then(|| location(&result.path, None, None));
            notifications.push(json!({
                "level": "error",
                "message": { "text": format!("not checked: {reason}") },
                "descriptor": { "id": "not-checked" },
                "locations": at.into_iter().collect::<Vec<_>>(),
            }));
        }
        for note in notes(result) {
            notifications.push(json!({
                "level": "note",
                "message": { "text": format!("not fully checked: {}", note.detail) },
                "descriptor": { "id": note.class },
                "locations": [location(&result.path, None, None)],
            }));
        }
    }

    let log = json!({
        "$schema": SCHEMA,
        "version": "2.1.0",
        "runs": [{
            // Columns count Unicode scalar values (`SourceLocation`).
            "columnKind": "unicodeCodePoints",
            "tool": { "driver": {
                "name": "html-conform",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/casoon/barrierlab",
                "rules": rules,
            }},
            "invocations": [{
                "executionSuccessful": successful,
                "toolExecutionNotifications": notifications,
            }],
            "results": sarif_results,
        }],
    });
    let mut text = serde_json::to_string_pretty(&log).expect("sarif serialises");
    text.push('\n');
    text
}

fn is_result(finding: &Finding) -> bool {
    matches!(finding.outcome, Outcome::Fail | Outcome::Review)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable() {
        // Pinned: changing the derivation re-opens every alert downstream.
        let expected = format!("{:016x}:2", fnv1a(b"a\0b.html\0m"));
        assert_eq!(fingerprint("a", "b.html", "m", 2), expected);
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn uri_is_percent_encoded() {
        assert_eq!(uri("a b/ü.html"), "a%20b/%C3%BC.html");
        assert_eq!(uri("/abs/x.html"), "file:///abs/x.html");
    }
}
