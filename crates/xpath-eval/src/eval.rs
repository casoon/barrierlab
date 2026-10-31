//! Evaluates an `ast::Expr` against a `Document`/`Node<'a>` (§2-§3): the
//! value model and conversions live in `value.rs`; this module adds the
//! `EvaluationContext`, the operator semantics (§3.3/§3.4), and the full
//! location-path/predicate evaluation (§2.2-§2.4).
//!
//! `last()`/`position()`/`count()` are implemented as functions directly
//! here; the rest of the core function library (§4, including `id()`) is
//! implemented in `functions.rs` and dispatched to from
//! `evaluate_function` below (Phase 05).

use crate::ast::{
    AdditiveOp, Axis, EqualityOp, Expr, FilterExpr, FunctionCall, LocationPath, MultiplicativeOp,
    NodeTest, PathExpr, PrimaryExpr, QName, RelationalOp, Step,
};
use crate::axes;
use crate::cache::{AttributeIndex, DocumentCache, TestKey};
use crate::document::{Node, NodeKind};
use crate::functions;
use crate::value::{Value, string_to_number};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::ControlFlow;
use std::rc::Rc;

/// A variable-binding lookup hook: given a variable's `QName`, returns its
/// bound value, if any.
pub type VariableLookup<'a, N> = dyn Fn(&QName) -> Option<Value<N>> + 'a;

/// A namespace-prefix resolution hook: given a name test's raw prefix
/// string, returns the namespace URI it's bound to, if any. `None` means
/// the prefix is unresolvable via this hook — a prefixed name test then
/// never matches (see `resolve_prefix`/`ResolvedTest`).
pub type NamespaceLookup<'a> = dyn Fn(&str) -> Option<String> + 'a;

/// The context an `Expr` is evaluated against (§2.2/§2.4): the context node,
/// its 1-based proximity position and the context size within whatever
/// node-set it was drawn from, a variable-binding lookup hook, and a
/// namespace-prefix resolution hook.
///
/// `None` in `variables` means "no variables are bound" — every
/// `PrimaryExpr::Variable` lookup then fails with `EvalError::UnboundVariable`.
/// Nothing in this phase binds variables yet (that is a `schematron-engine`
/// `<let>` concern), but `PrimaryExpr::Variable` still needs this real,
/// clearly-erroring path rather than a panic or a silently-wrong default.
///
/// `None` in `namespaces` means "no namespace bindings are declared" —
/// prefixed name tests then never match any node (see `resolve_prefix`'s
/// doc comment for the full rationale).
///
/// `'ctx` is the node/document borrow lifetime (`N: Node<'ctx>`); `'hook` is
/// an independent lifetime for the `variables`/`namespaces` hook references.
/// The two are deliberately decoupled (see `plan/05b-decouple-hook-lifetime.md`):
/// a hook built locally within a single `evaluate()` call (e.g. a closure
/// over a temporary namespace-binding table) does not need to live as long
/// as the document itself, and there is no `'hook: 'ctx` (or reverse) bound
/// between them — nothing here ever stores the hook and the node together
/// in a way that would require one.
///
/// `_ctx` is a zero-sized marker: `'ctx` is only used in the `N: Node<'ctx>`
/// bound above, not in any field type on its own, and Rust requires every
/// declared lifetime parameter of a struct to appear in a field (`E0392`
/// otherwise) — this `PhantomData` satisfies that without adding any real
/// state.
#[derive(Clone, Copy)]
pub struct EvaluationContext<'ctx, 'hook, N: Node<'ctx>> {
    pub node: N,
    pub position: usize,
    pub size: usize,
    pub variables: Option<&'hook VariableLookup<'hook, N>>,
    pub namespaces: Option<&'hook NamespaceLookup<'hook>>,
    /// Per-document memo (see [`DocumentCache`]). `None` evaluates
    /// everything from scratch; the result is the same either way.
    pub cache: Option<&'hook DocumentCache<N>>,
    /// Prefixes already resolved through `namespaces` during the current
    /// `evaluate()` call; set by [`evaluate`] itself.
    prefixes: Option<&'hook PrefixMemo>,
    _ctx: std::marker::PhantomData<&'ctx ()>,
}

/// Prefix → namespace URI, as the namespace hook answered it, so one
/// `evaluate()` call asks the hook once per prefix instead of once per
/// name test it evaluates (barrierlab#69).
#[derive(Default)]
struct PrefixMemo(RefCell<Vec<(String, Option<Rc<str>>)>>);

impl<'ctx, 'hook, N: Node<'ctx>> EvaluationContext<'ctx, 'hook, N> {
    /// A context whose node is both the sole member and the first member of
    /// its own (size-1) context node-set, with no variable bindings and no
    /// namespace context.
    pub fn new(node: N) -> Self {
        EvaluationContext {
            node,
            position: 1,
            size: 1,
            variables: None,
            namespaces: None,
            cache: None,
            prefixes: None,
            _ctx: std::marker::PhantomData,
        }
    }

    /// Same variable-binding and namespace-resolution hooks, different
    /// context node/position/size — used when descending into a predicate
    /// or a nested path step.
    fn with(&self, node: N, position: usize, size: usize) -> Self {
        EvaluationContext {
            node,
            position,
            size,
            variables: self.variables,
            namespaces: self.namespaces,
            cache: self.cache,
            prefixes: self.prefixes,
            _ctx: std::marker::PhantomData,
        }
    }

    /// Resolves `prefix` to a namespace URI (see `resolve_prefix`),
    /// through the memo when there is one.
    fn resolve(&self, prefix: &str) -> Option<Rc<str>> {
        let Some(memo) = self.prefixes else {
            return resolve_prefix(prefix, self.namespaces).map(Rc::from);
        };
        if let Some((_, uri)) = memo.0.borrow().iter().find(|(p, _)| p == prefix) {
            return uri.clone();
        }
        let uri: Option<Rc<str>> = resolve_prefix(prefix, self.namespaces).map(Rc::from);
        memo.0.borrow_mut().push((prefix.to_owned(), uri.clone()));
        uri
    }
}

/// An evaluation-time error. Distinct from `ParseError` (`parser.rs`), which
/// covers syntax — this covers semantic failures once a well-formed `Expr`
/// is evaluated against a concrete context.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// `PrimaryExpr::Function` named something other than one of the 27
    /// implemented core-library functions (`last`/`position`/`count`,
    /// Phase 04, plus the 24 more in `functions.rs`, Phase 05 — including
    /// `id()`, see its doc comment there).
    UnknownFunction(QName),
    /// `PrimaryExpr::Variable` has no binding via the context's lookup hook.
    UnboundVariable(QName),
    /// A function called with the wrong number of arguments.
    ArgumentCount {
        function: &'static str,
        expected: usize,
        got: usize,
    },
    /// `count()`'s argument, or a `Union`/`FilterExpr`-predicate operand,
    /// did not evaluate to a node-set.
    ExpectedNodeSet { context: &'static str },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UnknownFunction(name) => {
                write!(f, "unknown function: {}", format_qname(name))
            }
            EvalError::UnboundVariable(name) => {
                write!(f, "unbound variable: ${}", format_qname(name))
            }
            EvalError::ArgumentCount {
                function,
                expected,
                got,
            } => write!(f, "{function}() expects {expected} argument(s), got {got}"),
            EvalError::ExpectedNodeSet { context } => {
                write!(f, "expected a node-set in {context}")
            }
        }
    }
}

impl std::error::Error for EvalError {}

fn format_qname(q: &QName) -> String {
    match &q.prefix {
        Some(p) => format!("{p}:{}", q.local),
        None => q.local.clone(),
    }
}

/// Evaluates `expr` against `ctx`, producing an XPath 1.0 value.
pub fn evaluate<'ctx, 'hook, N: Node<'ctx>>(
    expr: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    with_prefix_memo(ctx, |ctx| evaluate_expr(expr, ctx))
}

/// Evaluates `expr` converted to a boolean, as `boolean(expr)` would
/// (§4.3). Same answer as `evaluate(expr, ctx)?.to_boolean()`, but a
/// location path stops at its first node instead of collecting all of
/// them (barrierlab#69). Like the `and`/`or` short-circuit, that can skip
/// an error evaluating the remaining nodes would have raised.
pub fn evaluate_boolean<'ctx, 'hook, N: Node<'ctx>>(
    expr: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<bool, EvalError> {
    with_prefix_memo(ctx, |ctx| evaluate_truth(expr, ctx))
}

/// Runs `f` with a prefix memo in place for the duration of one public
/// evaluation call.
fn with_prefix_memo<'ctx, 'hook, N: Node<'ctx>, R>(
    ctx: &EvaluationContext<'ctx, 'hook, N>,
    f: impl FnOnce(&EvaluationContext<'ctx, '_, N>) -> R,
) -> R {
    if ctx.prefixes.is_some() {
        return f(ctx);
    }
    let memo = PrefixMemo::default();
    let ctx = EvaluationContext {
        node: ctx.node,
        position: ctx.position,
        size: ctx.size,
        variables: ctx.variables,
        namespaces: ctx.namespaces,
        cache: ctx.cache,
        prefixes: Some(&memo),
        _ctx: std::marker::PhantomData,
    };
    f(&ctx)
}

