//! HTML5 conformance checking with browser-style error recovery.
//!
//! `check`/`check_with_options` combine six finding sources, in order:
//! HTML parser diagnostics, RELAX NG schema (content-model) validation,
//! Schematron-style assertion (co-constraint) checking,
//! `<script type="importmap"|"speculationrules">` JSON content validation
//! (`src/scripts.rs` — a value/content-format check like the RELAX NG
//! datatypes, just on element text content instead of an attribute), and
//! `<meta http-equiv="Content-Security-Policy">` enforcement against
//! inline script/style content elsewhere in the document
//! (`src/csp_enforcement.rs` — a genuine cross-element check needing a
//! real CSP source-list parser, not expressible as a `rules/*.sch` rule
//! or a `w:*` datatype), and table cell-grid integrity
//! (`src/table_integrity.rs` — laying a table out over its
//! `colspan`/`rowspan` values needs mutable state carried forward across
//! cells, which XPath 1.0 has no way to express).

// The old finding types are deprecated for hosts; the crate itself still builds them.
#![allow(deprecated)]

mod assertions;
mod csp_enforcement;
mod datatypes;
mod finding;
mod infoset;
mod parse;
mod rules;
mod schema;
#[cfg(any(test, feature = "runtime-compile", feature = "rdfa-lite"))]
mod schema_source;
mod scripts;
mod skipped;
mod table_integrity;

use assertions::SchematronEngine;

pub use finding::{
    CheckError, CheckReport, Finding, FindingDetail, RangedReport, Severity, Skipped, SkippedClass,
    SourceLocation, SourceRange,
};
pub use rules::{RuleInfo, RuleLayer, rules};

/// The deepest element nesting [`check_with_options`] validates; deeper
/// documents get [`CheckError::DocumentTooDeep`] instead of a report.
///
/// The validation layers after parsing (infoset normalisation, the
/// Schematron/XPath assertions, the script, CSP and table checks) walk the
/// tree recursively, one call frame per level. Unbounded, a few thousand
/// nested elements overflow a 2 MiB thread stack (tokio's worker default)
/// and abort the whole process. 512 is the depth at which Blink's HTML
/// parser stops nesting and starts appending further elements as siblings
/// (`kMaximumHTMLParserDOMTreeDepth`), so no page Chrome renders as
/// authored is refused. `tests/deep_nesting.rs` checks a document at
/// exactly this depth on a 2 MiB thread in a debug build.
///
/// The limit is not applied in the parser: WHATWG tree construction has no
/// depth limit, and flattening the tree the way Blink does would report
/// findings against a tree the specification does not build. The HTML
/// Standard allows implementation-specific limits on otherwise
/// unconstrained input; this checker uses that latitude by declining to
/// check, not by checking something else.
const MAX_NESTING_DEPTH: usize = 512;

/// Options that control which diagnostics are included in a check report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckOptions {
    /// Include recoverable HTML parser diagnostics in the report.
    pub include_parse_errors: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            include_parse_errors: true,
        }
    }
}

/// Which of vnu's schema presets [`check_with_preset`] checks against (feature `rdfa-lite`).
///
/// The default preset (what [`check`] and [`check_with_options`] use) is vnu's `html5-all`,
/// which accepts full RDFa 1.1. [`SchemaPreset::RdfaLite`] is vnu's `html5-rdfalite`: the same
/// schema with only the RDFa Lite 1.1 attributes (`vocab`, `typeof`, `property`, `resource`,
/// `prefix`); `about`, `content`, `datatype`, `rel`, `rev` and `inlist` are then not allowed
/// where they are RDFa-only. Without the feature this type does not exist, so a host cannot
/// select a preset the build does not contain.
#[cfg(feature = "rdfa-lite")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SchemaPreset {
    /// vnu's `html5-all`: full RDFa 1.1.
    #[default]
    Default,
    /// vnu's `html5-rdfalite`: RDFa Lite 1.1 only.
    RdfaLite,
}

