//! The CSP3 directive registry (§6 and subsections): maps a known
//! directive name to its value grammar, and interprets a [`Directive`]'s
//! raw value accordingly -- turning "name + raw value" (Phase 02) plus
//! the source-list parser (Phase 03) into a fully structured directive,
//! as sketched by the architecture diagram in `CLAUDE.md`.
//!
//! **Registry completeness is a snapshot, not a permanent guarantee** —
//! see `plan/04-directive-registry.md`'s risks section and the README's
//! status/scope notes (Phase 06). `webrtc` is deliberately left
//! unregistered pending verification against the then-current spec text
//! (its standardization status was in flux); `referrer` (CSP1, removed
//! before CSP3) is intentionally out of scope entirely -- this crate's
//! normative basis is CSP3 only (`CLAUDE.md`).

use crate::ast::Directive;
use crate::source_list::{Keyword, SourceExpression, SourceList, parse_source_list};

/// Which value grammar a directive's raw value should be parsed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueGrammar {
    /// `serialized-source-list` (CSP3 §2.3.1).
    SourceList,
    /// `frame-ancestors`' restricted `ancestor-source-list`: parsed with
    /// the same [`crate::parse_source_list`] as `SourceList`, but only
    /// scheme-source, host-source, and `'self'` are valid there -- see
    /// [`ancestor_source_list_is_valid`].
    AncestorSourceList,
    /// `sandbox`'s whitespace-separated, unquoted token list.
    SandboxTokens,
    /// No value expected (e.g. `upgrade-insecure-requests`).
    Boolean,
    /// A single raw token (e.g. `report-to`, `require-trusted-types-for`).
    Token,
    /// Whitespace-separated raw tokens with no further structure parsed
    /// by this crate (e.g. `report-uri`'s URI references, `plugin-types`'
    /// MIME-type patterns -- both grammars this crate deliberately does
    /// not implement, matching the narrow, generic scope from
    /// `CLAUDE.md`).
    TokenList,
    /// `trusted-types`' policy-name list (plus the `'allow-duplicates'`/
    /// `'none'` sentinel tokens), kept as raw tokens for the same reason
    /// as `TokenList`.
    TrustedTypes,
}

/// Whether a registered directive is CSP3-current or deprecated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DirectiveStatus {
    /// Part of the current CSP3 directive set.
    Current,
    /// Kept for backward compatibility, superseded by another directive
    /// (e.g. `report-uri` by `report-to`) or removed from later spec
    /// drafts, but still commonly seen in the wild.
    Deprecated,
}

const REGISTRY: &[(&str, ValueGrammar, DirectiveStatus)] = &[
    // Fetch directives (CSP3 §6.1).
    (
        "child-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "connect-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "default-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "font-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "frame-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "img-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "manifest-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "media-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "object-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "script-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "script-src-elem",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "script-src-attr",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "style-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "style-src-elem",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "style-src-attr",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "worker-src",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    // Document directives (CSP3 §6.3).
    (
        "base-uri",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "sandbox",
        ValueGrammar::SandboxTokens,
        DirectiveStatus::Current,
    ),
    // Navigation directives (CSP3 §6.4).
    (
        "form-action",
        ValueGrammar::SourceList,
        DirectiveStatus::Current,
    ),
    (
        "frame-ancestors",
        ValueGrammar::AncestorSourceList,
        DirectiveStatus::Current,
    ),
    // Reporting directives (CSP3 §6.5).
    ("report-to", ValueGrammar::Token, DirectiveStatus::Current),
    (
        "report-uri",
        ValueGrammar::TokenList,
        DirectiveStatus::Deprecated,
    ),
    // Boolean/valueless directives.
    (
        "upgrade-insecure-requests",
        ValueGrammar::Boolean,
        DirectiveStatus::Current,
    ),
    (
        "block-all-mixed-content",
        ValueGrammar::Boolean,
        DirectiveStatus::Deprecated,
    ),
    // Trusted Types directives.
    (
        "require-trusted-types-for",
        ValueGrammar::Token,
        DirectiveStatus::Current,
    ),
    (
        "trusted-types",
        ValueGrammar::TrustedTypes,
        DirectiveStatus::Current,
    ),
    // Deprecated, CSP2-era.
    (
        "plugin-types",
        ValueGrammar::TokenList,
        DirectiveStatus::Deprecated,
    ),
];

/// Looks up a directive name in the CSP3 directive registry.
/// ASCII-case-insensitive, per CSP3's directive-name matching rule (see
/// `plan/02-directive-splitting.md`). Returns `None` for unregistered
/// names -- per CSP3's forward-compatibility design, that is not itself
/// a syntax error (see `plan/DECISIONS.md`, 2026-08-22).
pub fn registry_lookup(name: &str) -> Option<(ValueGrammar, DirectiveStatus)> {
    REGISTRY
        .iter()
        .find(|(registered_name, _, _)| registered_name.eq_ignore_ascii_case(name))
        .map(|(_, grammar, status)| (*grammar, *status))
}

/// A directive's value, interpreted according to its registered
/// [`ValueGrammar`] (or [`DirectiveValue::Unknown`] if the directive
/// name isn't in the registry).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DirectiveValue {
    /// See [`ValueGrammar::SourceList`].
    SourceList(SourceList),
    /// See [`ValueGrammar::AncestorSourceList`].
    AncestorSourceList(SourceList),
    /// See [`ValueGrammar::SandboxTokens`].
    Sandbox(Vec<String>),
    /// See [`ValueGrammar::Boolean`].
    Boolean,
    /// See [`ValueGrammar::Token`].
    Token(Option<String>),
    /// See [`ValueGrammar::TokenList`].
    TokenList(Vec<String>),
    /// See [`ValueGrammar::TrustedTypes`].
    TrustedTypes(Vec<String>),
    /// The directive name isn't in the registry (see this module's docs
    /// for what that does and doesn't imply).
    Unknown,
}

