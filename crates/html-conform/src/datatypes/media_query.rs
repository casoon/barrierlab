//! `w:media-query` (`MediaQuery.java`/`MediaCondition.java`). Used by the
//! `media=""` attribute (`<style>`/`<link>`/`<source>` — see
//! `schema/html5/{meta,embed,phrase,media}.rnc`'s `common.data.mediaquery`),
//! whose value is a comma-separated media-query **list** per the HTML/CSS
//! spec (not a single query).
//!
//! ## Not vnu-parity by research — vnu-parity is *unverifiable* here
//!
//! Unlike `w:content-security-policy` (`src/datatypes/csp.rs`), this type
//! is **not** a from-source-verified reimplementation of vnu's actual
//! behavior. `plan/05c-research-group-b.md` item 24 already flagged this as
//! an open question when it was first researched: vnu's `MediaQuery.java`
//! has no grammar of its own — it wraps the literal in a synthetic
//! `@media <literal> {}` rule and hands it to an embedded, vendored **W3C
//! CSS Validator** (`org.w3c.css.css.StyleSheetParser`, profile
//! `css3svg`), a large, general-purpose CSS3 validator, not a small,
//! single-purpose class like `htmlunit-csp`. That research already noted:
//! "vermutlich nicht die neueren Media-Queries-Level-4/5-Range-Syntax
//! abdeckend — nicht unabhängig gegen die genaue vendorte
//! css-validator-Version verifiziert". Reimplementing that engine's exact
//! accept/reject behavior from source (the way `csp.rs` did for
//! `htmlunit-csp`) is out of scope here — it would mean sourcing and
//! reading a full CSS3 validator, not a single directive-grammar class.
//!
//! **This function's actual normative basis is the CSS Media Queries
//! Level 4 specification directly** (via `media-query-parse`, whose own
//! normative basis is the same spec), not vnu's specific, uncertain
//! runtime behavior. This is a deliberate, documented deviation from the
//! "vnu-Parität als Default" principle (`plan/05c-datatype-library.md`) —
//! justified here because the default itself isn't knowably achievable,
//! not because a shortcut was preferred over research.
//!
//! ## Accept/reject rule
//!
//! The `media=""` value is a `<media-query-list>` (CSS Media Queries §3.2):
//! valid iff every comma-separated entry parses as a syntactically valid
//! `<media-query>` *and* passes the semantic checks below. A problem in
//! either layer, in any individual entry inside an otherwise-parseable
//! list, is not silently downgraded to "never matches" here (which is what
//! a real CSS engine's error recovery would do when *evaluating* a
//! stylesheet) — this is a **conformance check**, not an evaluator, and
//! `w:content-security-policy`'s research (`src/datatypes/csp.rs`)
//! established the same pattern for this crate: any per-item problem
//! invalidates the whole checked value.
//!
//! ## Semantic layer (media types, media features, value types)
//!
//! `media-query-parse` deliberately parses only the `<media-query>`
//! *grammar* (`<general-enclosed>` included, since spec §3 prose mandates
//! it as a forward-compatibility fallback, not a syntax error — see that
//! crate's `src/parser.rs` module doc comment and `CLAUDE.md`: it has no
//! "matches a real device" concept, so per-feature semantics are
//! deliberately its caller's problem, not its own). A real CSS engine
//! *evaluating* a stylesheet is exactly that caller, and per spec §4.4
//! treats an unrecognized media type/feature name or a value that doesn't
//! match the feature's declared type as "this feature/condition is always
//! false", not a syntax error — a forward-compatible, non-fatal outcome by
//! design, not something this project's own research invented.
//!
//! An authoring-conformance checker is not that caller, though — the same
//! way an unknown CSS property name is not a *syntax* error to a browser's
//! forgiving parser but still a reportable defect to a CSS validator, vnu
//! (`MediaQuery.java`, wrapping a vendored W3C CSS Validator, per this
//! module's header) flags exactly these cases: unrecognized/deprecated
//! media types, unrecognized media features, and value/feature type
//! mismatches. `check_media_recognized` below reimplements that layer,
//! **not** against vnu's specific vendored CSS3 validator (unverifiable —
//! see this module's header), but against the CSS Media Queries Level 4
//! media-type list (§3.2, `https://www.w3.org/TR/mediaqueries-4/`, fetched
//! directly, not from training-data memory) plus its per-feature
//! definition tables (§4–§7), extended with the widely-shipped, stable
//! Media Queries Level 5 discrete features
//! (`https://www.w3.org/TR/mediaqueries-5/` §§ on `prefers-*`/
//! `forced-colors`/`dynamic-range`/`video-dynamic-range`/`inverted-colors`/
//! `scripting`/`nav-controls` — `prefers-color-scheme` is corpus-confirmed
//! accepted by vnu via `html/elements/meta/media-without-theme-color-novalid.html`,
//! whose only expected finding is an unrelated `theme-color` constraint,
//! not a "Bad value" on the `(prefers-color-scheme: dark)` media
//! condition). This is a **documented heuristic approximation of vnu**,
//! not itself normative — CSS Media Queries Level 4/5 explicitly do not
//! require rejecting any of this at the syntax level (see above); the
//! project's owner chose to add it anyway after this specific scope
//! tradeoff was raised.
//!
//! Very new/still-drafty Level 5 features (`device-posture`,
//! `horizontal-viewport-segments`, `vertical-viewport-segments`,
//! `environment-blending`) are deliberately not included: no corpus
//! fixture exercises them, and their spec status is materially less
//! settled than the Level 4 table or the Level 5 features included above.
use media_query_parse::{
    MediaCondition, MediaConditionWithoutOr, MediaFeature, MediaInParens, MediaQuery, MfName,
    MfRange, MfValue, parse_media_query, parse_media_query_list,
};

