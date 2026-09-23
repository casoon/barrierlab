use html5_parser::{Document, NodeKind, ParseError};

use crate::{Finding, Severity, SourceLocation};

/// HTML parsing output retained for the later validation layers.
pub(crate) struct ParsedHtml {
    document: Document,
    source: String,
    diagnostics: Vec<ParseError>,
}

impl ParsedHtml {
    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn diagnostics(&self) -> &[ParseError] {
        &self.diagnostics
    }
}

/// Parses HTML with the WHATWG error-recovery algorithm.
pub(crate) fn parse(html: &str) -> ParsedHtml {
    let result = html5_parser::parse(html);

    ParsedHtml {
        document: result.document,
        source: html.to_owned(),
        diagnostics: result.errors,
    }
}

/// Maps parser diagnostics onto the crate's public finding model.
///
/// Every diagnostic maps to `Severity::Error`. `html5_parser::ParseError`
/// (unlike xmloxide's old `ParseDiagnostic`, which carried its own
/// Warning/Error/Fatal severity) has no severity of its own — a WHATWG
/// "parse error" (§13.2.2) is just that, a single undifferentiated
/// category, and `html5_parser::ParseErrorKind`'s doc comment doesn't
/// define one either. Treating every occurrence as an error is the
/// straightforward reading of the spec, not a guess at a severity
/// gradient the spec itself doesn't define. `Position`'s line/column are
/// always 1-based and meaningful (no "unknown location" sentinel the way
/// xmloxide's `(0, 0)` was), so the location is always `Some`.
pub(crate) fn findings(parsed: &ParsedHtml) -> Vec<Finding> {
    parsed
        .diagnostics()
        .iter()
        .map(|diagnostic| Finding {
            rule_id: "parser.html5".to_owned(),
            severity: Severity::Error,
            message: diagnostic.kind.to_string(),
            location: Some(SourceLocation {
                line: diagnostic.position.line,
                column: diagnostic.position.column,
                byte_offset: diagnostic.position.byte_offset,
            }),
        })
        .chain(doctype_findings(parsed.document()))
        .chain(charset_after_1024_findings(parsed.document()))
        .collect()
}

/// The "initial" insertion mode's DOCTYPE handling (§13.2.6.4.1) is a
/// **tree-construction**-level parse error, not a tokenizer one — out of
/// scope for `html5_parser::ParseErrorKind` (Phase 07 there, "Slice 1",
/// deliberately scoped to the 52 named *tokenizer* errors only; see
/// `../html5-parser/plan/DECISIONS.md`). Detectable entirely from the
/// already-parsed [`Document`] here instead, with no `html5-parser`
/// change needed: a conforming document has exactly one `Doctype` child
/// of the document root, with name `"html"` and no public/system
/// identifier (or a system identifier of exactly `"about:legacy-compat"`)
/// — anything else is a parse error, classified per the spec's own
/// quirks-mode-detection algorithm.
///
/// **Not covered here** (would need real `html5-parser` tree-construction
/// tracking, not post-hoc tree inspection): a *second*, stray `<!DOCTYPE>`
/// appearing after the document preamble is parse-error-and-discarded by
/// the tree builder per spec (never inserted into the tree at all in most
/// insertion modes), so by the time this function sees the final
/// [`Document`] there is no trace of it left to detect — a documented,
/// accepted residual gap (`html/parser/stray-doctype-novalid.html`).
fn doctype_findings(document: &Document) -> Vec<Finding> {
    let doctype = document.children(document.root()).find_map(|id| {
        let NodeKind::Doctype {
            name,
            public_identifier,
            system_identifier,
        } = &document.node(id).kind
        else {
            return None;
        };
        Some((id, name.as_deref(), public_identifier, system_identifier))
    });

    let Some((id, name, public_identifier, system_identifier)) = doctype else {
        // No `Doctype` node in the tree at all — either genuinely no
        // `<!DOCTYPE>` was ever written, or the first thing the parser
        // saw was a start tag/EOF before one. Either way, WHATWG's
        // "initial" insertion mode's "anything else" branch: a parse
        // error. No natural node position exists to attach here (the
        // implicit `<html>` root is itself synthesized, no source
        // position — see `NormalizedNode::position`'s own doc comment).
        return vec![Finding {
            rule_id: "parser.html5".to_owned(),
            severity: Severity::Error,
            message: "start tag seen without seeing a doctype first, expected \
                      “<!DOCTYPE html>”"
                .to_owned(),
            location: None,
        }];
    };

    let is_public_id_missing = public_identifier.as_deref().is_none_or(str::is_empty);
    let is_system_id_missing_or_legacy_compat = matches!(
        system_identifier.as_deref(),
        None | Some("") | Some("about:legacy-compat")
    );
    if name == Some("html") && is_public_id_missing && is_system_id_missing_or_legacy_compat {
        return Vec::new();
    }

    let location = document.node(id).position.map(|position| SourceLocation {
        line: position.line,
        column: position.column,
        byte_offset: position.byte_offset,
    });
    let message = if matches_limited_quirks_doctype(
        public_identifier.as_deref(),
        system_identifier.as_deref(),
    ) {
        "almost standards mode doctype, expected “<!DOCTYPE html>”"
    } else {
        "obsolete doctype, expected “<!DOCTYPE html>”"
    };
    vec![Finding {
        rule_id: "parser.html5".to_owned(),
        severity: Severity::Error,
        message: message.to_owned(),
        location,
    }]
}

