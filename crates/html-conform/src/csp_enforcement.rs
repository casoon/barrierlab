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
//! - Inline `<script>`/`<style>` follow CSP3 "Does a source list allow all
//!   inline behavior" and the nonce/hash checks: a `'nonce-…'` source
//!   equal to the element's `nonce` attribute allows it; `'unsafe-inline'`
//!   is ignored as soon as the list carries a nonce or hash source (and,
//!   for scripts, `'strict-dynamic'`).
//! - Hash sources are **not evaluated**: no hashing dependency is
//!   justified for this check. An inline element that a hash source might
//!   allow is reported as `Info` ("not evaluated") instead of a warning.
//! - Fallback chains: `script-src-elem`/`script-src-attr` →
//!   `script-src` → `default-src`, and the `style-src-*` analogue.
//! - Multiple `<meta>` CSP declarations are enforced cumulatively (like
//!   multiple HTTP header policies would be): if *any* declared policy
//!   blocks the inline content, it is blocked.

use csp_parse::{
    Keyword, PolicyList, SourceExpression, SourceList, parse_policy_list, parse_source_list,
};
use html5_parser::{Document, NodeId, NodeKind};

use crate::{Finding, Severity, SourceLocation};

const RULE_ID: &str = "csp.meta-enforcement";

/// Collects every `Content-Security-Policy` `<meta http-equiv>` value in
/// the document, then flags inline scripts/styles/event handlers/style
/// attributes that a resulting policy would block.
pub(crate) fn findings(document: &Document) -> Vec<Finding> {
    let policies: Vec<PolicyList> = collect_meta_csp_contents(document, document.root())
        .iter()
        .map(|content| parse_policy_list(content))
        .collect();
    if policies.is_empty() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    collect_violations(document, document.root(), &policies, &mut findings);
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Allowed,
    /// Only a hash source could allow it; hashes are not evaluated.
    HashUnverified,
    Blocked,
}

/// Which inline construct is checked: the fallback chain of fetch
/// directives, and whether a `'strict-dynamic'` source disables
/// `'unsafe-inline'` (scripts only).
#[derive(Clone, Copy)]
struct Kind {
    chain: [&'static str; 3],
    strict_dynamic_counts: bool,
}

const SCRIPT_ELEM: Kind = Kind {
    chain: ["script-src-elem", "script-src", "default-src"],
    strict_dynamic_counts: true,
};
const SCRIPT_ATTR: Kind = Kind {
    chain: ["script-src-attr", "script-src", "default-src"],
    strict_dynamic_counts: true,
};
const STYLE_ELEM: Kind = Kind {
    chain: ["style-src-elem", "style-src", "default-src"],
    strict_dynamic_counts: false,
};
const STYLE_ATTR: Kind = Kind {
    chain: ["style-src-attr", "style-src", "default-src"],
    strict_dynamic_counts: false,
};

/// Combines the verdicts of all declared policies for one inline
/// construct. `nonce` is the element's `nonce` attribute, if any.
fn verdict(policies: &[PolicyList], kind: Kind, nonce: Option<&str>) -> Verdict {
    let mut result = Verdict::Allowed;
    for policy in policies.iter().flat_map(|list| &list.policies) {
        let directive = kind.chain.iter().find_map(|name| {
            policy
                .directives
                .iter()
                .find(|directive| directive.name.eq_ignore_ascii_case(name))
        });
        // Neither the specific directive nor default-src is present:
        // nothing restricts this fetch directive in *this* policy.
        let Some(directive) = directive else {
            continue;
        };
        let Some(raw_value) = &directive.raw_value else {
            return Verdict::Blocked;
        };
        match source_list_verdict(&parse_source_list(raw_value), kind, nonce) {
            Verdict::Blocked => return Verdict::Blocked,
            Verdict::HashUnverified => result = Verdict::HashUnverified,
            Verdict::Allowed => {}
        }
    }
    result
}

fn source_list_verdict(source_list: &SourceList, kind: Kind, nonce: Option<&str>) -> Verdict {
    let SourceList::Sources(entries) = source_list else {
        return Verdict::Blocked;
    };
    let mut has_nonce_or_hash = false;
    let mut has_hash = false;
    let mut unsafe_inline = false;
    let mut strict_dynamic = false;
    for expression in entries.iter().filter_map(|entry| entry.expression.as_ref()) {
        match expression {
            SourceExpression::Nonce(value) => {
                if nonce == Some(value.as_str()) {
                    return Verdict::Allowed;
                }
                has_nonce_or_hash = true;
            }
            SourceExpression::Hash(_) => {
                has_nonce_or_hash = true;
                has_hash = true;
            }
            SourceExpression::Keyword(Keyword::UnsafeInline) => unsafe_inline = true,
            SourceExpression::Keyword(Keyword::StrictDynamic) => strict_dynamic = true,
            _ => {}
        }
    }
    // CSP3: 'unsafe-inline' is ignored when the list has a nonce or hash
    // source (or 'strict-dynamic', for scripts).
    let ignored = has_nonce_or_hash || (kind.strict_dynamic_counts && strict_dynamic);
    if unsafe_inline && !ignored {
        Verdict::Allowed
    } else if has_hash {
        Verdict::HashUnverified
    } else {
        Verdict::Blocked
    }
}

fn collect_violations(
    document: &Document,
    id: NodeId,
    policies: &[PolicyList],
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
        let nonce = attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case("nonce"))
            .map(|attribute| attribute.value.as_str());

