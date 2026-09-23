//! Structural/list `w:*` datatypes for the RELAX NG datatype library
//! `http://whattf.org/datatype-draft` (see `plan/05c-datatype-library.md`,
//! item 4/16/17/18/20/22/23/24/25/26 in `plan/05c-research-group-a.md`).
//!
//! vnu-parity is the default for every check in this file: quirks in vnu's
//! actual behavior (`validator/validator`, `src/nu/validator/datatype/`) are
//! replicated deliberately, not "fixed" (see `plan/05c-datatype-library.md`,
//! "Verbindliches Prinzip: vnu-Parität als Default"). Deviations are called
//! out explicitly at the relevant function.
//!
//! Nothing calls these yet (Phase 05c builds the library in isolation before
//! Phase 05d wires it into `Schema::validate()`).

use url::Url;

/// vnu's shared `isWhitespace(char)` helper: exactly these five ASCII
/// characters, not Rust's `char::is_whitespace()` (which is full Unicode
/// whitespace and matches a different set). See
/// `plan/05c-research-group-a.md`, top of file.
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\x0c' | '\n' | '\r')
}

/// Tokenizes on runs of vnu's five-character whitespace set, dropping empty
/// tokens (so leading/trailing/repeated separators do not produce spurious
/// empty tokens).
fn split_ws_set(value: &str) -> impl Iterator<Item = &str> {
    value.split(is_whitespace).filter(|token| !token.is_empty())
}

/// `w:custom-element-name` (`CustomElementName.java`).
pub(crate) fn check_custom_element_name(value: &str) -> Result<(), String> {
    const PROHIBITED_NAMES: &[&str] = &[
        "annotation-xml",
        "color-profile",
        "font-face",
        "font-face-format",
        "font-face-name",
        "font-face-src",
        "font-face-uri",
        "missing-glyph",
    ];

    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => {
            return Err(format!(
                "custom element name must start with a lowercase ASCII letter: {value:?}"
            ));
        }
    }

    let mut has_hyphen = false;
    for c in chars {
        if c == '-' {
            has_hyphen = true;
            continue;
        }
        if c.is_ascii_uppercase() || is_whitespace(c) || c == '\0' || c == '/' || c == '>' {
            return Err(format!(
                "invalid character {c:?} in custom element name: {value:?}"
            ));
        }
    }

    if !has_hyphen {
        return Err(format!(
            "custom element name must contain a hyphen: {value:?}"
        ));
    }

    if PROHIBITED_NAMES.contains(&value) {
        return Err(format!(
            "custom element name is a reserved built-in name: {value:?}"
        ));
    }

    Ok(())
}

