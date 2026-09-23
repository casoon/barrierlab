//! Generic top-level splitting: policy-list -> policy -> directive.
//!
//! Phase 02 scope (see `plan/02-directive-splitting.md`). Infallible by
//! design (see `plan/DECISIONS.md`, 2026-08-22): CSP's own imperative
//! parsing algorithm never rejects a whole policy (list) for syntax
//! reasons at this level, it only ever drops or ignores individual
//! empty/malformed pieces.

use crate::ast::{Directive, Policy, PolicyList};

/// Parses a serialized CSP or comma-separated CSP list (as sent in the
/// `Content-Security-Policy` HTTP header) into a [`PolicyList`].
///
/// This only performs the generic, directive-independent split described
/// by CSP3 §2.2/§2.3 (`serialized-policy-list` / `serialized-policy` /
/// `serialized-directive`) -- directive values are kept raw. Directive
/// name/value byte-class conformance is available per directive via
/// [`Directive::name_is_valid`]/[`Directive::value_is_valid`].
pub fn parse_policy_list(input: &str) -> PolicyList {
    PolicyList {
        policies: input.split(',').map(parse_policy).collect(),
    }
}

fn parse_policy(segment: &str) -> Policy {
    let directives = segment
        .trim_ascii()
        .split(';')
        .map(str::trim_ascii)
        .filter(|token| !token.is_empty())
        .map(parse_directive)
        .collect();
    Policy { directives }
}

fn parse_directive(token: &str) -> Directive {
    match token.find(|c: char| c.is_ascii_whitespace()) {
        Some(split_at) => {
            let name = &token[..split_at];
            let rest = token[split_at..].trim_ascii_start();
            let raw_value = if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            };
            Directive {
                name: name.to_string(),
                raw_value,
            }
        }
        None => Directive {
            name: token.to_string(),
            raw_value: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directive(name: &str, raw_value: Option<&str>) -> Directive {
        Directive {
            name: name.to_string(),
            raw_value: raw_value.map(str::to_string),
        }
    }

    #[test]
    fn single_policy_multiple_directives() {
        let result = parse_policy_list(
            "default-src 'self'; img-src *; object-src 'none'; script-src example.com",
        );
        assert_eq!(
            result,
            PolicyList {
                policies: vec![Policy {
                    directives: vec![
                        directive("default-src", Some("'self'")),
                        directive("img-src", Some("*")),
                        directive("object-src", Some("'none'")),
                        directive("script-src", Some("example.com")),
                    ],
                }],
            }
        );
    }

    #[test]
    fn comma_separated_policy_list() {
        let result = parse_policy_list("default-src 'self', script-src 'none'");
        assert_eq!(result.policies.len(), 2);
        assert_eq!(
            result.policies[0].directives,
            vec![directive("default-src", Some("'self'"))]
        );
        assert_eq!(
            result.policies[1].directives,
            vec![directive("script-src", Some("'none'"))]
        );
    }

    #[test]
    fn empty_string_yields_single_empty_policy() {
        let result = parse_policy_list("");
        assert_eq!(
            result,
            PolicyList {
                policies: vec![Policy { directives: vec![] }],
            }
        );
    }

    #[test]
    fn whitespace_only_yields_empty_policy() {
        let result = parse_policy_list("   \t  ");
        assert_eq!(result.policies, vec![Policy { directives: vec![] }]);
    }

    #[test]
    fn semicolons_only_yield_empty_policy() {
        let result = parse_policy_list(";;;");
        assert_eq!(result.policies, vec![Policy { directives: vec![] }]);
    }

    #[test]
    fn empty_segments_between_commas_yield_empty_policies() {
        let result = parse_policy_list("default-src 'self',,report-to endpoint");
        assert_eq!(result.policies.len(), 3);
        assert_eq!(
            result.policies[0].directives,
            vec![directive("default-src", Some("'self'"))]
        );
        assert_eq!(result.policies[1].directives, vec![]);
        assert_eq!(
            result.policies[2].directives,
            vec![directive("report-to", Some("endpoint"))]
        );
    }

    #[test]
    fn multiple_whitespace_between_name_and_value_is_stripped() {
        let result = parse_policy_list("script-src    'self'");
        assert_eq!(
            result.policies[0].directives,
            vec![directive("script-src", Some("'self'"))]
        );
    }

    #[test]
    fn valueless_directive() {
        let result = parse_policy_list("upgrade-insecure-requests");
        assert_eq!(
            result.policies[0].directives,
            vec![directive("upgrade-insecure-requests", None)]
        );
    }

    #[test]
    fn directive_name_with_digits_and_hyphen() {
        let result = parse_policy_list("script-src-elem 'self'");
        assert_eq!(
            result.policies[0].directives,
            vec![directive("script-src-elem", Some("'self'"))]
        );
    }

    #[test]
    fn leading_and_trailing_whitespace_around_directive_is_trimmed() {
        let result = parse_policy_list("  script-src 'self'  ;  img-src *  ");
        assert_eq!(
            result.policies[0].directives,
            vec![
                directive("script-src", Some("'self'")),
                directive("img-src", Some("*")),
            ]
        );
    }

    #[test]
    fn fault_tolerance_malformed_directive_next_to_valid_ones() {
        // A control byte (0x01) in the value is outside directive-value's
        // allowed byte classes -- still parsed structurally, but flagged
        // as invalid via `value_is_valid`, not dropped or turned into a
        // parse error (see plan/DECISIONS.md, 2026-08-22).
        let result = parse_policy_list("default-src 'self'; script-src 'self' \u{1}bad");
        let directives = &result.policies[0].directives;
        assert_eq!(directives.len(), 2);
        assert!(directives[0].name_is_valid());
        assert!(directives[0].value_is_valid());
        assert!(directives[1].name_is_valid());
        assert!(!directives[1].value_is_valid());
    }

    #[test]
    fn invalid_directive_name_is_still_parsed_but_flagged() {
        let result = parse_policy_list("bad_name 'self'");
        let directive = &result.policies[0].directives[0];
        assert_eq!(directive.name, "bad_name");
        assert!(!directive.name_is_valid());
    }

    #[test]
    fn duplicate_directive_names_are_both_kept() {
        // Deduplication is deliberately out of scope for this phase --
        // see plan/DECISIONS.md, 2026-08-22.
        let result = parse_policy_list("default-src 'self'; default-src 'none'");
        assert_eq!(
            result.policies[0].directives,
            vec![
                directive("default-src", Some("'self'")),
                directive("default-src", Some("'none'")),
            ]
        );
    }
}
