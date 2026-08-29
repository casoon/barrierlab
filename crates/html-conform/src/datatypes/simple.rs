//! Pure validation-check functions for the "trivial/mechanical" subset of the
//! `http://whattf.org/datatype-draft` (`w:*`) custom RELAX NG datatypes used
//! by the vendored vnu HTML5 schema.
//!
//! Source of truth: `validator/validator`, Java package
//! `src/nu/validator/datatype/` (see `plan/05c-research-group-a.md`, items
//! 1-3, 6-15). Per `plan/05c-datatype-library.md`'s "vnu-Parität als
//! Default" principle, these functions replicate vnu's actual runtime
//! behavior — including documented vnu-specific quirks/bugs — rather than a
//! "corrected" reading of the informal spec prose.
//!
//! Nothing calls these functions yet; a later consolidation phase wires them
//! into the actual `relax_ng::DatatypeLibrary` trait implementation (see
//! `src/infoset.rs` for the established precedent of this pattern in this
//! crate).

/// The five ASCII whitespace characters vnu's own `isWhitespace(char)` helper
/// checks for — NOT Rust's `char::is_whitespace()`, which covers the much
/// broader Unicode whitespace set and would accept/reject the wrong
/// characters here.
fn is_html_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{0C}' | '\n' | '\r')
}

/// `w:ID` → `Id.java`: non-empty, and no vnu-whitespace character anywhere in
/// the string. Any other character (including what HTML4's NMTOKEN would
/// reject) is allowed.
pub(crate) fn check_id(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("ID must not be empty".to_string());
    }
    if value.chars().any(is_html_whitespace) {
        return Err("ID must not contain whitespace".to_string());
    }
    Ok(())
}

/// `w:IDREF` → `Idref.java` (extends `Id`): byte-for-byte identical check to
/// `w:ID` — only vnu's display name ("id reference") differs.
pub(crate) fn check_idref(value: &str) -> Result<(), String> {
    check_id(value)
}

/// `w:IDREFS` → `Idrefs.java`: valid as soon as the string contains at least
/// one non-whitespace character (i.e. not empty and not all-whitespace). No
/// per-token splitting, no duplicate check, no whitespace normalization.
pub(crate) fn check_idrefs(value: &str) -> Result<(), String> {
    if value.chars().any(|c| !is_html_whitespace(c)) {
        Ok(())
    } else {
        Err("IDREFS must contain at least one non-whitespace character".to_string())
    }
}

/// `w:non-empty-string` → `NonEmptyString.java`: just non-empty. No
/// trimming, no rejection of whitespace-only strings.
pub(crate) fn check_non_empty_string(value: &str) -> Result<(), String> {
    if value.is_empty() {
        Err("value must not be empty".to_string())
    } else {
        Ok(())
    }
}

/// `w:string` → `AsciiCaseInsensitiveString.java`: `checkValid` is a no-op —
/// every string is accepted unconditionally. This is vnu's actual, intended
/// behavior, not an unfinished stub.
pub(crate) fn check_string(_value: &str) -> Result<(), String> {
    Ok(())
}

/// Value-equality companion to [`check_string`]. `AsciiCaseInsensitiveString
/// ::createValue` returns the ASCII-lowercased value, so wherever the schema
/// compares against a fixed `w:string` value, the comparison is effectively
/// ASCII-case-insensitive (only `A-Z`/`a-z` folded, no full Unicode case
/// folding). Not called by anything in this batch — feeds a later
/// `DatatypeLibrary::values_equal` implementation.
pub(crate) fn values_equal_ascii_case_insensitive(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// `w:string-without-line-breaks` → `StringWithoutLineBreaks.java`: valid
/// unless it contains `\n` or `\r` anywhere. The empty string is valid.
pub(crate) fn check_string_without_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        Err("value must not contain line breaks".to_string())
    } else {
        Ok(())
    }
}

