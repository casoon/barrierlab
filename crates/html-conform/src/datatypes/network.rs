//! Network/URI `w:*` datatypes for `http://whattf.org/datatype-draft`.
//!
//! Covers `w:iri`, `w:iri-ref`, `w:iri-ref-http-or-https`, `w:mime-type`,
//! `w:mime-type-list`, `w:charset`, `w:meta-charset`,
//! `w:integrity-metadata`, and `w:refresh`. See
//! `plan/05c-datatype-library.md` ("Verbindliches Prinzip: vnu-Parität als
//! Default") and `plan/05c-research-group-b.md` (items 8-10, 13-16, 18, 19)
//! for the vnu (`nu.validator.datatype`) research this module is based on.

/// vnu's `isWhitespace` character set (see e.g.
/// `nu.validator.datatype.AbstractDatatype`): exactly these five ASCII
/// characters. Deliberately not Rust's `char::is_whitespace()`, which is
/// full Unicode whitespace and matches a different set.
const VNU_WHITESPACE: [char; 5] = [' ', '\t', '\u{0C}', '\n', '\r'];

fn is_vnu_whitespace(c: char) -> bool {
    VNU_WHITESPACE.contains(&c)
}

// ---------------------------------------------------------------------
// w:iri / w:iri-ref / w:iri-ref-http-or-https
// ---------------------------------------------------------------------
//
// vnu's `IriRef` (`nu.validator.datatype.IriRef`, source for `w:iri-ref`,
// with `w:iri` = `IriRef` plus an `isAbsolute()` requirement and
// `w:iri-ref-http-or-https` = `IriRef` plus a scheme restriction) is a
// hand-written scheme scanner (`ALPHA (ALPHA|DIGIT|+|.)* ":"`) that then
// delegates the actual URI grammar to the Galimatias library (a Java,
// WHATWG-URL-Standard-conformant parser), with several scheme-specific
// quirks on top: `javascript:` is never parsed at all; `data:` is
// additionally decoded through vnu's own `DataUri` decoder
// (base64/percent-decoding); `feed:`/`webcal:` are rewritten to `http:`
// before parsing; and unknown schemes are prefixed with `x-` so that
// Galimatias can still check generic URI syntax without failing on an
// unrecognized scheme.
//
// This is a **deliberate, documented approximation** of that behavior, not
// a byte-for-byte port: none of `javascript:`-skipping, `data:`-specific
// decoding, `feed:`/`webcal:` rewriting, or `x-`-prefixing of unknown
// schemes is replicated here. The `url` crate (a WHATWG-URL-Standard
// implementation from the same standard family as Galimatias) supplies the
// structural parse (scheme/authority/path/query/fragment) — but on its
// own it is **too lenient** to use directly as a conformance check: the
// WHATWG URL Standard is deliberately designed for browser compatibility
// (make the best sense of whatever a real webpage throws at it), not for
// rejecting malformed input, and normalizes away exactly the kinds of
// errors vnu's corpus expects to be caught (verified directly — see
// `plan/DECISIONS.md`'s Phase 08 entry): a raw space in the path becomes
// percent-encoded rather than rejected (`"http://f:21/ b"` → parses fine),
// a hex/octal-looking host becomes a resolved IPv4 address
// (`"http://192.0x00A80001"` → `192.168.0.1`), and a second raw `#`
// becomes literal fragment content instead of a syntax error. Galimatias
// (and RFC 3986/3987, which it implements) reject all three.
//
// [`reject_raw_iri_syntax_errors`] below is therefore run *first*, on the
// value with only its outer (HTML-attribute-level, "potentially
// surrounded by spaces") whitespace trimmed — it rejects the specific,
// corpus-verified set of raw byte patterns the `url` crate silently
// tolerates: internal whitespace, C0/C1 control characters, backslashes,
// malformed percent-encoding, and a second raw `#` in the fragment. This
// is not a full independent RFC 3986/3987 grammar (host-level numeric-vs-
// hostname disambiguation quirks like the hex/octal/fullwidth-digit IPv4
// cases above are not replicated — a smaller, accepted residual gap) but
// closes the overwhelming majority of the corpus gap this way rather than
// reimplementing a whole URI parser from scratch.