impl Directive {
    /// Interprets [`Directive::raw_value`] via the CSP3 directive
    /// registry for [`Directive::name`]. Computed on demand rather than
    /// stored on `Directive` itself, so [`crate::parse_policy_list`]
    /// (Phase 02) stays a pure, registry-independent split -- see
    /// `plan/DECISIONS.md`, 2026-08-22.
    pub fn value(&self) -> DirectiveValue {
        let Some((grammar, _status)) = registry_lookup(&self.name) else {
            return DirectiveValue::Unknown;
        };
        let raw = self.raw_value.as_deref().unwrap_or("");
        match grammar {
            ValueGrammar::SourceList => DirectiveValue::SourceList(parse_source_list(raw)),
            ValueGrammar::AncestorSourceList => {
                DirectiveValue::AncestorSourceList(parse_source_list(raw))
            }
            ValueGrammar::SandboxTokens => DirectiveValue::Sandbox(tokenize(raw)),
            ValueGrammar::Boolean => DirectiveValue::Boolean,
            ValueGrammar::Token => {
                DirectiveValue::Token(raw.split_ascii_whitespace().next().map(str::to_string))
            }
            ValueGrammar::TokenList => DirectiveValue::TokenList(tokenize(raw)),
            ValueGrammar::TrustedTypes => DirectiveValue::TrustedTypes(tokenize(raw)),
        }
    }

    /// For a [`ValueGrammar::Boolean`] directive (e.g.
    /// `upgrade-insecure-requests`): whether it unexpectedly carries a
    /// value. CSP3 defines these directives as valueless -- a present
    /// value is a caller-visible anomaly (a diagnostic), not silently
    /// dropped or a panic. `false` for non-boolean or unregistered
    /// directives.
    pub fn boolean_value_is_unexpected(&self) -> bool {
        matches!(
            registry_lookup(&self.name),
            Some((ValueGrammar::Boolean, _))
        ) && self.raw_value.is_some()
    }
}

fn tokenize(raw: &str) -> Vec<String> {
    raw.split_ascii_whitespace().map(str::to_string).collect()
}

