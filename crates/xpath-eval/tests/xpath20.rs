//! XPath 2.0 mode. Normative: W3C XPath 2.0 (Second Edition, "XPath 2.0" below)
//! and XQuery 1.0 and XPath 2.0 Functions and Operators ("F&O"). Each test names
//! the section it is written from.

mod common;

use common::{N, parse_xml};
use xpath_eval::{
    Document, EvaluationContext, Item, Node, QName, Value, VersionedError, XPathVersion,
    evaluate_boolean_versioned, evaluate_versioned, parse_versioned,
};

const DOC: &str = r#"<inv xmlns:c="urn:c">
  <c:line><c:id> a1 </c:id><c:amt cur="EUR">10.10</c:amt><c:scheme><c:id>vat</c:id></c:scheme></c:line>
  <c:line><c:id>b2</c:id><c:amt cur="EUR">20.20</c:amt><c:scheme><c:id>tax</c:id></c:scheme></c:line>
  <c:line><c:id>c3</c:id><c:amt cur="USD">0.30</c:amt></c:line>
  <and>the element is named and</and>
  <date>2026-01-31</date><start>2026-01-01</start><bad>2026-02-30</bad>
  <n>3</n><word>abc</word>
</inv>"#;

fn namespaces(prefix: &str) -> Option<String> {
    (prefix == "c").then(|| "urn:c".to_owned())
}

fn run_in(
    version: XPathVersion,
    xml: &str,
    expr: &str,
) -> Result<Vec<Item<N<'static>>>, VersionedError> {
    // The document must outlive the nodes: leak it (test-only).
    let doc: &'static _ = Box::leak(Box::new(parse_xml(xml)));
    let root = doc.root();
    let element = root
        .children()
        .find(|c| c.expanded_name().is_some())
        .unwrap();
    let expression = parse_versioned(expr, version)?;
    let lookup = |p: &str| namespaces(p);
    let vars = |q: &QName| (q.local == "ext").then(|| Value::NodeSet(vec![element]));
    let mut ctx = EvaluationContext::new(element);
    ctx.namespaces = Some(&lookup);
    ctx.variables = Some(&vars);
    evaluate_versioned(&expression, &ctx)
}

/// Items rendered as `type:value` (nodes as `node:string-value`).
fn typed(expr: &str) -> Vec<String> {
    render(run_in(XPathVersion::V2, DOC, expr).unwrap_or_else(|e| panic!("{expr}: {e}")))
}

fn render(items: Vec<Item<N<'_>>>) -> Vec<String> {
    items
        .into_iter()
        .map(|i| match i {
            Item::Node(n) => format!("node:{}", n.string_value().trim()),
            Item::Atomic(a) => format!("{}:{}", a.type_name(), a.to_xpath_string()),
        })
        .collect()
}

/// Just the values.
fn vals(expr: &str) -> Vec<String> {
    run_in(XPathVersion::V2, DOC, expr)
        .unwrap_or_else(|e| panic!("{expr}: {e}"))
        .into_iter()
        .map(|i| match i {
            Item::Node(n) => n.string_value().trim().to_owned(),
            Item::Atomic(a) => a.to_xpath_string(),
        })
        .collect()
}

fn one(expr: &str) -> String {
    let v = vals(expr);
    assert_eq!(v.len(), 1, "{expr}: {v:?}");
    v.into_iter().next().unwrap()
}

fn err(expr: &str) -> VersionedError {
    match run_in(XPathVersion::V2, DOC, expr) {
        Ok(v) => panic!("{expr}: expected an error, got {:?}", render(v)),
        Err(e) => e,
    }
}

fn code(e: &VersionedError) -> &'static str {
    match e {
        VersionedError::Static { code, .. } | VersionedError::Dynamic { code, .. } => code,
        other => panic!("no error code: {other}"),
    }
}

fn is_true(expr: &str) -> bool {
    one(expr) == "true"
}

// ---- version switch -------------------------------------------------------

// XPath 1.0 stays the default and keeps its semantics: §3.4 compares a string
// and a number by converting the string to a number, XPath 2.0 §3.5.2 raises a type error.
#[test]
fn default_version_is_xpath_1() {
    assert_eq!(XPathVersion::default(), XPathVersion::V1);
    let v1 = run_in(XPathVersion::V1, DOC, "'1' = 1").unwrap();
    assert_eq!(render(v1), ["xs:boolean:true"]);
    assert_eq!(code(&err("'1' = 1")), "XPTY0004");
}

// 1.0 numbers are doubles (XPath 1.0 §3.5), 2.0 decimals are exact (F&O §6.2).
#[test]
fn decimal_arithmetic_differs_between_modes() {
    let v1 = run_in(XPathVersion::V1, DOC, "0.1 + 0.2 = 0.3").unwrap();
    assert_eq!(render(v1), ["xs:boolean:false"]);
    assert_eq!(one("0.1 + 0.2 = 0.3"), "true");
}

#[test]
fn xpath_2_syntax_is_a_parse_error_in_1_0_mode() {
    for expr in ["if (1) then 2 else 3", "1 to 3", "for $x in (1) return $x"] {
        assert!(matches!(
            parse_versioned(expr, XPathVersion::V1),
            Err(VersionedError::Parse(_))
        ));
    }
}

