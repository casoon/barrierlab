//! `w:content-security-policy` (`ContentSecurityPolicy.java`). vnu does not
//! implement its own CSP grammar — it delegates entirely to a third-party
//! Java library, `htmlunit-csp` (`org.htmlunit.csp.Policy`,
//! <https://github.com/HtmlUnit/htmlunit-csp>). This module reimplements
//! that library's *parse-time syntax validation* rules against
//! [`csp_parse`]'s structured output, so it is a from-source-verified
//! reimplementation rather than a guess — see `plan/DECISIONS.md`, entry
//! "csp-parse veröffentlicht; `w:content-security-policy`s Accept/Reject-
//! Mapping ist eigene Recherche, nicht in dieser Sitzung entschieden"
//! (2026-08-22) for how this was researched, and the entry immediately
//! above it in the log for the implementation itself.
//!
//! ## vnu-parity model: why message text/severity bookkeeping is skipped
//!
//! Every diagnostic `htmlunit-csp` emits while parsing carries a severity
//! (Error/Warning/Info). vnu's `ContentSecurityPolicy.java` wrapper funnels
//! *all* of them into `newDatatypeException(message, WARN)`, where `WARN`
//! is a system property (`nu.validator.datatype.warn`) that defaults to
//! `false` — meaning **no severity is ever softened to a non-fatal
//! warning** in vnu's default configuration (the same `WARN`-gate pattern
//! already documented for `w:rel-value`/`w:sandbox-allow-list` in
//! `plan/05c-datatype-library.md`). Error, Warning, and Info diagnostics
//! therefore all make the checked string invalid by default. The one
//! documented exception — messages containing `"experimental directive"`
//! are discarded before counting — is a no-op against the actual
//! `htmlunit-csp` source as cloned 2026-08-22 (`grep -ri experimental`
//! over its `src/main/java` finds no such message), so it is not
//! implemented here.
//!
//! **Net rule: the policy is invalid iff `htmlunit-csp` would emit at
//! least one parse-time diagnostic of any severity.** This lets every
//! check below return `Err` on the first violation found — the exact
//! message text and severity bucket vnu would report are not
//! reconstructed, only the accept/reject outcome, consistent with how
//! every other `w:*` type in this module reports failures (a description,
//! not vnu's literal wording).
//!
//! ## Known, documented gap: `navigate-to` and `prefetch-src`
//!
//! `htmlunit-csp` also recognizes `navigate-to` (a `SourceExpressionDirective`
//! from an earlier CSP3 draft, later removed from the spec, but still
//! parsed) and `prefetch-src` (a deprecated fetch directive) as *known*
//! directive names with their own dedicated dedup-tracking — a single,
//! syntactically valid occurrence of either is accepted. `csp-parse`'s
//! directive registry deliberately excludes both (see its
//! `src/directive.rs` doc comment: `csp-parse`'s normative basis is CSP3
//! only). This module therefore treats them as unrecognized directives —
//! always invalid, even when well-formed and non-duplicated — which is
//! stricter than real `htmlunit-csp`. Accepted, bounded approximation, in
//! the same category as `w:language`'s missing IANA registry data or

use csp_parse::{
    Directive, DirectiveValue, HashAlgorithm, Policy, SourceExpression, SourceList, ValueGrammar,
    ancestor_source_list_is_valid, parse_policy_list, parse_source_list, registry_lookup,
};

/// `w:content-security-policy` → `ContentSecurityPolicy.java`. See this
/// module's doc comment for the vnu-parity model this implements.
pub(crate) fn check_content_security_policy(value: &str) -> Result<(), String> {
    // vnu's `ContentSecurityPolicy.java`: a blind substring removal, run
    // before ASCII-checking/parsing — it strips these two substrings
    // wherever they occur in the whole policy text (not scoped to
    // `sandbox` keyword position), then parses what's left. Replicated
    // faithfully, quirk included.
    let text = value
        .replace("allow-downloads", "")
        .replace("allow-presentation", "");

    if !text.is_ascii() {
        return Err("Content Security Policy must contain only ASCII characters".to_string());
    }

    for policy in &parse_policy_list(&text).policies {
        check_policy(policy)?;
    }
    Ok(())
}