/// `w:autocomplete-any` (`AutocompleteDetailsAny.java` /
/// `AbstractAutocompleteDetails.java`).
pub(crate) fn check_autocomplete_any(value: &str) -> Result<(), String> {
    const CONTACT_TYPES: &[&str] = &["home", "work", "mobile", "fax", "pager"];
    const FIELD_NAMES: &[&str] = &[
        "name",
        "honorific-prefix",
        "given-name",
        "additional-name",
        "family-name",
        "honorific-suffix",
        "nickname",
        "username",
        "new-password",
        "current-password",
        "one-time-code",
        "organization-title",
        "organization",
        "street-address",
        "address-line1",
        "address-line2",
        "address-line3",
        "address-level4",
        "address-level3",
        "address-level2",
        "address-level1",
        "country",
        "country-name",
        "postal-code",
        "cc-name",
        "cc-given-name",
        "cc-additional-name",
        "cc-family-name",
        "cc-number",
        "cc-exp",
        "cc-exp-month",
        "cc-exp-year",
        "cc-csc",
        "cc-type",
        "transaction-currency",
        "transaction-amount",
        "language",
        "bday",
        "bday-day",
        "bday-month",
        "bday-year",
        "sex",
        "url",
        "photo",
        "tel",
        "tel-country-code",
        "tel-national",
        "tel-area-code",
        "tel-local",
        "tel-local-prefix",
        "tel-local-suffix",
        "tel-extension",
        "email",
        "impp",
    ];

    let trimmed = value.trim_matches(is_whitespace);
    if trimmed.is_empty() {
        return Err("autocomplete value must not be empty".to_string());
    }

    let tokens: Vec<String> = split_ws_set(trimmed).map(str::to_ascii_lowercase).collect();

    let mut idx = 0;
    if idx < tokens.len() && tokens[idx].starts_with("section-") {
        idx += 1;
    }
    if idx < tokens.len() && (tokens[idx] == "shipping" || tokens[idx] == "billing") {
        idx += 1;
    }
    if idx < tokens.len() && CONTACT_TYPES.contains(&tokens[idx].as_str()) {
        idx += 1;
    }

    let remaining = &tokens[idx..];
    if remaining.is_empty() {
        return Err("autocomplete value has no field-name token".to_string());
    }

    let last = remaining.len() - 1;
    let mut field_name_count = 0;
    for (i, token) in remaining.iter().enumerate() {
        if token == "webauthn" {
            if i != last {
                return Err("\"webauthn\" is only valid as the sole or last token".to_string());
            }
            continue;
        }
        if token.starts_with("section-")
            || token == "shipping"
            || token == "billing"
            || CONTACT_TYPES.contains(&token.as_str())
        {
            return Err(format!(
                "token {token:?} is only valid earlier in the autocomplete sequence"
            ));
        }
        if !FIELD_NAMES.contains(&token.as_str()) {
            return Err(format!("unknown autocomplete field name: {token:?}"));
        }
        field_name_count += 1;
    }

    if field_name_count == 0 && (tokens.len() != 1 || tokens[0] != "webauthn") {
        return Err("autocomplete value has no field-name token".to_string());
    }
    if field_name_count > 1 {
        return Err("autocomplete value has more than one field-name token".to_string());
    }

    Ok(())
}

/// `w:browsing-context` (`BrowsingContext.java`).
pub(crate) fn check_browsing_context(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("browsing context name must not be empty".to_string());
    }
    if value.starts_with('_') {
        return Err("browsing context name must not start with '_'".to_string());
    }
    Ok(())
}

/// `w:browsing-context-or-keyword` (`BrowsingContextOrKeyword.java`).
pub(crate) fn check_browsing_context_or_keyword(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("browsing context name or keyword must not be empty".to_string());
    }
    if let Some(rest) = value.strip_prefix('_') {
        match rest.to_ascii_lowercase().as_str() {
            "blank" | "self" | "top" | "parent" => Ok(()),
            _ => Err(format!(
                "browsing context keyword must be one of _blank/_self/_top/_parent: {value:?}"
            )),
        }
    } else {
        Ok(())
    }
}

/// `w:keylabellist` (`KeyLabelList.java`).
///
/// vnu splits on Java's `\s+` regex, which (unlike this implementation's
/// `str::split_whitespace()`) leaves a phantom empty leading token when
/// `value` starts with whitespace: Java's `String.split` with the default
/// `limit = 0` keeps a leading empty match but drops trailing empty matches.
/// That phantom token is a documented vnu quirk (`plan/05c-research-group-a.md`,
/// item 20) — it silently passes the "exactly one codepoint" check (an empty
/// token's length never triggers the ">1 codepoint" rejection) and it can
/// never collide in the duplicate check (only one leading-whitespace run is
/// possible, so at most one phantom token ever exists). Its presence or
/// absence therefore never changes whether a given `value` as a whole is
/// accepted or rejected. We deliberately use `split_whitespace()` (which
/// produces no empty tokens at all) instead of hand-rolling the Java-`\s+`
/// equivalent split: it yields byte-identical accept/reject outcomes without
/// the extra bookkeeping needed to reproduce the phantom token itself.
pub(crate) fn check_keylabellist(value: &str) -> Result<(), String> {
    let mut seen: Vec<&str> = Vec::new();
    for token in value.split_whitespace() {
        if token.chars().count() != 1 {
            return Err(format!(
                "key label token must be exactly one character: {token:?}"
            ));
        }
        if seen.contains(&token) {
            return Err(format!("duplicate key label token: {token:?}"));
        }
        seen.push(token);
    }
    Ok(())
}