/// Whether `list` only contains expressions valid in `frame-ancestors`'
/// restricted `ancestor-source-list`: scheme-source, host-source, and
/// `'self'` -- unlike a regular `source-list`, no `'unsafe-inline'` (or
/// any other keyword), no nonce-source, no hash-source. Unrecognized
/// entries (`expression: None`) also make the list invalid.
pub fn ancestor_source_list_is_valid(list: &SourceList) -> bool {
    match list {
        SourceList::None => true,
        SourceList::Sources(entries) => entries.iter().all(|entry| {
            matches!(
                entry.expression,
                Some(SourceExpression::Scheme(_))
                    | Some(SourceExpression::Host(_))
                    | Some(SourceExpression::Keyword(Keyword::SelfKeyword))
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_policy_list;

    fn directive_value(policy_str: &str) -> DirectiveValue {
        parse_policy_list(policy_str).policies[0].directives[0].value()
    }

    #[test]
    fn fetch_directive_is_source_list() {
        assert!(matches!(
            directive_value("default-src 'self'"),
            DirectiveValue::SourceList(_)
        ));
    }

    #[test]
    fn sandbox_tokens() {
        assert_eq!(
            directive_value("sandbox allow-scripts allow-forms"),
            DirectiveValue::Sandbox(vec!["allow-scripts".to_string(), "allow-forms".to_string()])
        );
    }

    #[test]
    fn base_uri_and_form_action_are_source_list() {
        assert!(matches!(
            directive_value("base-uri 'self'"),
            DirectiveValue::SourceList(_)
        ));
        assert!(matches!(
            directive_value("form-action 'self'"),
            DirectiveValue::SourceList(_)
        ));
    }

    #[test]
    fn frame_ancestors_accepts_self_and_hosts_but_rejects_unsafe_inline_nonce_hash() {
        let allowed = directive_value("frame-ancestors 'self' example.com https:");
        match allowed {
            DirectiveValue::AncestorSourceList(list) => {
                assert!(ancestor_source_list_is_valid(&list));
            }
            other => panic!("expected AncestorSourceList, got {other:?}"),
        }

        for rejected_raw in [
            "frame-ancestors 'unsafe-inline'",
            "frame-ancestors 'nonce-abc123'",
            "frame-ancestors 'sha256-abc123'",
        ] {
            match directive_value(rejected_raw) {
                DirectiveValue::AncestorSourceList(list) => {
                    assert!(!ancestor_source_list_is_valid(&list), "{rejected_raw}");
                }
                other => panic!("expected AncestorSourceList, got {other:?}"),
            }
        }
    }

    #[test]
    fn report_to_and_report_uri() {
        assert_eq!(
            directive_value("report-to endpoint-1"),
            DirectiveValue::Token(Some("endpoint-1".to_string()))
        );
        assert_eq!(
            directive_value("report-uri https://example.com/csp-report"),
            DirectiveValue::TokenList(vec!["https://example.com/csp-report".to_string()])
        );
        assert_eq!(
            registry_lookup("report-uri").map(|(_, status)| status),
            Some(DirectiveStatus::Deprecated)
        );
    }

    #[test]
    fn boolean_directives() {
        let list = parse_policy_list("upgrade-insecure-requests");
        let directive = &list.policies[0].directives[0];
        assert_eq!(directive.value(), DirectiveValue::Boolean);
        assert!(!directive.boolean_value_is_unexpected());

        let list = parse_policy_list("upgrade-insecure-requests 'self'");
        let directive = &list.policies[0].directives[0];
        assert_eq!(directive.value(), DirectiveValue::Boolean);
        assert!(directive.boolean_value_is_unexpected());

        assert_eq!(
            registry_lookup("block-all-mixed-content").map(|(_, status)| status),
            Some(DirectiveStatus::Deprecated)
        );
    }

    #[test]
    fn trusted_types_directives() {
        assert_eq!(
            directive_value("require-trusted-types-for 'script'"),
            DirectiveValue::Token(Some("'script'".to_string()))
        );
        assert_eq!(
            directive_value("trusted-types my-policy 'allow-duplicates'"),
            DirectiveValue::TrustedTypes(vec![
                "my-policy".to_string(),
                "'allow-duplicates'".to_string(),
            ])
        );
    }

    #[test]
    fn plugin_types_is_deprecated_token_list() {
        assert_eq!(
            directive_value("plugin-types application/pdf"),
            DirectiveValue::TokenList(vec!["application/pdf".to_string()])
        );
        assert_eq!(
            registry_lookup("plugin-types").map(|(_, status)| status),
            Some(DirectiveStatus::Deprecated)
        );
    }

    #[test]
    fn unknown_directive_stays_syntactically_valid_but_unstructured() {
        let list = parse_policy_list("default-src 'self'; totally-unknown-directive foo");
        assert_eq!(list.policies[0].directives.len(), 2);
        assert_eq!(
            list.policies[0].directives[1].value(),
            DirectiveValue::Unknown
        );
    }

    #[test]
    fn registry_lookup_is_case_insensitive() {
        assert!(registry_lookup("Default-Src").is_some());
        assert!(registry_lookup("DEFAULT-SRC").is_some());
    }
}
