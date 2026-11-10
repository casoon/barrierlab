//! The function library of the XPath 2.0 mode: the `fn:` functions (XQuery 1.0
//! and XPath 2.0 Functions and Operators) and the `xs:` constructor functions
//! that the target rule sets use. Functions that exist in F&O but are not
//! implemented here fail with [`Error::Unsupported`]; a name that is no F&O
//! function at all is the static error `XPST0017`.

use super::atomic::{Arithmetic, Atomic, AtomicType, arithmetic, compare_values};
use super::decimal::Decimal;
use super::eval::{Env, FN_NAMESPACE, XS_NAMESPACE, atomize, effective_boolean_value};
use super::{Error, Item, Sequence};
use crate::ast::QName;
use crate::document::Node;
use std::cmp::Ordering;

/// Standard functions of F&O 1.0 that this crate does not implement yet.
/// `matches`, `replace` and `tokenize` wait for a regular-expression engine.
const KNOWN_UNIMPLEMENTED: &[&str] = &[
    "matches",
    "replace",
    "tokenize",
    "round-half-to-even",
    "string-to-codepoints",
    "codepoints-to-string",
    "compare",
    "codepoint-equal",
    "normalize-unicode",
    "encode-for-uri",
    "iri-to-uri",
    "escape-html-uri",
    "resolve-uri",
    "base-uri",
    "document-uri",
    "static-base-uri",
    "default-collation",
    "lang",
    "id",
    "idref",
    "root",
    "doc",
    "doc-available",
    "collection",
    "current-date",
    "current-dateTime",
    "current-time",
    "dateTime",
    "implicit-timezone",
    "one-or-more",
    "zero-or-one",
    "exactly-one",
    "insert-before",
    "remove",
    "subsequence",
    "index-of",
    "unordered",
    "deep-equal",
    "error",
    "trace",
    "nilled",
    "node-name",
    "resolve-QName",
    "QName",
    "namespace-uri-for-prefix",
    "in-scope-prefixes",
    "adjust-date-to-timezone",
];

/// Component and timezone functions of the date/time and duration types.
fn is_date_time_function(name: &str) -> bool {
    name.contains("-from-") || name.ends_with("-timezone")
}

pub(super) fn call<'e, 'ctx, 'hook, N: Node<'ctx>>(
    env: &Env<'e, 'ctx, 'hook, N>,
    name: &QName,
    args: Vec<Sequence<N>>,
) -> Result<Sequence<N>, Error> {
    let namespace = match &name.prefix {
        None => FN_NAMESPACE.to_owned(),
        Some(prefix) => env.resolve_prefix(prefix)?,
    };
    let shown = match &name.prefix {
        Some(p) => format!("{p}:{}", name.local),
        None => name.local.clone(),
    };
    match namespace.as_str() {
        XS_NAMESPACE => constructor(&name.local, &shown, args),
        FN_NAMESPACE => library(env, &name.local, &shown, args),
        _ => Err(Error::static_error(
            "XPST0017",
            format!("no function `{shown}` in that namespace"),
        )),
    }
}

/// `xs:T(arg)` (XPath 2.0 §3.10.4): the empty sequence stays empty, otherwise
/// a cast of the single atomized item.
fn constructor<'ctx, N: Node<'ctx>>(
    local: &str,
    shown: &str,
    mut args: Vec<Sequence<N>>,
) -> Result<Sequence<N>, Error> {
    let Some(ty) = AtomicType::from_local_name(local) else {
        return Err(match local {
            "float" | "dateTime" | "time" | "duration" | "dayTimeDuration"
            | "yearMonthDuration" | "gYear" | "gYearMonth" | "gMonth" | "gMonthDay" | "gDay"
            | "anyURI" | "QName" | "NCName" | "token" | "normalizedString" | "long" | "int"
            | "short" | "byte" | "nonNegativeInteger" | "positiveInteger" | "base64Binary"
            | "hexBinary" => Error::Unsupported(format!("the constructor function {shown}")),
            _ => Error::static_error("XPST0017", format!("no constructor function {shown}")),
        });
    };
    if args.len() != 1 {
        return Err(arity(shown, args.len()));
    }
    let atoms = atomize(&args.remove(0));
    match atoms.as_slice() {
        [] => Ok(Vec::new()),
        [single] => Ok(vec![Item::Atomic(single.cast(ty)?)]),
        _ => Err(Error::dynamic(
            "XPTY0004",
            format!("{shown} needs a single item"),
        )),
    }
}