/// Rejects raw character patterns the `url` crate's WHATWG-URL parser
/// would silently tolerate (see the module doc comment above for why a
/// second, independent pass is needed on top of `url::Url::parse`).
/// `value` must already have outer HTML-attribute-level whitespace
/// trimmed — anything this function finds is *inside* the IRI reference
/// itself, not incidental attribute-value padding.
fn reject_raw_iri_syntax_errors(value: &str) -> Result<(), String> {
    if value.chars().any(is_vnu_whitespace) {
        return Err("IRI reference must not contain whitespace".to_string());
    }
    if let Some(control) = value.chars().find(|&c| is_disallowed_control_char(c)) {
        return Err(format!(
            "IRI reference must not contain U+{:04X}",
            control as u32
        ));
    }
    if value.contains('\\') {
        return Err("IRI reference must not contain a backslash".to_string());
    }
    if has_malformed_percent_encoding(value) {
        return Err("IRI reference contains malformed percent-encoding".to_string());
    }
    if has_invalid_special_scheme_slashes(value) {
        return Err("IRI reference scheme must be followed by '//'".to_string());
    }
    if has_disallowed_userinfo(value) {
        return Err("IRI reference must not contain userinfo".to_string());
    }
    if has_invalid_square_brackets(value) {
        return Err("IRI reference contains invalid square brackets".to_string());
    }
    if has_data_fragment(value) {
        return Err("data: URL must not contain a fragment identifier".to_string());
    }
    if has_file_pipe_drive(value) {
        return Err("file: URL must not contain '|' drive letter syntax".to_string());
    }
    // The fragment is everything after the *first* raw `#` — RFC 3986's
    // fragment production doesn't include raw `#` itself, so a second one
    // is a syntax error (the `url` crate instead just keeps parsing it as
    // literal fragment content).
    if let Some((_, fragment)) = value.split_once('#')
        && fragment.contains('#')
    {
        return Err("IRI reference fragment must not contain another '#'".to_string());
    }
    Ok(())
}

fn has_data_fragment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("data:") && lower.contains('#')
}

fn has_file_pipe_drive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("file:") && lower.contains('|')
}

fn has_invalid_square_brackets(value: &str) -> bool {
    if let Some(i) = value.find("//") {
        let after_slashes = &value[i + 2..];
        let authority = after_slashes
            .find(['/', '?', '#'])
            .map_or(after_slashes, |j| &after_slashes[..j]);
        let host_port = authority
            .find('@')
            .map_or(authority, |k| &authority[k + 1..]);
        let host = if host_port.starts_with('[') {
            if let Some(close) = host_port.find(']') {
                &host_port[..close + 1]
            } else {
                host_port
            }
        } else if let Some(colon) = host_port.rfind(':') {
            &host_port[..colon]
        } else {
            host_port
        };
        if host.starts_with('[')
            && host.ends_with(']')
            && !host[1..host.len() - 1].contains('[')
            && !host[1..host.len() - 1].contains(']')
        {
            let rest = after_slashes
                .find(['/', '?', '#'])
                .map_or("", |j| &after_slashes[j..]);
            return rest.contains('[') || rest.contains(']');
        }
    }
    value.contains('[') || value.contains(']')
}

fn has_invalid_special_scheme_slashes(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    for scheme in &["http:", "https:", "ftp:", "ws:", "wss:"] {
        if lower.starts_with(scheme) && !lower.starts_with(&format!("{scheme}//")) {
            return true;
        }
    }
    if lower.starts_with("data:/")
        || (lower.starts_with("file:")
            && !lower.starts_with("file:///")
            && !lower.starts_with("file://"))
    {
        return true;
    }
    false
}

fn has_disallowed_userinfo(value: &str) -> bool {
    if let Some(i) = value.find("//") {
        let after_slashes = &value[i + 2..];
        let authority = after_slashes
            .find(['/', '?', '#'])
            .map_or(after_slashes, |j| &after_slashes[..j]);
        if authority.contains('@') {
            return true;
        }
    }
    false
}

/// C0/C1 control characters *not* already covered by [`is_vnu_whitespace`]
/// (`\t`/`\x0C`/`\n`/`\r`/space) — e.g. NUL (U+0000) or the C1 control
/// U+0091 the corpus specifically tests for.
fn is_disallowed_control_char(c: char) -> bool {
    matches!(c as u32, 0x00..=0x08 | 0x0B | 0x0E..=0x1F | 0x7F..=0x9F)
}

/// Whether `value` contains a `%` not followed by exactly two hex digits
/// (`pct-encoded = "%" HEXDIG HEXDIG`, RFC 3986 §2.1) — the `url` crate
/// leaves an invalid `%` as a literal character rather than rejecting it.
fn has_malformed_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let valid = bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit);
            if !valid {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

/// Trims HTML-attribute-level outer whitespace (`href`'s own "potentially
/// surrounded by spaces" allowance) and runs
/// [`reject_raw_iri_syntax_errors`], returning the trimmed value for the
/// structural `url`-crate parse that follows. Shared by all three
/// `w:iri*` checks below.
fn trimmed_and_syntax_checked<'a>(value: &'a str, empty_message: &str) -> Result<&'a str, String> {
    let trimmed = value.trim_matches(is_vnu_whitespace);
    if trimmed.is_empty() {
        return Err(empty_message.to_string());
    }
    reject_raw_iri_syntax_errors(trimmed)?;
    Ok(trimmed)
}