/// `w:rel-value` (`RelValue.java`). The full IANA link-relations registry
/// (plus `"sitemap"`) vnu's `RelValue.java` checks tokens longer than 3
/// characters against — copied verbatim from its `registeredValues` set
/// (`RelValue.java`, `validator/validator@388cb36`), not a "representative
/// slice": [`find_closest_rel_typo`] below needs the exact same candidate
/// set vnu suggests typo corrections from, or a correction this crate
/// offers could differ from vnu's (different message text isn't compared
/// by the differential test, but a *wrong* candidate — or none at all —
/// would still under- or over-fire relative to vnu).
const LINK_RELATIONS: &[&str] = &[
    "about",
    "acl",
    "alternate",
    "amphtml",
    "api-catalog",
    "appendix",
    "apple-touch-icon",
    "apple-touch-startup-image",
    "archives",
    "author",
    "blocked-by",
    "bookmark",
    "c2pa-manifest",
    "canonical",
    "chapter",
    "cite-as",
    "collection",
    "compression-dictionary",
    "contents",
    "convertedfrom",
    "copyright",
    "create-form",
    "current",
    "deprecation",
    "describedby",
    "describes",
    "disclosure",
    "dns-prefetch",
    "duplicate",
    "edit",
    "edit-form",
    "edit-media",
    "enclosure",
    "external",
    "first",
    "geofeed",
    "glossary",
    "help",
    "hosts",
    "hub",
    "ice-server",
    "icon",
    "index",
    "intervalafter",
    "intervalbefore",
    "intervalcontains",
    "intervaldisjoint",
    "intervalduring",
    "intervalequals",
    "intervalfinishedby",
    "intervalfinishes",
    "intervalin",
    "intervalmeets",
    "intervalmetby",
    "intervaloverlappedby",
    "intervaloverlaps",
    "intervalstartedby",
    "intervalstarts",
    "item",
    "last",
    "latest-version",
    "license",
    "linkset",
    "lrdd",
    "manifest",
    "mask-icon",
    "me",
    "media-feed",
    "memento",
    "micropub",
    "modulepreload",
    "monitor",
    "monitor-group",
    "next",
    "next-archive",
    "nofollow",
    "noopener",
    "noreferrer",
    "opener",
    "openid2.local_id",
    "openid2.provider",
    "original",
    "p3pv1",
    "payment",
    "pingback",
    "preconnect",
    "predecessor-version",
    "prefetch",
    "preload",
    "prerender",
    "prev",
    "prev-archive",
    "preview",
    "previous",
    "privacy-policy",
    "profile",
    "publication",
    "rdap-active",
    "rdap-bottom",
    "rdap-down",
    "rdap-top",
    "rdap-up",
    "related",
    "replies",
    "restconf",
    "ruleinput",
    "search",
    "section",
    "self",
    "service",
    "service-desc",
    "service-doc",
    "service-meta",
    "sip-trunking-capability",
    "sitemap",
    "sponsored",
    "start",
    "status",
    "stylesheet",
    "subsection",
    "successor-version",
    "sunset",
    "tag",
    "terms-of-service",
    "timegate",
    "timemap",
    "type",
    "ugc",
    "up",
    "version-history",
    "via",
    "webmention",
    "working-copy",
    "working-copy-of",
];

/// vnu's `RelValue.java` `TYPO_THRESHOLD`: the maximum Levenshtein distance
/// (inclusive, and never zero — an exact match isn't a typo) a token may
/// have from a [`LINK_RELATIONS`] entry to be flagged as a likely typo.
const REL_TYPO_MAX_DISTANCE: usize = 2;