#[test]
fn version_follows_the_query_binding() {
    let v = XPathVersion::from_query_binding;
    assert_eq!(v(None), Some(XPathVersion::V1));
    assert_eq!(v(Some("xslt")), Some(XPathVersion::V1));
    assert_eq!(v(Some("xpath")), Some(XPathVersion::V1));
    assert_eq!(v(Some("xslt2")), Some(XPathVersion::V2));
    assert_eq!(v(Some("xpath2")), Some(XPathVersion::V2));
    assert_eq!(v(Some("xquery")), None);
}

#[test]
fn boolean_evaluation_follows_the_version() {
    let doc = parse_xml(DOC);
    let root = doc.root();
    let ctx = EvaluationContext::new(root);
    let one = parse_versioned("('a','b')", XPathVersion::V2).unwrap();
    // XPath 2.0 §2.4.3: two atomic values have no effective boolean value.
    assert!(evaluate_boolean_versioned(&one, &ctx).is_err());
    let nodes = parse_versioned("//*", XPathVersion::V2).unwrap();
    assert!(evaluate_boolean_versioned(&nodes, &ctx).unwrap());
    let v1 = parse_versioned("//nothing", XPathVersion::V1).unwrap();
    assert!(!evaluate_boolean_versioned(&v1, &ctx).unwrap());
}

// ---- §3.8, §3.9, §3.7, §3.3 ---------------------------------------------

// XPath 2.0 §3.8: only the selected branch is evaluated.
#[test]
fn conditional_expression() {
    assert_eq!(one("if (1 = 1) then 'a' else 'b'"), "a");
    assert_eq!(one("if (c:line/c:id) then 'some' else 'none'"), "some");
    assert_eq!(one("if (c:nothing) then 'some' else 'none'"), "none");
    assert_eq!(one("if (true()) then 1 else 1 div 0"), "1");
    assert_eq!(
        one("if (false()) then if (true()) then 1 else 2 else 3"),
        "3"
    );
}

// XPath 2.0 §3.9: `some` over the empty sequence is false, `every` is true.
#[test]
fn quantified_expressions() {
    assert!(is_true("every $x in (1, 2, 3) satisfies $x > 0"));
    assert!(!is_true("every $x in (1, 2, 3) satisfies $x > 1"));
    assert!(is_true("some $x in (1, 2, 3) satisfies $x > 2"));
    assert!(!is_true("some $x in () satisfies true()"));
    assert!(is_true("every $x in () satisfies false()"));
    assert!(is_true("some $x in (1, 2), $y in (2, 3) satisfies $x = $y"));
    assert!(!is_true(
        "every $x in (1, 2), $y in (2, 3) satisfies $x = $y"
    ));
    // The pattern of the EN 16931 rules: a variable bound to nodes, used in a predicate.
    assert!(is_true(
        "every $cur in c:line[1]/c:amt/@cur satisfies exists(c:line/c:amt[@cur = $cur])"
    ));
}

// XPath 2.0 §3.7: tuples in order, first binding outermost.
#[test]
fn for_expression() {
    assert_eq!(vals("for $x in (1, 2, 3) return $x * 2"), ["2", "4", "6"]);
    assert_eq!(
        vals("for $x in (1, 2), $y in (10, 20) return $x + $y"),
        ["11", "21", "12", "22"]
    );
    assert_eq!(vals("for $x in () return 1"), Vec::<String>::new());
    assert_eq!(
        vals("for $l in c:line return string($l/c:id)"),
        [" a1 ", "b2", "c3"]
    );
}

// XPath 2.0 §3.3: sequences are flat; §3.3.1 range.
#[test]
fn sequences_and_ranges() {
    assert_eq!(vals("(1, 2, (), (3, 4))"), ["1", "2", "3", "4"]);
    assert_eq!(one("count(())"), "0");
    assert_eq!(vals("1 to 4"), ["1", "2", "3", "4"]);
    assert!(vals("4 to 1").is_empty());
    assert!(vals("() to 3").is_empty());
    assert_eq!(one("count(1 to 100)"), "100");
    assert_eq!(code(&err("1 to word")), "FORG0001");
    assert_eq!(code(&err("1 to 'a'")), "XPTY0004");
    assert_eq!(code(&err("1 to 1.5")), "XPTY0004");
    assert_eq!(vals("(5, 6, 7)[2]"), ["6"]);
    assert_eq!(vals("(5, 6, 7)[. > 5]"), ["6", "7"]);
}

// ---- comparisons, arithmetic ---------------------------------------------