fn evaluate_expr<'ctx, 'hook, N: Node<'ctx>>(
    expr: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    match expr {
        // §3.3: "Or"/"And" convert both operands to boolean. The spec does
        // not mandate an evaluation order, and expressions here have no
        // side effects other than errors — short-circuiting is a
        // deliberate, documented choice (not evaluating the right operand
        // can hide an error it would have raised).
        Expr::Or(..) | Expr::And(..) => Ok(Value::Boolean(evaluate_truth(expr, ctx)?)),
        Expr::Equality(l, op, r) => {
            if let Some(result) = attribute_literal_comparison(l, *op, r, ctx) {
                return Ok(Value::Boolean(result));
            }
            let lv = evaluate(l, ctx)?;
            let rv = evaluate(r, ctx)?;
            Ok(Value::Boolean(compare_equality(&lv, *op, &rv)))
        }
        Expr::Relational(l, op, r) => {
            let lv = evaluate(l, ctx)?;
            let rv = evaluate(r, ctx)?;
            Ok(Value::Boolean(compare_relational(&lv, *op, &rv)))
        }
        Expr::Additive(l, op, r) => {
            let lv = evaluate(l, ctx)?.to_number();
            let rv = evaluate(r, ctx)?.to_number();
            Ok(Value::Number(match op {
                AdditiveOp::Add => lv + rv,
                AdditiveOp::Sub => lv - rv,
            }))
        }
        Expr::Multiplicative(l, op, r) => {
            let lv = evaluate(l, ctx)?.to_number();
            let rv = evaluate(r, ctx)?.to_number();
            Ok(Value::Number(match op {
                MultiplicativeOp::Mul => lv * rv,
                // Rust's `f64` `/` and `%` already implement IEEE-754
                // division and remainder, which is exactly what XPath's
                // `div`/`mod` specify.
                MultiplicativeOp::Div => lv / rv,
                MultiplicativeOp::Mod => lv % rv,
            }))
        }
        Expr::Negate(e) => Ok(Value::Number(-evaluate(e, ctx)?.to_number())),
        Expr::Union(l, r) => {
            let lv = evaluate(l, ctx)?;
            let rv = evaluate(r, ctx)?;
            match (lv, rv) {
                (Value::NodeSet(mut a), Value::NodeSet(b)) => {
                    a.extend(b);
                    sort_dedup(&mut a);
                    Ok(Value::NodeSet(a))
                }
                _ => Err(EvalError::ExpectedNodeSet {
                    context: "union operand",
                }),
            }
        }
        Expr::Path(path) => evaluate_path(path, ctx),
    }
}

/// Evaluates `expr` converted to a boolean (§4.3 `boolean()`), without
/// building node-sets whose only use is whether they are empty: a location
/// path stops at its first node (barrierlab#69). Like the `and`/`or`
/// short-circuit above, stopping early can skip an error that evaluating
/// the remaining nodes would have raised.
pub(crate) fn evaluate_truth<'ctx, 'hook, N: Node<'ctx>>(
    expr: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<bool, EvalError> {
    match expr {
        Expr::Or(l, r) => Ok(evaluate_truth(l, ctx)? || evaluate_truth(r, ctx)?),
        Expr::And(l, r) => Ok(evaluate_truth(l, ctx)? && evaluate_truth(r, ctx)?),
        Expr::Path(PathExpr::Location(loc)) => location_path_exists(loc, ctx),
        Expr::Path(PathExpr::Filter(FilterExpr {
            primary,
            predicates,
        })) if predicates.is_empty() => match primary {
            PrimaryExpr::Parenthesized(inner) => evaluate_truth(inner, ctx),
            PrimaryExpr::Function(call)
                if call.name.prefix.is_none()
                    && call.args.len() == 1
                    && matches!(call.name.local.as_str(), "not" | "boolean") =>
            {
                let truth = evaluate_truth(&call.args[0], ctx)?;
                Ok(if call.name.local == "not" {
                    !truth
                } else {
                    truth
                })
            }
            _ => Ok(evaluate(expr, ctx)?.to_boolean()),
        },
        _ => Ok(evaluate(expr, ctx)?.to_boolean()),
    }
}

