//! Runs the Jaxen XPath test cases (`tests/corpus/jaxen/tests.xml`, vendored,
//! see `UPSTREAM.md` there; the file is the source of Jaxen's `XPathTestBase`).
//!
//! Semantics follow Jaxen's generator (`tests.xsl`): a `<context>` selects
//! context nodes in the document (one run per node); its in-scope namespace
//! declarations (except `var`/`xml`) become the namespace context and its
//! attributes in the `var` namespace become string variables. Inside:
//!
//! - `<test select count>`: the result is a node-set of that size;
//! - `<test select exception="true">`: parsing or evaluating fails;
//! - `<valueOf select>text</valueOf>`: `string(select)` equals the text; inside
//!   a `<test>` it is evaluated against the first selected node.
//!
//! Failing cases are listed in `tests/jaxen_known_failures.txt` as
//! `tests.xml:<line>`; the list is checked both ways. Re-bless after examining
//! every entry with `JAXEN_BLESS=1 cargo test -p xpath-eval --test jaxen`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::fs;

use xpath_eval::{
    Document, EvaluationContext, ExpandedName, Node as XNode, NodeKind, QName, Value, evaluate,
    parse,
};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/jaxen");
const KNOWN_FAILURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/jaxen_known_failures.txt"
);
const HEADER_END: &str = "# --- entries below, one per line ---\n";
const VAR_URI: &str = "https://github.com/jaxen-xpath/jaxen/test-harness/var";

struct Data {
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
    parent: Option<usize>,
    children: Vec<usize>,
    attributes: Vec<usize>,
    namespaces: Vec<usize>,
}

/// Arena in document order, so `document_order` is an index comparison.
struct Doc(Vec<Data>);

#[derive(Clone, Copy)]
struct N<'a>(&'a Doc, usize);

impl PartialEq for N<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for N<'_> {}

impl Document for Doc {
    type N<'a> = N<'a>;
    fn root(&self) -> N<'_> {
        N(self, 0)
    }
}