// XPath 2.0 §3.5.2: existential; untyped is cast to the other operand's type.
#[test]
fn general_comparisons() {
    assert!(is_true("(1, 2, 3) = 2"));
    assert!(is_true("(1, 2) != (1, 2)"));
    assert!(!is_true("() = ()"));
    assert!(is_true("n = 3"));
    assert!(is_true("n = 3.0"));
    assert!(is_true("n = '3'"));
    assert!(is_true("word = 'abc'"));
    assert!(is_true("c:line/c:amt/@cur = 'USD'"));
    assert_eq!(code(&err("word = 3")), "FORG0001");
    assert_eq!(code(&err("'1' = 1")), "XPTY0004");
    assert!(is_true("xs:date(date) >= xs:date(start)"));
    assert!(is_true("date >= xs:date('2026-01-01')"));
    assert!(is_true("c:line[1]/c:amt = 10.1"));
    assert!(is_true("c:line[1]/c:amt = xs:decimal('10.10')"));
}

// XPath 2.0 §3.5.1: empty operand gives empty; more than one item is an error.
#[test]
fn value_comparisons() {
    assert!(is_true("1 eq 1"));
    assert!(is_true("'a' lt 'b'"));
    assert!(is_true("2.5 gt 2"));
    assert!(vals("() eq 1").is_empty());
    assert_eq!(code(&err("(1, 2) eq 1")), "XPTY0004");
    assert_eq!(code(&err("'a' eq 1")), "XPTY0004");
    // Untyped operands are compared as strings under `eq`.
    assert!(is_true("word eq 'abc'"));
    assert_eq!(code(&err("n eq 3")), "XPTY0004");
    assert!(is_true("count(c:line) eq 3"));
    // NaN is unordered: `eq` false, `ne` true.
    assert!(!is_true("xs:double('NaN') eq xs:double('NaN')"));
    assert!(is_true("xs:double('NaN') ne xs:double('NaN')"));
}

// XPath 2.0 §3.5.3.
#[test]
fn node_comparisons() {
    assert!(is_true("c:line[1] is c:line[1]"));
    assert!(!is_true("c:line[1] is c:line[2]"));
    assert!(is_true("c:line[1] << c:line[2]"));
    assert!(is_true("c:line[2] >> c:line[1]"));
    assert!(vals("c:line[1] is c:nothing").is_empty());
    assert_eq!(code(&err("1 is 1")), "XPTY0004");
}

// XPath 2.0 §3.4, F&O §6.2.
#[test]
fn arithmetic() {
    assert_eq!(typed("5 div 2"), ["xs:decimal:2.5"]);
    assert_eq!(typed("4 div 2"), ["xs:decimal:2"]);
    assert_eq!(typed("5 idiv 2"), ["xs:integer:2"]);
    assert_eq!(typed("-5 idiv 2"), ["xs:integer:-2"]);
    assert_eq!(typed("5 mod 3"), ["xs:integer:2"]);
    assert_eq!(typed("-5 mod 3"), ["xs:integer:-2"]);
    assert_eq!(typed("5.5 mod 2"), ["xs:decimal:1.5"]);
    assert_eq!(typed("2 + 3 * 4 - 1"), ["xs:integer:13"]);
    assert_eq!(typed("1.5e0 + 1"), ["xs:double:2.5"]);
    assert_eq!(typed("0.1 + 0.2"), ["xs:decimal:0.3"]);
    assert_eq!(typed("n + 1"), ["xs:double:4"]);
    assert_eq!(typed("- -2"), ["xs:integer:2"]);
    assert_eq!(typed("-(1.5)"), ["xs:decimal:-1.5"]);
    assert_eq!(typed("+n"), ["xs:double:3"]);
    assert!(vals("() + 1").is_empty());
    assert!(vals("c:nothing * 2").is_empty());
    assert_eq!(code(&err("1 div 0")), "FOAR0001");
    assert_eq!(code(&err("1 idiv 0")), "FOAR0001");
    assert_eq!(typed("1e0 div 0"), ["xs:double:INF"]);
    assert_eq!(code(&err("(1, 2) + 1")), "XPTY0004");
    assert_eq!(code(&err("'a' + 1")), "XPTY0004");
    assert_eq!(code(&err("word + 1")), "FORG0001");
    // The EN 16931 amount check: exact to the cent.
    assert!(is_true(
        "xs:decimal(c:line[1]/c:amt) + xs:decimal(c:line[2]/c:amt) = 30.30"
    ));
    assert!(is_true(
        "xs:decimal(c:line[1]/c:amt) = round(sum(c:line[1]/xs:decimal(c:amt)) * 10 * 10) div 100"
    ));
}

#[test]
fn logical_operators_use_effective_boolean_value() {
    assert!(is_true("c:line and not(c:nothing)"));
    assert!(is_true("c:nothing or 'x'"));
    assert!(!is_true("() or 0"));
    assert_eq!(code(&err("(1, 2) and true()")), "FORG0006");
}

// ---- paths -------------------------------------------------------------

// XPath 2.0 §3.2: a step may be any expression, evaluated once per context
// node; atomic results are concatenated (the pattern `cac:TaxScheme/normalize-space(cbc:ID)`).
#[test]
fn function_call_as_path_step() {
    assert_eq!(vals("c:line/c:id/normalize-space(.)"), ["a1", "b2", "c3"]);
    assert_eq!(
        vals("c:line/c:scheme/normalize-space(upper-case(c:id))"),
        ["VAT", "TAX"]
    );
    assert!(is_true(
        "exists(c:line[c:scheme/normalize-space(upper-case(c:id)) = 'VAT'])"
    ));
    assert_eq!(vals("c:line/c:amt/xs:decimal(.)"), ["10.1", "20.2", "0.3"]);
    assert_eq!(one("sum(c:line/c:amt/xs:decimal(.))"), "30.6");
    assert_eq!(
        vals("c:line/c:scheme/(normalize-space(c:id) = 'vat')"),
        ["true", "false"]
    );
}

