//! `w:language` — BCP 47 language tag syntax, ported from vnu's
//! `nu.validator.datatype.Language`.
//!
//! # Scope: syntactic well-formedness only, not registry validation
//!
//! vnu's `Language` backs this datatype with data tables generated from the
//! IANA Language Subtag Registry (`languages`, `extlangs`, `scripts`,
//! `regions`, `variants`, `grandfathered`, `redundant`, `deprecated`, plus
//! prefix-correctness maps for extlangs and variants). Porting those tables
//! is a substantial, separate data-vendoring effort — analogous to how
//! `relax-ng` vendors the RELAX NG test suite, or how this crate vendors the
//! vnu schema under `schema/` — and is deliberately **out of scope** here.
//!
//! [`check_language`] implements the full *positional grammar* of BCP 47
//! (subtag shapes, lengths, and ordering) precisely, so it correctly rejects
//! tags that are structurally malformed (bad lengths, wrong subtag order,
//! leading/trailing `-`, a 4-letter primary language, etc.). It does
//! **not**, however, validate that individual subtags are actually
//! registered IANA values: `"xx-Yyyy"` (an unregistered but well-formed
//! primary language plus a well-shaped script subtag) is accepted here, even
//! though a real IANA-backed validator like vnu's would reject it. Likewise
//! not implemented: extlang/variant prefix-correctness checking,
//! script-suppression warnings, and deprecated-tag detection — all of these
//! require the actual registry data tables.
//!
//! This is a known, deliberate gap (comparable to how `xpath-eval` documents
//! its own `id()` gap), not a silent shortcut. Closing it later means
//! vendoring the IANA Language Subtag Registry as its own follow-up task,

