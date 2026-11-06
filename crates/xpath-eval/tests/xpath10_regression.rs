//! Own regression tests for XPath 1.0 (W3C Recommendation, 16 November 1999).
//!
//! The documents and expressions are written for this crate from the
//! specification; section numbers in the comments name the rule under test.
//! They lock the level reached with the externally fetched Jaxen corpus (see
//! `CONFORMANCE.md`), one section per category of that table, so the level
//! stays checked in CI without that corpus.

mod common;

use common::{Doc, N, parse_xml};
use xpath_eval::{Document, EvaluationContext, QName, Value, evaluate, parse};

// ---------------------------------------------------------------------------
// helpers

#[derive(Clone, Copy)]
enum Var {
    S(&'static str),
    N(f64),
    B(bool),
}

/// Evaluates `expr` with the node selected by `at` (absolute path, first node
/// in document order) as context node.
fn eval_full<'a>(
    doc: &'a Doc,
    at: &str,
    ns: Option<&[(&str, &str)]>,
    vars: &[(&str, Var)],
    expr: &str,
) -> Result<Value<N<'a>>, String> {
    let lookup_ns = |p: &str| {
        ns.and_then(|t| t.iter().find(|(k, _)| *k == p))
            .map(|(_, v)| (*v).to_owned())
    };
    let lookup_var = |q: &QName| {
        if q.prefix.is_some() {
            return None;
        }
        vars.iter()
            .find(|(k, _)| *k == q.local)
            .map(|(_, v)| match v {
                Var::S(s) => Value::String((*s).to_owned()),
                Var::N(n) => Value::Number(*n),
                Var::B(b) => Value::Boolean(*b),
            })
    };
    let context_node = if at == "/" {
        doc.root()
    } else {
        let e = parse(at).map_err(|e| e.to_string())?;
        match evaluate(&e, &EvaluationContext::new(doc.root())).map_err(|e| e.to_string())? {
            Value::NodeSet(n) if !n.is_empty() => n[0],
            _ => panic!("context path {at:?} selects nothing"),
        }
    };
    let parsed = parse(expr).map_err(|e| e.to_string())?;
    let mut ctx = EvaluationContext::new(context_node);
    if ns.is_some() {
        ctx.namespaces = Some(&lookup_ns);
    }
    if !vars.is_empty() {
        ctx.variables = Some(&lookup_var);
    }
    evaluate(&parsed, &ctx).map_err(|e| e.to_string())
}

fn eval<'a>(doc: &'a Doc, at: &str, expr: &str) -> Value<N<'a>> {
    eval_full(doc, at, None, &[], expr).unwrap_or_else(|e| panic!("{expr}: {e}"))
}

fn count_at(doc: &Doc, at: &str, expr: &str) -> usize {
    match eval(doc, at, expr) {
        Value::NodeSet(n) => n.len(),
        _ => panic!("{expr}: not a node-set"),
    }
}

fn count(doc: &Doc, expr: &str) -> usize {
    count_at(doc, "/", expr)
}

fn string_at(doc: &Doc, at: &str, expr: &str) -> String {
    eval(doc, at, expr).to_xpath_string()
}

fn string(doc: &Doc, expr: &str) -> String {
    string_at(doc, "/", expr)
}

fn number(doc: &Doc, expr: &str) -> f64 {
    eval(doc, "/", expr).to_number()
}

fn boolean(doc: &Doc, expr: &str) -> bool {
    eval(doc, "/", expr).to_boolean()
}

/// String-values of the selected nodes, in the order of the result node-set.
fn values_at(doc: &Doc, at: &str, expr: &str) -> Vec<String> {
    use xpath_eval::Node as _;
    match eval(doc, at, expr) {
        Value::NodeSet(n) => n.iter().map(|x| x.string_value()).collect(),
        _ => panic!("{expr}: not a node-set"),
    }
}

/// Parse error or evaluation error.
fn fails(doc: &Doc, expr: &str) -> bool {
    eval_full(doc, "/", None, &[], expr).is_err()
}

const LIB: &str = concat!(
    "<lib>",
    "<shelf n='1'>",
    "<book id='b1' year='1999'><t>Alpha</t><p>10</p></book>",
    "<book id='b2' year='2005'><t>Beta</t><p>20.5</p></book>",
    "</shelf>",
    "<shelf n='2'>",
    "<book id='b3' year='2005'><t>Gamma</t><p>-3</p></book>",
    "<note>x</note>",
    "</shelf>",
    "<!--c1--><?proc data here?>",
    "</lib>"
);

fn lib() -> Doc {
    parse_xml(LIB)
}

// ---------------------------------------------------------------------------
// 01 Axes (section 2.2, 2.5)

#[test]
fn child_and_abbreviated_paths() {
    let d = lib();
    assert_eq!(count(&d, "/lib/shelf"), 2);
    assert_eq!(count(&d, "/lib/shelf/book"), 3);
    assert_eq!(count(&d, "/lib/*"), 2); // `*` matches elements only
    assert_eq!(count(&d, "/lib/node()"), 4); // 2 shelves, comment, PI
    // The context is the root node: `lib` is its child, `shelf` is not.
    assert_eq!(count(&d, "child::lib/child::shelf"), 2);
    assert_eq!(count(&d, "child::lib"), 1);
    assert_eq!(count(&d, "child::shelf"), 0);
    assert_eq!(count(&d, "/"), 1);
    assert_eq!(string(&d, "name(/)"), "");
    // 2.5: `//` is /descendant-or-self::node()/ so [1] binds per parent.
    assert_eq!(count(&d, "//book[1]"), 2);
    assert_eq!(count(&d, "/descendant::book[1]"), 1);
    assert_eq!(count(&d, "//*"), 13);
    assert_eq!(count(&d, "//@*"), 8);
    assert_eq!(count_at(&d, "/lib/shelf[1]", ".//p"), 2);
    assert_eq!(count_at(&d, "/lib/shelf[1]", "./book/t"), 2);
}

#[test]
fn descendant_and_self_axes() {
    let d = lib();
    assert_eq!(count(&d, "/lib/descendant::p"), 3);
    assert_eq!(count(&d, "/descendant::*"), 13);
    assert_eq!(count(&d, "/descendant-or-self::*"), 13);
    // descendant excludes the context node, descendant-or-self includes it.
    assert_eq!(count_at(&d, "/lib/shelf[1]", "descendant::shelf"), 0);
    assert_eq!(
        count_at(&d, "/lib/shelf[1]", "descendant-or-self::shelf"),
        1
    );
    // shelf, 2 x (book, t, "text", p, "text")
    assert_eq!(
        count_at(&d, "/lib/shelf[1]", "descendant-or-self::node()"),
        11
    );
    assert_eq!(count_at(&d, "/lib/shelf[1]", "self::shelf"), 1);
    assert_eq!(count_at(&d, "/lib/shelf[1]", "self::book"), 0);
    assert_eq!(count_at(&d, "/lib/shelf[1]", "self::node()"), 1);
    // The root node is not an element.
    assert_eq!(count(&d, "self::*"), 0);
    assert_eq!(count(&d, "self::node()"), 1);
}