// XPath 2.0 §3.2: parenthesised expressions as steps, `/` and `//` with them.
#[test]
fn parenthesised_steps() {
    assert_eq!(one("count(//(c:id | c:amt))"), "8");
    assert_eq!(one("count((/inv|/nothing)/c:line)"), "3");
    assert_eq!(one("count(/inv/(c:line/c:id))"), "3");
    assert_eq!(one("count(/)"), "1");
    assert_eq!(one("count(//c:line[c:amt/@cur = 'EUR'])"), "2");
    assert_eq!(one("count(c:line/c:id/ancestor::*)"), "4");
    assert_eq!(vals("c:line[last()]/c:id"), ["c3"]);
    assert_eq!(vals("c:line[position() = 2]/c:id"), ["b2"]);
    assert_eq!(vals("(c:line/c:id)[2]"), ["b2"]);
    assert_eq!(
        vals("c:line[c:amt/@cur = 'USD']/c:id/preceding::c:id[1]"),
        ["tax"]
    );
}

// XPath 2.0 §3.2: nodes mixed with atomic values are XPTY0018, a non-node on
// the left of `/` is XPTY0019.
#[test]
fn path_type_errors() {
    assert_eq!(code(&err("c:line/(c:id, 1)")), "XPTY0018");
    assert_eq!(code(&err("(1, 2)/c:id")), "XPTY0019");
}

// XPath 2.0 §3.3.3.
#[test]
fn set_operators_give_document_order() {
    assert_eq!(vals("(c:line[2] | c:line[1])/c:id"), ["a1", "b2"]);
    assert_eq!(one("count(c:line union c:line[1])"), "3");
    assert_eq!(one("count(c:line intersect c:line[position() < 3])"), "2");
    assert_eq!(one("count(c:line except c:line[1])"), "2");
    assert_eq!(code(&err("(1) | (2)")), "XPTY0004");
}

#[test]
fn element_named_like_an_operator() {
    // `and` is a name test where an operand is expected (XPath 2.0 §A.2.2).
    assert_eq!(one("and"), "the element is named and");
    assert_eq!(one("count(/inv/and)"), "1");
    assert!(is_true("and and and"));
}

#[test]
fn variables_come_from_the_hook() {
    assert_eq!(one("count($ext/c:line)"), "3");
    assert_eq!(code(&err("$nothing")), "XPST0008");
}

// ---- casting, constructors (XPath 2.0 §3.10.2-§3.10.4) -------------------

#[test]
fn constructors_and_casts() {
    assert_eq!(typed("xs:decimal('1.50')"), ["xs:decimal:1.5"]);
    assert_eq!(typed("xs:decimal(c:line[1]/c:amt)"), ["xs:decimal:10.1"]);
    assert_eq!(typed("xs:decimal(c:nothing)"), Vec::<String>::new());
    assert_eq!(typed("xs:integer('  12 ')"), ["xs:integer:12"]);
    assert_eq!(typed("xs:integer(2.9)"), ["xs:integer:2"]);
    assert_eq!(typed("xs:double('1e3')"), ["xs:double:1000"]);
    assert_eq!(typed("xs:string(1.50)"), ["xs:string:1.5"]);
    assert_eq!(typed("xs:boolean('1')"), ["xs:boolean:true"]);
    assert_eq!(typed("'12' cast as xs:integer"), ["xs:integer:12"]);
    assert_eq!(typed("() cast as xs:integer?"), Vec::<String>::new());
    assert_eq!(code(&err("() cast as xs:integer")), "XPTY0004");
    assert_eq!(code(&err("xs:decimal('abc')")), "FORG0001");
    assert_eq!(code(&err("xs:decimal('1e2')")), "FORG0001");
    assert_eq!(code(&err("xs:decimal((1, 2))")), "XPTY0004");
    assert_eq!(code(&err("xs:date(bad)")), "FORG0001");
    assert_eq!(code(&err("xs:integer(xs:double('NaN'))")), "FOCA0002");
    assert_eq!(code(&err("xs:integer('9223372036854775808')")), "FOAR0002");
}

#[test]
fn castable() {
    assert!(is_true("'12' castable as xs:integer"));
    assert!(!is_true("'abc' castable as xs:integer"));
    assert!(is_true("() castable as xs:integer?"));
    assert!(!is_true("() castable as xs:integer"));
    assert!(!is_true("(1, 2) castable as xs:integer"));
    assert!(is_true("date castable as xs:date"));
    assert!(!is_true("bad castable as xs:date"));
    assert!(is_true("c:line[1]/c:amt castable as xs:decimal"));
    assert!(!is_true("c:line[1]/c:amt/@cur castable as xs:decimal"));
}