use crate::datatypes::misc::CSS_LENGTH_UNITS;

/// The `sizes=""` microsyntax's per-entry media-condition prefix
/// (`w:source-size-list`, `src/datatypes/misc.rs::check_source_size_entry`)
/// is a bare `<media-condition>` — unlike `media=""`'s `<media-query-list>`
/// (see [`check_media_query`]), it has no `<media-type>` branch at all
/// (HTML spec's "parse a sizes attribute": each non-final entry is
/// `<media-condition> S+ <source-size-value>`, not `<media-query> S+
/// <source-size-value>`). Reuses `media-query-parse`'s full
/// `<media-query>` parser (there is no standalone `<media-condition>`-only
/// entry point upstream — see `misc.rs`'s `check_source_size_list` doc
/// comment for why that gap was previously left open) and the same
/// semantic layer as `check_media_query`, but only accepts the
/// `MediaQuery::Condition` branch — `MediaQuery::TypeQuery` (a bare media
/// type like `all`, with or without `and (...)`) is a syntactically valid
/// `<media-query>` but not a valid `<media-condition>`, so it's rejected
/// here even though [`check_media_query`] would accept it.
pub(crate) fn check_media_condition_only(value: &str) -> Result<(), String> {
    match parse_media_query(value) {
        Ok(MediaQuery::Condition(condition)) => check_condition(&condition),
        Ok(MediaQuery::TypeQuery { media_type, .. }) => Err(format!(
            "\"{}\" is a media type, not a media condition (media conditions must be \
             parenthesized, or start with \"not\")",
            media_type.0
        )),
        Ok(_) => Err("unrecognized media-query form".to_owned()),
        Err(error) => Err(format!("not a valid media condition: {error:?}")),
    }
}

/// `w:media-query` → `MediaQuery.java`. See this module's doc comment for
/// the accept/reject rule and why it isn't vnu-parity-by-research.
pub(crate) fn check_media_query(value: &str) -> Result<(), String> {
    let results = parse_media_query_list(value);
    if results.is_empty() {
        // `parse_media_query_list("")` still yields one (invalid) entry
        // for the empty string in practice — this branch only guards the
        // degenerate case of a list-splitting change upstream ever
        // producing zero entries for a non-empty input, not a real
        // observed behavior today.
        return Err("media query list must not be empty".to_string());
    }
    for (index, result) in results.into_iter().enumerate() {
        let query = result
            .map_err(|error| format!("entry {index} is not a valid media query: {error:?}"))?;
        check_media_recognized(&query)
            .map_err(|error| format!("entry {index} is not a valid media query: {error}"))?;
    }
    Ok(())
}

/// The only three non-deprecated CSS media types (MQ4 §3.2). MQ4 also
/// lists eight further "deprecated" types (`tty`/`tv`/`projection`/
/// `handheld`/`braille`/`embossed`/`aural`/`speech`) that a *browser* must
/// still recognize as syntactically valid (matching nothing) — but "Authors
/// must not use these media types", and vnu's corpus fixtures
/// (`html/media-queries/{tv,projection}-novalid.html`) confirm vnu enforces
/// exactly that "must not" as a reportable error, not just a recommendation.
const MEDIA_TYPES: &[&str] = &["all", "screen", "print"];

