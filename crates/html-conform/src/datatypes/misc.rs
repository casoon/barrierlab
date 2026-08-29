//! Pure validation-check functions for a "no-op stub" / "geometry" subset of
//! the `http://whattf.org/datatype-draft` (`w:*`) custom RELAX NG datatypes
//! used by the vendored vnu HTML5 schema.
//!
//! Source of truth: `validator/validator`, Java package
//! `src/nu/validator/datatype/` (see `plan/05c-research-group-b.md`, items
//! 11, 12, 20, 21, 22, 23, 26). Per `plan/05c-datatype-library.md`'s
//! "vnu-Parität als Default" principle, these functions replicate vnu's
//! actual runtime behavior — including documented vnu-specific no-op stubs
//! and the accepted, temporary `w:source-size-list` media-condition gap —
//! rather than a "corrected" reading of the informal spec prose.
//!
//! Nothing calls these functions yet; a later consolidation phase wires them
//! into the actual `relax_ng::DatatypeLibrary` trait implementation (see
//! `src/infoset.rs` for the established precedent of this pattern in this

/// The five ASCII whitespace characters vnu's own `isWhitespace(char)`
/// helper checks for — NOT Rust's `char::is_whitespace()`, which covers the
/// much broader Unicode whitespace set and would accept/reject the wrong
/// characters here. Reimplemented locally (rather than imported from a
/// sibling batch file such as `simple.rs`) to keep this file's build
/// independent of the other parallel datatype batches.
fn is_html_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{0C}' | '\n' | '\r')
}

/// `w:email-address` → `EmailAddress.java` — **No-Op-Stub in vnu**
/// (`plan/05c-research-group-b.md` item 11): vnu's `checkValid` for this
/// type is a literal empty method body (historically a
/// `// TODO Auto-generated method stub` that was never filled in). It never
/// throws, so it accepts every string, including the empty string — there is
/// no WHATWG "valid e-mail address" regex implemented anywhere in vnu for
/// this type. Per the "vnu-Parität als Default" principle
/// (`plan/05c-datatype-library.md`), this is replicated 1:1 as a genuine,
/// intentional no-op — this is the finished, intended behavior, not an
/// unfinished TODO on our side.
pub(crate) fn check_email_address(_value: &str) -> Result<(), String> {
    Ok(())
}

/// `w:email-address-list` → `EmailAddressList.java` — **also a No-Op-Stub**
/// (`plan/05c-research-group-b.md` item 12): identical situation to
/// [`check_email_address`] — vnu's `checkValid` never validates anything,
/// so this accepts every string unconditionally, by deliberate vnu-parity
/// design, not because the check is unimplemented.
pub(crate) fn check_email_address_list(_value: &str) -> Result<(), String> {
    Ok(())
}

/// `w:color` → `Color.java` — **No-Op-Stub in vnu**
/// (`plan/05c-research-group-b.md` item 20): like `w:email-address`, vnu's
/// `checkValid` has an empty method body and validates nothing. Used for
/// `<link rel="mask-icon" color="">` and `<meta name="theme-color"
/// content="">`.
///
/// **Not to be confused with `w:simple-color`** (a different `w:*` type,
/// implemented in `src/datatypes/structural.rs` as part of a different
/// parallel batch): `w:simple-color` IS fully validated by vnu (`#` +
/// exactly 6 hex digits, i.e. `#RRGGBB`). `w:color` itself is genuinely
/// unvalidated in vnu — this is not a mistake, and not a stand-in for
/// `simple-color`'s logic; it is the deliberate, vnu-parity no-op state for
/// this specific type.
pub(crate) fn check_color(_value: &str) -> Result<(), String> {
    Ok(())
}

/// Local signed-integer shape check shared by [`check_circle`],
/// [`check_polyline`], and [`check_rectangle`]: optional leading `-`,
/// followed by one-or-more ASCII digits. Unlike `w:integer`'s own
/// documented vnu quirk (a lone `"-"` with no digits after it passes vnu's
/// `Int.java`/`AbstractInt` check), the coordinate grammars checked here
/// require at least one digit — a bare `"-"` is rejected.
fn check_signed_integer(s: &str) -> Result<(), String> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("\"{s}\" is not a valid integer"));
    }
    Ok(())
}