/// Whether `loc` selects at least one node. All steps but the last are
/// evaluated as usual; the last stops at its first node that passes the
/// node test and every predicate, provided no predicate depends on
/// proximity position.
fn location_path_exists<'ctx, 'hook, N: Node<'ctx>>(
    loc: &LocationPath,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<bool, EvalError> {
    let Some((last, init)) = loc.steps.split_last() else {
        return Ok(true);
    };
    // Keep `//X` (`descendant-or-self::node()/child::X`) together, as
    // `evaluate_location_steps` does.
    let (axis, init) = match init.split_last() {
        Some((before, rest))
            if is_plain_descendant_or_self(before)
                && last.axis == Axis::Child
                && last.predicates.iter().all(is_position_independent) =>
        {
            (Axis::Descendant, rest)
        }
        _ => (last.axis, init),
    };
    if !last.predicates.iter().all(is_position_independent) {
        return Ok(!evaluate_location_path(loc, vec![ctx.node], ctx)?.is_empty());
    }
    let start = if loc.is_absolute {
        axes::root_of(ctx.node)
    } else {
        ctx.node
    };
    let evaluated;
    let nodes: &[N] = if init.is_empty() {
        std::slice::from_ref(&start)
    } else {
        evaluated = evaluate_location_steps(init, vec![start], ctx)?;
        &evaluated
    };
    if axis == Axis::Descendant
        && ctx.cache.is_some()
        && let [n] = nodes
        && n.kind() == NodeKind::Root
    {
        // The cached list and attribute index answer this faster.
        let found = evaluate_step(axis, &last.node_test, &last.predicates, nodes, ctx)?;
        return Ok(!found.is_empty());
    }
    let test = ResolvedTest::new(&last.node_test, axis, ctx);
    if matches!(test, ResolvedTest::Never) {
        return Ok(false);
    }
    if let Some(found) = indexed_ancestor_exists(axis, &test, &last.predicates, nodes, ctx) {
        return Ok(found);
    }
    for &n in nodes {
        let found = try_find_on_axis(axis, n, |c| {
            if !test.matches(c) {
                return Ok(false);
            }
            let candidate = ctx.with(c, 1, 1);
            for predicate in &last.predicates {
                if !evaluate_truth(predicate, &candidate)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })?;
        if found {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `ancestor::X[F]` / `ancestor-or-self::X[F]` with `F` an
/// [`AttributeFilter`], answered from the cached index: the elements of the
/// whole document that pass `X` and `F` are looked up once, and each
/// ancestor is checked for membership instead of evaluating `F` on it
/// (barrierlab#69). `None` where that does not apply (no cache, another
/// axis or predicate shape, or the index cannot answer `F`); the caller
/// then walks the axis as usual.
fn indexed_ancestor_exists<'ctx, 'hook, N: Node<'ctx>>(
    axis: Axis,
    test: &ResolvedTest<'_>,
    predicates: &[Expr],
    nodes: &[N],
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Option<bool> {
    if !matches!(axis, Axis::Ancestor | Axis::AncestorOrSelf) {
        return None;
    }
    let cache = ctx.cache?;
    let [predicate] = predicates else {
        return None;
    };
    let (filter, None) = AttributeFilter::split(predicate)? else {
        return None;
    };
    let first = *nodes.first()?;
    let key = test.key()?;
    let list = cache.descendants(axes::root_of(first), &key, |n| test.matches(n))?;
    if list.is_empty() {
        return Some(false);
    }
    let positions = filter.positions(cache, &key, &list, ctx)?;
    let passes = |a: N| {
        test.matches(a)
            && list
                .binary_search_by(|x| x.document_order(a))
                .is_ok_and(|p| positions.binary_search(&(p as u32)).is_ok())
    };
    Some(nodes.iter().any(|&n| match axis {
        Axis::Ancestor => axes::ancestor(n).any(passes),
        _ => axes::ancestor_or_self(n).any(passes),
    }))
}

/// Whether some node on `axis` from `n` passes `f`, stopping at the first.
fn try_find_on_axis<'a, N: Node<'a>>(
    axis: Axis,
    n: N,
    mut f: impl FnMut(N) -> Result<bool, EvalError>,
) -> Result<bool, EvalError> {
    let mut check = |c: N| match f(c) {
        Ok(false) => ControlFlow::Continue(()),
        Ok(true) => ControlFlow::Break(Ok(true)),
        Err(e) => ControlFlow::Break(Err(e)),
    };
    let flow = match axis {
        Axis::Ancestor => axes::ancestor(n).try_for_each(&mut check),
        Axis::AncestorOrSelf => axes::ancestor_or_self(n).try_for_each(&mut check),
        Axis::Attribute => axes::attribute(n).try_for_each(&mut check),
        Axis::Child => axes::child(n).try_for_each(&mut check),
        Axis::Descendant => axes::descendant(n).try_for_each(&mut check),
        Axis::DescendantOrSelf => axes::descendant_or_self(n).try_for_each(&mut check),
        Axis::Following => axes::following(n).try_for_each(&mut check),
        Axis::FollowingSibling => axes::following_sibling(n).try_for_each(&mut check),
        Axis::Namespace => axes::namespace(n).try_for_each(&mut check),
        Axis::Parent => axes::parent(n).try_for_each(&mut check),
        Axis::Preceding => axes::preceding(n).try_for_each(&mut check),
        Axis::PrecedingSibling => axes::preceding_sibling(n).try_for_each(&mut check),
        Axis::SelfAxis => axes::self_axis(n).try_for_each(&mut check),
    };
    match flow {
        ControlFlow::Continue(()) => Ok(false),
        ControlFlow::Break(result) => result,
    }
}

/// `@name = 'literal'` or `@name != 'literal'` (either way round), compared
/// against the context node's attributes directly instead of building the
/// node-set and a string value first (barrierlab#69). Same answer as the
/// general node-set/string rule of §3.4.
fn attribute_literal_comparison<'ctx, 'hook, N: Node<'ctx>>(
    l: &Expr,
    op: EqualityOp,
    r: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Option<bool> {
    let (test, literal) = match (attribute_name(l), string_literal(r)) {
        (Some(test), Some(literal)) => (test, literal),
        _ => (attribute_name(r)?, string_literal(l)?),
    };
    let test = ResolvedTest::new(test, Axis::Attribute, ctx);
    Some(
        axes::attribute(ctx.node)
            .filter(|&a| test.matches(a))
            .any(|a| a.with_string_value(|value| eq_result(op, value, literal))),
    )
}

/// The text of `e` if it is a bare string literal.
fn string_literal(e: &Expr) -> Option<&str> {
    match e {
        Expr::Path(PathExpr::Filter(FilterExpr {
            primary: PrimaryExpr::Literal(s),
            predicates,
        })) if predicates.is_empty() => Some(s),
        _ => None,
    }
}

// ---- Equality / Relational (§3.3/§3.4) ---------------------------------

fn eq_result<T: PartialEq>(op: EqualityOp, l: T, r: T) -> bool {
    match op {
        EqualityOp::Eq => l == r,
        EqualityOp::Ne => l != r,
    }
}

fn rel_result(op: RelationalOp, l: f64, r: f64) -> bool {
    match op {
        RelationalOp::Lt => l < r,
        RelationalOp::Le => l <= r,
        RelationalOp::Gt => l > r,
        RelationalOp::Ge => l >= r,
    }
}

/// §3.4, node-set comparison rules for `=`/`!=` (all four cases quoted
/// verbatim in the plan), falling back to the "no node-set operand" rule
/// (boolean, else number, else string) otherwise.
fn compare_equality<'a, N: Node<'a>>(lhs: &Value<N>, op: EqualityOp, rhs: &Value<N>) -> bool {
    match (lhs, rhs) {
        (Value::NodeSet(a), Value::NodeSet(b)) => {
            let b: Vec<String> = b.iter().map(|nb| nb.string_value()).collect();
            a.iter().any(|na| {
                na.with_string_value(|va| b.iter().any(|vb| eq_result(op, va, vb.as_str())))
            })
        }
        (Value::NodeSet(a), Value::Number(n)) | (Value::Number(n), Value::NodeSet(a)) => a
            .iter()
            .any(|na| na.with_string_value(|va| eq_result(op, string_to_number(va), *n))),
        (Value::NodeSet(a), Value::String(s)) | (Value::String(s), Value::NodeSet(a)) => a
            .iter()
            .any(|na| na.with_string_value(|va| eq_result(op, va, s.as_str()))),
        (Value::NodeSet(a), Value::Boolean(b)) | (Value::Boolean(b), Value::NodeSet(a)) => {
            eq_result(op, !a.is_empty(), *b)
        }
        _ => {
            // §3.4, "neither object to be compared is a node-set": boolean
            // if either side is boolean, else number if either side is
            // number, else string.
            if matches!(lhs, Value::Boolean(_)) || matches!(rhs, Value::Boolean(_)) {
                eq_result(op, lhs.to_boolean(), rhs.to_boolean())
            } else if matches!(lhs, Value::Number(_)) || matches!(rhs, Value::Number(_)) {
                eq_result(op, lhs.to_number(), rhs.to_number())
            } else {
                eq_result(op, lhs.to_xpath_string(), rhs.to_xpath_string())
            }
        }
    }
}

/// §3.4, relational operators. When neither operand is a node-set, always
/// convert both to number (never string/boolean comparison). When a
/// node-set is involved, the same four node-set-vs-X cases as equality
/// apply, but — per the plan's clarification of the spec text — the
/// underlying per-node "comparison" is always a numeric one: node-set vs
/// node-set compares the two nodes' string-values *converted to numbers*
/// (not compared as strings), and node-set vs string/boolean likewise
/// converts the non-node-set side to a number before comparing, since a
/// relational operator's result must in the end come from a numeric
/// comparison.
fn compare_relational<'a, N: Node<'a>>(lhs: &Value<N>, op: RelationalOp, rhs: &Value<N>) -> bool {
    match (lhs, rhs) {
        (Value::NodeSet(a), Value::NodeSet(b)) => {
            let b: Vec<f64> = b.iter().map(|&nb| node_number(nb)).collect();
            a.iter().any(|&na| {
                let va = node_number(na);
                b.iter().any(|&vb| rel_result(op, va, vb))
            })
        }
        (Value::NodeSet(a), Value::Number(n)) => {
            a.iter().any(|&na| rel_result(op, node_number(na), *n))
        }
        (Value::Number(n), Value::NodeSet(a)) => {
            a.iter().any(|&na| rel_result(op, *n, node_number(na)))
        }
        (Value::NodeSet(a), Value::String(s)) => {
            let sn = string_to_number(s);
            a.iter().any(|&na| rel_result(op, node_number(na), sn))
        }
        (Value::String(s), Value::NodeSet(a)) => {
            let sn = string_to_number(s);
            a.iter().any(|&na| rel_result(op, sn, node_number(na)))
        }
        (Value::NodeSet(a), Value::Boolean(b)) => {
            rel_result(op, bool_to_number(!a.is_empty()), bool_to_number(*b))
        }
        (Value::Boolean(b), Value::NodeSet(a)) => {
            rel_result(op, bool_to_number(*b), bool_to_number(!a.is_empty()))
        }
        _ => rel_result(op, lhs.to_number(), rhs.to_number()),
    }
}

/// A node's string-value converted to a number (§3.4's per-node comparison).
fn node_number<'a, N: Node<'a>>(n: N) -> f64 {
    n.with_string_value(string_to_number)
}

fn bool_to_number(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

// ---- Location paths / steps / predicates (§2.2-§2.4) -------------------

pub(crate) fn sort_dedup<'a, N: Node<'a>>(nodes: &mut Vec<N>) {
    nodes.sort_by(|a, b| a.document_order(*b));
    nodes.dedup();
}

/// Calls `f` for every node on `axis` from `n`, in axis order, without
/// collecting the axis first.
fn for_each_on_axis<'a, N: Node<'a>>(axis: Axis, n: N, f: impl FnMut(N)) {
    match axis {
        Axis::Ancestor => axes::ancestor(n).for_each(f),
        Axis::AncestorOrSelf => axes::ancestor_or_self(n).for_each(f),
        Axis::Attribute => axes::attribute(n).for_each(f),
        Axis::Child => axes::child(n).for_each(f),
        Axis::Descendant => axes::descendant(n).for_each(f),
        Axis::DescendantOrSelf => axes::descendant_or_self(n).for_each(f),
        Axis::Following => axes::following(n).for_each(f),
        Axis::FollowingSibling => axes::following_sibling(n).for_each(f),
        Axis::Namespace => axes::namespace(n).for_each(f),
        Axis::Parent => axes::parent(n).for_each(f),
        Axis::Preceding => axes::preceding(n).for_each(f),
        Axis::PrecedingSibling => axes::preceding_sibling(n).for_each(f),
        Axis::SelfAxis => axes::self_axis(n).for_each(f),
    }
}

/// The "principal node type" of an axis (§2.3): attribute nodes for the
/// attribute axis, namespace nodes for the namespace axis, elements for
/// every other axis. Only relevant to `NameTest` node tests (`*`,
/// `prefix:*`, `QName`) — node-type tests (`text()` etc.) match by kind
/// regardless of axis.
fn principal_node_kind(axis: Axis) -> NodeKind {
    match axis {
        Axis::Attribute => NodeKind::Attribute,
        Axis::Namespace => NodeKind::Namespace,
        _ => NodeKind::Element,
    }
}

/// Resolves `prefix` to a namespace URI via `namespaces` (Phase 04a).
/// `None` means the prefix cannot be resolved to any URI — either there is
/// no resolver hook at all, or one is present but doesn't know this
/// prefix. Both cases are treated identically: per spec an unbound
/// namespace prefix is invalid, and a `NodeTest` carrying it can never
/// match a real node's namespace URI. (Earlier, a missing hook fell back
/// to comparing the raw prefix string itself against `namespace_uri` —
/// correct only by coincidence, when a caller happened to use the URI
/// itself as the "prefix"; that fallback has been removed as a footgun for
/// callers who don't supply a resolver.)
fn resolve_prefix<'hook>(
    prefix: &str,
    namespaces: Option<&NamespaceLookup<'hook>>,
) -> Option<String> {
    namespaces.and_then(|lookup| lookup(prefix))
}

/// A `NodeTest` with its prefix already resolved, so a step resolves it
/// once instead of once per candidate node (barrierlab#69).
///
/// `NodeTest::QName`/`NamespaceWildcard` carry the expression's raw
/// `prefix` string, which only ever matches through a real, hook-resolved
/// namespace URI (see `resolve_prefix`) — never through the prefix string
/// itself. An unresolvable prefix becomes `Never`.
enum ResolvedTest<'t> {
    Node,
    Kind(NodeKind),
    ProcessingInstruction(&'t str),
    AnyName(NodeKind),
    NamespaceWildcard(NodeKind, Rc<str>),
    Name(NodeKind, Option<Rc<str>>, &'t str),
    Never,
}

impl<'t> ResolvedTest<'t> {
    fn new<'ctx, N: Node<'ctx>>(
        test: &'t NodeTest,
        axis: Axis,
        ctx: &EvaluationContext<'ctx, '_, N>,
    ) -> Self {
        let principal = principal_node_kind(axis);
        match test {
            NodeTest::Node => ResolvedTest::Node,
            NodeTest::Text => ResolvedTest::Kind(NodeKind::Text),
            NodeTest::Comment => ResolvedTest::Kind(NodeKind::Comment),
            NodeTest::ProcessingInstruction(None) => {
                ResolvedTest::Kind(NodeKind::ProcessingInstruction)
            }
            NodeTest::ProcessingInstruction(Some(target)) => {
                ResolvedTest::ProcessingInstruction(target)
            }
            // §5: element, attribute and namespace nodes always have an
            // expanded name, so the principal kind alone decides `*`.
            NodeTest::AnyName => ResolvedTest::AnyName(principal),
            NodeTest::NamespaceWildcard(prefix) => match ctx.resolve(prefix) {
                Some(uri) => ResolvedTest::NamespaceWildcard(principal, uri),
                None => ResolvedTest::Never,
            },
            NodeTest::QName(qname) => match &qname.prefix {
                None => ResolvedTest::Name(principal, None, &qname.local),
                Some(prefix) => match ctx.resolve(prefix) {
                    Some(uri) => ResolvedTest::Name(principal, Some(uri), &qname.local),
                    None => ResolvedTest::Never,
                },
            },
        }
    }

    /// The cache key for this test; `None` for `Never`, which needs no cache.
    fn key(&self) -> Option<TestKey> {
        Some(match self {
            ResolvedTest::Node => TestKey::Node,
            ResolvedTest::Kind(kind) | ResolvedTest::AnyName(kind) => TestKey::Kind(*kind),
            ResolvedTest::ProcessingInstruction(target) => {
                TestKey::ProcessingInstruction((*target).to_owned())
            }
            ResolvedTest::NamespaceWildcard(kind, uri) => {
                TestKey::NamespaceWildcard(*kind, uri.to_string())
            }
            ResolvedTest::Name(kind, uri, local) => TestKey::Name(
                *kind,
                uri.as_deref().map(str::to_owned),
                (*local).to_owned(),
            ),
            ResolvedTest::Never => return None,
        })
    }

    fn matches<'a, N: Node<'a>>(&self, n: N) -> bool {
        match self {
            ResolvedTest::Node => true,
            ResolvedTest::Kind(kind) | ResolvedTest::AnyName(kind) => n.kind() == *kind,
            ResolvedTest::ProcessingInstruction(target) => {
                n.kind() == NodeKind::ProcessingInstruction
                    && n.expanded_name().is_some_and(|e| e.local_name == *target)
            }
            ResolvedTest::NamespaceWildcard(kind, uri) => {
                n.kind() == *kind
                    && n.expanded_name()
                        .is_some_and(|e| e.namespace_uri.as_deref() == Some(&**uri))
            }
            ResolvedTest::Name(kind, uri, local) => {
                n.kind() == *kind && n.has_expanded_name(uri.as_deref(), local)
            }
            ResolvedTest::Never => false,
        }
    }
}

/// §2.4, verbatim: "A PredicateExpr is evaluated by evaluating the Expr and
/// converting the result to a boolean. If the result is a number, the
/// result will be converted to true if the number is equal to the context
/// position and will be converted to false otherwise; if the result is not
/// a number, then the result will be converted as if by a call to the
/// boolean() function."
fn predicate_truth<'a, N: Node<'a>>(val: &Value<N>, position: usize) -> bool {
    match val {
        Value::Number(n) => *n == position as f64,
        _ => val.to_boolean(),
    }
}

/// Applies each predicate in `predicates` to `nodes` **in sequence**: every
/// predicate's context size/position come from the node-set the *previous*
/// predicate filtered down to, not from the original, unfiltered input —
/// this is what makes `child::*[1][self::foo]` differ from
/// `child::*[self::foo][1]` (see `plan/04-evaluation-core.md`'s worked
/// example and the module-level test of the same shape).
fn apply_predicates<'ctx, 'hook, N: Node<'ctx>>(
    predicates: &[Expr],
    nodes: Vec<N>,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Vec<N>, EvalError> {
    let mut current = nodes;
    for pred in predicates {
        let numeric = may_be_number(pred);
        let size = current.len();
        let mut next = Vec::with_capacity(current.len());
        for (i, &n) in current.iter().enumerate() {
            let position = i + 1;
            let pred_ctx = ctx.with(n, position, size);
            let keep = if numeric {
                predicate_truth(&evaluate(pred, &pred_ctx)?, position)
            } else {
                evaluate_truth(pred, &pred_ctx)?
            };
            if keep {
                next.push(n);
            }
        }
        current = next;
    }
    Ok(current)
}

/// Evaluates one step (`axis::node_test[predicates]`) against each of
/// `context_nodes` individually — the axis is walked and node-test-filtered
/// per context node (so per-node proximity positions/predicates are
/// computed against that node's own axis order), and only the union of
/// all those per-node results is sorted into document order and
/// deduplicated at the end.
fn evaluate_step<'ctx, 'hook, N: Node<'ctx>>(
    axis: Axis,
    node_test: &NodeTest,
    predicates: &[Expr],
    context_nodes: &[N],
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Vec<N>, EvalError> {
    let test = ResolvedTest::new(node_test, axis, ctx);
    if matches!(test, ResolvedTest::Never) {
        return Ok(Vec::new());
    }
    if axis == Axis::Descendant
        && let [root] = context_nodes
        && root.kind() == NodeKind::Root
        && let Some(cache) = ctx.cache
        && let Some(key) = test.key()
        && let Some(list) = cache.descendants(*root, &key, |n| test.matches(n))
    {
        return evaluate_cached_descendants(cache, &key, &list, predicates, ctx);
    }
    let mut result = Vec::new();
    for &n in context_nodes {
        if predicates.is_empty() {
            for_each_on_axis(axis, n, |c| {
                if test.matches(c) {
                    result.push(c);
                }
            });
        } else {
            let mut candidates = Vec::new();
            for_each_on_axis(axis, n, |c| {
                if test.matches(c) {
                    candidates.push(c);
                }
            });
            result.extend(apply_predicates(predicates, candidates, ctx)?);
        }
    }
    sort_dedup(&mut result);
    Ok(result)
}

/// `descendant::X[p...]` from the root, over the cached list for `X`
/// (barrierlab#69). If the first predicate is an [`AttributeFilter`], the
/// candidates come from the cached attribute index instead of evaluating
/// that filter once per node; the rest of such a predicate (`F and R`) is
/// then evaluated, with the node's original position and size, only for
/// those candidates — exactly the nodes for which `and` would have
/// evaluated it. If the index cannot answer, the predicates are evaluated
/// the ordinary way, so errors surface exactly as without the cache. The
/// list is in document order and without duplicates, so no sorting is
/// needed.
fn evaluate_cached_descendants<'ctx, 'hook, N: Node<'ctx>>(
    cache: &DocumentCache<N>,
    key: &TestKey,
    list: &[N],
    predicates: &[Expr],
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Vec<N>, EvalError> {
    let Some((first, rest)) = predicates.split_first() else {
        return Ok(list.to_vec());
    };
    if list.is_empty() {
        return Ok(Vec::new());
    }
    if let Some((filter, residual)) = AttributeFilter::split(first)
        && let Some(positions) = filter.positions(cache, key, list, ctx)
    {
        let mut nodes = Vec::with_capacity(positions.len());
        for position in positions {
            let n = list[position as usize];
            let keep = match residual {
                None => true,
                Some(residual) => {
                    evaluate(residual, &ctx.with(n, position as usize + 1, list.len()))?
                        .to_boolean()
                }
            };
            if keep {
                nodes.push(n);
            }
        }
        return apply_predicates(rest, nodes, ctx);
    }
    apply_predicates(predicates, list.to_vec(), ctx)
}

/// A predicate the attribute index answers alone: `@name`, `@name = E`
/// (either way round, `E` independent of the context node, position and
/// size), and `and`/`or` combinations of them. §3.4: `@name = E` holds
/// exactly for the nodes with such an attribute whose string-value equals
/// one of `E`'s strings.
enum AttributeFilter<'e> {
    Has(&'e NodeTest),
    Equals(&'e NodeTest, &'e Expr),
    And(Box<AttributeFilter<'e>>, Box<AttributeFilter<'e>>),
    Or(Box<AttributeFilter<'e>>, Box<AttributeFilter<'e>>),
}

impl<'e> AttributeFilter<'e> {
    /// `predicate` as a filter, or as `filter and residual`.
    fn split(predicate: &'e Expr) -> Option<(Self, Option<&'e Expr>)> {
        if let Some(filter) = Self::of(predicate) {
            return Some((filter, None));
        }
        let Expr::And(l, r) = predicate else {
            return None;
        };
        Some((Self::of(l)?, Some(r)))
    }

    fn of(expr: &'e Expr) -> Option<Self> {
        match expr {
            Expr::And(l, r) => Some(Self::And(Box::new(Self::of(l)?), Box::new(Self::of(r)?))),
            Expr::Or(l, r) => Some(Self::Or(Box::new(Self::of(l)?), Box::new(Self::of(r)?))),
            Expr::Equality(l, EqualityOp::Eq, r) => match (attribute_name(l), attribute_name(r)) {
                (Some(test), _) if !depends_on_context(r) => Some(Self::Equals(test, r)),
                (_, Some(test)) if !depends_on_context(l) => Some(Self::Equals(test, l)),
                _ => None,
            },
            Expr::Path(PathExpr::Filter(FilterExpr {
                primary: PrimaryExpr::Parenthesized(inner),
                predicates,
            })) if predicates.is_empty() => Self::of(inner),
            _ => attribute_name(expr).map(Self::Has),
        }
    }

    /// Ascending positions in `list` of the nodes passing the filter, or
    /// `None` if the index cannot answer: an `E` that fails or yields a
    /// number or boolean (which compare differently), or a prefix that
    /// resolves to nothing.
    fn positions<'ctx, N: Node<'ctx>>(
        &self,
        cache: &DocumentCache<N>,
        key: &TestKey,
        list: &[N],
        ctx: &EvaluationContext<'ctx, '_, N>,
    ) -> Option<Vec<u32>> {
        match self {
            Self::Has(test) => Some(
                attribute_index(cache, key, list, test, ctx)?
                    .present
                    .clone(),
            ),
            Self::Equals(test, other) => {
                let index = attribute_index(cache, key, list, test, ctx)?;
                // Evaluated where the first candidate would have evaluated
                // it; its value does not depend on that choice.
                let values = match evaluate(other, &ctx.with(list[0], 1, list.len())).ok()? {
                    Value::String(s) => vec![s],
                    Value::NodeSet(nodes) => nodes.iter().map(|n| n.string_value()).collect(),
                    Value::Number(_) | Value::Boolean(_) => return None,
                };
                let mut positions: Vec<u32> = values
                    .iter()
                    .filter_map(|v| index.by_value.get(v.as_str()))
                    .flatten()
                    .copied()
                    .collect();
                if values.len() > 1 {
                    positions.sort_unstable();
                    positions.dedup();
                }
                Some(positions)
            }
            Self::And(l, r) => Some(intersect_sorted(
                &l.positions(cache, key, list, ctx)?,
                &r.positions(cache, key, list, ctx)?,
            )),
            Self::Or(l, r) => Some(union_sorted(
                &l.positions(cache, key, list, ctx)?,
                &r.positions(cache, key, list, ctx)?,
            )),
        }
    }
}

/// The cached index over `list` of the attributes named by `test`.
fn attribute_index<'ctx, N: Node<'ctx>>(
    cache: &DocumentCache<N>,
    key: &TestKey,
    list: &[N],
    test: &NodeTest,
    ctx: &EvaluationContext<'ctx, '_, N>,
) -> Option<Rc<AttributeIndex>> {
    let attribute = ResolvedTest::new(test, Axis::Attribute, ctx);
    let attribute_key = attribute.key()?;
    Some(cache.attribute_index(key, list, &attribute_key, |a| attribute.matches(a)))
}

fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

fn union_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Less => {
                out.push(a[i]);
                i += 1;
            }
            Ordering::Greater => {
                out.push(b[j]);
                j += 1;
            }
            Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

/// The name test of `e` if it is a bare `@QName`.
fn attribute_name(e: &Expr) -> Option<&NodeTest> {
    match e {
        Expr::Path(PathExpr::Location(LocationPath {
            is_absolute: false,
            steps,
        })) => match steps.as_slice() {
            [
                Step {
                    axis: Axis::Attribute,
                    node_test: test @ NodeTest::QName(_),
                    predicates,
                },
            ] if predicates.is_empty() => Some(test),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `expr` may depend on the context node, position or size.
/// Conservative: unknown functions count as dependent.
fn depends_on_context(expr: &Expr) -> bool {
    match expr {
        Expr::Or(l, r)
        | Expr::And(l, r)
        | Expr::Equality(l, _, r)
        | Expr::Relational(l, _, r)
        | Expr::Additive(l, _, r)
        | Expr::Multiplicative(l, _, r)
        | Expr::Union(l, r) => depends_on_context(l) || depends_on_context(r),
        Expr::Negate(e) => depends_on_context(e),
        Expr::Path(PathExpr::Location(loc)) => !loc.is_absolute,
        Expr::Path(PathExpr::Filter(filter) | PathExpr::FilterLocation(filter, _)) => {
            primary_depends_on_context(&filter.primary)
        }
    }
}

fn primary_depends_on_context(primary: &PrimaryExpr) -> bool {
    match primary {
        PrimaryExpr::Literal(_) | PrimaryExpr::Number(_) | PrimaryExpr::Variable(_) => false,
        PrimaryExpr::Parenthesized(inner) => depends_on_context(inner),
        PrimaryExpr::Function(call) => {
            if call.name.prefix.is_some() {
                return true;
            }
            let reads_context = match call.name.local.as_str() {
                "position" | "last" | "lang" => true,
                "string" | "string-length" | "normalize-space" | "name" | "local-name"
                | "namespace-uri" | "number" => call.args.is_empty(),
                "id" | "count" | "concat" | "starts-with" | "contains" | "substring-before"
                | "substring-after" | "substring" | "translate" | "boolean" | "not" | "true"
                | "false" | "sum" | "floor" | "ceiling" | "round" => false,
                _ => true,
            };
            reads_context || call.args.iter().any(depends_on_context)
        }
    }
}

/// Evaluates `steps` in turn. A `descendant-or-self::node()` step without
/// predicates followed by a `child` step whose predicates do not depend on
/// proximity position (the expansion of `//X[p]`) is evaluated as the one
/// step `descendant::X[p]`: same node-set (§2.5 points out the difference
/// only for positional predicates such as `//para[1]`), without
/// materialising every node of the subtree first (barrierlab#69).
fn evaluate_location_steps<'ctx, 'hook, N: Node<'ctx>>(
    steps: &[Step],
    mut nodes: Vec<N>,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Vec<N>, EvalError> {
    let mut i = 0;
    while i < steps.len() {
        let step = &steps[i];
        if let Some(next) = steps.get(i + 1)
            && is_plain_descendant_or_self(step)
            && next.axis == Axis::Child
            && next.predicates.iter().all(is_position_independent)
        {
            nodes = evaluate_step(
                Axis::Descendant,
                &next.node_test,
                &next.predicates,
                &nodes,
                ctx,
            )?;
            i += 2;
        } else {
            nodes = evaluate_step(step.axis, &step.node_test, &step.predicates, &nodes, ctx)?;
            i += 1;
        }
    }
    Ok(nodes)
}

/// `descendant-or-self::node()` with no predicates — what `//` expands to.
fn is_plain_descendant_or_self(step: &Step) -> bool {
    step.axis == Axis::DescendantOrSelf
        && step.node_test == NodeTest::Node
        && step.predicates.is_empty()
}

/// Whether a predicate selects the same nodes whatever the context
/// position and size: it never yields a number (a number predicate
/// compares against the position, §2.4) and calls neither `position()` nor
/// `last()` against the predicate's own context. Conservative: anything it
/// cannot prove, such as a variable that might hold a number, counts as
/// dependent.
fn is_position_independent(predicate: &Expr) -> bool {
    !may_be_number(predicate) && !uses_context_position(predicate)
}

/// Whether `expr` may evaluate to a number.
fn may_be_number(expr: &Expr) -> bool {
    match expr {
        Expr::Or(..) | Expr::And(..) | Expr::Equality(..) | Expr::Relational(..) => false,
        Expr::Additive(..) | Expr::Multiplicative(..) | Expr::Negate(_) => true,
        Expr::Union(..) => false,
        Expr::Path(PathExpr::Location(_) | PathExpr::FilterLocation(..)) => false,
        Expr::Path(PathExpr::Filter(filter)) => {
            filter.predicates.is_empty() && primary_may_be_number(&filter.primary)
        }
    }
}

fn primary_may_be_number(primary: &PrimaryExpr) -> bool {
    match primary {
        PrimaryExpr::Literal(_) => false,
        PrimaryExpr::Number(_) | PrimaryExpr::Variable(_) => true,
        PrimaryExpr::Parenthesized(inner) => may_be_number(inner),
        PrimaryExpr::Function(call) => {
            call.name.prefix.is_some()
                || !matches!(
                    call.name.local.as_str(),
                    "boolean"
                        | "not"
                        | "true"
                        | "false"
                        | "lang"
                        | "contains"
                        | "starts-with"
                        | "string"
                        | "concat"
                        | "substring"
                        | "substring-before"
                        | "substring-after"
                        | "normalize-space"
                        | "translate"
                        | "local-name"
                        | "namespace-uri"
                        | "name"
                        | "id"
                )
        }
    }
}

/// Whether `expr` calls `position()` or `last()` against its own context.
/// Predicates nested inside it have their own context and are skipped.
fn uses_context_position(expr: &Expr) -> bool {
    match expr {
        Expr::Or(l, r)
        | Expr::And(l, r)
        | Expr::Equality(l, _, r)
        | Expr::Relational(l, _, r)
        | Expr::Additive(l, _, r)
        | Expr::Multiplicative(l, _, r)
        | Expr::Union(l, r) => uses_context_position(l) || uses_context_position(r),
        Expr::Negate(e) => uses_context_position(e),
        Expr::Path(PathExpr::Location(_)) => false,
        Expr::Path(PathExpr::Filter(filter) | PathExpr::FilterLocation(filter, _)) => {
            primary_uses_context_position(&filter.primary)
        }
    }
}

fn primary_uses_context_position(primary: &PrimaryExpr) -> bool {
    match primary {
        PrimaryExpr::Literal(_) | PrimaryExpr::Number(_) | PrimaryExpr::Variable(_) => false,
        PrimaryExpr::Parenthesized(inner) => uses_context_position(inner),
        PrimaryExpr::Function(call) => {
            (call.name.prefix.is_none() && matches!(call.name.local.as_str(), "position" | "last"))
                || call.args.iter().any(uses_context_position)
        }
    }
}

fn evaluate_location_path<'ctx, 'hook, N: Node<'ctx>>(
    loc: &LocationPath,
    start: Vec<N>,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Vec<N>, EvalError> {
    let start = if loc.is_absolute {
        start
            .first()
            .map(|&n| vec![axes::root_of(n)])
            .unwrap_or_default()
    } else {
        start
    };
    evaluate_location_steps(&loc.steps, start, ctx)
}

fn evaluate_path<'ctx, 'hook, N: Node<'ctx>>(
    path: &PathExpr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    match path {
        PathExpr::Location(loc) => {
            let result = evaluate_location_path(loc, vec![ctx.node], ctx)?;
            Ok(Value::NodeSet(result))
        }
        PathExpr::Filter(filter) => evaluate_filter_expr(filter, ctx),
        PathExpr::FilterLocation(filter, loc) => {
            let filter_val = evaluate_filter_expr(filter, ctx)?;
            let start = match filter_val {
                Value::NodeSet(nodes) => nodes,
                _ => {
                    return Err(EvalError::ExpectedNodeSet {
                        context: "path expression left of '/'",
                    });
                }
            };
            let result = evaluate_location_path(loc, start, ctx)?;
            Ok(Value::NodeSet(result))
        }
    }
}

fn evaluate_filter_expr<'ctx, 'hook, N: Node<'ctx>>(
    filter: &FilterExpr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    let primary_val = evaluate_primary(&filter.primary, ctx)?;
    if filter.predicates.is_empty() {
        return Ok(primary_val);
    }
    match primary_val {
        Value::NodeSet(mut nodes) => {
            // `nodes` may come from a variable binding (`PrimaryExpr::Variable`)
            // in caller-supplied, arbitrary order — unlike a location step's
            // result, it isn't guaranteed to already be in document order.
            // Predicate proximity position must be document-order-based
            // (§2.4), so sort (and dedup, for the same reason `evaluate_step`
            // does) before applying predicates.
            sort_dedup(&mut nodes);
            let filtered = apply_predicates(&filter.predicates, nodes, ctx)?;
            Ok(Value::NodeSet(filtered))
        }
        _ => Err(EvalError::ExpectedNodeSet {
            context: "filter expression predicate",
        }),
    }
}

fn evaluate_primary<'ctx, 'hook, N: Node<'ctx>>(
    primary: &PrimaryExpr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    match primary {
        PrimaryExpr::Variable(qname) => {
            let bound = ctx.variables.and_then(|lookup| lookup(qname));
            bound.ok_or_else(|| EvalError::UnboundVariable(qname.clone()))
        }
        PrimaryExpr::Parenthesized(inner) => evaluate(inner, ctx),
        PrimaryExpr::Literal(s) => Ok(Value::String(s.clone())),
        PrimaryExpr::Number(n) => Ok(Value::Number(*n)),
        PrimaryExpr::Function(call) => evaluate_function(call, ctx),
    }
}

pub(crate) fn check_arity(
    function: &'static str,
    args: &[Expr],
    expected: usize,
) -> Result<(), EvalError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(EvalError::ArgumentCount {
            function,
            expected,
            got: args.len(),
        })
    }
}

/// `last()`/`position()`/`count()` are implemented here (Phase 04); every
/// other core-library function (Phase 05, `id()` excluded) is dispatched to
/// `functions::dispatch`, which itself falls back to
/// `EvalError::UnknownFunction` for any name it does not implement either —
/// deliberately: a generic fallback would hide the difference between "not
/// yet implemented" and "genuinely does not exist".
fn evaluate_function<'ctx, 'hook, N: Node<'ctx>>(
    call: &FunctionCall,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Value<N>, EvalError> {
    if call.name.prefix.is_none() {
        match call.name.local.as_str() {
            "last" => {
                check_arity("last", &call.args, 0)?;
                return Ok(Value::Number(ctx.size as f64));
            }
            "position" => {
                check_arity("position", &call.args, 0)?;
                return Ok(Value::Number(ctx.position as f64));
            }
            "count" => {
                check_arity("count", &call.args, 1)?;
                return match evaluate(&call.args[0], ctx)? {
                    Value::NodeSet(nodes) => Ok(Value::Number(nodes.len() as f64)),
                    _ => Err(EvalError::ExpectedNodeSet {
                        context: "count() argument",
                    }),
                };
            }
            _ => {}
        }
    }
    functions::dispatch(call, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::fixture::{Fixture, TestNode, build};

    fn eval_str<'a>(src: &str, node: TestNode<'a>) -> Result<Value<TestNode<'a>>, EvalError> {
        let expr = crate::parse(src).expect("test expression must parse");
        let ctx = EvaluationContext::new(node);
        evaluate(&expr, &ctx)
    }

    fn node_set<'a>(v: Result<Value<TestNode<'a>>, EvalError>) -> Vec<TestNode<'a>> {
        match v.expect("expected Ok") {
            Value::NodeSet(nodes) => nodes,
            other => panic!("expected a node-set, got {other:?}"),
        }
    }

    fn root(f: &Fixture) -> TestNode<'_> {
        f.doc.node(f.html)
    }

    // ---- Or / And (short-circuit) --------------------------------------

    #[test]
    fn or_short_circuits_when_left_is_true() {
        let f = build();
        assert_eq!(
            eval_str("(1 = 1) or unknownfn()", root(&f)),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn or_evaluates_right_when_left_is_false() {
        let f = build();
        assert!(matches!(
            eval_str("(1 = 0) or unknownfn()", root(&f)),
            Err(EvalError::UnknownFunction(_))
        ));
    }

    #[test]
    fn and_short_circuits_when_left_is_false() {
        let f = build();
        assert_eq!(
            eval_str("(1 = 0) and unknownfn()", root(&f)),
            Ok(Value::Boolean(false))
        );
    }

    #[test]
    fn and_evaluates_right_when_left_is_true() {
        let f = build();
        assert!(matches!(
            eval_str("(1 = 1) and unknownfn()", root(&f)),
            Err(EvalError::UnknownFunction(_))
        ));
    }

    // ---- Equality: all four node-set comparison cases -------------------

    #[test]
    fn equality_node_set_vs_node_set() {
        let f = build();
        assert_eq!(
            eval_str(
                "/html/body/span/@data-x = /html/body/span/@data-x",
                root(&f)
            ),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval_str("/html/body/span/@data-x = /html/@id", root(&f)),
            Ok(Value::Boolean(false))
        );
        assert_eq!(
            eval_str("/html/body/span/@data-x != /html/@id", root(&f)),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn equality_node_set_vs_number() {
        let f = build();
        assert_eq!(
            eval_str("/html/body/span/@data-x = 1", root(&f)),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval_str("/html/body/span/@data-x = 2", root(&f)),
            Ok(Value::Boolean(false))
        );
        assert_eq!(
            eval_str("/html/body/span/@data-x != 2", root(&f)),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn equality_node_set_vs_string() {
        let f = build();
        assert_eq!(
            eval_str("/html/body/span/@data-x = '1'", root(&f)),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval_str("/html/@id = 'nope'", root(&f)),
            Ok(Value::Boolean(false))
        );
    }

    #[test]
    fn equality_node_set_vs_boolean() {
        let f = build();
        // Non-empty node-set converts to boolean true.
        assert_eq!(
            eval_str("/html/body/span/@data-x = (1 = 1)", root(&f)),
            Ok(Value::Boolean(true))
        );
        // Empty node-set converts to boolean false.
        assert_eq!(
            eval_str("/html/body/nonexistent = (1 = 1)", root(&f)),
            Ok(Value::Boolean(false))
        );
    }

    // ---- Relational operators with a node-set operand -------------------

    #[test]
    fn relational_with_node_set_operand() {
        let f = build();
        assert_eq!(
            eval_str("/html/body/span/@data-x > 0", root(&f)),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval_str("/html/body/span/@data-x < 0", root(&f)),
            Ok(Value::Boolean(false))
        );
        // node-set vs string: the string is converted to a number too.
        assert_eq!(
            eval_str("/html/body/span/@data-x > '0'", root(&f)),
            Ok(Value::Boolean(true))
        );
        // node-set vs boolean.
        assert_eq!(
            eval_str("/html/body/span/@data-x > (1 = 0)", root(&f)),
            Ok(Value::Boolean(true))
        );
    }

    // ---- Additive / Multiplicative / Negate ------------------------------

    #[test]
    fn arithmetic_operators() {
        let f = build();
        assert_eq!(eval_str("1 + 2", root(&f)), Ok(Value::Number(3.0)));
        assert_eq!(eval_str("1 - 2", root(&f)), Ok(Value::Number(-1.0)));
        assert_eq!(eval_str("2 * 3", root(&f)), Ok(Value::Number(6.0)));
        assert_eq!(eval_str("7 div 2", root(&f)), Ok(Value::Number(3.5)));
        assert_eq!(eval_str("7 mod 3", root(&f)), Ok(Value::Number(1.0)));
        assert_eq!(eval_str("-1 + 2", root(&f)), Ok(Value::Number(1.0)));
    }

    // ---- Union with dedup -------------------------------------------------

    #[test]
    fn union_deduplicates_overlapping_node_sets() {
        let f = build();
        let nodes = node_set(eval_str(
            "/html/body/child::node() | /html/body/child::*",
            root(&f),
        ));
        assert_eq!(
            nodes,
            vec![
                f.doc.node(f.text0),
                f.doc.node(f.span),
                f.doc.node(f.comment),
                f.doc.node(f.pi),
                f.doc.node(f.text2),
            ],
            "span (present in both operands) must appear exactly once, in document order"
        );
    }

    // ---- Multi-step paths with predicates --------------------------------

    #[test]
    fn multi_step_path_with_numeric_predicate() {
        let f = build();
        let nodes = node_set(eval_str("/html/body/child::node()[3]", root(&f)));
        assert_eq!(nodes, vec![f.doc.node(f.comment)]);
    }

    #[test]
    fn multi_step_mixed_axis_path_with_predicate() {
        let f = build();
        // descendant-or-self (via "//") + child::text()[2] — the classic
        // XPath gotcha: `[2]` is *not* "the 2nd text node in the whole
        // subtree". A step's predicates apply once per context node
        // contributed by the previous step, each with its own local
        // position/size, and the results are unioned — exactly like the
        // well-known `//para[1]` example (which selects the first `para`
        // *child of each element that has one*, not the first `para` in
        // the document). Here, `descendant-or-self::node()` contributes
        // html/body/text0/span/text1/comment/pi/text2 as context nodes;
        // only `body` has 2+ text children ([text0, text2]), so `[2]`
        // picks `body`'s 2nd text child (text2) — `span`'s single text
        // child (text1) has no position 2 and contributes nothing.
        let nodes = node_set(eval_str("/html//text()[2]", root(&f)));
        assert_eq!(nodes, vec![f.doc.node(f.text2)]);
    }

    // ---- Sequential predicate filtering (the plan's flagged risk) --------

    #[test]
    fn predicate_order_changes_the_result() {
        let f = build();
        // body's children in document order: text0, span, comment, pi, text2
        // — span is the 2nd child.
        let position_then_test = node_set(eval_str(
            "/html/body/child::node()[2][self::span]",
            root(&f),
        ));
        assert_eq!(
            position_then_test,
            vec![f.doc.node(f.span)],
            "[2] keeps the 2nd child (span); [self::span] then confirms it against itself"
        );

        let test_then_position = node_set(eval_str(
            "/html/body/child::node()[self::span][2]",
            root(&f),
        ));
        assert_eq!(
            test_then_position,
            Vec::<TestNode>::new(),
            "[self::span] first narrows to just [span] (size 1); [2] then has nothing at position 2"
        );
    }

    // ---- last()/position()/count() ---------------------------------------

    #[test]
    fn last_position_count_in_predicates() {
        let f = build();
        assert_eq!(
            node_set(eval_str("/html/body/child::node()[last()]", root(&f))),
            vec![f.doc.node(f.text2)]
        );
        assert_eq!(
            node_set(eval_str(
                "/html/body/child::node()[position() > 1]",
                root(&f)
            )),
            vec![
                f.doc.node(f.span),
                f.doc.node(f.comment),
                f.doc.node(f.pi),
                f.doc.node(f.text2),
            ]
        );
        assert_eq!(
            eval_str("count(/html/body/child::node())", root(&f)),
            Ok(Value::Number(5.0))
        );
    }

    // ---- §2.4 predicate coercion: numeric vs. boolean predicate ----------

    #[test]
    fn numeric_predicate_selects_by_position() {
        let f = build();
        let nodes = node_set(eval_str("/html/body/child::node()[3]", root(&f)));
        assert_eq!(nodes, vec![f.doc.node(f.comment)]);
    }

    #[test]
    fn non_numeric_predicate_uses_boolean_coercion() {
        let f = build();
        // "span" as a predicate means "does this candidate have a child
        // named span?" — none of body's children (text/comment/pi nodes,
        // or span itself, whose own child is a text node) do.
        let nodes = node_set(eval_str("/html/body/child::node()[span]", root(&f)));
        assert_eq!(nodes, Vec::<TestNode>::new());
    }

    // ---- Errors -----------------------------------------------------------

    #[test]
    fn unknown_function_is_an_error() {
        let f = build();
        assert!(matches!(
            eval_str("foo()", root(&f)),
            Err(EvalError::UnknownFunction(_))
        ));
    }

    #[test]
    fn unbound_variable_is_an_error() {
        let f = build();
        assert!(matches!(
            eval_str("$missing", root(&f)),
            Err(EvalError::UnboundVariable(_))
        ));
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let f = build();
        assert!(matches!(
            eval_str("last(1)", root(&f)),
            Err(EvalError::ArgumentCount {
                function: "last",
                expected: 0,
                got: 1
            })
        ));
        assert!(matches!(
            eval_str("count()", root(&f)),
            Err(EvalError::ArgumentCount {
                function: "count",
                expected: 1,
                got: 0
            })
        ));
    }

    #[test]
    fn count_of_non_node_set_is_an_error() {
        let f = build();
        assert!(matches!(
            eval_str("count(1)", root(&f)),
            Err(EvalError::ExpectedNodeSet { .. })
        ));
    }

    // ---- Variable-binding hook ---------------------------------------------

    #[test]
    fn bound_variable_resolves_via_the_lookup_hook() {
        let f = build();
        let lookup = |q: &QName| -> Option<Value<TestNode<'_>>> {
            if q.prefix.is_none() && q.local == "x" {
                Some(Value::Number(42.0))
            } else {
                None
            }
        };
        let ctx = EvaluationContext {
            node: root(&f),
            position: 1,
            size: 1,
            variables: Some(&lookup),
            namespaces: None,
            cache: None,
            prefixes: None,
            _ctx: std::marker::PhantomData,
        };
        let expr = crate::parse("$x + 1").unwrap();
        assert_eq!(evaluate(&expr, &ctx), Ok(Value::Number(43.0)));
    }

    #[test]
    fn filter_expr_sorts_a_variable_supplied_node_set_into_document_order_before_predicates() {
        let f = build();
        let span = f.doc.node(f.span);
        let pi = f.doc.node(f.pi);
        // Deliberately unsorted: `pi` comes after `span` in document order,
        // but is listed first here. `$nodes[1]` must still pick the
        // document-order-first node (`span`), not the first `Vec` element —
        // otherwise it depends on the caller's arbitrary `Vec` order.
        let lookup = |q: &QName| -> Option<Value<TestNode<'_>>> {
            if q.prefix.is_none() && q.local == "nodes" {
                Some(Value::NodeSet(vec![pi, span]))
            } else {
                None
            }
        };
        let ctx = EvaluationContext {
            node: root(&f),
            position: 1,
            size: 1,
            variables: Some(&lookup),
            namespaces: None,
            cache: None,
            prefixes: None,
            _ctx: std::marker::PhantomData,
        };
        let expr = crate::parse("$nodes[1]").unwrap();
        assert_eq!(evaluate(&expr, &ctx), Ok(Value::NodeSet(vec![span])));
    }

    #[test]
    fn position_independence_is_conservative() {
        let independent = |src: &str| is_position_independent(&crate::parse(src).unwrap());
        assert!(independent("@a"));
        assert!(independent("@a = 1"));
        assert!(independent("not(@a)"));
        assert!(independent("foo[1]"));
        assert!(independent("x[position() = 2]"));
        assert!(!independent("1"));
        assert!(!independent("$x"));
        assert!(!independent("count(x)"));
        assert!(!independent("position() = 1"));
        assert!(!independent("not(last() = 1)"));
        assert!(!independent("@a and position() = 1"));
    }

    // ---- Namespace-resolution hook (Phase 04a) -----------------------------
    //
    // `matches_node_test` is exercised directly here (not through
    // `evaluate`/`parse`) because it needs a node whose `expanded_name()`
    // carries a real, non-`None` `namespace_uri` — the shared
    // `document::fixture` tree never populates one (see the pre-existing
    // doc comment on `ResolvedTest`), so a minimal standalone `Node`
    // impl is used instead of extending that fixture (out of scope here,
    // touching `document.rs`).

    fn matches_node_test<'a, N: Node<'a>>(
        test: &NodeTest,
        axis: Axis,
        n: N,
        namespaces: Option<&NamespaceLookup<'_>>,
    ) -> bool {
        let mut ctx = EvaluationContext::new(n);
        ctx.namespaces = namespaces;
        ResolvedTest::new(test, axis, &ctx).matches(n)
    }

    /// A single-node `Node` impl whose only purpose is carrying a chosen
    /// `namespace_uri`/`local_name` pair for the namespace-resolution tests
    /// below.
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct NsTestNode {
        namespace_uri: Option<&'static str>,
        local_name: &'static str,
    }

    impl<'a> Node<'a> for NsTestNode {
        fn kind(self) -> NodeKind {
            NodeKind::Element
        }

        fn parent(self) -> Option<Self> {
            None
        }

        fn children(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }

        fn attributes(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }

        fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }

        fn expanded_name(self) -> Option<crate::document::ExpandedName> {
            Some(crate::document::ExpandedName {
                namespace_uri: self.namespace_uri.map(str::to_string),
                local_name: self.local_name.to_string(),
            })
        }

        fn string_value(self) -> String {
            String::new()
        }

        fn document_order(self, _other: Self) -> std::cmp::Ordering {
            std::cmp::Ordering::Equal
        }
    }

    fn ns_node() -> NsTestNode {
        NsTestNode {
            namespace_uri: Some("http://example/ns"),
            local_name: "rect",
        }
    }

    fn qname_test() -> NodeTest {
        NodeTest::QName(QName {
            prefix: Some("p".to_string()),
            local: "rect".to_string(),
        })
    }

    #[test]
    fn qname_test_matches_when_hook_resolves_prefix_to_the_real_uri() {
        let lookup =
            |p: &str| -> Option<String> { (p == "p").then(|| "http://example/ns".to_string()) };
        assert!(matches_node_test(
            &qname_test(),
            Axis::Child,
            ns_node(),
            Some(&lookup)
        ));
    }

    #[test]
    fn qname_test_does_not_match_when_hook_does_not_know_the_prefix() {
        // The hook is present but returns `None` for "p" — that prefix is
        // unbound, so no node can match, regardless of what the raw prefix
        // string happens to look like.
        let lookup = |_: &str| -> Option<String> { None };
        assert!(!matches_node_test(
            &qname_test(),
            Axis::Child,
            ns_node(),
            Some(&lookup)
        ));
    }

    #[test]
    fn qname_test_does_not_match_when_hook_present_but_unresolvable_even_if_raw_prefix_equals_the_uri()
     {
        // An unresolvable prefix must NOT fall back to comparing the raw
        // prefix string against the namespace URI — not even when that
        // raw string happens to equal the real URI.
        let mut node = ns_node();
        node.namespace_uri = Some("p");
        let lookup = |_: &str| -> Option<String> { None };
        assert!(!matches_node_test(
            &qname_test(),
            Axis::Child,
            node,
            Some(&lookup)
        ));
    }

    #[test]
    fn qname_test_does_not_match_with_no_hook_at_all() {
        // No hook at all (`namespaces: None`) — a prefixed name test can
        // never match any node.
        assert!(!matches_node_test(
            &qname_test(),
            Axis::Child,
            ns_node(),
            None
        ));
    }

    #[test]
    fn qname_test_does_not_match_with_no_hook_at_all_even_if_raw_prefix_equals_the_uri() {
        // Regression: earlier, no hook at all fell back to comparing the
        // raw prefix string itself against the namespace URI, so this
        // *would* have matched. That fallback is gone — no hook now means
        // no match, symmetric with the hook-present-but-unresolvable case
        // above.
        let mut node = ns_node();
        node.namespace_uri = Some("p");
        assert!(!matches_node_test(&qname_test(), Axis::Child, node, None));
    }

    #[test]
    fn namespace_wildcard_matches_when_hook_resolves_prefix_to_the_real_uri() {
        let lookup =
            |p: &str| -> Option<String> { (p == "p").then(|| "http://example/ns".to_string()) };
        let test = NodeTest::NamespaceWildcard("p".to_string());
        assert!(matches_node_test(
            &test,
            Axis::Child,
            ns_node(),
            Some(&lookup)
        ));
    }

    #[test]
    fn namespace_wildcard_does_not_match_when_hook_does_not_know_the_prefix() {
        let lookup = |_: &str| -> Option<String> { None };
        let test = NodeTest::NamespaceWildcard("p".to_string());
        assert!(!matches_node_test(
            &test,
            Axis::Child,
            ns_node(),
            Some(&lookup)
        ));
    }

    #[test]
    fn namespace_wildcard_does_not_match_when_hook_present_but_unresolvable_even_if_raw_prefix_equals_the_uri()
     {
        let mut node = ns_node();
        node.namespace_uri = Some("p");
        let lookup = |_: &str| -> Option<String> { None };
        let test = NodeTest::NamespaceWildcard("p".to_string());
        assert!(!matches_node_test(&test, Axis::Child, node, Some(&lookup)));
    }

    #[test]
    fn namespace_wildcard_does_not_match_with_no_hook_at_all() {
        let test = NodeTest::NamespaceWildcard("p".to_string());
        assert!(!matches_node_test(&test, Axis::Child, ns_node(), None));
    }

    // ---- Phase 05b regression: hook lifetime decoupled from doc lifetime --
    //
    // A minimal parent/child-linked tree, purpose-built (like `NsTestNode`
    // above) because `document::fixture`'s shared tree never sets a real
    // namespace URI on an element. Unlike `NsTestNode`, this one has actual
    // parent/child links so a real `evaluate()` call — parse, axis walk,
    // node-test filtering, all of it — can be exercised end-to-end, not
    // just `matches_node_test` in isolation.
    #[derive(Debug)]
    enum HookLifetimeEntry {
        Root {
            children: Vec<usize>,
        },
        Element {
            #[allow(dead_code)]
            parent: usize,
            namespace_uri: Option<&'static str>,
            local_name: &'static str,
        },
    }

    #[derive(Debug)]
    struct HookLifetimeArena(Vec<HookLifetimeEntry>);

    #[derive(Clone, Copy, Debug)]
    struct HookLifetimeNode<'a> {
        arena: &'a HookLifetimeArena,
        idx: usize,
    }

    impl<'a> PartialEq for HookLifetimeNode<'a> {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self.arena, other.arena) && self.idx == other.idx
        }
    }
    impl<'a> Eq for HookLifetimeNode<'a> {}

    impl<'a> Node<'a> for HookLifetimeNode<'a> {
        fn kind(self) -> NodeKind {
            match &self.arena.0[self.idx] {
                HookLifetimeEntry::Root { .. } => NodeKind::Root,
                HookLifetimeEntry::Element { .. } => NodeKind::Element,
            }
        }
        fn parent(self) -> Option<Self> {
            match &self.arena.0[self.idx] {
                HookLifetimeEntry::Root { .. } => None,
                HookLifetimeEntry::Element { parent, .. } => Some(HookLifetimeNode {
                    arena: self.arena,
                    idx: *parent,
                }),
            }
        }
        fn children(self) -> impl Iterator<Item = Self> + 'a {
            let arena = self.arena;
            let indices = match &arena.0[self.idx] {
                HookLifetimeEntry::Root { children } => children.clone(),
                HookLifetimeEntry::Element { .. } => Vec::new(),
            };
            indices
                .into_iter()
                .map(move |i| HookLifetimeNode { arena, idx: i })
        }
        fn attributes(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }
        fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }
        fn expanded_name(self) -> Option<crate::document::ExpandedName> {
            match &self.arena.0[self.idx] {
                HookLifetimeEntry::Root { .. } => None,
                HookLifetimeEntry::Element {
                    namespace_uri,
                    local_name,
                    ..
                } => Some(crate::document::ExpandedName {
                    namespace_uri: namespace_uri.map(str::to_string),
                    local_name: local_name.to_string(),
                }),
            }
        }
        fn string_value(self) -> String {
            String::new()
        }
        fn document_order(self, other: Self) -> std::cmp::Ordering {
            self.idx.cmp(&other.idx)
        }
    }

    #[test]
    fn evaluate_result_with_locally_scoped_namespace_hook_outlives_the_hook() {
        // The document: lives for this whole test function — clearly
        // longer than the inner block below, exactly like a caller's
        // document living as long as its own function's `'a`.
        let arena = HookLifetimeArena(vec![
            HookLifetimeEntry::Root {
                children: vec![1, 2],
            },
            HookLifetimeEntry::Element {
                parent: 0,
                namespace_uri: Some("http://example/ns"),
                local_name: "rect",
            },
            HookLifetimeEntry::Element {
                parent: 0,
                namespace_uri: None,
                local_name: "other",
            },
        ]);
        let root_node = HookLifetimeNode {
            arena: &arena,
            idx: 0,
        };

        // This inner block is the point of the test (see
        // `plan/05b-decouple-hook-lifetime.md`): the namespace-resolution
        // hook, and the `EvaluationContext` borrowing it via `'hook`, are
        // both built *and dropped* here — modeling a `schematron-engine`
        // caller that builds its namespace hook locally from a `Schema`'s
        // `<ns>` bindings inside a function body. Before Phase 05b,
        // `EvaluationContext`'s single lifetime forced this hook to live
        // exactly as long as `root_node`'s document borrow, so returning
        // `nodes` (computed via the hook, but only actually borrowing the
        // outer, longer-lived `arena`) out of this block could never
        // compile (rustc E0515). With `'hook` decoupled from `'ctx`, it
        // does.
        let nodes = {
            let bound_uri = "http://example/ns".to_string();
            let namespace_hook = move |prefix: &str| -> Option<String> {
                (prefix == "p").then(|| bound_uri.clone())
            };
            let ctx = EvaluationContext {
                node: root_node,
                position: 1,
                size: 1,
                variables: None,
                namespaces: Some(&namespace_hook),
                cache: None,
                prefixes: None,
                _ctx: std::marker::PhantomData,
            };
            let expr = crate::parse("p:rect").expect("test expression must parse");
            match evaluate(&expr, &ctx).expect("expected Ok") {
                Value::NodeSet(nodes) => nodes,
                other => panic!("expected a node-set, got {other:?}"),
            }
        };

        // Used *after* the inner scope (and the hook it contained) has
        // ended — the actual assertion that the fix works, not just that
        // it compiles.
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes[0],
            HookLifetimeNode {
                arena: &arena,
                idx: 1
            }
        );
    }
}
