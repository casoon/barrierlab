//! Validates a document (via the generic [`Element`] trait) against a
//! [`CompiledSchema`](crate::CompiledSchema) using Brzozowski-derivative
//! pattern matching — the standard approach for RELAX NG validators,
//! derived independently from the pattern algebra in the spec (not ported
//! from any other implementation; see `plan/DECISIONS.md`).
//!
//! # The derivative idea
//!
//! A pattern describes the set of "things" (child sequences, attribute
//! sets, text values) it accepts. Given one such thing has just been seen
//! (an attribute, a child element, a run of text), the *derivative* of the
//! pattern with respect to it is a new pattern describing what must still
//! follow. A pattern is *nullable* if it accepts having nothing more
//! follow at all. Validating a whole element reduces to: derive its
//! content pattern through every attribute (unordered) and then every
//! child in order, and check the result is nullable.
//!
//! The derivatives themselves live in `crate::derive`, on interned
//! patterns. Every one of them transparently follows `ref` — including
//! through `interleave` — which is deliberate: the most serious bug found
//! in the `xmloxide` RELAX NG parser (the reason this crate exists at all,
//! see `plan/DECISIONS.md`) was exactly a `ref` inside `interleave` not
//! being resolved for attribute matching, rejecting almost every attribute
//! in a modular schema. `attribute_matches_through_ref_and_interleave_xmloxide_issue_56`
//! (in the test module) is a standing regression test for that failure
//! mode.
//!
//! # Datatypes
//!
//! `data`/`value` matching is delegated to a [`DatatypeRegistry`] the
//! caller supplies to [`validate`] — this module never hardcodes datatype
//! semantics itself (see `crate::datatypes`). Every `data`/`value`
//! reachable in the schema is checked against the registry once, upfront,
//! before any document is touched, so an unregistered `datatypeLibrary` or
//! an invalid facet fails fast with a single, clear [`DatatypeError`]
//! rather than partway through validation.

use std::collections::BTreeSet;
use std::fmt;

use crate::CompiledSchema;
use crate::datatypes::{DatatypeContext, DatatypeError, DatatypeRegistry};
use crate::derive::{Id, Interner, NOT_ALLOWED, param_pairs};
use crate::grammar::Pattern;
use crate::simplify::Definitions;

/// An expanded (namespace-resolved) name: local name plus an optional
/// namespace URI. `None` means "no namespace" — the caller's document
/// implementation is responsible for resolving any prefixes itself (this
/// crate has no XML/namespace parser of its own).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ExpandedName {
    /// The namespace URI, or `None` for "no namespace".
    pub namespace: Option<String>,
    /// The local (unprefixed) name.
    pub local: String,
}

impl ExpandedName {
    /// Builds an expanded name from a namespace (`None` for "no
    /// namespace") and a local name.
    pub fn new(namespace: Option<impl Into<String>>, local: impl Into<String>) -> Self {
        Self {
            namespace: namespace.map(Into::into),
            local: local.into(),
        }
    }
}

impl fmt::Display for ExpandedName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(namespace) => write!(formatter, "{{{namespace}}}{}", self.local),
            None => formatter.write_str(&self.local),
        }
    }
}

/// One child of an element, in document order. Comments and processing
/// instructions are not part of this model — a correct `Element`
/// implementation simply never yields them here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Content<E> {
    /// A child element.
    Element(E),
    /// A single, non-empty, already-merged run of text — if two text
    /// nodes are adjacent in the underlying document (however "adjacent"
    /// is defined there, e.g. around a comment), the implementation must
    /// merge them into one `Text` before they reach here.
    Text(String),
}

/// The minimal view of a document element this crate needs to validate it.
/// Implement this over whatever tree type the caller already has — no
/// document is parsed by this crate itself.
pub trait Element: Sized {
    /// Whatever shape of location this implementation can report per
    /// element — a formatted `String` (`"line:column"` or an XPath-like
    /// pointer), a structured `{line, column, byte_offset}` struct the
    /// caller defines, or `()` for callers with no location tracking at
    /// all. Carried through unchanged into any [`ValidationError`] raised
    /// against the element, so callers with real position data don't
    /// have to re-parse a formatted string to get it back out — see
    /// [`ValidationError::location`].
    type Location;
    /// This element's expanded name.
    fn name(&self) -> ExpandedName;
    /// This element's attributes, in any order (attribute order is never
    /// significant in XML, and RELAX NG never treats it as such either).
    /// `xmlns`/`xmlns:*` namespace declarations are not attributes for
    /// RELAX NG's purposes and must not be yielded here.
    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)>;
    /// This element's children, in document order.
    fn children(&self) -> impl Iterator<Item = Content<Self>>;
    /// An optional location, included in any [`ValidationError`] raised
    /// against this element when available.
    fn location(&self) -> Option<Self::Location> {
        None
    }
    /// The XML namespace prefix bindings in scope at this element
    /// (`("", uri)` for the default namespace), including any inherited
    /// from ancestors — needed only to resolve `QName`/`NOTATION`-typed
    /// attribute/text *values* (not element/attribute names, which arrive
    /// already-expanded via [`Self::name`]). Default: no bindings, so
    /// `QName`/`NOTATION` values only match when unprefixed and the
    /// default namespace (if any) is what the schema expects.
    fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
        std::iter::empty()
    }
}

