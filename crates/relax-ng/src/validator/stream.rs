//! Push validation: the same derivative steps as the tree walk in the
//! parent module, driven by events instead of an [`Element`](super::Element)
//! tree. See `docs/packages/relax-ng.md` (section "Streaming validation")
//! for the event model and its limits.

use std::collections::BTreeSet;
use std::fmt;

use super::{ExpandedName, ValidationError, ValidationErrorKind, check_datatypes_supported};
use crate::CompiledSchema;
use crate::datatypes::{DatatypeContext, DatatypeError, DatatypeRegistry};
use crate::derive::{Id, Interner, NOT_ALLOWED};

/// A misuse of [`StreamValidator`]: the event sequence is not a single
/// well-formed element. Never a finding about the document's validity —
/// those are [`ValidationError`]s. After a `StreamError` the validator's
/// state is unspecified; discard it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamError {
    /// `end_element` without a matching `start_element`.
    UnbalancedEnd,
    /// A second root element started after the first one was closed.
    SecondRoot,
    /// Text other than whitespace outside the root element.
    TextOutsideRoot,
    /// `finish` before the root element was started and closed.
    Unfinished,
}

impl fmt::Display for StreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnbalancedEnd => "end_element without a matching start_element",
            Self::SecondRoot => "a second root element started",
            Self::TextOutsideRoot => "text outside the root element",
            Self::Unfinished => "the root element was not started and closed",
        })
    }
}

impl std::error::Error for StreamError {}

/// One open element: its remaining pattern and the text collected since
/// the last child boundary.
struct Frame<L> {
    pattern: Id,
    context: DatatypeContext,
    location: Option<L>,
    /// Text since the last child boundary, not yet offered to the pattern:
    /// a `data`/`value`/`list` needs the complete run.
    pending: String,
    /// Children (elements and flushed text runs) seen so far.
    children: usize,
}

/// A push validator: validates one document from `start_element` /
/// `text` / `end_element` events, without a document tree. The findings
/// are the same [`ValidationError`]s, in the same order, as
/// [`validate`](crate::validate) reports for the equivalent tree.
///
/// `L` is the location type carried into findings (as
/// [`Element::Location`](super::Element::Location) in the tree path).
pub struct StreamValidator<'s, L> {
    interner: Interner<'s>,
    start: Id,
    stack: Vec<Frame<L>>,
    path: Vec<ExpandedName>,
    errors: Vec<ValidationError<L>>,
    /// Depth inside an unexpected element's subtree, which is skipped.
    skipping: usize,
    seen_root: bool,
}

impl<'s, L: Clone> StreamValidator<'s, L> {
    /// Starts a validation of one document against `schema`. Like
    /// [`validate`](crate::validate), returns [`DatatypeError`] instead if
    /// `registry` cannot check every datatype the schema uses.
    pub fn new(
        schema: &'s CompiledSchema,
        registry: &'s DatatypeRegistry,
    ) -> Result<Self, DatatypeError> {
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
        let mut interner = Interner::new(&schema.definitions, registry);
        let start = interner.intern(&schema.start);
        Ok(Self {
            interner,
            start,
            stack: Vec::new(),
            path: Vec::new(),
            errors: Vec::new(),
            skipping: 0,
            seen_root: false,
        })
    }

