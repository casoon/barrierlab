//! Evaluation of the XPath 2.0 subset against the caller's document (XPath 2.0
//! §2.1.2 dynamic evaluation phase). The dynamic context is the focus (context
//! item, position, size) plus the variable and namespace hooks of
//! [`EvaluationContext`]; every expression yields a [`Sequence`].

use super::ast::{AxisStep, Expr, NodeComparison, SetOp};
use super::atomic::{
    Arithmetic, Atomic, AtomicType, Comparison, arithmetic, compare_values, general_pair,
};
use super::{Error, Item, Sequence, from_value, functions};
use crate::ast::{Axis, NodeTest, QName};
use crate::axes;
use crate::document::{Node, NodeKind};
use crate::eval::EvaluationContext;
use std::rc::Rc;

const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
pub(super) const XS_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
pub(super) const FN_NAMESPACE: &str = "http://www.w3.org/2005/xpath-functions";

/// Largest sequence `a to b` may produce.
const MAX_RANGE: i64 = 10_000_000;

pub(super) fn evaluate<'ctx, 'hook, N: Node<'ctx>>(
    expr: &Expr,
    ctx: &EvaluationContext<'ctx, 'hook, N>,
) -> Result<Sequence<N>, Error> {
    let env = Env {
        ctx,
        focus: Some(Focus {
            item: Item::Node(ctx.node),
            position: ctx.position,
            size: ctx.size,
        }),
        vars: None,
    };
    env.eval(expr)
}

#[derive(Clone)]
pub(super) struct Focus<N> {
    pub item: Item<N>,
    pub position: usize,
    pub size: usize,
}

struct Binding<N> {
    name: QName,
    value: Sequence<N>,
    next: Option<Rc<Binding<N>>>,
}

#[derive(Clone)]
pub(super) struct Env<'e, 'ctx, 'hook, N: Node<'ctx>> {
    ctx: &'e EvaluationContext<'ctx, 'hook, N>,
    pub focus: Option<Focus<N>>,
    vars: Option<Rc<Binding<N>>>,
}

/// Atomization (XPath 2.0 §2.4.2): a node becomes its string-value as
/// `xs:untypedAtomic` — there is no schema, so no node has a typed value.
pub(super) fn atomize<'ctx, N: Node<'ctx>>(sequence: &[Item<N>]) -> Vec<Atomic> {
    sequence
        .iter()
        .map(|item| match item {
            Item::Node(n) => Atomic::Untyped(n.string_value()),
            Item::Atomic(a) => a.clone(),
        })
        .collect()
}

/// Effective boolean value (XPath 2.0 §2.4.3).
pub(super) fn effective_boolean_value<'ctx, N: Node<'ctx>>(
    sequence: &[Item<N>],
) -> Result<bool, Error> {
    match sequence {
        [] => Ok(false),
        [Item::Node(_), ..] => Ok(true),
        [Item::Atomic(a)] => a.effective_boolean(),
        _ => Err(Error::dynamic(
            "FORG0006",
            "no effective boolean value for a sequence of several atomic values",
        )),
    }
}

fn boolean<N>(b: bool) -> Sequence<N> {
    vec![Item::Atomic(Atomic::Boolean(b))]
}

/// Exactly zero or one atomized item, as the operands of arithmetic and value
/// comparison require (§3.4, §3.5.1).
fn single_atomic<'ctx, N: Node<'ctx>>(
    sequence: &[Item<N>],
    what: &str,
) -> Result<Option<Atomic>, Error> {
    let mut atoms = atomize(sequence);
    if atoms.len() > 1 {
        return Err(Error::dynamic(
            "XPTY0004",
            format!(
                "{what} needs a single item, got a sequence of {}",
                atoms.len()
            ),
        ));
    }
    Ok(atoms.pop())
}

pub(super) fn sort_nodes<'ctx, N: Node<'ctx>>(nodes: &mut Vec<N>) {
    nodes.sort_by(|a, b| a.document_order(*b));
    nodes.dedup();
}