/// Validates `root` against `schema` using `registry` to check `data`/
/// `value` patterns. Returns the list of problems found (empty means
/// valid) — or, if the schema uses a `datatypeLibrary`/type/parameter
/// `registry` doesn't recognize, a [`DatatypeError`] *instead of*
/// attempting validation at all, since we can't know whether the document
/// would pass a check we can't perform.
pub fn validate<E: Element>(
    schema: &CompiledSchema,
    registry: &DatatypeRegistry,
    root: &E,
) -> Result<Vec<ValidationError<E::Location>>, DatatypeError> {
    let mut checked_refs = BTreeSet::new();
    check_datatypes_supported(
        &schema.start,
        &schema.definitions,
        registry,
        &mut checked_refs,
    )?;
    for pattern in schema.definitions.values() {
        check_datatypes_supported(pattern, &schema.definitions, registry, &mut checked_refs)?;
    }

    let mut errors = Vec::new();
    let mut interner = Interner::new(&schema.definitions, registry);
    let start = interner.intern(&schema.start);
    match interner.element_contents(start, &root.name()) {
        Some(content) => validate_element_content(&mut interner, content, root, &mut errors),
        None => errors.push(ValidationError {
            kind: ValidationErrorKind::UnexpectedElement(root.name()),
            path: Vec::new(),
            location: root.location(),
        }),
    }
    Ok(errors)
}

/// A single validation problem, with the path of element names from the
/// document root down to (and including) the element it occurred on, and
/// an optional location if [`Element::location`] provided one.
///
/// Generic over `L` (== the reporting [`Element`] implementation's own
/// [`Element::Location`]) rather than hardcoding a formatted `String`, so
/// a caller with real per-node position data (line/column/byte offset,
/// or anything else) gets it back out structured, not as text to
/// re-parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError<L> {
    kind: ValidationErrorKind,
    path: Vec<ExpandedName>,
    location: Option<L>,
}

impl<L> ValidationError<L> {
    /// What kind of problem this is.
    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }
    /// The element names from the document root down to (and including)
    /// the element this problem occurred on.
    pub fn path(&self) -> &[ExpandedName] {
        &self.path
    }
    /// The location [`Element::location`] provided for the affected
    /// element, if any.
    pub fn location(&self) -> Option<&L> {
        self.location.as_ref()
    }
}

impl<L: fmt::Display> fmt::Display for ValidationError<L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.kind)?;
        if let Some(location) = &self.location {
            write!(formatter, " at {location}")?;
        }
        Ok(())
    }
}

impl<L: fmt::Debug + fmt::Display> std::error::Error for ValidationError<L> {}

/// What kind of problem a [`ValidationError`] describes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationErrorKind {
    /// An element with this name isn't allowed at this position.
    UnexpectedElement(ExpandedName),
    /// A text child isn't allowed at this position (and isn't part of a
    /// `data`/`value`/`list` that could accept it).
    UnexpectedText,
    /// This attribute name isn't allowed on this element at all.
    UnexpectedAttribute(ExpandedName),
    /// This attribute is allowed on this element, but its value doesn't
    /// match what the schema requires.
    InvalidAttributeValue(ExpandedName, String),
    /// The element ended while the schema still required more content
    /// (a missing child element, missing text, or a missing attribute).
    MissingContent,
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedElement(name) => write!(formatter, "unexpected element `{name}`"),
            Self::UnexpectedText => formatter.write_str("unexpected text"),
            Self::UnexpectedAttribute(name) => write!(formatter, "unexpected attribute `{name}`"),
            Self::InvalidAttributeValue(name, value) => {
                write!(formatter, "invalid value `{value}` for attribute `{name}`")
            }
            Self::MissingContent => formatter.write_str("missing required content"),
        }
    }
}

/// One element whose content is being validated: its remaining pattern
/// and the children not yet consumed. [`validate_element_content`] keeps
/// these on an explicit stack instead of recursing per nesting level.
///
/// `element` is `None` for the root, which the caller only lends.
struct Frame<E: Element> {
    element: Option<E>,
    pattern: Id,
    context: DatatypeContext,
    children: std::vec::IntoIter<Content<E>>,
    strip_whitespace: bool,
}

