//! `queryBinding="xslt2"`/`"xpath2"`: the schema's expressions run in XPath 2.0 mode of
//! `xpath-eval` (casoon/barrierlab#62, slice 6). Self-written schema and document; the
//! constructs are the ones the EN 16931 validation Schematron uses (`xpath-eval`'s
//! `CONFORMANCE.md`).

#![cfg(feature = "xpath2")]

mod support;

use schematron_engine::{SchematronEvalError, evaluate, parse};
use support::RoxmlDoc;

const INVOICE: &str = r#"<Invoice xmlns:cbc="urn:cbc" xmlns:cac="urn:cac">
    <cbc:DocumentCurrencyCode>eur</cbc:DocumentCurrencyCode>
    <cac:Line><cbc:ID> 1 </cbc:ID><cbc:Amount>10.10</cbc:Amount></cac:Line>
    <cac:Line><cbc:ID>2</cbc:ID><cbc:Amount>20.20</cbc:Amount></cac:Line>
    <cac:Line><cbc:ID>3</cbc:ID><cbc:Amount>0.1</cbc:Amount></cac:Line>
    <cbc:PayableAmount>30.40</cbc:PayableAmount>
</Invoice>"#;

fn schema(binding: &str, body: &str) -> String {
    format!(
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="{binding}">
            <ns prefix="cbc" uri="urn:cbc"/><ns prefix="cac" uri="urn:cac"/>
            {body}
        </schema>"#
    )
}

fn messages(binding: &str, body: &str) -> Result<Vec<String>, SchematronEvalError> {
    let schema = parse(&schema(binding, body)).unwrap();
    let doc = RoxmlDoc::parse(INVOICE);
    evaluate(&schema, &doc).map(|reports| reports.into_iter().map(|r| r.message).collect())
}

// ISO Schematron 2016, section 5.4.4 (queryBinding): `xslt2` and `xpath2` select XPath 2.0.
#[test]
fn xslt2_and_xpath2_run_in_xpath_2_0_mode() {
    for binding in ["xslt2", "xpath2", "XSLT2"] {
        let found = messages(
            binding,
            r#"<pattern><rule context="/Invoice">
                <assert test="upper-case(cbc:DocumentCurrencyCode) = 'EUR'">a</assert>
                <assert test="exists(cac:Line)">b</assert>
                <assert test="every $l in cac:Line satisfies string-length(normalize-space($l/cbc:ID)) = 1">c</assert>
                <assert test="(1, 2, 3) = 2">d</assert>
            </rule></pattern>"#,
        )
        .unwrap();
        assert_eq!(found, Vec::<String>::new(), "{binding}");
    }
}