#[test]
fn parent_and_ancestor_axes() {
    let d = lib();
    assert_eq!(count(&d, "//p/parent::book"), 3);
    // A node-set holds each node once.
    assert_eq!(count(&d, "//book/parent::*"), 2);
    assert_eq!(count(&d, "//book/.."), 2);
    assert_eq!(count(&d, "//t/.."), 3);
    // 2.2: the parent of an attribute is its element.
    assert_eq!(count(&d, "//@n/.."), 2);
    assert_eq!(string(&d, "name(/lib/shelf[1]/@n/..)"), "shelf");
    // The document element's parent is the root node; the root has none.
    assert_eq!(count(&d, "/lib/.."), 1);
    assert_eq!(count(&d, "/.."), 0);
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "ancestor::*"), 2);
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "ancestor::node()"), 3); // + root
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "ancestor-or-self::*"), 3);
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "ancestor::note"), 0);
}

#[test]
fn sibling_axes() {
    let d = lib();
    assert_eq!(count_at(&d, "/lib/shelf[1]", "following-sibling::*"), 1);
    assert_eq!(
        count_at(&d, "/lib/shelf[1]", "following-sibling::node()"),
        3
    );
    assert_eq!(
        count_at(&d, "/lib/shelf[1]", "preceding-sibling::node()"),
        0
    );
    assert_eq!(
        count_at(
            &d,
            "/lib/processing-instruction()",
            "preceding-sibling::node()"
        ),
        3
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[1]", "following-sibling::*[1]/@n"),
        "2"
    );
    // Attributes and namespace nodes have no siblings (2.2).
    assert_eq!(
        count_at(&d, "/lib/shelf[1]/@n", "following-sibling::node()"),
        0
    );
    assert_eq!(
        count_at(&d, "/lib/shelf[1]/@n", "preceding-sibling::node()"),
        0
    );
}

#[test]
fn following_and_preceding_axes() {
    let d = lib();
    let t1 = "/lib/shelf[1]/book[1]/t";
    // p, book2, t, p, shelf2, book3, t, p, note
    assert_eq!(count_at(&d, t1, "following::*"), 9);
    // Ancestors and attributes are not on the following/preceding axes.
    assert_eq!(count_at(&d, t1, "following::book"), 2);
    assert_eq!(count_at(&d, t1, "preceding::*"), 0);
    // 2.2: preceding excludes ancestors: shelf1 + 3 x (book, t, p).
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "preceding::*"), 10);
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "preceding::lib"), 0);
    assert_eq!(count_at(&d, "/lib/shelf[2]/note", "following::*"), 0);
    assert_eq!(count(&d, "//preceding::x"), 0);
    assert_eq!(count(&d, "//following::x"), 0);
    // Text nodes count for node(): after book3's <p> only <note> follows.
    assert_eq!(count_at(&d, "/lib/shelf[2]/book/p", "following::node()"), 4);
}

#[test]
fn attribute_axis_and_principal_node_type() {
    let d = lib();
    assert_eq!(count(&d, "/lib/shelf/@n"), 2);
    assert_eq!(count(&d, "//book/attribute::id"), 3);
    assert_eq!(count(&d, "//book/@*"), 6);
    // 2.3: the principal node type of the attribute axis is attribute, of
    // the child axis element - `*` on child never yields an attribute.
    assert_eq!(count(&d, "/lib/shelf[1]/*"), 2);
    assert_eq!(count(&d, "/lib/shelf[1]/@*"), 1);
    assert_eq!(count(&d, "//book/@missing"), 0);
    assert_eq!(string(&d, "/lib/shelf[2]/@n"), "2");
    // An attribute may serve as context node.
    assert_eq!(string_at(&d, "/lib/shelf[2]/@n", "."), "2");
    assert_eq!(count_at(&d, "/lib/shelf[2]/@n", "parent::shelf"), 1);
    // The attribute axis of a non-element is empty.
    assert_eq!(count(&d, "/@*"), 0);
}

#[test]
fn results_are_in_document_order_and_reverse_axes_count_backwards() {
    let d = lib();
    // The node-set is in document order whatever the axis direction...
    assert_eq!(
        values_at(&d, "/lib/shelf[2]/note", "preceding::t"),
        ["Alpha", "Beta", "Gamma"]
    );
    assert_eq!(
        values_at(&d, "/lib/shelf[2]/note", "ancestor::*/@n"),
        ["2"] // only shelf has @n; lib has none
    );
    // ... but positions in a predicate count along the axis (2.4), so on
    // reverse axes [1] is the nearest node.
    assert_eq!(string_at(&d, "/lib/shelf[2]/note", "preceding::*[1]"), "-3");
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "preceding::*[2]"),
        "Gamma"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "name(preceding::*[1])"),
        "p"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "name(ancestor::*[1])"),
        "shelf"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "name(ancestor::*[last()])"),
        "lib"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "name(ancestor::*[position()=2])"),
        "lib"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[2]/note", "name(preceding-sibling::*[1])"),
        "book"
    );
    assert_eq!(string(&d, "//p[.=20.5]/preceding-sibling::*[1]"), "Beta");
    // Forward axes count forwards.
    assert_eq!(
        string_at(&d, "/lib/shelf[1]/book[1]/t", "name(following::*[1])"),
        "p"
    );
    assert_eq!(
        string_at(&d, "/lib/shelf[1]/book[1]/t", "following::*[last()]"),
        "x"
    );
    // Chained steps: [1] applies per step result, last step of a path.
    assert_eq!(
        string(&d, "//shelf[2]/note/preceding::*[1][name()='p']"),
        "-3"
    );
    assert_eq!(count(&d, "//shelf[2]/note/preceding::*[1][name()='t']"), 0);
}

// ---------------------------------------------------------------------------
// 02 Node tests: comment / PI / text (section 2.3)

#[test]
fn comment_pi_and_text_node_tests() {
    let d = lib();
    assert_eq!(count(&d, "/lib/comment()"), 1);
    assert_eq!(count(&d, "//comment()"), 1);
    assert_eq!(string(&d, "/lib/comment()"), "c1");
    assert_eq!(count(&d, "/lib/processing-instruction()"), 1);
    assert_eq!(count(&d, "/lib/processing-instruction('proc')"), 1);
    assert_eq!(count(&d, "/lib/processing-instruction('other')"), 0);
    assert_eq!(count(&d, "/lib/processing-instruction(\"proc\")"), 1);
    // The string-value of a PI is its data without the target (5.5).
    assert_eq!(string(&d, "/lib/processing-instruction()"), "data here");
    // 5.5: the name of a PI is its target.
    assert_eq!(string(&d, "name(/lib/processing-instruction())"), "proc");
    assert_eq!(
        string(&d, "local-name(/lib/processing-instruction())"),
        "proc"
    );
    assert_eq!(string(&d, "name(/lib/comment())"), "");
    assert_eq!(count(&d, "//text()"), 7); // 3 t, 3 p, 1 note
    assert_eq!(count(&d, "//t/text()"), 3);
    assert_eq!(string(&d, "//book[@id='b2']/t/text()"), "Beta");
    // `*` does not match comments, PIs or text.
    assert_eq!(count(&d, "/lib/*[self::comment()]"), 0);
}

#[test]
fn top_level_comment_and_pi_belong_to_the_root_node() {
    let d = parse_xml("<!--top--><?pi x?><r/><!--after-->");
    assert_eq!(count(&d, "/comment()"), 2);
    assert_eq!(count(&d, "/processing-instruction()"), 1);
    assert_eq!(count(&d, "/processing-instruction('pi')"), 1);
    assert_eq!(count(&d, "/processing-instruction('zz')"), 0);
    assert_eq!(count(&d, "/*"), 1);
    assert_eq!(count(&d, "/node()"), 4);
    assert_eq!(count(&d, "//comment()"), 2);
    // Order: comment, PI, element, comment.
    assert_eq!(values_at(&d, "/", "/comment()"), ["top", "after"]);
    assert_eq!(string(&d, "/"), ""); // root string-value: text descendants only
}