// XML Schema Part 2 §3.2.9, F&O §10: dates compare by value.
#[test]
fn dates() {
    assert!(is_true("xs:date('2026-01-31') gt xs:date('2026-01-01')"));
    assert!(is_true("xs:date('2024-02-29') lt xs:date('2024-03-01')"));
    assert_eq!(typed("xs:date('2026-01-31')"), ["xs:date:2026-01-31"]);
    assert_eq!(code(&err("xs:date('2026-02-29')")), "FORG0001");
    assert_eq!(code(&err("xs:date('2026-1-1')")), "FORG0001");
    assert_eq!(code(&err("xs:date('2026-01-01') = 5")), "XPTY0004");
}

// ---- functions ------------------------------------------------------------

// F&O §7: string functions. Arguments are `xs:string?`: untyped is accepted,
// other types are a type error.
#[test]
fn string_functions() {
    assert_eq!(one("upper-case('abC')"), "ABC");
    assert_eq!(one("lower-case('AbC')"), "abc");
    assert_eq!(one("upper-case(c:nothing)"), "");
    assert_eq!(one("normalize-space(c:line[1]/c:id)"), "a1");
    assert_eq!(one("normalize-space('  a   b ')"), "a b");
    assert_eq!(one("string-length(c:line[1]/c:id)"), "4");
    assert_eq!(one("string-length(())"), "0");
    assert!(is_true("ends-with('abc', 'bc')"));
    assert!(!is_true("ends-with('abc', 'ab')"));
    assert!(is_true("ends-with('abc', '')"));
    assert!(is_true("contains('abc', 'b')"));
    assert!(is_true("starts-with('abc', 'ab')"));
    assert_eq!(one("substring-before('a=b', '=')"), "a");
    assert_eq!(one("substring-after('a=b', '=')"), "b");
    assert_eq!(one("substring-after('abc', '')"), "abc");
    assert_eq!(one("substring('abcdef', 2, 3)"), "bcd");
    assert_eq!(one("substring('abcdef', 5)"), "ef");
    assert_eq!(one("substring('abc', 0, 2)"), "a");
    assert_eq!(one("translate('abcabc', 'ab', 'X')"), "XcXc");
    assert_eq!(one("string-join(('a', 'b', 'c'), '-')"), "a-b-c");
    assert_eq!(
        one("string-join(c:line/c:id/normalize-space(.), ',')"),
        "a1,b2,c3"
    );
    assert_eq!(one("concat('a', 1, 2.5, (), 'b')"), "a12.5b");
    assert_eq!(one("string(c:line[1]/c:id)"), " a1 ");
    assert_eq!(one("string(1.0)"), "1");
    assert_eq!(code(&err("upper-case(5)")), "XPTY0004");
    assert_eq!(code(&err("string-length(1)")), "XPTY0004");
    assert_eq!(code(&err("concat('a')")), "XPST0017");
}

// F&O §15: sequence functions and aggregates.
#[test]
fn sequence_functions() {
    assert!(is_true("exists(c:line)"));
    assert!(!is_true("exists(c:nothing)"));
    assert!(is_true("empty(c:nothing)"));
    assert!(is_true("exists(c:line[c:amt/@cur = 'USD'])"));
    assert_eq!(vals("distinct-values((1, 2, 1, 3, 2))"), ["1", "2", "3"]);
    assert_eq!(vals("distinct-values(('a', 'b', 'a'))"), ["a", "b"]);
    assert_eq!(vals("distinct-values((1, 1.0, '1'))"), ["1", "1"]);
    assert_eq!(vals("distinct-values(c:line/c:amt/@cur)"), ["EUR", "USD"]);
    assert_eq!(vals("reverse((1, 2, 3))"), ["3", "2", "1"]);
    assert_eq!(typed("sum(())"), ["xs:integer:0"]);
    assert_eq!(typed("sum((), 'none')"), ["xs:string:none"]);
    assert_eq!(typed("sum((1, 2, 3))"), ["xs:integer:6"]);
    assert_eq!(typed("sum((0.1, 0.2))"), ["xs:decimal:0.3"]);
    assert_eq!(typed("sum(c:line[1]/c:amt)"), ["xs:double:10.1"]);
    assert_eq!(typed("avg((1, 2))"), ["xs:decimal:1.5"]);
    assert!(vals("avg(())").is_empty());
    assert_eq!(typed("max((1, 3, 2))"), ["xs:integer:3"]);
    assert_eq!(typed("min((2.5, 1))"), ["xs:decimal:1"]);
    assert_eq!(typed("max(('a', 'c', 'b'))"), ["xs:string:c"]);
    assert_eq!(code(&err("sum(('a', 'b'))")), "FORG0006");
    assert_eq!(code(&err("max((1, 'a'))")), "XPTY0004");
    assert_eq!(code(&err("boolean((1, 2))")), "FORG0006");
}