/// Checks a complete HTML document with the default options.
///
/// Findings are returned in parser order. Technical initialization failures are
/// returned as [`CheckError`] rather than being represented as findings.
pub fn check(html: &str) -> Result<CheckReport, CheckError> {
    check_with_options(html, CheckOptions::default())
}

/// Checks a complete HTML document with explicit options.
///
/// This function always uses HTML5 error recovery. `include_parse_errors`
/// controls whether recovered parser diagnostics become report findings.
/// Schema and assertion findings are always included — the checker only
/// omits parser diagnostics on request, not the conformance findings that
/// are the point of running it at all.
///
/// # Errors
///
/// [`CheckError::DocumentTooDeep`] when the parsed tree nests deeper than
/// the checker validates (512 levels). No findings are returned then, not
/// even the parser's: the document was not checked, and a host must report
/// it as not run, never as passed.
///
/// [`CheckError::Initialization`] for a genuine setup failure in this
/// checker itself (the embedded HTML5 schema failed to compile, or the
/// embedded assertion rule set failed to parse).
///
/// Never for a document that is merely non-conformant, which is reported
/// through [`CheckReport::findings`] instead.
pub fn check_with_options(html: &str, options: CheckOptions) -> Result<CheckReport, CheckError> {
    check_impl(html, options, schema::validate_document).map(RangedReport::into_report)
}

/// Like [`check_with_options`], and additionally reports where each finding
/// ends where its layer knows it. See [`RangedReport`] for which findings
/// carry a range and the units of the positions. Not available together
/// with a `SchemaPreset` (feature `rdfa-lite`) yet.
///
/// # Errors
///
/// The same as [`check_with_options`].
pub fn check_with_ranges(html: &str, options: CheckOptions) -> Result<RangedReport, CheckError> {
    check_impl(html, options, schema::validate_document)
}

/// [`check_with_options`] against a chosen [`SchemaPreset`] (feature `rdfa-lite`).
/// `SchemaPreset::Default` gives exactly the result of [`check_with_options`].
///
/// # Errors
///
/// As [`check_with_options`].
#[cfg(feature = "rdfa-lite")]
pub fn check_with_preset(
    html: &str,
    options: CheckOptions,
    preset: SchemaPreset,
) -> Result<CheckReport, CheckError> {
    match preset {
        SchemaPreset::Default => check_impl(html, options, schema::validate_document),
        SchemaPreset::RdfaLite => check_impl(html, options, schema::validate_document_rdfa_lite),
    }
    .map(RangedReport::into_report)
}

type SchemaErrors = Vec<relax_ng::ValidationError<SourceLocation>>;

fn check_impl(
    html: &str,
    options: CheckOptions,
    validate_schema: fn(&infoset::NormalizedHtmlDocument) -> Result<SchemaErrors, String>,
) -> Result<RangedReport, CheckError> {
    let parsed = parse::parse(html);
    let depth = parse::nesting_depth(parsed.document());
    if depth > MAX_NESTING_DEPTH {
        return Err(CheckError::DocumentTooDeep {
            depth,
            limit: MAX_NESTING_DEPTH,
        });
    }
    let document = infoset::normalize(parsed.document(), parsed.source());

    let mut findings = if options.include_parse_errors {
        parse::findings(&parsed)
    } else {
        Vec::new()
    };

    let schema_errors =
        validate_schema(&document).map_err(|message| CheckError::Initialization {
            message: format!("schema validation setup failed: {message}"),
        })?;
    findings.extend(schema::findings(&schema_errors));

    let assertion_failures = assertions::RuleSetEngine
        .check(&document)
        .map_err(|error| CheckError::Initialization {
            message: format!("assertion engine setup failed: {error}"),
        })?;
    findings.extend(assertions::findings(&assertion_failures));

    findings.extend(scripts::findings(parsed.document()));
    findings.extend(csp_enforcement::findings(parsed.document()));
    findings.extend(table_integrity::findings(parsed.document()));

    let skipped = skipped::detect(parsed.document(), &findings);

    let ends = parse::start_tag_ends(parsed.document());
    let ranges = findings
        .iter()
        .map(|finding| {
            if finding.rule_id == parse::RULE_ID {
                return None;
            }
            let start = finding.location?;
            let end = *ends.get(&start.byte_offset)?;
            Some(SourceRange { start, end })
        })
        .collect();

    let paths = parse::element_paths(
        parsed.document(),
        findings
            .iter()
            .filter(|finding| finding.rule_id != parse::RULE_ID)
            .filter_map(|finding| finding.location.map(|location| location.byte_offset)),
    );

    Ok(RangedReport::new(
        CheckReport::new(findings, skipped, paths),
        ranges,
    ))
}