fn check_policy(policy: &Policy) -> Result<(), String> {
    let mut seen: Vec<String> = Vec::new();
    for directive in &policy.directives {
        if !directive.name_is_valid() {
            return Err(format!("invalid CSP directive name {:?}", directive.name));
        }
        if !directive.value_is_valid() {
            return Err(format!(
                "invalid CSP directive value for {:?}",
                directive.name
            ));
        }

        let Some((grammar, _status)) = registry_lookup(&directive.name) else {
            // Includes `navigate-to`/`prefetch-src` — see this module's
            // doc comment ("Known, documented gap").
            return Err(format!("unrecognized CSP directive {:?}", directive.name));
        };

        let lower_name = directive.name.to_ascii_lowercase();
        if seen.contains(&lower_name) {
            return Err(format!("duplicate CSP directive {:?}", directive.name));
        }
        seen.push(lower_name.clone());

        check_directive(&lower_name, grammar, directive)?;
    }
    Ok(())
}

fn check_directive(
    lower_name: &str,
    grammar: ValueGrammar,
    directive: &Directive,
) -> Result<(), String> {
    // `ValueGrammar::Token`/`TokenList` are each shared by two directives
    // with genuinely different validation rules in real `htmlunit-csp`
    // (report-to vs. require-trusted-types-for; report-uri vs.
    // plugin-types) — dispatch on the directive name for those, not the
    // shared grammar tag.
    match lower_name {
        "report-to" => return check_report_to(directive),
        "require-trusted-types-for" => return check_require_trusted_types_for(directive),
        "report-uri" => return check_report_uri(directive),
        "plugin-types" => return check_plugin_types(directive),
        _ => {}
    }
    match grammar {
        ValueGrammar::SourceList => check_source_list(directive.raw_value.as_deref().unwrap_or("")),
        ValueGrammar::AncestorSourceList => check_ancestor_source_list(directive),
        ValueGrammar::SandboxTokens => check_sandbox(directive),
        ValueGrammar::Boolean => {
            if directive.boolean_value_is_unexpected() {
                Err(format!("{lower_name} directive does not take a value"))
            } else {
                Ok(())
            }
        }
        ValueGrammar::TrustedTypes => check_trusted_types(directive),
        _ => unreachable!(
            "report-to/report-uri/require-trusted-types-for/plugin-types are matched by name above; \
             every other registered directive uses SourceList/AncestorSourceList/SandboxTokens/Boolean/TrustedTypes"
        ),
    }
}

/// `SourceExpressionDirective` — shared by every fetch directive plus
/// `base-uri`/`form-action`.
fn check_source_list(raw: &str) -> Result<(), String> {
    match parse_source_list(raw) {
        SourceList::None => Ok(()),
        SourceList::Sources(entries) => {
            if entries.is_empty() {
                return Err(
                    "source-expression list must not be empty (use 'none' instead)".to_string(),
                );
            }
            let mut seen: Vec<&SourceExpression> = Vec::new();
            for entry in &entries {
                match &entry.expression {
                    // Covers unrecognized tokens *and* the removed
                    // `'unsafe-redirect'`/`'unsafe-hashed-attributes'`
                    // keywords and a `'none'` combined with other
                    // entries — none of those match any `Keyword`
                    // variant `csp-parse` recognizes, so they all
                    // surface as `expression: None` here.
                    None => return Err(format!("unrecognized source-expression {:?}", entry.raw)),
                    Some(expr) => {
                        if seen.contains(&expr) {
                            return Err(format!("duplicate source-expression {:?}", entry.raw));
                        }
                        check_hash_strictness(expr)?;
                        seen.push(expr);
                    }
                }
            }
            Ok(())
        }
        _ => unreachable!("csp_parse::SourceList only has None/Sources variants today"),
    }
}

