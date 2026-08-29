//! Detects inline script/style content that a `<meta http-equiv="Content-
//! Security-Policy">` declared elsewhere in the same document would
//! block (Phase 08's `html/warnings/csp-*` cluster, 4 fixtures).
//!
//! Same category of thing as `src/scripts.rs`: a genuine cross-element
//! check (does *this* element's content violate a policy declared on
//! *another* element) that needs a real CSP directive-list parser
//! (`csp-parse`, already a dependency for `w:content-security-policy`'s
//! own syntax validation in `src/datatypes/csp.rs`) — not expressible in
//! `rules/*.sch` (XPath 1.0 has no CSP source-list grammar), and not a
//! `w:*` datatype either (this isn't validating one attribute's value
//! grammar, it's checking document-wide enforcement). Walks the raw
//! `html5_parser::Document`, same as `scripts.rs` and `parse.rs`'s
//! `doctype_findings`.
//!
//! **Deliberately narrow, evidence-scoped model**, not a general CSP
//! enforcement engine:
//! - Only `<meta http-equiv>` CSP (no HTTP header — this checker only
//!   ever sees a single HTML document, no response headers).
//! - Only the `'unsafe-inline'` keyword is treated as "allows inline" —
//!   a real browser also allows a matching `nonce-*`/hash source, but no
//!   corpus fixture exercises that combination for this check, and
//!   guessing at nonce/hash matching semantics here would be exactly the
//!   kind of extrapolation `rules/README.md` warns against.
//! - `script-src`/`style-src`, each falling back to `default-src` if
//!   absent (CSP3 §6.1's fetch-directive fallback list) — the only two
//!   fallback chains any corpus fixture exercises.
//! - Multiple `<meta>` CSP declarations are enforced cumulatively (like
//!   multiple HTTP header policies would be): if *any* declared policy's
//!   effective source list lacks `'unsafe-inline'`, the corresponding
//!   inline content is blocked.

use csp_parse::{Keyword, SourceExpression, SourceList, parse_policy_list, parse_source_list};
use html5_parser::{Document, NodeId, NodeKind};

use crate::{Finding, Severity, SourceLocation};

const RULE_ID: &str = "csp.meta-enforcement";

/// Collects every `Content-Security-Policy` `<meta http-equiv>` value in
/// the document, then flags inline scripts/styles/event handlers/style
/// attributes that a resulting policy would block.
pub(crate) fn findings(document: &Document) -> Vec<Finding> {
    let policy_contents = collect_meta_csp_contents(document, document.root());
    if policy_contents.is_empty() {
        return Vec::new();
    }

    let script_blocked = policy_contents
        .iter()
        .any(|content| directive_blocks_inline(content, "script-src"));
    let style_blocked = policy_contents
        .iter()
        .any(|content| directive_blocks_inline(content, "style-src"));

    if !script_blocked && !style_blocked {
        return Vec::new();
    }

    let mut findings = Vec::new();
    collect_violations(
        document,
        document.root(),
        script_blocked,
        style_blocked,
        &mut findings,
    );
    findings
}

fn collect_meta_csp_contents(document: &Document, id: NodeId) -> Vec<String> {
    let mut contents = Vec::new();
    collect_meta_csp_contents_into(document, id, &mut contents);
    contents
}

fn collect_meta_csp_contents_into(document: &Document, id: NodeId, out: &mut Vec<String>) {
    let node = document.node(id);
    if let NodeKind::Element {
        name, attributes, ..
    } = &node.kind
        && name.eq_ignore_ascii_case("meta")
    {
        let is_csp = attributes.iter().any(|attribute| {
            attribute.name.eq_ignore_ascii_case("http-equiv")
                && attribute
                    .value
                    .eq_ignore_ascii_case("content-security-policy")
        });
        if is_csp
            && let Some(content) = attributes
                .iter()
                .find(|attribute| attribute.name.eq_ignore_ascii_case("content"))
        {
            out.push(content.value.clone());
        }
    }
    for child in document.children(id) {
        collect_meta_csp_contents_into(document, child, out);
    }
}

/// `true` if a `Content-Security-Policy` `content` value's effective
/// source list for `fetch_directive` (`script-src`/`style-src`, falling
/// back to `default-src`) does *not* contain `'unsafe-inline'` — i.e.
/// inline content governed by that directive is blocked.
fn directive_blocks_inline(policy_content: &str, fetch_directive: &str) -> bool {
    let policy_list = parse_policy_list(policy_content);
    policy_list.policies.iter().any(|policy| {
        let directive = policy
            .directives
            .iter()
            .find(|directive| directive.name.eq_ignore_ascii_case(fetch_directive))
            .or_else(|| {
                policy
                    .directives
                    .iter()
                    .find(|directive| directive.name.eq_ignore_ascii_case("default-src"))
            });
        let Some(directive) = directive else {
            // Neither the specific directive nor default-src is present:
            // nothing restricts this fetch directive, so nothing is
            // blocked by *this* policy.
            return false;
        };
        let Some(raw_value) = &directive.raw_value else {
            return true;
        };
        let source_list = parse_source_list(raw_value);
        !source_list_allows_unsafe_inline(&source_list)
    })
}