// The same expression under queryBinding xslt is XPath 1.0 and fails there.
#[test]
fn xslt_binding_stays_xpath_1_0() {
    let error = messages(
        "xslt",
        r#"<pattern><rule context="/Invoice"><assert test="exists(cac:Line)">x</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(matches!(error, SchematronEvalError::Eval(_)), "{error}");
}

// XPath 2.0 section 3.2.1.1: a path step may be an expression (`cac:Line/(cbc:ID)`), and
// a rule context is matched like an XSLT pattern, anywhere in the document.
#[test]
fn rule_contexts_are_patterns() {
    let found = messages(
        "xslt2",
        r#"<pattern>
            <rule context="cbc:ID | cbc:Amount"><report test="true()">leaf <name/></report></rule>
        </pattern>
        <pattern><rule context="/Invoice/cac:Line/(cbc:Amount)"><report test="true()">amount</report></rule></pattern>
        <pattern><rule context="//cac:Line[2]"><report test="true()">second</report></rule></pattern>"#,
    )
    .unwrap();
    assert_eq!(found.iter().filter(|m| m.starts_with("leaf")).count(), 6);
    assert_eq!(found.iter().filter(|m| *m == "amount").count(), 3);
    assert_eq!(found.iter().filter(|m| *m == "second").count(), 1);
}

// F&O 6.2: xs:decimal arithmetic is exact (0.1 + 0.2 = 0.3), unlike the 1.0 number.
// A `let` keeps its sequence and type: `xs:decimal` is not turned into a double.
#[test]
fn let_variables_are_sequences_with_their_types() {
    let found = messages(
        "xslt2",
        r#"<let name="total" value="sum(for $a in //cac:Line/cbc:Amount return xs:decimal($a))"/>
        <pattern><rule context="/Invoice">
            <let name="amounts" value="cac:Line/cbc:Amount"/>
            <let name="payable" value="xs:decimal(cbc:PayableAmount)"/>
            <assert test="$total = $payable">total <value-of select="$total"/> differs from <value-of select="$payable"/></assert>
            <assert test="xs:decimal('0.1') + xs:decimal('0.2') = xs:decimal('0.3')">decimal</assert>
            <report test="count($amounts) = 3">lines: <value-of select="$amounts"/></report>
        </rule></pattern>"#,
    )
    .unwrap();
    // 10.10 + 20.20 + 0.1 = 30.40 exactly, so no assert fires; the sequence is joined by spaces.
    assert_eq!(found, ["lines: 10.10 20.20 0.1"]);
}

// F&O 7.6.2 fn:matches and 7.6.3 fn:replace, 7.4.5 fn:string-join.
#[test]
fn regular_expressions_and_string_functions() {
    let found = messages(
        "xslt2",
        r#"<pattern><rule context="/Invoice">
            <assert test="matches(cbc:PayableAmount, '^[0-9]+\.[0-9]{2}$')">pattern</assert>
            <report test="true()"><value-of select="replace(cbc:PayableAmount, '\.', ',')"/> <value-of select="string-join(cac:Line/normalize-space(cbc:ID), '+')"/></report>
        </rule></pattern>"#,
    )
    .unwrap();
    assert_eq!(found, ["30,40 1+2+3"]);
}

// A construct `xpath-eval` does not implement is an error that says so, never a pass:
// the schema is not silently run without that check.
#[test]
fn unsupported_features_are_reported_not_skipped() {
    let error = messages(
        "xslt2",
        r#"<pattern><rule context="/Invoice"><assert test="format-number(1, '0') = '1'">x</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("format-number"), "{error}");
}

// XPath 2.0 syntax errors keep the parse error variant.
#[test]
fn syntax_errors_are_parse_errors() {
    let error = messages(
        "xslt2",
        r#"<pattern><rule context="/Invoice"><assert test="1 +">x</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(matches!(error, SchematronEvalError::Parse(_)), "{error}");
}

// A rule context that selects values instead of nodes is an error, as in XPath 1.0 mode.
#[test]
fn context_must_select_nodes() {
    let error = messages(
        "xslt2",
        r#"<pattern><rule context="1 + 1"><assert test="true()">x</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(
        matches!(error, SchematronEvalError::ContextNotNodeSet { .. }),
        "{error}"
    );
}

// Peppol and XRechnung declare helper functions with `xsl:function` (check digits, IBAN). This
// crate does not run XSLT: calling one is reported as unsupported with its name, not as an
// unknown function and not skipped.
#[test]
fn xsl_function_declarations_are_named_when_called() {
    let body = r#"<ns prefix="u" uri="utils"/>
        <xsl:function xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="u:twice" as="xs:integer">
            <xsl:param name="n" as="xs:integer"/>
            <xsl:sequence select="$n * 2"/>
        </xsl:function>
        <pattern><rule context="/Invoice"><assert test="u:twice(2) = 4">x</assert></rule></pattern>"#;
    let parsed = parse(&schema("xslt2", body)).unwrap();
    assert_eq!(parsed.xslt_functions, ["u:twice"]);
    let error = messages("xslt2", body).unwrap_err();
    assert!(error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("u:twice"), "{error}");
    assert!(error.to_string().contains("xsl:function"), "{error}");

    // A function the schema does not declare stays what it is: an unknown function.
    let error = messages(
        "xslt2",
        r#"<ns prefix="u" uri="utils"/>
        <pattern><rule context="/Invoice"><assert test="u:nothing(2) = 4">x</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(!error.is_unsupported(), "{error}");
}
