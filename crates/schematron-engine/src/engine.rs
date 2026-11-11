//! Phase 03: evaluates a parsed [`Schema`] against a caller-provided
//! document, producing structured [`Report`]s for fired checks.
//!
//! Execution model (based on general knowledge of the Schematron
//! reference implementation rather than a verbatim ISO 19757-3 quote):
//!
//! - Patterns are independent: the same node may be claimed by rules in
//!   several different patterns.
//! - Within a single pattern, at most one rule applies per node: rules are
//!   tried in schema order, and a node already claimed by an earlier rule
//!   in the same pattern is not re-evaluated by later rules in that
//!   pattern ("first matching rule wins" — this is not a Stage-1
//!   simplification but the actual ISO Schematron conflict-resolution
//!   rule; `rule` has no `priority` attribute in the grammar).
//! - `rule context="X"` is evaluated as the path
//!   `descendant-or-self::node()/X` from the document root — the usual
//!   pragmatic reduction of an XSLT-style match pattern to a regular
//!   location path. When `X` is itself a `|`-union of several alternatives
//!   (e.g. `context="a | b"`), each top-level alternative gets its own
//!   `descendant-or-self::node()/` prefix (on the parsed expression, see
//!   `context.rs`) rather than prefixing the whole string once: XPath's `/` binds
//!   tighter than `|`, so a single shared prefix would only apply to the
//!   first alternative — every later one would be evaluated as a bare
//!   relative path from the document root instead of "anywhere in the
//!   document", matching only a direct root child of that name.
//! - `assert test="X"` fires (produces a `Report`) when `X` evaluates to
//!   `false`; `report test="X"` fires when `X` evaluates to `true`.
//! - `<let name="..." value="...">` (Stage 2)
//!   binds XPath variables visible to `context`/`test` expressions in its
//!   scope (schema/pattern/rule), evaluated via `extend_lets`.
//! - `phase` selection (Stage 2) restricts
//!   evaluation to a subset of patterns via [`evaluate_with_phase`].
//!   `evaluate` is `evaluate_with_phase` with `schema.default_phase`
//!   (ISO's `#DEFAULT`); `phase_id: None` in `evaluate_with_phase` means
//!   "all patterns" (ISO's `#ALL`).
//! - `diagnostics` (Stage 2): a fired check's
//!   `diagnostics="id1 id2"` IDREFS are rendered the same way as its own
//!   message, against the same context node. Unknown ids are skipped, not
//!   an error (same accepted simplification as `Phase.active`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xpath_eval::{
    Atomic, Document, DocumentCache, EvaluationContext, Expr, Expression, Item, Node, NodeKind,
    NodeTest, QName, Sequence, Value, VersionedError, XPathVersion,
};

use crate::context::Alternative;
use crate::schema::{
    Check, CheckKind, Diagnostic, LetBinding, LetValue, MessagePart, Pattern, Schema,
};
use crate::svrl::Event;

/// One fired check: a failed `assert`, or a true-evaluating `report`.
/// Checks that do not fire are not represented at all (no `fired: bool`
/// field) — this is the list of assertion results, not an evaluation trace.
#[derive(Clone, Debug)]
pub struct Report<N> {
    pub pattern_id: Option<String>,
    pub check_id: Option<String>,
    pub role: Option<String>,
    pub kind: CheckKind,
    pub message: String,
    /// Rendered messages of this check's `diagnostics` IDREFS, in
    /// reference order.
    pub diagnostics: Vec<String>,
    pub node: N,
}

/// An error evaluating a [`Schema`] against a document.
///
/// Distinct from `xpath_eval::EvalError` (note the different name) so
/// callers can `use` both without a collision. Wraps both failure modes
/// `xpath-eval` can produce — a `context`/`test` string that is not
/// syntactically valid XPath ([`SchematronEvalError::Parse`]), and a
/// well-formed expression that fails at evaluation time
/// ([`SchematronEvalError::Eval`]) — plus one failure mode specific to
/// this crate's `context`-as-path-selection simplification: a `context`
/// expression that parses but does not evaluate to a node-set
/// ([`SchematronEvalError::ContextNotNodeSet`]).
#[derive(Debug, Clone, PartialEq)]
pub enum SchematronEvalError {
    /// A `context` or `test` expression failed to parse as XPath.
    Parse(xpath_eval::ParseError),
    /// A well-formed expression failed at evaluation time.
    Eval(xpath_eval::EvalError),
    /// A rule's `context`, once evaluated as `descendant-or-self::node()/`
    /// prefixed onto the raw context string, did not evaluate to a
    /// node-set (e.g. it evaluated to a number or string instead) —
    /// `context` must be a location-path/node-set expression.
    ContextNotNodeSet { context: String },
    /// [`evaluate_with_phase`] was called with a `phase_id` that does not
    /// match any `Schema.phases[_].id` (or `Schema.default_phase` names
    /// such an unknown phase).
    UnknownPhase { phase: String },
    /// An XPath 2.0 expression (`queryBinding` `xslt2`/`xpath2`) could not be
    /// evaluated: [`VersionedError::Unsupported`] for a construct or function
    /// `xpath-eval` does not implement (the evaluation did not run, which is
    /// not the same as passing), or a static or dynamic XPath error.
    Query(VersionedError),
}

impl SchematronEvalError {
    /// `true` when the schema uses an XPath 2.0 feature `xpath-eval` does not
    /// implement: the run is incomplete, not failed.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, SchematronEvalError::Query(error) if error.is_unsupported())
    }
}

/// Maps an XPath error to this crate's error: the 1.0 errors keep their
/// variants, everything specific to XPath 2.0 becomes [`SchematronEvalError::Query`].
#[cfg(feature = "xpath2")]
fn from_versioned(error: VersionedError) -> SchematronEvalError {
    match error {
        VersionedError::Parse(error) => SchematronEvalError::Parse(error),
        VersionedError::Eval(error) => SchematronEvalError::Eval(error),
        other => SchematronEvalError::Query(other),
    }
}

impl std::fmt::Display for SchematronEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchematronEvalError::Parse(err) => write!(f, "XPath parse error: {err}"),
            SchematronEvalError::Eval(err) => write!(f, "XPath evaluation error: {err}"),
            SchematronEvalError::ContextNotNodeSet { context } => {
                write!(f, "rule context {context:?} did not evaluate to a node-set")
            }
            SchematronEvalError::UnknownPhase { phase } => {
                write!(f, "no <phase id={phase:?}> in this schema")
            }
            SchematronEvalError::Query(err) => write!(f, "XPath 2.0 error: {err}"),
        }
    }
}

impl std::error::Error for SchematronEvalError {}

/// Evaluates `schema` against `document` under ISO's `#DEFAULT` phase
/// resolution: `schema.default_phase` if set, otherwise `#ALL` (every
/// pattern) — see [`evaluate_with_phase`].
/// Returns one [`Report`] per fired check, in the order patterns/rules/
/// checks appear in the schema and nodes are visited (document order
/// within a rule's matches).
pub fn evaluate<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
) -> Result<Vec<Report<D::N<'a>>>, SchematronEvalError> {
    evaluate_with_phase(schema, document, schema.default_phase.as_deref())
}

