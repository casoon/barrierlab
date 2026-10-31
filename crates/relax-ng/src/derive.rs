//! Interned patterns and memoised derivatives — the engine behind
//! [`crate::validate`] (barrierlab#65).
//!
//! The validator never derives [`Pattern`] trees directly. Every pattern it
//! touches is interned into a [`Node`] with a small numeric [`Id`]:
//! structurally equal nodes get the same id, so a derivative that comes out
//! the same as an earlier one *is* the earlier one. That makes three things
//! cheap that were expensive on trees:
//!
//! - comparing and deduplicating patterns (an id comparison),
//! - remembering per-pattern facts (nullability, whether text is wanted,
//!   the closed form of a `group` position) once instead of re-walking,
//! - remembering derivatives: real documents repeat the same element names,
//!   attributes and attribute values against the same content models over
//!   and over.
//!
//! Leaves the schema owns (`element`, `attribute`, `data`, `value`, `list`)
//! are interned by address — they are never built during validation, only
//! referenced — and `ref` by its definition's position, so neither needs a
//! string comparison at validation time.
//!
//! The rules themselves are unchanged from the tree-based validator they
//! replace: same derivative cases, same §6.2.7 handling, same
//! simplifications when building `choice`/`group`/`interleave` (`choice`
//! additionally flattens nested choices and drops duplicate branches, which
//! changes no result: `choice` is associative and idempotent).

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

use crate::datatypes::{DatatypeContext, DatatypeRegistry};
use crate::grammar::{Context, Param, Pattern};
use crate::simplify::{Definitions, name_class_matches};
use crate::validator::ExpandedName;

/// An interned pattern.
pub(crate) type Id = u32;

pub(crate) const EMPTY: Id = 0;
pub(crate) const NOT_ALLOWED: Id = 1;
const TEXT: Id = 2;

/// A pattern as the validator sees it: composites hold ids, leaves point
/// into the compiled schema.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Node<'s> {
    Empty,
    NotAllowed,
    Text,
    Leaf(Leaf<'s>),
    Ref(usize),
    Choice(Rc<[Id]>),
    Group(Rc<[Id]>),
    Interleave(Rc<[Id]>),
    OneOrMore(Id),
}

/// A schema-owned `element`/`attribute`/`data`/`value`/`list` pattern,
/// compared and hashed by address.
#[derive(Clone, Copy)]
struct Leaf<'s>(&'s Pattern);

impl PartialEq for Leaf<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Leaf<'_> {}

impl Hash for Leaf<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

/// Facts about one node, each computed the first time it is asked for.
#[derive(Clone, Copy, Default)]
struct Facts {
    nullable: Option<bool>,
    structurally_nullable: Option<bool>,
    wants_text: Option<bool>,
    accepts_text: Option<bool>,
    closed: Option<Id>,
}

/// One event a pattern is derived by. `group` is sequential for child
/// elements and text, unordered for attributes.
#[derive(Clone, Copy)]
enum Event<'e> {
    Element {
        name: &'e ExpandedName,
        name_id: u32,
    },
    Attribute {
        name: &'e ExpandedName,
        name_id: u32,
        value: &'e str,
        context_id: u32,
        context: &'e DatatypeContext,
    },
    Text {
        text: &'e str,
        context: &'e DatatypeContext,
    },
}

impl Event<'_> {
    fn sequential(&self) -> bool {
        !matches!(self, Event::Attribute { .. })
    }
}

/// Multiply-rotate hashing for maps keyed only by ids and other small
/// integers the interner hands out itself. Maps keyed by document strings
/// (names, attribute values, namespace bindings) keep the standard,
/// collision-resistant hasher.
#[derive(Clone, Copy, Default)]
pub(crate) struct IdHasher(u64);

impl IdHasher {
    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(Self::SEED);
    }
}

impl Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