/// `htmlunit-csp` additionally warns (invalid, per this module's
/// vnu-parity model) when a hash-source's Base64 value has the wrong
/// length for its algorithm, or contains base64url characters (`-`/`_`)
/// — checks beyond the base `hash-source` grammar `csp-parse` itself
/// already enforces.
fn check_hash_strictness(expr: &SourceExpression) -> Result<(), String> {
    let SourceExpression::Hash(hash) = expr else {
        return Ok(());
    };
    let expected_len = match hash.algorithm {
        HashAlgorithm::Sha256 => 44,
        HashAlgorithm::Sha384 => 64,
        HashAlgorithm::Sha512 => 88,
        _ => return Ok(()),
    };
    if hash.value.len() != expected_len {
        return Err(format!(
            "wrong length for {:?} hash value {:?}",
            hash.algorithm, hash.value
        ));
    }
    if hash.value.contains(['-', '_']) {
        return Err(format!(
            "hash value {:?} must not contain base64url characters",
            hash.value
        ));
    }
    Ok(())
}

/// `FrameAncestorsDirective` (`frame-ancestors`): same host/scheme/`'self'`
/// logic as a regular source-list, but no keywords/nonces/hashes allowed —
/// `csp_parse::ancestor_source_list_is_valid` already implements exactly
/// that restriction.
fn check_ancestor_source_list(directive: &Directive) -> Result<(), String> {
    let raw = directive.raw_value.as_deref().unwrap_or("");
    let list = parse_source_list(raw);
    if let SourceList::Sources(entries) = &list {
        if entries.is_empty() {
            return Err("ancestor-source list must not be empty (use 'none' instead)".to_string());
        }
        let mut seen: Vec<&SourceExpression> = Vec::new();
        for entry in entries {
            match &entry.expression {
                None => return Err(format!("unrecognized ancestor-source {:?}", entry.raw)),
                Some(expr) => {
                    if seen.contains(&expr) {
                        return Err(format!("duplicate ancestor-source {:?}", entry.raw));
                    }
                    seen.push(expr);
                }
            }
        }
    }
    if !ancestor_source_list_is_valid(&list) {
        return Err(
            "frame-ancestors only allows scheme-source, host-source, and 'self'".to_string(),
        );
    }
    Ok(())
}

/// `SandboxDirective`. 11 of `htmlunit-csp`'s 13 known keywords: the other
/// two (`allow-downloads`, `allow-presentation`) can never survive
/// [`check_content_security_policy`]'s preprocessing strip to reach here
/// as themselves, so they are omitted rather than dead-listed.
const SANDBOX_KEYWORDS: &[&str] = &[
    "allow-forms",
    "allow-modals",
    "allow-orientation-lock",
    "allow-pointer-lock",
    "allow-popups",
    "allow-popups-to-escape-sandbox",
    "allow-same-origin",
    "allow-scripts",
    "allow-storage-access-by-user-activation",
    "allow-top-navigation",
    "allow-top-navigation-by-user-activation",
];

fn check_sandbox(directive: &Directive) -> Result<(), String> {
    let DirectiveValue::Sandbox(tokens) = directive.value() else {
        unreachable!(
            "a directive registered as ValueGrammar::SandboxTokens always yields DirectiveValue::Sandbox"
        );
    };
    let mut seen: Vec<String> = Vec::new();
    for token in &tokens {
        let lower = token.to_ascii_lowercase();
        if !SANDBOX_KEYWORDS.contains(&lower.as_str()) {
            return Err(format!("unrecognized sandbox keyword {token:?}"));
        }
        if seen.contains(&lower) {
            return Err(format!("duplicate sandbox keyword {token:?}"));
        }
        seen.push(lower);
    }
    Ok(())
}

