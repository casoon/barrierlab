//! The `serialized-source-list` value grammar (CSP3 §2.3.1) shared by
//! most directives (`default-src`, `script-src`, …, `base-uri`,
//! `form-action`, and others -- see `plan/04-directive-registry.md` for
//! which directives use it). Operates on a directive's raw value string
//! (e.g. [`crate::Directive::raw_value`]).
//!
//! Parsing is infallible and lenient, consistent with
//! [`crate::parse_policy_list`] (see `plan/DECISIONS.md`, 2026-08-22): a
//! token that doesn't match any `source-expression` alternative is kept
//! as an unrecognized [`SourceListEntry`] rather than failing the whole
//! parse.

use crate::hash::{HashExpression, parse_hash_expression};

/// A parsed `serialized-source-list` (CSP3 §2.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceList {
    /// The sole `'none'` keyword -- the only form in which `'none'` may
    /// appear (it is an alternative to, not a member of, a list of other
    /// `source-expression`s).
    None,
    /// One or more whitespace-separated `source-expression` tokens.
    Sources(Vec<SourceListEntry>),
}

/// A single whitespace-separated token from a [`SourceList::Sources`]
/// list, together with its recognized [`SourceExpression`] (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceListEntry {
    /// The original token text, unmodified.
    pub raw: String,
    /// The recognized `source-expression`, or `None` if `raw` doesn't
    /// match any of the five alternatives.
    pub expression: Option<SourceExpression>,
}

/// One recognized `source-expression` alternative (CSP3 §2.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceExpression {
    /// A `scheme-source` (e.g. `https:`), without the trailing `:`.
    Scheme(String),
    /// A `host-source`.
    Host(HostSource),
    /// A `keyword-source` (e.g. `'self'`).
    Keyword(Keyword),
    /// The base64-value from a `'nonce-...'` token, without the
    /// `'nonce-`/`'` wrapper.
    Nonce(String),
    /// A `hash-source` (e.g. `'sha256-...'`).
    Hash(HashExpression),
}

/// A parsed `host-source` (CSP3 §2.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HostSource {
    /// The `scheme-part` before `://`, if present.
    pub scheme: Option<String>,
    /// The `host-part`.
    pub host: HostPart,
    /// The `port-part`, if present.
    pub port: Option<PortPart>,
    /// The raw `path-part`, if present, including its leading `/`.
    pub path: Option<String>,
}

/// A parsed `host-part` (CSP3 §2.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostPart {
    /// Bare `*` -- matches any host.
    AnyHost,
    /// `[ "*." ] 1*host-char *( "." 1*host-char ) [ "." ]`.
    Named {
        /// Whether the host started with the `*.` wildcard-label prefix.
        wildcard_prefix: bool,
        /// The dot-separated labels, in order (the `*.` prefix, if any,
        /// is not itself a label).
        labels: Vec<String>,
        /// Whether the host ended with a trailing `.`.
        trailing_dot: bool,
    },
}

/// A parsed `port-part` (CSP3 §2.3.1). Kept as a digit string rather
/// than a numeric type: the ABNF (`1*DIGIT`) does not bound the value to
/// a valid 16-bit port number, and this crate does not normalize.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PortPart {
    /// `1*DIGIT`, kept as a string (see this enum's docs).
    Number(String),
    /// `*`.
    Wildcard,
}

/// All `keyword-source` values (CSP3 §2.3.1). See
/// `plan/03-source-list-grammar.md`: this list was pulled from an
/// automated spec fetch, not verified character-for-character against
/// the current spec text -- re-check before treating it as exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Keyword {
    /// `'self'`.
    SelfKeyword,
    /// `'unsafe-inline'`.
    UnsafeInline,
    /// `'unsafe-eval'`.
    UnsafeEval,
    /// `'strict-dynamic'`.
    StrictDynamic,
    /// `'unsafe-hashes'`.
    UnsafeHashes,
    /// `'report-sample'`.
    ReportSample,
    /// `'unsafe-allow-redirects'`.
    UnsafeAllowRedirects,
    /// `'wasm-unsafe-eval'`.
    WasmUnsafeEval,
}