/// Validates `root` (whose content must match `content`) and every
/// descendant. Iterative over document depth: each open element is a
/// [`Frame`] on a heap-allocated stack, so a document nested arbitrarily
/// deep cannot overflow the call stack (the pattern walks below still
/// recurse, but only as deep as the schema's own pattern nesting).
fn validate_element_content<E: Element>(
    interner: &mut Interner<'_>,
    content: Id,
    root: &E,
    errors: &mut Vec<ValidationError<E::Location>>,
) {
    let mut path = Vec::new();
    let mut stack = vec![open_frame(interner, content, root, &mut path, errors)];

    while let Some(frame) = stack.last_mut() {
        let Some(child) = frame.children.next() else {
            let frame = stack.pop().expect("the loop just saw a frame");
            if !interner.nullable(frame.pattern) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::MissingContent,
                    path: path.clone(),
                    location: frame.element.as_ref().unwrap_or(root).location(),
                });
            }
            path.pop();
            continue;
        };
        match child {
            Content::Text(text) if frame.strip_whitespace && text.trim().is_empty() => {}
            Content::Text(text) => {
                let derived = interner.deriv_text(frame.pattern, &text, &frame.context);
                if derived == NOT_ALLOWED {
                    // §6.2.7: a text node that consists entirely of
                    // whitespace is treated as if it weren't there at all
                    // when it doesn't otherwise fit the content model —
                    // but *only* if nothing reachable there actually wants
                    // text (`wants_text`); a bare `data`/`value` (or a
                    // `choice` where every branch wants text) still must
                    // see a real, matching event. Non-whitespace text that
                    // doesn't fit is always an error.
                    if !text.trim().is_empty() || interner.wants_text(frame.pattern) {
                        errors.push(ValidationError {
                            kind: ValidationErrorKind::UnexpectedText,
                            path: path.clone(),
                            location: frame.element.as_ref().unwrap_or(root).location(),
                        });
                    }
                } else {
                    frame.pattern = derived;
                }
            }
            Content::Element(child_element) => {
                let name = child_element.name();
                match interner.element_contents(frame.pattern, &name) {
                    Some(child_content) => {
                        // The parent's derivative depends only on the
                        // child's name, not on how its content validates,
                        // so it can be taken before the child is entered.
                        frame.pattern = interner.deriv_element(frame.pattern, &name);
                        let mut child_frame =
                            open_frame(interner, child_content, &child_element, &mut path, errors);
                        child_frame.element = Some(child_element);
                        stack.push(child_frame);
                    }
                    None => errors.push(ValidationError {
                        kind: ValidationErrorKind::UnexpectedElement(name),
                        path: path.clone(),
                        location: child_element.location(),
                    }),
                }
            }
        }
    }
}

/// Enters `element`: pushes its name onto `path`, derives `content`
/// through its (unordered) attributes and collects its children for the
/// ordered pass in [`validate_element_content`]. The frame's `element`
/// is left `None`; the caller moves an owned child in.
fn open_frame<E: Element>(
    interner: &mut Interner<'_>,
    content: Id,
    element: &E,
    path: &mut Vec<ExpandedName>,
    errors: &mut Vec<ValidationError<E::Location>>,
) -> Frame<E> {
    path.push(element.name());
    let mut pattern = content;
    let bindings: Vec<(String, String)> = element.namespace_bindings().collect();
    let context_id = interner.context_id(&bindings);
    let context = DatatypeContext::from_bindings(bindings);

    // Attributes are unordered — fold the derivative over them in
    // whatever order the caller's iterator happens to yield.
    for (name, value) in element.attributes() {
        if !interner.attribute_name_known(pattern, &name) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::UnexpectedAttribute(name),
                path: path.clone(),
                location: element.location(),
            });
            continue;
        }
        let derived = interner.deriv_attribute(pattern, &name, &value, context_id, &context);
        if derived == NOT_ALLOWED {
            errors.push(ValidationError {
                kind: ValidationErrorKind::InvalidAttributeValue(name, value),
                path: path.clone(),
                location: element.location(),
            });
        } else {
            pattern = derived;
        }
    }

    // Children are ordered.
    let children: Vec<_> = element.children().collect();
    // §6.2.7 weak match: next to other children, whitespace-only text is
    // stripped before matching, not offered to the pattern. Offering it
    // lets a `text` further on consume it and so skip an optional element
    // that has to come first — `<fieldset> <legend>` would lose its legend.
    let strip_whitespace = children.len() > 1;
    Frame {
        element: None,
        pattern,
        context,
        children: children.into_iter(),
        strip_whitespace,
    }
}