impl<'a> XNode<'a> for N<'a> {
    fn kind(self) -> NodeKind {
        self.0.0[self.1].kind
    }
    fn parent(self) -> Option<Self> {
        self.0.0[self.1].parent.map(|p| N(self.0, p))
    }
    fn children(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1].children.iter().map(move |&c| N(self.0, c))
    }
    fn attributes(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1]
            .attributes
            .iter()
            .map(move |&c| N(self.0, c))
    }
    fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
        self.0.0[self.1]
            .namespaces
            .iter()
            .map(move |&c| N(self.0, c))
    }
    fn expanded_name(self) -> Option<ExpandedName> {
        self.0.0[self.1].name.clone()
    }
    fn string_value(self) -> String {
        let d = &self.0.0[self.1];
        match d.kind {
            NodeKind::Root | NodeKind::Element => {
                let mut out = String::new();
                collect_text(self.0, self.1, &mut out);
                out
            }
            _ => d.text.clone().unwrap_or_default(),
        }
    }
    fn document_order(self, other: Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

fn collect_text(doc: &Doc, idx: usize, out: &mut String) {
    for &c in &doc.0[idx].children {
        let d = &doc.0[c];
        match d.kind {
            NodeKind::Text => out.push_str(d.text.as_deref().unwrap_or_default()),
            NodeKind::Element => collect_text(doc, c, out),
            _ => {}
        }
    }
}

/// XPath knows "in a namespace" or "in none", never the empty URI.
fn ns(uri: Option<&str>) -> Option<String> {
    uri.filter(|u| !u.is_empty()).map(str::to_owned)
}

fn push(
    nodes: &mut Vec<Data>,
    parent: usize,
    kind: NodeKind,
    name: Option<ExpandedName>,
    text: Option<String>,
) -> usize {
    let idx = nodes.len();
    nodes.push(Data {
        kind,
        name,
        text,
        parent: Some(parent),
        children: Vec::new(),
        attributes: Vec::new(),
        namespaces: Vec::new(),
    });
    idx
}

fn add(nodes: &mut Vec<Data>, parent: usize, x: roxmltree::Node<'_, '_>) {
    for c in x.children() {
        match c.node_type() {
            roxmltree::NodeType::Element => {
                let name = ExpandedName {
                    namespace_uri: ns(c.tag_name().namespace()),
                    local_name: c.tag_name().name().to_owned(),
                };
                let idx = push(nodes, parent, NodeKind::Element, Some(name), None);
                nodes[parent].children.push(idx);
                // XPath 1.0 (5.4): every element has the `xml` namespace node;
                // roxmltree does not report it.
                let xml = ExpandedName {
                    namespace_uri: None,
                    local_name: "xml".to_owned(),
                };
                let uri = "http://www.w3.org/XML/1998/namespace".to_owned();
                let i = push(nodes, idx, NodeKind::Namespace, Some(xml), Some(uri));
                nodes[idx].namespaces.push(i);
                for n in c.namespaces() {
                    let name = ExpandedName {
                        namespace_uri: None,
                        local_name: n.name().unwrap_or_default().to_owned(),
                    };
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Namespace,
                        Some(name),
                        Some(n.uri().to_owned()),
                    );
                    nodes[idx].namespaces.push(i);
                }
                for a in c.attributes() {
                    let name = ExpandedName {
                        namespace_uri: ns(a.namespace()),
                        local_name: a.name().to_owned(),
                    };
                    let i = push(
                        nodes,
                        idx,
                        NodeKind::Attribute,
                        Some(name),
                        Some(a.value().to_owned()),
                    );
                    nodes[idx].attributes.push(i);
                }
                add(nodes, idx, c);
            }
            roxmltree::NodeType::Text => {
                let i = push(
                    nodes,
                    parent,
                    NodeKind::Text,
                    None,
                    Some(c.text().unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Comment => {
                let i = push(
                    nodes,
                    parent,
                    NodeKind::Comment,
                    None,
                    Some(c.text().unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::PI => {
                let pi = c.pi().expect("PI node");
                let name = ExpandedName {
                    namespace_uri: None,
                    local_name: pi.target.to_owned(),
                };
                let i = push(
                    nodes,
                    parent,
                    NodeKind::ProcessingInstruction,
                    Some(name),
                    Some(pi.value.unwrap_or_default().to_owned()),
                );
                nodes[parent].children.push(i);
            }
            roxmltree::NodeType::Root => {}
        }
    }
}

/// Reads a file as UTF-8, or as ISO-8859-1 when it is not UTF-8
/// (`moreover.xml`); roxmltree takes `&str` and ignores the declaration.
fn read_text(path: &str) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|e| panic!("{path}: {e} - run xtask/vendor-jaxen.sh"));
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    }
}

fn load(url: &str) -> Doc {
    let text = read_text(&format!("{CORPUS}/{url}"));
    let x = roxmltree::Document::parse_with_options(
        &text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{url}: {e}"));
    let mut nodes = vec![Data {
        kind: NodeKind::Root,
        name: None,
        text: None,
        parent: None,
        children: Vec::new(),
        attributes: Vec::new(),
        namespaces: Vec::new(),
    }];
    add(&mut nodes, 0, x.root());
    Doc(nodes)
}

struct Env {
    namespaces: Vec<(String, String)>,
    variables: Vec<(String, String)>,
}

/// Evaluates `select` with `node` as the context node.
fn eval<'a>(env: &Env, node: N<'a>, select: &str) -> Result<Value<N<'a>>, String> {
    let expr = parse(select).map_err(|e| e.to_string())?;
    let lookup_ns = |p: &str| {
        env.namespaces
            .iter()
            .find(|(k, _)| k == p)
            .map(|(_, v)| v.clone())
    };
    let lookup_var = |q: &QName| {
        if q.prefix.is_some() {
            return None;
        }
        env.variables
            .iter()
            .find(|(k, _)| *k == q.local)
            .map(|(_, v)| Value::String(v.clone()))
    };
    let mut ctx = EvaluationContext::new(node);
    ctx.namespaces = Some(&lookup_ns);
    ctx.variables = Some(&lookup_var);
    evaluate(&expr, &ctx).map_err(|e| e.to_string())
}

fn string_of(v: &Value<N<'_>>) -> String {
    v.to_xpath_string()
}

fn check_value_of<'a>(env: &Env, node: N<'a>, v: roxmltree::Node<'_, '_>) -> Result<(), String> {
    let select = v.attribute("select").unwrap();
    let expected = v.text().unwrap_or_default();
    let got = string_of(&eval(env, node, select)?);
    if got == expected {
        Ok(())
    } else {
        Err(format!(
            "valueOf {select:?}: expected {expected:?}, got {got:?}"
        ))
    }
}

fn check_test<'a>(env: &Env, node: N<'a>, t: roxmltree::Node<'_, '_>) -> Result<(), String> {
    let select = t.attribute("select").unwrap();
    if t.attribute("exception").is_some() {
        return match eval(env, node, select) {
            Err(_) => Ok(()),
            Ok(_) => Err(format!("{select:?}: an error was expected")),
        };
    }
    let Value::NodeSet(nodes) = eval(env, node, select)? else {
        return Err(format!("{select:?}: not a node-set"));
    };
    if let Some(count) = t.attribute("count") {
        let count: usize = count.parse().unwrap();
        if nodes.len() != count {
            return Err(format!(
                "{select:?}: expected {count} nodes, got {}",
                nodes.len()
            ));
        }
    }
    for v in t.children().filter(|c| c.has_tag_name("valueOf")) {
        let first = *nodes
            .first()
            .ok_or_else(|| format!("{select:?}: no node for valueOf"))?;
        check_value_of(env, first, v)?;
    }
    Ok(())
}