/// `w:iri-ref` (`nu.validator.datatype.IriRef`). See the module-level
/// doc comment above for the documented scope decision (scheme-dispatch
/// quirks not replicated). The smaller residual host-numeric-parsing gap
/// it used to mention is now closed by [`has_lenient_ipv4_host`] below.
pub(crate) fn check_iri_ref(value: &str) -> Result<(), String> {
    let value = trimmed_and_syntax_checked(value, "IRI reference must not be empty")?;
    if let Ok(url) = url::Url::parse(value) {
        if has_lenient_ipv4_host(value, &url) {
            return Err(lenient_ipv4_host_message(value));
        }
        return Ok(());
    }
    let base = url::Url::parse("http://example.org/foo/bar").expect("synthetic base URL is valid");
    match url::Url::options().base_url(Some(&base)).parse(value) {
        Ok(url) if has_lenient_ipv4_host(value, &url) => Err(lenient_ipv4_host_message(value)),
        Ok(_) => Ok(()),
        Err(err) => Err(format!("not a valid IRI reference: {err}")),
    }
}

/// `w:iri` (`nu.validator.datatype.Iri`, `IriRef` with `isAbsolute()`
/// required). Unlike [`check_iri_ref`], no base-relative fallback is
/// attempted — the value itself must parse as an absolute URL.
pub(crate) fn check_iri(value: &str) -> Result<(), String> {
    let value = trimmed_and_syntax_checked(value, "IRI must not be empty")?;
    match url::Url::parse(value) {
        Ok(url) if has_lenient_ipv4_host(value, &url) => Err(lenient_ipv4_host_message(value)),
        Ok(_) => Ok(()),
        Err(_) => Err("not an absolute URL".to_string()),
    }
}

/// `w:iri-ref-http-or-https` (`nu.validator.datatype.IriRefHttpOrHttps`):
/// like [`check_iri_ref`], but the resolved scheme must be exactly `http`
/// or `https` (the `url` crate normalizes scheme case, so a plain
/// comparison is sufficient).
pub(crate) fn check_iri_ref_http_or_https(value: &str) -> Result<(), String> {
    let value = trimmed_and_syntax_checked(value, "IRI reference must not be empty")?;
    let url = if let Ok(url) = url::Url::parse(value) {
        url
    } else {
        let base =
            url::Url::parse("http://example.org/foo/bar").expect("synthetic base URL is valid");
        url::Url::options()
            .base_url(Some(&base))
            .parse(value)
            .map_err(|err| format!("not a valid IRI reference: {err}"))?
    };
    if has_lenient_ipv4_host(value, &url) {
        return Err(lenient_ipv4_host_message(value));
    }
    if url.scheme() == "http" || url.scheme() == "https" {
        Ok(())
    } else {
        Err(format!(
            "scheme must be `http` or `https`, was `{}`",
            url.scheme()
        ))
    }
}

fn lenient_ipv4_host_message(value: &str) -> String {
    format!("not a valid IRI reference: `{value}`'s host is not a valid IPv4 address")
}

/// Whether `url`'s host resolved to an IPv4 address only via a leniency
/// the `url` crate's WHATWG-URL parser applies that RFC 3986 (and vnu's
/// Galimatias-based `IriRef`, which implements RFC 3986/3987) does not:
/// fewer than four dot-separated parts, hex-/octal-prefixed parts, a
/// percent-encoded host that decodes to one of those forms, or full-width
/// Unicode digits — all silently normalized by WHATWG's lenient IPv4
/// parser into a canonical dotted-decimal address instead of rejected.
/// RFC 3986's `IPv4address` production is exactly four plain-ASCII-decimal
/// `dec-octet`s, nothing else.
///
/// Detected by re-extracting the *raw* host substring straight from
/// `raw_value` (before any of the `url` crate's own percent-decoding or
/// Unicode normalization) and checking it isn't already in that strict
/// four-decimal-group form itself — if `url` resolved an IPv4 host from
/// something that doesn't already look like plain dotted-decimal, some
/// leniency must have been applied. Confirmed against
/// `html/elements/a/href/host-192.0x00A80001-novalid.html`,
/// `host-IP-address-fullwidth-novalid.html`, and
/// `host-IP-address-percent-encoded-novalid.html` (and their
/// `area`/`audio`/`base`/... siblings across every element with an IRI-ref
/// attribute) — all expected errors, all previously accepted here.
fn has_lenient_ipv4_host(raw_value: &str, url: &url::Url) -> bool {
    if !matches!(url.host(), Some(url::Host::Ipv4(_))) {
        return false;
    }
    match extract_raw_host(raw_value) {
        Some(raw_host) => !is_strict_dotted_decimal_ipv4(raw_host),
        None => false,
    }
}

/// Best-effort extraction of the raw `host[:port]` authority component
/// straight from the original (un-normalized) IRI-ref text — deliberately
/// simple string scanning, not a full authority grammar, since it's only
/// ever consulted after the `url` crate has already confirmed the whole
/// value parses successfully with an IPv4 host; this only re-examines what
/// the author actually wrote there. Bracketed (IPv6-literal-shaped) hosts
/// are left alone — they're never the numeric-IPv4 leniency this exists
/// to catch.
fn extract_raw_host(value: &str) -> Option<&str> {
    let after_scheme = &value[value.find("//")? + 2..];
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .map_or(after_scheme, |i| &after_scheme[..i]);
    let host_and_port = match authority_end.rfind('@') {
        Some(i) => &authority_end[i + 1..],
        None => authority_end,
    };
    if host_and_port.starts_with('[') {
        return None;
    }
    Some(match host_and_port.rfind(':') {
        Some(i) => &host_and_port[..i],
        None => host_and_port,
    })
}