/// `w:rel-value` (`RelValue.java`)'s typo heuristic: the closest
/// [`LINK_RELATIONS`] entry to `token` (already lowercased, colon-stripped)
/// by Levenshtein distance, if one is close enough to plausibly be a typo
/// — `None` otherwise (including when `token` is itself an exact match,
/// distance 0). Ports `RelValue.java`'s `findClosestMatch` verbatim:
/// candidates of length ≤ 3 are skipped (too short to meaningfully bound a
/// typo distance), the distance must be in `1..=REL_TYPO_MAX_DISTANCE`,
/// the length difference must be at most 2, and — to avoid nonsense
/// suggestions like `"cite"` → `"item"` — the candidate must share either
/// its first or its last character with `token`. Ties keep the
/// first-found (lowest) distance, matching `findClosestMatch`'s
/// strict-`<`-only update.
fn find_closest_rel_typo(token: &str) -> Option<&'static str> {
    let mut best: Option<(&'static str, usize)> = None;
    for &candidate in LINK_RELATIONS {
        if candidate.len() <= 3 {
            continue;
        }
        let distance = levenshtein_distance(token, candidate);
        if distance == 0 || distance > REL_TYPO_MAX_DISTANCE {
            continue;
        }
        if token.len().abs_diff(candidate.len()) > 2 {
            continue;
        }
        let same_start = token.as_bytes().first() == candidate.as_bytes().first();
        let same_end = token.as_bytes().last() == candidate.as_bytes().last();
        if !same_start && !same_end {
            continue;
        }
        if best.is_none_or(|(_, best_distance)| distance < best_distance) {
            best = Some((candidate, distance));
        }
    }
    best.map(|(candidate, _)| candidate)
}

/// Plain Levenshtein (single-character insert/delete/substitute) edit
/// distance between two ASCII-lowercase strings, byte-wise (every
/// [`LINK_RELATIONS`] entry and every rel-value token this is called on is
/// ASCII, so operating on bytes rather than `char`s changes nothing here).
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut previous_row: Vec<usize> = (0..=b.len()).collect();
    let mut current_row = vec![0usize; b.len() + 1];
    for (i, &a_byte) in a.iter().enumerate() {
        current_row[0] = i + 1;
        for (j, &b_byte) in b.iter().enumerate() {
            let cost = usize::from(a_byte != b_byte);
            current_row[j + 1] = (previous_row[j + 1] + 1)
                .min(current_row[j] + 1)
                .min(previous_row[j] + cost);
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }
    previous_row[b.len()]
}

/// Ports `RelValue.java`'s duplicate check, length-3-or-under exemption,
/// exact-match acceptance, and Levenshtein-distance typo hint (see
/// [`find_closest_rel_typo`]) — the only difference from vnu being that
/// this crate has no separate info/warning-severity channel yet, so a
/// typo hint (vnu: always non-fatal, `newDatatypeException(..., true)`)
/// surfaces as a hard `Err` here rather than a softer diagnostic. Since
/// the differential test only checks *whether* `check()` found something,
/// not its severity, this still matches vnu's pass/fail verdict.
pub(crate) fn check_rel_value(value: &str) -> Result<(), String> {
    let mut seen: Vec<&str> = Vec::new();
    for token in split_ws_set(value) {
        let stripped = token.strip_prefix(':').unwrap_or(token);
        if seen.contains(&stripped) {
            return Err(format!("duplicate rel-value token: {stripped:?}"));
        }
        seen.push(stripped);

        if stripped.len() <= 3 {
            continue;
        }
        let lower = stripped.to_ascii_lowercase();
        if LINK_RELATIONS.contains(&lower.as_str()) {
            continue;
        }
        if let Some(closest) = find_closest_rel_typo(&lower) {
            return Err(format!(
                "rel-value {stripped:?} looks like a typo for {closest:?}"
            ));
        }
    }
    Ok(())
}

