//! Runs the Jaxen XPath test cases (`tests/corpus/jaxen/tests.xml`). The corpus
//! is NOT in the repository: fetch it with `xtask/vendor-jaxen.sh`, then run
//! `cargo test -p xpath-eval --test jaxen -- --ignored` (see `CONFORMANCE.md`).
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
//! every entry with `JAXEN_BLESS=1 cargo test -p xpath-eval --test jaxen -- --ignored`.

use std::collections::{BTreeMap, HashSet};
use std::fs;

mod common;

use common::{Doc, N, parse_xml};
use xpath_eval::{Document, EvaluationContext, QName, Value, evaluate, parse};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/jaxen");
const KNOWN_FAILURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/jaxen_known_failures.txt"
);
const HEADER_END: &str = "# --- entries below, one per line ---\n";
const VAR_URI: &str = "https://github.com/jaxen-xpath/jaxen/test-harness/var";

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
    parse_xml(&read_text(&format!("{CORPUS}/{url}")))
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
#[ignore = "needs the externally fetched Jaxen corpus, see CONFORMANCE.md"]
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