/// Local non-negative-integer shape check: one-or-more ASCII digits, no
/// sign character permitted at all (not even `+`).
fn check_unsigned_integer(s: &str) -> Result<(), String> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("\"{s}\" is not a valid non-negative integer"));
    }
    Ok(())
}

/// `w:circle` → `Circle.java` (`plan/05c-research-group-b.md` item 21):
/// used by `<area shape="circle" coords="">`. Exactly three
/// comma-separated values (x, y, radius); x and y are signed integers,
/// radius is an unsigned (non-negative) integer.
pub(crate) fn check_circle(value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 3 {
        return Err("circle coords must have three comma-separated integers".to_string());
    }
    check_signed_integer(parts[0])?;
    check_signed_integer(parts[1])?;
    check_unsigned_integer(parts[2])?;
    Ok(())
}

/// `w:polyline` → `Polyline.java` (`plan/05c-research-group-b.md` item 22):
/// used by `<area shape="poly" coords="">`. At least six comma-separated
/// values, an even count overall, every value a signed integer.
pub(crate) fn check_polyline(value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() < 6 {
        return Err("polyline coords must have at least six comma-separated integers".to_string());
    }
    if parts.len() % 2 != 0 {
        return Err(
            "polyline coords must have an even number of comma-separated integers".to_string(),
        );
    }
    for part in &parts {
        check_signed_integer(part)?;
    }
    Ok(())
}

/// `w:rectangle` → `Rectangle.java` (`plan/05c-research-group-b.md` item
/// 23): used by `<area shape="rect" coords="">`. Exactly four
/// comma-separated signed integers (left, top, right, bottom), plus the
/// non-degenerate-rectangle constraints `left < right` and `top < bottom`.
///
/// The four parts are parsed as `i32` (rather than `i64`) to mirror vnu's
/// own use of Java's 32-bit `Integer.parseInt` for this type — an
/// out-of-`i32`-range value is rejected the same way vnu would reject it.
pub(crate) fn check_rectangle(value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 4 {
        return Err(
            "rectangle coords must have exactly four comma-separated integers (left,top,right,bottom)"
                .to_string(),
        );
    }
    for part in &parts {
        check_signed_integer(part)?;
    }
    let left: i32 = parts[0]
        .parse()
        .map_err(|_| format!("\"{}\" does not fit in a 32-bit integer", parts[0]))?;
    let top: i32 = parts[1]
        .parse()
        .map_err(|_| format!("\"{}\" does not fit in a 32-bit integer", parts[1]))?;
    let right: i32 = parts[2]
        .parse()
        .map_err(|_| format!("\"{}\" does not fit in a 32-bit integer", parts[2]))?;
    let bottom: i32 = parts[3]
        .parse()
        .map_err(|_| format!("\"{}\" does not fit in a 32-bit integer", parts[3]))?;
    if left >= right {
        return Err("rectangle left must be less than right".to_string());
    }
    if top >= bottom {
        return Err("rectangle top must be less than bottom".to_string());
    }
    Ok(())
}

/// CSS math function names allowed to terminate a `w:source-size-list` entry
/// (case-insensitive), per `plan/05c-research-group-b.md` item 26.
const SOURCE_SIZE_MATH_FUNCTIONS: &[&str] = &[
    "calc", "min", "max", "clamp", "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "pow",
    "sqrt", "hypot", "log", "exp", "abs", "sign",
];

/// CSS `<length>` units, including the CSS Values 4 viewport-unit variants,
/// per `plan/05c-research-group-b.md` item 26 (originally researched for
/// `w:source-size-list`; also reused by `w:media-query`'s `width`/`height`
/// media-feature value-type check — the CSS `<length>` production is the
/// same type in both contexts).
pub(crate) const CSS_LENGTH_UNITS: &[&str] = &[
    "em", "ex", "ch", "rem", "cap", "ic", "vw", "svw", "lvw", "dvw", "vh", "svh", "lvh", "dvh",
    "vi", "svi", "lvi", "dvi", "vb", "svb", "lvb", "dvb", "vmin", "svmin", "lvmin", "dvmin",
    "vmax", "svmax", "lvmax", "dvmax", "cm", "mm", "q", "in", "pc", "pt", "px",
];

