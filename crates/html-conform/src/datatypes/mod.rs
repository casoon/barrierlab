//! The `http://whattf.org/datatype-draft` custom RELAX NG datatype library
//! used by the vendored vnu HTML5 schema (Phase 05c). Each submodule owns
//! one batch of `w:*` types (see `plan/05c-datatype-library.md` for the
//! batch split and `plan/05c-research-group-a.md`/`05c-research-group-b.md`
//! for the per-type research this was implemented from); this module ties
//! them all together into the actual [`relax_ng::DatatypeLibrary`] impl.
//!
//! **vnu-parity as the default** (see `plan/05c-datatype-library.md`,
//! "Verbindliches Prinzip"): every type replicates vnu's actual behavior,
//! including documented vnu-specific quirks/bugs, rather than a "corrected"
//! reading of the underlying spec — `html-conform` is positioned as
//! vnu-comparable, not vnu-stricter-or-looser.
//!
//! **All 50 of the 52 `w:*` names** referenced (directly or as
//! dead/commented-out references) by the vendored HTML5 schema that
//! actually correspond to a distinct vnu datatype are implemented here —
//! the remaining 2 (`w:sizes`/`w:default-style`) don't exist as distinct
//! datatypes in vnu itself (dead schema references, see the research
//! docs), so there is nothing left to implement.
//! `w:content-security-policy` (49th, `csp.rs`) and `w:media-query`
//! (50th, `media_query.rs`) were added once their respective sister
//! projects (`csp-parse`, `media-query-parse`) were published — see
//! `DECISIONS.md`. `w:xml-name` and `w:svg-pathdata` (51st/52nd,
//! `svg_mathml.rs`) were added once the vendored SVG 1.1/MathML 3 schema
//! modules (`schema/svg11/`, `schema/mml3/`) were wired in — the only two
//! `w:*` names those modules reference that the HTML5 modules don't.
//!
//! Registered with a [`relax_ng::DatatypeRegistry`] and wired into
//! [`crate::check`] via `src/schema.rs`'s `DATATYPE_REGISTRY`.

mod csp;
mod datetime;
mod language;
mod media_query;
mod misc;
mod network;
mod simple;
pub(crate) mod structural;
mod svg_mathml;

use relax_ng::{DatatypeContext, DatatypeLibrary};

/// The 50 implemented `w:*` local names (without the `w:` prefix — RELAX NG
/// resolves the prefix via the schema's `datatypes w = "..."` declaration
/// and passes only the local name to [`DatatypeLibrary`]).
const KNOWN_TYPES: &[&str] = &[
    "ID",
    "IDREF",
    "IDREFS",
    "non-empty-string",
    "string",
    "string-without-line-breaks",
    "zero",
    "integer",
    "integer-non-negative",
    "integer-positive",
    "float",
    "float-non-negative",
    "float-positive",
    "hash-name",
    "custom-element-name",
    "autocomplete-any",
    "browsing-context",
    "browsing-context-or-keyword",
    "keylabellist",
    "language",
    "rel-value",
    "sandbox-allow-list",
    "script-type",
    "microdata-property",
    "simple-color",
    "date",
    "datetime-local",
    "datetime-tz",
    "month",
    "time",
    "time-datetime",
    "week",
    "iri",
    "iri-ref",
    "iri-ref-http-or-https",
    "email-address",
    "email-address-list",
    "mime-type",
    "mime-type-list",
    "charset",
    "meta-charset",
    "integrity-metadata",
    "refresh",
    "color",
    "circle",
    "polyline",
    "rectangle",
    "source-size-list",
    "image-candidate-strings",
    "content-security-policy",
    "media-query",
    "xml-name",
    "svg-pathdata",
];

/// The `http://whattf.org/datatype-draft` datatype library. None of the 52
/// types take RNG parameters (facets) — every one is a plain value check.
pub(crate) struct WhatwgDatatypeLibrary;

impl DatatypeLibrary for WhatwgDatatypeLibrary {
    fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String> {
        if !KNOWN_TYPES.contains(&type_name) {
            return Err(format!(
                "unknown datatype `{type_name}` in library `http://whattf.org/datatype-draft`"
            ));
        }
        if !params.is_empty() {
            return Err(format!("datatype `{type_name}` does not take parameters"));
        }
        Ok(())
    }

    fn matches(
        &self,
        type_name: &str,
        _params: &[(&str, &str)],
        value: &str,
        _context: &DatatypeContext,
    ) -> bool {
        check(type_name, value).is_ok()
    }

    fn values_equal(
        &self,
        type_name: &str,
        expected: &str,
        _expected_context: &DatatypeContext,
        actual: &str,
        _actual_context: &DatatypeContext,
    ) -> bool {
        // `w:string`'s value comparison is ASCII-case-insensitive (vnu's
        // `AsciiCaseInsensitiveString.createValue` lowercases before
        // comparing) — every other type compares its raw text exactly.
        if type_name == "string" {
            simple::values_equal_ascii_case_insensitive(expected, actual)
        } else {
            expected == actual
        }
    }
}

