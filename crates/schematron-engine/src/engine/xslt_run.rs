//! Runs the `xsl:function` declarations of a schema when an XPath 2.0 expression calls one
//! (the `FunctionLookup` hook of `xpath-eval`). The model is in `crate::xslt`; the subset of
//! XSLT 2.0 it covers, and what it simplifies, is documented in
//! `docs/packages/schematron-engine.md`.

use xpath_eval::{Atomic, AtomicType, Item, Node, Sequence, VersionedError};

use super::{Bound, Env, Scope, run_in_context};
use crate::xslt::{Instruction, ItemType, Occurrence, SequenceType, VariableValue, XsltFunction};

/// The function hook: `None` if the schema declares no function of that name and arity.
pub(super) fn call<'n, N: Node<'n>>(
    env: &Env<'_, N>,
    namespace: &str,
    local: &str,
    args: Vec<Sequence<N>>,
    depth: usize,
) -> Option<Result<Sequence<N>, VersionedError>> {
    let function = env.functions.iter().find(|function| {
        function.namespace.as_deref() == Some(namespace)
            && function.local == local
            && function.params.len() == args.len()
    })?;
    Some(run_function(env, function, args, depth))
}

fn run_function<'n, N: Node<'n>>(
    env: &Env<'_, N>,
    function: &XsltFunction,
    args: Vec<Sequence<N>>,
    depth: usize,
) -> Result<Sequence<N>, VersionedError> {
    if let Some(reason) = &function.unsupported {
        return Err(VersionedError::Unsupported(format!(
            "the function {} declared with xsl:function uses {reason}",
            function.name
        )));
    }
    let mut scope = Scope::new(None);
    for (param, value) in function.params.iter().zip(args) {
        let value = match &param.declared {
            Some(declared) => convert(
                value,
                declared,
                &format!("argument ${} of {}", param.name, function.name),
            )?,
            None => value,
        };
        scope.bindings.push((param.name.clone(), Bound::V2(value)));
    }
    let result = run(env, &function.body, &mut scope, depth)?;
    match &function.result_type {
        Some(declared) => convert(
            result,
            declared,
            &format!("the result of {}", function.name),
        ),
        None => Ok(result),
    }
}

/// Runs a sequence constructor; its `xsl:variable`s are visible to the instructions after them.
fn run<'n, N: Node<'n>>(
    env: &Env<'_, N>,
    instructions: &[Instruction],
    scope: &mut Scope<'_, N>,
    depth: usize,
) -> Result<Sequence<N>, VersionedError> {
    let mut out = Vec::new();
    for instruction in instructions {
        match instruction {
            Instruction::Variable {
                name,
                declared,
                value,
            } => {
                let mut value = match value {
                    VariableValue::Select(select) => evaluate(env, select, scope, depth)?,
                    VariableValue::Content(body) => {
                        run(env, body, &mut Scope::new(Some(scope)), depth)?
                    }
                    // Not a document node holding a text node, which XSLT would build here:
                    // its string-value, as the one kind of value every use in the rule sets
                    // (`translate`, `string-length`, a comparison) reads it as.
                    VariableValue::Text(text) => {
                        vec![Item::Atomic(Atomic::Untyped(text.clone()))]
                    }
                };
                if let Some(declared) = declared {
                    value = convert(value, declared, &format!("the variable ${name}"))?;
                }
                scope.bindings.push((name.clone(), Bound::V2(value)));
            }
            Instruction::Sequence { select } => out.extend(evaluate(env, select, scope, depth)?),
            Instruction::ValueOf { select, separator } => {
                let value = evaluate(env, select, scope, depth)?;
                let parts: Vec<String> = atomize(&value)
                    .iter()
                    .map(Atomic::to_xpath_string)
                    .collect();
                out.push(Item::Atomic(Atomic::String(
                    parts.join(separator.as_deref().unwrap_or(" ")),
                )));
            }
            Instruction::Text(text) => out.push(Item::Atomic(Atomic::String(text.clone()))),
            Instruction::If { test, body } => {
                if test_of(env, test, scope, depth)? {
                    out.extend(run(env, body, &mut Scope::new(Some(scope)), depth)?);
                }
            }
            Instruction::Choose { whens, otherwise } => {
                let mut chosen = None;
                for (test, body) in whens {
                    if test_of(env, test, scope, depth)? {
                        chosen = Some(body);
                        break;
                    }
                }
                if let Some(body) = chosen.or(otherwise.as_ref()) {
                    out.extend(run(env, body, &mut Scope::new(Some(scope)), depth)?);
                }
            }
        }
    }
    Ok(out)
}