/// WHATWG's "limited-quirks mode" DOCTYPE conditions (§13.2.6.4.1) —
/// ASCII-case-insensitive `public identifier starts-with` matches, two of
/// which are additionally gated on a non-missing/non-empty system
/// identifier. Deliberately *not* also implementing the much larger
/// "full quirks mode" list right above it in the spec: vnu's own two
/// relevant corpus fixtures (`html/parser/legacy-doctype-novalid.html`,
/// a full-quirks-list match, and `quirky-doctype-novalid.html`, which
/// matches neither list at all) both report the exact same "Obsolete
/// doctype" message — the finer WHATWG quirks/no-match distinction
/// collapses to one vnu message either way, so there is nothing the full
/// list would change here.
fn matches_limited_quirks_doctype(
    public_identifier: Option<&str>,
    system_identifier: Option<&str>,
) -> bool {
    let Some(public_identifier) = public_identifier else {
        return false;
    };
    let starts_with_ci = |prefix: &str| {
        public_identifier.len() >= prefix.len()
            && public_identifier[..prefix.len()].eq_ignore_ascii_case(prefix)
    };
    let system_id_present = !matches!(system_identifier, None | Some(""));

    starts_with_ci("-//W3C//DTD XHTML 1.0 Frameset//")
        || starts_with_ci("-//W3C//DTD XHTML 1.0 Transitional//")
        || (system_id_present && starts_with_ci("-//W3C//DTD HTML 4.01 Frameset//"))
        || (system_id_present && starts_with_ci("-//W3C//DTD HTML 4.01 Transitional//"))
}

fn charset_after_1024_findings(document: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    fn walk(document: &Document, id: html5_parser::NodeId, findings: &mut Vec<Finding>) {
        let node = document.node(id);
        if let NodeKind::Element {
            name, attributes, ..
        } = &node.kind
        {
            let is_meta = name.eq_ignore_ascii_case("meta");
            let has_charset = is_meta
                && attributes.iter().any(|attr| {
                    attr.name.eq_ignore_ascii_case("charset")
                        || (attr.name.eq_ignore_ascii_case("http-equiv")
                            && attr.value.eq_ignore_ascii_case("content-type"))
                });
            if has_charset {
                let pos = node
                    .position
                    .and_then(|p| (p.byte_offset > 1024).then_some(p));
                if let Some(pos) = pos {
                    findings.push(Finding {
                        rule_id: "parser.html5".to_owned(),
                        severity: Severity::Error,
                        message:
                            "A “charset” attribute on a “meta” element found after the first 1024 bytes."
                                .to_owned(),
                        location: Some(SourceLocation {
                            line: pos.line,
                            column: pos.column,
                            byte_offset: pos.byte_offset,
                        }),
                    });
                }
            }
        }
        for child_id in document.children(id) {
            walk(document, child_id, findings);
        }
    }
    walk(document, document.root(), &mut findings);
    findings
}