/// `w:sandbox-allow-list` (`SandboxAllowList.java`).
pub(crate) fn check_sandbox_allow_list(value: &str) -> Result<(), String> {
    const SANDBOX_KEYWORDS: &[&str] = &[
        "allow-downloads",
        "allow-forms",
        "allow-modals",
        "allow-orientation-lock",
        "allow-pointer-lock",
        "allow-popups",
        "allow-popups-to-escape-sandbox",
        "allow-presentation",
        "allow-same-origin",
        "allow-scripts",
        "allow-top-navigation",
        "allow-top-navigation-by-user-activation",
        "allow-top-navigation-to-custom-protocols",
    ];

    let mut seen: Vec<String> = Vec::new();
    for token in split_ws_set(value) {
        let lower = token.to_ascii_lowercase();
        if seen.iter().any(|s| s == &lower) {
            return Err(format!("duplicate sandbox keyword: {lower:?}"));
        }
        if !SANDBOX_KEYWORDS.contains(&lower.as_str()) {
            return Err(format!("unknown sandbox keyword: {lower:?}"));
        }
        seen.push(lower);
    }

    let has = |keyword: &str| seen.iter().any(|s| s == keyword);

    // vnu-parity default: `SandboxAllowList.java`'s `WARN` system property
    // defaults to `false`, and for this specific combination that makes it a
    // *hard* error by default (not softened to a warning) — see
    // `plan/05c-research-group-a.md`, item 23.
    if has("allow-scripts") && has("allow-same-origin") {
        return Err("sandbox must not combine allow-scripts with allow-same-origin".to_string());
    }
    // No `WARN` gate at all for this combination — always a hard error.
    if has("allow-top-navigation") && has("allow-top-navigation-by-user-activation") {
        return Err(
            "sandbox must not combine allow-top-navigation with allow-top-navigation-by-user-activation"
                .to_string(),
        );
    }

    Ok(())
}

/// MIME token character set shared by `w:script-type` (and, structurally, by
/// vnu's `MimeType.java`, which `ScriptType.java` extends): ASCII 33-126
/// excluding `( ) < > @ , ; : \ " / [ ] ? = { }`.
fn is_mime_token_char(c: char) -> bool {
    matches!(c as u32, 33..=126)
        && !matches!(
            c,
            '(' | ')'
                | '<'
                | '>'
                | '@'
                | ','
                | ';'
                | ':'
                | '\\'
                | '"'
                | '/'
                | '['
                | ']'
                | '?'
                | '='
                | '{'
                | '}'
        )
}

fn take_mime_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut token = String::new();
    while let Some(&c) = chars.peek() {
        if is_mime_token_char(c) {
            token.push(c);
            chars.next();
        } else {
            break;
        }
    }
    token
}

/// `w:script-type` (`ScriptType.java extends MimeType.java`).
///
/// Implements the `type "/" subtype *( ";" parameter )` MIME grammar itself
/// (not shared with the `network` batch's `w:mime-type`, which owns its own
/// copy — see the task brief for item 24), plus the JavaScript-MIME-type
/// special case: for the fixed set of JS type/subtype pairs, no trailing
/// parameters are allowed at all.
pub(crate) fn check_script_type(value: &str) -> Result<(), String> {
    const JS_MIME_TYPES: &[&str] = &[
        "application/ecmascript",
        "application/javascript",
        "application/x-ecmascript",
        "application/x-javascript",
        "text/ecmascript",
        "text/javascript",
        "text/javascript1.0",
        "text/javascript1.1",
        "text/javascript1.2",
        "text/javascript1.3",
        "text/javascript1.4",
        "text/javascript1.5",
        "text/jscript",
        "text/livescript",
        "text/x-ecmascript",
        "text/x-javascript",
    ];

    let mut chars = value.chars().peekable();

    let media_type = take_mime_token(&mut chars);
    if media_type.is_empty() {
        return Err(format!("script type is missing a MIME type: {value:?}"));
    }
    if chars.next() != Some('/') {
        return Err(format!("script type is missing '/': {value:?}"));
    }
    let subtype = take_mime_token(&mut chars);
    if subtype.is_empty() {
        return Err(format!("script type is missing a subtype: {value:?}"));
    }

    let full_type = format!("{media_type}/{subtype}");
    let is_js_type = JS_MIME_TYPES.contains(&full_type.as_str());

    if chars.peek().is_none() {
        return Ok(());
    }

    if is_js_type {
        return Err(format!(
            "JavaScript script type {full_type:?} must not have parameters: {value:?}"
        ));
    }

    loop {
        match chars.next() {
            Some(';') => {}
            Some(c) => {
                return Err(format!(
                    "unexpected character {c:?} in script type: {value:?}"
                ));
            }
            None => break,
        }
        while chars.peek().is_some_and(|&c| is_whitespace(c)) {
            chars.next();
        }

        let name = take_mime_token(&mut chars);
        if name.is_empty() {
            return Err(format!(
                "script type parameter is missing a name: {value:?}"
            ));
        }
        if chars.next() != Some('=') {
            return Err(format!("script type parameter is missing '=': {value:?}"));
        }

        if chars.peek() == Some(&'"') {
            chars.next();
            loop {
                match chars.next() {
                    Some('\\') => {
                        if chars.next().is_none() {
                            return Err(format!(
                                "script type parameter has a dangling escape: {value:?}"
                            ));
                        }
                    }
                    Some('"') => break,
                    Some(_) => {}
                    None => {
                        return Err(format!(
                            "script type parameter has an unterminated quoted string: {value:?}"
                        ));
                    }
                }
            }
        } else {
            let param_value = take_mime_token(&mut chars);
            if param_value.is_empty() {
                return Err(format!(
                    "script type parameter is missing a value: {value:?}"
                ));
            }
        }

        while chars.peek().is_some_and(|&c| is_whitespace(c)) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
    }

    Ok(())
}

