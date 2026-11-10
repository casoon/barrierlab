//! The compiled HTML5 + SVG 1.1 + MathML 3 RELAX NG schema, loaded once from
//! `prebuilt/html5.rngc` and cached for reuse by later phases (content-model
//! validation).
//!
//! Compiling the vendored `.rnc` modules costs billions of instructions, so
//! that happens when the artefact is generated, not in every process
//! (barrierlab#72): the artefact is the serialized *simplified* pattern
//! ([`relax_ng::Schema::to_bytes`]), embedded with `include_bytes!`. The
//! test `prebuilt_schema_is_current` recompiles the sources and fails
//! when the artefact differs; `UPDATE_PREBUILT=1 cargo test -p html-conform
//! prebuilt_schema_is_current` rewrites it. The old compile-at-runtime path
//! stays in `src/schema_source.rs`, selected by the `runtime-compile` feature.
//!
//! No file-system access happens at runtime.
//!
//! [`validate_document`]/[`findings`] are the entry points
//! [`crate::check_with_options`] uses (Phase 05d) — see also
//! `src/datatypes/mod.rs` for the `w:*` custom datatype library
//! [`DATATYPE_REGISTRY`] registers.

use std::sync::LazyLock;

use relax_ng::Schema;

/// The serialized, simplified schema — see the module documentation.
#[cfg(not(feature = "runtime-compile"))]
const PREBUILT_SCHEMA: &[u8] = include_bytes!("../prebuilt/html5.rngc");

#[cfg(not(feature = "runtime-compile"))]
fn load_html5_schema() -> Result<Schema, String> {
    Schema::from_bytes(PREBUILT_SCHEMA).map_err(|error| error.to_string())
}

#[cfg(feature = "runtime-compile")]
fn load_html5_schema() -> Result<Schema, String> {
    crate::schema_source::compile_html5_schema().map_err(|error| error.to_string())
}

/// Loaded once (the error is kept as text, so repeated access never loads
/// again).
static HTML5_SCHEMA: LazyLock<Result<Schema, String>> = LazyLock::new(load_html5_schema);

/// The compiled, vendored HTML5 RELAX NG schema — loaded once (on first
/// access) and cached for the lifetime of the process. Later phases
/// (content-model validation) call [`Schema::validate`] against it.
pub(crate) fn html5_schema() -> Result<&'static Schema, &'static str> {
    HTML5_SCHEMA.as_ref().map_err(String::as_str)
}

/// The `datatypeLibrary` URI the vendored schema's custom `w:*` types are
/// declared under (`schema/html5/html5.rnc`: `datatypes w =
/// "http://whattf.org/datatype-draft"`).
const WHATWG_DATATYPE_LIBRARY_URI: &str = "http://whattf.org/datatype-draft";

/// The datatype registry [`validate_document`] uses: RELAX NG's built-ins
/// plus XSD (both already in [`relax_ng::DatatypeRegistry::new`]) plus
/// this crate's own `w:*` library (Phase 05c, `src/datatypes/mod.rs`) —
/// without which `Schema::validate` would fail immediately with
/// [`relax_ng::DatatypeError`] for *every* document, since it checks the
/// whole schema's datatype usage upfront, not just the parts a given
/// document reaches (see `Schema::validate`'s own doc comment).
static DATATYPE_REGISTRY: LazyLock<relax_ng::DatatypeRegistry> = LazyLock::new(|| {
    let mut registry = relax_ng::DatatypeRegistry::new();
    registry.register(
        WHATWG_DATATYPE_LIBRARY_URI,
        crate::datatypes::WhatwgDatatypeLibrary,
    );
    registry
});

/// Validates `document`'s root element (see
/// [`crate::infoset::NormalizedHtmlDocument::root_element`]) against the
/// vendored HTML5 schema.
///
/// `Err` only for a genuine setup failure — the embedded schema itself
/// failed to compile, or references a datatype [`DATATYPE_REGISTRY`]
/// doesn't know about — never a property of the checked document itself;
/// `Ok(&[])` means the document is schema-valid. Also `Ok(&[])` (not an
/// error) if `document` has no root element at all — nothing to validate.
pub(crate) fn validate_document(
    document: &crate::infoset::NormalizedHtmlDocument,
) -> Result<Vec<relax_ng::ValidationError<crate::finding::SourceLocation>>, String> {
    let schema = html5_schema().map_err(str::to_owned)?;
    let Some(root) = document.root_element() else {
        return Ok(Vec::new());
    };
    schema
        .validate(&DATATYPE_REGISTRY, &root)
        .map_err(|error| error.to_string())
}