fn nodes_only<'ctx, N: Node<'ctx>>(
    sequence: Sequence<N>,
    code: &'static str,
    what: &str,
) -> Result<Vec<N>, Error> {
    sequence
        .into_iter()
        .map(|item| match item {
            Item::Node(n) => Ok(n),
            Item::Atomic(a) => Err(Error::dynamic(
                code,
                format!("{what} needs nodes, got {}", a.type_name()),
            )),
        })
        .collect()
}

/// A node test with its prefix resolved once per step.
enum Test {
    Node,
    Kind(NodeKind),
    ProcessingInstruction(Option<String>),
    AnyName(NodeKind),
    Namespace(NodeKind, String),
    Name(NodeKind, Option<String>, String),
}

impl Test {
    fn matches<'a, N: Node<'a>>(&self, n: N) -> bool {
        match self {
            Test::Node => true,
            Test::Kind(kind) | Test::AnyName(kind) => n.kind() == *kind,
            Test::ProcessingInstruction(target) => {
                n.kind() == NodeKind::ProcessingInstruction
                    && target
                        .as_ref()
                        .is_none_or(|t| n.expanded_name().is_some_and(|e| e.local_name == *t))
            }
            Test::Namespace(kind, uri) => {
                n.kind() == *kind
                    && n.expanded_name()
                        .is_some_and(|e| e.namespace_uri.as_deref() == Some(uri.as_str()))
            }
            Test::Name(kind, uri, local) => {
                n.kind() == *kind && n.has_expanded_name(uri.as_deref(), local)
            }
        }
    }
}

fn axis_nodes<'a, N: Node<'a>>(axis: Axis, n: N) -> Vec<N> {
    match axis {
        Axis::Ancestor => axes::ancestor(n).collect(),
        Axis::AncestorOrSelf => axes::ancestor_or_self(n).collect(),
        Axis::Attribute => axes::attribute(n).collect(),
        Axis::Child => axes::child(n).collect(),
        Axis::Descendant => axes::descendant(n).collect(),
        Axis::DescendantOrSelf => axes::descendant_or_self(n).collect(),
        Axis::Following => axes::following(n).collect(),
        Axis::FollowingSibling => axes::following_sibling(n).collect(),
        Axis::Namespace => axes::namespace(n).collect(),
        Axis::Parent => axes::parent(n).collect(),
        Axis::Preceding => axes::preceding(n).collect(),
        Axis::PrecedingSibling => axes::preceding_sibling(n).collect(),
        Axis::SelfAxis => axes::self_axis(n).collect(),
    }
}