fn check(type_name: &str, value: &str) -> Result<(), String> {
    match type_name {
        "ID" => simple::check_id(value),
        "IDREF" => simple::check_idref(value),
        "IDREFS" => simple::check_idrefs(value),
        "non-empty-string" => simple::check_non_empty_string(value),
        "string" => simple::check_string(value),
        "string-without-line-breaks" => simple::check_string_without_line_breaks(value),
        "zero" => simple::check_zero(value),
        "integer" => simple::check_integer(value),
        "integer-non-negative" => simple::check_integer_non_negative(value),
        "integer-positive" => simple::check_integer_positive(value),
        "float" => simple::check_float(value),
        "float-non-negative" => simple::check_float_non_negative(value),
        "float-positive" => simple::check_float_positive(value),
        "hash-name" => simple::check_hash_name(value),
        "custom-element-name" => structural::check_custom_element_name(value),
        "autocomplete-any" => structural::check_autocomplete_any(value),
        "browsing-context" => structural::check_browsing_context(value),
        "browsing-context-or-keyword" => structural::check_browsing_context_or_keyword(value),
        "keylabellist" => structural::check_keylabellist(value),
        "language" => language::check_language(value),
        "rel-value" => structural::check_rel_value(value),
        "sandbox-allow-list" => structural::check_sandbox_allow_list(value),
        "script-type" => structural::check_script_type(value),
        "microdata-property" => structural::check_microdata_property(value),
        "simple-color" => structural::check_simple_color(value),
        "date" => datetime::check_date(value),
        "datetime-local" => datetime::check_datetime_local(value),
        "datetime-tz" => datetime::check_datetime_tz(value),
        "month" => datetime::check_month(value),
        "time" => datetime::check_time(value),
        "time-datetime" => datetime::check_time_datetime(value),
        "week" => datetime::check_week(value),
        "iri" => network::check_iri(value),
        "iri-ref" => network::check_iri_ref(value),
        "iri-ref-http-or-https" => network::check_iri_ref_http_or_https(value),
        "email-address" => misc::check_email_address(value),
        "email-address-list" => misc::check_email_address_list(value),
        "mime-type" => network::check_mime_type(value),
        "mime-type-list" => network::check_mime_type_list(value),
        "charset" => network::check_charset(value),
        "meta-charset" => network::check_meta_charset(value),
        "integrity-metadata" => network::check_integrity_metadata(value),
        "refresh" => network::check_refresh(value),
        "color" => misc::check_color(value),
        "circle" => misc::check_circle(value),
        "polyline" => misc::check_polyline(value),
        "rectangle" => misc::check_rectangle(value),
        "source-size-list" => misc::check_source_size_list(value),
        "image-candidate-strings" => misc::check_image_candidate_strings(value),
        "content-security-policy" => csp::check_content_security_policy(value),
        "media-query" => media_query::check_media_query(value),
        "xml-name" => svg_mathml::check_xml_name(value),
        "svg-pathdata" => svg_mathml::check_svg_pathdata(value),
        _ => Err(format!("unknown datatype `{type_name}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::{KNOWN_TYPES, WhatwgDatatypeLibrary};
    use relax_ng::{DatatypeContext, DatatypeLibrary};

    #[test]
    fn all_known_types_validate_params_with_no_params() {
        let lib = WhatwgDatatypeLibrary;
        for &type_name in KNOWN_TYPES {
            assert!(
                lib.validate_params(type_name, &[]).is_ok(),
                "{type_name} should accept zero params"
            );
        }
    }

    #[test]
    fn validate_params_rejects_unknown_type() {
        let lib = WhatwgDatatypeLibrary;
        assert!(lib.validate_params("not-a-real-type", &[]).is_err());
    }

    #[test]
    fn validate_params_rejects_any_params() {
        let lib = WhatwgDatatypeLibrary;
        assert!(lib.validate_params("zero", &[("foo", "bar")]).is_err());
    }

    #[test]
    fn matches_dispatches_to_the_right_checker() {
        let lib = WhatwgDatatypeLibrary;
        let ctx = DatatypeContext::default();
        assert!(lib.matches("zero", &[], "0", &ctx));
        assert!(!lib.matches("zero", &[], "00", &ctx));
        assert!(lib.matches("simple-color", &[], "#ff0000", &ctx));
        assert!(!lib.matches("simple-color", &[], "red", &ctx));
    }

    #[test]
    fn matches_returns_false_for_unknown_type_instead_of_panicking() {
        let lib = WhatwgDatatypeLibrary;
        let ctx = DatatypeContext::default();
        assert!(!lib.matches("not-a-real-type", &[], "anything", &ctx));
    }

    #[test]
    fn values_equal_is_ascii_case_insensitive_for_string_only() {
        let lib = WhatwgDatatypeLibrary;
        let ctx = DatatypeContext::default();
        assert!(lib.values_equal("string", "Foo", &ctx, "foo", &ctx));
        assert!(!lib.values_equal("hash-name", "#Foo", &ctx, "#foo", &ctx));
    }
}
