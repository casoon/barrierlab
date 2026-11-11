//! `xsl:function` declarations of a schema (XSLT 2.0, section 10.3), read into a small model that
//! `engine` runs when an XPath 2.0 expression calls the function. Only the part of XSLT the
//! inventoried rule sets (Peppol BIS Billing 3.0, XRechnung) use is modelled: `xsl:param`,
//! `xsl:variable`, `xsl:sequence`, `xsl:value-of`, `xsl:text`, `xsl:if` and
//! `xsl:choose`/`xsl:when`/`xsl:otherwise`. Any other instruction in a body is recorded as the
//! reason the function is unsupported ([`XsltFunction::unsupported`]); calling such a function
//! ends the run as unsupported, never as a pass.

use roxmltree::Node;
use xpath_eval::AtomicType;

use crate::schema::NamespaceBinding;

const XSLT_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";
const XS_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

/// Instructions nested deeper than this (an `xsl:if` in an `xsl:when` in ...) make the function
/// unsupported: the interpreter that runs them is recursive.
const MAX_NESTING: usize = 32;

/// One `xsl:function` child of the `schema` element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XsltFunction {
    /// `@name` as written (`u:mod11`).
    pub name: String,
    /// The names of the `xsl:param` children, in order; their number is the arity.
    pub parameters: Vec<String>,
    /// The first construct of the function that this crate does not run, as a sentence; calling
    /// the function is then an unsupported-feature error carrying it. `None`: the function runs.
    pub unsupported: Option<String>,
    /// The namespace URI of `name`, from the schema's `ns` elements or, failing that, the
    /// namespace declarations in scope at the declaration. `None` if the prefix is bound nowhere.
    pub(crate) namespace: Option<String>,
    pub(crate) local: String,
    pub(crate) params: Vec<Param>,
    pub(crate) result_type: Option<SequenceType>,
    pub(crate) body: Vec<Instruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Param {
    pub name: String,
    pub declared: Option<SequenceType>,
}