/// Evaluates `schema` against `document`, restricted to the patterns
/// activated by the phase named `phase_id` — `None` means `#ALL` (every
/// pattern, ignoring `Schema.phases` entirely). `Some(id)` with no
/// matching `Schema.phases[_].id` is [`SchematronEvalError::UnknownPhase`].
pub fn evaluate_with_phase<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
    phase_id: Option<&str>,
) -> Result<Vec<Report<D::N<'a>>>, SchematronEvalError> {
    evaluate_in(schema, document, phase_id, &DocumentCache::new(), None)
}

/// [`evaluate`] with a full trace instead of the fired checks alone: which
/// patterns were active, which rules fired on which node, and the fired
/// checks, in evaluation order — the input of [`crate::to_svrl`].
pub fn evaluate_trace<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
) -> Result<Vec<Event<D::N<'a>>>, SchematronEvalError> {
    evaluate_trace_with_phase(schema, document, schema.default_phase.as_deref())
}

/// [`evaluate_with_phase`] with a full trace, see [`evaluate_trace`].
pub fn evaluate_trace_with_phase<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
    phase_id: Option<&str>,
) -> Result<Vec<Event<D::N<'a>>>, SchematronEvalError> {
    let mut events = Vec::new();
    evaluate_in(
        schema,
        document,
        phase_id,
        &DocumentCache::new(),
        Some(&mut events),
    )?;
    Ok(events)
}

/// [`evaluate`] with a caller-held [`DocumentCache`] for `document`, so
/// several schemas checked against the same document share what the XPath
/// evaluator memoises about it (barrierlab#69). Results are the same as
/// with [`evaluate`].
pub fn evaluate_with_cache<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
    cache: &DocumentCache<D::N<'a>>,
) -> Result<Vec<Report<D::N<'a>>>, SchematronEvalError> {
    evaluate_in(
        schema,
        document,
        schema.default_phase.as_deref(),
        cache,
        None,
    )
}

fn evaluate_in<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
    phase_id: Option<&str>,
    cache: &DocumentCache<D::N<'a>>,
    trace: Option<&mut Vec<Event<D::N<'a>>>>,
) -> Result<Vec<Report<D::N<'a>>>, SchematronEvalError> {
    evaluate_schema(schema, document, phase_id, cache, trace).map_err(|error| {
        // A call of a function the schema declares with `xsl:function` is not an
        // unknown function: this crate does not run XSLT, so say that.
        if let SchematronEvalError::Query(VersionedError::Static {
            code: "XPST0017",
            message,
        }) = &error
            && let Some(name) = schema
                .xslt_functions
                .iter()
                .find(|name| message.contains(&format!("`{name}`")))
        {
            return SchematronEvalError::Query(VersionedError::Unsupported(format!(
                "the function {name} declared with xsl:function (XSLT is not run)"
            )));
        }
        error
    })
}

fn evaluate_schema<'a, D: Document>(
    schema: &Schema,
    document: &'a D,
    phase_id: Option<&str>,
    cache: &DocumentCache<D::N<'a>>,
    mut trace: Option<&mut Vec<Event<D::N<'a>>>>,
) -> Result<Vec<Report<D::N<'a>>>, SchematronEvalError> {
    let ns_lookup = |prefix: &str| {
        schema
            .ns
            .iter()
            .find(|binding| binding.prefix == prefix)
            .map(|binding| binding.uri.clone())
    };

    let phase = phase_id
        .map(|id| {
            schema
                .phases
                .iter()
                .find(|phase| phase.id == id)
                .ok_or_else(|| SchematronEvalError::UnknownPhase {
                    phase: id.to_owned(),
                })
        })
        .transpose()?;

    let version =
        XPathVersion::from_query_binding(schema.query_binding.as_deref()).ok_or_else(|| {
            SchematronEvalError::Query(VersionedError::Unsupported(format!(
                "queryBinding `{}`",
                schema.query_binding.as_deref().unwrap_or_default()
            )))
        })?;
    #[cfg(not(feature = "xpath2"))]
    if version == XPathVersion::V2 {
        return Err(SchematronEvalError::Query(VersionedError::Unsupported(
            "queryBinding `xslt2`/`xpath2` (the feature `xpath2` of schematron-engine is off)"
                .to_owned(),
        )));
    }
    let exprs = ExprCache::default();
    let exprs2 = ExprCache2::default();
    let env = Env {
        ns_lookup: &ns_lookup,
        exprs: &exprs,
        exprs2: &exprs2,
        version,
        cache,
    };
    let mut reports = Vec::new();
    let mut index = None;

    let mut schema_scope = Scope::new(None);
    extend_lets(&mut schema_scope, &schema.lets, document.root(), &env)?;

    let mut phase_scope = Scope::new(Some(&schema_scope));
    if let Some(phase) = phase {
        extend_lets(&mut phase_scope, &phase.lets, document.root(), &env)?;
    }

    for pattern in &schema.patterns {
        if let Some(phase) = phase {
            let is_active = pattern
                .id
                .as_deref()
                .is_some_and(|id| phase.active.iter().any(|active| active == id));
            if !is_active {
                continue;
            }
        }

        if let Some(trace) = trace.as_deref_mut() {
            trace.push(Event::ActivePattern {
                id: pattern.id.clone(),
            });
        }

        let mut pattern_scope = Scope::new(Some(&phase_scope));
        extend_lets(&mut pattern_scope, &pattern.lets, document.root(), &env)?;

        let matched = match_pattern(pattern, document, &env, &mut index)?;

        for (rule, nodes) in pattern.rules.iter().zip(matched) {
            for node in nodes {
                let mut node_scope = Scope::new(Some(&pattern_scope));
                extend_lets(&mut node_scope, &rule.lets, node, &env)?;
                if let Some(trace) = trace.as_deref_mut() {
                    trace.push(Event::FiredRule {
                        context: rule.context.clone(),
                        node,
                    });
                }

                for check in &rule.checks {
                    if let Some(report) = evaluate_check(
                        check,
                        node,
                        pattern.id.clone(),
                        &node_scope,
                        &env,
                        &schema.diagnostics,
                    )? {
                        if let Some(trace) = trace.as_deref_mut() {
                            trace.push(Event::Check {
                                report: report.clone(),
                                test: check.test.clone(),
                                diagnostic_ids: check
                                    .diagnostics
                                    .iter()
                                    .filter(|id| schema.diagnostics.iter().any(|d| &d.id == *id))
                                    .cloned()
                                    .collect(),
                            });
                        }
                        reports.push(report);
                    }
                }
            }
        }
    }

    Ok(reports)
}

/// The nodes a `child::` step can select (everything but the root,
/// attributes and namespace nodes), in document order, and the elements
/// among them by expanded name. Built by one walk per document.
struct NodeIndex<N> {
    all: Vec<N>,
    by_name: HashMap<(Option<String>, String), Vec<N>>,
}