/// RFC 3986's `IPv4address = dec-octet "." dec-octet "." dec-octet "."
/// dec-octet` shape check: exactly four dot-separated groups, each one or
/// more plain ASCII digits. Doesn't re-check the 0-255 range (the `url`
/// crate already confirmed this parses as a valid IPv4 address overall;
/// this only asks whether the *raw* text was already in that strict
/// shape, or whether `url` had to apply some leniency to get there).
fn is_strict_dotted_decimal_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

// ---------------------------------------------------------------------
// w:mime-type / w:mime-type-list
// ---------------------------------------------------------------------
//
// `w:mime-type` (`nu.validator.datatype.MimeType`): a hand-written
// RFC-2045-style parser for `type "/" subtype *( ";" parameter )`.

/// RFC 2045 "tspecials" that are excluded from otherwise-ASCII-33-126
/// token characters.
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

/// Splits the leading run of MIME token characters off `s`, returning
/// `(token, rest)`.
fn parse_mime_token(s: &str) -> (&str, &str) {
    let end = s.find(|c: char| !is_mime_token_char(c)).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// `w:mime-type` (`nu.validator.datatype.MimeType`). Whitespace is
/// tolerated on either side of the `;` separating parameters (not just
/// before it), since attribute values such as
/// `text/html; charset=utf-8` are the common real-world form.
pub(crate) fn check_mime_type(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("MIME type must not be empty".to_string());
    }
    if value.starts_with(is_vnu_whitespace) || value.ends_with(is_vnu_whitespace) {
        return Err("MIME type must not contain leading or trailing whitespace".to_string());
    }
    let (supertype, rest) = parse_mime_token(value);
    if supertype.is_empty() {
        return Err(format!("missing MIME supertype in `{value}`"));
    }
    let rest = rest
        .strip_prefix('/')
        .ok_or_else(|| format!("missing `/` between type and subtype in `{value}`"))?;
    let (subtype, mut rest) = parse_mime_token(rest);
    if subtype.is_empty() {
        return Err(format!("missing MIME subtype after `/` in `{value}`"));
    }

    loop {
        let trimmed = rest.trim_start_matches(is_vnu_whitespace);
        if trimmed.is_empty() {
            return Ok(());
        }
        let after_semi = trimmed
            .strip_prefix(';')
            .ok_or_else(|| format!("unexpected trailing characters `{trimmed}` in `{value}`"))?;
        // Whitespace is tolerated on both sides of the `;` (common in
        // practice, e.g. `text/html; charset=utf-8`), not just before it.
        let after_semi = after_semi.trim_start_matches(is_vnu_whitespace);
        let (param_name, after_name) = parse_mime_token(after_semi);
        if param_name.is_empty() {
            return Err(format!("expected a parameter name after `;` in `{value}`"));
        }
        let after_eq = after_name
            .strip_prefix('=')
            .ok_or_else(|| format!("expected `=` after parameter name in `{value}`"))?;
        if let Some(after_quote) = after_eq.strip_prefix('"') {
            let mut chars = after_quote.char_indices();
            let mut end = None;
            while let Some((i, c)) = chars.next() {
                if c == '\\' {
                    chars.next();
                } else if c == '"' {
                    end = Some(i);
                    break;
                }
            }
            match end {
                Some(i) => rest = &after_quote[i + 1..],
                None => return Err(format!("unterminated quoted parameter value in `{value}`")),
            }
        } else {
            let (param_value, after_value) = parse_mime_token(after_eq);
            if param_value.is_empty() {
                return Err(format!("expected a parameter value after `=` in `{value}`"));
            }
            rest = after_value;
        }
    }
}

/// `w:mime-type-list` (`nu.validator.datatype.MimeTypeList`): a
/// comma-separated list of MIME-type patterns as used by `accept=""`.
/// Reuses [`is_mime_token_char`]/[`parse_mime_token`] from `w:mime-type`,
/// but list entries never carry `;parameter`s in practice, so each entry
/// is validated as a bare `type/subtype` (or a `*/*`, `type/*`, or
/// `.extension` pattern) rather than delegating to the full
/// [`check_mime_type`] parameter grammar.
pub(crate) fn check_mime_type_list(value: &str) -> Result<(), String> {
    if value.trim_matches(is_vnu_whitespace).is_empty() {
        return Err("MIME type list must not be empty".to_string());
    }
    for entry in value.split(',') {
        check_mime_type_list_entry(entry.trim_matches(is_vnu_whitespace))?;
    }
    Ok(())
}