fn source_list_allows_unsafe_inline(source_list: &SourceList) -> bool {
    match source_list {
        SourceList::None => false,
        SourceList::Sources(entries) => entries.iter().any(|entry| {
            matches!(
                entry.expression,
                Some(SourceExpression::Keyword(Keyword::UnsafeInline))
            )
        }),
        _ => false,
    }
}

fn collect_violations(
    document: &Document,
    id: NodeId,
    script_blocked: bool,
    style_blocked: bool,
    findings: &mut Vec<Finding>,
) {
    let node = document.node(id);
    if let NodeKind::Element {
        name, attributes, ..
    } = &node.kind
    {
        let location = node.position.map(|position| SourceLocation {
            line: position.line,
            column: position.column,
            byte_offset: position.byte_offset,
        });

        if script_blocked {
            if name.eq_ignore_ascii_case("script")
                && !attributes
                    .iter()
                    .any(|attribute| attribute.name.eq_ignore_ascii_case("src"))
            {
                findings.push(finding(
                    "Inline script violates Content Security Policy (meta tag): blocked by \
                     \"script-src\" directive (missing \"'unsafe-inline'\" or nonce/hash)."
                        .to_owned(),
                    location,
                ));
            }
            for attribute in attributes {
                if attribute.name.len() > 2
                    && attribute.name.as_bytes()[0..2].eq_ignore_ascii_case(b"on")
                {
                    findings.push(finding(
                        format!(
                            "Event handler attribute \"{}\" violates Content Security Policy \
                             (meta tag): blocked by \"script-src\" directive.",
                            attribute.name
                        ),
                        location,
                    ));
                }
            }
        }

        if style_blocked {
            if name.eq_ignore_ascii_case("style") {
                findings.push(finding(
                    "Inline style violates Content Security Policy (meta tag): blocked by \
                     \"style-src\" directive (missing \"'unsafe-inline'\" or nonce/hash)."
                        .to_owned(),
                    location,
                ));
            }
            if attributes
                .iter()
                .any(|attribute| attribute.name.eq_ignore_ascii_case("style"))
            {
                findings.push(finding(
                    "The \"style\" attribute violates Content Security Policy (meta tag): \
                     blocked by \"style-src\" directive."
                        .to_owned(),
                    location,
                ));
            }
        }
    }
    for child in document.children(id) {
        collect_violations(document, child, script_blocked, style_blocked, findings);
    }
}

fn finding(message: String, location: Option<SourceLocation>) -> Finding {
    Finding {
        rule_id: RULE_ID.to_owned(),
        severity: Severity::Warning,
        message,
        location,
    }
}

#[cfg(test)]
mod tests {
    fn csp_findings(html: &str) -> Vec<crate::Finding> {
        crate::check(html)
            .expect("HTML5 parsing should recover")
            .findings
            .into_iter()
            .filter(|finding| finding.rule_id == super::RULE_ID)
            .collect()
    }

    #[test]
    fn no_meta_csp_is_clean() {
        assert!(
            csp_findings("<!doctype html><title>t</title><script>alert(1)</script>").is_empty()
        );
    }

    #[test]
    fn unsafe_inline_script_src_allows_inline_script() {
        assert!(
            csp_findings(
                "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'unsafe-inline'\">\
             <script>alert(1)</script>"
            )
            .is_empty()
        );
    }

    #[test]
    fn inline_script_without_unsafe_inline_is_flagged() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'\">\
             <script>alert(1)</script>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn event_handler_attribute_is_flagged() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'\">\
             <body><button onclick=\"go()\">x</button></body>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn inline_style_element_is_flagged() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'self'\">\
             <style>body{color:red}</style>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn style_attribute_is_flagged() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"style-src 'self'\">\
             <body><p style=\"color:red\">x</p></body>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn default_src_fallback_blocks_inline_script() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'\">\
             <script>alert(1)</script>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn script_src_present_shadows_default_src_fallback() {
        // default-src forbids unsafe-inline, but the more specific
        // script-src explicitly allows it — script-src wins.
        assert!(
            csp_findings(
                "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" \
             content=\"default-src 'self'; script-src 'unsafe-inline'\">\
             <script>alert(1)</script>"
            )
            .is_empty()
        );
    }

    #[test]
    fn external_script_with_src_is_not_flagged() {
        assert!(
            csp_findings(
                "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'\">\
             <script src=\"a.js\"></script>"
            )
            .is_empty()
        );
    }
}
