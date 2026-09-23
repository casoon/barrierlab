//! The `hash-algorithm "-" base64-value` grammar (CSP3 §2.3.1), kept as
//! its own module deliberately: this exact sub-grammar, *without* CSP's
//! surrounding `'...'` quotes, is what Subresource Integrity's unquoted
//! `integrity=""` hash tokens (e.g. `sha256-...`) also use. `csp-parse`'s
//! `hash-source` (the quoted `'sha256-...'` form used inside a
//! `source-list`) is built on top of this module -- see
//! `plan/DECISIONS.md`, 2026-08-21 and 2026-08-22 entries.

/// One of the three hash algorithms CSP3 recognizes (`hash-algorithm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HashAlgorithm {
    /// `sha256`.
    Sha256,
    /// `sha384`.
    Sha384,
    /// `sha512`.
    Sha512,
}

impl HashAlgorithm {
    fn as_str(self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha384 => "sha384",
            HashAlgorithm::Sha512 => "sha512",
        }
    }

    const ALL: [HashAlgorithm; 3] = [
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
    ];
}

/// A parsed `hash-algorithm "-" base64-value` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HashExpression {
    /// The hash algorithm.
    pub algorithm: HashAlgorithm,
    /// The base64-value, exactly as it appeared (no decoding performed).
    pub value: String,
}

/// Parses `hash-algorithm "-" base64-value` (CSP3 §2.3.1) from `input`,
/// *without* CSP's surrounding `'...'` quotes -- callers parsing a
/// `hash-source` token strip the quotes first and pass the inner string
/// here; callers parsing an SRI `integrity=""` hash token pass it as-is.
pub fn parse_hash_expression(input: &str) -> Option<HashExpression> {
    for algorithm in HashAlgorithm::ALL {
        if let Some(value) = input
            .strip_prefix(algorithm.as_str())
            .and_then(|rest| rest.strip_prefix('-'))
        {
            return if is_valid_base64_value(value) {
                Some(HashExpression {
                    algorithm,
                    value: value.to_string(),
                })
            } else {
                None
            };
        }
    }
    None
}

/// Whether `s` matches `base64-value = 1*( ALPHA / DIGIT / "+" / "/" /
/// "-" / "_" ) *2( "=" )` (CSP3 §2.3.1).
pub(crate) fn is_valid_base64_value(s: &str) -> bool {
    let bytes = s.as_bytes();
    let body_len = bytes
        .iter()
        .take_while(|&&b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_'))
        .count();
    if body_len == 0 {
        return false;
    }
    let padding = &bytes[body_len..];
    padding.len() <= 2 && padding.iter().all(|&b| b == b'=')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_algorithm() {
        assert_eq!(
            parse_hash_expression("sha256-abc123"),
            Some(HashExpression {
                algorithm: HashAlgorithm::Sha256,
                value: "abc123".to_string(),
            })
        );
        assert_eq!(
            parse_hash_expression("sha384-abc123").map(|h| h.algorithm),
            Some(HashAlgorithm::Sha384)
        );
        assert_eq!(
            parse_hash_expression("sha512-abc123").map(|h| h.algorithm),
            Some(HashAlgorithm::Sha512)
        );
    }

    #[test]
    fn accepts_base64_with_padding_and_url_safe_chars() {
        assert!(parse_hash_expression("sha256-abc+/12==").is_some());
        assert!(parse_hash_expression("sha256-abc-_12").is_some());
    }

    #[test]
    fn rejects_unknown_algorithm() {
        assert_eq!(parse_hash_expression("sha1-abc123"), None);
        assert_eq!(parse_hash_expression("md5-abc123"), None);
    }

    #[test]
    fn rejects_empty_or_invalid_base64_value() {
        assert_eq!(parse_hash_expression("sha256-"), None);
        assert_eq!(parse_hash_expression("sha256-abc def"), None);
        assert_eq!(parse_hash_expression("sha256-abc==="), None); // 3x '=' > *2
    }
}