impl<'e, 'ctx, 'hook, N: Node<'ctx>> Env<'e, 'ctx, 'hook, N> {
    pub(super) fn with_focus(&self, item: Item<N>, position: usize, size: usize) -> Self {
        Env {
            ctx: self.ctx,
            focus: Some(Focus {
                item,
                position,
                size,
            }),
            vars: self.vars.clone(),
        }
    }

    fn with_variable(&self, name: &QName, value: Sequence<N>) -> Self {
        Env {
            ctx: self.ctx,
            focus: self.focus.clone(),
            vars: Some(Rc::new(Binding {
                name: name.clone(),
                value,
                next: self.vars.clone(),
            })),
        }
    }

    pub(super) fn context_item(&self) -> Result<&Item<N>, Error> {
        self.focus
            .as_ref()
            .map(|f| &f.item)
            .ok_or_else(|| Error::dynamic("XPDY0002", "the context item is absent"))
    }

    fn context_node(&self, what: &str) -> Result<N, Error> {
        match self.context_item()? {
            Item::Node(n) => Ok(*n),
            Item::Atomic(a) => Err(Error::dynamic(
                "XPTY0020",
                format!("{what} needs a node as context item, got {}", a.type_name()),
            )),
        }
    }

    /// Resolves a prefix through the caller's hook, falling back to the
    /// prefixes every XPath 2.0 static context predefines (`xml`, `xs`, `fn`).
    /// An unbound prefix is a static error (`XPST0081`), not a name that
    /// silently matches nothing.
    pub(super) fn resolve_prefix(&self, prefix: &str) -> Result<String, Error> {
        if let Some(uri) = self.ctx.namespaces.and_then(|lookup| lookup(prefix)) {
            return Ok(uri);
        }
        match prefix {
            "xml" => Ok(XML_NAMESPACE.into()),
            "xs" => Ok(XS_NAMESPACE.into()),
            "fn" => Ok(FN_NAMESPACE.into()),
            _ => Err(Error::static_error(
                "XPST0081",
                format!("namespace prefix `{prefix}` is not bound"),
            )),
        }
    }

    pub(super) fn eval(&self, expr: &Expr) -> Result<Sequence<N>, Error> {
        match expr {
            Expr::Literal(a) => Ok(vec![Item::Atomic(a.clone())]),
            Expr::Variable(name) => self.variable(name),
            Expr::ContextItem => Ok(vec![self.context_item()?.clone()]),
            Expr::Sequence(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.extend(self.eval(item)?);
                }
                Ok(out)
            }
            Expr::Range(from, to) => self.range(from, to),
            Expr::If(condition, then, otherwise) => {
                if self.truth(condition)? {
                    self.eval(then)
                } else {
                    self.eval(otherwise)
                }
            }
            Expr::For(bindings, body) => {
                let mut out = Vec::new();
                self.for_each(bindings, body, &mut out)?;
                Ok(out)
            }
            Expr::Quantified(every, bindings, body) => {
                Ok(boolean(self.quantified(*every, bindings, body)?))
            }
            Expr::Or(a, b) => Ok(boolean(self.truth(a)? || self.truth(b)?)),
            Expr::And(a, b) => Ok(boolean(self.truth(a)? && self.truth(b)?)),
            Expr::General(a, op, b) => self.general(a, *op, b),
            Expr::Value(a, op, b) => self.value_comparison(a, *op, b),
            Expr::Node(a, op, b) => self.node_comparison(a, *op, b),
            Expr::Arithmetic(a, op, b) => self.binary_arithmetic(a, *op, b),
            Expr::Unary(negative, operand) => self.unary(*negative, operand),
            Expr::SetOperation(a, op, b) => self.set_operation(a, *op, b),
            Expr::Cast(operand, ty, optional, castable) => {
                self.cast(operand, *ty, *optional, *castable)
            }
            Expr::Root => {
                let node = self.context_node("`/`")?;
                Ok(vec![Item::Node(axes::root_of(node))])
            }
            Expr::Path(left, right) => self.path(left, right),
            Expr::Step(step) => {
                let node = self.context_node("an axis step")?;
                Ok(self
                    .axis_step(step, node)?
                    .into_iter()
                    .map(Item::Node)
                    .collect())
            }
            Expr::Filter(base, predicates) => {
                let mut items = self.eval(base)?;
                for predicate in predicates {
                    items = self.filter(items, predicate)?;
                }
                Ok(items)
            }
            Expr::Call(name, args) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg)?);
                }
                functions::call(self, name, values)
            }
        }
    }

    pub(super) fn truth(&self, expr: &Expr) -> Result<bool, Error> {
        effective_boolean_value(&self.eval(expr)?)
    }

    fn variable(&self, name: &QName) -> Result<Sequence<N>, Error> {
        let mut binding = self.vars.as_ref();
        while let Some(b) = binding {
            if b.name == *name {
                return Ok(b.value.clone());
            }
            binding = b.next.as_ref();
        }
        if let Some(value) = self.ctx.variables.and_then(|lookup| lookup(name)) {
            return Ok(from_value(value));
        }
        let shown = match &name.prefix {
            Some(p) => format!("{p}:{}", name.local),
            None => name.local.clone(),
        };
        Err(Error::static_error(
            "XPST0008",
            format!("variable `${shown}` is not bound"),
        ))
    }

    /// `a to b` (§3.3.1).
    fn range(&self, from: &Expr, to: &Expr) -> Result<Sequence<N>, Error> {
        let bound = |expr: &Expr| -> Result<Option<i64>, Error> {
            let Some(atom) = single_atomic(&self.eval(expr)?, "the operand of `to`")? else {
                return Ok(None);
            };
            let atom = match atom {
                Atomic::Untyped(_) => atom.cast(AtomicType::Integer)?,
                other => other,
            };
            match atom {
                Atomic::Integer(i) => Ok(Some(i)),
                other => Err(Error::dynamic(
                    "XPTY0004",
                    format!("`to` needs xs:integer operands, got {}", other.type_name()),
                )),
            }
        };
        let (Some(from), Some(to)) = (bound(from)?, bound(to)?) else {
            return Ok(Vec::new());
        };
        if to >= from && (to - from) >= MAX_RANGE {
            return Err(Error::dynamic(
                "FOAR0002",
                "the range is too large for this implementation",
            ));
        }
        Ok((from..=to)
            .map(|i| Item::Atomic(Atomic::Integer(i)))
            .collect())
    }

    /// §3.7: the tuple stream of the bindings, the body evaluated for each.
    fn for_each(
        &self,
        bindings: &[(QName, Expr)],
        body: &Expr,
        out: &mut Sequence<N>,
    ) -> Result<(), Error> {
        let Some(((name, source), rest)) = bindings.split_first() else {
            out.extend(self.eval(body)?);
            return Ok(());
        };
        for item in self.eval(source)? {
            self.with_variable(name, vec![item])
                .for_each(rest, body, out)?;
        }
        Ok(())
    }

    /// §3.9: `every` is true for an empty binding sequence, `some` false.
    fn quantified(
        &self,
        every: bool,
        bindings: &[(QName, Expr)],
        body: &Expr,
    ) -> Result<bool, Error> {
        let Some(((name, source), rest)) = bindings.split_first() else {
            return self.truth(body);
        };
        for item in self.eval(source)? {
            let holds = self
                .with_variable(name, vec![item])
                .quantified(every, rest, body)?;
            if holds != every {
                return Ok(holds);
            }
        }
        Ok(every)
    }

    /// §3.5.2: true if some pair of atomized items compares true.
    fn general(&self, a: &Expr, op: Comparison, b: &Expr) -> Result<Sequence<N>, Error> {
        let left = atomize(&self.eval(a)?);
        let right = atomize(&self.eval(b)?);
        for x in &left {
            for y in &right {
                if general_pair(x, y, op)? {
                    return Ok(boolean(true));
                }
            }
        }
        Ok(boolean(false))
    }

    /// §3.5.1: empty operand gives the empty sequence, otherwise a boolean.
    fn value_comparison(&self, a: &Expr, op: Comparison, b: &Expr) -> Result<Sequence<N>, Error> {
        let left = single_atomic(&self.eval(a)?, "a value comparison")?;
        let right = single_atomic(&self.eval(b)?, "a value comparison")?;
        let (Some(x), Some(y)) = (left, right) else {
            return Ok(Vec::new());
        };
        // Untyped operands are compared as xs:string.
        let as_string = |a: Atomic| match a {
            Atomic::Untyped(s) => Atomic::String(s),
            other => other,
        };
        Ok(boolean(
            op.holds(compare_values(&as_string(x), &as_string(y))?),
        ))
    }

    /// §3.5.3.
    fn node_comparison(
        &self,
        a: &Expr,
        op: NodeComparison,
        b: &Expr,
    ) -> Result<Sequence<N>, Error> {
        let operand = |expr: &Expr| -> Result<Option<N>, Error> {
            let mut nodes = nodes_only(self.eval(expr)?, "XPTY0004", "a node comparison")?;
            match nodes.len() {
                0 => Ok(None),
                1 => Ok(nodes.pop()),
                _ => Err(Error::dynamic(
                    "XPTY0004",
                    "a node comparison needs a single node",
                )),
            }
        };
        let (Some(x), Some(y)) = (operand(a)?, operand(b)?) else {
            return Ok(Vec::new());
        };
        Ok(boolean(match op {
            NodeComparison::Is => x == y,
            NodeComparison::Precedes => x != y && x.document_order(y).is_lt(),
            NodeComparison::Follows => x != y && x.document_order(y).is_gt(),
        }))
    }

    /// §3.4: an empty operand gives the empty sequence.
    fn binary_arithmetic(&self, a: &Expr, op: Arithmetic, b: &Expr) -> Result<Sequence<N>, Error> {
        let left = single_atomic(&self.eval(a)?, "an arithmetic operand")?;
        let right = single_atomic(&self.eval(b)?, "an arithmetic operand")?;
        let (Some(x), Some(y)) = (left, right) else {
            return Ok(Vec::new());
        };
        Ok(vec![Item::Atomic(arithmetic(&x, op, &y)?)])
    }

    fn unary(&self, negative: bool, operand: &Expr) -> Result<Sequence<N>, Error> {
        let Some(value) = single_atomic(&self.eval(operand)?, "a unary operator")? else {
            return Ok(Vec::new());
        };
        let value = match value {
            Atomic::Untyped(_) => value.cast(AtomicType::Double)?,
            other => other,
        };
        let result = match value {
            v if !negative && v.is_numeric() => v,
            Atomic::Integer(i) => Atomic::Integer(
                i.checked_neg()
                    .ok_or_else(|| Error::dynamic("FOAR0002", "numeric overflow"))?,
            ),
            Atomic::Decimal(d) => Atomic::Decimal(d.negated()),
            Atomic::Double(d) => Atomic::Double(-d),
            other => {
                return Err(Error::dynamic(
                    "XPTY0004",
                    format!("unary arithmetic needs a number, got {}", other.type_name()),
                ));
            }
        };
        Ok(vec![Item::Atomic(result)])
    }

    /// §3.3.3: set operators work on nodes and return document order.
    fn set_operation(&self, a: &Expr, op: SetOp, b: &Expr) -> Result<Sequence<N>, Error> {
        let left = nodes_only(self.eval(a)?, "XPTY0004", "a set operator")?;
        let right = nodes_only(self.eval(b)?, "XPTY0004", "a set operator")?;
        let mut result = match op {
            SetOp::Union => left.into_iter().chain(right).collect(),
            SetOp::Intersect => left.into_iter().filter(|n| right.contains(n)).collect(),
            SetOp::Except => left.into_iter().filter(|n| !right.contains(n)).collect(),
        };
        sort_nodes(&mut result);
        Ok(result.into_iter().map(Item::Node).collect())
    }

    /// §3.10.2 `cast as` and §3.10.3 `castable as`.
    fn cast(
        &self,
        operand: &Expr,
        ty: AtomicType,
        optional: bool,
        castable: bool,
    ) -> Result<Sequence<N>, Error> {
        let atoms = atomize(&self.eval(operand)?);
        let attempt = match atoms.as_slice() {
            [] if optional => {
                return Ok(if castable { boolean(true) } else { Vec::new() });
            }
            [] => Err(Error::dynamic("XPTY0004", "cast of an empty sequence")),
            [single] => single.cast(ty),
            _ => Err(Error::dynamic(
                "XPTY0004",
                "cast of a sequence of several items",
            )),
        };
        match (attempt, castable) {
            (Ok(_), true) => Ok(boolean(true)),
            (Ok(value), false) => Ok(vec![Item::Atomic(value)]),
            (Err(e @ Error::Unsupported(_)), _) => Err(e),
            (Err(_), true) => Ok(boolean(false)),
            (Err(e), false) => Err(e),
        }
    }

    /// §3.2 `E1/E2`: `E2` runs once per node of `E1`. All-node results are
    /// returned in document order without duplicates; all-atomic results are
    /// concatenated in order; a mixture is `XPTY0018`.
    fn path(&self, left: &Expr, right: &Expr) -> Result<Sequence<N>, Error> {
        let nodes = nodes_only(self.eval(left)?, "XPTY0019", "the left side of `/`")?;
        let size = nodes.len();
        let mut results: Sequence<N> = Vec::new();
        for (i, node) in nodes.into_iter().enumerate() {
            results.extend(self.with_focus(Item::Node(node), i + 1, size).eval(right)?);
        }
        let node_count = results
            .iter()
            .filter(|item| matches!(item, Item::Node(_)))
            .count();
        if node_count == 0 {
            return Ok(results);
        }
        if node_count != results.len() {
            return Err(Error::dynamic(
                "XPTY0018",
                "the result of `/` mixes nodes and atomic values",
            ));
        }
        let mut nodes: Vec<N> = results
            .into_iter()
            .filter_map(|item| match item {
                Item::Node(n) => Some(n),
                Item::Atomic(_) => None,
            })
            .collect();
        sort_nodes(&mut nodes);
        Ok(nodes.into_iter().map(Item::Node).collect())
    }

    fn resolve_test(&self, test: &NodeTest, axis: Axis) -> Result<Test, Error> {
        let principal = if axis == Axis::Attribute {
            NodeKind::Attribute
        } else {
            NodeKind::Element
        };
        Ok(match test {
            NodeTest::Node => Test::Node,
            NodeTest::Text => Test::Kind(NodeKind::Text),
            NodeTest::Comment => Test::Kind(NodeKind::Comment),
            NodeTest::ProcessingInstruction(target) => Test::ProcessingInstruction(target.clone()),
            NodeTest::AnyName => Test::AnyName(principal),
            NodeTest::NamespaceWildcard(prefix) => {
                Test::Namespace(principal, self.resolve_prefix(prefix)?)
            }
            NodeTest::QName(name) => {
                let uri = name
                    .prefix
                    .as_deref()
                    .map(|p| self.resolve_prefix(p))
                    .transpose()?;
                Test::Name(principal, uri, name.local.clone())
            }
        })
    }

    /// §3.2.1: the nodes on the axis that pass the test, then each predicate
    /// with positions counted along the axis direction; the result is in
    /// document order.
    fn axis_step(&self, step: &AxisStep, node: N) -> Result<Vec<N>, Error> {
        let test = self.resolve_test(&step.test, step.axis)?;
        let mut nodes: Vec<N> = axis_nodes(step.axis, node)
            .into_iter()
            .filter(|n| test.matches(*n))
            .collect();
        for predicate in &step.predicates {
            let size = nodes.len();
            let mut kept = Vec::new();
            for (i, n) in nodes.into_iter().enumerate() {
                let result = self
                    .with_focus(Item::Node(n), i + 1, size)
                    .eval(predicate)?;
                if predicate_holds(&result, i + 1)? {
                    kept.push(n);
                }
            }
            nodes = kept;
        }
        sort_nodes(&mut nodes);
        Ok(nodes)
    }

    /// §3.2.2 predicate on a sequence.
    fn filter(&self, items: Sequence<N>, predicate: &Expr) -> Result<Sequence<N>, Error> {
        let size = items.len();
        let mut kept = Vec::new();
        for (i, item) in items.into_iter().enumerate() {
            let result = self.with_focus(item.clone(), i + 1, size).eval(predicate)?;
            if predicate_holds(&result, i + 1)? {
                kept.push(item);
            }
        }
        Ok(kept)
    }
}

/// §3.2.2: a single numeric predicate value selects by position, anything else
/// by effective boolean value.
fn predicate_holds<'ctx, N: Node<'ctx>>(
    result: &[Item<N>],
    position: usize,
) -> Result<bool, Error> {
    if let [Item::Atomic(number)] = result
        && number.is_numeric()
    {
        let wanted = Atomic::Integer(position as i64);
        return Ok(compare_values(number, &wanted)?.is_some_and(|o| o.is_eq()));
    }
    effective_boolean_value(result)
}