fn arity(shown: &str, got: usize) -> Error {
    Error::static_error(
        "XPST0017",
        format!("no function {shown} with {got} argument(s)"),
    )
}

fn atom<N>(a: Atomic) -> Sequence<N> {
    vec![Item::Atomic(a)]
}

fn string_seq<N>(s: String) -> Sequence<N> {
    atom(Atomic::String(s))
}

fn boolean_seq<N>(b: bool) -> Sequence<N> {
    atom(Atomic::Boolean(b))
}

fn integer_seq<N>(n: usize) -> Sequence<N> {
    atom(Atomic::Integer(n as i64))
}

/// An argument declared `xs:string?`: untyped values are accepted, other
/// types are a type error (XPath 2.0 §3.1.5.2, function conversion rules).
fn opt_string<'ctx, N: Node<'ctx>>(
    arg: &[Item<N>],
    function: &str,
) -> Result<Option<String>, Error> {
    let mut atoms = atomize(arg);
    if atoms.len() > 1 {
        return Err(Error::dynamic(
            "XPTY0004",
            format!(
                "{function}: a single string is expected, got {} items",
                atoms.len()
            ),
        ));
    }
    match atoms.pop() {
        None => Ok(None),
        Some(Atomic::String(s) | Atomic::Untyped(s)) => Ok(Some(s)),
        Some(other) => Err(Error::dynamic(
            "XPTY0004",
            format!("{function}: expected xs:string, got {}", other.type_name()),
        )),
    }
}

fn string_or_empty<'ctx, N: Node<'ctx>>(arg: &[Item<N>], function: &str) -> Result<String, Error> {
    Ok(opt_string(arg, function)?.unwrap_or_default())
}

/// An argument declared `numeric?`: untyped values become `xs:double`.
fn opt_numeric<'ctx, N: Node<'ctx>>(
    arg: &[Item<N>],
    function: &str,
) -> Result<Option<Atomic>, Error> {
    let mut atoms = atomize(arg);
    if atoms.len() > 1 {
        return Err(Error::dynamic(
            "XPTY0004",
            format!(
                "{function}: a single number is expected, got {} items",
                atoms.len()
            ),
        ));
    }
    match atoms.pop() {
        None => Ok(None),
        Some(a @ Atomic::Untyped(_)) => Ok(Some(a.cast(AtomicType::Double)?)),
        Some(a) if a.is_numeric() => Ok(Some(a)),
        Some(other) => Err(Error::dynamic(
            "XPTY0004",
            format!("{function}: expected a number, got {}", other.type_name()),
        )),
    }
}

fn to_f64(a: &Atomic) -> f64 {
    match a {
        Atomic::Integer(i) => *i as f64,
        Atomic::Decimal(d) => d.to_f64(),
        Atomic::Double(d) => *d,
        _ => f64::NAN,
    }
}

fn opt_node<'ctx, N: Node<'ctx>>(arg: &[Item<N>], function: &str) -> Result<Option<N>, Error> {
    match arg {
        [] => Ok(None),
        [Item::Node(n)] => Ok(Some(*n)),
        _ => Err(Error::dynamic(
            "XPTY0004",
            format!("{function}: a single node is expected"),
        )),
    }
}

/// F&O `fn:round`: ties go toward positive infinity.
fn round_double(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let floor = x.floor();
    if x - floor >= 0.5 { floor + 1.0 } else { floor }
}

