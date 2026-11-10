//! The XPath 2.0 mode. XPath 1.0 stays the default and is untouched: this
//! module has its own parser, AST and evaluator (a 2.0 expression is not a 1.0
//! expression with more operators — comparisons, arithmetic and the value
//! model differ), and only shares the axes, the `Node`/`Document` traits and
//! the namespace/variable hooks of [`EvaluationContext`].
//!
//! Normative: W3C XPath 2.0 (Second Edition) and XQuery 1.0 and XPath 2.0
//! Functions and Operators. Supported are the constructs listed in
//! `docs/packages/xpath-eval.md`; everything else fails with
//! [`Error::Unsupported`] naming the feature, never with a silent result.

mod ast;
mod atomic;
mod decimal;
mod eval;
mod functions;
mod parser;

use crate::document::Node;
use crate::eval::{EvalError, EvaluationContext};
use crate::parser::ParseError;
use crate::value::Value;

pub use atomic::{Atomic, AtomicType, Date};
pub use decimal::Decimal;

/// Which XPath language an expression is parsed and evaluated in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum XPathVersion {
    /// XPath 1.0 (W3C Recommendation, 1999): the default and the reference
    /// behaviour of this crate.
    #[default]
    V1,
    /// XPath 2.0 (W3C Recommendation, 2010), the subset in `docs/packages/xpath-eval.md`.
    V2,
}

impl XPathVersion {
    /// Maps a Schematron `queryBinding` to the XPath version, the way ISO
    /// Schematron defines the binding names: `xslt`/`xpath` and an absent
    /// binding are XPath 1.0 (`schematron-engine` already accepts exactly
    /// these), `xslt2`/`xpath2` are XPath 2.0. Anything else is `None`.
    pub fn from_query_binding(binding: Option<&str>) -> Option<XPathVersion> {
        match binding.map(str::to_ascii_lowercase).as_deref() {
            None | Some("xslt") | Some("xpath") => Some(XPathVersion::V1),
            Some("xslt2") | Some("xpath2") => Some(XPathVersion::V2),
            Some(_) => None,
        }
    }
}

/// An item of an XPath 2.0 sequence (XPath 2.0 §2.1, data model): a node or
/// an atomic value.
#[derive(Debug, Clone, PartialEq)]
pub enum Item<N> {
    Node(N),
    Atomic(Atomic),
}

/// A sequence of items. There are no nested sequences; the empty sequence is
/// the empty vector.
pub type Sequence<N> = Vec<Item<N>>;

/// Why an expression could not be parsed or evaluated in a given
/// [`XPathVersion`].
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Syntax error.
    Parse(ParseError),
    /// The expression uses a language feature or function that this crate does
    /// not implement. The caller should report the check as "not run" with
    /// this reason, not as passed or failed.
    Unsupported(String),
    /// A static error (XPath 2.0 §2.3.1) found while evaluating: unknown
    /// function (`XPST0017`), unbound variable (`XPST0008`), unbound namespace
    /// prefix (`XPST0081`).
    Static { code: &'static str, message: String },
    /// A dynamic error (XPath 2.0 §2.3.2): type error (`XPTY0004`), invalid
    /// cast (`FORG0001`), division by zero (`FOAR0001`), and so on.
    Dynamic { code: &'static str, message: String },
    /// An error of the XPath 1.0 evaluator (only in [`XPathVersion::V1`]).
    Eval(EvalError),
}

impl Error {
    pub(crate) fn dynamic(code: &'static str, message: impl Into<String>) -> Error {
        Error::Dynamic {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn static_error(code: &'static str, message: impl Into<String>) -> Error {
        Error::Static {
            code,
            message: message.into(),
        }
    }

    /// `true` for [`Error::Unsupported`]: the expression is valid XPath 2.0
    /// that this crate does not evaluate yet.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Error::Unsupported(_))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Parse(e) => e.fmt(f),
            Error::Unsupported(what) => write!(f, "unsupported XPath 2.0 feature: {what}"),
            Error::Static { code, message } | Error::Dynamic { code, message } => {
                write!(f, "{code}: {message}")
            }
            Error::Eval(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// A parsed expression together with the language it was parsed in.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression(Inner);

#[derive(Debug, Clone, PartialEq)]
enum Inner {
    V1(crate::ast::Expr),
    V2(ast::Expr),
}

impl Expression {
    pub fn version(&self) -> XPathVersion {
        match self.0 {
            Inner::V1(_) => XPathVersion::V1,
            Inner::V2(_) => XPathVersion::V2,
        }
    }
}

/// Parses `source` as XPath 1.0 (`XPathVersion::V1`, same as [`crate::parse`])
/// or as XPath 2.0.
pub fn parse_versioned(source: &str, version: XPathVersion) -> Result<Expression, Error> {
    Ok(Expression(match version {
        XPathVersion::V1 => Inner::V1(crate::parser::parse(source).map_err(Error::Parse)?),
        XPathVersion::V2 => Inner::V2(parser::parse(source)?),
    }))
}

/// Evaluates `expression` to a sequence. A 1.0 expression yields the
/// 1.0 value as a sequence of one item (a node-set as its nodes in the order
/// 1.0 returned them, a number as `xs:double`).
pub fn evaluate_versioned<'ctx, 'hook, N: Node<'ctx>>(
    expression: &Expression,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Sequence<N>, Error> {
    match &expression.0 {
        Inner::V1(expr) => {
            let value = crate::eval::evaluate(expr, ctx).map_err(Error::Eval)?;
            Ok(from_value(value))
        }
        Inner::V2(expr) => eval::evaluate(expr, ctx),
    }
}

/// Evaluates `expression` to a boolean: `boolean()` for 1.0, the effective
/// boolean value (XPath 2.0 §2.4.3) for 2.0.
pub fn evaluate_boolean_versioned<'ctx, 'hook, N: Node<'ctx>>(
    expression: &Expression,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<bool, Error> {
    match &expression.0 {
        Inner::V1(expr) => crate::eval::evaluate_boolean(expr, ctx).map_err(Error::Eval),
        Inner::V2(expr) => eval::effective_boolean_value(&eval::evaluate(expr, ctx)?),
    }
}

pub(crate) fn from_value<N>(value: Value<N>) -> Sequence<N> {
    match value {
        Value::NodeSet(nodes) => nodes.into_iter().map(Item::Node).collect(),
        Value::Boolean(b) => vec![Item::Atomic(Atomic::Boolean(b))],
        Value::Number(n) => vec![Item::Atomic(Atomic::Double(n))],
        Value::String(s) => vec![Item::Atomic(Atomic::String(s))],
    }
}