impl<'a, N: Node<'a>> NodeIndex<N> {
    fn new(root: N) -> Self {
        let mut index = NodeIndex {
            all: Vec::new(),
            by_name: HashMap::new(),
        };
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if node != root {
                index.all.push(node);
                if node.kind() == NodeKind::Element
                    && let Some(name) = node.expanded_name()
                {
                    index
                        .by_name
                        .entry((name.namespace_uri, name.local_name))
                        .or_default()
                        .push(node);
                }
            }
            let start = stack.len();
            stack.extend(node.children());
            stack[start..].reverse();
        }
        index
    }
}

/// A [`Alternative::Node`] with its name test resolved, so most nodes are
/// rejected on their name without evaluating anything.
enum NodeMatcher<'c> {
    /// `*`, or a name test: an element, optionally of this expanded name,
    /// whose `predicates` all hold.
    Element {
        name: Option<(Option<String>, &'c str)>,
        predicates: &'c [Expr],
    },
    /// Any other node test: the whole `self::` step is evaluated.
    Step(&'c Expr),
}

impl<'c> NodeMatcher<'c> {
    fn new(
        test: &'c NodeTest,
        predicates: &'c [Expr],
        step: &'c Expr,
        ns_lookup: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        match test {
            NodeTest::AnyName => NodeMatcher::Element {
                name: None,
                predicates,
            },
            NodeTest::QName(qname) => match qname.prefix.as_deref() {
                None => NodeMatcher::Element {
                    name: Some((None, &qname.local)),
                    predicates,
                },
                Some(prefix) => match ns_lookup(prefix) {
                    Some(uri) => NodeMatcher::Element {
                        name: Some((Some(uri), &qname.local)),
                        predicates,
                    },
                    // Unbound prefix: left to the evaluator to report.
                    None => NodeMatcher::Step(step),
                },
            },
            _ => NodeMatcher::Step(step),
        }
    }

    fn matches<'a, N: Node<'a>>(
        &self,
        node: N,
        ctx: &EvaluationContext<'a, '_, N>,
    ) -> Result<bool, SchematronEvalError> {
        match self {
            NodeMatcher::Step(step) => {
                xpath_eval::evaluate_boolean(step, ctx).map_err(SchematronEvalError::Eval)
            }
            NodeMatcher::Element { name, predicates } => {
                if node.kind() != NodeKind::Element {
                    return Ok(false);
                }
                if let Some((uri, local)) = name
                    && !node.has_expanded_name(uri.as_deref(), local)
                {
                    return Ok(false);
                }
                for predicate in *predicates {
                    if !xpath_eval::evaluate_boolean(predicate, ctx)
                        .map_err(SchematronEvalError::Eval)?
                    {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

/// For each rule of `pattern`, the nodes it claims in document order: a
/// node belongs to the first rule of the pattern whose context matches it.
fn match_pattern<'a, D: Document>(
    pattern: &Pattern,
    document: &'a D,
    env: &Env<'_, D::N<'a>>,
    index: &mut Option<NodeIndex<D::N<'a>>>,
) -> Result<Vec<Vec<D::N<'a>>>, SchematronEvalError> {
    let root = document.root();
    let mut ctx = EvaluationContext::new(root);
    ctx.namespaces = Some(env.ns_lookup);
    ctx.cache = Some(env.cache);

    #[cfg(feature = "xpath2")]
    if env.version == XPathVersion::V2 {
        return match_pattern_v2(pattern, &ctx);
    }

    let mut compiled = Vec::new();
    for rule in &pattern.rules {
        compiled.push(
            rule.compiled
                .alternatives(&rule.context)
                .map_err(SchematronEvalError::Parse)?,
        );
    }

    let mut matched = Vec::with_capacity(pattern.rules.len());
    let mut claimed: Vec<D::N<'a>> = Vec::new();
    for (rule, alternatives) in pattern.rules.iter().zip(&compiled) {
        let mut nodes = Vec::new();
        for alternative in alternatives.iter() {
            match alternative {
                Alternative::Node {
                    test,
                    predicates,
                    step,
                } => {
                    let index = index.get_or_insert_with(|| NodeIndex::new(root));
                    let matcher = NodeMatcher::new(test, predicates, step, env.ns_lookup);
                    let candidates = match &matcher {
                        NodeMatcher::Element {
                            name: Some((uri, local)),
                            ..
                        } => index
                            .by_name
                            .get(&(uri.clone(), (*local).to_owned()))
                            .map_or(&[][..], Vec::as_slice),
                        _ => index.all.as_slice(),
                    };
                    for &node in candidates {
                        let mut node_ctx = ctx;
                        node_ctx.node = node;
                        if matcher.matches(node, &node_ctx)? {
                            nodes.push(node);
                        }
                    }
                }
                Alternative::Set(expr) => {
                    match xpath_eval::evaluate(expr, &ctx).map_err(SchematronEvalError::Eval)? {
                        Value::NodeSet(set) => nodes.extend(set),
                        _ => {
                            return Err(SchematronEvalError::ContextNotNodeSet {
                                context: rule.context.clone(),
                            });
                        }
                    }
                }
            }
        }
        nodes.sort_by(|a, b| a.document_order(*b));
        nodes.dedup();

        nodes.retain(|&node| {
            claimed
                .binary_search_by(|probe| probe.document_order(node))
                .is_err()
        });
        claimed = merge_in_document_order(claimed, nodes.clone());
        matched.push(nodes);
    }

    Ok(matched)
}

/// [`match_pattern`] for XPath 2.0. A rule context is a pattern: it matches the nodes that
/// `descendant-or-self::node()/(context)` selects (a context starting with `/` and without a
/// union is evaluated from the root alone, which selects the same nodes without walking every
/// node). Rule contexts are not rewritten into per-node tests as in 1.0; each is evaluated.
#[cfg(feature = "xpath2")]
fn match_pattern_v2<'a, N: Node<'a>>(
    pattern: &Pattern,
    ctx: &EvaluationContext<'a, '_, N>,
) -> Result<Vec<Vec<N>>, SchematronEvalError> {
    let mut matched = Vec::with_capacity(pattern.rules.len());
    let mut claimed: Vec<N> = Vec::new();
    for rule in &pattern.rules {
        let trimmed = rule.context.trim();
        let source = if trimmed.starts_with('/') && !trimmed.contains('|') {
            trimmed.to_owned()
        } else {
            format!("descendant-or-self::node()/({trimmed})")
        };
        let expr =
            xpath_eval::parse_versioned(&source, XPathVersion::V2).map_err(from_versioned)?;
        let mut nodes = Vec::new();
        for item in xpath_eval::evaluate_versioned(&expr, ctx).map_err(from_versioned)? {
            match item {
                Item::Node(node) => nodes.push(node),
                Item::Atomic(_) => {
                    return Err(SchematronEvalError::ContextNotNodeSet {
                        context: rule.context.clone(),
                    });
                }
            }
        }
        nodes.sort_by(|a, b| a.document_order(*b));
        nodes.dedup();
        nodes.retain(|&node| {
            claimed
                .binary_search_by(|probe| probe.document_order(node))
                .is_err()
        });
        claimed = merge_in_document_order(claimed, nodes.clone());
        matched.push(nodes);
    }
    Ok(matched)
}

/// Merges two node lists that are each in document order and disjoint.
fn merge_in_document_order<'n, N: Node<'n>>(a: Vec<N>, b: Vec<N>) -> Vec<N> {
    if b.is_empty() {
        return a;
    }
    let mut merged = Vec::with_capacity(a.len() + b.len());
    let (mut a, mut b) = (a.into_iter().peekable(), b.into_iter().peekable());
    while let (Some(&x), Some(&y)) = (a.peek(), b.peek()) {
        if x.document_order(y).is_lt() {
            merged.push(x);
            a.next();
        } else {
            merged.push(y);
            b.next();
        }
    }
    merged.extend(a);
    merged.extend(b);
    merged
}

/// What every expression of one evaluation shares: the schema's namespace
/// bindings, the parsed expressions and the per-document XPath memo.
struct Env<'e, N> {
    ns_lookup: &'e dyn Fn(&str) -> Option<String>,
    exprs: &'e ExprCache,
    #[cfg_attr(not(feature = "xpath2"), allow(dead_code))]
    exprs2: &'e ExprCache2,
    /// The language every expression of this schema is parsed and evaluated in.
    version: XPathVersion,
    cache: &'e DocumentCache<N>,
}

/// Parsed XPath expressions of one evaluation, keyed by the address of the
/// schema string they come from: every `test`, `<let>` value and
/// `value-of` is parsed once per document rather than once per context
/// node (barrierlab#69). Still parsed on first use, so an expression that
/// is never evaluated cannot fail the run, exactly as before.
#[derive(Default)]
struct ExprCache {
    parsed: RefCell<HashMap<(*const u8, usize), Rc<Expr>>>,
}

impl ExprCache {
    fn get(&self, raw: &str) -> Result<Rc<Expr>, SchematronEvalError> {
        let key = (raw.as_ptr(), raw.len());
        if let Some(expr) = self.parsed.borrow().get(&key) {
            return Ok(Rc::clone(expr));
        }
        let expr = Rc::new(xpath_eval::parse(raw).map_err(SchematronEvalError::Parse)?);
        self.parsed.borrow_mut().insert(key, Rc::clone(&expr));
        Ok(expr)
    }
}

/// [`ExprCache`] for XPath 2.0 mode.
#[derive(Default)]
#[cfg_attr(not(feature = "xpath2"), allow(dead_code))]
struct ExprCache2 {
    parsed: RefCell<HashMap<(*const u8, usize), Rc<Expression>>>,
}

#[cfg(feature = "xpath2")]
impl ExprCache2 {
    fn get(&self, raw: &str) -> Result<Rc<Expression>, SchematronEvalError> {
        let key = (raw.as_ptr(), raw.len());
        if let Some(expr) = self.parsed.borrow().get(&key) {
            return Ok(Rc::clone(expr));
        }
        let expr =
            Rc::new(xpath_eval::parse_versioned(raw, XPathVersion::V2).map_err(from_versioned)?);
        self.parsed.borrow_mut().insert(key, Rc::clone(&expr));
        Ok(expr)
    }
}

/// The value of a `<let>`: an XPath 1.0 value, or in XPath 2.0 mode a sequence.
#[derive(Clone)]
enum Bound<N> {
    V1(Value<N>),
    V2(Sequence<N>),
}

impl<'n, N: Node<'n>> Bound<N> {
    /// The text a `value-of` writes for this value.
    fn to_text(&self) -> String {
        match self {
            Bound::V1(value) => value.to_xpath_string(),
            #[cfg(feature = "xpath2")]
            Bound::V2(sequence) => xpath_eval::sequence_to_string(sequence),
            #[cfg(not(feature = "xpath2"))]
            Bound::V2(_) => unreachable!("XPath 2.0 mode is off"),
        }
    }
}

/// One level of `<let>` bindings (schema, phase, pattern or rule), looked
/// up innermost first. Replaces copying every enclosing binding into a
/// fresh map for each context node (barrierlab#69); a later binding of the
/// same name still wins over an earlier one.
struct Scope<'p, N> {
    bindings: Vec<(String, Bound<N>)>,
    parent: Option<&'p Scope<'p, N>>,
}

impl<'p, N: Clone> Scope<'p, N> {
    fn new(parent: Option<&'p Scope<'p, N>>) -> Self {
        Scope {
            bindings: Vec::new(),
            parent,
        }
    }

    fn get(&self, name: &str) -> Option<Bound<N>> {
        let mut scope = Some(self);
        while let Some(current) = scope {
            if let Some((_, value)) = current.bindings.iter().rev().find(|(n, _)| n == name) {
                return Some(value.clone());
            }
            scope = current.parent;
        }
        None
    }
}

/// Evaluates the XPath expression `raw` against `node`, with
/// `ns_lookup`/`vars` wired up as the namespace/variable hooks — the one
/// place every `<let>`/`value-of/@select` expression evaluation goes
/// through, in the language of the schema's `queryBinding`.
fn evaluate_xpath<'n, N: Node<'n>>(
    raw: &str,
    node: N,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
) -> Result<Bound<N>, SchematronEvalError> {
    match env.version {
        XPathVersion::V1 => {
            let expr = env.exprs.get(raw)?;
            evaluate_parsed(&expr, node, vars, env).map(Bound::V1)
        }
        #[cfg(feature = "xpath2")]
        XPathVersion::V2 => {
            let expr = env.exprs2.get(raw)?;
            with_context(node, vars, env, |ctx| {
                xpath_eval::evaluate_versioned(&expr, ctx)
            })
            .map_err(from_versioned)
            .map(Bound::V2)
        }
        #[cfg(not(feature = "xpath2"))]
        XPathVersion::V2 => {
            unreachable!("queryBinding xslt2 is rejected without the feature xpath2")
        }
    }
}

/// Evaluates an `assert`/`report` test straight to a boolean (see
/// `xpath_eval::evaluate_boolean`; the effective boolean value in XPath 2.0).
fn evaluate_test<'n, N: Node<'n>>(
    raw: &str,
    node: N,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
) -> Result<bool, SchematronEvalError> {
    match env.version {
        XPathVersion::V1 => {
            let expr = env.exprs.get(raw)?;
            with_context(node, vars, env, |ctx| {
                xpath_eval::evaluate_boolean(&expr, ctx)
            })
            .map_err(SchematronEvalError::Eval)
        }
        #[cfg(feature = "xpath2")]
        XPathVersion::V2 => {
            let expr = env.exprs2.get(raw)?;
            with_context(node, vars, env, |ctx| {
                xpath_eval::evaluate_boolean_versioned(&expr, ctx)
            })
            .map_err(from_versioned)
        }
        #[cfg(not(feature = "xpath2"))]
        XPathVersion::V2 => {
            unreachable!("queryBinding xslt2 is rejected without the feature xpath2")
        }
    }
}

fn evaluate_parsed<'n, N: Node<'n>>(
    expr: &Expr,
    node: N,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
) -> Result<Value<N>, SchematronEvalError> {
    with_context(node, vars, env, |ctx| xpath_eval::evaluate(expr, ctx))
        .map_err(SchematronEvalError::Eval)
}

/// Runs `f` against `node` with `vars` and `env` wired up as the variable,
/// namespace and cache hooks (`variables` for 1.0 values, `sequence_variables`
/// for 2.0 sequences).
fn with_context<'n, N: Node<'n>, T>(
    node: N,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
    f: impl FnOnce(&EvaluationContext<'n, '_, N>) -> T,
) -> T {
    let lookup = |name: &QName| {
        if name.prefix.is_some() {
            return None;
        }
        match vars.get(&name.local) {
            Some(Bound::V1(value)) => Some(value),
            _ => None,
        }
    };
    let sequence_lookup = |name: &QName| {
        if name.prefix.is_some() {
            return None;
        }
        match vars.get(&name.local) {
            Some(Bound::V2(sequence)) => Some(sequence),
            _ => None,
        }
    };
    let mut ctx = EvaluationContext::new(node);
    ctx.namespaces = Some(env.ns_lookup);
    ctx.variables = Some(&lookup);
    ctx.sequence_variables = Some(&sequence_lookup);
    ctx.cache = Some(env.cache);
    f(&ctx)
}

/// Evaluates `lets` in document order against `context_node`, adding each
/// binding to `scope` as it goes — so a binding's `value` expression can
/// reference earlier bindings (both from an enclosing scope and from
/// earlier in `lets` itself), but not later ones in `lets`. Scoping:
/// (schema/pattern-level
/// `<let>` evaluate against the document root, rule-level against the
/// matched context node).
fn extend_lets<'n, N: Node<'n>>(
    scope: &mut Scope<'_, N>,
    lets: &[LetBinding],
    context_node: N,
    env: &Env<'_, N>,
) -> Result<(), SchematronEvalError> {
    for binding in lets {
        let value = match &binding.value {
            LetValue::Expr(expr) => evaluate_xpath(expr, context_node, scope, env)?,
            // issue #3: literal-XML content binds as its pre-computed
            // string-value — see `LetValue::Literal`'s doc comment.
            LetValue::Literal(text) => match env.version {
                XPathVersion::V1 => Bound::V1(Value::String(text.clone())),
                XPathVersion::V2 => Bound::V2(vec![Item::Atomic(Atomic::String(text.clone()))]),
            },
        };
        scope.bindings.push((binding.name.clone(), value));
    }
    Ok(())
}

/// Evaluates a single `assert`/`report` check against a claimed context
/// node, returning `Some(Report)` if it fires (see [`Report`]'s doc
/// comment for what "fires" means per [`CheckKind`]), `None` otherwise.
fn evaluate_check<'n, N: Node<'n>>(
    check: &Check,
    node: N,
    pattern_id: Option<String>,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
    diagnostics: &[Diagnostic],
) -> Result<Option<Report<N>>, SchematronEvalError> {
    let boolean = evaluate_test(&check.test, node, vars, env)?;

    let fires = match check.kind {
        CheckKind::Assert => !boolean,
        CheckKind::Report => boolean,
    };

    if !fires {
        return Ok(None);
    }

    let message = render_message(&check.message, node, vars, env)?;

    let rendered_diagnostics = check
        .diagnostics
        .iter()
        .filter_map(|id| diagnostics.iter().find(|d| &d.id == id))
        .map(|d| render_message(&d.message, node, vars, env))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(Report {
        pattern_id,
        check_id: check.id.clone(),
        role: check.role.clone(),
        kind: check.kind,
        message,
        diagnostics: rendered_diagnostics,
        node,
    }))
}

/// Renders a [`Check`]'s message `parts` against `node` — `MessagePart::Text`
/// is appended as-is, `MessagePart::ValueOf`'s `select` is parsed and
/// evaluated against `node` (with the same namespace/variable hooks as the
/// check's own `test`) and its `Value::to_xpath_string()` appended,
/// `MessagePart::Name`'s `path` (or `node` itself, absent a `path`) resolves
/// to the named node's expanded name via XPath's own `name()` function
/// (issue #4 — reuses `xpath-eval`'s existing `name()`/`qname_string`
/// implementation rather than reimplementing qualified-name formatting
/// here) — then the whole concatenated result is trimmed (leading/trailing
/// whitespace only; trimming happens here, not at parse time). Only called for checks that
/// actually fire (see call site) — a broken `value-of`/`name` in a message
/// that never fires must not surface as an error.
fn render_message<'n, N: Node<'n>>(
    parts: &[MessagePart],
    node: N,
    vars: &Scope<'_, N>,
    env: &Env<'_, N>,
) -> Result<String, SchematronEvalError> {
    let mut message = String::new();
    for part in parts {
        match part {
            MessagePart::Text(text) => message.push_str(text),
            MessagePart::ValueOf(select) => {
                let value = evaluate_xpath(select, node, vars, env)?;
                message.push_str(&value.to_text());
            }
            MessagePart::Name(path) => {
                let select = match path {
                    Some(path) => format!("name({path})"),
                    None => "name()".to_owned(),
                };
                let value = match env.version {
                    XPathVersion::V1 => {
                        let expr =
                            xpath_eval::parse(&select).map_err(SchematronEvalError::Parse)?;
                        Bound::V1(evaluate_parsed(&expr, node, vars, env)?)
                    }
                    #[cfg(feature = "xpath2")]
                    XPathVersion::V2 => {
                        let expr = xpath_eval::parse_versioned(&select, XPathVersion::V2)
                            .map_err(from_versioned)?;
                        Bound::V2(
                            with_context(node, vars, env, |ctx| {
                                xpath_eval::evaluate_versioned(&expr, ctx)
                            })
                            .map_err(from_versioned)?,
                        )
                    }
                    #[cfg(not(feature = "xpath2"))]
                    XPathVersion::V2 => {
                        unreachable!("queryBinding xslt2 is rejected without the feature xpath2")
                    }
                };
                message.push_str(&value.to_text());
            }
        }
    }
    Ok(message.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    use xpath_eval::{ExpandedName, NodeKind};

    /// A minimal in-memory element tree, test-only — `xpath-eval`'s own
    /// fixture (`document::fixture`) is `pub(crate)` there and not usable
    /// from this crate, so this is a small, purpose-built equivalent: just
    /// enough (`Root`/`Element`, parent/children, an optional namespace
    /// URI per element) to drive the evaluation-model test matrix below.
    #[derive(Debug)]
    struct ElementData {
        namespace_uri: Option<String>,
        local_name: String,
        parent: Option<usize>,
        children: Vec<usize>,
    }

    #[derive(Debug)]
    struct Arena {
        elements: Vec<ElementData>,
    }

    struct TestDoc {
        arena: Arena,
    }

    impl TestDoc {
        fn node(&self, idx: usize) -> TestNode<'_> {
            TestNode {
                arena: &self.arena,
                idx,
            }
        }
    }

    impl xpath_eval::Document for TestDoc {
        type N<'a> = TestNode<'a>;

        fn root(&self) -> Self::N<'_> {
            self.node(0)
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct TestNode<'a> {
        arena: &'a Arena,
        idx: usize,
    }

    impl<'a> PartialEq for TestNode<'a> {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self.arena, other.arena) && self.idx == other.idx
        }
    }

    impl<'a> Eq for TestNode<'a> {}

    impl<'a> Node<'a> for TestNode<'a> {
        fn kind(self) -> NodeKind {
            if self.idx == 0 {
                NodeKind::Root
            } else {
                NodeKind::Element
            }
        }

        fn parent(self) -> Option<Self> {
            self.arena.elements[self.idx].parent.map(|p| TestNode {
                arena: self.arena,
                idx: p,
            })
        }

        fn children(self) -> impl Iterator<Item = Self> + 'a {
            let arena = self.arena;
            self.arena.elements[self.idx]
                .children
                .clone()
                .into_iter()
                .map(move |i| TestNode { arena, idx: i })
        }

        fn attributes(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }

        fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
            std::iter::empty()
        }

        fn expanded_name(self) -> Option<ExpandedName> {
            if self.idx == 0 {
                None
            } else {
                let element = &self.arena.elements[self.idx];
                Some(ExpandedName {
                    namespace_uri: element.namespace_uri.clone(),
                    local_name: element.local_name.clone(),
                })
            }
        }

        fn string_value(self) -> String {
            String::new()
        }

        fn document_order(self, other: Self) -> Ordering {
            self.idx.cmp(&other.idx)
        }
    }

    /// Builds a document with a root and, for each `(namespace_uri,
    /// local_name)` pair, one direct root child element, in that order.
    fn doc_with_root_children(children: &[(Option<&str>, &str)]) -> TestDoc {
        let mut elements = vec![ElementData {
            namespace_uri: None,
            local_name: String::new(),
            parent: None,
            children: Vec::new(),
        }];
        for (namespace_uri, local_name) in children {
            let idx = elements.len();
            elements.push(ElementData {
                namespace_uri: namespace_uri.map(str::to_owned),
                local_name: (*local_name).to_owned(),
                parent: Some(0),
                children: Vec::new(),
            });
            elements[0].children.push(idx);
        }
        TestDoc {
            arena: Arena { elements },
        }
    }

    fn schema(sch_xml: &str) -> Schema {
        crate::parse(sch_xml).expect("test schema must parse")
    }

    /// Builds `root -> wrapper -> item`, i.e. `item` is a *grandchild* of
    /// the document root, not a direct child like `doc_with_root_children`
    /// produces. Needed to distinguish `descendant-or-self::node()/X` (finds
    /// `item` anywhere) from a bare, unprefixed `X` evaluated relative to
    /// the root (would only find a direct root child literally named `X`) —
    /// every other fixture in this module is flat, which happens to make
    /// both forms equivalent and so cannot catch a union-context bug that
    /// only manifests once a context alternative isn't parenthesized.
    fn doc_with_nested_item() -> TestDoc {
        let elements = vec![
            ElementData {
                namespace_uri: None,
                local_name: String::new(),
                parent: None,
                children: vec![1],
            },
            ElementData {
                namespace_uri: None,
                local_name: "wrapper".to_owned(),
                parent: Some(0),
                children: vec![2],
            },
            ElementData {
                namespace_uri: None,
                local_name: "item".to_owned(),
                parent: Some(1),
                children: Vec::new(),
            },
        ];
        TestDoc {
            arena: Arena { elements },
        }
    }

    #[test]
    fn assert_true_produces_no_report() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="true()">should not fire</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 0);
    }

    #[test]
    fn assert_false_produces_one_report() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern id="p1">
                    <rule context="item">
                        <assert id="a1" role="error" test="false()">item is invalid</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 1);
        let report = &reports[0];
        assert_eq!(report.pattern_id.as_deref(), Some("p1"));
        assert_eq!(report.check_id.as_deref(), Some("a1"));
        assert_eq!(report.role.as_deref(), Some("error"));
        assert_eq!(report.kind, CheckKind::Assert);
        assert_eq!(report.message, "item is invalid");
        assert_eq!(report.node, doc.node(1));
    }

    /// A `context` with a `|`-union of two alternatives must match a node
    /// through *either* alternative, no matter which one is written first —
    /// same as a bare `context="item"` would for the same node. Regression
    /// test for a real bug: `evaluate`/`evaluate_with_phase` built the path
    /// as `format!("descendant-or-self::node()/{}", rule.context)` with no
    /// parentheses around `rule.context`. XPath's `/` binds tighter than
    /// `|`, so `descendant-or-self::node()/A | B` parsed as
    /// `(descendant-or-self::node()/A) | B` — only the *first* alternative
    /// got the document-wide prefix; every later alternative was evaluated
    /// as a bare relative path from the document root, matching only a
    /// direct root child literally named `B`. Every other context-union
    /// test in this module happened to use `doc_with_root_children` (a flat
    /// tree), which made a prefixed and an unprefixed match identical and
    /// so never exposed this — `doc_with_nested_item` puts `item` two
    /// levels down specifically to catch it.
    #[test]
    fn context_union_matches_through_every_alternative() {
        let doc = doc_with_nested_item();

        let first_alternative = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item | does-not-exist">
                        <report test="true()">observed item</report>
                    </rule>
                </pattern>
            </schema>"#,
        );
        let reports = evaluate(&first_alternative, &doc).unwrap();
        assert_eq!(reports.len(), 1, "item as the first union alternative");

        let second_alternative = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="does-not-exist | item">
                        <report test="true()">observed item</report>
                    </rule>
                </pattern>
            </schema>"#,
        );
        let reports = evaluate(&second_alternative, &doc).unwrap();
        assert_eq!(reports.len(), 1, "item as the second union alternative");
    }

    #[test]
    fn evaluate_with_cache_matches_evaluate_across_schemas() {
        let doc = doc_with_nested_item();
        let schemas = [
            schema(
                r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                    <pattern>
                        <rule context="item">
                            <assert test="count(//item) = 2">two items</assert>
                            <report test="ancestor::*">nested item</report>
                        </rule>
                        <rule context="*">
                            <report test="true()">other element</report>
                        </rule>
                    </pattern>
                </schema>"#,
            ),
            schema(
                r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                    <pattern>
                        <rule context="*[not(self::item)]">
                            <report test="//item">has an item</report>
                        </rule>
                    </pattern>
                </schema>"#,
            ),
        ];
        let cache = DocumentCache::new();
        for schema in &schemas {
            let fresh = evaluate(schema, &doc).unwrap();
            let shared = evaluate_with_cache(schema, &doc, &cache).unwrap();
            let summary = |reports: &[Report<_>]| {
                reports
                    .iter()
                    .map(|r| (r.message.clone(), r.node))
                    .collect::<Vec<_>>()
            };
            assert!(!fresh.is_empty());
            assert_eq!(summary(&shared), summary(&fresh));
        }
    }

    #[test]
    fn context_unions_split_only_at_the_top_level() {
        let count = |context: &str| {
            crate::context::CompiledContext::new(context)
                .alternatives(context)
                .unwrap()
                .len()
        };
        assert_eq!(count("item"), 1);
        assert_eq!(count("a | b | c"), 3);
        // A `|` inside a predicate is not a context-level union.
        assert_eq!(count("a[@x = 'p|q']"), 1);
        assert_eq!(count("a[b | c]"), 1);
        assert_eq!(count("(a | b)/c"), 1);
        assert_eq!(count("a[@x = 'p|q'] | b"), 2);
    }

    #[test]
    fn report_true_fires() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <report test="true()">observed item</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].kind, CheckKind::Report);
        assert_eq!(reports[0].message, "observed item");
    }

    /// Two rules in the SAME pattern with overlapping context: only the
    /// first rule's checks run against the shared node (the core of the
    /// "one rule per node per pattern, first match wins" model) — belt
    /// and braces distinctly-recognizable messages so this is proven, not
    /// just plausible from reading the code.
    #[test]
    fn same_pattern_first_matching_rule_wins() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">RULE-ONE-FIRED</assert>
                    </rule>
                    <rule context="item">
                        <assert test="false()">RULE-TWO-FIRED</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["RULE-ONE-FIRED"]);
    }

    /// The counter-case: two PATTERNS with overlapping context both fire
    /// independently for the same node — patterns do not share a claimed
    /// set.
    #[test]
    fn independent_patterns_both_fire() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">PATTERN-ONE-FIRED</assert>
                    </rule>
                </pattern>
                <pattern>
                    <rule context="item">
                        <assert test="false()">PATTERN-TWO-FIRED</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["PATTERN-ONE-FIRED", "PATTERN-TWO-FIRED"]);
    }

    /// A prefixed name test in `context` that would NOT match its target
    /// node without namespace resolution via `Schema.ns` — proves the
    /// `Schema.ns` → `xpath-eval` namespace-hook wiring works end-to-end.
    /// Without it, `t:item`'s prefix falls back to being compared directly
    /// against the real namespace URI `urn:test` (they're unequal), the
    /// context would not match, and no report would fire.
    #[test]
    fn namespace_binding_resolves_prefixed_context() {
        let doc = doc_with_root_children(&[(Some("urn:test"), "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <ns prefix="t" uri="urn:test"/>
                <pattern>
                    <rule context="t:item">
                        <assert test="false()">NS-MATCH</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].message, "NS-MATCH");
    }

    #[test]
    fn invalid_test_expression_is_a_parse_error_not_a_panic() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="((">broken</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let result = evaluate(&schema, &doc);
        assert!(matches!(result, Err(SchematronEvalError::Parse(_))));
    }

    #[test]
    fn schema_level_let_is_visible_to_checks() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <let name="x" value="'schema'"/>
                <pattern>
                    <rule context="item">
                        <report test="$x = 'schema'">SCHEMA-LET</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["SCHEMA-LET"]);
    }

    /// A pattern-level `<let>` with the same name as a schema-level one
    /// shadows it within that pattern — proven positively (the check fires
    /// only if `$x` really is the pattern-level value), not just by absence
    /// of a failure.
    #[test]
    fn pattern_level_let_shadows_schema_level_let() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <let name="x" value="'schema'"/>
                <pattern>
                    <let name="x" value="'pattern'"/>
                    <rule context="item">
                        <report test="$x = 'pattern'">PATTERN-WINS</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["PATTERN-WINS"]);
    }

    /// A rule-level `<let>` referencing the context node is re-evaluated
    /// per matched node, not computed once for the rule — the two root
    /// children have different names, only one matches.
    #[test]
    fn rule_level_let_is_reevaluated_per_context_node() {
        let doc = doc_with_root_children(&[(None, "alpha"), (None, "beta")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="*">
                        <let name="n" value="local-name()"/>
                        <assert test="$n = 'alpha'">NOT-ALPHA</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].message, "NOT-ALPHA");
        assert_eq!(reports[0].node, doc.node(2));
    }

    /// A `<let>` can reference an earlier `<let>` declared in the same
    /// scope (sequential visibility, like sibling `xsl:variable`s).
    #[test]
    fn let_can_reference_earlier_let_in_same_scope() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <let name="a" value="'A'"/>
                <let name="b" value="concat($a, 'B')"/>
                <pattern>
                    <rule context="item">
                        <report test="$b = 'AB'">SEQUENTIAL</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["SEQUENTIAL"]);
    }

    /// Issue #3, end-to-end: a `<let>` bound to literal-XML content is
    /// usable in a `test` expression via its string-value, same as any
    /// other string.
    #[test]
    fn literal_xml_let_is_usable_as_a_string_in_a_test_expression() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <let name="x"><foo>literal</foo></let>
                <pattern>
                    <rule context="item">
                        <report test="$x = 'literal'">LITERAL-LET</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["LITERAL-LET"]);
    }

    #[test]
    fn unbound_variable_is_an_eval_error_not_a_panic() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="$missing">broken</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let result = evaluate(&schema, &doc);
        assert!(matches!(result, Err(SchematronEvalError::Eval(_))));
    }

    /// Two patterns, one phase activating only one of them —
    /// `evaluate_with_phase` fires only the activated pattern's checks.
    #[test]
    fn evaluate_with_phase_restricts_to_activated_patterns() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <phase id="p1">
                    <active pattern="pa"/>
                </phase>
                <phase id="p2">
                    <active pattern="pb"/>
                </phase>
                <pattern id="pa">
                    <rule context="item">
                        <assert test="false()">PATTERN-A</assert>
                    </rule>
                </pattern>
                <pattern id="pb">
                    <rule context="item">
                        <assert test="false()">PATTERN-B</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports_p1 = evaluate_with_phase(&schema, &doc, Some("p1")).unwrap();
        let messages_p1: Vec<&str> = reports_p1.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages_p1, vec!["PATTERN-A"]);

        let reports_p2 = evaluate_with_phase(&schema, &doc, Some("p2")).unwrap();
        let messages_p2: Vec<&str> = reports_p2.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages_p2, vec!["PATTERN-B"]);

        let reports_all = evaluate_with_phase(&schema, &doc, None).unwrap();
        let messages_all: Vec<&str> = reports_all.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages_all, vec!["PATTERN-A", "PATTERN-B"]);
    }

    /// `evaluate()` honors `schema/@defaultPhase` (ISO's `#DEFAULT`) — the
    /// counter-proof to the previous test's `None`/`#ALL` case: with a
    /// `defaultPhase` set, the non-activated pattern does NOT fire, even
    /// though calling `evaluate_with_phase(.., None)` on the same schema
    /// would fire both (proven above).
    #[test]
    fn evaluate_honors_default_phase() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" defaultPhase="p1">
                <phase id="p1">
                    <active pattern="pa"/>
                </phase>
                <pattern id="pa">
                    <rule context="item">
                        <assert test="false()">PATTERN-A</assert>
                    </rule>
                </pattern>
                <pattern id="pb">
                    <rule context="item">
                        <assert test="false()">PATTERN-B</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["PATTERN-A"]);
    }

    #[test]
    fn evaluate_with_phase_unknown_id_is_an_error() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern id="pa">
                    <rule context="item">
                        <assert test="false()">PATTERN-A</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let error = evaluate_with_phase(&schema, &doc, Some("does-not-exist")).unwrap_err();
        assert_eq!(
            error,
            SchematronEvalError::UnknownPhase {
                phase: "does-not-exist".to_owned(),
            }
        );
    }

    #[test]
    fn phase_level_let_is_visible_within_that_phase() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <phase id="p1">
                    <let name="x" value="'phase'"/>
                    <active pattern="pa"/>
                </phase>
                <pattern id="pa">
                    <rule context="item">
                        <report test="$x = 'phase'">PHASE-LET</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate_with_phase(&schema, &doc, Some("p1")).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["PHASE-LET"]);
    }

    /// Regression: a plain-text message (no `value-of`) with leading/
    /// trailing whitespace is still trimmed the same way it was in Stage 1
    /// — trimming just moved from parse time to render time.
    #[test]
    fn plain_text_message_is_trimmed_at_render_time() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            "<schema xmlns=\"http://purl.oclc.org/dsdl/schematron\">\
                <pattern>\
                    <rule context=\"item\">\
                        <assert test=\"false()\">\n  first   second  \n</assert>\
                    </rule>\
                </pattern>\
            </schema>",
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].message, "first   second");
    }

    #[test]
    fn value_of_interpolates_the_context_node() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">Element: <value-of select="local-name()"/>.</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].message, "Element: item.");
    }

    /// Issue #4: a self-closing `<name/>` resolves to the firing check's
    /// own context node's name.
    #[test]
    fn name_resolves_to_the_context_node_name() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">Element <name/> is invalid.</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].message, "Element item is invalid.");
    }

    /// The counter-case: `<name path="...">` resolves to a *different*
    /// node's name than the firing check's own context node.
    #[test]
    fn name_with_path_resolves_to_a_different_node() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">Parent is <name path=".."/>.</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        // The root test node has an empty local name (see `TestNode`/
        // `doc_with_root_children`) — proves `path` is actually evaluated
        // against a different node than `.` (the context node, "item"),
        // not just ignored.
        assert_eq!(reports[0].message, "Parent is .");
    }

    #[test]
    fn value_of_can_reference_a_let_variable() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <let name="x" value="'bound-value'"/>
                        <assert test="false()">x is <value-of select="$x"/></assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].message, "x is bound-value");
    }

    /// A broken `value-of` in a check that never fires must not surface as
    /// an error — the message is only rendered for fired checks.
    #[test]
    fn broken_value_of_in_a_non_firing_check_is_not_an_error() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="true()">broken: <value-of select="(("/></assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 0);
    }

    /// The counter-case: the same broken `value-of` in a check that DOES
    /// fire surfaces as a parse error, not a panic.
    #[test]
    fn broken_value_of_in_a_firing_check_is_a_parse_error() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">broken: <value-of select="(("/></assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let result = evaluate(&schema, &doc);
        assert!(matches!(result, Err(SchematronEvalError::Parse(_))));
    }

    /// End-to-end: an `<extends>`-inlined check fires during evaluation
    /// exactly like one written inline in the rule (parser-level coverage
    /// is in `parser::tests`, this is the evaluation-side integration
    /// check).
    #[test]
    fn extends_inlined_check_fires_during_evaluation() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule abstract="true" id="base">
                        <assert test="false()">FROM-BASE</assert>
                    </rule>
                    <rule context="item">
                        <extends rule="base"/>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["FROM-BASE"]);
    }

    /// End-to-end (issue #7): a `<let>` declared in an abstract rule pulled
    /// in via `<extends>` is visible to the extending rule's own `test`
    /// expressions, not just its inlined checks.
    #[test]
    fn extends_inlined_let_is_visible_to_own_checks() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule abstract="true" id="base">
                        <let name="x" value="'from-base'"/>
                    </rule>
                    <rule context="item">
                        <extends rule="base"/>
                        <report test="$x = 'from-base'">INHERITED-LET</report>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        let messages: Vec<&str> = reports.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(messages, vec!["INHERITED-LET"]);
    }

    /// End-to-end: an `is-a`-instantiated abstract pattern evaluates with
    /// its parameters correctly substituted into `context`/`test`.
    #[test]
    fn is_a_instantiated_pattern_evaluates_with_substituted_context() {
        let doc = doc_with_root_children(&[(None, "alpha"), (None, "beta")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern abstract="true" id="tpl">
                    <rule context="$el">
                        <assert test="false()">MATCHED-$el</assert>
                    </rule>
                </pattern>
                <pattern is-a="tpl" id="concrete">
                    <param name="el" value="alpha"/>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports.len(), 1);
        // The message text itself is plain (unparameterized) content, so
        // it's untouched by substitution — only context/test are.
        assert_eq!(reports[0].message, "MATCHED-$el");
        assert_eq!(reports[0].node, doc.node(1));
    }

    #[test]
    fn fired_check_carries_its_rendered_diagnostics() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <diagnostics>
                    <diagnostic id="d1">first</diagnostic>
                    <diagnostic id="d2">element is <value-of select="local-name()"/></diagnostic>
                </diagnostics>
                <pattern>
                    <rule context="item">
                        <assert test="false()" diagnostics="d1 d2">broken</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].diagnostics, vec!["first", "element is item"]);
    }

    #[test]
    fn unknown_diagnostic_id_is_silently_skipped() {
        // NOTE: this used to be a *parse-time-succeeds* case where an
        // unknown `diagnostics="..."` id was silently dropped at
        // evaluation time. Referential integrity for `diagnostics="..."`
        // is now checked at parse time instead (see `DECISIONS.md` and
        // `parser::tests::diagnostics_unknown_reference_is_an_error`) — an
        // unknown id no longer reaches `evaluate` at all, it's a
        // `ParseError`.
        let schema_xml = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <diagnostics>
                    <diagnostic id="d1">known</diagnostic>
                </diagnostics>
                <pattern>
                    <rule context="item">
                        <assert test="false()" diagnostics="d1 missing">broken</assert>
                    </rule>
                </pattern>
            </schema>"#;

        assert!(crate::parse(schema_xml).is_err());
    }

    #[test]
    fn check_without_diagnostics_attribute_yields_an_empty_list() {
        let doc = doc_with_root_children(&[(None, "item")]);
        let schema = schema(
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                <pattern>
                    <rule context="item">
                        <assert test="false()">broken</assert>
                    </rule>
                </pattern>
            </schema>"#,
        );

        let reports = evaluate(&schema, &doc).unwrap();
        assert_eq!(reports[0].diagnostics, Vec::<String>::new());
    }
}