#[test]
fn string_value_of_nodes() {
    // 5.2 element: concatenation of text descendants, comments and PIs
    // excluded; 5.3 attribute: its value; 5.7 text: its characters.
    let d = parse_xml("<m>a<b>b</b>c<!--x-->d<?p y?></m>");
    assert_eq!(string(&d, "/m"), "abcd");
    assert_eq!(string(&d, "/"), "abcd");
    assert_eq!(count(&d, "/m/text()"), 3);
    assert_eq!(string(&d, "/m/b"), "b");
    let e = parse_xml("<e a='v&amp;w'>x&amp;y<![CDATA[<z>]]></e>");
    assert_eq!(string(&e, "/e"), "x&y<z>");
    assert_eq!(string(&e, "/e/@a"), "v&w");
    assert_eq!(count(&e, "/e/text()"), 1);
    // Whitespace-only text nodes are nodes and count.
    let w = parse_xml("<w> <a/> </w>");
    assert_eq!(count(&w, "/w/text()"), 2);
    assert_eq!(string(&w, "string-length(/w)"), "2");
    assert_eq!(string(&lib(), "/lib/shelf[1]"), "Alpha10Beta20.5");
    assert_eq!(string(&lib(), "/lib"), "Alpha10Beta20.5Gamma-3x");
    // string() of an empty node-set is "", of a node-set the first node in
    // document order (4.2).
    assert_eq!(string(&lib(), "string(/lib/nothing)"), "");
    assert_eq!(string(&lib(), "string(//t)"), "Alpha");
    assert_eq!(
        string_at(&lib(), "/lib/shelf[2]/book/t", "string()"),
        "Gamma"
    );
}

// ---------------------------------------------------------------------------
// 03 Predicates and positions (section 2.4, 3.3)

#[test]
fn predicates_filter_with_boolean_or_position() {
    let d = lib();
    assert_eq!(count(&d, "//book[@year]"), 3);
    assert_eq!(count(&d, "//book[@year=2005]"), 2);
    assert_eq!(count(&d, "//book[@year='2005']"), 2);
    assert_eq!(count(&d, "//book[@year>2000][t='Beta']"), 1);
    assert_eq!(count(&d, "//book[not(note)]"), 3);
    assert_eq!(count(&d, "//shelf[note]"), 1);
    assert_eq!(count(&d, "//book[t and p>5]"), 2);
    assert_eq!(count(&d, "//book[@id='b1' or @id='b3']"), 2);
    assert_eq!(count(&d, "//book[p > 5]"), 2);
    assert_eq!(count(&d, "//t[.='Beta']"), 1);
    assert_eq!(count(&d, "//book[@year=//book[@id='b1']/@year]"), 1);
    // Whitespace around tokens is insignificant (3.7).
    assert_eq!(count(&d, " / lib / shelf [ 1 ] / book "), 2);
    assert_eq!(count(&d, "/lib/ child :: shelf"), 2);
    // A numeric predicate is a position test: [n] means [position()=n],
    // an expression yielding a number is converted likewise.
    assert_eq!(count(&d, "//shelf[1+1]"), 1);
    assert_eq!(string(&d, "//shelf[1+1]/@n"), "2");
    // A non-integral number never equals a position.
    assert_eq!(count(&d, "//shelf[1.5]"), 0);
    assert_eq!(count(&d, "//shelf[0]"), 0);
    // A string predicate is a boolean test (non-empty = true), not a position.
    assert_eq!(count(&d, "//shelf['0']"), 2);
    assert_eq!(count(&d, "//shelf['']"), 0);
}

#[test]
fn positional_predicates() {
    let d = lib();
    // position() is relative to the step's input per parent node.
    assert_eq!(count(&d, "//p[1]"), 3);
    assert_eq!(count(&d, "(//p)[1]"), 1);
    assert_eq!(string(&d, "(//p)[last()]"), "-3");
    assert_eq!(count(&d, "//book[position()>1]"), 1);
    assert_eq!(string(&d, "//book[position()>1]/@id"), "b2");
    assert_eq!(count(&d, "//book[last()]"), 2); // b2 and b3
    assert_eq!(string(&d, "//book[last()-1]/@id"), "b1");
    assert_eq!(string(&d, "/lib/shelf[last()]/@n"), "2");
    assert_eq!(count(&d, "/lib/shelf[position()=last()]"), 1);
    // Stacked predicates renumber: the second sees only survivors of the first.
    assert_eq!(count(&d, "//book[@year=2005][1]"), 2);
    assert_eq!(count(&d, "/lib/shelf/book[@year=2005][2]"), 0);
    assert_eq!(string(&d, "(/lib/shelf/book[@year=2005])[2]/@id"), "b3");
    // Filter expression: position() over the whole set in document order.
    assert_eq!(string(&d, "(//book)[2]/t"), "Beta");
    assert_eq!(string(&d, "(//book)[last()]/@id"), "b3");
    // Outside any predicate the context has position 1 and size 1.
    assert_eq!(number(&d, "position()"), 1.0);
    assert_eq!(number(&d, "last()"), 1.0);
    // 2.5: //x[1] differs from /descendant::x[1].
    assert_eq!(count(&d, "//t[1]"), 3);
    assert_eq!(count(&d, "/descendant::t[1]"), 1);
    // Numeric comparison inside a position test.
    assert_eq!(count(&d, "//book[position() < 10]"), 3);
}

// ---------------------------------------------------------------------------
// 04 Node-set functions (section 4.1)

#[test]
fn node_set_functions() {
    let d = lib();
    assert_eq!(number(&d, "count(//book)"), 3.0);
    assert_eq!(number(&d, "count(//nothing)"), 0.0);
    assert_eq!(number(&d, "count(//book | //shelf)"), 5.0);
    assert_eq!(
        number(&d, "count(/lib/shelf[2]/descendant-or-self::*)"),
        5.0
    );
    assert_eq!(number(&d, "count(/lib/shelf/book[1])") + 1.0, 3.0);
    assert_eq!(number(&d, "count(//book)*2+1"), 7.0);
    assert_eq!(string(&d, "local-name(/lib/shelf)"), "shelf");
    assert_eq!(string(&d, "name(//t)"), "t");
    assert_eq!(string(&d, "name()"), ""); // root node has no name
    assert_eq!(string(&d, "name(.)"), "");
    assert_eq!(string(&d, "name(..)"), "");
    assert_eq!(string(&d, "name(/*)"), "lib");
    assert_eq!(string(&d, "name(/lib/shelf/@n)"), "n");
    assert_eq!(string_at(&d, "/lib/shelf[1]", "name()"), "shelf");
    assert_eq!(string_at(&d, "/lib/shelf[1]", "name(.)"), "shelf");
    assert_eq!(string_at(&d, "/lib/shelf[1]", "name(..)"), "lib");
    // The first node in document order is used; an empty set gives "".
    assert_eq!(string(&d, "name(//book/*)"), "t");
    assert_eq!(string(&d, "name(//nothing)"), "");
    assert_eq!(string(&d, "namespace-uri(/lib)"), "");
    assert_eq!(string(&d, "(//*[local-name()='t'])[2]"), "Beta");
    // last() and position() inside a function argument predicate.
    assert_eq!(number(&d, "count(//book[position()=last()])"), 2.0);
}