/// `w:zero` → `Zero.java`: valid iff the value is the exact one-character
/// string `"0"`. `"00"`, `"-0"`, `""`, `"0 "` are all invalid.
pub(crate) fn check_zero(value: &str) -> Result<(), String> {
    if value == "0" {
        Ok(())
    } else {
        Err("value must be exactly \"0\"".to_string())
    }
}

/// `w:integer` → `Int.java`/`AbstractInt.checkInt`: optional leading `-`,
/// then one-or-more ASCII digits.
///
/// **Deliberately replicated vnu bug**: a lone `"-"` (a minus sign with no
/// digits after it) passes this check in vnu. The real implementation
/// checks for a leading `-` and then loops over the remaining characters
/// requiring each to be a digit — but never separately requires that loop to
/// run at least once. For any non-empty input other than a bare `"-"`, that
/// loop is non-empty anyway (either there was no sign, so the whole
/// non-empty string is scanned, or there was a sign followed by at least one
/// more character), so this only manifests for the single-character string
/// `"-"`. Per `plan/05c-datatype-library.md`'s vnu-parity principle, this is
/// intentionally reproduced, not "fixed". Source:
/// `plan/05c-research-group-a.md` item 10, citing `Int.java`/
/// `AbstractInt.checkInt`.
pub(crate) fn check_integer(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("integer must not be empty".to_string());
    }
    let chars: Vec<char> = value.chars().collect();
    let start = if chars[0] == '-' { 1 } else { 0 };
    for &c in &chars[start..] {
        if !c.is_ascii_digit() {
            return Err(format!("invalid integer: non-digit character '{c}'"));
        }
    }
    Ok(())
}

/// `w:integer-non-negative` → `IntNonNegative.java`: one-or-more ASCII
/// digits only, no sign character permitted at all (not even `+`). Leading
/// zeros are allowed.
pub(crate) fn check_integer_non_negative(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("integer must not be empty".to_string());
    }
    if !value.chars().all(|c| c.is_ascii_digit()) {
        return Err("value must contain only ASCII digits, no sign".to_string());
    }
    Ok(())
}

/// `w:integer-positive` → `IntPositive.java`: digits only (no sign), and not
/// all-zero — `"0"`/`"000"` are invalid, `"001"` is valid since its value is
/// non-zero despite the leading zeros.
pub(crate) fn check_integer_positive(value: &str) -> Result<(), String> {
    check_integer_non_negative(value)?;
    if value.chars().all(|c| c == '0') {
        return Err("value must not be all zeros".to_string());
    }
    Ok(())
}

/// Which of the three `FloatingPointExponent*` vnu classes to enforce. All
/// three share the same core "CSS/HTML floating-point-with-exponent" grammar
/// (optional leading dot form, optional exponent) but differ in how they
/// treat sign and an all-zero mantissa — see [`check_float_value`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatVariant {
    /// `w:float` → `FloatingPointExponent.java`.
    Any,
    /// `w:float-non-negative` → `FloatingPointExponentNonNegative.java`.
    NonNegative,
    /// `w:float-positive` → `FloatingPointExponentPositive.java`.
    Positive,
}