/// `w:microdata-property` (`MicrodataProperty.java extends Iri extends
/// IriRef`).
///
/// Depends on absolute-URL validation. `Iri`/`IriRef` are full RFC-3987
/// parsers not otherwise ported by this batch; the `url` crate (added as a
/// dependency by a concurrent task) provides an equivalent WHATWG-URL-based
/// absolute-URL check via `Url::parse`.
pub(crate) fn check_microdata_property(value: &str) -> Result<(), String> {
    if value.contains('.') || value.contains(':') {
        if Url::parse(value).is_ok() {
            Ok(())
        } else {
            Err(format!(
                "microdata property containing '.' or ':' must be an absolute URL: {value:?}"
            ))
        }
    } else {
        Ok(())
    }
}

/// `w:simple-color` (`SimpleColor.java`).
pub(crate) fn check_simple_color(value: &str) -> Result<(), String> {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() != 7 {
        return Err(format!(
            "simple color must be exactly 7 characters: {value:?}"
        ));
    }
    if chars[0] != '#' {
        return Err(format!("simple color must start with '#': {value:?}"));
    }
    if !chars[1..].iter().all(char::is_ascii_hexdigit) {
        return Err(format!(
            "simple color must be '#' followed by 6 hex digits: {value:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_element_name_accepts_valid_name() {
        assert!(check_custom_element_name("my-element").is_ok());
    }

    #[test]
    fn custom_element_name_rejects_missing_hyphen() {
        assert!(check_custom_element_name("myelement").is_err());
    }

    #[test]
    fn custom_element_name_rejects_uppercase() {
        assert!(check_custom_element_name("my-Element").is_err());
    }

    #[test]
    fn custom_element_name_rejects_reserved_name() {
        assert!(check_custom_element_name("font-face").is_err());
    }

    #[test]
    fn autocomplete_any_accepts_simple_field_name() {
        assert!(check_autocomplete_any("email").is_ok());
    }

    #[test]
    fn autocomplete_any_accepts_webauthn_alone() {
        assert!(check_autocomplete_any("webauthn").is_ok());
    }

    #[test]
    fn autocomplete_any_rejects_webauthn_not_last() {
        assert!(check_autocomplete_any("webauthn name").is_err());
    }

    #[test]
    fn autocomplete_any_rejects_unknown_field_name() {
        assert!(check_autocomplete_any("not-a-field").is_err());
    }

    #[test]
    fn autocomplete_any_rejects_two_field_names() {
        assert!(check_autocomplete_any("name email").is_err());
    }

    #[test]
    fn browsing_context_or_keyword_accepts_blank() {
        assert!(check_browsing_context_or_keyword("_blank").is_ok());
    }

    #[test]
    fn browsing_context_or_keyword_rejects_bogus_keyword() {
        assert!(check_browsing_context_or_keyword("_bogus").is_err());
    }

    #[test]
    fn browsing_context_or_keyword_accepts_plain_name() {
        assert!(check_browsing_context_or_keyword("my-frame").is_ok());
    }

    #[test]
    fn keylabellist_rejects_duplicate_tokens() {
        assert!(check_keylabellist("a b a").is_err());
    }

    #[test]
    fn keylabellist_rejects_multi_char_token() {
        assert!(check_keylabellist("a ab").is_err());
    }

    #[test]
    fn keylabellist_accepts_distinct_single_char_tokens() {
        assert!(check_keylabellist("a b c").is_ok());
    }

    #[test]
    fn rel_value_rejects_duplicate_token() {
        assert!(check_rel_value("icon icon").is_err());
    }

    #[test]
    fn rel_value_accepts_short_token_always() {
        assert!(check_rel_value("xyz").is_ok());
    }

    #[test]
    fn rel_value_accepts_unknown_non_duplicate_token() {
        assert!(check_rel_value("totally-unknown-relation").is_ok());
    }

    #[test]
    fn rel_value_flags_likely_typo() {
        // html/attributes/rel/rel-typo-{alternate,stylesheet,author,canonical}-hasinfo.html
        assert!(check_rel_value("alternat").is_err());
        assert!(check_rel_value("styleshet").is_err());
        assert!(check_rel_value("authr").is_err());
        assert!(check_rel_value("canonicl").is_err());
    }

    #[test]
    fn rel_value_accepts_exact_known_relation() {
        assert!(check_rel_value("alternate stylesheet").is_ok());
    }

    #[test]
    fn rel_value_accepts_exact_known_relation_case_insensitively() {
        assert!(check_rel_value("ALTERNATE").is_ok());
    }

    #[test]
    fn levenshtein_distance_matches_known_values() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("alternate", "alternate"), 0);
        assert_eq!(levenshtein_distance("alternat", "alternate"), 1);
    }

    #[test]
    fn find_closest_rel_typo_returns_none_for_an_exact_match() {
        assert_eq!(find_closest_rel_typo("alternate"), None);
    }

    #[test]
    fn find_closest_rel_typo_finds_the_expected_candidate() {
        assert_eq!(find_closest_rel_typo("alternat"), Some("alternate"));
        assert_eq!(find_closest_rel_typo("styleshet"), Some("stylesheet"));
    }

    #[test]
    fn sandbox_allow_list_rejects_unknown_keyword() {
        assert!(check_sandbox_allow_list("allow-bogus").is_err());
    }

    #[test]
    fn sandbox_allow_list_rejects_duplicate() {
        assert!(check_sandbox_allow_list("allow-forms allow-forms").is_err());
    }

    #[test]
    fn sandbox_allow_list_rejects_scripts_and_same_origin() {
        assert!(check_sandbox_allow_list("allow-scripts allow-same-origin").is_err());
    }

    #[test]
    fn sandbox_allow_list_rejects_conflicting_top_navigation() {
        assert!(
            check_sandbox_allow_list(
                "allow-top-navigation allow-top-navigation-by-user-activation"
            )
            .is_err()
        );
    }

    #[test]
    fn script_type_accepts_plain_javascript() {
        assert!(check_script_type("text/javascript").is_ok());
    }

    #[test]
    fn script_type_rejects_javascript_with_params() {
        assert!(check_script_type("text/javascript;charset=utf-8").is_err());
    }

    #[test]
    fn script_type_accepts_non_js_type_with_params() {
        assert!(check_script_type("text/plain;charset=utf-8").is_ok());
    }

    #[test]
    fn microdata_property_accepts_bare_token() {
        assert!(check_microdata_property("name").is_ok());
    }

    #[test]
    fn microdata_property_accepts_valid_absolute_url() {
        assert!(check_microdata_property("https://example.com/prop").is_ok());
    }

    #[test]
    fn microdata_property_rejects_invalid_url_with_colon() {
        // Contains ':' but is not parseable as an absolute URL at all (no
        // valid scheme token before the space): the `url` crate reports
        // `RelativeUrlWithoutBase` for it, unlike e.g. "not:a-valid-url",
        // which the WHATWG URL algorithm accepts as a valid non-special
        // URL with scheme "not" and opaque path "a-valid-url".
        assert!(check_microdata_property("not a url:").is_err());
    }

    #[test]
    fn simple_color_accepts_valid_hex() {
        assert!(check_simple_color("#ff0000").is_ok());
    }

    #[test]
    fn simple_color_rejects_wrong_length() {
        assert!(check_simple_color("#fff").is_err());
    }

    #[test]
    fn simple_color_rejects_non_hex_char() {
        assert!(check_simple_color("#gg0000").is_err());
    }
}