fn bekannte() -> HashSet<String> {
    fs::read_to_string(KNOWN_FAILURES)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn jaxen_xpath_corpus() {
    let text = read_text(&format!("{CORPUS}/tests.xml"));
    let tests = roxmltree::Document::parse(&text).unwrap();
    let mut cases = 0usize;
    // key -> detail; one entry per case line, first failure wins.
    let mut failures: BTreeMap<String, String> = BTreeMap::new();
    let mut record = |x: roxmltree::Node<'_, '_>, r: Result<(), String>| {
        cases += 1;
        if let Err(d) = r {
            let row = tests.text_pos_at(x.range().start).row;
            failures.entry(format!("tests.xml:{row:04}")).or_insert(d);
        }
    };
    for d in tests
        .root_element()
        .children()
        .filter(|c| c.has_tag_name("document"))
    {
        let doc = load(d.attribute("url").unwrap());
        for c in d.children().filter(|c| c.has_tag_name("context")) {
            let env = Env {
                namespaces: c
                    .namespaces()
                    .filter(|n| !matches!(n.name(), None | Some("var") | Some("xml")))
                    .map(|n| (n.name().unwrap().to_owned(), n.uri().to_owned()))
                    .collect(),
                variables: c
                    .attributes()
                    .filter(|a| a.namespace() == Some(VAR_URI))
                    .map(|a| (a.name().to_owned(), a.value().to_owned()))
                    .collect(),
            };
            let no_env = Env {
                namespaces: Vec::new(),
                variables: Vec::new(),
            };
            let contexts = match eval(&no_env, doc.root(), c.attribute("select").unwrap()) {
                Ok(Value::NodeSet(n)) => n,
                other => {
                    record(
                        c,
                        Err(format!("context did not select nodes: {:?}", other.err())),
                    );
                    continue;
                }
            };
            for &node in &contexts {
                for x in c.children().filter(roxmltree::Node::is_element) {
                    let r = match x.tag_name().name() {
                        "test" => check_test(&env, node, x),
                        "valueOf" => check_value_of(&env, node, x),
                        other => Err(format!("unknown element {other}")),
                    };
                    record(x, r);
                }
            }
        }
    }
    assert!(cases > 0, "no cases found in the corpus");

    if std::env::var_os("JAXEN_BLESS").is_some() {
        let old = fs::read_to_string(KNOWN_FAILURES).unwrap_or_default();
        let head = old
            .find(HEADER_END)
            .unwrap_or_else(|| panic!("header of {KNOWN_FAILURES} lacks the marker line"))
            + HEADER_END.len();
        let mut out = old[..head].to_owned();
        out.push('\n');
        for (k, d) in &failures {
            out.push_str(&format!("{k}  {d}\n"));
        }
        fs::write(KNOWN_FAILURES, out).unwrap();
        println!(
            "JAXEN_BLESS: {} of {cases} case runs listed",
            failures.len()
        );
        return;
    }

    let known = bekannte();
    let new: Vec<String> = failures
        .iter()
        .filter(|(k, _)| !known.contains(*k))
        .map(|(k, d)| format!("{k}: {d}"))
        .collect();
    let stale: Vec<&String> = known
        .iter()
        .filter(|k| !failures.contains_key(*k))
        .collect();
    assert!(
        new.is_empty(),
        "{} case(s) not in tests/jaxen_known_failures.txt:\n{}",
        new.len(),
        new.join("\n")
    );
    assert!(
        stale.is_empty(),
        "tests/jaxen_known_failures.txt lists passing cases {stale:?} - re-bless with JAXEN_BLESS=1"
    );
}