#[test]
fn id_function_without_id_information() {
    // 4.1/5.2: `id()` finds attributes of type ID. This host model does not
    // declare ID attributes (the Node default), so id() is an empty
    // node-set - never an error, even when attributes are *named* "id".
    let d = lib();
    assert_eq!(count(&d, "id('b1')"), 0);
    assert_eq!(count(&d, "id('b1 b2')"), 0);
    assert_eq!(count(&d, "id(//book/@id)"), 0);
    assert_eq!(count(&d, "id('b1')/t"), 0);
    assert!(fails(&d, "id()"));
    assert!(fails(&d, "id('a', 'b')"));
}

// ---------------------------------------------------------------------------
// 05 String functions (section 4.2)

#[test]
fn string_function_basics() {
    let d = lib();
    assert_eq!(string(&d, "concat('a', 'b', 'c')"), "abc");
    assert_eq!(string(&d, "concat('a', 1, true())"), "a1true");
    assert_eq!(string(&d, "concat('x', //t)"), "xAlpha");
    assert_eq!(
        string_at(&d, "/lib/shelf[1]/book[1]", "concat(t, '-', p)"),
        "Alpha-10"
    );
    assert!(fails(&d, "concat('a')"));
    assert!(boolean(&d, "starts-with('abcdef', 'abc')"));
    assert!(!boolean(&d, "starts-with('abcdef', 'bc')"));
    assert!(boolean(&d, "starts-with('abc', '')"));
    assert!(boolean(&d, "contains('abcdef', 'cd')"));
    assert!(!boolean(&d, "contains('abcdef', 'dc')"));
    assert!(boolean(&d, "contains('abc', '')"));
    assert!(!boolean(&d, "contains('', 'a')"));
    assert_eq!(string(&d, "substring-before('1999/04/01', '/')"), "1999");
    assert_eq!(string(&d, "substring-after('1999/04/01', '/')"), "04/01");
    assert_eq!(string(&d, "substring-before('abc', 'z')"), "");
    assert_eq!(string(&d, "substring-after('abc', 'z')"), "");
    assert_eq!(string(&d, "substring-after('abc', '')"), "abc");
    assert_eq!(string(&d, "substring-before('abc', '')"), "");
    assert_eq!(string(&d, "substring-after('a-b-c', '-')"), "b-c");
    assert_eq!(number(&d, "string-length('')"), 0.0);
    assert_eq!(number(&d, "string-length('abc')"), 3.0);
    // Length counts characters, not bytes or UTF-16 units.
    assert_eq!(number(&d, "string-length('\u{e9}')"), 1.0);
    assert_eq!(number(&d, "string-length('a\u{1F600}')"), 2.0);
    assert_eq!(
        string_at(&d, "/lib/shelf[1]/book[1]/t", "string-length()"),
        "5"
    );
    assert_eq!(string(&d, "string-length(//t)"), "5");
}

#[test]
fn substring_rounds_and_handles_nan_and_infinity() {
    // 4.2: characters whose position p satisfies
    // round(start) <= p < round(start) + round(length).
    let d = lib();
    assert_eq!(string(&d, "substring('abcdef', 2, 3)"), "bcd");
    assert_eq!(string(&d, "substring('abcdef', 2.5)"), "cdef"); // round(2.5)=3
    assert_eq!(string(&d, "substring('abcdef', 1.4, 2.5)"), "abc"); // 1 .. 1+3
    assert_eq!(string(&d, "substring('abcdef', 1.5, 2.5)"), "bcd"); // 2 .. 2+3
    assert_eq!(string(&d, "substring('abcdef', -1, 3)"), "a"); // -1 <= p < 2
    assert_eq!(string(&d, "substring('abcdef', 0)"), "abcdef");
    assert_eq!(string(&d, "substring('abcdef', 7)"), "");
    assert_eq!(string(&d, "substring('abcdef', 5, 100)"), "ef");
    assert_eq!(string(&d, "substring('abcdef', 2, 0)"), "");
    assert_eq!(string(&d, "substring('abcdef', 2, -1)"), "");
    assert_eq!(string(&d, "substring('abcdef', 0 div 0)"), "");
    assert_eq!(string(&d, "substring('abcdef', 2, 0 div 0)"), "");
    assert_eq!(string(&d, "substring('abcdef', -1 div 0)"), "abcdef");
    assert_eq!(string(&d, "substring('abcdef', 3, 1 div 0)"), "cdef");
    assert_eq!(string(&d, "substring('abcdef', -1 div 0, 1 div 0)"), ""); // -inf + inf = NaN
    assert_eq!(string(&d, "substring('abcdef', -5, 1 div 0)"), "abcdef");
    assert!(fails(&d, "substring('abc')"));
}

#[test]
fn normalize_space_and_translate() {
    let d = lib();
    assert_eq!(string(&d, "normalize-space('  a \t\n b  ')"), "a b");
    assert_eq!(string(&d, "normalize-space('')"), "");
    assert_eq!(string(&d, "normalize-space('   ')"), "");
    assert_eq!(string(&d, "normalize-space('a  b')"), "a b");
    assert_eq!(
        string_at(&d, "/lib/shelf[1]/book[1]/t", "normalize-space()"),
        "Alpha"
    );
    // Not in the XPath whitespace set (#x20 #x9 #xD #xA): NBSP stays.
    assert_eq!(
        string(&d, "normalize-space('a\u{a0}\u{a0}b')"),
        "a\u{a0}\u{a0}b"
    );
    // 4.2 translate: mapped by position; unmatched `from` chars deleted;
    // the first occurrence in `from` wins; extra `to` chars ignored.
    assert_eq!(string(&d, "translate('bar', 'abc', 'ABC')"), "BAr");
    assert_eq!(string(&d, "translate('--aaa--', 'abc-', 'ABC')"), "AAA");
    assert_eq!(string(&d, "translate('aaa', 'aa', 'xy')"), "xxx");
    assert_eq!(string(&d, "translate('abc', 'b', 'XYZ')"), "aXc");
    assert_eq!(string(&d, "translate('abc', '', 'XYZ')"), "abc");
    assert_eq!(string(&d, "translate('abc', 'abc', '')"), "");
    assert_eq!(string(&d, "translate('', 'a', 'b')"), "");
}

#[test]
fn string_conversion_of_numbers_and_booleans() {
    // 4.2: no exponent, no leading '+', "-0" prints as "0", NaN/Infinity words.
    let d = lib();
    assert_eq!(string(&d, "string(1 div 0)"), "Infinity");
    assert_eq!(string(&d, "string(-1 div 0)"), "-Infinity");
    assert_eq!(string(&d, "string(0 div 0)"), "NaN");
    assert_eq!(string(&d, "string(2.0)"), "2");
    assert_eq!(string(&d, "string(2.50)"), "2.5");
    assert_eq!(string(&d, "string(-0.5)"), "-0.5");
    assert_eq!(string(&d, "string(10 div 4)"), "2.5");
    assert_eq!(string(&d, "string(100000000000)"), "100000000000");
    assert_eq!(string(&d, "string(0.00001)"), "0.00001");
    assert_eq!(string(&d, "string(-0)"), "0");
    assert_eq!(string(&d, "string(true())"), "true");
    assert_eq!(string(&d, "string(false())"), "false");
    assert_eq!(string(&d, "string(1 = 1)"), "true");
    assert!(fails(&d, "string(1, 2)"));
}

// ---------------------------------------------------------------------------
// 06 Boolean functions (section 4.3)