/// The rule id of every finding this layer reports.
pub(crate) const RULE_ID: &str = "schema.html5";

/// Maps [`relax_ng::ValidationError`]s onto this crate's public
/// [`crate::Finding`] model. A single fixed `rule_id` (`schema.html5`,
/// matching `src/parse.rs`'s `parser.html5` convention) — RELAX NG grammar
/// conformance is one rule, not a per-error-kind taxonomy this crate
/// hasn't been asked to invent.
pub(crate) fn findings(
    errors: &[relax_ng::ValidationError<crate::finding::SourceLocation>],
) -> Vec<crate::finding::Finding> {
    errors
        .iter()
        .map(|error| crate::finding::Finding {
            rule_id: RULE_ID.to_owned(),
            severity: crate::finding::Severity::Error,
            message: error.to_string(),
            // `relax_ng::Element::Location` is generic as of Phase 08
            // (was a hardcoded `Option<String>`) — `src/infoset.rs`'s
            // `relax_ng::Element for NormalizedNode` impl sets it to
            // `crate::finding::SourceLocation` directly (the same
            // `{line, column, byte_offset}` shape `Finding` needs), so
            // this is a straight copy now, not a re-parse of formatted
            // text. Still `None` for synthesized (implicit `<html>`/
            // `<head>`/`<body>`, etc.) nodes, which genuinely have no
            // source position — see `NormalizedNode::position`'s own
            // doc comment.
            location: error.location().copied(),
            detail: Some(detail(error)),
        })
        .collect()
}

