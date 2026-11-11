//! Unicode Normalization Form C check for text runs.
//!
//! The HTML Standard asks authors for NFC text (Character encodings: "Authors
//! are encouraged to use ... normalized in Unicode Normalization Form C",
//! with the Character Model for the World Wide Web: String Matching as the
//! reason). vnu implements this as a warning in its `NormalizationChecker`;
//! this module reproduces the one check the W3C SVG test
//! `struct-cond-02-t-haswarn` exercises: a **text run** that is not in NFC.
//!
//! A text run is the concatenation of consecutive character data between
//! element boundaries; comments do not end a run, start and end tags and
//! processing instructions do (vnu: `flush()` at start tag, end tag and
//! processing instruction). Like vnu, only the first non-NFC piece of a run is
//! reported, once per run, with the NFC form as the suggestion.
//!
//! Not covered (vnu reports them too): "starts with a composing character"
//! for text runs, names and attribute values, and attribute values that are
//! not in NFC. They need the canonical combining class table in addition.
//!
//! Walks the raw `html5_parser::Document`, like `scripts.rs`.

use html5_parser::{Document, NodeId, NodeKind};
use icu_normalizer::ComposingNormalizerBorrowed;

use crate::{Finding, Severity, SourceLocation};

pub(crate) const RULE_ID: &str = "text.normalization";

pub(crate) fn findings(document: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    // Iterative: the tree may be as deep as `MAX_NESTING_DEPTH`.
    let mut stack = vec![document.root()];
    while let Some(id) = stack.pop() {
        collect_runs(document, id, &mut findings);
        let children: Vec<NodeId> = document.children(id).collect();
        stack.extend(children.into_iter().rev());
    }
    findings
}

/// Splits the direct children of `id` into text runs and checks each.
fn collect_runs(document: &Document, id: NodeId, findings: &mut Vec<Finding>) {
    let mut run = String::new();
    let mut start: Option<SourceLocation> = None;
    for child in document.children(id) {
        let node = document.node(child);
        match &node.kind {
            NodeKind::Text { content } => {
                if run.is_empty() {
                    start = node.position.map(|position| SourceLocation {
                        line: position.line,
                        column: position.column,
                        byte_offset: position.byte_offset,
                    });
                }
                run.push_str(content);
            }
            NodeKind::Comment { .. } => {}
            _ => flush(&mut run, start, findings),
        }
    }
    flush(&mut run, start, findings);
}

fn flush(run: &mut String, location: Option<SourceLocation>, findings: &mut Vec<Finding>) {
    let nfc = ComposingNormalizerBorrowed::new_nfc();
    if !run.is_empty() && !nfc.is_normalized(run) {
        let normalized = nfc.normalize(run);
        findings.push(Finding {
            rule_id: RULE_ID.to_owned(),
            severity: Severity::Warning,
            message: format!(
                "Text run is not in Unicode Normalization Form C. Should instead be \u{201c}{normalized}\u{201d}. \
                 (Copy and paste that into your source document to replace the un-normalized text.)"
            ),
            location,
            detail: None,
        });
    }
    run.clear();
}

#[cfg(test)]
mod tests {
    use crate::check;

    fn messages(body: &str) -> Vec<String> {
        check(&format!(
            "<!doctype html><html lang=en><title>t</title><body>{body}"
        ))
        .expect("check")
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id == super::RULE_ID)
        .map(|finding| finding.message)
        .collect()
    }

    #[test]
    fn decomposed_text_is_reported_with_the_nfc_form() {
        let found = messages("<p>Vie\u{0323}\u{0302}t</p>");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("\u{201c}Vi\u{1ec7}t\u{201d}"),
            "{found:?}"
        );
    }

    #[test]
    fn composed_text_is_not_reported() {
        assert!(messages("<p>Vi\u{1ec7}t \u{00e9}</p>").is_empty());
    }

    #[test]
    fn a_run_is_reported_once_and_comments_do_not_split_it() {
        // "e" + comment + U+0301 is one run: NFC composes it.
        assert_eq!(messages("<p>e<!-- c -->\u{0301}</p>").len(), 1);
        // An element boundary ends the run: the accent alone is already NFC-stable.
        assert!(messages("<p>e<b></b>\u{0301}</p>").is_empty());
        assert_eq!(messages("<p>e\u{0301} e\u{0301}</p>").len(), 1);
    }
}
