//! SVRL output (barrierlab#79): structure of the serialised trace, and that
//! every generated location path selects exactly the node it names.

mod support;

use schematron_engine::{
    Event, evaluate, evaluate_trace, evaluate_trace_with_phase, location_path, parse, to_svrl,
    to_svrl_with,
};
use support::RoxmlDoc;
use xpath_eval::{Document, EvaluationContext, Value};

const SCHEMA: &str = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
    <ns prefix="x" uri="urn:x"/>
    <phase id="p1"><active pattern="items"/></phase>
    <pattern id="items">
        <rule context="x:item">
            <assert id="has-id" role="error" test="@id" diagnostics="d1">item <name/> needs an id &amp; more</assert>
            <report test="@old">legacy "item"</report>
        </rule>
    </pattern>
    <pattern id="other">
        <rule context="x:nothing"><assert test="false()">never</assert></rule>
    </pattern>
    <diagnostics><diagnostic id="d1">saw <value-of select="count(../x:item)"/> items</diagnostic></diagnostics>
</schema>"#;

const XML: &str = r#"<x:list xmlns:x="urn:x"><x:item id="a"/><x:item old="1"/></x:list>"#;

#[test]
fn trace_serialises_to_svrl() {
    let schema = parse(SCHEMA).unwrap();
    let doc = RoxmlDoc::parse(XML);
    let events = evaluate_trace_with_phase(&schema, &doc, Some("p1")).unwrap();
    let svrl = to_svrl(&schema, Some("p1"), &events);
    let expected = r#"<svrl:schematron-output xmlns:svrl="http://purl.oclc.org/dsdl/svrl" phase="p1">
  <svrl:ns-prefix-in-attribute-values prefix="x" uri="urn:x"/>
  <svrl:active-pattern id="items"/>
  <svrl:fired-rule context="x:item"/>
  <svrl:fired-rule context="x:item"/>
  <svrl:failed-assert id="has-id" role="error" test="@id" location="/*[local-name()='list' and namespace-uri()='urn:x'][1]/*[local-name()='item' and namespace-uri()='urn:x'][2]">
    <svrl:diagnostic-reference diagnostic="d1">saw 2 items</svrl:diagnostic-reference>
    <svrl:text>item x:item needs an id &amp; more</svrl:text>
  </svrl:failed-assert>
  <svrl:successful-report test="@old" location="/*[local-name()='list' and namespace-uri()='urn:x'][1]/*[local-name()='item' and namespace-uri()='urn:x'][2]">
    <svrl:text>legacy &quot;item&quot;</svrl:text>
  </svrl:successful-report>
</svrl:schematron-output>
"#;
    assert_eq!(svrl, expected);
}

#[test]
fn trace_checks_match_plain_evaluation() {
    let schema = parse(SCHEMA).unwrap();
    let doc = RoxmlDoc::parse(XML);
    let fired = evaluate(&schema, &doc).unwrap();
    let checks = evaluate_trace(&schema, &doc)
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event, Event::Check { .. }))
        .count();
    assert_eq!(fired.len(), checks);
}

#[test]
fn passing_document_has_no_check_elements() {
    let schema = parse(SCHEMA).unwrap();
    let doc = RoxmlDoc::parse(r#"<x:list xmlns:x="urn:x"><x:item id="a"/></x:list>"#);
    let svrl = to_svrl(&schema, None, &evaluate_trace(&schema, &doc).unwrap());
    assert!(svrl.contains("<svrl:fired-rule"));
    assert!(!svrl.contains("failed-assert") && !svrl.contains("successful-report"));
    assert!(!svrl.contains("phase="));
}

#[test]
fn custom_location_is_used() {
    let schema = parse(SCHEMA).unwrap();
    let doc = RoxmlDoc::parse(XML);
    let events = evaluate_trace(&schema, &doc).unwrap();
    let svrl = to_svrl_with(&schema, None, &events, |_| "here".to_owned());
    assert!(svrl.contains(r#"location="here""#));
}

/// Every node of a document with namespaces, repeated names, text, comment
/// and attributes: the path selects exactly that node.
#[test]
fn location_paths_round_trip() {
    let doc = RoxmlDoc::parse(
        r#"<a xmlns:p="urn:p" k="1" p:k="2">t1<b/><p:b>x</p:b><b>y<!--c-->z<b/></b>t2</a>"#,
    );
    let root = doc.root();
    let mut stack = vec![root];
    let mut seen = 0;
    while let Some(node) = stack.pop() {
        use xpath_eval::Node;
        let path = location_path(node);
        let expr = xpath_eval::parse(&path).unwrap_or_else(|e| panic!("{path}: {e:?}"));
        let value = xpath_eval::evaluate(&expr, &EvaluationContext::new(root)).unwrap();
        match value {
            Value::NodeSet(set) => {
                let set: Vec<_> = set.into_iter().collect();
                assert_eq!(set, vec![node], "{path}");
            }
            other => panic!("{path}: {other:?}"),
        }
        seen += 1;
        stack.extend(node.children());
        stack.extend(node.attributes());
    }
    assert!(seen >= 12, "only {seen} nodes visited");
}