/// The structured arguments of one schema error. `on` is the element the
/// error occurred on (the last name of the error's path); for an unexpected
/// element that is the parent, so the offender comes from the error kind.
fn detail(
    error: &relax_ng::ValidationError<crate::finding::SourceLocation>,
) -> crate::finding::FindingDetail {
    use crate::finding::FindingDetail;
    use relax_ng::ValidationErrorKind as Kind;

    let on = error
        .path()
        .last()
        .map(|name| name.local.clone())
        .unwrap_or_default();
    match error.kind() {
        Kind::UnexpectedElement(name) => FindingDetail::UnexpectedElement {
            element: name.local.clone(),
        },
        Kind::UnexpectedText => FindingDetail::UnexpectedText { parent: on },
        Kind::UnexpectedAttribute(name) => FindingDetail::UnexpectedAttribute {
            attribute: name.local.clone(),
            element: on,
        },
        Kind::InvalidAttributeValue(name, value) => FindingDetail::InvalidAttributeValue {
            attribute: name.local.clone(),
            value: value.clone(),
            element: on,
        },
        Kind::MissingContent => FindingDetail::MissingContent { element: on },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_html5_schema_compiles() {
        html5_schema().expect("the real vendored HTML5 schema should compile");
    }

    #[test]
    fn cached_schema_is_reused_across_calls() {
        let first = html5_schema().expect("schema compiles");
        let second = html5_schema().expect("schema compiles");

        // Same `&'static Schema` on both accesses: `LazyLock` computed it
        // once and served the cached value the second time, rather than
        // recompiling.
        assert!(std::ptr::eq(first, second));
    }

    /// The artefact must be what compiling the vendored sources gives today.
    /// `UPDATE_PREBUILT=1` rewrites it instead of failing.
    #[test]
    fn prebuilt_schema_is_current() {
        let fresh = crate::schema_source::compile_html5_schema()
            .expect("the vendored schema compiles")
            .to_bytes()
            .expect("the simplified schema serializes");
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/prebuilt/html5.rngc");
        if std::env::var_os("UPDATE_PREBUILT").is_some() {
            std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/prebuilt"))
                .expect("create prebuilt/");
            std::fs::write(path, &fresh).expect("write prebuilt/html5.rngc");
            return;
        }
        let stored = std::fs::read(path).expect("prebuilt/html5.rngc exists");
        assert!(
            stored == fresh,
            "prebuilt/html5.rngc is stale; regenerate it with `UPDATE_PREBUILT=1 cargo test -p html-conform prebuilt_schema_is_current`"
        );
    }

    /// The prebuilt schema and the one compiled from the sources must give the same errors,
    /// with the same locations, for every document of the vendored corpus.
    #[test]
    #[ignore = "slow (~4655 documents) — run with `cargo test --release -p html-conform --lib prebuilt_and_compiled -- --ignored`"]
    fn prebuilt_and_compiled_schema_agree_on_the_corpus() {
        fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("corpus directory") {
                let path = entry.expect("directory entry").path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "html")
                {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        collect(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus"),
            &mut files,
        );
        assert!(files.len() > 1000, "the corpus is vendored");
        files.sort();

        let prebuilt = Schema::from_bytes(
            &std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/prebuilt/html5.rngc"))
                .expect("prebuilt/html5.rngc exists"),
        )
        .expect("the artefact loads");
        let compiled = crate::schema_source::compile_html5_schema().expect("the sources compile");

        let mut with_errors = 0;
        for path in &files {
            let html = std::fs::read_to_string(path).expect("UTF-8 fixture");
            let parsed = crate::parse::parse(&html);
            if crate::parse::nesting_depth(parsed.document()) > crate::MAX_NESTING_DEPTH {
                continue;
            }
            let document = crate::infoset::normalize(parsed.document(), parsed.source());
            let Some(root) = document.root_element() else {
                continue;
            };
            let render = |schema: &Schema| -> Vec<String> {
                schema
                    .validate(&DATATYPE_REGISTRY, &root)
                    .expect("datatypes are known")
                    .iter()
                    .map(|error| format!("{error} @ {:?}", error.location()))
                    .collect()
            };
            let (from_artefact, from_sources) = (render(&prebuilt), render(&compiled));
            assert_eq!(from_artefact, from_sources, "{}", path.display());
            with_errors += usize::from(!from_artefact.is_empty());
        }
        assert!(with_errors > 100, "the corpus has invalid documents");
    }

    /// Pushes `element` and its subtree into `validator` as events.
    fn push_events<E>(validator: &mut relax_ng::StreamValidator<'_, E::Location>, element: &E)
    where
        E: relax_ng::Element,
        E::Location: Clone,
    {
        validator
            .start_element(
                element.name(),
                element.attributes(),
                element.namespace_bindings(),
                element.location(),
            )
            .expect("well-formed events");
        for child in element.children() {
            match child {
                relax_ng::Content::Text(text) => validator.text(&text).expect("text"),
                relax_ng::Content::Element(child) => push_events(validator, &child),
            }
        }
        validator.end_element().expect("well-formed events");
    }

    /// The push validator reports exactly the errors, locations and order of the tree walk
    /// for every document of the vendored corpus (barrierlab#77).
    #[test]
    #[ignore = "slow (~4655 documents) — run with `cargo test --release -p html-conform --lib stream_and_tree -- --ignored`"]
    fn stream_and_tree_validation_agree_on_the_corpus() {
        fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("corpus directory") {
                let path = entry.expect("directory entry").path();
                if path.is_dir() {
                    collect(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "html")
                {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        collect(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus"),
            &mut files,
        );
        assert!(files.len() > 1000, "the corpus is vendored");
        files.sort();

        let schema = html5_schema().expect("the schema loads");
        let (mut checked, mut with_errors) = (0, 0);
        for path in &files {
            let html = std::fs::read_to_string(path).expect("UTF-8 fixture");
            let parsed = crate::parse::parse(&html);
            if crate::parse::nesting_depth(parsed.document()) > crate::MAX_NESTING_DEPTH {
                continue;
            }
            let document = crate::infoset::normalize(parsed.document(), parsed.source());
            let Some(root) = document.root_element() else {
                continue;
            };
            let render =
                |errors: &[relax_ng::ValidationError<crate::finding::SourceLocation>]| {
                    errors
                        .iter()
                        .map(|error| format!("{error} @ {:?}", error.location()))
                        .collect::<Vec<_>>()
                };
            let tree = schema
                .validate(&DATATYPE_REGISTRY, &root)
                .expect("datatypes are known");
            let mut validator = schema
                .stream_validator(&DATATYPE_REGISTRY)
                .expect("datatypes are known");
            push_events(&mut validator, &root);
            let stream = validator.finish().expect("complete document");
            assert_eq!(render(&tree), render(&stream), "{}", path.display());
            checked += 1;
            with_errors += usize::from(!tree.is_empty());
        }
        eprintln!("stream vs tree: {checked} documents identical, {with_errors} with errors");
        assert!(with_errors > 100, "the corpus has invalid documents");
    }
}