fn check_mime_type_list_entry(entry: &str) -> Result<(), String> {
    if entry.is_empty() {
        return Err("empty entry in MIME type list".to_string());
    }
    if entry == "*/*" {
        return Ok(());
    }
    if let Some(ext) = entry.strip_prefix('.') {
        return if !ext.is_empty() && ext.chars().all(is_mime_token_char) {
            Ok(())
        } else {
            Err(format!("invalid file extension entry `{entry}`"))
        };
    }
    let (supertype, rest) = parse_mime_token(entry);
    if supertype.is_empty() {
        return Err(format!("invalid MIME type list entry `{entry}`"));
    }
    let rest = rest
        .strip_prefix('/')
        .ok_or_else(|| format!("invalid MIME type list entry `{entry}` (missing `/`)"))?;
    if rest == "*" {
        return Ok(());
    }
    let (subtype, rest) = parse_mime_token(rest);
    if subtype.is_empty() || !rest.is_empty() {
        return Err(format!("invalid MIME type list entry `{entry}`"));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// w:charset / w:meta-charset
// ---------------------------------------------------------------------

/// RFC 2978 `mime-charsetc` character class.
fn is_mime_charsetc(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '-' | '!' | '#' | '$' | '%' | '&' | '\'' | '+' | '_' | '`' | '{' | '}' | '~' | '^'
        )
}

/// `w:charset` (`nu.validator.datatype.Charset`): RFC 2978 character
/// class, then must be the WHATWG Encoding Standard's *preferred*
/// (canonical) name for the encoding, not merely a recognized alias
/// (verified against `encoding_rs::Encoding::for_label`, e.g. the
/// recognized alias `"latin1"` resolves to canonical `"windows-1252"`
/// and is therefore rejected). The `"replacement"` pseudo-encoding is
/// always rejected, matching vnu's explicit handling of it.
pub(crate) fn check_charset(value: &str) -> Result<(), String> {
    if value.is_empty() || !value.chars().all(is_mime_charsetc) {
        return Err(format!("`{value}` is not a valid character set name"));
    }
    let lower = value.to_ascii_lowercase();
    let encoding = encoding_rs::Encoding::for_label(lower.as_bytes())
        .ok_or_else(|| format!("`{value}` is not a recognized character encoding name"))?;
    if encoding.name().eq_ignore_ascii_case("replacement") {
        return Err("the \"replacement\" pseudo-encoding is not a valid charset".to_string());
    }
    if !encoding.name().eq_ignore_ascii_case(&lower) {
        return Err(format!(
            "`{value}` is not the preferred name for this encoding; use `{}`",
            encoding.name()
        ));
    }
    Ok(())
}

/// `w:meta-charset` (`nu.validator.datatype.MetaCharset`): the WHATWG
/// "extracting a character encoding from a meta element" algorithm,
/// applied to a `<meta http-equiv="Content-Type" content="...">`-style
/// value. Implements the documented "false `charset` match without `=`"
/// retry quirk: if `charset` is found but not followed by `=` (after
/// skipping whitespace), search continues for the *next* occurrence of
/// `charset` instead of failing outright.
pub(crate) fn check_meta_charset(value: &str) -> Result<(), String> {
    let lower = value.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("text/html;")
        .ok_or_else(|| "must start with `text/html;`".to_string())?;
    if rest.is_empty() {
        return Err("ended prematurely after `text/html;`".to_string());
    }

    let mut search_from = 0usize;
    loop {
        let idx = match rest[search_from..].find("charset") {
            Some(i) => search_from + i,
            None => return Err("no `charset` parameter found".to_string()),
        };
        let after_kw = &rest[idx + "charset".len()..];
        let after_ws = after_kw.trim_start_matches(is_vnu_whitespace);
        let Some(after_eq) = after_ws.strip_prefix('=') else {
            search_from = idx + "charset".len();
            continue;
        };
        let after_ws2 = after_eq.trim_start_matches(is_vnu_whitespace);

        let extracted = if let Some(q) = after_ws2.strip_prefix('"') {
            match q.find('"') {
                Some(end) => &q[..end],
                None => {
                    search_from = idx + "charset".len();
                    continue;
                }
            }
        } else if let Some(q) = after_ws2.strip_prefix('\'') {
            match q.find('\'') {
                Some(end) => &q[..end],
                None => {
                    search_from = idx + "charset".len();
                    continue;
                }
            }
        } else {
            let end = after_ws2
                .find(|c: char| is_vnu_whitespace(c) || c == ';')
                .unwrap_or(after_ws2.len());
            &after_ws2[..end]
        };

        if extracted.is_empty() || !extracted.chars().all(is_mime_charsetc) {
            return Err(format!("extracted charset name `{extracted}` is not valid"));
        }
        if extracted != "utf-8" {
            return Err(format!(
                "extracted charset must be `utf-8`, was `{extracted}`"
            ));
        }
        return Ok(());
    }
}

// ---------------------------------------------------------------------
// w:integrity-metadata
// ---------------------------------------------------------------------

/// `w:integrity-metadata` (`nu.validator.datatype.IntegrityMetadata`):
/// whitespace-separated list of SRI hash-with-options tokens. vnu
/// delegates the base64 check to `htmlunit-csp`'s `Hash.parseHash`; per
/// `plan/DECISIONS.md` ("Phase 05c"), this crate does not depend on
/// `csp-parse` for this and instead hand-validates the base64 character
/// set/padding shape.
pub(crate) fn check_integrity_metadata(value: &str) -> Result<(), String> {
    for token in value.split(is_vnu_whitespace).filter(|t| !t.is_empty()) {
        check_integrity_metadata_token(token)?;
    }
    Ok(())
}

fn check_integrity_metadata_token(token: &str) -> Result<(), String> {
    let lower = token.to_ascii_lowercase();
    let prefix_len = if lower.starts_with("sha256-")
        || lower.starts_with("sha384-")
        || lower.starts_with("sha512-")
    {
        7
    } else {
        return Err(format!(
            "`{token}` does not start with `sha256-`, `sha384-`, or `sha512-`"
        ));
    };
    let rest = &token[prefix_len..];
    if rest.is_empty() {
        return Err(format!("`{token}` has no hash value after the prefix"));
    }
    let hash_part = match rest.find('?') {
        Some(i) => &rest[..i],
        None => rest,
    };
    if !is_valid_base64_shape(hash_part) {
        return Err(format!("`{hash_part}` in `{token}` is not valid base64"));
    }
    Ok(())
}

/// Lenient structural base64 check: standard or base64url alphabet plus
/// optional `=`/`==` padding — no multiple-of-4 length requirement.
/// Matches vnu's actual behavior, which delegates to `htmlunit-csp`'s
/// `Hash.parseHash` (`org.htmlunit.csp.value.Hash`,
/// `org.htmlunit.csp.Utils.IS_BASE64_VALUE`, character class
/// `[a-zA-Z0-9+/\-_]+=?=?` — no padding-length requirement at all).
/// Confirmed against `html/datatypes/integrity-valid-isvalid.html`
/// (`sha256-abc123def456ghi789jkl012mno345pqr678stu901v`, 43 base64
/// characters with no padding — expected clean, previously rejected
/// here for not being a multiple of 4). Exact decoding correctness is
/// still out of scope (see module doc comment for
/// `w:integrity-metadata`).
fn is_valid_base64_shape(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let (data, padding) = match s.find('=') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    if data.is_empty() {
        return false;
    }
    if !data
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '-' || c == '_')
    {
        return false;
    }
    matches!(padding, "" | "=" | "==")
}