/// Interned nodes plus every memo table, for one schema and registry.
pub(crate) struct Interner<'s> {
    definitions: &'s Definitions,
    registry: &'s DatatypeRegistry,
    nodes: Vec<Node<'s>>,
    facts: Vec<Facts>,
    ids: IdMap<Node<'s>, Id>,
    /// Schema patterns already interned, by address.
    schema_ids: IdMap<usize, Id>,
    /// Each definition's interned body, by position.
    ref_targets: Vec<Option<Id>>,
    names: HashMap<ExpandedName, u32>,
    contexts: HashMap<Vec<(String, String)>, u32>,
    element_derivs: IdMap<(Id, u32), Id>,
    element_contents: IdMap<(Id, u32), Option<Id>>,
    attribute_known: IdMap<(Id, u32), bool>,
    attribute_derivs: IdMap<(Id, u32, u32), HashMap<Box<str>, Id>>,
}

impl<'s> Interner<'s> {
    pub(crate) fn new(definitions: &'s Definitions, registry: &'s DatatypeRegistry) -> Self {
        let mut interner = Self {
            definitions,
            registry,
            nodes: Vec::new(),
            facts: Vec::new(),
            ids: IdMap::default(),
            schema_ids: IdMap::default(),
            ref_targets: vec![None; definitions.len()],
            names: HashMap::new(),
            contexts: HashMap::new(),
            element_derivs: IdMap::default(),
            element_contents: IdMap::default(),
            attribute_known: IdMap::default(),
            attribute_derivs: IdMap::default(),
        };
        for node in [Node::Empty, Node::NotAllowed, Node::Text] {
            interner.node_id(node);
        }
        interner
    }

    fn node_id(&mut self, node: Node<'s>) -> Id {
        if let Some(&id) = self.ids.get(&node) {
            return id;
        }
        let id = Id::try_from(self.nodes.len()).expect("fewer than 2^32 distinct patterns");
        self.nodes.push(node.clone());
        self.facts.push(Facts::default());
        self.ids.insert(node, id);
        id
    }