#[test]
fn boolean_conversion_and_functions() {
    let d = lib();
    assert!(boolean(&d, "boolean(//t)"));
    assert!(!boolean(&d, "boolean(//nothing)"));
    assert!(boolean(&d, "boolean('false')")); // non-empty string
    assert!(!boolean(&d, "boolean('')"));
    assert!(boolean(&d, "boolean(' ')"));
    assert!(!boolean(&d, "boolean(0)"));
    assert!(boolean(&d, "boolean(-1)"));
    assert!(!boolean(&d, "boolean(0 div 0)")); // NaN is false
    assert!(boolean(&d, "boolean(1 div 0)"));
    assert!(boolean(&d, "true()"));
    assert!(!boolean(&d, "false()"));
    assert!(boolean(&d, "not(false())"));
    assert!(!boolean(&d, "not(//t)"));
    assert!(boolean(&d, "not(//nothing)"));
    assert!(boolean(&d, "not('')"));
    assert!(boolean(&d, "true() and true()"));
    assert!(!boolean(&d, "true() and false()"));
    assert!(boolean(&d, "false() or true()"));
    assert!(!boolean(&d, "false() or false()"));
    // and binds tighter than or (3.4 grammar).
    assert!(boolean(&d, "true() or false() and false()"));
    assert!(!boolean(&d, "(true() or false()) and false()"));
    assert!(fails(&d, "not()"));
    assert!(fails(&d, "true(1)"));
    assert!(fails(&d, "boolean()"));
}

#[test]
fn lang_function() {
    // 4.3: nearest xml:lang ancestor-or-self; case-insensitive; a language
    // matches itself or itself followed by "-" and a subtag.
    let d =
        parse_xml("<d xml:lang='en-US'><e/><f xml:lang='de'><g/></f><h xml:lang='EN'><i/></h></d>");
    let l = |at: &str, arg: &str| eval(&d, at, &format!("lang('{arg}')")).to_boolean();
    assert!(l("/d/e", "en"));
    assert!(l("/d/e", "en-US"));
    assert!(l("/d/e", "en-us"));
    assert!(l("/d/e", "EN"));
    assert!(!l("/d/e", "en-GB"));
    assert!(!l("/d/e", "e"));
    assert!(!l("/d/e", "en-"));
    assert!(l("/d/f/g", "de"));
    assert!(!l("/d/f/g", "en")); // nearest xml:lang wins
    assert!(l("/d/h/i", "en"));
    assert!(l("/d/h", "en"));
    assert!(!l("/d/h", "en-US"));
    // No xml:lang anywhere: false for every argument.
    let n = parse_xml("<d><e/></d>");
    assert!(!eval(&n, "/d/e", "lang('en')").to_boolean());
    assert!(!eval(&n, "/d/e", "lang('')").to_boolean());
    // As a predicate.
    assert_eq!(count(&d, "//*[lang('de')]"), 2);
    assert_eq!(count(&d, "//*[lang('en')]"), 4); // d, e, h, i
}

// ---------------------------------------------------------------------------
// 07 Number functions (section 4.4) and arithmetic (section 3.5)

#[test]
fn number_functions() {
    let d = lib();
    assert_eq!(number(&d, "sum(//p)"), 27.5);
    assert_eq!(number(&d, "sum(//nothing)"), 0.0);
    assert_eq!(number(&d, "sum(//book/@year)"), 1999.0 + 2005.0 + 2005.0);
    assert!(number(&d, "sum(//t)").is_nan()); // non-numeric strings
    assert!(fails(&d, "sum(1)"));
    assert_eq!(number(&d, "floor(-1.5)"), -2.0);
    assert_eq!(number(&d, "floor(1.5)"), 1.0);
    assert_eq!(number(&d, "ceiling(-1.5)"), -1.0);
    assert_eq!(number(&d, "ceiling(1.2)"), 2.0);
    assert_eq!(number(&d, "round(2.5)"), 3.0); // ties toward +infinity
    assert_eq!(number(&d, "round(-2.5)"), -2.0);
    assert_eq!(number(&d, "round(2.4)"), 2.0);
    assert_eq!(number(&d, "round(-2.6)"), -3.0);
    assert_eq!(string(&d, "string(round(-0.4))"), "0"); // negative zero prints "0"
    assert_eq!(number(&d, "round(1 div 0)"), f64::INFINITY);
    assert!(number(&d, "round(0 div 0)").is_nan());
    assert_eq!(number(&d, "floor(//book[2]/p)"), 20.0);
    assert_eq!(number(&d, "round(//book[2]/p)"), 21.0);
    // number(): 3.5 grammar - optional '-', digits with optional fraction,
    // surrounding whitespace; no '+', no exponent.
    assert_eq!(number(&d, "number('12')"), 12.0);
    assert_eq!(number(&d, "number(' 12 ')"), 12.0);
    assert_eq!(number(&d, "number('-12.5')"), -12.5);
    assert_eq!(number(&d, "number('.5')"), 0.5);
    assert_eq!(number(&d, "number('5.')"), 5.0);
    assert!(number(&d, "number('+5')").is_nan());
    assert!(number(&d, "number('1e3')").is_nan());
    assert!(number(&d, "number('abc')").is_nan());
    assert!(number(&d, "number('')").is_nan());
    assert!(number(&d, "number('1 2')").is_nan());
    assert_eq!(number(&d, "number(true())"), 1.0);
    assert_eq!(number(&d, "number(false())"), 0.0);
    assert_eq!(number(&d, "number(//book[2]/p)"), 20.5);
    assert!(number(&d, "number(//nothing)").is_nan());
    assert_eq!(number(&d, "number(//book[1]/@year)"), 1999.0);
    assert_eq!(number(&d, "number(1 = 1) + 1"), 2.0);
    assert_eq!(string_at(&d, "/lib/shelf[2]/book/p", "number()"), "-3");
}

#[test]
fn arithmetic() {
    let d = lib();
    assert_eq!(number(&d, "2 + 3 * 4"), 14.0); // * before +
    assert_eq!(number(&d, "(2 + 3) * 4"), 20.0);
    // Left associativity (3.5).
    assert_eq!(number(&d, "10 - 4 - 3"), 3.0);
    assert_eq!(number(&d, "24 div 4 div 3"), 2.0);
    assert_eq!(number(&d, "20 mod 7 mod 4"), 2.0);
    assert_eq!(number(&d, "10 div 4"), 2.5);
    // mod: sign follows the dividend (as Java/ECMAScript %).
    assert_eq!(number(&d, "5 mod 2"), 1.0);
    assert_eq!(number(&d, "5 mod -2"), 1.0);
    assert_eq!(number(&d, "-5 mod 2"), -1.0);
    assert_eq!(number(&d, "-5 mod -2"), -1.0);
    assert_eq!(number(&d, "5.5 mod 2"), 1.5);
    assert!(number(&d, "5 mod 0").is_nan());
    assert_eq!(number(&d, "1 div 0"), f64::INFINITY);
    assert_eq!(number(&d, "-1 div 0"), f64::NEG_INFINITY);
    assert!(number(&d, "0 div 0").is_nan());
    assert!(number(&d, "(1 div 0) - (1 div 0)").is_nan());
    // Unary minus, repeated and mixed with binary minus.
    assert_eq!(number(&d, "- -1"), 1.0);
    assert_eq!(number(&d, "2 - -1"), 3.0);
    assert_eq!(number(&d, "-(2 + 3)"), -5.0);
    assert_eq!(number(&d, ".5 + .5"), 1.0);
    // Operands are converted with number().
    assert_eq!(number(&d, "'3' + '4'"), 7.0);
    assert_eq!(number(&d, "true() + true()"), 2.0);
    assert_eq!(number(&d, "//book[1]/p + //book[2]/p"), 30.5);
    assert_eq!(number(&d, "count(//book) * count(//shelf)"), 6.0);
    assert!(number(&d, "//nothing + 1").is_nan());
    assert_eq!(
        number(&d, "5 + count(/lib/shelf[2]/note/ancestor::*) - 1"),
        6.0
    );
}

