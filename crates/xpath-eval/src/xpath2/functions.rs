//! The function library of the XPath 2.0 mode: the `fn:` functions (XQuery 1.0
//! and XPath 2.0 Functions and Operators) and the `xs:` constructor functions
//! that the target rule sets use. Functions that exist in F&O but are not
//! implemented here fail with [`VersionedError::Unsupported`]; a name that is no F&O
//! function at all is the static error `XPST0017`.

use super::atomic::{Arithmetic, Atomic, AtomicType, arithmetic, compare_values};
use super::decimal::Decimal;
use super::eval::{Env, FN_NAMESPACE, XS_NAMESPACE, atomize, effective_boolean_value};
use super::{Item, Sequence, VersionedError};
use crate::ast::QName;
use crate::document::Node;
use std::cmp::Ordering;

/// Every function F&O 1.0 defines in `fn:` and XSLT 2.0 defines for XPath
/// (name, fewest and most arguments). A call that matches an entry but is not
/// implemented here is `Unsupported`; a name or arity in neither is `XPST0017`.
/// An explicit table, not a guess from the name.
const KNOWN_FUNCTIONS: &[(&str, usize, usize)] = &[
    // F&O 1.0 §2 accessors, §3 errors and tracing
    ("node-name", 1, 1),
    ("nilled", 1, 1),
    ("string", 0, 1),
    ("data", 1, 1),
    ("base-uri", 0, 1),
    ("document-uri", 1, 1),
    ("error", 0, 3),
    ("trace", 2, 2),
    // §6 numeric
    ("number", 0, 1),
    ("abs", 1, 1),
    ("ceiling", 1, 1),
    ("floor", 1, 1),
    ("round", 1, 1),
    ("round-half-to-even", 1, 2),
    // §7 strings
    ("codepoints-to-string", 1, 1),
    ("string-to-codepoints", 1, 1),
    ("compare", 2, 3),
    ("codepoint-equal", 2, 2),
    ("concat", 2, usize::MAX),
    ("string-join", 2, 2),
    ("substring", 2, 3),
    ("string-length", 0, 1),
    ("normalize-space", 0, 1),
    ("normalize-unicode", 1, 2),
    ("upper-case", 1, 1),
    ("lower-case", 1, 1),
    ("translate", 3, 3),
    ("encode-for-uri", 1, 1),
    ("iri-to-uri", 1, 1),
    ("escape-html-uri", 1, 1),
    ("contains", 2, 3),
    ("starts-with", 2, 3),
    ("ends-with", 2, 3),
    ("substring-before", 2, 3),
    ("substring-after", 2, 3),
    ("matches", 2, 3),
    ("replace", 3, 4),
    ("tokenize", 2, 3),
    // §8 boolean
    ("true", 0, 0),
    ("false", 0, 0),
    ("not", 1, 1),
    ("boolean", 1, 1),
    // §10 date, time and duration
    ("years-from-duration", 1, 1),
    ("months-from-duration", 1, 1),
    ("days-from-duration", 1, 1),
    ("hours-from-duration", 1, 1),
    ("minutes-from-duration", 1, 1),
    ("seconds-from-duration", 1, 1),
    ("dateTime", 2, 2),
    ("year-from-dateTime", 1, 1),
    ("month-from-dateTime", 1, 1),
    ("day-from-dateTime", 1, 1),
    ("hours-from-dateTime", 1, 1),
    ("minutes-from-dateTime", 1, 1),
    ("seconds-from-dateTime", 1, 1),
    ("timezone-from-dateTime", 1, 1),
    ("year-from-date", 1, 1),
    ("month-from-date", 1, 1),
    ("day-from-date", 1, 1),
    ("timezone-from-date", 1, 1),
    ("hours-from-time", 1, 1),
    ("minutes-from-time", 1, 1),
    ("seconds-from-time", 1, 1),
    ("timezone-from-time", 1, 1),
    ("adjust-dateTime-to-timezone", 1, 2),
    ("adjust-date-to-timezone", 1, 2),
    ("adjust-time-to-timezone", 1, 2),
    ("current-dateTime", 0, 0),
    ("current-date", 0, 0),
    ("current-time", 0, 0),
    ("implicit-timezone", 0, 0),
    // §11 QNames
    ("resolve-QName", 2, 2),
    ("QName", 2, 2),
    ("prefix-from-QName", 1, 1),
    ("local-name-from-QName", 1, 1),
    ("namespace-uri-from-QName", 1, 1),
    ("namespace-uri-for-prefix", 2, 2),
    ("in-scope-prefixes", 1, 1),
    // §14 nodes
    ("name", 0, 1),
    ("local-name", 0, 1),
    ("namespace-uri", 0, 1),
    ("lang", 1, 2),
    ("root", 0, 1),
    // §15 sequences
    ("index-of", 2, 3),
    ("empty", 1, 1),
    ("exists", 1, 1),
    ("distinct-values", 1, 2),
    ("insert-before", 3, 3),
    ("remove", 2, 2),
    ("reverse", 1, 1),
    ("subsequence", 2, 3),
    ("unordered", 1, 1),
    ("zero-or-one", 1, 1),
    ("one-or-more", 1, 1),
    ("exactly-one", 1, 1),
    ("deep-equal", 2, 3),
    ("count", 1, 1),
    ("avg", 1, 1),
    ("max", 1, 2),
    ("min", 1, 2),
    ("sum", 1, 2),
    ("id", 1, 2),
    ("idref", 1, 2),
    ("doc", 1, 1),
    ("doc-available", 1, 1),
    ("collection", 0, 1),
    ("position", 0, 0),
    ("last", 0, 0),
    ("static-base-uri", 0, 0),
    ("default-collation", 0, 0),
    ("resolve-uri", 1, 2),
    // XSLT 2.0 §16 additional functions
    ("current", 0, 0),
    ("document", 1, 2),
    ("key", 2, 3),
    ("format-number", 2, 3),
    ("format-dateTime", 2, 5),
    ("format-date", 2, 5),
    ("format-time", 2, 5),
    ("generate-id", 0, 1),
    ("system-property", 1, 1),
    ("unparsed-entity-uri", 1, 2),
    ("unparsed-entity-public-id", 1, 2),
    ("unparsed-text", 1, 2),
    ("unparsed-text-available", 1, 2),
    ("function-available", 1, 2),
    ("element-available", 1, 1),
    ("type-available", 1, 1),
    ("regex-group", 1, 1),
    ("current-group", 0, 0),
    ("current-grouping-key", 0, 0),
];

