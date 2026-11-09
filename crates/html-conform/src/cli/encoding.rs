//! Turns the bytes of a document into text, following the WHATWG encoding
//! sniffing for a file without a transport charset: a byte order mark, then
//! the `<meta>` prescan of the first 1024 bytes, then UTF-8. There is no
//! HTTP header here, so nothing outranks the document itself.

use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

/// How many bytes the prescan looks at (HTML Standard, "prescan a byte stream").
const PRESCAN_LIMIT: usize = 1024;

/// A decoded document.
#[derive(Debug)]
pub struct Decoded {
    pub text: String,
    /// The encoding is not UTF-8 and came from the document's own `<meta>`
    /// declaration (not from a byte order mark): the declaration agrees with
    /// how the bytes were read.
    pub meta_declared_legacy: bool,
}

/// Decodes `bytes`, or says why they cannot be decoded without loss.
pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    let (encoding, bom_len, from_bom) = match Encoding::for_bom(bytes) {
        Some((encoding, len)) => (encoding, len, true),
        None => (prescan(bytes).unwrap_or(UTF_8), 0, false),
    };
    match encoding.decode_without_bom_handling_and_without_replacement(&bytes[bom_len..]) {
        Some(text) => Ok(Decoded {
            text: text.into_owned(),
            meta_declared_legacy: !from_bom && encoding != UTF_8,
        }),
        None => Err(format!(
            "not valid {}; the file was not checked",
            encoding.name()
        )),
    }
}

/// The encoding a `<meta charset>` or `<meta http-equiv=content-type>` in the
/// first 1024 bytes names, if any.
fn prescan(bytes: &[u8]) -> Option<&'static Encoding> {
    let bytes = &bytes[..bytes.len().min(PRESCAN_LIMIT)];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"<!--") {
            // Comment: skip to `-->`; the scan may end inside it.
            let end = find(&bytes[i + 2..], b"-->")?;
            i += 2 + end + 3;
        } else if bytes[i] == b'<' && is_meta_start(&bytes[i..]) {
            i += 5;
            let (found, next) = meta_attributes(bytes, i);
            i = next;
            if let Some(encoding) = found {
                return Some(encoding);
            }
        } else if bytes[i] == b'<'
            && bytes
                .get(i + 1)
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'/' && is_alpha(bytes, i + 2))
        {
            // Another tag: skip its name and attributes.
            i += 1;
            if bytes[i] == b'/' {
                i += 1;
            }
            while i < bytes.len() && !is_space(bytes[i]) && bytes[i] != b'>' {
                i += 1;
            }
            loop {
                let (attribute, next) = attribute(bytes, i);
                i = next;
                if attribute.is_none() {
                    break;
                }
            }
        } else if bytes[i] == b'<' && matches!(bytes.get(i + 1), Some(b'!' | b'/' | b'?')) {
            i += find(&bytes[i..], b">")? + 1;
        } else {
            i += 1;
        }
    }
    None
}