/// CSS `<resolution>` units (MQ4 §5.1's `<resolution>` value, plus the
/// `infinite` keyword MQ4 added alongside it).
const RESOLUTION_UNITS: &[&str] = &["dpi", "dpcm", "dppx"];

/// A media feature's declared value domain, per its MQ4/MQ5 definition
/// table (see this module's header for the exact sections/URLs).
enum ValueDomain {
    /// `<length>` (`width`/`height`) — a `<dimension>` in
    /// [`CSS_LENGTH_UNITS`], or the unitless literal `0`.
    Length,
    /// `<ratio>` (`aspect-ratio`).
    Ratio,
    /// `<resolution> | infinite` (`resolution`).
    Resolution,
    /// `<integer>` (`color`/`color-index`/`monochrome`) — a non-negative
    /// whole `<number>`, not a `<dimension>`.
    Integer,
    /// `<mq-boolean> = <integer [0,1]>` (`grid`) — exactly `0` or `1`.
    MqBoolean,
    /// A fixed, case-insensitive keyword set (every other feature below).
    Keywords(&'static [&'static str]),
}

/// One entry per recognized media feature: its declared value domain, and
/// whether it is MQ4's "range" type (supports the `min-`/`max-` name
/// prefixes and `<mf-range>` comparison syntax — MQ4 §2.4.1) as opposed to
/// "discrete" (neither).
struct FeatureSpec {
    domain: ValueDomain,
    is_range: bool,
}

/// The recognized media-feature table: name → spec. Range features per MQ4
/// §§4.1–4.3/5.1/6.1–6.3; discrete features per MQ4 §§4.4/5.2–5.6/6.4/7;
/// the trailing MQ5 group per this module's header.
const FEATURES: &[(&str, FeatureSpec)] = &[
    (
        "width",
        FeatureSpec {
            domain: ValueDomain::Length,
            is_range: true,
        },
    ),
    (
        "height",
        FeatureSpec {
            domain: ValueDomain::Length,
            is_range: true,
        },
    ),
    (
        "aspect-ratio",
        FeatureSpec {
            domain: ValueDomain::Ratio,
            is_range: true,
        },
    ),
    (
        "resolution",
        FeatureSpec {
            domain: ValueDomain::Resolution,
            is_range: true,
        },
    ),
    (
        "color",
        FeatureSpec {
            domain: ValueDomain::Integer,
            is_range: true,
        },
    ),
    (
        "color-index",
        FeatureSpec {
            domain: ValueDomain::Integer,
            is_range: true,
        },
    ),
    (
        "monochrome",
        FeatureSpec {
            domain: ValueDomain::Integer,
            is_range: true,
        },
    ),
    (
        "orientation",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["portrait", "landscape"]),
            is_range: false,
        },
    ),
    (
        "scan",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["interlace", "progressive"]),
            is_range: false,
        },
    ),
    (
        "grid",
        FeatureSpec {
            domain: ValueDomain::MqBoolean,
            is_range: false,
        },
    ),
    (
        "update",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "slow", "fast"]),
            is_range: false,
        },
    ),
    (
        "overflow-block",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "scroll", "paged"]),
            is_range: false,
        },
    ),
    (
        "overflow-inline",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "scroll"]),
            is_range: false,
        },
    ),
    (
        "color-gamut",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["srgb", "p3", "rec2020"]),
            is_range: false,
        },
    ),
    (
        "pointer",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "coarse", "fine"]),
            is_range: false,
        },
    ),
    (
        "hover",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "hover"]),
            is_range: false,
        },
    ),
    (
        "any-pointer",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "coarse", "fine"]),
            is_range: false,
        },
    ),
    (
        "any-hover",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "hover"]),
            is_range: false,
        },
    ),
    (
        "scripting",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "initial-only", "enabled"]),
            is_range: false,
        },
    ),
    (
        "nav-controls",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "back"]),
            is_range: false,
        },
    ),
    (
        "prefers-reduced-motion",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["no-preference", "reduce"]),
            is_range: false,
        },
    ),
    (
        "prefers-reduced-transparency",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["no-preference", "reduce"]),
            is_range: false,
        },
    ),
    (
        "prefers-contrast",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["no-preference", "less", "more", "custom"]),
            is_range: false,
        },
    ),
    (
        "forced-colors",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "active"]),
            is_range: false,
        },
    ),
    (
        "prefers-color-scheme",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["light", "dark"]),
            is_range: false,
        },
    ),
    (
        "dynamic-range",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["standard", "high"]),
            is_range: false,
        },
    ),
    (
        "video-dynamic-range",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["standard", "high"]),
            is_range: false,
        },
    ),
    (
        "inverted-colors",
        FeatureSpec {
            domain: ValueDomain::Keywords(&["none", "inverted"]),
            is_range: false,
        },
    ),
];