#[cfg(test)]
mod tests {
    // Phase 02 dependency spike (now html5-parser, see plan/DECISIONS.md's
    // Phase 08 migration entry): confirm the parser parses minimal HTML
    // into a tree. See plan/02-dependency-spike.md.
    use html5_parser::NodeKind;

    use super::{findings, parse};

    #[test]
    fn smoke_parses_minimal_html_to_tree() {
        let parsed = parse("<p>Hello <b>world</b></p>");
        let document = parsed.document();
        let root_element = document
            .children(document.root())
            .find(|&node| matches!(document.node(node).kind, NodeKind::Element { .. }))
            .expect("document should have a root element");
        let NodeKind::Element { name, .. } = &document.node(root_element).kind else {
            unreachable!("just matched as Element above");
        };
        assert_eq!(name, "html");
    }

    #[test]
    fn retains_recoverable_parse_diagnostic_with_location() {
        // `<!doctype html>` here specifically so this test isolates the
        // tokenizer-level diagnostic it's named for — without it, the new
        // `doctype_findings` (missing doctype) would also fire, adding an
        // unrelated second finding.
        let parsed = parse("<!doctype html><p>&notAnEntity;</p>");
        let parser_findings = findings(&parsed);

        assert_eq!(parser_findings.len(), 1);
        assert_eq!(parser_findings[0].rule_id, "parser.html5");
        assert!(parser_findings[0].location.is_some());
    }

    #[test]
    fn plain_doctype_html_has_no_doctype_finding() {
        let parsed = parse("<!doctype html><title>t</title>");
        assert!(findings(&parsed).is_empty());
    }

    #[test]
    fn missing_doctype_is_a_finding_with_no_location() {
        // html/parser/no-doctype-novalid.html
        let parsed = parse("<meta charset=utf-8><title>no doctype</title>");
        let parser_findings = findings(&parsed);
        assert_eq!(parser_findings.len(), 1);
        assert_eq!(parser_findings[0].rule_id, "parser.html5");
        assert!(
            parser_findings[0]
                .message
                .contains("without seeing a doctype")
        );
        assert_eq!(parser_findings[0].location, None);
    }

    #[test]
    fn legacy_doctype_with_no_quirks_list_match_is_obsolete() {
        // html/parser/legacy-doctype-novalid.html and
        // quirky-doctype-novalid.html: neither matches the
        // limited-quirks list, both get "obsolete doctype".
        let parsed = parse(
            r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">"#,
        );
        let parser_findings = findings(&parsed);
        assert_eq!(parser_findings.len(), 1);
        assert!(parser_findings[0].message.contains("obsolete doctype"));
        assert!(parser_findings[0].location.is_some());
    }

    #[test]
    fn quirky_doctype_with_no_system_id_is_also_obsolete() {
        let parsed = parse(r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.0//EN">"#);
        let parser_findings = findings(&parsed);
        assert_eq!(parser_findings.len(), 1);
        assert!(parser_findings[0].message.contains("obsolete doctype"));
    }

    #[test]
    fn html_4_01_transitional_with_system_id_is_almost_standards() {
        // html/parser/almost-standards-doctype-novalid.html
        let parsed = parse(
            r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" "http://www.w3.org/TR/html4/loose.dtd">"#,
        );
        let parser_findings = findings(&parsed);
        assert_eq!(parser_findings.len(), 1);
        assert!(
            parser_findings[0]
                .message
                .contains("almost standards mode doctype")
        );
    }

    #[test]
    fn about_legacy_compat_system_id_is_not_a_finding() {
        // §13.2.6.4.1's explicit allowance for the "iframe srcdoc"
        // legacy-compat sentinel system identifier.
        let parsed = parse(r#"<!DOCTYPE html SYSTEM "about:legacy-compat">"#);
        assert!(findings(&parsed).is_empty());
    }
}