/// Splits `s` on top-level `,` characters, i.e. commas that are not nested
/// inside `(...)`. Used to separate `w:source-size-list` entries without
/// breaking on commas inside media conditions or CSS functions (e.g.
/// `calc(1px, 2px)` — not valid CSS, but the splitter must not be fooled by
/// it either way; depth-tracking handles both real and malformed nesting the
/// same way vnu's own hand-written parser does).
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Validates a plain CSS `<length>` token as required by the final segment
/// of a `w:source-size-list` entry: literal `0`/`0.0`/etc. is valid without
/// a unit; otherwise the trailing alphabetic run is the unit (must be in
/// [`CSS_LENGTH_UNITS`], case-insensitively) and the remainder must
/// be a non-negative CSS number.
fn check_source_size_length(token: &str) -> Result<(), String> {
    if let Ok(v) = token.parse::<f64>()
        && v == 0.0
    {
        return Ok(());
    }
    let split_at = token
        .rfind(|c: char| !c.is_ascii_alphabetic())
        .map(|i| i + 1)
        .unwrap_or(0);
    let (number, unit) = token.split_at(split_at);
    if unit.is_empty() {
        return Err(format!("length \"{token}\" is missing a unit"));
    }
    if number.is_empty() {
        return Err(format!("length \"{token}\" is missing a numeric part"));
    }
    match number.parse::<f64>() {
        Ok(v) if v >= 0.0 => {}
        Ok(_) => return Err(format!("length \"{token}\" must not be negative")),
        Err(_) => return Err(format!("\"{number}\" is not a valid CSS number")),
    }
    if !CSS_LENGTH_UNITS
        .iter()
        .any(|u| u.eq_ignore_ascii_case(unit))
    {
        return Err(format!("unknown CSS length unit \"{unit}\" in \"{token}\""));
    }
    Ok(())
}