fn is_alpha(bytes: &[u8], at: usize) -> bool {
    bytes.get(at).is_some_and(u8::is_ascii_alphabetic)
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn is_meta_start(bytes: &[u8]) -> bool {
    bytes.len() > 5
        && bytes[1..5].eq_ignore_ascii_case(b"meta")
        && (is_space(bytes[5]) || bytes[5] == b'/')
}

/// Reads the attributes of one `<meta>` tag from `i`; returns the encoding it
/// declares and the position after the attributes.
fn meta_attributes(bytes: &[u8], mut i: usize) -> (Option<&'static Encoding>, usize) {
    let mut got_pragma = false;
    let mut need_pragma: Option<bool> = None;
    let mut charset: Option<&'static Encoding> = None;
    let mut seen: Vec<Vec<u8>> = Vec::new();
    while let (Some((name, value)), next) = attribute(bytes, i) {
        i = next;
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        match name.as_slice() {
            b"http-equiv" => got_pragma |= value.eq_ignore_ascii_case(b"content-type"),
            b"content" => {
                if charset.is_none()
                    && let Some(encoding) = charset_in_content(&value)
                {
                    charset = Some(encoding);
                    need_pragma = Some(true);
                }
            }
            b"charset" => {
                charset = Encoding::for_label(trim(&value));
                need_pragma = Some(false);
            }
            _ => {}
        }
    }
    let encoding = match (need_pragma, charset) {
        (Some(need), Some(encoding)) if !need || got_pragma => Some(encoding),
        _ => None,
    };
    (encoding.map(normalise), i)
}

/// A meta-declared UTF-16 means UTF-8 (the document is ASCII-compatible
/// enough to have been read), and `x-user-defined` means windows-1252.
fn normalise(encoding: &'static Encoding) -> &'static Encoding {
    if encoding == UTF_16LE || encoding == UTF_16BE {
        UTF_8
    } else if encoding.name() == "x-user-defined" {
        WINDOWS_1252
    } else {
        encoding
    }
}

fn trim(value: &[u8]) -> &[u8] {
    let start = value
        .iter()
        .position(|b| !is_space(*b))
        .unwrap_or(value.len());
    let end = value
        .iter()
        .rposition(|b| !is_space(*b))
        .map_or(start, |p| p + 1);
    &value[start..end]
}

/// "Algorithm for extracting a character encoding from a meta element".
fn charset_in_content(value: &[u8]) -> Option<&'static Encoding> {
    let mut i = 0;
    loop {
        let at = value[i..]
            .windows(7)
            .position(|w| w.eq_ignore_ascii_case(b"charset"))?;
        i += at + 7;
        while value.get(i).is_some_and(|b| is_space(*b)) {
            i += 1;
        }
        if value.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while value.get(i).is_some_and(|b| is_space(*b)) {
            i += 1;
        }
        let rest = &value[i..];
        let label = match rest.first()? {
            quote @ (b'"' | b'\'') => {
                let end = rest[1..].iter().position(|b| b == quote)?;
                &rest[1..=end]
            }
            _ => {
                let end = rest
                    .iter()
                    .position(|b| is_space(*b) || *b == b';')
                    .unwrap_or(rest.len());
                &rest[..end]
            }
        };
        return Encoding::for_label(label);
    }
}

/// "Get an attribute" of the prescan: a lowercased name and its value, or
/// `None` at the end of the tag (after a `>` or when the input ends).
type Attribute = (Vec<u8>, Vec<u8>);

fn attribute(bytes: &[u8], mut i: usize) -> (Option<Attribute>, usize) {
    while bytes.get(i).is_some_and(|b| is_space(*b) || *b == b'/') {
        i += 1;
    }
    if i >= bytes.len() {
        return (None, i);
    }
    if bytes[i] == b'>' {
        return (None, i + 1);
    }
    let mut name = Vec::new();
    while let Some(&b) = bytes.get(i) {
        if b == b'=' && !name.is_empty() {
            break;
        }
        if is_space(b) || b == b'/' || b == b'>' {
            break;
        }
        name.push(b.to_ascii_lowercase());
        i += 1;
    }
    while bytes.get(i).is_some_and(|b| is_space(*b)) {
        i += 1;
    }
    if bytes.get(i) != Some(&b'=') {
        return (Some((name, Vec::new())), i);
    }
    i += 1;
    while bytes.get(i).is_some_and(|b| is_space(*b)) {
        i += 1;
    }
    let mut value = Vec::new();
    match bytes.get(i) {
        Some(&quote @ (b'"' | b'\'')) => {
            i += 1;
            while let Some(&b) = bytes.get(i) {
                i += 1;
                if b == quote {
                    break;
                }
                value.push(b.to_ascii_lowercase());
            }
        }
        Some(b'>') | None => {}
        _ => {
            while let Some(&b) = bytes.get(i) {
                if is_space(b) || b == b'>' {
                    break;
                }
                value.push(b.to_ascii_lowercase());
                i += 1;
            }
        }
    }
    (Some((name, value)), i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sniffed(html: &[u8]) -> &'static str {
        prescan(html).unwrap_or(UTF_8).name()
    }

    #[test]
    fn bom_wins_over_meta() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "<p>é".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        assert_eq!(decode(&bytes).unwrap().text, "<p>é");
        let mut utf8 = b"\xEF\xBB\xBF<meta charset=latin1>".to_vec();
        utf8.extend("é".as_bytes());
        assert!(decode(&utf8).unwrap().text.ends_with('é'));
    }

    #[test]
    fn meta_charset_and_http_equiv() {
        assert_eq!(sniffed(b"<meta charset=iso-8859-1>"), "windows-1252");
        assert_eq!(sniffed(b"<META CHARSET=\"Shift_JIS\">"), "Shift_JIS");
        assert_eq!(
            sniffed(b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=koi8-r\">"),
            "KOI8-R"
        );
        // `content` alone is not a declaration.
        assert_eq!(
            sniffed(b"<meta content=\"text/html; charset=koi8-r\">"),
            "UTF-8"
        );
    }

    #[test]
    fn prescan_ignores_comments_and_the_tail() {
        assert_eq!(sniffed(b"<!-- <meta charset=koi8-r> --><p>"), "UTF-8");
        let mut late = vec![b' '; 1100];
        late.extend(b"<meta charset=koi8-r>");
        assert_eq!(sniffed(&late), "UTF-8");
        assert_eq!(sniffed(b"<title x='<meta charset=koi8-r>'>"), "UTF-8");
    }

    #[test]
    fn utf16_in_meta_means_utf8() {
        assert_eq!(sniffed(b"<meta charset=utf-16le>"), "UTF-8");
    }

    #[test]
    fn undecodable_bytes_are_an_error() {
        assert!(decode(b"<p>\xFF").unwrap_err().contains("UTF-8"));
        assert!(decode(b"<meta charset=shift_jis><p>\x81").is_err());
        assert_eq!(
            decode(b"<meta charset=latin1><p>\xE9").unwrap().text,
            "<meta charset=latin1><p>\u{e9}"
        );
    }
}
