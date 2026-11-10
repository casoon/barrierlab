//! The XPath 2.0 expression tree (grammar productions of XPath 2.0 §A.1).
//! Node tests and axes are the 1.0 types; `//` is desugared into
//! `descendant-or-self::node()` steps during parsing, as in the 1.0 AST.

use super::atomic::{Arithmetic, Atomic, AtomicType, Comparison};
use crate::ast::{Axis, NodeTest, QName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    Union,
    Intersect,
    Except,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeComparison {
    Is,
    Precedes,
    Follows,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisStep {
    pub axis: Axis,
    pub test: NodeTest,
    pub predicates: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// §3.1.1 literal.
    Literal(Atomic),
    /// §3.1.2 `$name`.
    Variable(QName),
    /// §3.1.4 `.`
    ContextItem,
    /// §3.3 `a, b` (also `()` as the empty vector).
    Sequence(Vec<Expr>),
    /// §3.3.1 `a to b`.
    Range(Box<Expr>, Box<Expr>),
    /// §3.8 `if (c) then a else b`.
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// §3.7 `for $v in e (, $w in f)* return r`.
    For(Vec<(QName, Expr)>, Box<Expr>),
    /// §3.9 `some|every $v in e (, ...)* satisfies c`; `true` is `every`.
    Quantified(bool, Vec<(QName, Expr)>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    /// §3.5.2 `=`, `!=`, `<`, ...
    General(Box<Expr>, Comparison, Box<Expr>),
    /// §3.5.1 `eq`, `ne`, `lt`, ...
    Value(Box<Expr>, Comparison, Box<Expr>),
    /// §3.5.3 `is`, `<<`, `>>`.
    Node(Box<Expr>, NodeComparison, Box<Expr>),
    /// §3.4 binary arithmetic.
    Arithmetic(Box<Expr>, Arithmetic, Box<Expr>),
    /// §3.4 unary `-` (`true`) and `+` (`false`).
    Unary(bool, Box<Expr>),
    /// §3.3.3 `union`/`|`, `intersect`, `except`.
    SetOperation(Box<Expr>, SetOp, Box<Expr>),
    /// §3.10.2/§3.10.3 `cast as T?` and `castable as T?`: `(expr, type, optional, castable)`.
    Cast(Box<Expr>, AtomicType, bool, bool),
    /// §3.2 `/`: the root of the tree containing the context node.
    Root,
    /// §3.2 `a/b`: `b` is evaluated once per node of `a`.
    Path(Box<Expr>, Box<Expr>),
    /// §3.2.1 axis step.
    Step(AxisStep),
    /// §3.2.1 `E[p]...`.
    Filter(Box<Expr>, Vec<Expr>),
    /// §3.1.5 function call.
    Call(QName, Vec<Expr>),
}