/// Shared state-machine implementation of vnu's hand-rolled
/// `FloatingPointExponent*` parsers (per `plan/05c-research-group-a.md`
/// items 13-15). Grammar common to all three variants:
///
/// ```text
/// sign?  mantissa  exponent?
/// mantissa = '.' digit+
///          | digit+ ( '.' digit+ )?
/// exponent = ('e' | 'E') ('+' | '-')? digit+
/// ```
///
/// `sign` is only ever `-` (never `+` — vnu's `isCSS()` branch that would
/// allow a leading `+` is not reachable for any of these three types).
/// Variant-specific end-state rules, verified against the research doc:
///
/// - [`FloatVariant::Any`] (`w:float`): no further restriction — any sign,
///   any mantissa digits.
/// - [`FloatVariant::NonNegative`] (`w:float-non-negative`): if a leading
///   `-` is present, every digit in the mantissa (integer part AND
///   fractional part, but NOT the exponent) must be `'0'` — i.e. only
///   spellings of negative zero (`-0`, `-0.0`, `-0.000e5`) are accepted;
///   `-1` is rejected because its mantissa contains a non-zero digit.
/// - [`FloatVariant::Positive`] (`w:float-positive`): a leading `-` is
///   rejected outright (hard error, checked before the mantissa is parsed);
///   additionally, if every mantissa digit (integer + fractional part) is
///   `'0'`, the value is rejected even without a sign (`"0"`, `"0.0"`,
///   `"0e10"` invalid; `"0.01"` valid because it has a non-zero digit).
fn check_float_value(value: &str, variant: FloatVariant) -> Result<(), String> {
    let chars: Vec<char> = value.chars().collect();
    let n = chars.len();
    let mut i = 0usize;

    let mut negative = false;
    if i < n && chars[i] == '-' {
        if variant == FloatVariant::Positive {
            return Err("float-positive must not start with '-'".to_string());
        }
        negative = true;
        i += 1;
    }

    let mut saw_mantissa_digit = false;
    let mut mantissa_all_zero = true;

    if i < n && chars[i] == '.' {
        i += 1;
        let frac_start = i;
        while i < n && chars[i].is_ascii_digit() {
            if chars[i] != '0' {
                mantissa_all_zero = false;
            }
            saw_mantissa_digit = true;
            i += 1;
        }
        if i == frac_start {
            return Err("expected digit(s) after '.'".to_string());
        }
    } else {
        let int_start = i;
        while i < n && chars[i].is_ascii_digit() {
            if chars[i] != '0' {
                mantissa_all_zero = false;
            }
            saw_mantissa_digit = true;
            i += 1;
        }
        if i == int_start {
            return Err("expected digit(s) or '.'".to_string());
        }
        if i < n && chars[i] == '.' {
            i += 1;
            let frac_start = i;
            while i < n && chars[i].is_ascii_digit() {
                if chars[i] != '0' {
                    mantissa_all_zero = false;
                }
                saw_mantissa_digit = true;
                i += 1;
            }
            if i == frac_start {
                return Err("expected digit(s) after '.'".to_string());
            }
        }
    }

    if !saw_mantissa_digit {
        return Err("no mantissa digits found".to_string());
    }

    if variant == FloatVariant::NonNegative && negative && !mantissa_all_zero {
        return Err(
            "float-non-negative: a negative value must be an all-zero mantissa (e.g. -0, -0.0)"
                .to_string(),
        );
    }

    if i < n && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < n && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        let exp_start = i;
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return Err("expected digit(s) in exponent".to_string());
        }
    }

    if i != n {
        return Err(format!("unexpected trailing character at position {i}"));
    }

    if variant == FloatVariant::Positive && mantissa_all_zero {
        return Err("float-positive: value must not be zero".to_string());
    }

    Ok(())
}

/// `w:float` → `FloatingPointExponent.java`. See [`check_float_value`] for
/// the shared grammar and per-variant end-state rules.
pub(crate) fn check_float(value: &str) -> Result<(), String> {
    check_float_value(value, FloatVariant::Any)
}

/// `w:float-non-negative` → `FloatingPointExponentNonNegative.java`. See
/// [`check_float_value`] for the shared grammar and per-variant end-state
/// rules.
pub(crate) fn check_float_non_negative(value: &str) -> Result<(), String> {
    check_float_value(value, FloatVariant::NonNegative)
}

/// `w:float-positive` → `FloatingPointExponentPositive.java`. See
/// [`check_float_value`] for the shared grammar and per-variant end-state
/// rules.
pub(crate) fn check_float_positive(value: &str) -> Result<(), String> {
    check_float_value(value, FloatVariant::Positive)
}