/// Validates a single (already comma-split and whitespace-trimmed)
/// `w:source-size-list` entry. `is_last` controls whether a missing
/// media-condition prefix is tolerated (only the final entry may be a bare
/// length/function).
fn check_source_size_entry(entry: &str, is_last: bool) -> Result<(), String> {
    if let Some(stripped) = entry.strip_suffix(')') {
        // Entry ends in a CSS math function call — find the matching '(' by
        // walking backward from the end, tracking depth.
        let bytes = stripped.as_bytes();
        let mut depth = 1i32;
        let mut open_idx = None;
        for i in (0..bytes.len()).rev() {
            match bytes[i] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        open_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let open_idx = open_idx
            .ok_or_else(|| format!("mismatched parentheses in source size entry \"{entry}\""))?;
        let before = &stripped[..open_idx];
        let func_name_start = before
            .rfind(|c: char| !c.is_ascii_alphanumeric())
            .map(|i| i + 1)
            .unwrap_or(0);
        let func_name = &before[func_name_start..];
        if !SOURCE_SIZE_MATH_FUNCTIONS
            .iter()
            .any(|f| f.eq_ignore_ascii_case(func_name))
        {
            return Err(format!(
                "unknown CSS math function \"{func_name}\" in source size entry \"{entry}\""
            ));
        }
        let media_condition = before[..func_name_start].trim_matches(is_html_whitespace);
        if media_condition.is_empty() {
            if !is_last {
                return Err(
                    "non-final source size list entry must have a media condition prefix"
                        .to_string(),
                );
            }
        } else if !media_condition_uses_css_math_function(media_condition) {
            crate::datatypes::media_query::check_media_condition_only(media_condition)?;
        }
        return Ok(());
    }

    // Entry ends in a plain CSS <length>: the last whitespace-delimited
    // token is the length, everything before it is the (unvalidated, see
    // `check_source_size_list` doc comment) media-condition fragment.
    let last_ws = entry.rfind(is_html_whitespace);
    let (media_condition, length) = match last_ws {
        Some(i) => (&entry[..i], &entry[i + 1..]),
        None => ("", entry),
    };
    if length.is_empty() {
        return Err(format!("source size list entry \"{entry}\" has no length"));
    }
    check_source_size_length(length)?;
    let media_condition = media_condition.trim_matches(is_html_whitespace);
    if media_condition.is_empty() {
        if !is_last {
            return Err(
                "non-final source size list entry must have a media condition prefix".to_string(),
            );
        }
    } else if !media_condition_uses_css_math_function(media_condition) {
        crate::datatypes::media_query::check_media_condition_only(media_condition)?;
    }
    Ok(())
}

/// `media-query-parse`'s `<mf-value>` (`MfValue`, `src/datatypes/
/// media_query.rs`) has no CSS math-function variant at all (`calc()`,
/// `min()`, etc.) — a real, upstream gap, not something to route around
/// with a hand-rolled parser here. `check_media_condition_only` fails to
/// parse a media-feature value using one, so entries like
/// `(min-width:calc(500px)) 500px` — real, corpus-confirmed valid
/// (`html/elements/picture/picture-isvalid.html`) — are deliberately
/// left unchecked rather than falsely rejected, the same leniency
/// `check_source_size_entry`'s own length-side math-function handling
/// already has to have (see `SOURCE_SIZE_MATH_FUNCTIONS` above).
fn media_condition_uses_css_math_function(media_condition: &str) -> bool {
    let lower = media_condition.to_ascii_lowercase();
    SOURCE_SIZE_MATH_FUNCTIONS
        .iter()
        .any(|name| lower.contains(&format!("{name}(")))
}

fn check_source_size_entries(s: &str) -> Result<(), String> {
    let entries = split_top_level_commas(s);
    let last_index = entries.len() - 1;
    for (idx, entry) in entries.iter().enumerate() {
        let trimmed_entry = strip_surrounding_css_comments(entry);
        if trimmed_entry.is_empty() {
            return Err("source size list entry must not be empty".to_string());
        }
        check_source_size_entry(trimmed_entry, idx == last_index)?;
    }
    Ok(())
}

/// Strips CSS comments (`/* ... */`, non-nesting per CSS Syntax Module
/// Level 3) that appear at the very start and/or end of `s` (interspersed
/// with HTML whitespace), leaving any comment in the *interior* of `s`
/// untouched as literal characters.
///
/// Deliberately narrower than "strip every comment anywhere": a comment
/// between two otherwise-adjacent characters is a genuine CSS token
/// boundary (per the CSS tokenizer's `consume-comment`, run between
/// tokens) — `+/**/50vw` tokenizes as the two tokens `+` and `50vw`, not
/// the single number `+50vw`. This crate doesn't tokenize the CSS length
/// grammar in full (see `check_source_size_length`'s simpler split-on-
/// trailing-unit approach), so an *interior* comment is left alone and the
/// entry fails to parse as a single token — matching vnu's own rejection
/// of `html/elements/picture/sizes-microsyntax-css-comment-after-plus-
/// novalid.html` (`sizes='+/**/50vw'`, expected an error). Only the
/// leading/trailing cases are unambiguous regardless of tokenization
/// depth (nothing precedes/follows the comment to accidentally fuse with),
/// which is also exactly what vnu's own corpus names as distinct,
/// individually-tested cases (`sizes-microsyntax-leading-css-comment`/
/// `-trailing-css-comment`, both in `html/elements/picture/picture-
/// isvalid.html`, expected clean).
fn strip_surrounding_css_comments(s: &str) -> &str {
    let mut s = s.trim_matches(is_html_whitespace);
    loop {
        if let Some(rest) = s.strip_prefix("/*") {
            match rest.find("*/") {
                Some(end) => {
                    s = rest[end + 2..].trim_matches(is_html_whitespace);
                    continue;
                }
                None => {
                    // Unterminated comment: EOF ends it (`consume-comment`),
                    // consuming the rest of the entry.
                    return "";
                }
            }
        }
        if let Some(before) = s.strip_suffix("*/")
            && let Some(start) = before.rfind("/*")
        {
            s = before[..start].trim_matches(is_html_whitespace);
            continue;
        }
        break;
    }
    s
}

/// `w:source-size-list` → `SourceSizeList.java` (`plan/05c-research-group-b.md`
/// item 26): the `sizes=""` microsyntax on `<img>`/`<source>` for responsive
/// images — a comma-separated list of `(media-condition) length` entries,
/// where the final entry may omit its media condition, plus the
/// not-yet-stabilized `sizes="auto[, ...]"` feature.
///
/// **Known, documented, temporary limitation** (see
/// `plan/05c-datatype-library.md`'s "Risiken" section and
/// `plan/05c-research-group-b.md` item 26): vnu delegates the actual
/// media-condition grammar of each entry to its embedded CSS engine's
/// `MediaCondition` parser. `media-query-parse` (the sister project this
/// gap was waiting on) is now published (see `plan/DECISIONS.md`, the
/// `w:media-query` entries) and used for the separate `w:media-query`
/// datatype (`src/datatypes/media_query.rs`) — but it exposes no standalone
/// `<media-condition>`-fragment parser (only `parse_media_query`/
/// `parse_media_query_list`, which parse the full `<media-query>` grammar
/// including the optional `only`/`not`/media-type prefix this microsyntax's
/// bracketed condition doesn't have), so wiring it in here would need
/// either a synthetic-wrapper workaround or a new upstream entry point —
/// deliberately not done in this pass, to keep that change scoped and
/// reviewable on its own. Until it is, this function only checks that a
/// non-final entry's media-condition prefix is *present* (i.e. non-empty
/// after trimming); it does **not** validate the media-condition syntax
/// itself. This is an accepted, cited gap for this phase, not a silently
/// incomplete implementation.
pub(crate) fn check_source_size_list(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("source size list must not be empty".to_string());
    }

    let trimmed = value.trim_matches(is_html_whitespace);
    if trimmed.is_empty() {
        return Err("source size list must not be empty".to_string());
    }

    if let Some(prefix) = trimmed.get(..4)
        && prefix.eq_ignore_ascii_case("auto")
    {
        let after_auto = trimmed[4..].trim_start_matches(is_html_whitespace);
        if after_auto.is_empty() {
            // Bare "auto" (nothing, or only trailing whitespace, follows).
            return Ok(());
        }
        if let Some(rest) = after_auto.strip_prefix(',') {
            return check_source_size_entries(rest);
        }
        // "auto" was followed by something other than a comma (e.g.
        // "autofoo") — not the auto keyword; fall through and treat the
        // whole original literal as the entries list.
    }
    check_source_size_entries(trimmed)
}