pub(super) fn call<'e, 'ctx, 'hook, N: Node<'ctx>>(
    env: &Env<'e, 'ctx, 'hook, N>,
    name: &QName,
    args: Vec<Sequence<N>>,
) -> Result<Sequence<N>, VersionedError> {
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
        _ => Err(VersionedError::static_error(
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
) -> Result<Sequence<N>, VersionedError> {
    let Some(ty) = AtomicType::from_local_name(local) else {
        return Err(match local {
            "float" | "dateTime" | "time" | "duration" | "dayTimeDuration"
            | "yearMonthDuration" | "gYear" | "gYearMonth" | "gMonth" | "gMonthDay" | "gDay"
            | "anyURI" | "QName" | "NCName" | "token" | "normalizedString" | "long" | "int"
            | "short" | "byte" | "nonNegativeInteger" | "positiveInteger" | "base64Binary"
            | "hexBinary" => {
                VersionedError::Unsupported(format!("the constructor function {shown}"))
            }
            _ => {
                VersionedError::static_error("XPST0017", format!("no constructor function {shown}"))
            }
        });
    };
    if args.len() != 1 {
        return Err(arity(shown, args.len()));
    }
    let atoms = atomize(&args.remove(0));
    match atoms.as_slice() {
        [] => Ok(Vec::new()),
        [single] => Ok(vec![Item::Atomic(single.cast(ty)?)]),
        _ => Err(VersionedError::dynamic(
            "XPTY0004",
            format!("{shown} needs a single item"),
        )),
    }
}

fn arity(shown: &str, got: usize) -> VersionedError {
    VersionedError::static_error(
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
) -> Result<Option<String>, VersionedError> {
    let mut atoms = atomize(arg);
    if atoms.len() > 1 {
        return Err(VersionedError::dynamic(
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
        Some(other) => Err(VersionedError::dynamic(
            "XPTY0004",
            format!("{function}: expected xs:string, got {}", other.type_name()),
        )),
    }
}

fn string_or_empty<'ctx, N: Node<'ctx>>(
    arg: &[Item<N>],
    function: &str,
) -> Result<String, VersionedError> {
    Ok(opt_string(arg, function)?.unwrap_or_default())
}

/// An argument declared `numeric?`: untyped values become `xs:double`.
fn opt_numeric<'ctx, N: Node<'ctx>>(
    arg: &[Item<N>],
    function: &str,
) -> Result<Option<Atomic>, VersionedError> {
    let mut atoms = atomize(arg);
    if atoms.len() > 1 {
        return Err(VersionedError::dynamic(
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
        Some(other) => Err(VersionedError::dynamic(
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

fn opt_node<'ctx, N: Node<'ctx>>(
    arg: &[Item<N>],
    function: &str,
) -> Result<Option<N>, VersionedError> {
    match arg {
        [] => Ok(None),
        [Item::Node(n)] => Ok(Some(*n)),
        _ => Err(VersionedError::dynamic(
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
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    // F&O fn:round: a negative argument that rounds to zero gives negative zero.
    if rounded == 0.0 && x < 0.0 {
        -0.0
    } else {
        rounded
    }
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
) -> Result<Sequence<N>, VersionedError> {
    Ok(match value {
        None => Vec::new(),
        Some(Atomic::Integer(i)) => {
            atom(Atomic::Integer(integer(i).ok_or_else(|| {
                VersionedError::dynamic("FOAR0002", "numeric overflow")
            })?))
        }
        Some(Atomic::Decimal(d)) => atom(Atomic::Decimal(decimal(d))),
        Some(Atomic::Double(d)) => atom(Atomic::Double(double(d))),
        Some(_) => unreachable!("opt_numeric returns numbers"),
    })
}

/// Items of `arg` as numbers for `sum`, `avg`, `min`, `max`: untyped becomes
/// `xs:double` (F&O §15.4).
fn aggregate_operands<'ctx, N: Node<'ctx>>(arg: &[Item<N>]) -> Result<Vec<Atomic>, VersionedError> {
    atomize(arg)
        .into_iter()
        .map(|a| match a {
            Atomic::Untyped(_) => a.cast(AtomicType::Double),
            other => Ok(other),
        })
        .collect()
}

fn sum_of(operands: Vec<Atomic>, function: &str) -> Result<Option<Atomic>, VersionedError> {
    let mut iter = operands.into_iter();
    let Some(first) = iter.next() else {
        return Ok(None);
    };
    if !first.is_numeric() {
        return Err(VersionedError::dynamic(
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
) -> Result<Sequence<N>, VersionedError> {
    let n = args.len();
    // Zero-argument forms default to the context item.
    let arg_or_context = |args: &mut Vec<Sequence<N>>| -> Result<Sequence<N>, VersionedError> {
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
                .ok_or_else(|| VersionedError::dynamic("XPDY0002", "the context item is absent"))?;
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
                _ => Err(VersionedError::dynamic(
                    "XPTY0004",
                    "string() needs a single item",
                )),
            }
        }
        ("concat", 2..) => {
            let mut out = String::new();
            for arg in &args {
                match atomize(arg).as_slice() {
                    [] => {}
                    [single] => out.push_str(&single.to_xpath_string()),
                    _ => {
                        return Err(VersionedError::dynamic(
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
                        return Err(VersionedError::dynamic(
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
                    Err(e @ VersionedError::Unsupported(_)) => return Err(e),
                    _ => f64::NAN,
                },
                [] => f64::NAN,
                _ => {
                    return Err(VersionedError::dynamic(
                        "XPTY0004",
                        "number() needs a single item",
                    ));
                }
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

        _ if KNOWN_FUNCTIONS
            .iter()
            .any(|&(name, min, max)| name == local && (min..=max).contains(&n)) =>
        {
            Err(VersionedError::Unsupported(format!(
                "the function {shown} with {n} argument(s)"
            )))
        }
        _ => Err(arity(shown, n)),
    }
}