/// RFC 7230 `tchar` (§3.2.6), used by [`check_report_to`].
fn is_rfc7230_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// `report-to`: exactly one value, matching the RFC 7230 `token` grammar.
fn check_report_to(directive: &Directive) -> Result<(), String> {
    let tokens: Vec<&str> = directive
        .raw_value
        .as_deref()
        .unwrap_or("")
        .split_ascii_whitespace()
        .collect();
    match tokens.as_slice() {
        [] => Err("report-to directive requires a value".to_string()),
        [only] => {
            if only.bytes().all(is_rfc7230_tchar) {
                Ok(())
            } else {
                Err(format!("invalid report-to token {only:?}"))
            }
        }
        _ => Err("report-to directive accepts exactly one value".to_string()),
    }
}

/// `require-trusted-types-for`: the only defined keyword is `'script'`
/// (case-insensitive); duplicates and anything else are invalid.
fn check_require_trusted_types_for(directive: &Directive) -> Result<(), String> {
    let raw = directive.raw_value.as_deref().unwrap_or("");
    let tokens: Vec<&str> = raw.split_ascii_whitespace().collect();
    if tokens.is_empty() {
        return Err("require-trusted-types-for directive requires a value".to_string());
    }
    let mut seen_script = false;
    for token in &tokens {
        if token.eq_ignore_ascii_case("'script'") {
            if seen_script {
                return Err("duplicate keyword 'script'".to_string());
            }
            seen_script = true;
        } else {
            return Err(format!(
                "unrecognized require-trusted-types-for value {token:?}"
            ));
        }
    }
    Ok(())
}

/// `report-uri` (deprecated): at least one value required; duplicate URIs
/// are Info-severity in `htmlunit-csp` but still invalid under this
/// module's vnu-parity model (see the module doc comment). No syntax
/// validation of the URI values themselves — matches `ReportUriDirective`,
/// which accepts any non-whitespace token as-is.
fn check_report_uri(directive: &Directive) -> Result<(), String> {
    let DirectiveValue::TokenList(tokens) = directive.value() else {
        unreachable!(
            "report-uri is registered as ValueGrammar::TokenList, always yields DirectiveValue::TokenList"
        );
    };
    if tokens.is_empty() {
        return Err("report-uri directive requires at least one value".to_string());
    }
    let mut seen: Vec<&String> = Vec::new();
    for uri in &tokens {
        if seen.contains(&uri) {
            return Err(format!("duplicate report-uri value {uri:?}"));
        }
        seen.push(uri);
    }
    Ok(())
}

/// `type/subtype` per RFC 2045 §5.1, as used by `plugin-types`.
fn is_media_type_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '^'
                | '&'
                | '*'
                | '-'
                | '_'
                | '+'
                | '{'
                | '}'
                | '|'
                | '\''
                | '.'
                | '`'
                | '~'
        )
}

/// `plugin-types` (deprecated): each value must be a `type/subtype` media
/// type; duplicates and literal `*` in either part are invalid. Unlike
/// every other value grammar in this module, an *empty* value list is
/// explicitly allowed (`w3c/webappsec-csp#374`).
fn check_plugin_types(directive: &Directive) -> Result<(), String> {
    let DirectiveValue::TokenList(tokens) = directive.value() else {
        unreachable!(
            "plugin-types is registered as ValueGrammar::TokenList, always yields DirectiveValue::TokenList"
        );
    };
    let mut seen: Vec<String> = Vec::new();
    for token in &tokens {
        let Some((type_part, subtype_part)) = token.split_once('/') else {
            return Err(format!("expecting media-type but found {token:?}"));
        };
        if type_part.is_empty()
            || subtype_part.is_empty()
            || !type_part.chars().all(is_media_type_char)
            || !subtype_part.chars().all(is_media_type_char)
        {
            return Err(format!("expecting media-type but found {token:?}"));
        }
        if type_part == "*" || subtype_part == "*" {
            return Err(format!(
                "media type {token:?} can only be matched literally, not with a wildcard"
            ));
        }
        let normalized = format!(
            "{}/{}",
            type_part.to_ascii_lowercase(),
            subtype_part.to_ascii_lowercase()
        );
        if seen.contains(&normalized) {
            return Err(format!("duplicate media type {token:?}"));
        }
        seen.push(normalized);
    }
    Ok(())
}