fn is_descriptor_with_comma(s: &str) -> bool {
    if let Some(num) = s.strip_suffix('w').or_else(|| s.strip_suffix('W')) {
        !num.is_empty() && num.chars().all(|c| c.is_ascii_digit())
    } else if let Some(num) = s.strip_suffix('x').or_else(|| s.strip_suffix('X')) {
        num.parse::<f64>().is_ok()
    } else {
        false
    }
}

fn parse_srcset_candidates(value: &str) -> Result<Vec<(&str, Option<&str>)>, String> {
    let trimmed = value.trim_matches(is_html_whitespace);
    if trimmed.is_empty() {
        return Err("srcset attribute value must not be empty".to_string());
    }
    if trimmed.starts_with(',') || trimmed.ends_with(',') || trimmed.contains(",,") {
        return Err("srcset contains empty candidate string".to_string());
    }

    let mut candidates = Vec::new();
    let raw_tokens: Vec<&str> = trimmed
        .split(is_html_whitespace)
        .filter(|s| !s.is_empty())
        .collect();

    let mut tokens = Vec::new();
    for t in raw_tokens {
        if let Some(pos) = t.find(',')
            && t != ","
            && !t.starts_with(',')
            && !t.ends_with(',')
            && is_descriptor_with_comma(&t[..pos])
        {
            tokens.push(&t[..=pos]);
            tokens.push(&t[pos + 1..]);
        } else {
            tokens.push(t);
        }
    }

    let mut i = 0;
    while i < tokens.len() {
        let mut token = tokens[i];
        i += 1;

        if token == "," {
            return Err("empty candidate string".to_string());
        }

        let mut comma_after = false;
        if token.ends_with(',') {
            token = token.trim_end_matches(',');
            if token.is_empty() {
                return Err("empty candidate string".to_string());
            }
            comma_after = true;
        }

        let url = token;

        if comma_after || i >= tokens.len() {
            candidates.push((url, None));
            continue;
        }

        let next_token = tokens[i];
        if next_token == "," {
            i += 1;
            candidates.push((url, None));
            continue;
        }

        if next_token.starts_with(',') {
            return Err("empty candidate string".to_string());
        }

        let mut desc = next_token;
        i += 1;

        if desc.ends_with(',') {
            desc = desc.trim_end_matches(',');
            if desc.is_empty() {
                return Err("empty candidate string".to_string());
            }
        }

        candidates.push((url, Some(desc)));
    }

    Ok(candidates)
}