// F&O §6.4: the result keeps the argument's type; `round` ties go toward +infinity.
#[test]
fn numeric_functions() {
    assert_eq!(typed("abs(-1.5)"), ["xs:decimal:1.5"]);
    assert_eq!(typed("abs(-2)"), ["xs:integer:2"]);
    assert_eq!(typed("abs(-1.5e0)"), ["xs:double:1.5"]);
    assert_eq!(
        typed("abs(xs:decimal(c:line[1]/c:amt)) - 1 < 10"),
        ["xs:boolean:true"]
    );
    assert_eq!(typed("round(2.5)"), ["xs:decimal:3"]);
    assert_eq!(typed("round(-2.5)"), ["xs:decimal:-2"]);
    assert_eq!(typed("round(2.4)"), ["xs:decimal:2"]);
    assert_eq!(typed("round(2.5e0)"), ["xs:double:3"]);
    assert_eq!(typed("round(7)"), ["xs:integer:7"]);
    assert_eq!(typed("floor(-1.2)"), ["xs:decimal:-2"]);
    assert_eq!(typed("ceiling(-1.2)"), ["xs:decimal:-1"]);
    assert!(vals("round(())").is_empty());
    assert_eq!(typed("number('x')"), ["xs:double:NaN"]);
    assert_eq!(typed("number(n)"), ["xs:double:3"]);
    assert_eq!(code(&err("abs('a')")), "XPTY0004");
    // The pattern of the amount checks: rounding to the cent.
    assert_eq!(
        typed("round(10.105 * 10 * 10) div 100"),
        ["xs:decimal:10.11"]
    );
}

#[test]
fn name_and_focus_functions() {
    assert_eq!(one("name(c:line[1])"), "c:line");
    assert_eq!(one("local-name(c:line[1])"), "line");
    assert_eq!(one("namespace-uri(c:line[1])"), "urn:c");
    assert_eq!(one("c:line[1]/name()"), "c:line");
    assert_eq!(one("name(c:nothing)"), "");
    assert_eq!(vals("c:line/position()"), ["1", "2", "3"]);
    assert_eq!(vals("c:line/last()"), ["3", "3", "3"]);
    assert_eq!(vals("data(c:line[1]/c:amt)"), ["10.10"]);
}

// ---- parser ---------------------------------------------------------------

// XPath 2.0 §A.2.3.1 comments, §A.2.1 literals.
#[test]
fn lexical_forms() {
    assert_eq!(one("1 (: a (: nested :) comment :) + 2"), "3");
    assert_eq!(one("'it''s'"), "it's");
    assert_eq!(one("\"say \"\"hi\"\"\""), "say \"hi\"");
    assert_eq!(typed(".5"), ["xs:decimal:0.5"]);
    assert_eq!(typed("1e2"), ["xs:double:100"]);
    assert_eq!(typed("1.5E-1"), ["xs:double:0.15"]);
    assert_eq!(typed("12"), ["xs:integer:12"]);
    assert_eq!(typed("1.0"), ["xs:decimal:1"]);
    assert!(matches!(
        parse_versioned("1 (: open", XPathVersion::V2),
        Err(VersionedError::Parse(_))
    ));
    assert!(matches!(
        parse_versioned("'open", XPathVersion::V2),
        Err(VersionedError::Parse(_))
    ));
}

// XPath 2.0 §3 precedence: or < and < comparison < to < +,- < *,div < union < unary.
#[test]
fn operator_precedence() {
    assert_eq!(vals("1 to 2 + 1"), ["1", "2", "3"]);
    assert!(is_true("1 + 1 = 2 and 2 * 2 = 4 or false()"));
    assert_eq!(one("-2 + 5"), "3");
    assert_eq!(one("2 * 3 mod 4"), "2");
    assert_eq!(one("10 - 2 - 3"), "5");
    assert!(is_true("1 eq 1 or 1 eq 2"));
    assert_eq!(one("count(c:line | c:line)"), "3");
}

#[test]
fn hyphen_in_names() {
    // `a-b` is one name, `a - b` a subtraction.
    assert_eq!(one("count(/inv/c:line) - count(/inv/c:line)"), "0");
    assert!(vals("c:line-x").is_empty());
}

#[test]
fn syntax_errors() {
    for expr in [
        "",
        "1 +",
        "(1",
        "if (1) then 2",
        "for $x in 1",
        "$",
        "a[",
        "1 2",
        "@",
        "child::",
        "foo::a",
        "cast as xs:integer",
        "1 cast xs:integer",
    ] {
        assert!(
            matches!(
                parse_versioned(expr, XPathVersion::V2),
                Err(VersionedError::Parse(_))
            ),
            "{expr:?}"
        );
    }
}

// ---- unsupported features are errors, never a result ------------------------

#[test]
fn unsupported_features_are_reported_as_such() {
    for expr in [
        "matches('aa', '(a)\\1')",
        "round-half-to-even(2.5)",
        "compare('a', 'b')",
        "'a' instance of xs:string",
        "1 treat as xs:integer",
        "xs:dateTime('2026-01-01T00:00:00')",
        "xs:duration('P1D')",
        "'1' cast as xs:float",
        "xs:date('2026-01-01') - xs:date('2025-01-01')",
        "xs:date('2026-01-01Z')",
        "year-from-date(xs:date('2026-01-01'))",
        "c:line/element()",
        "/inv/element(c:line)",
        "//*:line",
        "c:line/namespace::*",
        "99999999999999999999",
    ] {
        let e = match run_in(XPathVersion::V2, DOC, expr) {
            Err(e) => e,
            Ok(v) => panic!("{expr}: expected unsupported, got {:?}", render(v)),
        };
        assert!(e.is_unsupported(), "{expr}: {e}");
        assert!(e.to_string().starts_with("unsupported XPath 2.0 feature: "));
    }
}

