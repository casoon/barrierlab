//! Rule contexts, parsed once at schema load into [`CompiledContext`].
//!
//! A Schematron rule matches the nodes `descendant-or-self::node()/context`
//! selects. The context is parsed with `xpath_eval::parse` and rewritten on
//! the AST, one [`Alternative`] per top-level `|` operand.

use std::borrow::Cow;

use xpath_eval::{
    Axis, Expr, FilterExpr, LocationPath, NodeTest, ParseError, PathExpr, PrimaryExpr, Step,
};

/// One operand of a (possibly unioned) rule context.
#[derive(Clone, Debug)]
pub(crate) enum Alternative {
    /// A single `child::` step whose predicates do not depend on position:
    /// a node matches iff its test and predicates hold for it alone, so it
    /// can be tested while walking the document once.
    Node {
        test: NodeTest,
        predicates: Vec<Expr>,
        /// The whole `self::` step, for node tests matched by evaluation.
        step: Expr,
    },
    /// Anything else: evaluated once, from the document root, to the set of
    /// matching nodes.
    Set(Expr),
}

/// The parsed form of a `Rule.context`. Keeps the source it was parsed from,
/// so a `context` edited after parsing is compiled afresh instead of used
/// stale.
#[derive(Clone, Debug)]
pub(crate) struct CompiledContext {
    source: String,
    alternatives: Result<Vec<Alternative>, ParseError>,
}

/// Compiled contexts of the same source are the same, so schemas compare by
/// their source text alone.
impl PartialEq for CompiledContext {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for CompiledContext {}

impl CompiledContext {
    pub(crate) fn new(context: &str) -> Self {
        CompiledContext {
            source: context.to_owned(),
            alternatives: compile(context),
        }
    }

    /// The alternatives for `context`, the rule's current context string.
    pub(crate) fn alternatives(&self, context: &str) -> Result<Cow<'_, [Alternative]>, ParseError> {
        if self.source == context {
            self.alternatives
                .as_deref()
                .map(Cow::Borrowed)
                .map_err(Clone::clone)
        } else {
            compile(context).map(Cow::Owned)
        }
    }
}

fn compile(context: &str) -> Result<Vec<Alternative>, ParseError> {
    let mut alternatives = Vec::new();
    collect(xpath_eval::parse(context)?, &mut alternatives);
    Ok(alternatives)
}

fn collect(expr: Expr, out: &mut Vec<Alternative>) {
    match expr {
        Expr::Union(left, right) => {
            collect(*left, out);
            collect(*right, out);
        }
        Expr::Path(PathExpr::Location(path)) => out.push(location(path)),
        Expr::Path(PathExpr::Filter(FilterExpr {
            primary: PrimaryExpr::Parenthesized(inner),
            predicates,
        })) if predicates.is_empty() => collect(*inner, out),
        other => out.push(Alternative::Set(other)),
    }
}

fn location(mut path: LocationPath) -> Alternative {
    if path.is_absolute {
        return Alternative::Set(Expr::Path(PathExpr::Location(path)));
    }
    if let [step] = path.steps.as_slice()
        && step.axis == Axis::Child
        && step.predicates.iter().all(is_boolean_valued)
    {
        let step = path.steps.remove(0);
        return Alternative::Node {
            test: step.node_test.clone(),
            predicates: step.predicates.clone(),
            step: Expr::Path(PathExpr::Location(LocationPath {
                is_absolute: false,
                steps: vec![Step {
                    axis: Axis::SelfAxis,
                    ..step
                }],
            })),
        };
    }
    path.steps.insert(
        0,
        Step {
            axis: Axis::DescendantOrSelf,
            node_test: NodeTest::Node,
            predicates: Vec::new(),
        },
    );
    Alternative::Set(Expr::Path(PathExpr::Location(path)))
}

/// Whether a predicate is certainly a boolean test, never a position: a
/// number-valued predicate means `position() = n`, and `position()`/`last()`
/// see the step's context size, which a per-node test does not have.
fn is_boolean_valued(predicate: &Expr) -> bool {
    let boolean_shaped = match predicate {
        Expr::Or(..) | Expr::And(..) | Expr::Equality(..) | Expr::Relational(..) => true,
        Expr::Path(PathExpr::Location(_)) => true,
        Expr::Path(PathExpr::Filter(FilterExpr {
            primary: PrimaryExpr::Function(call),
            predicates,
        })) => {
            predicates.is_empty()
                && call.name.prefix.is_none()
                && matches!(
                    call.name.local.as_str(),
                    "not" | "boolean" | "contains" | "starts-with" | "true" | "false" | "lang"
                )
        }
        _ => false,
    };
    boolean_shaped && !mentions_position(predicate)
}

fn mentions_position(expr: &Expr) -> bool {
    match expr {
        Expr::Or(a, b) | Expr::And(a, b) | Expr::Union(a, b) => {
            mentions_position(a) || mentions_position(b)
        }
        Expr::Equality(a, _, b)
        | Expr::Relational(a, _, b)
        | Expr::Additive(a, _, b)
        | Expr::Multiplicative(a, _, b) => mentions_position(a) || mentions_position(b),
        Expr::Negate(a) => mentions_position(a),
        Expr::Path(PathExpr::Location(path)) => location_mentions_position(path),
        Expr::Path(PathExpr::Filter(filter)) => filter_mentions_position(filter),
        Expr::Path(PathExpr::FilterLocation(filter, path)) => {
            filter_mentions_position(filter) || location_mentions_position(path)
        }
    }
}

fn location_mentions_position(path: &LocationPath) -> bool {
    path.steps
        .iter()
        .any(|step| step.predicates.iter().any(mentions_position))
}

fn filter_mentions_position(filter: &FilterExpr) -> bool {
    filter.predicates.iter().any(mentions_position)
        || match &filter.primary {
            PrimaryExpr::Parenthesized(inner) => mentions_position(inner),
            PrimaryExpr::Function(call) => {
                matches!(call.name.local.as_str(), "position" | "last")
                    || call.args.iter().any(mentions_position)
            }
            PrimaryExpr::Variable(_) | PrimaryExpr::Literal(_) | PrimaryExpr::Number(_) => false,
        }
}