// ---------------------------------------------------------------------
// w:refresh
// ---------------------------------------------------------------------

/// `w:refresh` (`nu.validator.datatype.Refresh`, extends `IriRef`):
/// hand-rolled state machine for
/// `<meta http-equiv="refresh" content="N;url=...">`. Rejects quoted
/// URLs and requires mandatory whitespace after `;` — deliberately
/// stricter than actual browser behavior (a conformance checker, not a
/// browser reimplementation), matching vnu.
pub(crate) fn check_refresh(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;

    let digit_start = i;
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return Err("must start with one or more digits".to_string());
    }
    if i == n {
        return Ok(());
    }
    if bytes[i] != b';' {
        return Err(format!(
            "expected `;` after the digits, found `{}`",
            bytes[i] as char
        ));
    }
    i += 1;
    if i == n || !is_vnu_whitespace(bytes[i] as char) {
        return Err("expected whitespace after `;`".to_string());
    }
    while i < n && is_vnu_whitespace(bytes[i] as char) {
        i += 1;
    }
    for expected in *b"url" {
        if i >= n || !bytes[i].eq_ignore_ascii_case(&expected) {
            return Err(format!(
                "expected `{}` in the `url` keyword",
                expected as char
            ));
        }
        i += 1;
    }
    if i >= n || bytes[i] != b'=' {
        return Err("expected `=` after the `url` keyword".to_string());
    }
    i += 1;
    if i >= n {
        return Err("expected a URL after `=`".to_string());
    }
    if bytes[i] == b'"' || bytes[i] == b'\'' {
        return Err("quoted URLs are not allowed".to_string());
    }
    if is_vnu_whitespace(bytes[i] as char) {
        return Err("whitespace is not allowed immediately after `=`".to_string());
    }
    if value.chars().next_back().is_some_and(is_vnu_whitespace) {
        return Err("trailing whitespace is not allowed".to_string());
    }
    check_iri_ref(&value[i..])
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- check_iri_ref / check_iri / check_iri_ref_http_or_https --

    #[test]
    fn iri_ref_rejects_empty() {
        assert!(check_iri_ref("").is_err());
        assert!(check_iri_ref("   ").is_err());
    }

    #[test]
    fn iri_ref_accepts_absolute_url() {
        assert!(check_iri_ref("http://example.com/path?q=1").is_ok());
    }

    #[test]
    fn iri_ref_accepts_relative_reference() {
        assert!(check_iri_ref("../foo/bar.html").is_ok());
        assert!(check_iri_ref("#fragment").is_ok());
    }

    #[test]
    fn iri_ref_rejects_unparsable_value() {
        // Has a scheme (so the base-relative fallback is not attempted),
        // but an empty host makes it fail even the absolute parse.
        assert!(check_iri_ref("http://").is_err());
    }

    // -- reject_raw_iri_syntax_errors: cases the `url` crate alone would
    // silently accept, verified against real vnu corpus fixtures
    // (tests/corpus/html/elements/{a,area,...}/href/*-novalid.html) — see
    // this module's doc comment on why a second, independent pass is
    // needed on top of `url::Url::parse`.

    #[test]
    fn iri_ref_rejects_internal_whitespace_but_trims_outer() {
        // `url`'s WHATWG parser percent-encodes internal whitespace
        // instead of rejecting it (verified directly against the crate).
        assert!(check_iri_ref("http://f:21/ b").is_err());
        assert!(check_iri_ref("http://f:21/b ?").is_err());
        assert!(check_iri_ref("http://f:21/b?d #").is_err());
        assert!(check_iri_ref("http://example.com/foo\tbar").is_err());
        assert!(check_iri_ref("http://example.\norg").is_err());
        // HTML5's "potentially surrounded by spaces" allowance: outer
        // whitespace around the whole attribute value is still fine.
        assert!(check_iri_ref("  http://example.com/  ").is_ok());
    }

    #[test]
    fn iri_ref_rejects_disallowed_control_characters() {
        assert!(check_iri_ref("http://example.com/foo\u{91}bar").is_err());
        assert!(check_iri_ref("http://example.com/foo\u{0}bar").is_err());
    }

    #[test]
    fn iri_ref_rejects_raw_backslash() {
        assert!(check_iri_ref(":\\").is_err());
        assert!(check_iri_ref("http://foo.com/\\@").is_err());
    }

    #[test]
    fn iri_ref_rejects_malformed_percent_encoding() {
        assert!(check_iri_ref("http://example.com/foo%").is_err());
        assert!(check_iri_ref("http://example.com/foo/%2e%2").is_err());
        // A *valid* percent-encoded triplet is fine.
        assert!(check_iri_ref("http://example.com/foo%20bar").is_ok());
    }

    #[test]
    fn iri_ref_rejects_second_raw_hash_in_fragment() {
        assert!(check_iri_ref("http://foo/path#f#g").is_err());
        // A single `#` starting the fragment is fine.
        assert!(check_iri_ref("http://foo/path#fragment").is_ok());
    }

    #[test]
    fn iri_ref_accepts_plain_dotted_decimal_ipv4_host() {
        assert!(check_iri_ref("http://192.168.0.1/").is_ok());
        assert!(check_iri_ref("http://127.0.0.1:8080/path").is_ok());
    }

    #[test]
    fn iri_ref_rejects_hex_prefixed_ipv4_host() {
        // html/elements/a/href/host-192.0x00A80001-novalid.html
        assert!(check_iri_ref("http://192.0x00A80001").is_err());
    }

    #[test]
    fn iri_ref_rejects_fullwidth_digit_ipv4_host() {
        // html/elements/a/href/host-IP-address-fullwidth-novalid.html
        assert!(check_iri_ref("http://\u{FF10}\u{FF38}\u{FF43}\u{FF10}.\u{FF10}\u{FF12}\u{FF15}\u{FF10}.\u{FF10}.\u{FF11}").is_err());
    }

    #[test]
    fn iri_ref_rejects_percent_encoded_hex_ipv4_host() {
        // html/elements/a/href/host-IP-address-percent-encoded-novalid.html
        assert!(check_iri_ref("http://%30%78%63%30%2e%30%32%35%30.01").is_err());
    }

    #[test]
    fn iri_ref_does_not_flag_bracketed_ipv6_hosts() {
        assert!(check_iri_ref("http://[::1]/path").is_ok());
    }

    #[test]
    fn iri_requires_absolute_url() {
        assert!(check_iri("http://example.com/").is_ok());
        assert!(check_iri("../foo/bar.html").is_err());
        assert!(check_iri("").is_err());
    }

    #[test]
    fn iri_ref_http_or_https_accepts_http_and_https() {
        assert!(check_iri_ref_http_or_https("http://example.com/").is_ok());
        assert!(check_iri_ref_http_or_https("HTTPS://example.com/").is_ok());
        assert!(check_iri_ref_http_or_https("/relative/path").is_ok());
    }

    #[test]
    fn iri_ref_http_or_https_rejects_other_scheme() {
        assert!(check_iri_ref_http_or_https("ftp://example.com/").is_err());
        assert!(check_iri_ref_http_or_https("mailto:a@example.com").is_err());
    }

    // -- check_mime_type --

    #[test]
    fn mime_type_accepts_simple_type() {
        assert!(check_mime_type("text/html").is_ok());
    }

    #[test]
    fn mime_type_accepts_parameters() {
        assert!(check_mime_type("text/html;charset=utf-8").is_ok());
        assert!(check_mime_type("text/html; charset=\"utf-8\"").is_ok());
        assert!(check_mime_type(r#"text/html; charset="a\"b""#).is_ok());
    }

    #[test]
    fn mime_type_rejects_empty() {
        assert!(check_mime_type("").is_err());
    }

    #[test]
    fn mime_type_rejects_missing_subtype() {
        assert!(check_mime_type("text/").is_err());
        assert!(check_mime_type("text").is_err());
    }

    #[test]
    fn mime_type_rejects_trailing_semicolon() {
        assert!(check_mime_type("text/html;").is_err());
    }

    #[test]
    fn mime_type_rejects_unterminated_quote() {
        assert!(check_mime_type("text/html;charset=\"utf-8").is_err());
    }

    #[test]
    fn mime_type_rejects_trailing_equals() {
        assert!(check_mime_type("text/html;charset=").is_err());
    }

    // -- check_mime_type_list --

    #[test]
    fn mime_type_list_accepts_entries() {
        assert!(check_mime_type_list("image/png, image/*,*/*, .png").is_ok());
    }

    #[test]
    fn mime_type_list_rejects_bad_entry() {
        assert!(check_mime_type_list("image/png, bogus").is_err());
        assert!(check_mime_type_list("").is_err());
    }

    // -- check_charset --

    #[test]
    fn charset_accepts_canonical_name() {
        assert!(check_charset("utf-8").is_ok());
        assert!(check_charset("UTF-8").is_ok());
    }

    #[test]
    fn charset_rejects_non_canonical_alias() {
        // Verified against actual encoding_rs behavior:
        // Encoding::for_label(b"latin1") == Some(windows-1252), not latin1.
        let err = check_charset("latin1").unwrap_err();
        assert!(err.contains("windows-1252"));
    }

    #[test]
    fn charset_rejects_replacement() {
        assert!(check_charset("replacement").is_err());
    }

    #[test]
    fn charset_rejects_unknown_label() {
        assert!(check_charset("not-a-real-charset").is_err());
    }

    #[test]
    fn charset_rejects_invalid_characters() {
        assert!(check_charset("utf 8").is_err());
        assert!(check_charset("").is_err());
    }

    // -- check_meta_charset --

    #[test]
    fn meta_charset_accepts_utf8() {
        assert!(check_meta_charset("text/html; charset=utf-8").is_ok());
        assert!(check_meta_charset("text/html;charset=\"UTF-8\"").is_ok());
    }

    #[test]
    fn meta_charset_rejects_non_utf8() {
        assert!(check_meta_charset("text/html; charset=iso-8859-1").is_err());
    }

    #[test]
    fn meta_charset_requires_prefix() {
        assert!(check_meta_charset("charset=utf-8").is_err());
        assert!(check_meta_charset("text/html;").is_err());
    }

    #[test]
    fn meta_charset_retries_after_false_match_without_equals() {
        // "charset" appears first without a following "=", must retry and
        // find the second occurrence.
        assert!(check_meta_charset("text/html;charset charset=utf-8").is_ok());
    }

    // -- check_integrity_metadata --

    #[test]
    fn integrity_metadata_accepts_valid_token() {
        assert!(check_integrity_metadata("sha256-abc123==").is_ok());
        assert!(check_integrity_metadata("SHA384-YWJjZGVmZ2g=").is_ok());
    }

    #[test]
    fn integrity_metadata_accepts_multiple_tokens() {
        assert!(check_integrity_metadata("sha256-YWJjZA== sha512-YWJjZGVmZ2g=").is_ok());
    }

    #[test]
    fn integrity_metadata_rejects_wrong_prefix() {
        assert!(check_integrity_metadata("md5-YWJjZA==").is_err());
    }

    #[test]
    fn integrity_metadata_rejects_invalid_base64_char() {
        assert!(check_integrity_metadata("sha256-abc$123").is_err());
    }

    #[test]
    fn integrity_metadata_rejects_empty_hash() {
        assert!(check_integrity_metadata("sha256-").is_err());
    }

    #[test]
    fn integrity_metadata_accepts_unpadded_base64() {
        // html/datatypes/integrity-valid-isvalid.html: a 43-character
        // (not a multiple of 4) unpadded base64 hash — vnu's real
        // acceptance grammar has no padding-length requirement.
        assert!(
            check_integrity_metadata("sha256-abc123def456ghi789jkl012mno345pqr678stu901v").is_ok()
        );
    }

    #[test]
    fn integrity_metadata_accepts_base64url_characters() {
        assert!(check_integrity_metadata("sha256-abc-123_456").is_ok());
    }

    // -- check_refresh --

    #[test]
    fn refresh_accepts_bare_number() {
        assert!(check_refresh("5").is_ok());
    }

    #[test]
    fn refresh_accepts_number_with_url() {
        assert!(check_refresh("5; url=http://example.com").is_ok());
    }

    #[test]
    fn refresh_rejects_quoted_url() {
        assert!(check_refresh("5; url='http://example.com'").is_err());
        assert!(check_refresh("5; url=\"http://example.com\"").is_err());
    }

    #[test]
    fn refresh_requires_whitespace_after_semicolon() {
        // No space between `;` and `url=` — whitespace after `;` is
        // mandatory per the state machine.
        assert!(check_refresh("5;url=http://example.com").is_err());
    }

    #[test]
    fn refresh_rejects_missing_digits() {
        assert!(check_refresh(";url=http://example.com").is_err());
    }

    #[test]
    fn refresh_rejects_trailing_whitespace() {
        assert!(check_refresh("5; url=http://example.com ").is_err());
    }
}