// XPath 2.0 §2.3.1: unknown function (XPST0017), unbound prefix (XPST0081) are
// static errors, not silent empty results as in the 1.0 evaluator.
#[test]
fn static_errors() {
    assert_eq!(code(&err("nonsense(1)")), "XPST0017");
    assert_eq!(code(&err("c:nonsense(1)")), "XPST0017");
    assert_eq!(code(&err("upper-case()")), "XPST0017");
    assert_eq!(code(&err("xs:nonsense(1)")), "XPST0017");
    assert_eq!(code(&err("zz:a")), "XPST0081");
    assert_eq!(code(&err("//zz:*")), "XPST0081");
    assert_eq!(code(&err("zz:f(1)")), "XPST0081");
}

#[test]
fn xs_and_fn_prefixes_are_predefined() {
    assert_eq!(one("fn:upper-case('a')"), "A");
    assert_eq!(one("xs:string(1)"), "1");
}

// ---- review fixes ---------------------------------------------------------

// F&O 17.1.2 / XML Schema: decimal to double is one correctly rounded conversion.
#[test]
fn decimal_to_double_rounds_once() {
    assert!(is_true("0.000000000000000000000000000001 = 1e-30"));
    assert_eq!(typed("xs:decimal(1e-30) * 1e0"), ["xs:double:1.0E-30"]);
}

// XPath 2.0 3.1.5: a function with an arity that F&O or XSLT 2.0 defines but this
// crate lacks is unsupported; XPST0017 only for what exists in neither.
#[test]
fn known_functions_without_implementation_are_unsupported() {
    for expr in [
        "contains('a', 'a', 'http://collation')",
        "max((1, 2), 'http://collation')",
        "distinct-values((1, 2), 'http://collation')",
        "key('k', 'v')",
        "document('x.xml')",
        "current()",
        "format-number(1, '0.00')",
        "fn:round-half-to-even(2.5, 1)",
    ] {
        match run_in(XPathVersion::V2, DOC, expr) {
            Err(e) => assert!(e.is_unsupported(), "{expr}: {e}"),
            Ok(v) => panic!("{expr}: got {:?}", render(v)),
        }
    }
    assert_eq!(code(&err("concat('a')")), "XPST0017");
    assert_eq!(code(&err("contains('a')")), "XPST0017");
    assert_eq!(code(&err("max()")), "XPST0017");
    assert_eq!(code(&err("not-a-function(1)")), "XPST0017");
}

// F&O 6.3.2 fn:number takes at most one item.
#[test]
fn number_of_a_sequence_is_a_type_error() {
    assert_eq!(code(&err("number((1, 2))")), "XPTY0004");
    assert_eq!(typed("number(())"), ["xs:double:NaN"]);
}

// F&O 17.1.3: -2^63 is a valid xs:integer, 2^63 is not.
#[test]
fn integer_range_edges() {
    assert_eq!(
        typed("xs:integer(-9223372036854775808e0)"),
        ["xs:integer:-9223372036854775808"]
    );
    assert_eq!(code(&err("xs:integer(9223372036854775808e0)")), "FOAR0002");
    assert_eq!(code(&err("xs:integer(-9223372036854777856e0)")), "FOAR0002");
}

// Beyond 38 places is the limit of this implementation: never FORG0001 or FOAR0002.
#[test]
fn decimal_beyond_the_implementation_limit_is_unsupported() {
    let lit = "0.000000000000000000000000000000000000001";
    let long = "1".repeat(40);
    for expr in [
        format!("{lit} + 1"),
        format!("xs:decimal('{lit}')"),
        format!("xs:decimal('{long}')"),
        format!("{long}.5"),
        "xs:decimal(1e-40)".to_owned(),
    ] {
        let e = err(&expr);
        assert!(e.is_unsupported(), "{expr}: {e}");
    }
    assert_eq!(
        typed("0.00000000000000000000000000000000000001 + 1"),
        ["xs:decimal:1.00000000000000000000000000000000000001"]
    );
}

// F&O 6.4.4 fn:round: -0.5e0 rounds to negative zero.
#[test]
fn round_keeps_the_sign_of_zero() {
    assert_eq!(typed("round(-0.5e0)"), ["xs:double:-0"]);
    assert_eq!(typed("round(-0.4e0)"), ["xs:double:-0"]);
    assert_eq!(typed("round(0.5e0)"), ["xs:double:1"]);
}