/// Exact (non-`min-`/`max-`-prefixed) feature-name lookup, used for
/// `<mf-boolean>` — `(min-width)` alone isn't a meaningful boolean-context
/// use per MQ4 §2.4.1/§2.4.2 (the prefixed forms exist only for the range
/// context), so boolean form requires the bare name.
fn lookup_feature_exact(name: &str) -> Option<&'static FeatureSpec> {
    FEATURES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, spec)| spec)
}

/// Feature-name lookup for `<mf-plain>`/`<mf-range>`, honoring the
/// `min-`/`max-` prefixes MQ4 §2.4.1 grants to "range"-type features only.
fn lookup_feature(name: &str) -> Option<&'static FeatureSpec> {
    if let Some(spec) = lookup_feature_exact(name) {
        return Some(spec);
    }
    for prefix in ["min-", "max-"] {
        if name.len() > prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(prefix) {
            let spec = lookup_feature_exact(&name[prefix.len()..])?;
            return spec.is_range.then_some(spec);
        }
    }
    None
}

/// Checks a single `<mf-value>` against a feature's declared
/// [`ValueDomain`].
fn check_value_domain(domain: &ValueDomain, value: &MfValue) -> Result<(), String> {
    match (domain, value) {
        (ValueDomain::Length, MfValue::Number(number)) if *number == 0.0 => Ok(()),
        (ValueDomain::Length, MfValue::Dimension { unit, .. })
            if CSS_LENGTH_UNITS
                .iter()
                .any(|u| u.eq_ignore_ascii_case(unit)) =>
        {
            Ok(())
        }
        (ValueDomain::Ratio, MfValue::Ratio { .. }) => Ok(()),
        (ValueDomain::Resolution, MfValue::Dimension { unit, .. })
            if RESOLUTION_UNITS
                .iter()
                .any(|u| u.eq_ignore_ascii_case(unit)) =>
        {
            Ok(())
        }
        (ValueDomain::Resolution, MfValue::Ident(ident))
            if ident.eq_ignore_ascii_case("infinite") =>
        {
            Ok(())
        }
        (ValueDomain::Integer, MfValue::Number(number))
            if *number >= 0.0 && number.fract() == 0.0 =>
        {
            Ok(())
        }
        (ValueDomain::MqBoolean, MfValue::Number(number)) if *number == 0.0 || *number == 1.0 => {
            Ok(())
        }
        (ValueDomain::Keywords(keywords), MfValue::Ident(ident))
            if keywords.iter().any(|k| k.eq_ignore_ascii_case(ident)) =>
        {
            Ok(())
        }
        _ => Err(format!(
            "value {value:?} does not match the feature's expected type"
        )),
    }
}

/// Validates one `<media-feature>` (from `<mf-boolean>`/`<mf-plain>`/
/// `<mf-range>`) against [`FEATURES`].
///
/// Uses `.0`/field access rather than tuple/struct destructuring
/// patterns: `MfName`, `GeneralEnclosed`, and the enums matched below are
/// all `#[non_exhaustive]` in `media-query-parse` — Rust forbids
/// positional tuple-struct patterns on a `#[non_exhaustive]` type from
/// outside its defining crate even when the field itself is `pub`, and
/// requires a trailing `..`/wildcard arm on such enums/structs so that a
/// future crate version adding a variant doesn't silently miscompile
/// here. The wildcard arms below fail closed (reject) rather than accept,
/// consistent with this being a conformance checker — see this module's
/// header. `Cargo.toml` pins this dependency to `=0.1.0` exactly, so no
/// such new variant can appear without an explicit version bump.
fn check_feature(feature: &MediaFeature) -> Result<(), String> {
    match feature {
        MediaFeature::Boolean(name) => lookup_feature_exact(&name.0)
            .map(|_| ())
            .ok_or_else(|| format!("unrecognized media feature \"{}\"", name.0)),
        MediaFeature::Plain { name, value } => {
            let spec = lookup_feature(&name.0)
                .ok_or_else(|| format!("unrecognized media feature \"{}\"", name.0))?;
            check_value_domain(&spec.domain, value)
        }
        MediaFeature::Range(range) => check_range(range),
        _ => Err("unrecognized media-feature form".to_owned()),
    }
}