#[cfg(test)]
mod tests {
    use super::{CheckOptions, SourceLocation, check, check_with_options};

    #[test]
    fn valid_html_has_no_parser_findings() {
        let report = check(r#"<!doctype html><html lang="en"><title>Example</title><p>Hello</p>"#)
            .expect("HTML5 parsing should recover");

        assert!(report.findings.is_empty());
        assert!(!report.has_errors());
    }

    #[test]
    fn element_findings_get_a_structural_selector_and_a_stable_id() {
        let page = |gap: &str| {
            format!(
                "<!doctype html>{gap}<html lang=en><title>t</title><body><p>a</p>\n<table><tr><td>x</table>\n<ul><span>y</span></ul>&bogus;"
            )
        };
        let report = |html: &str| check(html).expect("check").to_report();
        let before = report(&page(""));
        let after = report(&page("\n\n<!-- c -->\n"));
        assert_eq!(before.findings.len(), after.findings.len());

        let element: Vec<_> = before
            .findings
            .iter()
            .filter(|f| f.rule_id != "parser.html5")
            .collect();
        assert!(!element.is_empty());
        let span = element
            .iter()
            .find(|f| f.rule_id == "schema.html5" && f.message.contains("span"))
            .expect("span in ul");
        assert_eq!(
            span.location.selector.as_deref(),
            Some("html > body > ul:nth-child(3) > span:nth-child(1)")
        );
        for f in &element {
            assert!(f.location.selector.is_some(), "{}", f.rule_id);
        }
        // Moving the document down by a few lines keeps every element id.
        let ids = |r: &a11y_report::Report| {
            r.findings
                .iter()
                .filter(|f| f.rule_id != "parser.html5")
                .map(|f| f.id())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&before), ids(&after));
        // Parser findings have no element anchor and keep the position id.
        let parser: Vec<_> = before
            .findings
            .iter()
            .filter(|f| f.rule_id == "parser.html5")
            .collect();
        assert!(!parser.is_empty());
        for f in parser {
            assert!(f.location.selector.is_none());
        }
        let parser_ids = |r: &a11y_report::Report| {
            r.findings
                .iter()
                .filter(|f| f.rule_id == "parser.html5")
                .map(|f| f.id())
                .collect::<Vec<_>>()
        };
        assert_ne!(parser_ids(&before), parser_ids(&after));
    }

    #[test]
    fn schema_findings_carry_structured_details() {
        use super::FindingDetail;

        let report = check(
            r#"<!doctype html><html lang="en"><title>t</title><body><p bogus="1">x</p><ul><span>y</span></ul><input type="search2">"#,
        )
        .expect("HTML5 parsing should recover");
        let details: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == "schema.html5")
            .filter_map(|finding| finding.detail.as_ref())
            .collect();