// Nesting and chain limits: a parse error, never a stack overflow.
#[test]
fn deep_expressions_are_rejected_not_fatal() {
    let parse_fails = |expr: String| {
        assert!(
            matches!(
                parse_versioned(&expr, XPathVersion::V2),
                Err(VersionedError::Parse(_))
            ),
            "{}",
            &expr[..expr.len().min(40)]
        );
    };
    parse_fails(format!("{}1{}", "(".repeat(600), ")".repeat(600)));
    parse_fails(format!("{}1{}", "not(".repeat(600), ")".repeat(600)));
    parse_fails(format!("1{}", "+1".repeat(2000)));
    parse_fails(format!("1{}", " * 1".repeat(2000)));
    parse_fails(format!("a{}", "/a".repeat(2000)));
    parse_fails(format!("{}1", "if (1) then 1 else ".repeat(600)));
    parse_fails(format!("for {} return 1", vec!["$a in 1"; 600].join(", ")));
    // Within the limit it works, in the default test-thread stack.
    assert_eq!(one(&format!("1{}", "+1".repeat(100))), "101");
    assert_eq!(one(&format!("{}1{}", "(".repeat(20), ")".repeat(20))), "1");
    assert_eq!(
        one(&format!("{}true(){}", "not(".repeat(20), ")".repeat(20))),
        "true"
    );
    // `or`/`and` chains are walked without recursion.
    assert!(is_true(&format!(
        "false(){} or true()",
        " or false()".repeat(3000)
    )));
    assert!(!is_true(&format!(
        "true(){} and false()",
        " and true()".repeat(3000)
    )));
    parse_fails(format!("true(){}", " and true()".repeat(5000)));
}

// F&O 7.2.2 fn:string-to-codepoints and 7.2.1 fn:codepoints-to-string (used by the check-digit
// functions of the Peppol BIS Billing 3.0 rules, e.g. `reverse(for $i in
// string-to-codepoints(...) return $i - 48)`).
#[test]
fn codepoint_functions() {
    assert_eq!(
        vals("string-to-codepoints('Thérèse')"),
        ["84", "104", "233", "114", "232", "115", "101"]
    );
    assert!(vals("string-to-codepoints('')").is_empty());
    assert!(vals("string-to-codepoints(())").is_empty());
    assert_eq!(typed("string-to-codepoints('9')"), ["xs:integer:57"]);
    assert_eq!(one("string-to-codepoints('9') - 48"), "9");
    assert_eq!(one("codepoints-to-string((84, 233, 115))"), "Tés");
    assert_eq!(one("codepoints-to-string(128512)"), "\u{1F600}");
    assert_eq!(one("codepoints-to-string(())"), "");
    // The digits of a number as strings, as the Greek TIN check does.
    assert_eq!(
        vals("for $ch in string-to-codepoints('123') return codepoints-to-string($ch)"),
        ["1", "2", "3"]
    );
    // 0 and a surrogate are no XML characters (FOCH0001); a string is no integer (XPTY0004).
    assert_eq!(code(&err("codepoints-to-string(0)")), "FOCH0001");
    assert_eq!(code(&err("codepoints-to-string(55296)")), "FOCH0001");
    assert_eq!(code(&err("codepoints-to-string('a')")), "XPTY0004");
    assert_eq!(code(&err("string-to-codepoints(1)")), "XPTY0004");
}

// The sequence-valued variable hook binds a variable to a sequence with its atomic types; it is
// consulted before the 1.0 `variables` hook. `sequence_to_string` joins atomized items with a space.
#[test]
fn sequence_variables_keep_sequences_and_types() {
    use xpath_eval::{Atomic, sequence_to_string};
    let doc = parse_xml("<a><b>x</b><b>y</b></a>");
    let root = doc.root();
    let bound = |q: &QName| match q.local.as_str() {
        "i" => Some(vec![Item::Atomic(Atomic::Integer(5))]),
        "s" => Some(vec![
            Item::Atomic(Atomic::Integer(1)),
            Item::Atomic(Atomic::String("two".to_owned())),
        ]),
        "n" => Some(
            root.children()
                .flat_map(|a| a.children())
                .map(Item::Node)
                .collect(),
        ),
        _ => None,
    };
    let one_value = |q: &QName| (q.local == "i" || q.local == "v").then_some(Value::Number(99.0));
    let mut ctx = EvaluationContext::new(root);
    ctx.sequence_variables = Some(&bound);
    ctx.variables = Some(&one_value);
    let run = |source: &str| {
        evaluate_versioned(&parse_versioned(source, XPathVersion::V2).unwrap(), &ctx).unwrap()
    };
    // The type survives (a 1.0 value would arrive as xs:double), and the sequence hook wins.
    assert!(matches!(run("$i")[..], [Item::Atomic(Atomic::Integer(5))]));
    assert_eq!(sequence_to_string(&run("$i idiv 2")), "2");
    // The 1.0 hook still serves names the sequence hook does not know.
    assert_eq!(sequence_to_string(&run("$v")), "99");
    assert_eq!(sequence_to_string(&run("count($s)")), "2");
    assert_eq!(sequence_to_string(&run("$s")), "1 two");
    assert_eq!(sequence_to_string(&run("$n")), "x y");
    assert_eq!(sequence_to_string(&run("()")), "");
}