    fn node(&self, id: Id) -> Node<'s> {
        self.nodes[id as usize].clone()
    }

    /// The id for a document name, so memo keys stay small.
    pub(crate) fn name_id(&mut self, name: &ExpandedName) -> u32 {
        if let Some(&id) = self.names.get(name) {
            return id;
        }
        let id = u32::try_from(self.names.len()).expect("fewer than 2^32 distinct names");
        self.names.insert(name.clone(), id);
        id
    }

    /// The id for an element's namespace bindings: attribute derivatives
    /// are only reused under the same bindings, since `QName`-typed values
    /// resolve against them.
    pub(crate) fn context_id(&mut self, bindings: &[(String, String)]) -> u32 {
        if let Some(&id) = self.contexts.get(bindings) {
            return id;
        }
        let id = u32::try_from(self.contexts.len()).expect("fewer than 2^32 contexts");
        self.contexts.insert(bindings.to_vec(), id);
        id
    }

    /// Interns a pattern of the compiled schema.
    pub(crate) fn intern(&mut self, pattern: &'s Pattern) -> Id {
        let address = std::ptr::from_ref(pattern) as usize;
        if let Some(&id) = self.schema_ids.get(&address) {
            return id;
        }
        let id = match pattern {
            Pattern::Empty => EMPTY,
            Pattern::NotAllowed => NOT_ALLOWED,
            Pattern::Text => TEXT,
            Pattern::Element { .. }
            | Pattern::Attribute { .. }
            | Pattern::Data { .. }
            | Pattern::Value { .. }
            | Pattern::List(_) => self.node_id(Node::Leaf(Leaf(pattern))),
            Pattern::Group(values) => {
                let ids = values.iter().map(|value| self.intern(value)).collect();
                self.group(ids)
            }
            Pattern::Interleave(values) => {
                let ids = values.iter().map(|value| self.intern(value)).collect();
                self.interleave(ids)
            }
            Pattern::Choice(values) => {
                let ids = values.iter().map(|value| self.intern(value)).collect();
                self.choice(ids)
            }
            Pattern::OneOrMore(values) => {
                let inner = self.intern(&values[0]);
                if inner == NOT_ALLOWED {
                    NOT_ALLOWED
                } else {
                    self.node_id(Node::OneOrMore(inner))
                }
            }
            Pattern::Ref(name) => {
                let position = self
                    .definitions
                    .position(name)
                    .expect("RELAX NG §4.18 guarantees every ref target exists");
                self.node_id(Node::Ref(position))
            }
            Pattern::Optional(_)
            | Pattern::ZeroOrMore(_)
            | Pattern::Mixed(_)
            | Pattern::ExternalRef { .. }
            | Pattern::ParentRef(_)
            | Pattern::Grammar(_) => {
                unreachable!("resolved into other patterns by simplify_pattern")
            }
        };
        self.schema_ids.insert(address, id);
        id
    }

    /// A `ref`'s target. Following `ref` never loops: RELAX NG §4.19,
    /// checked at compile time, rules out a `ref` chain reaching itself
    /// without crossing an `element`, and `element` is a leaf here.
    fn ref_target(&mut self, position: usize) -> Id {
        if let Some(id) = self.ref_targets[position] {
            return id;
        }
        let id = self.intern(self.definitions.pattern_at(position));
        self.ref_targets[position] = Some(id);
        id
    }

    /// `choice` without `notAllowed` branches, nested choices flattened and
    /// duplicates dropped (first occurrence kept); a single branch stands
    /// for itself.
    fn choice(&mut self, values: Vec<Id>) -> Id {
        // Short lists are checked linearly; HTML's content-model choices
        // run to over a hundred branches, so longer ones use a set.
        const LINEAR: usize = 16;
        let mut branches: Vec<Id> = Vec::with_capacity(values.len());
        let mut seen: IdMap<Id, ()> = IdMap::default();
        let mut add = |branch: Id| {
            if branches.len() < LINEAR {
                if !branches.contains(&branch) {
                    branches.push(branch);
                }
            } else {
                if seen.is_empty() {
                    seen.extend(branches.iter().map(|&known| (known, ())));
                }
                if seen.insert(branch, ()).is_none() {
                    branches.push(branch);
                }
            }
        };
        for value in values {
            match &self.nodes[value as usize] {
                Node::NotAllowed => {}
                Node::Choice(inner) => inner.iter().for_each(|&branch| add(branch)),
                _ => add(value),
            }
        }
        match branches.len() {
            0 => NOT_ALLOWED,
            1 => branches[0],
            _ => self.node_id(Node::Choice(branches.into())),
        }
    }

    fn group(&mut self, values: Vec<Id>) -> Id {
        self.sequence(values, Node::Group)
    }

    fn interleave(&mut self, values: Vec<Id>) -> Id {
        self.sequence(values, Node::Interleave)
    }

    /// `group`/`interleave` as simplification builds them: any
    /// `notAllowed` operand makes the whole `notAllowed`, `empty` operands
    /// are dropped when others remain, and a single operand stands for
    /// itself.
    fn sequence(&mut self, mut values: Vec<Id>, wrap: fn(Rc<[Id]>) -> Node<'s>) -> Id {
        if values.contains(&NOT_ALLOWED) {
            return NOT_ALLOWED;
        }
        if values.len() > 1 {
            values.retain(|&value| value != EMPTY);
        }
        match values.len() {
            0 => EMPTY,
            1 => values[0],
            _ => self.node_id(wrap(values.into())),
        }
    }

    /// Whether `id` accepts having nothing more follow. `data`/`value`
    /// ask the registry whether it accepts the implicit empty string.
    pub(crate) fn nullable(&mut self, id: Id) -> bool {
        if let Some(known) = self.facts[id as usize].nullable {
            return known;
        }
        let result = match self.node(id) {
            Node::Empty | Node::Text => true,
            Node::NotAllowed => false,
            Node::Leaf(Leaf(pattern)) => self.leaf_nullable(pattern),
            Node::Choice(values) => values.iter().any(|&value| self.nullable(value)),
            Node::Group(values) | Node::Interleave(values) => {
                values.iter().all(|&value| self.nullable(value))
            }
            Node::OneOrMore(inner) => self.nullable(inner),
            Node::Ref(position) => {
                let target = self.ref_target(position);
                self.nullable(target)
            }
        };
        self.facts[id as usize].nullable = Some(result);
        result
    }

    fn leaf_nullable(&mut self, pattern: &'s Pattern) -> bool {
        match pattern {
            Pattern::Element { .. } | Pattern::Attribute { .. } => false,
            Pattern::Data {
                datatype,
                datatype_library,
                params,
                except,
                ..
            } => {
                let empty = DatatypeContext::empty();
                self.data_matches(datatype, datatype_library, params, "", &empty)
                    && !self.text_matches_except(except, "", &empty)
            }
            Pattern::Value {
                value,
                datatype,
                datatype_library,
                namespace,
                context,
            } => self.value_equals(
                value,
                datatype,
                datatype_library,
                namespace,
                context,
                "",
                &DatatypeContext::empty(),
            ),
            // A `list` with no text at all behaves like its inner pattern
            // seeing zero tokens: structurally, not rule-3-nullable — a
            // bare `data`/`value` slot still needs an actual token.
            Pattern::List(values) => {
                let inner = self.intern(&values[0]);
                self.structurally_nullable(inner)
            }
            _ => unreachable!("only schema leaves are interned as leaves"),
        }
    }

    /// Whether `id` is satisfied by *zero* events in the strict structural
    /// sense: unlike [`Self::nullable`], a bare `data`/`value`/`list` needs
    /// one text event to occur at all, even if its datatype would accept
    /// the empty string. That narrower rule (§6.2.7 rule 3) belongs only
    /// at the top level of an element or attribute; this one is what rule
    /// 2 needs, what decides whether a `list` has enough tokens, and what
    /// gates `group`'s sequential "skip a satisfied prefix". An
    /// `attribute` counts as satisfied: attributes are matched separately
    /// from child and token sequencing.
    pub(crate) fn structurally_nullable(&mut self, id: Id) -> bool {
        if let Some(known) = self.facts[id as usize].structurally_nullable {
            return known;
        }
        let result = match self.node(id) {
            Node::Empty | Node::Text => true,
            Node::NotAllowed => false,
            Node::Leaf(Leaf(pattern)) => match pattern {
                Pattern::Attribute { .. } => true,
                Pattern::List(values) => {
                    let inner = self.intern(&values[0]);
                    self.structurally_nullable(inner)
                }
                _ => false,
            },
            Node::Choice(values) => values
                .iter()
                .any(|&value| self.structurally_nullable(value)),
            Node::Group(values) | Node::Interleave(values) => values
                .iter()
                .all(|&value| self.structurally_nullable(value)),
            Node::OneOrMore(inner) => self.structurally_nullable(inner),
            Node::Ref(position) => {
                let target = self.ref_target(position);
                self.structurally_nullable(target)
            }
        };
        self.facts[id as usize].structurally_nullable = Some(result);
        result
    }

    /// Whether some *reachable* position in `id` is a text-desiring leaf
    /// (`text`/`data`/`value`/`list`) that a whitespace-only event, once it
    /// fails to match, must count as a real candidate for rather than be
    /// dropped as insignificant whitespace (§6.2.7). A `choice` refuses to
    /// drop only if *every* branch wants text — one that doesn't is where
    /// the whitespace harmlessly falls through. `interleave` and `group`
    /// need every operand satisfied, so one operand wanting text is
    /// enough; `group` only looks as far as its satisfied prefix reaches.
    pub(crate) fn wants_text(&mut self, id: Id) -> bool {
        if let Some(known) = self.facts[id as usize].wants_text {
            return known;
        }
        let result = match self.node(id) {
            Node::Text => true,
            Node::Empty | Node::NotAllowed => false,
            Node::Leaf(Leaf(pattern)) => matches!(
                pattern,
                Pattern::Data { .. } | Pattern::Value { .. } | Pattern::List(_)
            ),
            Node::Choice(values) => values.iter().all(|&value| self.wants_text(value)),
            Node::Interleave(values) => values.iter().any(|&value| self.wants_text(value)),
            Node::Group(values) => {
                let mut found = false;
                for &value in values.iter() {
                    found = found || self.wants_text(value);
                    if !self.structurally_nullable(value) {
                        break;
                    }
                }
                found
            }
            Node::OneOrMore(inner) => self.wants_text(inner),
            Node::Ref(position) => {
                let target = self.ref_target(position);
                self.wants_text(target)
            }
        };
        self.facts[id as usize].wants_text = Some(result);
        result
    }

    /// Whether a text event could match anywhere in `id` without crossing
    /// an `element`. If not, every text derivative of `id` is `notAllowed`
    /// and need not be computed.
    fn accepts_text(&mut self, id: Id) -> bool {
        if let Some(known) = self.facts[id as usize].accepts_text {
            return known;
        }
        let result = match self.node(id) {
            Node::Text => true,
            Node::Empty | Node::NotAllowed => false,
            Node::Leaf(Leaf(pattern)) => matches!(
                pattern,
                Pattern::Data { .. } | Pattern::Value { .. } | Pattern::List(_)
            ),
            Node::Choice(values) | Node::Group(values) | Node::Interleave(values) => {
                values.iter().any(|&value| self.accepts_text(value))
            }
            Node::OneOrMore(inner) => self.accepts_text(inner),
            Node::Ref(position) => {
                let target = self.ref_target(position);
                self.accepts_text(target)
            }
        };
        self.facts[id as usize].accepts_text = Some(result);
        result
    }

    /// The closed form of a `group` position a later position's event was
    /// matched past: standard concatenation-derivative rule, D(A·B)'s
    /// "skip A" branch is D(B), so A is fixed to `empty` and cannot match
    /// more child events later. `attribute` positions stay as they are:
    /// they are matched separately, and closing one would mark a missing
    /// attribute as present.
    fn closed(&mut self, id: Id) -> Id {
        if let Some(known) = self.facts[id as usize].closed {
            return known;
        }
        let result = match self.node(id) {
            Node::Leaf(Leaf(Pattern::Attribute { .. })) => id,
            Node::Group(values) => {
                let closed = values.iter().map(|&value| self.closed(value)).collect();
                self.group(closed)
            }
            Node::Interleave(values) => {
                let closed = values.iter().map(|&value| self.closed(value)).collect();
                self.interleave(closed)
            }
            Node::Choice(values) => {
                let closed = values.iter().map(|&value| self.closed(value)).collect();
                self.choice(closed)
            }
            Node::OneOrMore(inner) => self.closed(inner),
            Node::Ref(position) => {
                let target = self.ref_target(position);
                self.closed(target)
            }
            _ => EMPTY,
        };
        self.facts[id as usize].closed = Some(result);
        result
    }

    /// The derivative of `id` by a child element named `name` (just the
    /// name: the child's own content is validated separately).
    pub(crate) fn deriv_element(&mut self, id: Id, name: &ExpandedName) -> Id {
        let name_id = self.name_id(name);
        self.deriv(id, Event::Element { name, name_id })
    }

    /// The derivative of `id` by an attribute. `context_id` must be
    /// [`Self::context_id`] of the bindings `context` was built from.
    pub(crate) fn deriv_attribute(
        &mut self,
        id: Id,
        name: &ExpandedName,
        value: &str,
        context_id: u32,
        context: &DatatypeContext,
    ) -> Id {
        let name_id = self.name_id(name);
        self.deriv(
            id,
            Event::Attribute {
                name,
                name_id,
                value,
                context_id,
                context,
            },
        )
    }

    /// The derivative of `id` by a text value: a child text run, or one
    /// token of a `list`.
    pub(crate) fn deriv_text(&mut self, id: Id, text: &str, context: &DatatypeContext) -> Id {
        self.deriv(id, Event::Text { text, context })
    }

    fn deriv(&mut self, id: Id, event: Event<'_>) -> Id {
        match event {
            Event::Element { name_id, .. } => {
                if let Some(&known) = self.element_derivs.get(&(id, name_id)) {
                    return known;
                }
                let result = self.deriv_uncached(id, event);
                self.element_derivs.insert((id, name_id), result);
                result
            }
            Event::Attribute {
                name_id,
                value,
                context_id,
                ..
            } => {
                let key = (id, name_id, context_id);
                if let Some(&known) = self
                    .attribute_derivs
                    .get(&key)
                    .and_then(|values| values.get(value))
                {
                    return known;
                }
                let result = self.deriv_uncached(id, event);
                self.attribute_derivs
                    .entry(key)
                    .or_default()
                    .insert(value.into(), result);
                result
            }
            Event::Text { .. } => {
                if !self.accepts_text(id) {
                    return NOT_ALLOWED;
                }
                self.deriv_uncached(id, event)
            }
        }
    }

    fn deriv_uncached(&mut self, id: Id, event: Event<'_>) -> Id {
        match self.node(id) {
            Node::Empty | Node::NotAllowed => NOT_ALLOWED,
            // `text` absorbs any amount of text, including separate runs
            // between elements, so it is not used up.
            Node::Text => match event {
                Event::Text { .. } => TEXT,
                _ => NOT_ALLOWED,
            },
            Node::Leaf(Leaf(pattern)) => self.deriv_leaf(pattern, event),
            Node::Choice(values) => {
                let derived = values
                    .iter()
                    .map(|&value| self.deriv(value, event))
                    .collect();
                self.choice(derived)
            }
            Node::Group(values) => self.deriv_positions(&values, event.sequential(), event, false),
            Node::Interleave(values) => self.deriv_positions(&values, false, event, true),
            // Consume one occurrence now, then either continue or stop.
            Node::OneOrMore(inner) => {
                let derived = self.deriv(inner, event);
                let rest = self.choice(vec![id, EMPTY]);
                self.group(vec![derived, rest])
            }
            Node::Ref(position) => {
                let may_match = match event {
                    Event::Element { name, .. } => {
                        self.definitions.may_reach_element_at(position, &name.local)
                    }
                    Event::Attribute { name, .. } => self
                        .definitions
                        .may_reach_attribute_at(position, &name.local),
                    Event::Text { .. } => true,
                };
                if !may_match {
                    return NOT_ALLOWED;
                }
                let target = self.ref_target(position);
                self.deriv(target, event)
            }
        }
    }

    /// Derives every operand position `event` can match at, rebuilding the
    /// container around the one that changed. If `sequential` (`group` for
    /// child content), a position is only reachable once every earlier one
    /// is structurally satisfied, and those earlier ones are closed in the
    /// result. Otherwise (`interleave`, and `group` for attributes, which
    /// have no order) every position stays open.
    fn deriv_positions(
        &mut self,
        values: &[Id],
        sequential: bool,
        event: Event<'_>,
        interleave: bool,
    ) -> Id {
        let mut alternatives = Vec::new();
        for (index, &value) in values.iter().enumerate() {
            let derived = self.deriv(value, event);
            if derived != NOT_ALLOWED {
                let mut branch = values.to_vec();
                if sequential {
                    for earlier in branch.iter_mut().take(index) {
                        *earlier = self.closed(*earlier);
                    }
                }
                branch[index] = derived;
                alternatives.push(if interleave {
                    self.interleave(branch)
                } else {
                    self.group(branch)
                });
            }
            if sequential && !self.structurally_nullable(value) {
                break;
            }
        }
        self.choice(alternatives)
    }

    fn deriv_leaf(&mut self, pattern: &'s Pattern, event: Event<'_>) -> Id {
        let matched = match (pattern, event) {
            (
                Pattern::Element {
                    name: name_class, ..
                },
                Event::Element { name, .. },
            ) => name_class_matches(name_class, &name.namespace, &name.local),
            (
                Pattern::Attribute {
                    name: name_class,
                    body,
                    ..
                },
                Event::Attribute {
                    name,
                    value,
                    context,
                    ..
                },
            ) => {
                name_class_matches(name_class, &name.namespace, &name.local) && {
                    let body = self.intern(&body[0]);
                    self.value_matches(body, value, context)
                }
            }
            (
                Pattern::Data {
                    datatype,
                    datatype_library,
                    params,
                    except,
                    ..
                },
                Event::Text { text, context },
            ) => {
                self.data_matches(datatype, datatype_library, params, text, context)
                    && !self.text_matches_except(except, text, context)
            }
            (
                Pattern::Value {
                    value,
                    datatype,
                    datatype_library,
                    namespace,
                    context: schema_context,
                },
                Event::Text { text, context },
            ) => self.value_equals(
                value,
                datatype,
                datatype_library,
                namespace,
                schema_context,
                text,
                context,
            ),
            (Pattern::List(values), Event::Text { text, context }) => {
                let inner = self.intern(&values[0]);
                self.list_matches(inner, text, context)
            }
            _ => false,
        };
        if matched { EMPTY } else { NOT_ALLOWED }
    }

    fn data_matches(
        &self,
        datatype: &str,
        datatype_library: &Option<String>,
        params: &[Param],
        text: &str,
        context: &DatatypeContext,
    ) -> bool {
        let library = datatype_library.as_deref().unwrap_or("");
        self.registry
            .get(library)
            .expect("checked upfront by check_datatypes_supported")
            .matches(datatype, &param_pairs(params), text, context)
    }

    #[allow(clippy::too_many_arguments)]
    fn value_equals(
        &self,
        value: &str,
        datatype: &Option<String>,
        datatype_library: &Option<String>,
        namespace: &Option<String>,
        schema_context: &Context,
        text: &str,
        context: &DatatypeContext,
    ) -> bool {
        let library = datatype_library.as_deref().unwrap_or("");
        let type_name = datatype.as_deref().unwrap_or("token");
        self.registry
            .get(library)
            .expect("checked upfront by check_datatypes_supported")
            .values_equal(
                type_name,
                value,
                &schema_value_context(namespace, schema_context),
                text,
                context,
            )
    }

    fn text_matches_except(
        &mut self,
        except: &'s [Pattern],
        text: &str,
        context: &DatatypeContext,
    ) -> bool {
        except.iter().any(|item| {
            let item = self.intern(item);
            let derived = self.deriv_text(item, text, context);
            self.nullable(derived)
        })
    }

    /// Whether `text`, split on XML whitespace, matches `id` (a `list`'s
    /// inner pattern) token by token. Ends with structural nullability, not
    /// [`Self::nullable`]: a slot no token reached is not excused because
    /// its datatype would also accept the empty string.
    fn list_matches(&mut self, id: Id, text: &str, context: &DatatypeContext) -> bool {
        let mut current = id;
        for token in text.split_ascii_whitespace() {
            current = self.deriv_text(current, token, context);
            if current == NOT_ALLOWED {
                return false;
            }
        }
        self.structurally_nullable(current)
    }

    /// Whether `value` (an attribute value) satisfies `id` on its own,
    /// including §6.2.7: a whitespace-only or empty value that doesn't
    /// otherwise fit still matches if nothing in `id` wants text (e.g.
    /// `attribute id { empty }` accepts `id=" "`).
    fn value_matches(&mut self, id: Id, value: &str, context: &DatatypeContext) -> bool {
        let derived = self.deriv_text(id, value, context);
        if derived != NOT_ALLOWED && self.structurally_nullable(derived) {
            return true;
        }
        value.trim().is_empty() && !self.wants_text(id)
    }

    /// Whether some `attribute` reachable in `id` (through the same
    /// positions an attribute derivative looks at) has a name class
    /// matching `name`, whatever the value: tells "not allowed here" apart
    /// from "allowed, but wrong value".
    pub(crate) fn attribute_name_known(&mut self, id: Id, name: &ExpandedName) -> bool {
        let name_id = self.name_id(name);
        self.attribute_known_by_id(id, name, name_id)
    }

    fn attribute_known_by_id(&mut self, id: Id, name: &ExpandedName, name_id: u32) -> bool {
        if let Some(&known) = self.attribute_known.get(&(id, name_id)) {
            return known;
        }
        let result = match self.node(id) {
            Node::Leaf(Leaf(Pattern::Attribute {
                name: name_class, ..
            })) => name_class_matches(name_class, &name.namespace, &name.local),
            Node::Group(values) | Node::Interleave(values) | Node::Choice(values) => values
                .iter()
                .any(|&value| self.attribute_known_by_id(value, name, name_id)),
            Node::OneOrMore(inner) => self.attribute_known_by_id(inner, name, name_id),
            Node::Ref(position) => {
                self.definitions
                    .may_reach_attribute_at(position, &name.local)
                    && {
                        let target = self.ref_target(position);
                        self.attribute_known_by_id(target, name, name_id)
                    }
            }
            _ => false,
        };
        self.attribute_known.insert((id, name_id), result);
        result
    }

    /// The union (a `choice`) of the content of every `element` reachable
    /// in `id` whose name class matches `name` — through the same positions
    /// an element derivative looks at — or `None` if there is none.
    pub(crate) fn element_contents(&mut self, id: Id, name: &ExpandedName) -> Option<Id> {
        let name_id = self.name_id(name);
        self.element_contents_by_id(id, name, name_id)
    }

    fn element_contents_by_id(&mut self, id: Id, name: &ExpandedName, name_id: u32) -> Option<Id> {
        if let Some(&known) = self.element_contents.get(&(id, name_id)) {
            return known;
        }
        let mut found = Vec::new();
        match self.node(id) {
            Node::Leaf(Leaf(Pattern::Element {
                name: name_class,
                body,
                ..
            })) => {
                if name_class_matches(name_class, &name.namespace, &name.local) {
                    found.push(self.intern(&body[0]));
                }
            }
            Node::Choice(values) | Node::Interleave(values) => {
                for &value in values.iter() {
                    found.extend(self.element_contents_by_id(value, name, name_id));
                }
            }
            Node::Group(values) => {
                for &value in values.iter() {
                    found.extend(self.element_contents_by_id(value, name, name_id));
                    if !self.structurally_nullable(value) {
                        break;
                    }
                }
            }
            Node::OneOrMore(inner) => {
                found.extend(self.element_contents_by_id(inner, name, name_id));
            }
            Node::Ref(position) if self.definitions.may_reach_element_at(position, &name.local) => {
                let target = self.ref_target(position);
                found.extend(self.element_contents_by_id(target, name, name_id));
            }
            _ => {}
        }
        let result = (!found.is_empty()).then(|| self.choice(found));
        self.element_contents.insert((id, name_id), result);
        result
    }
}

pub(crate) fn param_pairs(params: &[Param]) -> Vec<(&str, &str)> {
    params
        .iter()
        .map(|param| (param.name.as_str(), param.value.as_str()))
        .collect()
}

/// The namespace-resolution context for a `value` pattern's *own* literal
/// text, as written in the schema — used as the "expected" side of
/// `values_equal` for `QName`/`NOTATION`-like types. Deliberately built
/// from the schema, never the document: `namespace` is §4.9's (already
/// inherited) `ns` attribute, used to resolve the literal if it's
/// unprefixed; `schema_context.namespaces` are the schema's own in-scope
/// `xmlns:` bindings, used to resolve it if it has a prefix.
fn schema_value_context(namespace: &Option<String>, schema_context: &Context) -> DatatypeContext {
    let mut bindings: Vec<(String, String)> = schema_context
        .namespaces
        .iter()
        .map(|(prefix, uri)| (prefix.clone(), uri.clone()))
        .collect();
    bindings.push((String::new(), namespace.clone().unwrap_or_default()));
    DatatypeContext::from_bindings(bindings)
}