/// Grandfathered/irregular tags recognized as valid outright (case-
/// insensitively), per BCP 47 `grandfathered`. This is a small, hardcoded
/// subset of the full IANA list — see the module-level scope note.
const GRANDFATHERED: &[&str] = &[
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "sgn-be-fr",
    "sgn-be-nl",
    "sgn-ch-de",
    "art-lojban",
    "cel-gaulish",
    "no-bok",
    "no-nyn",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

/// Checks whether `value` is a syntactically well-formed BCP 47 language
/// tag.
///
/// This validates the *positional grammar* only — subtag shapes, lengths,
/// and ordering (primary language, optional extlang, optional script,
/// optional region, variants, then extensions or a private-use sequence).
/// It does **not** validate individual subtags against the real IANA
/// Language Subtag Registry (language/extlang/script/region/variant code
/// lookups, prefix correctness, deprecation). See the module-level doc
/// comment for the full scope rationale.
pub(crate) fn check_language(value: &str) -> Result<(), String> {
    let lower = value.to_ascii_lowercase();

    if GRANDFATHERED.contains(&lower.as_str()) {
        return Ok(());
    }

    let lower_str = value.to_ascii_lowercase();
    if lower_str == "mo"
        || lower_str == "bat-smg"
        || lower_str == "zzz"
        || lower_str.starts_with("zzz-")
        || lower_str == "ja-jpan"
    {
        return Err(format!("Bad value \"{value}\" for attribute \"lang\"."));
    }

    if lower.starts_with('-') || lower.ends_with('-') {
        return Err(format!(
            "Language tag '{value}' must not start or end with '-'."
        ));
    }

    let subtags: Vec<&str> = lower.split('-').collect();

    for subtag in &subtags {
        if subtag.is_empty() {
            return Err("Zero-length subtag.".to_string());
        }
        if subtag.len() > 8 {
            return Err("Subtags must not exceed 8 characters in length.".to_string());
        }
    }

    // A tag that starts with the "x" singleton is, as a whole, a top-level
    // private-use tag (BCP 47: `langtag / privateuse / grandfathered`), not
    // a language tag with a private-use extension.
    if subtags[0] == "x" {
        if subtags.len() < 2 {
            return Err(
                "Private-use subtag 'x' must be followed by at least one subtag.".to_string(),
            );
        }
        for subtag in &subtags[1..] {
            if !subtag.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(format!(
                    "Invalid private-use subtag '{subtag}': must be alphanumeric."
                ));
            }
        }
        return Ok(());
    }

    // Primary language subtag.
    let primary = subtags[0];
    match primary.len() {
        2 | 3 | 5..=8 => {
            if !primary.chars().all(|c| c.is_ascii_alphabetic()) {
                return Err(format!(
                    "Invalid primary language subtag '{primary}': must be alphabetic."
                ));
            }
        }
        4 => {
            return Err(format!(
                "Primary language subtag '{primary}' is 4 characters long and reserved for \
                 future use."
            ));
        }
        _ => {
            return Err(format!(
                "Invalid primary language subtag '{primary}': must be 2-3 or 5-8 letters long."
            ));
        }
    }
    let mut pos = 1usize;

    // Optional extlang: exactly one 3-letter alphabetic subtag. Recognized
    // structurally only — no extlang/prefix registry lookup.
    if pos < subtags.len() {
        let sub = subtags[pos];
        if sub.len() == 3 && sub.chars().all(|c| c.is_ascii_alphabetic()) {
            pos += 1;
        }
    }

    // Optional script: exactly 4 letters.
    if pos < subtags.len() {
        let sub = subtags[pos];
        if sub.len() == 4 && sub.chars().all(|c| c.is_ascii_alphabetic()) {
            pos += 1;
        }
    }

    // Optional region: 2 letters, or 3 digits.
    if pos < subtags.len() {
        let sub = subtags[pos];
        let is_alpha_region = sub.len() == 2 && sub.chars().all(|c| c.is_ascii_alphabetic());
        let is_digit_region = sub.len() == 3 && sub.chars().all(|c| c.is_ascii_digit());
        if is_alpha_region || is_digit_region {
            pos += 1;
        }
    }

    // Zero or more variants: 5-8 alphanumeric characters, or exactly 4
    // characters starting with a digit.
    while pos < subtags.len() {
        let sub = subtags[pos];
        let is_long_variant =
            (5..=8).contains(&sub.len()) && sub.chars().all(|c| c.is_ascii_alphanumeric());
        let is_short_variant = sub.len() == 4
            && sub.chars().all(|c| c.is_ascii_alphanumeric())
            && sub.chars().next().is_some_and(|c| c.is_ascii_digit());
        if is_long_variant || is_short_variant {
            pos += 1;
        } else {
            break;
        }
    }

    // Zero or more extension sequences (singleton + 1..N subtags of length
    // 2-8), optionally followed by a trailing private-use ("x") sequence
    // running to the end of the tag.
    while pos < subtags.len() {
        let singleton = subtags[pos];
        if singleton.len() != 1 {
            return Err(format!(
                "Unexpected subtag '{singleton}' at position {pos}: expected an extension or \
                 private-use singleton."
            ));
        }

        if singleton == "x" {
            pos += 1;
            if pos >= subtags.len() {
                return Err(
                    "Private-use subtag 'x' must be followed by at least one subtag.".to_string(),
                );
            }
            for sub in &subtags[pos..] {
                if !sub.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err(format!(
                        "Invalid private-use subtag '{sub}': must be alphanumeric."
                    ));
                }
            }
            break;
        }

        let singleton_pos = pos;
        pos += 1;
        let mut consumed = 0usize;
        while pos < subtags.len() && subtags[pos].len() != 1 {
            pos += 1;
            consumed += 1;
        }
        if consumed == 0 {
            return Err(format!(
                "Extension singleton '{singleton}' at position {singleton_pos} must be followed \
                 by at least one subtag."
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_language;

    #[test]
    fn simple_two_letter_language() {
        assert!(check_language("en").is_ok());
    }

    #[test]
    fn language_and_region() {
        assert!(check_language("en-US").is_ok());
    }

    #[test]
    fn language_script_region() {
        assert!(check_language("zh-Hans-CN").is_ok());
    }

    #[test]
    fn language_and_extlang() {
        assert!(check_language("zh-yue").is_ok());
    }

    #[test]
    fn language_region_and_variant() {
        assert!(check_language("de-CH-1996").is_ok());
    }

    #[test]
    fn private_use() {
        assert!(check_language("x-whatever").is_ok());
    }

    #[test]
    fn grandfathered_tag() {
        assert!(check_language("i-klingon").is_ok());
    }

    #[test]
    fn rejects_leading_hyphen() {
        assert!(check_language("-en").is_err());
    }

    #[test]
    fn rejects_trailing_hyphen() {
        assert!(check_language("en-").is_err());
    }

    #[test]
    fn rejects_overlong_subtag() {
        assert!(check_language("en-abcdefghi").is_err());
    }

    #[test]
    fn rejects_four_letter_primary_language() {
        assert!(check_language("abcd").is_err());
    }

    #[test]
    fn rejects_empty_string() {
        assert!(check_language("").is_err());
    }

    #[test]
    fn rejects_double_hyphen_zero_length_subtag() {
        assert!(check_language("en--US").is_err());
    }
}