        assert!(details.contains(&&FindingDetail::UnexpectedAttribute {
            attribute: "bogus".to_owned(),
            element: "p".to_owned(),
        }));
        assert!(details.contains(&&FindingDetail::UnexpectedElement {
            element: "span".to_owned(),
        }));
        assert!(details.iter().any(|detail| matches!(
            detail,
            FindingDetail::InvalidAttributeValue { attribute, value, element }
                if attribute == "type" && value == "search2" && element == "input"
        )));
        assert!(
            report
                .findings
                .iter()
                .filter(|finding| finding.rule_id != "schema.html5")
                .all(|finding| finding.detail.is_none())
        );
    }

    #[test]
    fn svg_xlink_href_is_accepted_but_unknown_foreign_attributes_are_not() {
        // barrierlab#172: html5-parser keeps `xlink:href` as the source name
        // with the XLink namespace; the schema expects `{xlink}href`.
        let page = |svg: &str| {
            format!("<!doctype html><html lang=\"en\"><title>t</title><body>{svg}</body></html>")
        };
        let report = check(&page(
            r##"<svg><defs><circle id="a" r="1"/></defs><use xlink:href="#a"/></svg>"##,
        ))
        .expect("HTML5 parsing should recover");
        assert!(report.findings.is_empty(), "{:?}", report.findings);

        let report = check(&page(r##"<svg><use xlink:bogus="#a"/></svg>"##))
            .expect("HTML5 parsing should recover");
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn whitespace_before_a_leading_optional_child_is_ignored() {
        // barrierlab#41: inter-element whitespace before `legend` (and the
        // other "optionally X first, then flow content" models) is not text.
        for body in [
            "<fieldset> <legend>F</legend> <div>x</div></fieldset>",
            "<figure> <figcaption>F</figcaption> <img src=a.png alt=x></figure>",
            "<details> <summary>F</summary> <p>x</p></details>",
            "<table> <caption>F</caption> <tr><td>x</td></tr></table>",
        ] {
            let report = check(&format!(
                "<!doctype html><html lang=en><title>t</title>{body}"
            ))
            .expect("HTML5 parsing should recover");
            assert!(report.findings.is_empty(), "{body}: {:?}", report.findings);
        }
    }

    #[test]
    fn parser_diagnostics_can_be_excluded() {
        // Otherwise fully schema-conformant (has a <title>, per
        // schema/html5/meta.rnc's required head.inner) so that excluding
        // the recoverable parser diagnostic (an unknown entity reference)
        // leaves no findings at all — isolates this test to what it's
        // actually about (the `include_parse_errors` toggle), rather than
        // also depending on schema/assertion behavior.
        let report = check_with_options(
            r#"<!doctype html><html lang="en"><title>Example</title><p>&notAnEntity;</p>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert!(report.findings.is_empty());
    }

    #[test]
    fn parser_diagnostics_are_included_by_default() {
        let report =
            check(r#"<!doctype html><html lang="en"><title>Example</title><p>&notAnEntity;</p>"#)
                .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "parser.html5");
    }

    #[test]
    fn schema_violation_is_reported_as_a_finding() {
        // No <title> — schema/html5/meta.rnc's head.inner requires one.
        // Fires against the *synthesized* implicit `<head>` (no explicit
        // `<head>` tag in this input), so `location` is `None` here — see
        // `schema_violation_location_is_populated_for_an_explicit_element`
        // below for the populated case.
        let report = check_with_options(
            r#"<!doctype html><html lang="en"><p>Hello</p>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "schema.html5");
        assert_eq!(report.findings[0].location, None);
        assert!(report.has_errors());
    }

    #[test]
    fn schema_violation_location_is_populated_for_an_explicit_element() {
        // Phase 08: `relax_ng::Element::Location` became generic
        // (`src/infoset.rs` sets it to `crate::finding::SourceLocation`
        // directly) — a schema.html5 finding against an *explicit*
        // element now carries a real, structured position, not `None`.
        let report = check_with_options(
            r#"<!doctype html><html lang="en"><title>x</title><p bogus="1">hi</p>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule_id, "schema.html5");
        assert_eq!(
            report.findings[0].location,
            Some(SourceLocation {
                line: 1,
                column: 48,
                byte_offset: 47,
            })
        );
    }

    #[test]
    fn assertion_violation_is_reported_as_a_finding() {
        let report = check_with_options(
            r#"<!doctype html><html lang="en"><title>Example</title><div aria-hidden="true" tabindex="0">x</div>"#,
            CheckOptions {
                include_parse_errors: false,
            },
        )
        .expect("HTML5 parsing should recover");

        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].rule_id,
            "assertion.aria.hidden-not-focusable"
        );
    }

    /// `body` alone inside an otherwise valid document, so any finding is
    /// about `body` — the shape the four cases below were reported in.
    fn rule_ids_for_body(body: &str) -> Vec<String> {
        let html = format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
             <title>t</title></head>\n<body>{body}</body>\n</html>\n"
        );
        check(&html)
            .expect("HTML5 parsing should recover")
            .findings
            .into_iter()
            .map(|finding| finding.rule_id)
            .collect()
    }

    #[test]
    fn img_without_alt_is_reported() {
        assert_eq!(
            rule_ids_for_body(r#"<p><img src="a.png"></p>"#),
            ["assertion.elements.img-missing-alt"]
        );
    }

    #[test]
    fn button_inside_a_is_reported() {
        assert_eq!(
            rule_ids_for_body(r#"<a href="/"><button>x</button></a>"#),
            ["assertion.elements.button-in-a"]
        );
    }

    /// A block-level end tag with no matching element in scope
    /// (§13.2.6.4.7). html5-parser 0.3.0 dropped it silently; 0.4.0
    /// records `StrayEndTag`.
    #[test]
    fn stray_div_end_tag_is_reported() {
        assert_eq!(rule_ids_for_body("<p>x</p></div>"), ["parser.html5"]);
    }

    /// Misnested formatting elements, repaired by the adoption agency
    /// algorithm. html5-parser 0.3.0 recorded no parse error; 0.4.0
    /// records `MisnestedFormattingElement` (and `StrayEndTag` for the
    /// already-closed `</i>`).
    #[test]
    fn misnested_formatting_elements_are_reported() {
        assert!(
            rule_ids_for_body("<p><b><i>x</b></i></p>")
                .iter()
                .any(|rule_id| rule_id == "parser.html5")
        );
    }

    /// `name` next to RDFa `property` on `<meta>` is conforming
    /// (HTML+RDFa 1.1 §3.1 only drops the `name` requirement). Seen on
    /// gov.uk: `<meta name="title" property="og:title" content="…">`.
    #[test]
    fn meta_with_name_and_property_is_conforming() {
        let report = check(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
             <title>t</title>\
             <meta property=\"og:image\" content=\"https://example.org/a.png\">\
             <meta name=\"title\" property=\"og:title\" content=\"Welcome\">\
             </head>\n<body></body>\n</html>\n",
        )
        .expect("HTML5 parsing should recover");

        assert!(report.findings.is_empty(), "{:?}", report.findings);
    }

    /// RDFa 1.1 is part of vnu's default schema (barrierlab#173).
    #[test]
    fn rdfa_attributes_are_conforming_and_unknown_ones_are_not() {
        let page = |body: &str| {
            format!(
                "<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\">\
                 <title>t</title></head>\n<body vocab=\"http://schema.org/\">{body}</body>\n</html>\n"
            )
        };
        let report = check(&page(
            "<div typeof=\"Person\" prefix=\"ex: http://example.org/\">\
             <link property=\"ex:copy\" resource=\"_:a\">\
             <span property=\"name\" about=\"#me\">A</span></div>",
        ))
        .expect("HTML5 parsing should recover");
        assert!(report.findings.is_empty(), "{:?}", report.findings);

        let report =
            check(&page("<p propertyy=\"x\">A</p>")).expect("HTML5 parsing should recover");
        assert!(!report.findings.is_empty());
    }

    /// barrierlab#181: `xmlns:prefix` is reported on HTML elements (vnu does
    /// too), `rev` on `<a>` is accepted (vnu's legacy.rnc).
    #[test]
    fn xmlns_prefix_on_html_is_reported_and_rev_on_a_is_not() {
        assert!(
            !rule_ids_for_body(r#"<p xmlns:x="http://example.org/">A</p>"#).is_empty(),
            "xmlns:x on <p>"
        );
        assert!(rule_ids_for_body(r#"<a href="/" rev="made">A</a>"#).is_empty());
    }

    /// barrierlab#198 (HTML, "Attributes"): on a foreign element only
    /// `xmlns:xlink` bound to the XLink namespace is allowed.
    #[test]
    fn xmlns_prefix_on_svg_is_reported_except_xlink() {
        let xlink = r#"xmlns:xlink="http://www.w3.org/1999/xlink""#;
        assert!(rule_ids_for_body(&format!("<svg {xlink}><g {xlink}></g></svg>")).is_empty());
        for svg in [
            r#"<svg xmlns:xe="http://www.w3.org/2001/xml-events"></svg>"#,
            r#"<svg><g xmlns:xlink="http://example.net/bar"></g></svg>"#,
        ] {
            assert!(!rule_ids_for_body(svg).is_empty(), "{svg}");
        }
    }

    #[test]
    fn svg_a_inside_svg_a_is_reported() {
        let ids = rule_ids_for_body(r#"<svg><a href="/x"><a href="/x">x</a></a></svg>"#);
        assert!(
            ids.iter()
                .any(|id| id == "assertion.elements.svg-a-in-svg-a"),
            "{ids:?}"
        );
        assert!(rule_ids_for_body(r#"<svg><a href="/x"><text>x</text></a></svg>"#).is_empty());
    }

    #[test]
    fn input_type_search_is_conforming() {
        assert!(
            rule_ids_for_body(r#"<input type="search" name="q" aria-label="Search">"#).is_empty()
        );
    }

    /// Not a false positive: ARIA in HTML allows no role other than
    /// `searchbox` on `input type=search` without `list` (vnu's schema
    /// agrees). gov.uk's script adds `role="combobox"` to its search field;
    /// the conforming form is `type="text" role="combobox"`.
    #[test]
    fn combobox_role_on_input_type_search_is_reported() {
        assert!(
            rule_ids_for_body(
                r#"<input type="search" role="combobox" aria-expanded="false" aria-label="Search">"#
            )
            .iter()
            .any(|rule_id| rule_id == "schema.html5")
        );
        assert!(
            rule_ids_for_body(
                r#"<input type="text" role="combobox" aria-expanded="false" aria-label="Search">"#
            )
            .is_empty()
        );
    }
}

#[cfg(all(test, feature = "rdfa-lite"))]
mod rdfa_lite_tests {
    use super::{CheckOptions, SchemaPreset, check, check_with_preset};

    fn schema_findings(html: &str, preset: SchemaPreset) -> usize {
        check_with_preset(html, CheckOptions::default(), preset)
            .expect("HTML5 parsing should recover")
            .findings
            .iter()
            .filter(|finding| finding.rule_id == "schema.html5")
            .count()
    }

    #[test]
    fn rdfa_lite_preset_drops_the_non_lite_attributes_only() {
        let page = |attrs: &str| {
            format!(
                "<!doctype html><html lang=\"en\"><title>t</title><body><p {attrs}>x</p></body></html>"
            )
        };
        for lite in [
            r#"vocab="http://schema.org/""#,
            r#"typeof="Person""#,
            r#"property="name""#,
            r##"resource="#a""##,
            r#"prefix="a: http://a/""#,
        ] {
            assert_eq!(
                schema_findings(&page(lite), SchemaPreset::RdfaLite),
                0,
                "{lite}"
            );
        }
        for full in [
            r##"about="#a""##,
            r#"content="x""#,
            r#"datatype="xsd:string""#,
            r#"inlist="""#,
        ] {
            assert_eq!(
                schema_findings(&page(full), SchemaPreset::Default),
                0,
                "{full}"
            );
            assert!(
                schema_findings(&page(full), SchemaPreset::RdfaLite) > 0,
                "{full}"
            );
        }
    }

    #[test]
    fn default_preset_equals_check() {
        let html =
            r##"<!doctype html><html lang="en"><title>t</title><p about="#a" bogus="1">x</p>"##;
        let plain = check(html).expect("HTML5 parsing should recover");
        let preset = check_with_preset(html, CheckOptions::default(), SchemaPreset::Default)
            .expect("HTML5 parsing should recover");
        assert_eq!(plain.findings.len(), preset.findings.len());
    }
}