/// Every `data`/`value` reachable in the schema must use a `datatypeLibrary`
/// `registry` has an implementation for, with a known type name and
/// well-formed, applicable parameters — checked once, upfront, over the
/// *whole* schema (not lazily as validation happens to reach one), so the
/// caller gets a clear, consistent error regardless of which document path
/// would have triggered it.
fn check_datatypes_supported(
    pattern: &Pattern,
    definitions: &Definitions,
    registry: &DatatypeRegistry,
    checked_refs: &mut BTreeSet<String>,
) -> Result<(), DatatypeError> {
    match pattern {
        Pattern::Data {
            datatype,
            datatype_library,
            params,
            except,
            ..
        } => {
            let library = datatype_library.as_deref().unwrap_or("");
            let implementation = registry
                .get(library)
                .ok_or_else(|| DatatypeError::unsupported_library(library))?;
            implementation
                .validate_params(datatype, &param_pairs(params))
                .map_err(|reason| DatatypeError::type_problem(library, datatype, &reason))?;
            for child in except.iter() {
                check_datatypes_supported(child, definitions, registry, checked_refs)?;
            }
            Ok(())
        }
        Pattern::Value {
            datatype,
            datatype_library,
            ..
        } => {
            let library = datatype_library.as_deref().unwrap_or("");
            let type_name = datatype.as_deref().unwrap_or("token");
            let implementation = registry
                .get(library)
                .ok_or_else(|| DatatypeError::unsupported_library(library))?;
            implementation
                .validate_params(type_name, &[])
                .map_err(|reason| DatatypeError::type_problem(library, type_name, &reason))
        }
        Pattern::Element { body, .. } | Pattern::Attribute { body, .. } => {
            body.iter().try_for_each(|child| {
                check_datatypes_supported(child, definitions, registry, checked_refs)
            })
        }
        Pattern::Group(values)
        | Pattern::Interleave(values)
        | Pattern::Choice(values)
        | Pattern::OneOrMore(values)
        | Pattern::List(values) => values.iter().try_for_each(|child| {
            check_datatypes_supported(child, definitions, registry, checked_refs)
        }),
        Pattern::Ref(name) => {
            if checked_refs.insert(name.clone())
                && let Some(target) = definitions.get(name)
            {
                check_datatypes_supported(target, definitions, registry, checked_refs)?;
            }
            Ok(())
        }
        Pattern::Empty | Pattern::NotAllowed | Pattern::Text => Ok(()),
        Pattern::Optional(_)
        | Pattern::ZeroOrMore(_)
        | Pattern::Mixed(_)
        | Pattern::ExternalRef { .. }
        | Pattern::ParentRef(_)
        | Pattern::Grammar(_) => {
            unreachable!("resolved into other patterns by simplify_pattern")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datatypes::DatatypeLibrary;
    use crate::{ResolveError, SchemaResolver, SchemaSource, SchemaSyntax};

    struct NoResolver;

    impl SchemaResolver for NoResolver {
        fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
            Err(ResolveError::new(format!(
                "validator tests don't use includes (requested `{href}`)"
            )))
        }
    }

    fn schema(text: &str) -> CompiledSchema {
        let document = crate::parse(&SchemaSource::new(
            text,
            "memory:/main.rnc",
            SchemaSyntax::Compact,
        ))
        .expect("schema parses");
        crate::simplify(&document, &NoResolver).expect("schema simplifies")
    }

    fn xml_schema(text: &str) -> CompiledSchema {
        let document = crate::parse(&SchemaSource::new(
            text,
            "memory:/main.rng",
            SchemaSyntax::Xml,
        ))
        .expect("schema parses");
        crate::simplify(&document, &NoResolver).expect("schema simplifies")
    }

    fn registry() -> DatatypeRegistry {
        DatatypeRegistry::new()
    }

    /// A minimal, structured (not pre-formatted) location — exercises
    /// [`Element::Location`] as something other than `String`/`()`, the
    /// whole point of that associated type being generic rather than
    /// hardcoded.
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestLocation {
        line: u32,
        column: u32,
    }

    impl fmt::Display for TestLocation {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}:{}", self.line, self.column)
        }
    }

    #[derive(Clone, Debug)]
    struct TestElement {
        name: ExpandedName,
        attributes: Vec<(ExpandedName, String)>,
        children: Vec<Content<TestElement>>,
        namespace_bindings: Vec<(String, String)>,
        location: Option<TestLocation>,
    }

    impl TestElement {
        fn new(local: &str) -> Self {
            Self {
                name: ExpandedName::new(None::<String>, local),
                attributes: Vec::new(),
                children: Vec::new(),
                namespace_bindings: Vec::new(),
                location: None,
            }
        }

        fn new_namespaced(namespace: &str, local: &str) -> Self {
            Self {
                name: ExpandedName::new(Some(namespace), local),
                attributes: Vec::new(),
                children: Vec::new(),
                namespace_bindings: Vec::new(),
                location: None,
            }
        }

        fn at(mut self, line: u32, column: u32) -> Self {
            self.location = Some(TestLocation { line, column });
            self
        }

        fn attr(mut self, local: &str, value: &str) -> Self {
            self.attributes
                .push((ExpandedName::new(None::<String>, local), value.to_owned()));
            self
        }

        fn text(mut self, text: &str) -> Self {
            self.children.push(Content::Text(text.to_owned()));
            self
        }

        fn child(mut self, element: TestElement) -> Self {
            self.children.push(Content::Element(element));
            self
        }

        fn xmlns(mut self, prefix: &str, uri: &str) -> Self {
            self.namespace_bindings
                .push((prefix.to_owned(), uri.to_owned()));
            self
        }
    }

    impl Element for TestElement {
        type Location = TestLocation;
        fn name(&self) -> ExpandedName {
            self.name.clone()
        }
        fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
            self.attributes.clone().into_iter()
        }
        fn children(&self) -> impl Iterator<Item = Content<Self>> {
            self.children.clone().into_iter()
        }
        fn location(&self) -> Option<Self::Location> {
            self.location.clone()
        }
        fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
            self.namespace_bindings.clone().into_iter()
        }
    }

    fn kinds(errors: &[ValidationError<TestLocation>]) -> Vec<ValidationErrorKind> {
        errors.iter().map(|error| error.kind().clone()).collect()
    }

    fn name(local: &str) -> ExpandedName {
        ExpandedName::new(None::<String>, local)
    }

    // --- nullable ---

    /// Whether `pattern` is nullable, interned against `definitions`.
    fn nullable(pattern: &Pattern, definitions: &Definitions) -> bool {
        let registry = registry();
        let mut interner = Interner::new(definitions, &registry);
        let id = interner.intern(pattern);
        interner.nullable(id)
    }

    /// The element content of `compiled`'s `start = element … { content }`.
    fn start_content(compiled: &CompiledSchema) -> &Pattern {
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        &body[0]
    }

    #[test]
    fn nullable_base_cases() {
        let definitions = Definitions::default();
        assert!(nullable(&Pattern::Empty, &definitions));
        assert!(!nullable(&Pattern::NotAllowed, &definitions));
        assert!(nullable(&Pattern::Text, &definitions));
    }

    #[test]
    fn nullable_group_requires_every_operand_nullable() {
        let definitions = Definitions::default();
        assert!(nullable(
            &Pattern::Group(vec![Pattern::Empty, Pattern::Text].into()),
            &definitions
        ));
        assert!(!nullable(
            &Pattern::Group(vec![Pattern::Empty, Pattern::NotAllowed].into()),
            &definitions
        ));
    }

    #[test]
    fn nullable_choice_requires_any_operand_nullable() {
        let definitions = Definitions::default();
        assert!(nullable(
            &Pattern::Choice(vec![Pattern::NotAllowed, Pattern::Empty].into()),
            &definitions
        ));
        assert!(!nullable(
            &Pattern::Choice(vec![Pattern::NotAllowed, Pattern::NotAllowed].into()),
            &definitions
        ));
    }

    #[test]
    fn nullable_one_or_more_delegates_to_inner() {
        let definitions = Definitions::default();
        assert!(nullable(
            &Pattern::OneOrMore(vec![Pattern::Empty].into()),
            &definitions
        ));
        assert!(!nullable(
            &Pattern::OneOrMore(vec![Pattern::NotAllowed].into()),
            &definitions
        ));
    }

    #[test]
    fn nullable_list_delegates_to_inner_same_as_zero_tokens_would() {
        let definitions = Definitions::default();
        // `list { empty }` accepts having no text at all.
        assert!(nullable(
            &Pattern::List(vec![Pattern::Empty].into()),
            &definitions
        ));
        assert!(!nullable(
            &Pattern::List(vec![Pattern::NotAllowed].into()),
            &definitions
        ));
    }

    #[test]
    fn nullable_ref_resolves_through_definitions() {
        let definitions = Definitions::new(std::collections::BTreeMap::from([(
            "x".to_owned(),
            Pattern::Empty,
        )]));
        assert!(nullable(&Pattern::Ref("x".into()), &definitions));
    }

    // --- individual derivative functions: element / attribute / text ---

    #[test]
    fn deriv_element_event_matches_by_name_and_rejects_others() {
        let compiled = schema("start = element foo { empty }");
        let registry = registry();
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let start = interner.intern(&compiled.start);
        let after_foo = interner.deriv_element(start, &name("foo"));
        assert!(interner.nullable(after_foo));
        assert_eq!(interner.deriv_element(start, &name("bar")), NOT_ALLOWED);
    }

    #[test]
    fn deriv_attribute_checks_name_and_value() {
        let compiled = schema(r#"start = element foo { attribute id { "x" } }"#);
        let registry = registry();
        let context = DatatypeContext::empty();
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let context_id = interner.context_id(&[]);
        let content = interner.intern(start_content(&compiled));
        let matched = interner.deriv_attribute(content, &name("id"), "x", context_id, &context);
        assert!(interner.nullable(matched));
        assert_eq!(
            interner.deriv_attribute(content, &name("id"), "y", context_id, &context),
            NOT_ALLOWED
        );
        assert_eq!(
            interner.deriv_attribute(content, &name("other"), "x", context_id, &context),
            NOT_ALLOWED
        );
    }

    #[test]
    fn deriv_text_data_accepts_any_string_value_requires_exact_match() {
        let registry = registry();
        let context = DatatypeContext::empty();
        let compiled = schema("start = element foo { token }");
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        let derived = interner.deriv_text(content, "whatever text", &context);
        assert!(interner.nullable(derived));

        let compiled = schema(r#"start = element foo { "exact" }"#);
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        let derived = interner.deriv_text(content, "exact", &context);
        assert!(interner.nullable(derived));
        assert_eq!(
            interner.deriv_text(content, "different", &context),
            NOT_ALLOWED
        );
    }

    #[test]
    fn deriv_text_token_equality_collapses_whitespace() {
        let registry = registry();
        let context = DatatypeContext::empty();
        let compiled = schema(r#"start = element foo { token "a b" }"#);
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        let derived = interner.deriv_text(content, "a   b", &context);
        assert!(interner.nullable(derived));
    }

    // --- `group` (sequential/"after") vs `interleave` (unordered) ---

    #[test]
    fn group_derivative_is_sequential() {
        let registry = registry();
        let compiled = schema("start = element root { element a { empty }, element b { empty } }");
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        // "b" cannot be matched before "a" — group is ordered.
        assert_eq!(interner.deriv_element(content, &name("b")), NOT_ALLOWED);
        let after_a = interner.deriv_element(content, &name("a"));
        let after_a_b = interner.deriv_element(after_a, &name("b"));
        assert!(interner.nullable(after_a_b));
    }

    #[test]
    fn interleave_derivative_allows_either_order() {
        let registry = registry();
        let compiled = schema("start = element root { element a { empty } & element b { empty } }");
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        // Unlike `group`, "b" is allowed to come before "a".
        let after_b = interner.deriv_element(content, &name("b"));
        assert_ne!(after_b, NOT_ALLOWED);
        let after_b_a = interner.deriv_element(after_b, &name("a"));
        assert!(interner.nullable(after_b_a));
    }

    // --- the xmloxide regression this crate exists to fix (issue #56):
    // `ref` inside `interleave` must resolve for *attribute* matching. ---

    #[test]
    fn attribute_matches_through_ref_and_interleave_xmloxide_issue_56() {
        let compiled = schema(
            "start = element foo { attrs & text }\n\
             attrs = attribute id { text }",
        );
        let document = TestElement::new("foo").attr("id", "x").text("hello");
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }

    #[test]
    fn attribute_name_known_resolves_ref_through_group_and_choice_too() {
        // The same transparency, exercised for `group` (attribute-matching
        // semantics, unordered) and `choice`.
        let compiled = schema(
            "start = element foo { a, b }\n\
             a = attribute x { text } | attribute y { text }\n\
             b = empty",
        );
        let registry = registry();
        let mut interner = Interner::new(&compiled.definitions, &registry);
        let content = interner.intern(start_content(&compiled));
        assert!(interner.attribute_name_known(content, &name("x")));
        assert!(interner.attribute_name_known(content, &name("y")));
        assert!(!interner.attribute_name_known(content, &name("z")));
    }

    #[test]
    fn ref_is_transparent_for_child_elements_through_choice_and_group() {
        let compiled = schema(
            "start = element foo { greeting, farewell }\n\
             greeting = element hello { empty } | element hi { empty }\n\
             farewell = element bye { empty }",
        );
        let document = TestElement::new("foo")
            .child(TestElement::new("hi"))
            .child(TestElement::new("bye"));
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }

    // --- end-to-end `validate()`: hand-picked valid/invalid document pairs ---

    #[test]
    fn valid_document_with_required_attribute_and_text() {
        let compiled = schema("start = element foo { attribute id { text }, text }");
        let document = TestElement::new("foo").attr("id", "1").text("hello");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_required_attribute_is_reported() {
        let compiled = schema("start = element foo { attribute id { text }, text }");
        let document = TestElement::new("foo").text("hello");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::MissingContent]);
    }

    #[test]
    fn unexpected_attribute_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").attr("id", "1");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![ValidationErrorKind::UnexpectedAttribute(name("id"))]
        );
    }

    /// `Element::Location` is generic — a caller with real structured
    /// position data (not just a pre-formatted string) gets it back out
    /// of `ValidationError::location()` unchanged, not re-parsed from
    /// text. `TestLocation` (`{line, column}`) stands in for whatever
    /// shape a real caller's own position type has.
    #[test]
    fn structured_location_is_reported_on_the_validation_error() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").attr("id", "1").at(3, 7);
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].location(),
            Some(&TestLocation { line: 3, column: 7 })
        );
        // `Display` still renders it as formatted text for messages that
        // want that.
        assert_eq!(errors[0].to_string(), "unexpected attribute `id` at 3:7");
    }

    #[test]
    fn invalid_attribute_value_is_reported_distinctly_from_unexpected_attribute() {
        let compiled = schema(r#"start = element foo { attribute id { "x" } }"#);
        let document = TestElement::new("foo").attr("id", "y");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        // The mismatched attribute is left unconsumed (so later attributes
        // keep being checked against the schema rather than being drowned
        // out by one bad value), which is also, truthfully, "required
        // content still missing" once the element ends.
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::InvalidAttributeValue(name("id"), "y".to_owned()),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unexpected_child_element_is_reported() {
        let compiled = schema("start = element foo { element bar { empty } }");
        let document = TestElement::new("foo").child(TestElement::new("baz"));
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::UnexpectedElement(name("baz")),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unexpected_root_element_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("bar");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![ValidationErrorKind::UnexpectedElement(name("bar"))]
        );
    }

    #[test]
    fn unexpected_text_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").text("surprise");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::UnexpectedText]);
    }

    #[test]
    fn one_or_more_children_are_validated_recursively_and_repeatedly() {
        let compiled = schema("start = element foo { element item { text }+ }");
        let document = TestElement::new("foo")
            .child(TestElement::new("item").text("a"))
            .child(TestElement::new("item").text("b"));
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn one_or_more_requires_at_least_one_occurrence() {
        let compiled = schema("start = element foo { element item { empty }+ }");
        let document = TestElement::new("foo");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::MissingContent]);
    }

    #[test]
    fn interleaved_text_and_element_children_validate_regardless_of_order() {
        let compiled = schema("start = element foo { mixed { element bar { empty } } }");
        let document = TestElement::new("foo")
            .text("before ")
            .child(TestElement::new("bar"))
            .text(" after");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn list_datatype_matches_whitespace_separated_tokens() {
        let compiled = schema("start = element foo { list { token+ } }");
        let document = TestElement::new("foo").text("a b   c");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn list_datatype_rejects_a_token_that_does_not_match() {
        let compiled = schema(r#"start = element foo { list { "a", "b" } }"#);
        let document = TestElement::new("foo").text("a c");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unsupported_datatype_library_fails_fast_before_validating() {
        // Deliberately the whattf library, not XSD — `DatatypeRegistry::new`
        // now supports XSD out of the box (see `plan/DECISIONS.md`), so
        // this needs a genuinely unregistered library to still exercise
        // the fail-fast path; the whattf library is exactly that (a
        // caller like html-conform must register its own implementation).
        // XML syntax, not compact: compact-syntax identifiers don't accept
        // `-`, and `non-empty-string` is vnu's real type name.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="non-empty-string" datatypeLibrary="http://whattf.org/datatype-draft"/>
               </element>"#,
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &registry(), &document).unwrap_err();
        assert_eq!(error.library(), "http://whattf.org/datatype-draft");
    }

    #[test]
    fn builtin_only_registry_rejects_xsd_too() {
        let compiled = schema(
            "datatypes xsd = \"http://www.w3.org/2001/XMLSchema-datatypes\"\n\
             start = element foo { xsd:string }",
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &DatatypeRegistry::builtin_only(), &document).unwrap_err();
        assert_eq!(
            error.library(),
            "http://www.w3.org/2001/XMLSchema-datatypes"
        );
    }

    // --- XSD, via the default registry ---

    #[test]
    fn xsd_string_and_integer_types_match() {
        // Note: RNG XML's `type` attribute is always a bare string, never a
        // namespace-prefixed QName — the library comes from the separate
        // `datatypeLibrary` attribute (or §4.3 inheritance from an
        // ancestor), unlike compact syntax's `prefix:typeName` sugar.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <group>
                   <attribute name="count"><data type="positiveInteger"/></attribute>
                   <data type="string"/>
                 </group>
               </element>"#,
        );
        let valid = TestElement::new("foo").attr("count", "3").text("hello");
        assert!(validate(&compiled, &registry(), &valid).unwrap().is_empty());

        let invalid = TestElement::new("foo").attr("count", "0").text("hello");
        let errors = validate(&compiled, &registry(), &invalid).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::InvalidAttributeValue(name("count"), "0".to_owned()),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn xsd_pattern_facet_is_enforced() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <data type="string"><param name="pattern">[a-z]+</param></data>
               </element>"#,
        );
        let valid = TestElement::new("foo").text("abc");
        assert!(validate(&compiled, &registry(), &valid).unwrap().is_empty());
        let invalid = TestElement::new("foo").text("ABC");
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &invalid).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn xsd_decimal_value_equality_ignores_lexical_differences() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" xmlns:xsd="http://www.w3.org/2001/XMLSchema-datatypes" name="foo">
                 <value type="decimal" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">1.50</value>
               </element>"#,
        );
        // `1.5` and `01.50` are the same `decimal` value, just spelled
        // differently — real bignum-style equality, not string equality.
        let document = TestElement::new("foo").text("01.500");
        assert!(
            validate(&compiled, &registry(), &document)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn xsd_qname_resolves_prefix_via_document_namespace_bindings() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="QName" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes"/>
               </element>"#,
        );
        let document = TestElement::new("foo")
            .xmlns("h", "urn:example:h")
            .text("h:widget");
        assert!(
            validate(&compiled, &registry(), &document)
                .unwrap()
                .is_empty()
        );

        // No `h` binding in scope at all — the QName can't be resolved, so
        // it isn't a legal QName *value* here even though it's lexically
        // shaped like one.
        let unresolved = TestElement::new("foo").text("h:widget");
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &unresolved).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn xsd_unknown_type_name_is_a_datatype_error() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <data type="not-a-real-type"/>
               </element>"#,
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &registry(), &document).unwrap_err();
        assert_eq!(
            error.library(),
            "http://www.w3.org/2001/XMLSchema-datatypes"
        );
    }

    // --- external plugin: a dummy library, demonstrating the trait
    // contract a caller like html-conform would implement for vnu's
    // `http://whattf.org/datatype-draft` (not shipped by this crate). ---

    struct DummyNonEmptyStringLibrary;

    impl DatatypeLibrary for DummyNonEmptyStringLibrary {
        fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String> {
            if type_name != "non-empty-string" {
                return Err(format!("unknown dummy type `{type_name}`"));
            }
            if let Some((param, _)) = params.first() {
                return Err(format!(
                    "`non-empty-string` takes no parameters, found `{param}`"
                ));
            }
            Ok(())
        }

        fn matches(
            &self,
            type_name: &str,
            _params: &[(&str, &str)],
            value: &str,
            _context: &DatatypeContext,
        ) -> bool {
            type_name == "non-empty-string" && !value.is_empty()
        }

        fn values_equal(
            &self,
            type_name: &str,
            expected: &str,
            _expected_context: &DatatypeContext,
            actual: &str,
            _actual_context: &DatatypeContext,
        ) -> bool {
            type_name == "non-empty-string" && expected == actual
        }
    }

    #[test]
    fn external_plugin_library_can_be_registered_and_used() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="non-empty-string" datatypeLibrary="http://whattf.org/datatype-draft"/>
               </element>"#,
        );
        let mut registry = DatatypeRegistry::new();
        registry.register(
            "http://whattf.org/datatype-draft",
            DummyNonEmptyStringLibrary,
        );

        let valid = TestElement::new("foo").text("hello");
        assert!(validate(&compiled, &registry, &valid).unwrap().is_empty());

        // An empty text *event* against a bare `data` isn't excused by
        // §6.2.7 weak-match rule 2: that rule only drops a whitespace-only
        // event when the content seen *so far* already, strictly,
        // satisfies the pattern without it — a bare `data`/`value` never
        // does (it structurally requires a real event), so this is
        // reported directly, same as a genuinely non-empty mismatch would
        // be.
        let invalid = TestElement::new("foo").text("");
        assert_eq!(
            kinds(&validate(&compiled, &registry, &invalid).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    // --- adapter smoke test: a realistic html-conform/xmloxide-shaped tree
    // (namespace-qualified elements, an unprefixed attribute, nested
    // children, text) — this crate has no dependency on html-conform or
    // xmloxide (no HTML/XML-parser dependency in the core, see
    // `plan/DECISIONS.md`), so `TestElement` stands in for its sibling
    // project's `NormalizedNode` adapter output, confirming the `Element`
    // trait contract works for that shape before the Public-API phase. ---

    #[test]
    fn adapter_smoke_test_with_a_realistic_xhtml_shaped_tree() {
        const XHTML: &str = "http://www.w3.org/1999/xhtml";
        // Compact-syntax (`.rnc`) prefixed element names are a known,
        // separate gap outside Phase 05's scope (see `plan/00-STATUS.md`)
        // — the XML (`.rng`) syntax resolves `xmlns:`-qualified names
        // correctly, so this realistic, namespace-qualified smoke test
        // uses that syntax instead.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" xmlns:xhtml="http://www.w3.org/1999/xhtml" name="xhtml:html">
                 <group>
                   <element name="xhtml:head"><empty/></element>
                   <element name="xhtml:body">
                     <element name="xhtml:p">
                       <group>
                         <attribute name="id"><text/></attribute>
                         <text/>
                       </group>
                     </element>
                   </element>
                 </group>
               </element>"#,
        );
        let document = TestElement::new_namespaced(XHTML, "html")
            .child(TestElement::new_namespaced(XHTML, "head"))
            .child(
                TestElement::new_namespaced(XHTML, "body").child(
                    TestElement::new_namespaced(XHTML, "p")
                        .attr("id", "intro")
                        .text("hello"),
                ),
            );
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }

    /// A chain of `div`s `remaining` levels deep, generated on the fly
    /// (a boxed test tree would overflow on its own recursive `Drop`),
    /// ending in a `span`.
    struct Chain {
        remaining: usize,
    }

    impl Element for Chain {
        type Location = usize;
        fn name(&self) -> ExpandedName {
            name(if self.remaining == 0 { "span" } else { "div" })
        }
        fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
            std::iter::empty()
        }
        fn children(&self) -> impl Iterator<Item = Content<Self>> {
            (self.remaining > 0)
                .then(|| {
                    Content::Element(Chain {
                        remaining: self.remaining - 1,
                    })
                })
                .into_iter()
        }
        fn location(&self) -> Option<Self::Location> {
            Some(self.remaining)
        }
    }

    /// auditmysite#783: validation recursed once per nesting level and
    /// aborted the process with a stack overflow at a few hundred levels
    /// on a 2 MiB thread (tokio's worker default). It now keeps open
    /// elements on a heap stack; depth costs memory, not call frames.
    #[test]
    fn arbitrarily_deep_documents_do_not_overflow_the_stack() {
        const DEPTH: usize = 100_000;
        let errors = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let compiled = schema("start = d\nd = element div { d? }");
                validate(&compiled, &registry(), &Chain { remaining: DEPTH }).unwrap()
            })
            .unwrap()
            .join()
            .expect("validation must not overflow the stack");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].kind(),
            &ValidationErrorKind::UnexpectedElement(name("span"))
        );
        assert_eq!(errors[0].path().len(), DEPTH);
        assert_eq!(errors[0].location(), Some(&0));
    }
}