/// `<mf-range>` only applies to "range"-type features (MQ4 §2.4.1) — a
/// discrete feature (e.g. `(orientation > landscape)`) is rejected here
/// even though the crate's grammar-only parser accepts the shape.
fn check_range(range: &MfRange) -> Result<(), String> {
    let (name, values): (&MfName, Vec<&MfValue>) = match range {
        MfRange::NameFirst { name, value, .. } => (name, vec![value]),
        MfRange::ValueFirst { name, value, .. } => (name, vec![value]),
        MfRange::Interval {
            name, lower, upper, ..
        } => (name, vec![lower, upper]),
        _ => return Err("unrecognized range form".to_owned()),
    };
    let spec = lookup_feature(&name.0)
        .ok_or_else(|| format!("unrecognized media feature \"{}\"", name.0))?;
    if !spec.is_range {
        return Err(format!("\"{}\" does not support range syntax", name.0));
    }
    for value in values {
        check_value_domain(&spec.domain, value)?;
    }
    Ok(())
}

fn check_in_parens(in_parens: &MediaInParens) -> Result<(), String> {
    match in_parens {
        MediaInParens::Condition(condition) => check_condition(condition),
        MediaInParens::Feature(feature) => check_feature(feature),
        MediaInParens::GeneralEnclosed(enclosed) => Err(format!(
            "unrecognized parenthesized condition (tokens: {:?})",
            enclosed.tokens
        )),
        _ => Err("unrecognized parenthesized-condition form".to_owned()),
    }
}

fn check_condition(condition: &MediaCondition) -> Result<(), String> {
    match condition {
        MediaCondition::Not(in_parens) => check_in_parens(in_parens),
        MediaCondition::And(items) | MediaCondition::Or(items) => {
            items.iter().try_for_each(check_in_parens)
        }
        _ => Err("unrecognized media-condition form".to_owned()),
    }
}

fn check_condition_without_or(condition: &MediaConditionWithoutOr) -> Result<(), String> {
    match condition {
        MediaConditionWithoutOr::Not(in_parens) => check_in_parens(in_parens),
        MediaConditionWithoutOr::And(items) => items.iter().try_for_each(check_in_parens),
        _ => Err("unrecognized media-condition form".to_owned()),
    }
}