#[test]
fn operator_names_as_element_names() {
    // 3.7 lexical structure: `div`, `mod`, `and`, `or` and `*` are operators
    // only when a preceding token makes an operator possible; elsewhere they
    // are names.
    let d = parse_xml(
        "<r><div>1</div><mod>2</mod><and>3</and><or>4</or><a-b>5</a-b><a>7</a><b>2</b></r>",
    );
    assert_eq!(count(&d, "/r/div"), 1);
    assert_eq!(number(&d, "/r/div div /r/mod"), 0.5);
    assert_eq!(number(&d, "/r/mod mod 2"), 0.0);
    assert!(boolean(&d, "/r/and and /r/or"));
    assert!(boolean(&d, "/r/or or false()"));
    assert_eq!(number(&d, "count(/r/*) * 2"), 14.0);
    assert_eq!(number(&d, "/r/div * 2"), 2.0);
    assert_eq!(count(&d, "/r/*"), 7);
    // `-` is part of a name when adjacent; whitespace makes it subtraction.
    assert_eq!(string(&d, "/r/a-b"), "5");
    assert_eq!(number(&d, "/r/a - /r/b"), 5.0);
    assert_eq!(number(&d, "number(/r/a)-number(/r/b)"), 5.0);
}

// ---------------------------------------------------------------------------
// 08 Comparison operators (section 3.4)

fn nums() -> Doc {
    parse_xml("<n><v>1</v><v>2</v><v>3</v><w>3</w><w>4</w></n>")
}

#[test]
fn node_set_comparison_is_existential() {
    let d = nums();
    // Both = and != can be true for the same node-set: "some node ...".
    assert!(boolean(&d, "/n/v = 2"));
    assert!(boolean(&d, "/n/v != 2"));
    assert!(!boolean(&d, "/n/v = 9"));
    assert!(boolean(&d, "/n/v != 9"));
    // != is not the negation of =.
    assert!(boolean(&d, "not(/n/v = 9) and /n/v != 9"));
    assert!(boolean(&d, "/n/v = /n/w")); // 3 = 3
    assert!(boolean(&d, "/n/v != /n/w"));
    assert!(boolean(&d, "/n/v < /n/w")); // 1 < 3
    assert!(!boolean(&d, "/n/w < /n/v")); // min(w)=3 !< max(v)=3
    assert!(boolean(&d, "/n/w <= /n/v"));
    assert!(boolean(&d, "/n/v > 2")); // 3 > 2
    assert!(!boolean(&d, "/n/v > 3"));
    assert!(boolean(&d, "/n/v >= 3"));
    assert!(!boolean(&d, "/n/v < 1"));
    assert!(boolean(&d, "/n/v <= 1"));
    assert!(boolean(&d, "2 < /n/v")); // operand order irrelevant
    assert!(boolean(&d, "9 > /n/v"));
    assert!(!boolean(&d, "0 > /n/v"));
    // A single-valued set behaves like its value.
    assert!(boolean(&d, "/n/w[1] = 3"));
    assert!(!boolean(&d, "/n/w[1] != 3"));
    // Via a predicate (the typical use).
    assert_eq!(count(&d, "/n/*[. = /n/w]"), 3); // v=3, w=3, w=4
}

#[test]
fn empty_node_set_comparison() {
    let d = nums();
    // No node, so no node can satisfy any comparison with a number/string...
    assert!(!boolean(&d, "/n/zz = 1"));
    assert!(!boolean(&d, "/n/zz != 1"));
    assert!(!boolean(&d, "/n/zz < 1"));
    assert!(!boolean(&d, "/n/zz = ''"));
    assert!(!boolean(&d, "/n/zz = /n/zz"));
    assert!(!boolean(&d, "/n/zz != /n/v"));
    assert!(boolean(&d, "not(/n/zz = 1)"));
    // ... but against a boolean the set is converted with boolean().
    assert!(boolean(&d, "/n/zz = false()"));
    assert!(!boolean(&d, "/n/zz = true()"));
    assert!(boolean(&d, "/n/zz != true()"));
    assert!(boolean(&d, "/n/v = true()"));
    assert!(!boolean(&d, "/n/v = false()"));
    assert!(boolean(&d, "/n/v != false()"));
}

#[test]
fn comparison_type_conversions() {
    let d = nums();
    // node-set vs string: string comparison; vs number: number comparison.
    assert!(boolean(&d, "/n/v = '2'"));
    assert!(!boolean(&d, "/n/v = '2.0'")); // "2" vs "2.0"
    assert!(boolean(&d, "/n/v = 2.0")); // 2 vs 2
    assert!(boolean(&d, "/n/v = '3'"));
    // booleans: converted with boolean() when either side is boolean.
    assert!(boolean(&d, "true() = 'x'"));
    assert!(boolean(&d, "true() = 1"));
    assert!(boolean(&d, "false() = 0"));
    assert!(boolean(&d, "false() = ''"));
    assert!(boolean(&d, "true() = 5"));
    assert!(!boolean(&d, "false() = 'false'")); // 'false' is non-empty = true
    // number vs string: string converted to number.
    assert!(boolean(&d, "1 = '1.0'"));
    assert!(!boolean(&d, "'1' = '1.0'")); // both strings: string comparison
    assert!(boolean(&d, "'a' = 'a'"));
    assert!(boolean(&d, "'a' != 'b'"));
    assert!(!boolean(&d, "'a' = 'A'"));
    assert!(boolean(&d, "'abc' = \"abc\""));
    // Relational operators always compare numbers.
    assert!(!boolean(&d, "'10' < '9'"));
    assert!(boolean(&d, "'9' < '10'"));
    assert!(!boolean(&d, "'a' < 'b'")); // NaN
    assert!(!boolean(&d, "'a' >= 'a'"));
    assert!(boolean(&d, "true() > false()")); // 1 > 0
    assert!(boolean(&d, "'2' > 1"));
    // Equality is lower precedence than relational, and left associative.
    assert!(boolean(&d, "1 < 2 = true()"));
    assert!(boolean(&d, "1 = 1 = 1")); // (1=1)=1 -> true()=1
    assert!(boolean(&d, "2 = 2 = 2")); // (2=2)=2 -> true() = true()
    assert!(!boolean(&d, "0 = 0 = 0")); // (0=0)=0 -> true() = false()
}

#[test]
fn nan_comparisons() {
    let d = nums();
    // 3.4: every comparison involving NaN is false except !=.
    assert!(!boolean(&d, "(0 div 0) = (0 div 0)"));
    assert!(boolean(&d, "(0 div 0) != (0 div 0)"));
    assert!(!boolean(&d, "(0 div 0) < 1"));
    assert!(!boolean(&d, "(0 div 0) > 1"));
    assert!(!boolean(&d, "(0 div 0) <= (0 div 0)"));
    assert!(!boolean(&d, "(0 div 0) >= (0 div 0)"));
    assert!(boolean(&d, "not((0 div 0) = (0 div 0))"));
    assert!(!boolean(&d, "'x' = number('x')"));
    assert!(fails(&d, "1e0")); // no exponent syntax in XPath 1.0
    assert!(boolean(&d, "(1 div 0) = (2 div 0)"));
    assert!(boolean(&d, "(-1 div 0) < (1 div 0)"));
}

