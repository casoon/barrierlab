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

const XSL: &str = r#"xmlns:xsl="http://www.w3.org/1999/XSL/Transform""#;

fn with_functions(functions: &str, test: &str) -> String {
    format!(
        r#"<ns prefix="u" uri="utils"/>{functions}
        <pattern><rule context="/Invoice"><assert test="{test}">failed</assert></rule></pattern>"#
    )
}

// Peppol and XRechnung declare helper functions with `xsl:function` (check sums, IBAN). A call
// runs the body: parameters, variables, `xsl:sequence`, `xsl:choose`, `xsl:if`, `xsl:value-of`.
#[test]
fn xsl_functions_run() {
    let functions = format!(
        r#"<xsl:function {XSL} name="u:twice" as="xs:integer">
            <xsl:param name="n" as="xs:integer"/>
            <xsl:variable name="doubled" select="$n * 2"/>
            <xsl:sequence select="$doubled"/>
        </xsl:function>
        <xsl:function {XSL} name="u:sign" as="xs:string">
            <xsl:param name="n"/>
            <xsl:choose>
                <xsl:when test="$n &lt; 0"><xsl:sequence select="'negative'"/></xsl:when>
                <xsl:when test="$n = 0"><xsl:text>zero</xsl:text></xsl:when>
                <xsl:otherwise>
                    <xsl:if test="$n &gt; 100"><xsl:value-of select="'big'"/></xsl:if>
                    <xsl:if test="$n &lt;= 100"><xsl:value-of select="('posi', 'tive')" separator=""/></xsl:if>
                </xsl:otherwise>
            </xsl:choose>
        </xsl:function>
        <xsl:function {XSL} name="u:numerals" as="xs:string">
            <xsl:param name="s" as="xs:string"/>
            <xsl:variable name="allowed">0123456789</xsl:variable>
            <xsl:sequence select="translate($s, translate($s, $allowed, ''), '')"/>
        </xsl:function>"#
    );
    let parsed = parse(&schema("xslt2", &with_functions(&functions, "true()"))).unwrap();
    assert_eq!(parsed.xslt_functions, ["u:twice", "u:sign", "u:numerals"]);
    assert!(parsed.functions.iter().all(|f| f.unsupported.is_none()));
    // The arguments are nodes and sequences too: atomized and converted to the declared type.
    for test in [
        "u:twice(2) = 4",
        "u:twice(cac:Line[2]/cbc:ID) = 4",
        "u:sign(-1) = 'negative'",
        "u:sign(0) = 'zero'",
        "u:sign(5) = 'positive'",
        "u:sign(500) = 'big'",
        "u:numerals('a1b2c3') = '123'",
        "u:numerals(cbc:PayableAmount) = '3040'",
    ] {
        let found = messages("xslt2", &with_functions(&functions, test)).unwrap();
        assert_eq!(found, Vec::<String>::new(), "{test}");
    }
    // A wrong result is a failed assertion, so the passes above are not vacuous.
    let found = messages("xslt2", &with_functions(&functions, "u:twice(2) = 5")).unwrap();
    assert_eq!(found, ["failed"]);
}

// A function body sees the global `<let>`s (schema- and pattern-level), not a rule's, and has no
// context item: a path in it is the dynamic error XPDY0002, not a lookup in some document node.
#[test]
fn xsl_function_scope() {
    let functions = format!(
        r#"<xsl:function {XSL} name="u:limit" as="xs:integer"><xsl:sequence select="$limit"/></xsl:function>
        <xsl:function {XSL} name="u:rule" as="xs:integer"><xsl:sequence select="$inrule"/></xsl:function>
        <xsl:function {XSL} name="u:path" as="xs:string"><xsl:sequence select="string(cbc:PayableAmount)"/></xsl:function>"#
    );
    let schema = |test: &str| {
        format!(
            r#"<ns prefix="u" uri="utils"/>{functions}
            <let name="limit" value="7"/>
            <pattern><let name="local" value="8"/><rule context="/Invoice"><let name="inrule" value="9"/>
                <assert test="{test}">failed</assert></rule></pattern>"#
        )
    };
    assert_eq!(
        messages("xslt2", &schema("u:limit() = 7")).unwrap(),
        Vec::<String>::new()
    );
    let error = messages("xslt2", &schema("u:rule() = 9")).unwrap_err();
    assert!(error.to_string().contains("XPST0008"), "{error}");
    let error = messages("xslt2", &schema("u:path() = '30.40'")).unwrap_err();
    assert!(error.to_string().contains("XPDY0002"), "{error}");
}