/// The semantic layer described in this module's header: media type and
/// media feature/value recognition, on top of `media-query-parse`'s purely
/// syntactic result.
fn check_media_recognized(query: &MediaQuery) -> Result<(), String> {
    match query {
        MediaQuery::Condition(condition) => check_condition(condition),
        MediaQuery::TypeQuery {
            media_type,
            condition,
            ..
        } => {
            if !MEDIA_TYPES
                .iter()
                .any(|t| t.eq_ignore_ascii_case(&media_type.0))
            {
                return Err(format!("unrecognized media type \"{}\"", media_type.0));
            }
            condition
                .as_ref()
                .map_or(Ok(()), check_condition_without_or)
        }
        _ => Err("unrecognized media-query form".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::check_media_query;

    #[test]
    fn single_valid_query_is_ok() {
        assert!(check_media_query("screen and (min-width: 400px)").is_ok());
    }

    #[test]
    fn valid_list_is_ok() {
        assert!(check_media_query("screen, print").is_ok());
        assert!(check_media_query("(min-width: 400px), (max-width: 700px)").is_ok());
    }

    #[test]
    fn range_syntax_is_supported() {
        assert!(check_media_query("(400px <= width <= 700px)").is_ok());
    }

    #[test]
    fn invalid_single_query_is_rejected() {
        // `only` requires a following media type, which is missing here.
        assert!(check_media_query("only").is_err());
    }

    #[test]
    fn one_invalid_entry_invalidates_the_whole_list() {
        assert!(check_media_query("screen, only").is_err());
    }

    #[test]
    fn empty_value_is_rejected() {
        assert!(check_media_query("").is_err());
    }

    // The following pin down the semantic layer added on top of
    // `media-query-parse`'s syntax-only result (see this module's header)
    // — one test per corpus fixture in `html/media-queries` that was
    // previously a false negative (`check_media_query` returned `Ok` for a
    // value vnu reports as "Bad value").

    #[test]
    fn unrecognized_media_type_is_rejected() {
        // 002-novalid.html
        assert!(check_media_query("alla").is_err());
    }

    #[test]
    fn deprecated_media_type_is_rejected() {
        // projection-novalid.html, tv-novalid.html
        assert!(check_media_query("projection").is_err());
        assert!(check_media_query("tv and (scan: progressive)").is_err());
    }

    #[test]
    fn concatenated_media_type_is_rejected() {
        // 004-novalid.html, 005-novalid.html — not a `min-`/`max-`-prefix
        // case, just an unrecognized single ident.
        assert!(check_media_query("notscreen").is_err());
        assert!(check_media_query("onlyscreen").is_err());
    }

    #[test]
    fn trailing_semicolon_inside_feature_is_rejected() {
        // 008-novalid.html — falls back to `<general-enclosed>` in
        // `media-query-parse` since `<mf-plain>` doesn't parse past the
        // `;`; rejected here rather than treated as forward-compat.
        assert!(check_media_query("screen and (min-width: 400px;)").is_err());
    }

    #[test]
    fn unknown_length_unit_is_rejected() {
        // 009-novalid.html
        assert!(check_media_query("screen and (min-width: 400uu)").is_err());
    }

    #[test]
    fn nonzero_unitless_length_is_rejected() {
        // 010-novalid.html, 024-novalid.html
        assert!(check_media_query("screen and (min-width: 400)").is_err());
    }

    #[test]
    fn wrong_dimension_type_for_length_feature_is_rejected() {
        // 011-novalid.html — `dpi` is a resolution unit, not a length one.
        assert!(check_media_query("screen and (min-width: 400dpi)").is_err());
    }

    #[test]
    fn wrong_value_type_for_integer_feature_is_rejected() {
        // 019-novalid.html — `color` expects a plain `<integer>`, not a
        // `<dimension>`.
        assert!(check_media_query("screen and (color: 1em)").is_err());
    }

    #[test]
    fn removed_legacy_feature_name_is_rejected() {
        // device-aspect-ratio-novalid.html — dropped in MQ4, not in
        // `FEATURES`.
        assert!(check_media_query("screen and (device-aspect-ratio: 16/9)").is_err());
    }

    #[test]
    fn discrete_feature_does_not_accept_range_syntax() {
        assert!(check_media_query("(orientation > landscape)").is_err());
    }

    #[test]
    fn unrecognized_feature_name_is_rejected() {
        assert!(check_media_query("(made-up-feature: 1)").is_err());
    }

    // The following pin down that the semantic layer does *not* regress
    // any of `html/media-queries`' `*-isvalid.html`/`*-valid.html`
    // fixtures, all of which were already accepted before this layer
    // existed.

    #[test]
    fn unitless_zero_length_variants_are_still_accepted() {
        // 030/031/032/033-isvalid.html
        assert!(check_media_query("screen and (min-width: 0)").is_ok());
        assert!(check_media_query("screen and (min-width: 0.0)").is_ok());
        assert!(check_media_query("screen and (min-width: 00)").is_ok());
        assert!(check_media_query("screen and (min-width: .0)").is_ok());
    }

    #[test]
    fn integer_color_feature_is_still_accepted() {
        // 025/026/027-isvalid.html
        assert!(check_media_query("screen and (color: 0)").is_ok());
        assert!(check_media_query("screen and (color: 1)").is_ok());
        assert!(check_media_query("screen and (color: 2)").is_ok());
    }

    #[test]
    fn resolution_feature_is_still_accepted() {
        // 028-isvalid.html
        assert!(check_media_query("print and (min-resolution: 100dpi)").is_ok());
    }

    #[test]
    fn prefers_color_scheme_is_accepted() {
        // Corpus-confirmed accepted by vnu via
        // html/elements/meta/media-without-theme-color-novalid.html, whose
        // only expected finding is an unrelated `theme-color` constraint.
        assert!(check_media_query("(prefers-color-scheme: dark)").is_ok());
    }

    #[test]
    fn grid_boolean_feature_is_accepted() {
        assert!(check_media_query("(grid: 1)").is_ok());
        assert!(check_media_query("(grid: 0)").is_ok());
        assert!(check_media_query("(grid: 2)").is_err());
    }

    #[test]
    fn boolean_context_feature_is_accepted() {
        assert!(check_media_query("screen and (color)").is_ok());
        assert!(check_media_query("screen and (pointer)").is_ok());
    }
}