// ---------------------------------------------------------------------------
// 09 Unions (section 3.3)

#[test]
fn union_expressions() {
    let d = lib();
    assert_eq!(count(&d, "/lib/shelf | /lib/shelf/book"), 5);
    assert_eq!(count(&d, "//book | //book"), 3); // duplicates removed
    assert_eq!(count(&d, "//t | //t[1]"), 3);
    // The result is in document order, not operand order.
    assert_eq!(string(&d, "(//p | //t)[2]"), "10");
    assert_eq!(
        values_at(&d, "/", "//p | //t"),
        ["Alpha", "10", "Beta", "20.5", "Gamma", "-3"]
    );
    assert_eq!(count(&d, "(/lib/shelf)[1] | /lib/shelf[2]"), 2);
    assert_eq!(count(&d, "//@n | //@id"), 5);
    assert_eq!(count(&d, "//nothing | //other"), 0);
    assert_eq!(count(&d, "/lib/shelf[1]/book | /lib/shelf[2]/note"), 3);
    assert_eq!(count(&d, "//t | //p | //note"), 7);
    // Both operands must be node-sets.
    assert!(fails(&d, "1 | 2"));
    assert!(fails(&d, "//t | 'a'"));
    assert!(fails(&d, "true() | //t"));
}

// ---------------------------------------------------------------------------
// 10 Variables (section 3.1)

#[test]
fn variable_references() {
    let d = lib();
    let vars = [
        ("y", Var::S("2005")),
        ("lim", Var::N(5.0)),
        ("yes", Var::B(true)),
    ];
    let run = |expr: &str| eval_full(&d, "/", None, &vars, expr).unwrap();
    let Value::NodeSet(n) = run("//book[@year=$y]") else {
        panic!()
    };
    assert_eq!(n.len(), 2);
    let Value::NodeSet(n) = run("//book[p > $lim]") else {
        panic!()
    };
    assert_eq!(n.len(), 2);
    assert_eq!(run("$y").to_xpath_string(), "2005");
    assert_eq!(run("$lim + 1").to_number(), 6.0);
    assert!(run("$yes and $lim = 5").to_boolean());
    // The variable name is not interpolated into a literal.
    let Value::NodeSet(n) = run("//book[@year='$y']") else {
        panic!()
    };
    assert!(n.is_empty());
    // `concat($y, 'x')` evaluated as an expression in a predicate.
    let Value::NodeSet(n) = run("//book[concat(@year, 'x') = concat($y, 'x')]") else {
        panic!()
    };
    assert_eq!(n.len(), 2);
    // A reference without a binding is an error, not an empty value.
    assert!(eval_full(&d, "/", None, &vars, "$missing").is_err());
    assert!(eval_full(&d, "/", None, &vars, "//book[@year=$missing]").is_err());
    assert!(eval_full(&d, "/", None, &[], "$y").is_err());
}

// ---------------------------------------------------------------------------
// 11 Namespaces (section 2.3, 5.4)

const NSDOC: &str =
    "<p:a xmlns:p='urn:x' xmlns='urn:d'><b/><p:c/><q:d xmlns:q='urn:x'/><e xmlns=''/></p:a>";

#[test]
fn name_tests_with_namespace_bindings() {
    let d = parse_xml(NSDOC);
    let n = |expr: &str, ns: &[(&str, &str)]| match eval_full(&d, "/", Some(ns), &[], expr).unwrap()
    {
        Value::NodeSet(v) => v.len(),
        _ => panic!(),
    };
    let b = [("p", "urn:x")];
    assert_eq!(n("/p:a", &b), 1);
    assert_eq!(n("/p:a/p:c", &b), 1);
    // The prefix in the document and in the expression are unrelated; the
    // URI decides (2.3): `q:d` in the document is `p:d` here.
    assert_eq!(n("/p:a/p:d", &b), 1);
    assert_eq!(n("/p:a/p:*", &b), 2); // c and d
    assert_eq!(n("/p:a/*", &b), 4); // `*` matches any namespace
    // An unprefixed name test matches only no-namespace elements, so the
    // default namespace `urn:d` of <b/> is not matched by `b`.
    assert_eq!(n("/p:a/b", &b), 0);
    assert_eq!(n("/p:a/e", &b), 1); // xmlns='' puts <e> in no namespace
    assert_eq!(n("/p:a/e", &[]), 0); // (the root is p:a in urn:x, unbound)
    let other = [("z", "urn:x"), ("dd", "urn:d")];
    assert_eq!(n("/z:a/z:c", &other), 1);
    assert_eq!(n("/z:a/dd:b", &other), 1);
    assert_eq!(n("/z:a/dd:c", &other), 0);
    // Same prefix bound to another URI selects nothing.
    assert_eq!(n("/p:a", &[("p", "urn:other")]), 0);
    // Several prefixes for one URI are equivalent.
    assert_eq!(n("/p:a/q:d", &[("p", "urn:x"), ("q", "urn:x")]), 1);
}

#[test]
fn unbound_prefix_matches_nothing_deliberately() {
    // XPath 1.0 (2.3) makes an unbound prefix an error. xpath-eval instead
    // lets the name test match nothing (documented on EvaluationContext);
    // the one Jaxen case that expects an error depends on it (see
    // CONFORMANCE.md). This pins the actual behaviour.
    let d = parse_xml(NSDOC);
    // Hook present but without the prefix:
    assert_eq!(
        match eval_full(&d, "/", Some(&[("other", "urn:x")][..]), &[], "/p:a").unwrap() {
            Value::NodeSet(v) => v.len(),
            _ => panic!(),
        },
        0
    );
    // No hook at all:
    assert_eq!(count(&d, "/p:a"), 0);
    assert_eq!(count(&d, "/p:a/p:c"), 0);
    assert_eq!(count(&d, "//p:*"), 0);
    // Not an error, and unprefixed tests are unaffected.
    assert!(!fails(&d, "/p:a"));
    assert_eq!(count(&d, "/*"), 1);
    assert_eq!(count(&d, "/*/e"), 1);
}

#[test]
fn namespace_functions() {
    let d = parse_xml(NSDOC);
    assert_eq!(string(&d, "local-name(/*)"), "a");
    assert_eq!(string(&d, "namespace-uri(/*)"), "urn:x");
    assert_eq!(string(&d, "namespace-uri(/*/*[1])"), "urn:d");
    assert_eq!(string(&d, "namespace-uri(/*/e)"), "");
    assert_eq!(string(&d, "namespace-uri(/*/*[2])"), "urn:x");
    assert_eq!(string(&d, "name(/*)"), "p:a");
    assert_eq!(string(&d, "name(/*/*[1])"), "b"); // default namespace: no prefix
    assert_eq!(string(&d, "name(/*/*[2])"), "p:c");
    assert_eq!(string(&d, "name(/*/e)"), "e");
    assert_eq!(string(&d, "local-name(/*/*[3])"), "d");
    // Name tests compared via the functions, independent of any binding.
    assert_eq!(
        count(&d, "/*[namespace-uri()='urn:x' and local-name()='a']"),
        1
    );
    assert_eq!(count(&d, "/*/*[namespace-uri()='urn:x']"), 2);
    assert_eq!(count(&d, "/*/*[namespace-uri()='']"), 1);
    // namespace declarations are not attributes (5.4).
    assert_eq!(count(&d, "/*/@*"), 0);
    assert_eq!(count(&d, "//@*"), 0);
}