/// `@as`: an item type and an occurrence indicator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SequenceType {
    pub item: ItemType,
    pub occurrence: Occurrence,
    pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ItemType {
    Item,
    Node,
    Atomic(AtomicType),
    /// A type this crate does not check (`element()`, `xs:dateTime`, ...): using it is unsupported.
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Occurrence {
    One,
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Instruction {
    Variable {
        name: String,
        declared: Option<SequenceType>,
        value: VariableValue,
    },
    Sequence {
        select: String,
    },
    ValueOf {
        select: String,
        separator: Option<String>,
    },
    Text(String),
    If {
        test: String,
        body: Vec<Instruction>,
    },
    Choose {
        whens: Vec<(String, Vec<Instruction>)>,
        otherwise: Option<Vec<Instruction>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VariableValue {
    Select(String),
    /// A sequence constructor (with `@as`).
    Content(Vec<Instruction>),
    /// Text only, no `@as`: bound to the string-value of the text.
    Text(String),
}

/// The `xsl:function` elements that are children of `root`.
pub(crate) fn parse_functions(root: Node, ns: &[NamespaceBinding]) -> Vec<XsltFunction> {
    root.children()
        .filter(|child| is_xslt(*child, "function"))
        .filter_map(|function| parse_function(function, ns))
        .collect()
}

fn is_xslt(node: Node, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(XSLT_NAMESPACE)
        && node.tag_name().name() == local
}

/// The schema's `ns` bindings first, then the declarations in scope at `node`.
fn resolve_prefix(node: Node, ns: &[NamespaceBinding], prefix: &str) -> Option<String> {
    ns.iter()
        .find(|binding| binding.prefix == prefix)
        .map(|binding| binding.uri.clone())
        .or_else(|| node.lookup_namespace_uri(Some(prefix)).map(str::to_owned))
}

fn parse_function(function: Node, ns: &[NamespaceBinding]) -> Option<XsltFunction> {
    let name = function.attribute("name")?;
    let (prefix, local) = match name.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, name),
    };
    let namespace = prefix.and_then(|prefix| resolve_prefix(function, ns, prefix));
    let mut unsupported = None;
    let mut note = |reason: String| {
        unsupported.get_or_insert(reason);
    };
    if prefix.is_some() && namespace.is_none() {
        note(format!(
            "the prefix of the function name {name} is not bound"
        ));
    }
    let mut params = Vec::new();
    let mut body_nodes = Vec::new();
    for child in function.children() {
        if child.is_comment()
            || child.is_pi()
            || child.is_text() && child.text().is_none_or(|text| text.trim().is_empty())
        {
            continue;
        }
        if is_xslt(child, "param") {
            if !body_nodes.is_empty() {
                note("an xsl:param after another instruction".to_owned());
            }
            let Some(param_name) = child.attribute("name") else {
                note("an xsl:param without @name".to_owned());
                continue;
            };
            if child.attribute("select").is_some() || child.has_children() {
                note(format!(
                    "a default value for the parameter {param_name} (XSLT 2.0 has none for functions)"
                ));
            }
            params.push(Param {
                name: param_name.to_owned(),
                declared: child
                    .attribute("as")
                    .map(|source| parse_type(function, ns, source)),
            });
        } else {
            body_nodes.push(child);
        }
    }
    let body = parse_instructions(function, ns, &body_nodes, 1, &mut note);
    let result_type = function
        .attribute("as")
        .map(|source| parse_type(function, ns, source));
    Some(XsltFunction {
        name: name.to_owned(),
        parameters: params.iter().map(|param| param.name.clone()).collect(),
        unsupported,
        namespace,
        local: local.to_owned(),
        params,
        result_type,
        body,
    })
}

fn parse_type(node: Node, ns: &[NamespaceBinding], source: &str) -> SequenceType {
    let trimmed = source.trim();
    let (item, occurrence) = match trimmed.chars().last() {
        Some('?') => (&trimmed[..trimmed.len() - 1], Occurrence::ZeroOrOne),
        Some('*') => (&trimmed[..trimmed.len() - 1], Occurrence::ZeroOrMore),
        Some('+') => (&trimmed[..trimmed.len() - 1], Occurrence::OneOrMore),
        _ => (trimmed, Occurrence::One),
    };
    let item = item.trim();
    let item = match item {
        "item()" => ItemType::Item,
        "node()" => ItemType::Node,
        _ => match item.split_once(':') {
            // `xs` is predefined in the static context of every XPath 2.0 expression.
            Some((prefix, local))
                if resolve_prefix(node, ns, prefix)
                    .or_else(|| (prefix == "xs").then(|| XS_NAMESPACE.to_owned()))
                    .as_deref()
                    == Some(XS_NAMESPACE) =>
            {
                match local {
                    "untypedAtomic" => ItemType::Atomic(AtomicType::UntypedAtomic),
                    "string" => ItemType::Atomic(AtomicType::String),
                    "boolean" => ItemType::Atomic(AtomicType::Boolean),
                    "integer" => ItemType::Atomic(AtomicType::Integer),
                    "decimal" => ItemType::Atomic(AtomicType::Decimal),
                    "double" => ItemType::Atomic(AtomicType::Double),
                    "date" => ItemType::Atomic(AtomicType::Date),
                    _ => ItemType::Other,
                }
            }
            _ => ItemType::Other,
        },
    };
    SequenceType {
        item,
        occurrence,
        source: trimmed.to_owned(),
    }
}

fn parse_instructions(
    scope: Node,
    ns: &[NamespaceBinding],
    nodes: &[Node],
    depth: usize,
    note: &mut impl FnMut(String),
) -> Vec<Instruction> {
    let mut out = Vec::new();
    if depth > MAX_NESTING {
        note(format!("instructions nested more than {MAX_NESTING} deep"));
        return out;
    }
    for &node in nodes {
        if node.is_comment() || node.is_pi() {
            continue;
        }
        if node.is_text() {
            let text = node.text().unwrap_or_default();
            if !text.trim().is_empty() {
                out.push(Instruction::Text(text.to_owned()));
            }
            continue;
        }
        if !node.is_element() {
            continue;
        }
        if node.tag_name().namespace() != Some(XSLT_NAMESPACE) {
            note(format!(
                "the literal result element {}",
                node.tag_name().name()
            ));
            continue;
        }
        let children: Vec<Node> = node.children().collect();
        match node.tag_name().name() {
            "variable" => {
                let Some(name) = node.attribute("name") else {
                    note("an xsl:variable without @name".to_owned());
                    continue;
                };
                let declared = node
                    .attribute("as")
                    .map(|source| parse_type(scope, ns, source));
                let value = match (node.attribute("select"), children.is_empty()) {
                    (Some(select), true) => VariableValue::Select(select.to_owned()),
                    (Some(_), false) => {
                        note(format!("the variable {name} has both @select and content"));
                        continue;
                    }
                    (None, true) => VariableValue::Text(String::new()),
                    (None, false) if declared.is_some() => VariableValue::Content(
                        parse_instructions(scope, ns, &children, depth + 1, note),
                    ),
                    (None, false) => {
                        if children
                            .iter()
                            .all(|child| child.is_text() || child.is_comment())
                        {
                            VariableValue::Text(
                                children
                                    .iter()
                                    .filter_map(|child| child.text())
                                    .collect::<String>(),
                            )
                        } else {
                            note(format!(
                                "the variable {name} has a sequence constructor and no @as (it would build a temporary tree)"
                            ));
                            continue;
                        }
                    }
                };
                out.push(Instruction::Variable {
                    name: name.to_owned(),
                    declared,
                    value,
                });
            }
            "sequence" => match node.attribute("select") {
                Some(select) => out.push(Instruction::Sequence {
                    select: select.to_owned(),
                }),
                None => note("an xsl:sequence without @select".to_owned()),
            },
            "value-of" => match node.attribute("select") {
                Some(select)
                    if node
                        .attribute("separator")
                        .is_none_or(|s| !s.contains(['{', '}'])) =>
                {
                    out.push(Instruction::ValueOf {
                        select: select.to_owned(),
                        separator: node.attribute("separator").map(str::to_owned),
                    });
                }
                Some(_) => note(
                    "an xsl:value-of with an attribute value template as @separator".to_owned(),
                ),
                None => note("an xsl:value-of without @select".to_owned()),
            },
            "text" => {
                if children.iter().all(|child| child.is_text()) {
                    let text: String = children.iter().filter_map(|child| child.text()).collect();
                    if !text.is_empty() {
                        out.push(Instruction::Text(text));
                    }
                } else {
                    note("an xsl:text with child elements".to_owned());
                }
            }
            "if" => match node.attribute("test") {
                Some(test) => out.push(Instruction::If {
                    test: test.to_owned(),
                    body: parse_instructions(scope, ns, &children, depth + 1, note),
                }),
                None => note("an xsl:if without @test".to_owned()),
            },
            "choose" => {
                let mut whens = Vec::new();
                let mut otherwise = None;
                for child in children.iter().filter(|child| child.is_element()) {
                    let grandchildren: Vec<Node> = child.children().collect();
                    if is_xslt(*child, "when") {
                        match child.attribute("test") {
                            Some(test) => whens.push((
                                test.to_owned(),
                                parse_instructions(scope, ns, &grandchildren, depth + 1, note),
                            )),
                            None => note("an xsl:when without @test".to_owned()),
                        }
                    } else if is_xslt(*child, "otherwise") {
                        otherwise = Some(parse_instructions(
                            scope,
                            ns,
                            &grandchildren,
                            depth + 1,
                            note,
                        ));
                    } else {
                        note(format!("{} inside xsl:choose", child.tag_name().name()));
                    }
                }
                out.push(Instruction::Choose { whens, otherwise });
            }
            other => note(format!("the instruction xsl:{other}")),
        }
    }
    out
}