/// `w:hash-name` → `HashName.java`: must start with `#` and have length ≥ 2
/// (at least one character after the `#`). No further restriction on the
/// rest of the string.
pub(crate) fn check_hash_name(value: &str) -> Result<(), String> {
    if value.starts_with('#') && value.len() > 1 {
        Ok(())
    } else {
        Err("hash-name must start with '#' followed by at least one character".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_valid() {
        assert!(check_id("foo").is_ok());
    }

    #[test]
    fn id_empty_invalid() {
        assert!(check_id("").is_err());
    }

    #[test]
    fn id_whitespace_invalid() {
        assert!(check_id("foo bar").is_err());
        assert!(check_id("foo\tbar").is_err());
        assert!(check_id("foo\u{0C}bar").is_err());
    }

    #[test]
    fn idref_delegates_to_id() {
        assert!(check_idref("foo").is_ok());
        assert!(check_idref("foo bar").is_err());
        assert!(check_idref("").is_err());
    }

    #[test]
    fn idrefs_valid() {
        assert!(check_idrefs("foo bar").is_ok());
    }

    #[test]
    fn idrefs_empty_invalid() {
        assert!(check_idrefs("").is_err());
    }

    #[test]
    fn idrefs_all_whitespace_invalid() {
        assert!(check_idrefs("   \t\n").is_err());
    }

    #[test]
    fn non_empty_string_valid() {
        assert!(check_non_empty_string("foo").is_ok());
    }

    #[test]
    fn non_empty_string_empty_invalid() {
        assert!(check_non_empty_string("").is_err());
    }

    #[test]
    fn non_empty_string_whitespace_only_is_valid() {
        // No trimming, no rejection of whitespace-only strings.
        assert!(check_non_empty_string("   ").is_ok());
    }

    #[test]
    fn string_always_valid() {
        assert!(check_string("anything").is_ok());
        assert!(check_string("").is_ok());
        assert!(check_string("   \n\t").is_ok());
    }

    #[test]
    fn string_values_equal_ascii_case_insensitive() {
        assert!(values_equal_ascii_case_insensitive("Foo", "foo"));
        assert!(!values_equal_ascii_case_insensitive("Foo", "bar"));
        // Only ASCII A-Z/a-z folded, not full Unicode case folding.
        assert!(!values_equal_ascii_case_insensitive("Straße", "STRASSE"));
    }

    #[test]
    fn string_without_line_breaks_valid() {
        assert!(check_string_without_line_breaks("foo bar").is_ok());
    }

    #[test]
    fn string_without_line_breaks_empty_is_valid() {
        assert!(check_string_without_line_breaks("").is_ok());
    }

    #[test]
    fn string_without_line_breaks_rejects_newlines() {
        assert!(check_string_without_line_breaks("foo\nbar").is_err());
        assert!(check_string_without_line_breaks("foo\rbar").is_err());
    }

    #[test]
    fn zero_exact_match_valid() {
        assert!(check_zero("0").is_ok());
    }

    #[test]
    fn zero_empty_invalid() {
        assert!(check_zero("").is_err());
    }

    #[test]
    fn zero_rejects_variants() {
        assert!(check_zero("00").is_err());
        assert!(check_zero("-0").is_err());
        assert!(check_zero("0 ").is_err());
    }

    #[test]
    fn integer_valid() {
        assert!(check_integer("42").is_ok());
        assert!(check_integer("-42").is_ok());
    }

    #[test]
    fn integer_empty_invalid() {
        assert!(check_integer("").is_err());
    }

    #[test]
    fn integer_leading_zeros_allowed() {
        assert!(check_integer("007").is_ok());
    }

    #[test]
    fn integer_lone_minus_passes_per_vnu_quirk() {
        // Deliberately replicated vnu bug (Int.java/AbstractInt.checkInt):
        // a bare "-" with no digits after it passes vnu's check. Do not
        // "fix" this — see the doc comment on check_integer.
        assert!(check_integer("-").is_ok());
    }

    #[test]
    fn integer_rejects_leading_plus() {
        assert!(check_integer("+42").is_err());
    }

    #[test]
    fn integer_rejects_non_digit() {
        assert!(check_integer("4a").is_err());
    }

    #[test]
    fn integer_non_negative_valid() {
        assert!(check_integer_non_negative("007").is_ok());
    }

    #[test]
    fn integer_non_negative_empty_invalid() {
        assert!(check_integer_non_negative("").is_err());
    }

    #[test]
    fn integer_non_negative_rejects_any_sign() {
        assert!(check_integer_non_negative("-1").is_err());
        assert!(check_integer_non_negative("+1").is_err());
    }

    #[test]
    fn integer_positive_valid() {
        assert!(check_integer_positive("42").is_ok());
    }

    #[test]
    fn integer_positive_empty_invalid() {
        assert!(check_integer_positive("").is_err());
    }

    #[test]
    fn integer_positive_all_zero_invalid() {
        assert!(check_integer_positive("0").is_err());
        assert!(check_integer_positive("000").is_err());
    }

    #[test]
    fn integer_positive_leading_zero_nonzero_value_valid() {
        assert!(check_integer_positive("001").is_ok());
    }

    #[test]
    fn float_valid() {
        assert!(check_float("42.5").is_ok());
    }

    #[test]
    fn float_empty_invalid() {
        assert!(check_float("").is_err());
    }

    #[test]
    fn float_leading_dot_form_valid() {
        assert!(check_float(".5").is_ok());
    }

    #[test]
    fn float_negative_valid() {
        assert!(check_float("-1").is_ok());
    }

    #[test]
    fn float_rejects_leading_plus() {
        assert!(check_float("+1").is_err());
    }

    #[test]
    fn float_exponent_form_valid() {
        assert!(check_float("1e10").is_ok());
        assert!(check_float("1.5e-3").is_ok());
    }

    #[test]
    fn float_rejects_incomplete_forms() {
        assert!(check_float("5.").is_err());
        assert!(check_float("5e").is_err());
        assert!(check_float("5e+").is_err());
    }

    #[test]
    fn float_non_negative_valid() {
        assert!(check_float_non_negative("1.5").is_ok());
    }

    #[test]
    fn float_non_negative_empty_invalid() {
        assert!(check_float_non_negative("").is_err());
    }

    #[test]
    fn float_non_negative_zero_spellings_valid() {
        assert!(check_float_non_negative("-0").is_ok());
        assert!(check_float_non_negative("-0.0").is_ok());
        assert!(check_float_non_negative("-0.000e5").is_ok());
    }

    #[test]
    fn float_non_negative_rejects_real_negative() {
        assert!(check_float_non_negative("-1").is_err());
    }

    #[test]
    fn float_non_negative_exponent_form_valid() {
        assert!(check_float_non_negative("1e10").is_ok());
    }

    #[test]
    fn float_positive_valid() {
        assert!(check_float_positive("0.01").is_ok());
    }

    #[test]
    fn float_positive_empty_invalid() {
        assert!(check_float_positive("").is_err());
    }

    #[test]
    fn float_positive_rejects_all_zero() {
        assert!(check_float_positive("0").is_err());
        assert!(check_float_positive("0.0").is_err());
        assert!(check_float_positive("0e10").is_err());
    }

    #[test]
    fn float_positive_rejects_minus() {
        assert!(check_float_positive("-1").is_err());
    }

    #[test]
    fn float_positive_exponent_form_valid() {
        assert!(check_float_positive("1e10").is_ok());
    }

    #[test]
    fn hash_name_valid() {
        assert!(check_hash_name("#foo").is_ok());
    }

    #[test]
    fn hash_name_empty_invalid() {
        assert!(check_hash_name("").is_err());
    }

    #[test]
    fn hash_name_bare_hash_invalid() {
        assert!(check_hash_name("#").is_err());
    }

    #[test]
    fn hash_name_missing_hash_invalid() {
        assert!(check_hash_name("foo").is_err());
    }
}