    /// An element starts. `attributes` are all of its attributes (any
    /// order, no `xmlns` declarations); `namespace_bindings` the prefix
    /// bindings in scope, as
    /// [`Element::namespace_bindings`](super::Element::namespace_bindings).
    /// `location` is carried into every finding raised against the element.
    pub fn start_element(
        &mut self,
        name: ExpandedName,
        attributes: impl IntoIterator<Item = (ExpandedName, String)>,
        namespace_bindings: impl IntoIterator<Item = (String, String)>,
        location: Option<L>,
    ) -> Result<(), StreamError> {
        if self.skipping > 0 {
            self.skipping += 1;
            return Ok(());
        }
        let name_id = self.interner.name_id(&name);
        let content = if let Some(parent) = self.stack.last_mut() {
            flush(
                &mut self.interner,
                parent,
                &self.path,
                &mut self.errors,
                true,
            );
            parent.children += 1;
            let content = self
                .interner
                .element_contents_by_id(parent.pattern, &name, name_id);
            if content.is_some() {
                parent.pattern = self
                    .interner
                    .deriv_element_by_id(parent.pattern, &name, name_id);
            }
            content
        } else {
            if self.seen_root {
                return Err(StreamError::SecondRoot);
            }
            self.seen_root = true;
            self.interner.element_contents(self.start, &name)
        };
        let Some(content) = content else {
            self.errors.push(ValidationError {
                kind: ValidationErrorKind::UnexpectedElement(name),
                path: self.path.clone(),
                location,
                recovered: false,
            });
            self.skipping = 1;
            return Ok(());
        };

        self.path.push(name);
        let mut pattern = content;
        let bindings: Vec<(String, String)> = namespace_bindings.into_iter().collect();
        let context_id = self.interner.context_id(&bindings);
        let context = DatatypeContext::from_bindings(bindings);
        for (name, value) in attributes {
            let name_id = self.interner.name_id(&name);
            if !self.interner.attribute_known_by_id(pattern, &name, name_id) {
                self.errors.push(ValidationError {
                    kind: ValidationErrorKind::UnexpectedAttribute(name),
                    path: self.path.clone(),
                    location: location.clone(),
                    recovered: false,
                });
                continue;
            }
            let derived = self
                .interner
                .deriv_attribute_by_id(pattern, &name, name_id, &value, context_id, &context);
            if derived == NOT_ALLOWED {
                self.errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidAttributeValue(name, value),
                    path: self.path.clone(),
                    location: location.clone(),
                    recovered: false,
                });
            } else {
                pattern = derived;
            }
        }
        self.stack.push(Frame {
            pattern,
            context,
            location,
            pending: String::new(),
            children: 0,
        });
        Ok(())
    }

    /// Character data inside the current element. Consecutive calls are
    /// one run (text is merged up to the next start or end tag); an empty
    /// `text` is a no-op. Whitespace outside the root is ignored, any
    /// other text there is a [`StreamError::TextOutsideRoot`].
    pub fn text(&mut self, text: &str) -> Result<(), StreamError> {
        if self.skipping > 0 {
            return Ok(());
        }
        match self.stack.last_mut() {
            Some(frame) => frame.pending.push_str(text),
            None if text.trim().is_empty() => {}
            None => return Err(StreamError::TextOutsideRoot),
        }
        Ok(())
    }

    /// The current element ends.
    pub fn end_element(&mut self) -> Result<(), StreamError> {
        if self.skipping > 0 {
            self.skipping -= 1;
            return Ok(());
        }
        let Some(mut frame) = self.stack.pop() else {
            return Err(StreamError::UnbalancedEnd);
        };
        flush(
            &mut self.interner,
            &mut frame,
            &self.path,
            &mut self.errors,
            false,
        );
        if !self.interner.nullable(frame.pattern) {
            self.errors.push(ValidationError {
                kind: ValidationErrorKind::MissingContent,
                path: self.path.clone(),
                location: frame.location,
                recovered: false,
            });
        }
        self.path.pop();
        Ok(())
    }

    /// The document ends; returns the findings (empty means valid).
    /// [`StreamError::Unfinished`] if the root element is not complete.
    pub fn finish(self) -> Result<Vec<ValidationError<L>>, StreamError> {
        if !self.seen_root || !self.stack.is_empty() || self.skipping > 0 {
            return Err(StreamError::Unfinished);
        }
        Ok(self.errors)
    }
}