impl Keyword {
    const ALL: [(&'static str, Keyword); 8] = [
        ("self", Keyword::SelfKeyword),
        ("unsafe-inline", Keyword::UnsafeInline),
        ("unsafe-eval", Keyword::UnsafeEval),
        ("strict-dynamic", Keyword::StrictDynamic),
        ("unsafe-hashes", Keyword::UnsafeHashes),
        ("report-sample", Keyword::ReportSample),
        ("unsafe-allow-redirects", Keyword::UnsafeAllowRedirects),
        ("wasm-unsafe-eval", Keyword::WasmUnsafeEval),
    ];

    fn from_unquoted(s: &str) -> Option<Keyword> {
        Self::ALL
            .iter()
            .find(|(name, _)| *name == s)
            .map(|(_, keyword)| *keyword)
    }
}

/// Parses a directive's raw value as a `serialized-source-list`.
pub fn parse_source_list(raw: &str) -> SourceList {
    let tokens: Vec<&str> = raw.trim_ascii().split_ascii_whitespace().collect();
    if tokens.as_slice() == ["'none'"] {
        return SourceList::None;
    }
    SourceList::Sources(
        tokens
            .into_iter()
            .map(|token| SourceListEntry {
                raw: token.to_string(),
                expression: classify_source_expression(token),
            })
            .collect(),
    )
}

fn classify_source_expression(token: &str) -> Option<SourceExpression> {
    if token.len() >= 2 && token.starts_with('\'') && token.ends_with('\'') {
        let inner = &token[1..token.len() - 1];
        if let Some(keyword) = Keyword::from_unquoted(inner) {
            return Some(SourceExpression::Keyword(keyword));
        }
        if let Some(value) = inner.strip_prefix("nonce-") {
            return is_valid_nonce_value(value).then(|| SourceExpression::Nonce(value.to_string()));
        }
        return parse_hash_expression(inner).map(SourceExpression::Hash);
    }
    if let Some(scheme) = token.strip_suffix(':')
        && is_scheme(scheme)
    {
        return Some(SourceExpression::Scheme(scheme.to_string()));
    }
    parse_host_source(token).map(SourceExpression::Host)
}

fn is_valid_nonce_value(s: &str) -> bool {
    crate::hash::is_valid_base64_value(s)
}

/// `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` (RFC 3986 §3.1).
fn is_scheme(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn is_host_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

fn parse_host_source(token: &str) -> Option<HostSource> {
    let mut rest = token;
    let mut scheme = None;
    if let Some(pos) = rest.find("://") {
        let candidate = &rest[..pos];
        if !is_scheme(candidate) {
            return None;
        }
        scheme = Some(candidate.to_string());
        rest = &rest[pos + 3..];
    }
    let (host, rest) = split_host_part(rest)?;
    let (port, rest) = split_port_part(rest)?;
    let path = match rest {
        "" => None,
        p if p.starts_with('/') => Some(p.to_string()),
        _ => return None,
    };
    Some(HostSource {
        scheme,
        host,
        port,
        path,
    })
}

fn split_host_part(s: &str) -> Option<(HostPart, &str)> {
    if let Some(rest) = s.strip_prefix("*.") {
        let (labels, trailing_dot, rest) = scan_labels(rest)?;
        return Some((
            HostPart::Named {
                wildcard_prefix: true,
                labels,
                trailing_dot,
            },
            rest,
        ));
    }
    if let Some(rest) = s.strip_prefix('*') {
        return Some((HostPart::AnyHost, rest));
    }
    let (labels, trailing_dot, rest) = scan_labels(s)?;
    Some((
        HostPart::Named {
            wildcard_prefix: false,
            labels,
            trailing_dot,
        },
        rest,
    ))
}

/// Scans a leading `1*host-char *( "." 1*host-char ) [ "." ]` prefix of
/// `s`, returning its dot-separated labels, whether a trailing `.` was
/// present, and the unconsumed remainder.
fn scan_labels(s: &str) -> Option<(Vec<String>, bool, &str)> {
    let end = s
        .bytes()
        .take_while(|&b| is_host_char(b) || b == b'.')
        .count();
    if end == 0 {
        return None;
    }
    let (matched, rest) = s.split_at(end);
    let (label_str, trailing_dot) = match matched.strip_suffix('.') {
        Some(stripped) => (stripped, true),
        None => (matched, false),
    };
    if label_str.is_empty() {
        return None;
    }
    let labels: Vec<String> = label_str.split('.').map(str::to_string).collect();
    if labels.iter().any(String::is_empty) {
        return None;
    }
    Some((labels, trailing_dot, rest))
}

fn split_port_part(s: &str) -> Option<(Option<PortPart>, &str)> {
    let Some(rest) = s.strip_prefix(':') else {
        return Some((None, s));
    };
    if let Some(rest) = rest.strip_prefix('*') {
        return Some((Some(PortPart::Wildcard), rest));
    }
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return None;
    }
    let (digits, rest) = rest.split_at(digits_len);
    Some((Some(PortPart::Number(digits.to_string())), rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(raw: &str) -> Vec<Option<SourceExpression>> {
        match parse_source_list(raw) {
            SourceList::None => panic!("expected Sources, got None"),
            SourceList::Sources(entries) => entries.into_iter().map(|e| e.expression).collect(),
        }
    }

    #[test]
    fn none_is_exclusive() {
        assert_eq!(parse_source_list("'none'"), SourceList::None);
    }

    #[test]
    fn none_mixed_with_other_tokens_is_not_the_none_variant() {
        match parse_source_list("'none' 'self'") {
            SourceList::Sources(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].expression, None); // 'none' alone is not a source-expression
                assert!(entries[1].expression.is_some());
            }
            SourceList::None => panic!("must not collapse to None when combined with other tokens"),
        }
    }

    #[test]
    fn wildcard_hosts() {
        assert_eq!(
            sources("* *.example.com"),
            vec![
                Some(SourceExpression::Host(HostSource {
                    scheme: None,
                    host: HostPart::AnyHost,
                    port: None,
                    path: None,
                })),
                Some(SourceExpression::Host(HostSource {
                    scheme: None,
                    host: HostPart::Named {
                        wildcard_prefix: true,
                        labels: vec!["example".to_string(), "com".to_string()],
                        trailing_dot: false,
                    },
                    port: None,
                    path: None,
                })),
            ]
        );
    }

    #[test]
    fn host_with_numeric_and_wildcard_port() {
        assert_eq!(
            sources("example.com:443 example.com:*"),
            vec![
                Some(SourceExpression::Host(HostSource {
                    scheme: None,
                    host: HostPart::Named {
                        wildcard_prefix: false,
                        labels: vec!["example".to_string(), "com".to_string()],
                        trailing_dot: false,
                    },
                    port: Some(PortPart::Number("443".to_string())),
                    path: None,
                })),
                Some(SourceExpression::Host(HostSource {
                    scheme: None,
                    host: HostPart::Named {
                        wildcard_prefix: false,
                        labels: vec!["example".to_string(), "com".to_string()],
                        trailing_dot: false,
                    },
                    port: Some(PortPart::Wildcard),
                    path: None,
                })),
            ]
        );
    }

    #[test]
    fn host_with_path() {
        let result = sources("example.com/path/to/thing");
        match &result[0] {
            Some(SourceExpression::Host(HostSource { path, .. })) => {
                assert_eq!(path.as_deref(), Some("/path/to/thing"));
            }
            other => panic!("expected Host with path, got {other:?}"),
        }
    }

    #[test]
    fn host_with_scheme_and_path() {
        let result = sources("https://example.com/a");
        match &result[0] {
            Some(SourceExpression::Host(HostSource { scheme, path, .. })) => {
                assert_eq!(scheme.as_deref(), Some("https"));
                assert_eq!(path.as_deref(), Some("/a"));
            }
            other => panic!("expected Host with scheme+path, got {other:?}"),
        }
    }

    #[test]
    fn scheme_only() {
        assert_eq!(
            sources("https: data:"),
            vec![
                Some(SourceExpression::Scheme("https".to_string())),
                Some(SourceExpression::Scheme("data".to_string())),
            ]
        );
    }

    #[test]
    fn all_keywords() {
        let raw = "'self' 'unsafe-inline' 'unsafe-eval' 'strict-dynamic' \
                   'unsafe-hashes' 'report-sample' 'unsafe-allow-redirects' \
                   'wasm-unsafe-eval'";
        let expected = vec![
            Keyword::SelfKeyword,
            Keyword::UnsafeInline,
            Keyword::UnsafeEval,
            Keyword::StrictDynamic,
            Keyword::UnsafeHashes,
            Keyword::ReportSample,
            Keyword::UnsafeAllowRedirects,
            Keyword::WasmUnsafeEval,
        ];
        for (entry, keyword) in sources(raw).into_iter().zip(expected) {
            assert_eq!(entry, Some(SourceExpression::Keyword(keyword)));
        }
    }

    #[test]
    fn nonce_valid_and_invalid() {
        assert_eq!(
            sources("'nonce-abc123+/=='")[0],
            Some(SourceExpression::Nonce("abc123+/==".to_string()))
        );
        assert_eq!(sources("'nonce-'")[0], None); // empty base64-value
        assert_eq!(sources("'nonce-bad value'").len(), 2); // whitespace splits the token itself
    }

    #[test]
    fn hashes_with_and_without_padding() {
        assert_eq!(
            sources("'sha256-abc123=='")[0],
            Some(SourceExpression::Hash(HashExpression {
                algorithm: crate::hash::HashAlgorithm::Sha256,
                value: "abc123==".to_string(),
            }))
        );
        assert_eq!(
            sources("'sha384-abc123'")[0],
            Some(SourceExpression::Hash(HashExpression {
                algorithm: crate::hash::HashAlgorithm::Sha384,
                value: "abc123".to_string(),
            }))
        );
        assert_eq!(
            sources("'sha512-abc123'")[0],
            Some(SourceExpression::Hash(HashExpression {
                algorithm: crate::hash::HashAlgorithm::Sha512,
                value: "abc123".to_string(),
            }))
        );
    }

    #[test]
    fn unrecognized_tokens_are_kept_but_unclassified() {
        match parse_source_list("'self' not-a-valid-source!") {
            SourceList::Sources(entries) => {
                assert_eq!(entries.len(), 2);
                assert!(entries[0].expression.is_some());
                assert_eq!(entries[1].raw, "not-a-valid-source!");
                assert_eq!(entries[1].expression, None);
            }
            SourceList::None => panic!("expected Sources"),
        }
    }

    #[test]
    fn malformed_quotes_are_unrecognized() {
        assert_eq!(sources("'unknown-keyword'")[0], None);
        assert_eq!(sources("'self")[0], None);
    }
}