fn collapse_whitespace(s: &str) -> String {
    s.split([' ', '\t', '\n', '\r'])
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The numeric single-argument functions: the argument's type is kept
/// (F&O §6.4: `fn:abs`, `fn:ceiling`, `fn:floor`, `fn:round`).
fn numeric_function<N>(
    value: Option<Atomic>,
    integer: impl Fn(i64) -> Option<i64>,
    decimal: impl Fn(Decimal) -> Decimal,
    double: impl Fn(f64) -> f64,
) -> Result<Sequence<N>, Error> {
    Ok(match value {
        None => Vec::new(),
        Some(Atomic::Integer(i)) => {
            atom(Atomic::Integer(integer(i).ok_or_else(|| {
                Error::dynamic("FOAR0002", "numeric overflow")
            })?))
        }
        Some(Atomic::Decimal(d)) => atom(Atomic::Decimal(decimal(d))),
        Some(Atomic::Double(d)) => atom(Atomic::Double(double(d))),
        Some(_) => unreachable!("opt_numeric returns numbers"),
    })
}

/// Items of `arg` as numbers for `sum`, `avg`, `min`, `max`: untyped becomes
/// `xs:double` (F&O §15.4).
fn aggregate_operands<'ctx, N: Node<'ctx>>(arg: &[Item<N>]) -> Result<Vec<Atomic>, Error> {
    atomize(arg)
        .into_iter()
        .map(|a| match a {
            Atomic::Untyped(_) => a.cast(AtomicType::Double),
            other => Ok(other),
        })
        .collect()
}

fn sum_of(operands: Vec<Atomic>, function: &str) -> Result<Option<Atomic>, Error> {
    let mut iter = operands.into_iter();
    let Some(first) = iter.next() else {
        return Ok(None);
    };
    if !first.is_numeric() {
        return Err(Error::dynamic(
            "FORG0006",
            format!("{function}: expected numbers, got {}", first.type_name()),
        ));
    }
    let mut total = first;
    for next in iter {
        total = arithmetic(&total, Arithmetic::Add, &next)?;
    }
    Ok(Some(total))
}

/// Equality for `fn:distinct-values` (F&O §15.1.5): values of different type
/// families are distinct, NaN equals NaN, no error is raised.
fn distinct_equal(a: &Atomic, b: &Atomic) -> bool {
    match (a, b) {
        (Atomic::Double(x), Atomic::Double(y)) if x.is_nan() && y.is_nan() => true,
        (x, y) => matches!(compare_values(x, y), Ok(Some(Ordering::Equal))),
    }
}

fn library<'e, 'ctx, 'hook, N: Node<'ctx>>(
    env: &Env<'e, 'ctx, 'hook, N>,
    local: &str,
    shown: &str,
    mut args: Vec<Sequence<N>>,
) -> Result<Sequence<N>, Error> {
    let n = args.len();
    // Zero-argument forms default to the context item.
    let arg_or_context = |args: &mut Vec<Sequence<N>>| -> Result<Sequence<N>, Error> {
        if args.is_empty() {
            Ok(vec![env.context_item()?.clone()])
        } else {
            Ok(args.remove(0))
        }
    };
    match (local, n) {
        ("true", 0) => Ok(boolean_seq(true)),
        ("false", 0) => Ok(boolean_seq(false)),
        ("not", 1) => Ok(boolean_seq(!effective_boolean_value(&args[0])?)),
        ("boolean", 1) => Ok(boolean_seq(effective_boolean_value(&args[0])?)),
        ("count", 1) => Ok(integer_seq(args[0].len())),
        ("exists", 1) => Ok(boolean_seq(!args[0].is_empty())),
        ("empty", 1) => Ok(boolean_seq(args[0].is_empty())),
        ("position", 0) | ("last", 0) => {
            let focus = env
                .focus
                .as_ref()
                .ok_or_else(|| Error::dynamic("XPDY0002", "the context item is absent"))?;
            Ok(integer_seq(if local == "position" {
                focus.position
            } else {
                focus.size
            }))
        }
        ("data", 1) => Ok(atomize(&args[0]).into_iter().map(Item::Atomic).collect()),
        ("reverse", 1) => {
            let mut items = args.remove(0);
            items.reverse();
            Ok(items)
        }
        ("distinct-values", 1) => {
            let mut distinct: Vec<Atomic> = Vec::new();
            for a in atomize(&args[0]) {
                if !distinct.iter().any(|d| distinct_equal(d, &a)) {
                    distinct.push(a);
                }
            }
            Ok(distinct.into_iter().map(Item::Atomic).collect())
        }

        // ---- strings (F&O §7) -------------------------------------------
        ("string", 0 | 1) => {
            let arg = arg_or_context(&mut args)?;
            match arg.as_slice() {
                [] => Ok(string_seq(String::new())),
                [Item::Node(node)] => Ok(string_seq(node.string_value())),
                [Item::Atomic(a)] => Ok(string_seq(a.to_xpath_string())),
                _ => Err(Error::dynamic("XPTY0004", "string() needs a single item")),
            }
        }
        ("concat", 2..) => {
            let mut out = String::new();
            for arg in &args {
                match atomize(arg).as_slice() {
                    [] => {}
                    [single] => out.push_str(&single.to_xpath_string()),
                    _ => {
                        return Err(Error::dynamic(
                            "XPTY0004",
                            "concat() takes single items as arguments",
                        ));
                    }
                }
            }
            Ok(string_seq(out))
        }
        ("string-join", 2) => {
            let separator = string_or_empty(&args[1], "string-join")?;
            let mut parts = Vec::new();
            for a in atomize(&args[0]) {
                match a {
                    Atomic::String(s) | Atomic::Untyped(s) => parts.push(s),
                    other => {
                        return Err(Error::dynamic(
                            "XPTY0004",
                            format!("string-join: expected xs:string, got {}", other.type_name()),
                        ));
                    }
                }
            }
            Ok(string_seq(parts.join(&separator)))
        }
        ("string-length", 0 | 1) => {
            let arg = arg_or_context(&mut args)?;
            Ok(integer_seq(
                string_or_empty(&arg, "string-length")?.chars().count(),
            ))
        }
        ("normalize-space", 0 | 1) => {
            let arg = arg_or_context(&mut args)?;
            Ok(string_seq(collapse_whitespace(&string_or_empty(
                &arg,
                "normalize-space",
            )?)))
        }
        ("upper-case", 1) => Ok(string_seq(
            string_or_empty(&args[0], "upper-case")?.to_uppercase(),
        )),
        ("lower-case", 1) => Ok(string_seq(
            string_or_empty(&args[0], "lower-case")?.to_lowercase(),
        )),
        ("contains", 2) | ("starts-with", 2) | ("ends-with", 2) => {
            let haystack = string_or_empty(&args[0], local)?;
            let needle = string_or_empty(&args[1], local)?;
            Ok(boolean_seq(match local {
                "contains" => haystack.contains(&needle),
                "starts-with" => haystack.starts_with(&needle),
                _ => haystack.ends_with(&needle),
            }))
        }
        ("substring-before", 2) => {
            let haystack = string_or_empty(&args[0], local)?;
            let needle = string_or_empty(&args[1], local)?;
            Ok(string_seq(match haystack.find(&needle) {
                Some(i) if !needle.is_empty() => haystack[..i].to_owned(),
                _ => String::new(),
            }))
        }
        ("substring-after", 2) => {
            let haystack = string_or_empty(&args[0], local)?;
            let needle = string_or_empty(&args[1], local)?;
            Ok(string_seq(match haystack.find(&needle) {
                Some(i) => haystack[i + needle.len()..].to_owned(),
                None => String::new(),
            }))
        }
        ("substring", 2 | 3) => {
            let source = string_or_empty(&args[0], local)?;
            let start = opt_numeric(&args[1], local)?.map_or(f64::NAN, |a| to_f64(&a));
            let start = round_double(start);
            let end = if n == 3 {
                let length = opt_numeric(&args[2], local)?.map_or(f64::NAN, |a| to_f64(&a));
                start + round_double(length)
            } else {
                f64::INFINITY
            };
            Ok(string_seq(
                source
                    .chars()
                    .enumerate()
                    .filter(|(i, _)| {
                        let position = (*i + 1) as f64;
                        position >= start && position < end
                    })
                    .map(|(_, c)| c)
                    .collect(),
            ))
        }
        ("translate", 3) => {
            let source = string_or_empty(&args[0], local)?;
            let from: Vec<char> = string_or_empty(&args[1], local)?.chars().collect();
            let to: Vec<char> = string_or_empty(&args[2], local)?.chars().collect();
            Ok(string_seq(
                source
                    .chars()
                    .filter_map(|c| match from.iter().position(|f| *f == c) {
                        None => Some(c),
                        Some(i) => to.get(i).copied(),
                    })
                    .collect(),
            ))
        }

        // ---- numbers (F&O §6.4, §15.4) ----------------------------------
        ("number", 0 | 1) => {
            let arg = arg_or_context(&mut args)?;
            let value = match atomize(&arg).as_slice() {
                [single] => match single.cast(AtomicType::Double) {
                    Ok(Atomic::Double(d)) => d,
                    Err(e @ Error::Unsupported(_)) => return Err(e),
                    _ => f64::NAN,
                },
                _ => f64::NAN,
            };
            Ok(atom(Atomic::Double(value)))
        }
        ("abs", 1) => numeric_function(
            opt_numeric(&args[0], local)?,
            i64::checked_abs,
            Decimal::abs,
            f64::abs,
        ),
        ("ceiling", 1) => numeric_function(
            opt_numeric(&args[0], local)?,
            Some,
            Decimal::ceiling,
            f64::ceil,
        ),
        ("floor", 1) => numeric_function(
            opt_numeric(&args[0], local)?,
            Some,
            Decimal::floor,
            f64::floor,
        ),
        ("round", 1) => numeric_function(
            opt_numeric(&args[0], local)?,
            Some,
            Decimal::round,
            round_double,
        ),
        ("sum", 1 | 2) => {
            let operands = aggregate_operands(&args[0])?;
            Ok(match sum_of(operands, local)? {
                Some(total) => atom(total),
                None if n == 2 => args.remove(1),
                None => atom(Atomic::Integer(0)),
            })
        }
        ("avg", 1) => {
            let operands = aggregate_operands(&args[0])?;
            let count = operands.len() as i64;
            Ok(match sum_of(operands, local)? {
                Some(total) => atom(arithmetic(
                    &total,
                    Arithmetic::Div,
                    &Atomic::Integer(count),
                )?),
                None => Vec::new(),
            })
        }
        ("min", 1) | ("max", 1) => {
            let wanted = if local == "min" {
                Ordering::Less
            } else {
                Ordering::Greater
            };
            let mut best: Option<Atomic> = None;
            // Operands of mixed numeric types are compared and returned as the
            // promoted type (F&O §15.4: `fn:max` and `fn:min`).
            let mut rank = 0;
            for candidate in aggregate_operands(&args[0])? {
                rank = rank.max(match candidate {
                    Atomic::Decimal(_) => 1,
                    Atomic::Double(_) => 2,
                    _ => 0,
                });
                best = Some(match best {
                    None => candidate,
                    Some(current) => {
                        let nan = |a: &Atomic| matches!(a, Atomic::Double(d) if d.is_nan());
                        if nan(&current) || nan(&candidate) {
                            Atomic::Double(f64::NAN)
                        } else if compare_values(&candidate, &current)? == Some(wanted) {
                            candidate
                        } else {
                            current
                        }
                    }
                });
            }
            if let Some(value) = &best
                && value.is_numeric()
            {
                let promoted = [AtomicType::Integer, AtomicType::Decimal, AtomicType::Double][rank];
                best = Some(value.cast(promoted)?);
            }
            Ok(best.map(atom).unwrap_or_default())
        }

        // ---- names (F&O §14) --------------------------------------------
        ("name" | "local-name" | "namespace-uri", 0 | 1) => {
            let arg = arg_or_context(&mut args)?;
            let text = match opt_node(&arg, local)? {
                None => String::new(),
                Some(node) => match local {
                    "name" => crate::functions::qname_string(node),
                    "local-name" => node
                        .expanded_name()
                        .map(|e| e.local_name)
                        .unwrap_or_default(),
                    _ => node
                        .expanded_name()
                        .and_then(|e| e.namespace_uri)
                        .unwrap_or_default(),
                },
            };
            Ok(string_seq(text))
        }

        _ if KNOWN_UNIMPLEMENTED.contains(&local) || is_date_time_function(local) => {
            Err(Error::Unsupported(format!("the function fn:{local}")))
        }
        _ => Err(arity(shown, n)),
    }
}