        if name.eq_ignore_ascii_case("script")
            && !attributes
                .iter()
                .any(|attribute| attribute.name.eq_ignore_ascii_case("src"))
        {
            report(
                verdict(policies, SCRIPT_ELEM, nonce),
                "Inline script violates Content Security Policy (meta tag): blocked by \
                 \"script-src\" directive (missing \"'unsafe-inline'\" or nonce/hash)."
                    .to_owned(),
                location,
                findings,
            );
        }
        // Attributes can only be allowed by 'unsafe-inline' (nonces do not
        // apply; 'unsafe-hashes' is not evaluated).
        for attribute in attributes {
            if attribute.name.len() > 2
                && attribute.name.as_bytes()[0..2].eq_ignore_ascii_case(b"on")
            {
                report(
                    verdict(policies, SCRIPT_ATTR, None),
                    format!(
                        "Event handler attribute \"{}\" violates Content Security Policy \
                         (meta tag): blocked by \"script-src\" directive.",
                        attribute.name
                    ),
                    location,
                    findings,
                );
            }
        }

        if name.eq_ignore_ascii_case("style") {
            report(
                verdict(policies, STYLE_ELEM, nonce),
                "Inline style violates Content Security Policy (meta tag): blocked by \
                 \"style-src\" directive (missing \"'unsafe-inline'\" or nonce/hash)."
                    .to_owned(),
                location,
                findings,
            );
        }
        if attributes
            .iter()
            .any(|attribute| attribute.name.eq_ignore_ascii_case("style"))
        {
            report(
                verdict(policies, STYLE_ATTR, None),
                "The \"style\" attribute violates Content Security Policy (meta tag): \
                 blocked by \"style-src\" directive."
                    .to_owned(),
                location,
                findings,
            );
        }
    }
    for child in document.children(id) {
        collect_violations(document, child, policies, findings);
    }
}

fn report(
    verdict: Verdict,
    message: String,
    location: Option<SourceLocation>,
    findings: &mut Vec<Finding>,
) {
    match verdict {
        Verdict::Allowed => {}
        Verdict::Blocked => findings.push(finding(message, Severity::Warning, location)),
        Verdict::HashUnverified => findings.push(finding(
            "Inline content is governed by a Content Security Policy (meta tag) with hash \
             sources; hash matching is not evaluated, so whether it is allowed is unknown."
                .to_owned(),
            Severity::Info,
            location,
        )),
    }
}

fn finding(message: String, severity: Severity, location: Option<SourceLocation>) -> Finding {
    Finding {
        rule_id: RULE_ID.to_owned(),
        severity,
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

    const NONCE_META: &str = "<!doctype html><title>t</title>\
         <meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'nonce-abc123'\">";

    #[test]
    fn matching_nonce_allows_inline_script() {
        assert!(
            csp_findings(&format!(
                "{NONCE_META}<script nonce=\"abc123\">alert(1)</script>"
            ))
            .is_empty()
        );
    }

    #[test]
    fn mismatching_nonce_is_flagged() {
        let findings = csp_findings(&format!(
            "{NONCE_META}<script nonce=\"other\">alert(1)</script>"
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, crate::Severity::Warning);
    }

    #[test]
    fn missing_nonce_is_flagged() {
        assert_eq!(
            csp_findings(&format!("{NONCE_META}<script>alert(1)</script>")).len(),
            1
        );
    }

    #[test]
    fn unsafe_inline_is_ignored_when_nonce_present() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" \
             content=\"script-src 'unsafe-inline' 'nonce-abc123'\">\
             <script>alert(1)</script>",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn matching_nonce_allows_inline_style_via_default_src() {
        assert!(
            csp_findings(
                "<!doctype html><title>t</title>\
                 <meta http-equiv=\"Content-Security-Policy\" \
                 content=\"default-src 'nonce-abc123'\">\
                 <style nonce=\"abc123\">body{color:red}</style>"
            )
            .is_empty()
        );
    }

    #[test]
    fn hash_source_downgrades_to_info() {
        let findings = csp_findings(
            "<!doctype html><title>t</title>\
             <meta http-equiv=\"Content-Security-Policy\" \
             content=\"script-src 'sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU='\">\
             <script>alert(1)</script>",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, crate::Severity::Info);
    }
}