/// `tt-policy-name = 1*( ALPHA / DIGIT / "-" / "#" / "=" / "_" / "/" / "@" / "." / "%" )`.
fn is_tt_policy_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '#' | '=' | '_' | '/' | '@' | '.' | '%')
}

/// `TrustedTypesDirective` (`trusted-types`).
fn check_trusted_types(directive: &Directive) -> Result<(), String> {
    let raw = directive.raw_value.as_deref().unwrap_or("");
    let tokens: Vec<&str> = raw.split_ascii_whitespace().collect();
    if tokens.is_empty() {
        // "Empty trusted-types directive allows all policy names" —
        // Warning in htmlunit-csp, invalid here (module doc comment).
        return Err("trusted-types directive has no value".to_string());
    }

    let mut none = false;
    let mut allow_duplicates = false;
    let mut policy_names: Vec<&str> = Vec::new();

    for token in &tokens {
        if token.eq_ignore_ascii_case("'none'") {
            if none {
                return Err("duplicate keyword 'none'".to_string());
            }
            none = true;
        } else if token.eq_ignore_ascii_case("'allow-duplicates'") {
            if allow_duplicates {
                return Err("duplicate keyword 'allow-duplicates'".to_string());
            }
            allow_duplicates = true;
        } else if *token == "*" {
            // htmlunit-csp warns unconditionally the first time `*`
            // appears ("permits any policy name, which may reduce
            // security"), regardless of what else is in the directive —
            // under this module's vnu-parity model that alone makes any
            // use of `*` invalid, so this always fails immediately (the
            // `star`-combined-with-other-things checks real
            // `TrustedTypesDirective` also has are unreachable as a
            // result and are not ported).
            return Err("trusted-types wildcard (*) always triggers a diagnostic".to_string());
        } else if token.starts_with('\'') && token.ends_with('\'') {
            return Err(format!("unrecognized trusted-types keyword {token:?}"));
        } else if token.chars().all(is_tt_policy_name_char) {
            if policy_names.contains(token) {
                return Err(format!("duplicate trusted-types policy name {token:?}"));
            }
            policy_names.push(token);
        } else {
            return Err(format!("invalid trusted-types policy name {token:?}"));
        }
    }

    if none && (allow_duplicates || !policy_names.is_empty()) {
        return Err(
            "'none' must not be combined with any other trusted-types expression".to_string(),
        );
    }
    if allow_duplicates && policy_names.is_empty() {
        return Err(
            "'allow-duplicates' has no effect without policy names or wildcard".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_content_security_policy as check;

    #[test]
    fn simple_fetch_directive_is_valid() {
        assert!(check("default-src 'self'").is_ok());
    }

    #[test]
    fn empty_source_list_is_invalid() {
        assert!(check("default-src").is_err());
    }

    #[test]
    fn none_alone_is_valid_but_combined_with_others_is_not() {
        assert!(check("default-src 'none'").is_ok());
        assert!(check("default-src 'none' 'self'").is_err());
    }

    #[test]
    fn duplicate_directive_is_invalid() {
        assert!(check("default-src 'self'; default-src 'self'").is_err());
    }

    #[test]
    fn unrecognized_directive_is_invalid() {
        assert!(check("totally-made-up-directive 'self'").is_err());
    }

    #[test]
    fn malformed_directive_name_is_invalid() {
        assert!(check("def@ult-src 'self'").is_err());
    }

    #[test]
    fn duplicate_source_expression_is_invalid() {
        assert!(check("script-src example.com example.com").is_err());
    }

    #[test]
    fn unrecognized_source_expression_is_invalid() {
        assert!(check("script-src 'unsafe-redirect'").is_err());
    }

    #[test]
    fn sandbox_valid_and_duplicate_and_unknown() {
        assert!(check("sandbox allow-scripts allow-forms").is_ok());
        assert!(check("sandbox").is_ok()); // empty sandbox = maximal restrictions, allowed
        assert!(check("sandbox allow-scripts allow-scripts").is_err());
        assert!(check("sandbox not-a-real-keyword").is_err());
    }

    #[test]
    fn sandbox_allow_downloads_is_stripped_before_parsing() {
        // vnu's blind substring strip removes "allow-downloads" globally,
        // leaving "sandbox " (whitespace only) — treated as an empty,
        // valid sandbox directive, not an unrecognized-keyword error.
        assert!(check("sandbox allow-downloads").is_ok());
    }

    #[test]
    fn boolean_directives() {
        assert!(check("upgrade-insecure-requests").is_ok());
        assert!(check("block-all-mixed-content").is_ok());
        assert!(check("upgrade-insecure-requests 'self'").is_err());
        assert!(check("block-all-mixed-content 'self'").is_err());
    }

    #[test]
    fn frame_ancestors_valid_and_restricted() {
        assert!(check("frame-ancestors 'self' example.com https:").is_ok());
        assert!(check("frame-ancestors 'unsafe-inline'").is_err());
        assert!(check("frame-ancestors 'nonce-abc123'").is_err());
        assert!(check("frame-ancestors").is_err());
    }

    #[test]
    fn report_to_exactly_one_valid_token() {
        assert!(check("report-to endpoint-1").is_ok());
        assert!(check("report-to").is_err());
        assert!(check("report-to endpoint-1 endpoint-2").is_err());
        assert!(check("report-to \"quoted\"").is_err());
    }

    #[test]
    fn require_trusted_types_for_script_only() {
        assert!(check("require-trusted-types-for 'script'").is_ok());
        assert!(check("require-trusted-types-for").is_err());
        assert!(check("require-trusted-types-for 'script' 'script'").is_err());
        assert!(check("require-trusted-types-for 'other'").is_err());
    }

    #[test]
    fn report_uri_requires_value_and_rejects_duplicates() {
        assert!(check("report-uri https://example.com/r").is_ok());
        assert!(check("report-uri").is_err());
        assert!(check("report-uri https://example.com/r https://example.com/r").is_err());
    }

    #[test]
    fn plugin_types_media_type_grammar() {
        assert!(check("plugin-types application/pdf").is_ok());
        assert!(check("plugin-types").is_ok()); // empty list explicitly allowed
        assert!(check("plugin-types not-a-media-type").is_err());
        assert!(check("plugin-types */pdf").is_err());
        assert!(check("plugin-types application/pdf application/pdf").is_err());
    }

    #[test]
    fn trusted_types_policy_names_and_keywords() {
        assert!(check("trusted-types my-policy").is_ok());
        assert!(check("trusted-types 'none'").is_ok());
        assert!(check("trusted-types 'allow-duplicates' my-policy").is_ok());
        assert!(check("trusted-types").is_err()); // empty
        assert!(check("trusted-types *").is_err());
        assert!(check("trusted-types 'allow-duplicates'").is_err()); // no effect w/o names or wildcard
        assert!(check("trusted-types 'none' my-policy").is_err());
        assert!(check("trusted-types my-policy my-policy").is_err());
        assert!(check("trusted-types 'bad name'").is_err());
    }

    #[test]
    fn hash_source_length_and_charset() {
        let ok_sha256 = format!("script-src 'sha256-{}'", "a".repeat(44));
        assert!(check(&ok_sha256).is_ok());
        let wrong_length = format!("script-src 'sha256-{}'", "a".repeat(40));
        assert!(check(&wrong_length).is_err());
        let base64url_chars = format!("script-src 'sha256-{}'", "a".repeat(43) + "-");
        assert!(check(&base64url_chars).is_err());
    }

    #[test]
    fn nonce_valid_and_duplicate() {
        assert!(check("script-src 'nonce-abc123'").is_ok());
        assert!(check("script-src 'nonce-abc123' 'nonce-abc123'").is_err());
    }

    #[test]
    fn non_ascii_is_invalid() {
        assert!(check("default-src 'self' café").is_err());
    }

    #[test]
    fn comma_separated_policy_list_all_must_be_valid() {
        assert!(check("default-src 'self', script-src 'self'").is_ok());
        assert!(check("default-src 'self', script-src").is_err());
    }
}
