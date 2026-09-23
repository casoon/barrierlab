//! Phase 05 — spec-example and edge-case sweep (see
//! `plan/05-conformance-sweep.md`).
//!
//! No vendored testsuite: web-platform-tests' `content-security-policy/
//! parsing/` directory was checked before writing this file and confirmed
//! to test *enforcement* behavior (real image loads / violation events in
//! a browser), not pure syntax parsing exposed via an API -- not
//! vendorable here (see `plan/DECISIONS.md`, 2026-08-22). Instead: a
//! hand-picked set of spec-style example policies plus non-normative
//! real-world-shaped robustness cases, exercising the full pipeline
//! (`parse_policy_list` -> `Directive::value()`) across module
//! boundaries that the per-phase unit tests (in `src/`) don't cross.

use csp_parse::{
    DirectiveStatus, DirectiveValue, HashAlgorithm, HostPart, Keyword, SourceExpression,
    SourceList, ancestor_source_list_is_valid, parse_policy_list, registry_lookup,
};

/// A typical multi-directive fetch policy (shape used throughout CSP
/// guides/examples): default-src + a wildcard + a multi-host allowlist +
/// a single trusted script host.
#[test]
fn spec_style_multi_directive_fetch_policy() {
    let list = parse_policy_list(
        "default-src 'self'; img-src *; \
         object-src media1.example.com media2.example.com *.cdn.example.com; \
         script-src trustedscripts.example.com",
    );
    let directives = &list.policies[0].directives;
    assert_eq!(
        directives
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>(),
        vec!["default-src", "img-src", "object-src", "script-src"]
    );

    match directives[0].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => {
            assert_eq!(
                entries[0].expression,
                Some(SourceExpression::Keyword(Keyword::SelfKeyword))
            );
        }
        other => panic!("unexpected default-src value: {other:?}"),
    }

    match directives[1].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => match &entries[0].expression {
            Some(SourceExpression::Host(host)) => {
                assert_eq!(host.scheme, None);
                assert_eq!(host.host, HostPart::AnyHost);
                assert_eq!(host.port, None);
                assert_eq!(host.path, None);
            }
            other => panic!("unexpected img-src entry: {other:?}"),
        },
        other => panic!("unexpected img-src value: {other:?}"),
    }

    match directives[2].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => {
            assert_eq!(entries.len(), 3);
            assert!(entries.iter().all(|e| e.expression.is_some()));
        }
        other => panic!("unexpected object-src value: {other:?}"),
    }
}

/// Nonce- and hash-based script-src, including base64 padding.
#[test]
fn nonce_and_hash_source_script_src() {
    let list = parse_policy_list(
        "script-src 'nonce-2726c7f26c' 'sha256-B2yPHKaXnvFWtRChIbabYmUBFZdVfKKXHbWtWidDVF8='",
    );
    match &list.policies[0].directives[0].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => {
            assert_eq!(
                entries[0].expression,
                Some(SourceExpression::Nonce("2726c7f26c".to_string()))
            );
            match &entries[1].expression {
                Some(SourceExpression::Hash(hash)) => {
                    assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
                    assert_eq!(hash.value, "B2yPHKaXnvFWtRChIbabYmUBFZdVfKKXHbWtWidDVF8=");
                }
                other => panic!("unexpected second entry: {other:?}"),
            }
        }
        other => panic!("unexpected script-src value: {other:?}"),
    }
}

/// Both reporting directives, plus confirmation that `report-uri` is
/// registered as deprecated (CSP3 keeps it for backward compatibility
/// alongside `report-to`).
#[test]
fn reporting_directives() {
    let list = parse_policy_list("default-src 'self'; report-uri /csp-report-endpoint");
    assert_eq!(
        list.policies[0].directives[1].value(),
        DirectiveValue::TokenList(vec!["/csp-report-endpoint".to_string()])
    );
    assert_eq!(
        registry_lookup("report-uri").map(|(_, status)| status),
        Some(DirectiveStatus::Deprecated)
    );

    let list = parse_policy_list("default-src 'self'; report-to csp-endpoint");
    assert_eq!(
        list.policies[0].directives[1].value(),
        DirectiveValue::Token(Some("csp-endpoint".to_string()))
    );
}

/// `frame-ancestors` with a realistic self+host allowlist -- must be
/// accepted by `ancestor_source_list_is_valid`.
#[test]
fn frame_ancestors_realistic_allowlist() {
    let list = parse_policy_list("frame-ancestors 'self' https://example.com");
    match list.policies[0].directives[0].value() {
        DirectiveValue::AncestorSourceList(source_list) => {
            assert!(ancestor_source_list_is_valid(&source_list));
        }
        other => panic!("unexpected frame-ancestors value: {other:?}"),
    }
}