#[test]
fn namespace_axis() {
    let d = parse_xml(NSDOC);
    // 2.2/5.4: the namespace axis of an element holds one node per in-scope
    // prefix, always including `xml`; the root and other node types have none.
    assert_eq!(count(&d, "namespace::*"), 0); // root node
    assert_eq!(count(&d, "/namespace::*"), 0);
    assert_eq!(count(&d, "/*/namespace::*"), 3); // xml, p, default
    assert_eq!(count(&d, "/*/*[3]/namespace::*"), 4); // + q
    assert_eq!(count(&d, "/*/namespace::xml"), 1);
    assert_eq!(count(&d, "/*/namespace::p"), 1);
    assert_eq!(count(&d, "/*/namespace::q"), 0);
    assert_eq!(count(&d, "/*/*[3]/namespace::q"), 1);
    // The name of a namespace node is its prefix, its string-value the URI.
    assert_eq!(string(&d, "name(/*/namespace::p)"), "p");
    assert_eq!(string(&d, "/*/namespace::p"), "urn:x");
    assert_eq!(
        string(&d, "/*/namespace::xml"),
        "http://www.w3.org/XML/1998/namespace"
    );
    // Name tests on the namespace axis match the prefix, not the URI.
    assert_eq!(count(&d, "/*/namespace::z"), 0);
    // Every element has its own copies, so //namespace::* exceeds one element's.
    assert!(count(&d, "//namespace::*") > count(&d, "/*/namespace::*"));
    // The parent of a namespace node is its element.
    assert_eq!(count(&d, "/*/namespace::p/parent::*"), 1);
    assert_eq!(string(&d, "name(/*/namespace::p/..)"), "p:a");
    assert_eq!(count_at(&d, "/*/namespace::p", "parent::*"), 1);
    // Namespace nodes are not children.
    assert_eq!(count(&d, "/*/node()"), 4);
}

// ---------------------------------------------------------------------------
// 12 Errors (section 3, 4)

#[test]
fn syntax_errors_are_rejected() {
    for bad in [
        "",
        "//",
        "/lib/",
        "1 +",
        "+ 1",
        "foo(",
        "foo(1,",
        "@",
        "a[",
        "a[1",
        "]",
        ")",
        "(1",
        "1 2",
        "a b",
        "'unterminated",
        "\"unterminated",
        "/lib/child::",
        "bogus::x",
        "$",
        "$1a",
        "/lib/@@n",
        "/descendant::()",
        "a[]",
        "a | ",
        "| a",
        "1 = ",
        "child::*::x",
        "/lib/shelf[1]]",
        "text(1)",
        "node(",
        "/a b",
        "//@",
    ] {
        assert!(parse(bad).is_err(), "{bad:?} must not parse");
    }
}

#[test]
fn evaluation_errors() {
    let d = lib();
    // 3.2: calling an unknown function is an error.
    assert!(fails(&d, "nonesuch()"));
    assert!(fails(&d, "nonesuch(1)"));
    // 4: wrong number of arguments.
    assert!(fails(&d, "count()"));
    assert!(fails(&d, "count(//t, //p)"));
    assert!(fails(&d, "contains('a')"));
    assert!(fails(&d, "contains('a', 'b', 'c')"));
    assert!(fails(&d, "translate('a', 'b')"));
    assert!(fails(&d, "floor()"));
    assert!(fails(&d, "round(1, 2)"));
    assert!(fails(&d, "lang()"));
    assert!(fails(&d, "position(1)"));
    // 4.1: count() requires a node-set.
    assert!(fails(&d, "count(1)"));
    assert!(fails(&d, "count('a')"));
    assert!(fails(&d, "count(true())"));
    // 3.3: a path or predicate step needs a node-set.
    assert!(fails(&d, "'abc'/x"));
    assert!(fails(&d, "(1 + 1)/x"));
    assert!(fails(&d, "'abc'[1]"));
    assert!(fails(&d, "(1)[1]"));
    assert!(fails(&d, "count(//t)/x"));
    // Unprefixed variable that is not bound, and function names are
    // case-sensitive.
    assert!(fails(&d, "$nothing"));
    assert!(fails(&d, "COUNT(//t)"));
    assert!(fails(&d, "Not(true())"));
    // Valid neighbours of the above do not fail.
    assert!(!fails(&d, "count(//t)"));
    assert!(!fails(&d, "(//t)[1]/text()"));
    assert!(!fails(&d, "id('x')/y"));
}

// ---------------------------------------------------------------------------
// literals and lexical corner cases (section 3.1, 3.7)

#[test]
fn literals_and_names() {
    let d = lib();
    assert_eq!(string(&d, "\"it's\""), "it's");
    assert_eq!(string(&d, "'say \"hi\"'"), "say \"hi\"");
    assert_eq!(string(&d, "''"), "");
    assert_eq!(string(&d, "\"\""), "");
    // Names with dots, underscores and hyphens (NCName).
    let n = parse_xml("<r><a.b>1</a.b><_c>2</_c><d-e>3</d-e><f1>4</f1></r>");
    assert_eq!(string(&n, "/r/a.b"), "1");
    assert_eq!(string(&n, "/r/_c"), "2");
    assert_eq!(string(&n, "/r/d-e"), "3");
    assert_eq!(string(&n, "/r/f1"), "4");
    assert_eq!(number(&n, "sum(/r/*)"), 10.0);
    // A name that looks like an axis or node type is still a name when it
    // is not followed by `::` or `(`.
    let k = parse_xml("<r><child>1</child><text>2</text><node>3</node><comment>4</comment></r>");
    assert_eq!(string(&k, "/r/child"), "1");
    assert_eq!(string(&k, "/r/text"), "2");
    assert_eq!(string(&k, "/r/node"), "3");
    assert_eq!(string(&k, "/r/comment"), "4");
    assert_eq!(count(&k, "/r/text()"), 0);
    assert_eq!(count(&k, "/r/child::child"), 1);
    // Numbers: integers, decimals, leading/trailing dot.
    assert_eq!(number(&d, "007"), 7.0);
    assert_eq!(number(&d, "1.50"), 1.5);
    assert_eq!(number(&d, "3."), 3.0);
    // Unprefixed paths on documents with surrounding comments, and `.`.
    assert_eq!(string_at(&d, "/lib/shelf[1]/@n", "."), "1");
    assert_eq!(count_at(&d, "/lib/shelf[1]", "."), 1);
    assert_eq!(count_at(&d, "/lib/shelf[1]", "./."), 1);
    assert_eq!(count_at(&d, "/lib/shelf[1]", "../shelf"), 2);
}

#[test]
fn parsed_expression_reuse_is_stateless() {
    // Evaluating one parsed expression against several contexts (with the
    // same context repeated) gives independent results.
    let d = lib();
    let e = parse("count(preceding::*)").unwrap();
    let at = |p: &str| {
        let Value::NodeSet(n) =
            evaluate(&parse(p).unwrap(), &EvaluationContext::new(d.root())).unwrap()
        else {
            panic!()
        };
        n[0]
    };
    let a = evaluate(&e, &EvaluationContext::new(at("/lib/shelf[2]/note"))).unwrap();
    let b = evaluate(&e, &EvaluationContext::new(at("/lib/shelf[1]/book[1]/t"))).unwrap();
    let a2 = evaluate(&e, &EvaluationContext::new(at("/lib/shelf[2]/note"))).unwrap();
    assert_eq!(a.to_number(), 10.0);
    assert_eq!(b.to_number(), 0.0);
    assert_eq!(a2.to_number(), 10.0);
}