/// `w:image-candidate-strings` (`ImageCandidateStrings.java` in vnu).
pub(crate) fn check_image_candidate_strings(value: &str) -> Result<(), String> {
    let candidates = parse_srcset_candidates(value)?;
    let mut descriptors = Vec::new();
    let mut has_w = false;
    let mut has_x = false;

    for (url_str, desc_opt) in candidates {
        if url_str.is_empty() {
            return Err("empty candidate string".to_string());
        }
        super::network::check_iri_ref(url_str)?;

        let descriptor = if let Some(desc) = desc_opt {
            // Case-sensitive per the WHATWG srcset microsyntax ("parsing a
            // srcset attribute" checks for the literal lowercase
            // characters "w"/"x", nothing else) — `srcset-microsyntax-
            // uppercase-w-novalid.html` (`srcset="x 1W"`) confirms
            // uppercase is rejected, not tolerated.
            if let Some(num_str) = desc.strip_suffix('w') {
                if num_str.starts_with('+')
                    || num_str.starts_with('-')
                    || num_str.contains('.')
                    || num_str.contains('e')
                    || num_str.contains('E')
                {
                    return Err(format!("invalid width descriptor: `{desc}`"));
                }
                let val: u64 = num_str
                    .parse()
                    .map_err(|_| format!("invalid width descriptor: `{desc}`"))?;
                if val == 0 {
                    return Err("width descriptor must be greater than zero".to_string());
                }
                has_w = true;
                format!("{val}w")
            } else if let Some(num_str) = desc.strip_suffix('x') {
                if num_str.starts_with('+')
                    || num_str.starts_with('-')
                    || num_str.eq_ignore_ascii_case("nan")
                    || num_str.eq_ignore_ascii_case("infinity")
                {
                    return Err(format!("invalid pixel density descriptor: `{desc}`"));
                }
                let val: f64 = num_str
                    .parse()
                    .map_err(|_| format!("invalid pixel density descriptor: `{desc}`"))?;
                if val <= 0.0 {
                    return Err("pixel density descriptor must be greater than zero".to_string());
                }
                has_x = true;
                format!("{val}x")
            } else {
                return Err(format!("unrecognized descriptor unit: `{desc}`"));
            }
        } else {
            has_x = true;
            "1x".to_string()
        };

        if descriptors.contains(&descriptor) {
            return Err(format!("duplicate descriptor: `{descriptor}`"));
        }
        descriptors.push(descriptor);
    }

    if has_w && has_x {
        return Err(
            "cannot mix width (w) and pixel density (x) descriptors in the same srcset".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_candidate_strings_lowercase_w_valid() {
        assert!(check_image_candidate_strings("x 100w, y 200w").is_ok());
    }

    #[test]
    fn image_candidate_strings_uppercase_w_invalid() {
        assert!(check_image_candidate_strings("x 1W").is_err());
    }

    #[test]
    fn image_candidate_strings_uppercase_x_invalid() {
        assert!(check_image_candidate_strings("x 2X").is_err());
    }

    #[test]
    fn image_candidate_strings_lowercase_x_valid() {
        assert!(check_image_candidate_strings("x 2x").is_ok());
    }

    #[test]
    fn email_address_accepts_empty() {
        assert!(check_email_address("").is_ok());
    }

    #[test]
    fn email_address_accepts_garbage() {
        assert!(check_email_address("not an email @@ at all").is_ok());
    }

    #[test]
    fn email_address_list_accepts_empty() {
        assert!(check_email_address_list("").is_ok());
    }

    #[test]
    fn email_address_list_accepts_garbage() {
        assert!(check_email_address_list("not an email @@ at all, also not").is_ok());
    }

    #[test]
    fn color_accepts_empty() {
        assert!(check_color("").is_ok());
    }

    #[test]
    fn color_accepts_garbage() {
        assert!(check_color("not-a-color").is_ok());
    }

    #[test]
    fn circle_valid() {
        assert!(check_circle("10,20,5").is_ok());
    }

    #[test]
    fn circle_wrong_part_count_invalid() {
        assert!(check_circle("10,20").is_err());
        assert!(check_circle("10,20,5,1").is_err());
    }

    #[test]
    fn circle_negative_radius_invalid() {
        assert!(check_circle("10,20,-5").is_err());
    }

    #[test]
    fn circle_negative_x_y_valid() {
        assert!(check_circle("-10,-20,5").is_ok());
    }

    #[test]
    fn polyline_valid_six_parts() {
        assert!(check_polyline("1,2,3,4,5,6").is_ok());
    }

    #[test]
    fn polyline_too_few_parts_invalid() {
        assert!(check_polyline("1,2,3,4").is_err());
    }

    #[test]
    fn polyline_odd_count_at_or_above_six_invalid() {
        assert!(check_polyline("1,2,3,4,5,6,7").is_err());
    }

    #[test]
    fn polyline_valid_eight_parts() {
        assert!(check_polyline("1,2,3,4,5,6,7,8").is_ok());
    }

    #[test]
    fn rectangle_valid() {
        assert!(check_rectangle("0,0,10,10").is_ok());
    }

    #[test]
    fn rectangle_left_ge_right_invalid() {
        assert!(check_rectangle("10,0,10,10").is_err());
        assert!(check_rectangle("20,0,10,10").is_err());
    }

    #[test]
    fn rectangle_top_ge_bottom_invalid() {
        assert!(check_rectangle("0,10,10,10").is_err());
        assert!(check_rectangle("0,20,10,10").is_err());
    }

    #[test]
    fn rectangle_non_integer_part_invalid() {
        assert!(check_rectangle("0,0,10,foo").is_err());
    }

    #[test]
    fn source_size_list_bare_auto_valid() {
        assert!(check_source_size_list("auto").is_ok());
    }

    #[test]
    fn source_size_list_auto_with_fallback_valid() {
        assert!(check_source_size_list("auto, 100vw").is_ok());
    }

    #[test]
    fn source_size_list_bare_final_length_valid() {
        assert!(check_source_size_list("480px").is_ok());
    }

    #[test]
    fn source_size_list_two_entries_valid() {
        assert!(check_source_size_list("(min-width: 600px) 480px, 800px").is_ok());
    }

    #[test]
    fn source_size_list_bare_media_type_invalid() {
        // "all" is a <media-type>, not a <media-condition> — sizes'
        // per-entry grammar has no <media-type> branch at all.
        assert!(check_source_size_list("all 500px, 100vw").is_err());
    }

    #[test]
    fn source_size_list_media_type_and_condition_invalid() {
        assert!(check_source_size_list("all and (min-width:500px) 500px, 100vw").is_err());
    }

    #[test]
    fn source_size_list_general_enclosed_invalid() {
        assert!(check_source_size_list("(123) 500px, 100vw").is_err());
    }

    #[test]
    fn source_size_list_media_condition_missing_parens_invalid() {
        assert!(check_source_size_list("min-width:500px 500px, 100vw").is_err());
    }

    #[test]
    fn source_size_list_media_condition_syntax_error_invalid() {
        assert!(check_source_size_list("(}) 500px, 100vw").is_err());
    }

    #[test]
    fn source_size_list_media_condition_empty_feature_value_invalid() {
        assert!(check_source_size_list("(min-width:) 800px, 320px").is_err());
    }

    #[test]
    fn source_size_list_non_final_missing_media_condition_invalid() {
        assert!(check_source_size_list("480px, 800px").is_err());
    }

    #[test]
    fn source_size_list_calc_entry_valid() {
        assert!(check_source_size_list("calc(100vw - 20px)").is_ok());
    }

    #[test]
    fn source_size_list_unknown_unit_invalid() {
        assert!(check_source_size_list("480xyz").is_err());
    }

    #[test]
    fn source_size_list_negative_length_invalid() {
        assert!(check_source_size_list("-10px").is_err());
    }

    #[test]
    fn source_size_list_empty_invalid() {
        assert!(check_source_size_list("").is_err());
    }

    #[test]
    fn source_size_list_leading_css_comment_valid() {
        assert!(check_source_size_list("/**/50vw").is_ok());
    }

    #[test]
    fn source_size_list_trailing_css_comment_valid() {
        assert!(check_source_size_list("50vw/**/").is_ok());
    }

    #[test]
    fn source_size_list_interior_css_comment_invalid() {
        // html/elements/picture/sizes-microsyntax-css-comment-after-plus-
        // novalid.html: a comment strictly *between* two other characters
        // is a real CSS token boundary (splits `+` from `50vw`), not
        // something to silently paper over — the sign and the number end
        // up as separate tokens, which is not a valid `<length>`.
        assert!(check_source_size_list("+/**/50vw").is_err());
    }
}