/// `upgrade-insecure-requests` combined with a scheme-only `default-src`.
#[test]
fn upgrade_insecure_requests_with_scheme_source() {
    let list = parse_policy_list("upgrade-insecure-requests; default-src https:");
    assert_eq!(
        list.policies[0].directives[0].value(),
        DirectiveValue::Boolean
    );
    match list.policies[0].directives[1].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => {
            assert_eq!(
                entries[0].expression,
                Some(SourceExpression::Scheme("https".to_string()))
            );
        }
        other => panic!("unexpected default-src value: {other:?}"),
    }
}

/// Trusted Types, sandbox: less common but well-defined directives.
#[test]
fn trusted_types_and_sandbox() {
    let list = parse_policy_list("require-trusted-types-for 'script'; trusted-types my-policy");
    assert_eq!(
        list.policies[0].directives[0].value(),
        DirectiveValue::Token(Some("'script'".to_string()))
    );
    assert_eq!(
        list.policies[0].directives[1].value(),
        DirectiveValue::TrustedTypes(vec!["my-policy".to_string()])
    );

    let list = parse_policy_list("sandbox allow-forms allow-scripts");
    assert_eq!(
        list.policies[0].directives[0].value(),
        DirectiveValue::Sandbox(vec!["allow-forms".to_string(), "allow-scripts".to_string()])
    );
}

/// A comma-joined pair of policies, as produced when combining multiple
/// `Content-Security-Policy` header instances into one string -- each
/// half must parse independently.
#[test]
fn comma_joined_policies_parse_independently() {
    let list = parse_policy_list("default-src 'self' 'unsafe-inline', default-src 'none'");
    assert_eq!(list.policies.len(), 2);
    assert!(matches!(
        list.policies[0].directives[0].value(),
        DirectiveValue::SourceList(SourceList::Sources(_))
    ));
    assert_eq!(
        list.policies[1].directives[0].value(),
        DirectiveValue::SourceList(SourceList::None)
    );
}

/// Edge case not covered by the Phase 03 unit tests: a host-part with
/// the ABNF's optional trailing `.` (`host-part = ... *( "." 1*host-char
/// ) [ "." ]`), e.g. as in a fully-qualified domain name.
#[test]
fn host_with_trailing_dot() {
    let list = parse_policy_list("script-src example.com.");
    match &list.policies[0].directives[0].value() {
        DirectiveValue::SourceList(SourceList::Sources(entries)) => match &entries[0].expression {
            Some(SourceExpression::Host(host)) => {
                assert_eq!(
                    host.host,
                    HostPart::Named {
                        wildcard_prefix: false,
                        labels: vec!["example".to_string(), "com".to_string()],
                        trailing_dot: true,
                    }
                );
            }
            other => panic!("unexpected entry: {other:?}"),
        },
        other => panic!("unexpected script-src value: {other:?}"),
    }
}

/// Non-normative robustness case, shaped after real-world CSP headers
/// (not copied from any single site) combining a wide range of directive
/// categories in one policy, per `plan/05-conformance-sweep.md`'s
/// "real-world examples as additional robustness cases, not a normative
/// source" step.
#[test]
fn real_world_shaped_robustness_case() {
    let list = parse_policy_list(
        "default-src 'self'; \
         script-src 'self' 'strict-dynamic' 'nonce-abc123' https://cdn.example.com; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         connect-src 'self' https://api.example.com wss://ws.example.com; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         object-src 'none'; \
         upgrade-insecure-requests; \
         report-to csp-endpoint; \
         report-uri /csp-report-endpoint",
    );
    let directives = &list.policies[0].directives;
    assert_eq!(directives.len(), 11);
    for directive in directives {
        // Every directive name in this policy is registered -- none
        // should fall back to `Unknown`.
        assert_ne!(
            directive.value(),
            DirectiveValue::Unknown,
            "{} unexpectedly unregistered",
            directive.name
        );
    }
}

/// Matches the exact shape of web-platform-tests'
/// `content-security-policy/parsing/invalid-directive.html` (an
/// enforcement test, not vendored here -- see module docs): an unknown,
/// valueless directive (`aaa`) next to a real one must not affect the
/// real directive's structured value.
#[test]
fn unknown_valueless_directive_does_not_affect_neighboring_directive() {
    let list = parse_policy_list("img-src 'none'; aaa;");
    let directives = &list.policies[0].directives;
    assert_eq!(
        directives[0].value(),
        DirectiveValue::SourceList(SourceList::None)
    );
    assert_eq!(directives[1].name, "aaa");
    assert_eq!(directives[1].value(), DirectiveValue::Unknown);
}