// A recursive function runs; a call nested beyond the limit is an error, not a stack overflow.
#[test]
fn xsl_function_recursion_is_limited() {
    let functions = format!(
        r#"<xsl:function {XSL} name="u:sum-to" as="xs:integer">
            <xsl:param name="n" as="xs:integer"/>
            <xsl:sequence select="if ($n = 0) then 0 else $n + u:sum-to($n - 1)"/>
        </xsl:function>"#
    );
    let found = messages("xslt2", &with_functions(&functions, "u:sum-to(10) = 55")).unwrap();
    assert_eq!(found, Vec::<String>::new());
    let deepest = xpath_eval::MAX_FUNCTION_DEPTH - 1;
    let found = messages(
        "xslt2",
        &with_functions(&functions, &format!("u:sum-to({deepest}) > 0")),
    )
    .unwrap();
    assert_eq!(found, Vec::<String>::new());
    let error = messages("xslt2", &with_functions(&functions, "u:sum-to(100000) > 0")).unwrap_err();
    assert!(!error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("FOER0000"), "{error}");
}

// Argument and result types follow the function conversion rules: a value that does not fit is
// XPTY0004, a wrong arity is an unknown function.
#[test]
fn xsl_function_types_are_checked() {
    let functions = format!(
        r#"<xsl:function {XSL} name="u:len" as="xs:integer">
            <xsl:param name="s" as="xs:string"/>
            <xsl:sequence select="string-length($s)"/>
        </xsl:function>
        <xsl:function {XSL} name="u:bad" as="xs:integer"><xsl:sequence select="'text'"/></xsl:function>"#
    );
    let run = |test: &str| messages("xslt2", &with_functions(&functions, test));
    assert_eq!(run("u:len('abc') = 3").unwrap(), Vec::<String>::new());
    assert_eq!(
        run("u:len(cbc:PayableAmount) = 5").unwrap(),
        Vec::<String>::new()
    );
    for test in [
        "u:len(5) = 1",
        "u:len(('a', 'b')) = 1",
        "u:len(()) = 1",
        "u:bad() = 1",
    ] {
        let error = run(test).unwrap_err();
        assert!(error.to_string().contains("XPTY0004"), "{test}: {error}");
    }
    let error = run("u:len('a', 'b') = 1").unwrap_err();
    assert!(error.to_string().contains("XPST0017"), "{error}");
    // A function the schema does not declare stays what it is: an unknown function.
    let error = run("u:nothing(2) = 4").unwrap_err();
    assert!(!error.is_unsupported(), "{error}");
}

// What the interpreter does not run is reported as unsupported, with the construct named, and
// only when the function is called.
#[test]
fn xsl_function_with_unsupported_instruction_is_not_run() {
    let functions = format!(
        r#"<xsl:function {XSL} name="u:loop" as="xs:integer">
            <xsl:param name="n"/>
            <xsl:for-each select="1 to 3"><xsl:sequence select="."/></xsl:for-each>
        </xsl:function>"#
    );
    let schema_text = with_functions(&functions, "u:loop(1) = 1");
    let parsed = parse(&schema("xslt2", &schema_text)).unwrap();
    assert!(
        parsed.functions[0]
            .unsupported
            .as_deref()
            .unwrap()
            .contains("xsl:for-each")
    );
    let error = messages("xslt2", &schema_text).unwrap_err();
    assert!(error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("u:loop"), "{error}");
    assert!(error.to_string().contains("xsl:for-each"), "{error}");
    // Not called: nothing happens.
    let found = messages("xslt2", &with_functions(&functions, "true()")).unwrap();
    assert_eq!(found, Vec::<String>::new());
}

// A pattern with `<let>`s and no rule declares variables for the whole schema (XRechnung's
// `common.sch`), a pattern with rules keeps its `<let>`s to itself, and a rule context reads the
// variables in scope.
#[test]
fn variable_patterns_and_variables_in_contexts() {
    let found = messages(
        "xslt2",
        r#"<pattern id="vars"><let name="wanted" value="'cac:Line'"/><let name="count" value="3"/></pattern>
        <pattern><let name="own" value="1"/>
            <rule context="/Invoice[$count = 3]"><assert test="$wanted = 'cac:Line'">global</assert></rule></pattern>
        <pattern><rule context="/Invoice[$count = 4]"><assert test="false()">never</assert></rule></pattern>"#,
    )
    .unwrap();
    assert_eq!(found, Vec::<String>::new());
    // The variable of a pattern with a rule is not visible in another pattern.
    let error = messages(
        "xslt2",
        r#"<pattern><let name="own" value="1"/><rule context="/Invoice"><assert test="true()">a</assert></rule></pattern>
        <pattern><rule context="/Invoice"><assert test="$own = 1">b</assert></rule></pattern>"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("XPST0008"), "{error}");
}
