//! SVRL (Schematron Validation Report Language, ISO/IEC 19757-3 Annex D,
//! informative) output, written as a string: no XML library is involved.
//!
//! [`crate::evaluate_trace`] records what SVRL reports (active patterns,
//! fired rules, fired checks); [`to_svrl`] serialises that trace.

use std::fmt::Write;

use xpath_eval::{ExpandedName, Node, NodeKind};

use crate::engine::Report;
use crate::schema::{CheckKind, Schema};

const SVRL_NS: &str = "http://purl.oclc.org/dsdl/svrl";

/// One step of an evaluation, in evaluation order. SVRL is flat: a
/// [`Event::FiredRule`] belongs to the preceding [`Event::ActivePattern`],
/// a [`Event::Check`] to the preceding [`Event::FiredRule`].
#[derive(Clone, Debug)]
pub enum Event<N> {
    /// A pattern that took part in the run (active in the chosen phase).
    ActivePattern { id: Option<String> },
    /// A rule matched `node` (one event per matched node).
    FiredRule { context: String, node: N },
    /// A fired `assert` (failed) or `report` (successful).
    Check {
        report: Report<N>,
        /// The check's raw `test` expression.
        test: String,
        /// The ids of `report.diagnostics`, same order and length.
        diagnostic_ids: Vec<String>,
    },
}

/// Serialises a trace as an SVRL document, with node locations from
/// [`location_path`]. `phase` is the phase the trace was produced with.
pub fn to_svrl<'a, N: Node<'a>>(
    schema: &Schema,
    phase: Option<&str>,
    events: &[Event<N>],
) -> String {
    to_svrl_with(schema, phase, events, location_path)
}

/// [`to_svrl`] with a caller-supplied `location` for a node, for hosts that
/// know a better path (e.g. with source ids) than the generic one.
pub fn to_svrl_with<'a, N: Node<'a>>(
    schema: &Schema,
    phase: Option<&str>,
    events: &[Event<N>],
    location: impl Fn(N) -> String,
) -> String {
    let mut out = String::new();
    let _ = write!(out, "<svrl:schematron-output xmlns:svrl=\"{SVRL_NS}\"");
    if let Some(phase) = phase {
        let _ = write!(out, " phase=\"{}\"", escape(phase));
    }
    out.push_str(">\n");
    for ns in &schema.ns {
        let _ = writeln!(
            out,
            "  <svrl:ns-prefix-in-attribute-values prefix=\"{}\" uri=\"{}\"/>",
            escape(&ns.prefix),
            escape(&ns.uri)
        );
    }
    for event in events {
        match event {
            Event::ActivePattern { id } => {
                out.push_str("  <svrl:active-pattern");
                if let Some(id) = id {
                    let _ = write!(out, " id=\"{}\"", escape(id));
                }
                out.push_str("/>\n");
            }
            Event::FiredRule { context, .. } => {
                let _ = writeln!(out, "  <svrl:fired-rule context=\"{}\"/>", escape(context));
            }
            Event::Check {
                report,
                test,
                diagnostic_ids,
            } => {
                let element = match report.kind {
                    CheckKind::Assert => "failed-assert",
                    CheckKind::Report => "successful-report",
                };
                let _ = write!(out, "  <svrl:{element}");
                if let Some(id) = &report.check_id {
                    let _ = write!(out, " id=\"{}\"", escape(id));
                }
                if let Some(role) = &report.role {
                    let _ = write!(out, " role=\"{}\"", escape(role));
                }
                let _ = writeln!(
                    out,
                    " test=\"{}\" location=\"{}\">",
                    escape(test),
                    escape(&location(report.node))
                );
                for (id, text) in diagnostic_ids.iter().zip(&report.diagnostics) {
                    let _ = writeln!(
                        out,
                        "    <svrl:diagnostic-reference diagnostic=\"{}\">{}</svrl:diagnostic-reference>",
                        escape(id),
                        escape(text)
                    );
                }
                let _ = writeln!(
                    out,
                    "    <svrl:text>{}</svrl:text>\n  </svrl:{element}>",
                    escape(&report.message)
                );
            }
        }
    }
    out.push_str("</svrl:schematron-output>\n");
    out
}

/// An XPath 1.0 location path selecting exactly `node`, e.g.
/// `/a[1]/*[local-name()='b' and namespace-uri()='urn:x'][2]/@id`. Names in
/// no namespace are written as plain name tests; others by `local-name()`
/// and `namespace-uri()`, so the path needs no prefix bindings.
pub fn location_path<'a, N: Node<'a>>(node: N) -> String {
    let mut steps = Vec::new();
    let mut current = node;
    while current.kind() != NodeKind::Root {
        steps.push(step(current));
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    if steps.is_empty() {
        return "/".to_owned();
    }
    steps.iter().rev().fold(String::new(), |mut path, step| {
        path.push('/');
        path.push_str(step);
        path
    })
}

/// 1-based position of `node` among its preceding siblings of the same
/// kind and expanded name, itself included.
fn position<'a, N: Node<'a>>(node: N, name: &Option<ExpandedName>) -> usize {
    node.parent().map_or(1, |parent| {
        parent
            .children()
            .take_while(|&sibling| sibling != node)
            .filter(|sibling| sibling.kind() == node.kind() && &sibling.expanded_name() == name)
            .count()
            + 1
    })
}

fn step<'a, N: Node<'a>>(node: N) -> String {
    let name = node.expanded_name();
    match node.kind() {
        NodeKind::Element => format!("{}[{}]", name_test(name.as_ref()), position(node, &name)),
        NodeKind::Attribute => format!("@{}", name_test(name.as_ref())),
        NodeKind::Namespace => match &name {
            Some(name) => format!("namespace::*[local-name()={}]", literal(&name.local_name)),
            None => "namespace::*[not(local-name())]".to_owned(),
        },
        NodeKind::Text => format!("text()[{}]", position(node, &name)),
        NodeKind::Comment => format!("comment()[{}]", position(node, &name)),
        NodeKind::ProcessingInstruction | NodeKind::Root => {
            let test = match &name {
                Some(name) => format!("processing-instruction({})", literal(&name.local_name)),
                None => "processing-instruction()".to_owned(),
            };
            format!("{test}[{}]", position(node, &name))
        }
    }
}

fn name_test(name: Option<&ExpandedName>) -> String {
    match name {
        None => "*".to_owned(),
        Some(name) => match &name.namespace_uri {
            None => name.local_name.clone(),
            Some(uri) => format!(
                "*[local-name()={} and namespace-uri()={}]",
                literal(&name.local_name),
                literal(uri)
            ),
        },
    }
}

fn literal(value: &str) -> String {
    if value.contains('\'') {
        format!("\"{value}\"")
    } else {
        format!("'{value}'")
    }
}

/// Escapes text for element content and double-quoted attribute values;
/// tab and line breaks become character references so they survive
/// attribute-value normalisation.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out
}