/// Offers the collected text of `frame` to its pattern as one text node.
/// `followed` is whether a sibling child starts right after it.
fn flush<L: Clone>(
    interner: &mut Interner<'_>,
    frame: &mut Frame<L>,
    path: &[ExpandedName],
    errors: &mut Vec<ValidationError<L>>,
    followed: bool,
) {
    if frame.pending.is_empty() {
        return;
    }
    let text = std::mem::take(&mut frame.pending);
    // §6.2.7 weak match, as in the tree walk: whitespace-only text next to
    // other children is not offered to the pattern. "Other children" means
    // an earlier child or one that starts right after this run.
    let strip_whitespace = followed || frame.children > 0;
    frame.children += 1;
    if strip_whitespace && text.trim().is_empty() {
        return;
    }
    let derived = interner.deriv_text(frame.pattern, &text, &frame.context);
    if derived == NOT_ALLOWED {
        if !text.trim().is_empty() || interner.wants_text(frame.pattern) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::UnexpectedText,
                path: path.to_vec(),
                location: frame.location.clone(),
                recovered: false,
            });
        }
    } else {
        frame.pattern = derived;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResolveError, SchemaResolver, SchemaSource, SchemaSyntax};

    struct NoResolver;

    impl SchemaResolver for NoResolver {
        fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
            Err(ResolveError::new(format!("no includes (`{href}`)")))
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

    fn name(local: &str) -> ExpandedName {
        ExpandedName::new(None::<String>, local)
    }

    fn open(validator: &mut StreamValidator<'_, u32>, local: &str, at: u32) {
        validator
            .start_element(name(local), [], [], Some(at))
            .expect("events");
    }

    fn kinds(errors: &[ValidationError<u32>]) -> Vec<ValidationErrorKind> {
        errors.iter().map(|error| error.kind().clone()).collect()
    }

    #[test]
    fn reports_findings_with_path_and_location() {
        let compiled = schema("start = element a { element b { empty } }");
        let registry = DatatypeRegistry::new();
        let mut validator = StreamValidator::new(&compiled, &registry).unwrap();
        open(&mut validator, "a", 1);
        open(&mut validator, "c", 2);
        open(&mut validator, "b", 3); // inside the skipped subtree
        validator.end_element().unwrap();
        validator.end_element().unwrap();
        validator.end_element().unwrap();
        let errors = validator.finish().unwrap();
        assert_eq!(
            kinds(&errors),
            [
                ValidationErrorKind::UnexpectedElement(name("c")),
                ValidationErrorKind::MissingContent
            ]
        );
        assert_eq!(errors[0].path(), [name("a")]);
        assert_eq!(errors[0].location(), Some(&2));
        assert_eq!(errors[1].location(), Some(&1));
    }

    #[test]
    fn data_needs_the_complete_text() {
        let compiled = schema(
            "datatypes xsd = \"http://www.w3.org/2001/XMLSchema-datatypes\"\n\
             start = element n { xsd:integer }",
        );
        let registry = DatatypeRegistry::new();
        let mut validator = StreamValidator::<()>::new(&compiled, &registry).unwrap();
        validator.start_element(name("n"), [], [], None).unwrap();
        validator.text("1").unwrap();
        validator.text("2").unwrap();
        validator.end_element().unwrap();
        assert!(validator.finish().unwrap().is_empty());
    }

    #[test]
    fn whitespace_only_text_is_weak_next_to_children() {
        let compiled = schema("start = element a { element b { empty }?, text }");
        let registry = DatatypeRegistry::new();
        let mut validator = StreamValidator::<()>::new(&compiled, &registry).unwrap();
        validator.start_element(name("a"), [], [], None).unwrap();
        validator.text(" ").unwrap();
        validator.start_element(name("b"), [], [], None).unwrap();
        validator.end_element().unwrap();
        validator.end_element().unwrap();
        // The leading blank must not have consumed `text` and so
        // forbidden the following `b`.
        assert!(validator.finish().unwrap().is_empty());
    }

    #[test]
    fn misuse_is_an_explicit_error() {
        let compiled = schema("start = element a { empty }");
        let registry = DatatypeRegistry::new();
        let fresh = || StreamValidator::<()>::new(&compiled, &registry).unwrap();

        assert_eq!(fresh().end_element(), Err(StreamError::UnbalancedEnd));
        assert_eq!(fresh().text("x"), Err(StreamError::TextOutsideRoot));
        assert!(fresh().text(" \n").is_ok());
        assert_eq!(fresh().finish().unwrap_err(), StreamError::Unfinished);

        let mut validator = fresh();
        validator.start_element(name("a"), [], [], None).unwrap();
        assert_eq!(validator.finish().unwrap_err(), StreamError::Unfinished);

        let mut validator = fresh();
        validator.start_element(name("a"), [], [], None).unwrap();
        validator.end_element().unwrap();
        assert_eq!(
            validator.start_element(name("a"), [], [], None),
            Err(StreamError::SecondRoot)
        );
    }

    #[test]
    fn many_distinct_attribute_values_past_the_memo_cap_still_validate() {
        // More distinct values than the derivative memo keeps: the findings
        // for a bad value must be the same whether it was memoised (early)
        // or not (late).
        let schema = schema(r#"element list { element item { attribute id { xsd:integer } }* }"#);
        let registry = DatatypeRegistry::new();
        let findings_with_bad_value_at = |bad: usize| {
            let mut v = StreamValidator::<()>::new(&schema, &registry).unwrap();
            v.start_element(name("list"), [], [], None).unwrap();
            for i in 0..3000 {
                let value = if i == bad {
                    "x".to_owned()
                } else {
                    i.to_string()
                };
                v.start_element(name("item"), [(name("id"), value)], [], None)
                    .unwrap();
                v.end_element().unwrap();
            }
            v.end_element().unwrap();
            v.finish().unwrap().len()
        };
        let early = findings_with_bad_value_at(5);
        assert!(early > 0);
        assert_eq!(findings_with_bad_value_at(2500), early);
    }
}
