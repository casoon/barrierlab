use super::*;

/// A document's quirks-mode flag, determined once from its `DOCTYPE`
/// token (§13.2.6.4.1, the "initial" insertion mode). Not just inert
/// metadata: e.g. "in body"'s `<table>` start-tag rule (§13.2.6.4.7) only
/// auto-closes an open `p` element when the document is *not* in quirks
/// mode, so this genuinely shapes the produced tree — that's why it's
/// implemented here despite `html-conform`'s `normalize()` dropping the
/// `DocumentType` node itself from its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum QuirksMode {
    #[default]
    NoQuirks,
    Quirks,
    LimitedQuirks,
}

/// Public identifiers that set the document to quirks mode when matched
/// *exactly* (§13.2.6.4.1).
pub(super) const QUIRKS_PUBLIC_IDENTIFIER_EXACT: &[&str] = &[
    "-//W3O//DTD W3 HTML Strict 3.0//EN//",
    "-/W3C/DTD HTML 4.0 Transitional/EN",
    "HTML",
];

/// The system identifier that sets the document to quirks mode when
/// matched exactly (§13.2.6.4.1).
pub(super) const QUIRKS_SYSTEM_IDENTIFIER_EXACT: &str =
    "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd";

/// Public identifier prefixes that set the document to quirks mode
/// (§13.2.6.4.1), regardless of the system identifier.
pub(super) const QUIRKS_PUBLIC_IDENTIFIER_PREFIXES: &[&str] = &[
    "+//Silmaril//dtd html Pro v0r11 19970101//",
    "-//AS//DTD HTML 3.0 asWedit + extensions//",
    "-//AdvaSoft Ltd//DTD HTML 3.0 asWedit + extensions//",
    "-//IETF//DTD HTML 2.0 Level 1//",
    "-//IETF//DTD HTML 2.0 Level 2//",
    "-//IETF//DTD HTML 2.0 Strict Level 1//",
    "-//IETF//DTD HTML 2.0 Strict Level 2//",
    "-//IETF//DTD HTML 2.0 Strict//",
    "-//IETF//DTD HTML 2.0//",
    "-//IETF//DTD HTML 2.1E//",
    "-//IETF//DTD HTML 3.0//",
    "-//IETF//DTD HTML 3.2 Final//",
    "-//IETF//DTD HTML 3.2//",
    "-//IETF//DTD HTML 3//",
    "-//IETF//DTD HTML Level 0//",
    "-//IETF//DTD HTML Level 1//",
    "-//IETF//DTD HTML Level 2//",
    "-//IETF//DTD HTML Level 3//",
    "-//IETF//DTD HTML Strict Level 0//",
    "-//IETF//DTD HTML Strict Level 1//",
    "-//IETF//DTD HTML Strict Level 2//",
    "-//IETF//DTD HTML Strict Level 3//",
    "-//IETF//DTD HTML Strict//",
    "-//IETF//DTD HTML//",
    "-//Metrius//DTD Metrius Presentational//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 2.0 Tables//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 3.0 Tables//",
    "-//Netscape Comm. Corp.//DTD HTML//",
    "-//Netscape Comm. Corp.//DTD Strict HTML//",
    "-//O'Reilly and Associates//DTD HTML 2.0//",
    "-//O'Reilly and Associates//DTD HTML Extended 1.0//",
    "-//O'Reilly and Associates//DTD HTML Extended Relaxed 1.0//",
    "-//SQ//DTD HTML 2.0 HoTMetaL + extensions//",
    "-//SoftQuad Software//DTD HoTMetaL PRO 6.0::19990601::extensions to HTML 4.0//",
    "-//SoftQuad//DTD HoTMetaL PRO 4.0::19971010::extensions to HTML 4.0//",
    "-//Spyglass//DTD HTML 2.0 Extended//",
    "-//Sun Microsystems Corp.//DTD HotJava HTML//",
    "-//Sun Microsystems Corp.//DTD HotJava Strict HTML//",
    "-//W3C//DTD HTML 3 1995-03-24//",
    "-//W3C//DTD HTML 3.2 Draft//",
    "-//W3C//DTD HTML 3.2 Final//",
    "-//W3C//DTD HTML 3.2//",
    "-//W3C//DTD HTML 3.2S Draft//",
    "-//W3C//DTD HTML 4.0 Frameset//",
    "-//W3C//DTD HTML 4.0 Transitional//",
    "-//W3C//DTD HTML Experimental 19960712//",
    "-//W3C//DTD HTML Experimental 970421//",
    "-//W3C//DTD W3 HTML//",
    "-//W3O//DTD W3 HTML 3.0//",
    "-//WebTechs//DTD Mozilla HTML 2.0//",
    "-//WebTechs//DTD Mozilla HTML//",
];