fn evaluate<'n, N: Node<'n>>(
    env: &Env<'_, N>,
    source: &str,
    scope: &Scope<'_, N>,
    depth: usize,
) -> Result<Sequence<N>, VersionedError> {
    let expression = env.exprs2.get_versioned(source)?;
    run_in_context(env.root, Some(depth), scope, env, |ctx| {
        xpath_eval::evaluate_versioned(&expression, ctx)
    })
}

fn test_of<'n, N: Node<'n>>(
    env: &Env<'_, N>,
    source: &str,
    scope: &Scope<'_, N>,
    depth: usize,
) -> Result<bool, VersionedError> {
    let expression = env.exprs2.get_versioned(source)?;
    run_in_context(env.root, Some(depth), scope, env, |ctx| {
        xpath_eval::evaluate_boolean_versioned(&expression, ctx)
    })
}

/// The function conversion rules (XPath 2.0 section 3.1.5.2) for a declared type: atomization,
/// `xs:untypedAtomic` cast to the expected atomic type, numeric promotion
/// (`xs:integer` to `xs:decimal` to `xs:double`), then the occurrence check.
fn convert<'n, N: Node<'n>>(
    value: Sequence<N>,
    declared: &SequenceType,
    what: &str,
) -> Result<Sequence<N>, VersionedError> {
    let type_error = |message: String| VersionedError::Dynamic {
        code: "XPTY0004",
        message: format!("{what} (declared `{}`): {message}", declared.source),
    };
    let converted: Sequence<N> = match declared.item {
        ItemType::Item => value,
        ItemType::Node => {
            if value.iter().any(|item| matches!(item, Item::Atomic(_))) {
                return Err(type_error(
                    "an atomic value where a node is required".into(),
                ));
            }
            value
        }
        ItemType::Atomic(expected) => {
            let mut out = Vec::with_capacity(value.len());
            for atom in atomize(&value) {
                out.push(Item::Atomic(promote(atom, expected).map_err(|got| {
                    type_error(format!(
                        "a value of type {got} where {expected:?} is required"
                    ))
                })??));
            }
            out
        }
        ItemType::Other => {
            return Err(VersionedError::Unsupported(format!(
                "the declared type {} of {what}",
                declared.source
            )));
        }
    };
    let count = converted.len();
    let fits = match declared.occurrence {
        Occurrence::One => count == 1,
        Occurrence::ZeroOrOne => count <= 1,
        Occurrence::ZeroOrMore => true,
        Occurrence::OneOrMore => count >= 1,
    };
    if !fits {
        return Err(type_error(format!("{count} items")));
    }
    Ok(converted)
}

fn atomize<'n, N: Node<'n>>(value: &[Item<N>]) -> Vec<Atomic> {
    value
        .iter()
        .map(|item| match item {
            Item::Node(node) => Atomic::Untyped(node.string_value()),
            Item::Atomic(atom) => atom.clone(),
        })
        .collect()
}

/// `Err(type name)` if `atom` cannot be used where `expected` is required; `Ok(Err(..))` if the
/// cast of an untyped value fails.
fn promote(
    atom: Atomic,
    expected: AtomicType,
) -> Result<Result<Atomic, VersionedError>, &'static str> {
    Ok(match (&atom, expected) {
        (Atomic::Untyped(_), _) => atom.cast(expected),
        (Atomic::String(_), AtomicType::String)
        | (Atomic::Boolean(_), AtomicType::Boolean)
        | (Atomic::Integer(_), AtomicType::Integer)
        | (Atomic::Decimal(_), AtomicType::Decimal)
        | (Atomic::Double(_), AtomicType::Double)
        | (Atomic::Date(_), AtomicType::Date)
        | (_, AtomicType::UntypedAtomic) => Ok(atom),
        (Atomic::Integer(_), AtomicType::Decimal | AtomicType::Double)
        | (Atomic::Decimal(_), AtomicType::Double) => atom.cast(expected),
        _ => return Err(atom.type_name()),
    })
}