/// Public identifier prefixes that set the document to quirks mode only
/// when the system identifier is missing or the empty string
/// (§13.2.6.4.1) — the same two prefixes instead set *limited*-quirks
/// mode when a system identifier is present, see
/// [`LIMITED_QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITH_SYSTEM_IDENTIFIER`].
pub(super) const QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITHOUT_SYSTEM_IDENTIFIER: &[&str] = &[
    "-//W3C//DTD HTML 4.01 Frameset//",
    "-//W3C//DTD HTML 4.01 Transitional//",
];

/// Public identifier prefixes that set the document to limited-quirks
/// mode unconditionally (§13.2.6.4.1).
pub(super) const LIMITED_QUIRKS_PUBLIC_IDENTIFIER_PREFIXES: &[&str] = &[
    "-//W3C//DTD XHTML 1.0 Frameset//",
    "-//W3C//DTD XHTML 1.0 Transitional//",
];

/// Public identifier prefixes that set the document to limited-quirks
/// mode when the system identifier is present (neither missing nor the
/// empty string) — see
/// [`QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITHOUT_SYSTEM_IDENTIFIER`] for
/// the same two prefixes' quirks-mode counterpart.
pub(super) const LIMITED_QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITH_SYSTEM_IDENTIFIER: &[&str] = &[
    "-//W3C//DTD HTML 4.01 Frameset//",
    "-//W3C//DTD HTML 4.01 Transitional//",
];

pub(super) fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// "Determine the quirks mode" from a `DOCTYPE` token, per the condition
/// checks §13.2.6.4.1's "initial" insertion mode runs before switching to
/// "before html". Comparisons are ASCII case-insensitive per spec; a
/// system identifier of `Some("")` counts as present (not missing) — only
/// `None` counts as missing.
///
/// Deliberately doesn't model the spec's "iframe `srcdoc` document" or
/// "parser cannot change the mode flag" exceptions: this crate parses
/// whole documents only, never `srcdoc` content or HTML fragments (see
/// plan/03-tree-construction.md's scope decision), so both are always in
/// their default/false state here.
pub(crate) fn determine_quirks_mode(doctype: &DoctypeToken) -> QuirksMode {
    let name = doctype.name.as_deref();
    let public_id = doctype.public_identifier.as_deref();
    let system_id = doctype.system_identifier.as_deref();
    let system_id_missing_or_empty = system_id.is_none_or(str::is_empty);

    let quirks = doctype.force_quirks
        || name != Some("html")
        || public_id.is_some_and(|id| {
            QUIRKS_PUBLIC_IDENTIFIER_EXACT
                .iter()
                .any(|exact| id.eq_ignore_ascii_case(exact))
        })
        || system_id.is_some_and(|id| id.eq_ignore_ascii_case(QUIRKS_SYSTEM_IDENTIFIER_EXACT))
        || public_id.is_some_and(|id| {
            QUIRKS_PUBLIC_IDENTIFIER_PREFIXES
                .iter()
                .any(|prefix| starts_with_ignore_ascii_case(id, prefix))
        })
        || (system_id_missing_or_empty
            && public_id.is_some_and(|id| {
                QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITHOUT_SYSTEM_IDENTIFIER
                    .iter()
                    .any(|prefix| starts_with_ignore_ascii_case(id, prefix))
            }));
    if quirks {
        return QuirksMode::Quirks;
    }

    let limited_quirks = public_id.is_some_and(|id| {
        LIMITED_QUIRKS_PUBLIC_IDENTIFIER_PREFIXES
            .iter()
            .any(|prefix| starts_with_ignore_ascii_case(id, prefix))
    }) || (!system_id_missing_or_empty
        && public_id.is_some_and(|id| {
            LIMITED_QUIRKS_PUBLIC_IDENTIFIER_PREFIXES_WITH_SYSTEM_IDENTIFIER
                .iter()
                .any(|prefix| starts_with_ignore_ascii_case(id, prefix))
        }));
    if limited_quirks {
        QuirksMode::LimitedQuirks
    } else {
        QuirksMode::NoQuirks
    }
}
