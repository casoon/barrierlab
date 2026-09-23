//! Embeds the vendored HTML5 (`schema/html5/*.rnc`), SVG 1.1
//! (`schema/svg11/*.rnc`), and MathML 3 (`schema/mml3/*.rnc`) RELAX NG
//! schema modules and compiles them together via
//! [`relax_ng::Schema::compile`], caching the result for reuse by later
//! phases (content-model validation).
//!
//! No file-system access happens at runtime — every `.rnc` module is
//! embedded at compile time via `include_str!`, and `include`s between
//! them are resolved against that embedded copy.
//!
//! [`validate_document`]/[`findings`] are the entry points
//! [`crate::check_with_options`] uses (Phase 05d) — see also
//! `src/datatypes/mod.rs` for the `w:*` custom datatype library
//! [`DATATYPE_REGISTRY`] registers.

use std::sync::LazyLock;

use relax_ng::{CompileError, ResolveError, Schema, SchemaResolver, SchemaSource, SchemaSyntax};

/// The 19 vendored `schema/html5/*.rnc` modules `html5/html5.rnc`
/// `include`s, embedded at compile time. Keyed by the relative href
/// exactly as it appears in `html5.rnc`'s own `include` statements (e.g.
/// `"common.rnc"`) — bare, since every one of these is a same-directory
/// sibling include.
const HTML5_FILES: &[(&str, &str)] = &[
    (
        "applications.rnc",
        include_str!("../schema/html5/applications.rnc"),
    ),
    ("aria.rnc", include_str!("../schema/html5/aria.rnc")),
    ("block.rnc", include_str!("../schema/html5/block.rnc")),
    ("common.rnc", include_str!("../schema/html5/common.rnc")),
    (
        "core-scripting.rnc",
        include_str!("../schema/html5/core-scripting.rnc"),
    ),
    ("data.rnc", include_str!("../schema/html5/data.rnc")),
    ("embed.rnc", include_str!("../schema/html5/embed.rnc")),
    (
        "form-datatypes.rnc",
        include_str!("../schema/html5/form-datatypes.rnc"),
    ),
    ("media.rnc", include_str!("../schema/html5/media.rnc")),
    ("meta.rnc", include_str!("../schema/html5/meta.rnc")),
    (
        "microdata.rnc",
        include_str!("../schema/html5/microdata.rnc"),
    ),
    ("phrase.rnc", include_str!("../schema/html5/phrase.rnc")),
    ("revision.rnc", include_str!("../schema/html5/revision.rnc")),
    ("ruby.rnc", include_str!("../schema/html5/ruby.rnc")),
    (
        "sectional.rnc",
        include_str!("../schema/html5/sectional.rnc"),
    ),
    (
        "structural.rnc",
        include_str!("../schema/html5/structural.rnc"),
    ),
    ("tables.rnc", include_str!("../schema/html5/tables.rnc")),
    (
        "web-components.rnc",
        include_str!("../schema/html5/web-components.rnc"),
    ),
    (
        "web-forms.rnc",
        include_str!("../schema/html5/web-forms.rnc"),
    ),
    (
        "web-forms2.rnc",
        include_str!("../schema/html5/web-forms2.rnc"),
    ),
];

/// `html5/html5.rnc` itself — looked up under the directory-prefixed href
/// [`ROOT_ENTRY`] uses to include it (`"html5/html5.rnc"`), since from the
/// synthetic root's perspective it's no longer the top of the compile.
const HTML5_ROOT_MODULE: (&str, &str) =
    ("html5/html5.rnc", include_str!("../schema/html5/html5.rnc"));

/// vnu's real default schema entry point
/// (`http://s.validator.nu/html5-all.rnc`, confirmed via
/// `TestRunner.java`'s `DEFAULT_SCHEMA` constant and
/// `schema/.drivers/html5-all.rnc`'s own `include` graph — see
/// `plan/DECISIONS.md`) patches `<svg>`/`<math>` into `html5.rnc`'s
/// patterns from a *separate* driver file, not from `html5.rnc` itself —
/// confirmed by full-text search, `html5.rnc`'s own `include` list never
/// mentions either. Vendored via `xtask/vendor-svg-mathml.sh` (same
/// pinned commit as `xtask/vendor-schema.sh`).
const SVG_MATHML_PATCH: (&str, &str) = (
    "html5-svg-mathml.rnc",
    include_str!("../schema/html5-svg-mathml.rnc"),
);

/// The 39 vendored `schema/svg11/*.rnc` modules reachable from
/// `svg11-inc.rnc`'s own `include` graph (SVG 1.1 *full* profile — not
/// Basic/Tiny, which `svg11-inc.rnc` never reaches). Keyed bare, same
/// same-directory-sibling reasoning as [`HTML5_FILES`], except
/// `svg11-inc.rnc` itself, which also needs the directory-prefixed key
/// [`SVG_MATHML_PATCH`]'s own module uses to `include` it.
const SVG11_FILES: &[(&str, &str)] = &[
    (
        "svg-animation.rnc",
        include_str!("../schema/svg11/svg-animation.rnc"),
    ),
    (
        "svg-animevents-attrib.rnc",
        include_str!("../schema/svg11/svg-animevents-attrib.rnc"),
    ),
    (
        "svg-basic-clip.rnc",
        include_str!("../schema/svg11/svg-basic-clip.rnc"),
    ),
    (
        "svg-basic-filter.rnc",
        include_str!("../schema/svg11/svg-basic-filter.rnc"),
    ),
    (
        "svg-basic-font.rnc",
        include_str!("../schema/svg11/svg-basic-font.rnc"),
    ),
    (
        "svg-basic-graphics-attrib.rnc",
        include_str!("../schema/svg11/svg-basic-graphics-attrib.rnc"),
    ),
    (
        "svg-basic-structure.rnc",
        include_str!("../schema/svg11/svg-basic-structure.rnc"),
    ),
    (
        "svg-basic-text.rnc",
        include_str!("../schema/svg11/svg-basic-text.rnc"),
    ),
    ("svg-clip.rnc", include_str!("../schema/svg11/svg-clip.rnc")),
    (
        "svg-conditional.rnc",
        include_str!("../schema/svg11/svg-conditional.rnc"),
    ),
    (
        "svg-container-attrib.rnc",
        include_str!("../schema/svg11/svg-container-attrib.rnc"),
    ),
    (
        "svg-core-attrib.rnc",
        include_str!("../schema/svg11/svg-core-attrib.rnc"),
    ),
    (
        "svg-cursor.rnc",
        include_str!("../schema/svg11/svg-cursor.rnc"),
    ),
    (
        "svg-datatypes.rnc",
        include_str!("../schema/svg11/svg-datatypes.rnc"),
    ),
    (
        "svg-docevents-attrib.rnc",
        include_str!("../schema/svg11/svg-docevents-attrib.rnc"),
    ),
    (
        "svg-extensibility.rnc",
        include_str!("../schema/svg11/svg-extensibility.rnc"),
    ),
    (
        "svg-extresources-attrib.rnc",
        include_str!("../schema/svg11/svg-extresources-attrib.rnc"),
    ),
    (
        "svg-filter.rnc",
        include_str!("../schema/svg11/svg-filter.rnc"),
    ),
    ("svg-font.rnc", include_str!("../schema/svg11/svg-font.rnc")),
    (
        "svg-gradient.rnc",
        include_str!("../schema/svg11/svg-gradient.rnc"),
    ),
    (
        "svg-graphevents-attrib.rnc",
        include_str!("../schema/svg11/svg-graphevents-attrib.rnc"),
    ),
    (
        "svg-graphics-attrib.rnc",
        include_str!("../schema/svg11/svg-graphics-attrib.rnc"),
    ),
    (
        "svg-hyperlink.rnc",
        include_str!("../schema/svg11/svg-hyperlink.rnc"),
    ),
    (
        "svg-image.rnc",
        include_str!("../schema/svg11/svg-image.rnc"),
    ),
    (
        "svg-marker.rnc",
        include_str!("../schema/svg11/svg-marker.rnc"),
    ),
    ("svg-mask.rnc", include_str!("../schema/svg11/svg-mask.rnc")),
    (
        "svg-opacity-attrib.rnc",
        include_str!("../schema/svg11/svg-opacity-attrib.rnc"),
    ),
    (
        "svg-paint-attrib.rnc",
        include_str!("../schema/svg11/svg-paint-attrib.rnc"),
    ),
    (
        "svg-pattern.rnc",
        include_str!("../schema/svg11/svg-pattern.rnc"),
    ),
    (
        "svg-profile.rnc",
        include_str!("../schema/svg11/svg-profile.rnc"),
    ),
    (
        "svg-script.rnc",
        include_str!("../schema/svg11/svg-script.rnc"),
    ),
    (
        "svg-shape.rnc",
        include_str!("../schema/svg11/svg-shape.rnc"),
    ),
    (
        "svg-structure.rnc",
        include_str!("../schema/svg11/svg-structure.rnc"),
    ),
    (
        "svg-style.rnc",
        include_str!("../schema/svg11/svg-style.rnc"),
    ),
    ("svg-text.rnc", include_str!("../schema/svg11/svg-text.rnc")),
    ("svg-view.rnc", include_str!("../schema/svg11/svg-view.rnc")),
    (
        "svg-viewport-attrib.rnc",
        include_str!("../schema/svg11/svg-viewport-attrib.rnc"),
    ),
    (
        "svg-xlink-attrib.rnc",
        include_str!("../schema/svg11/svg-xlink-attrib.rnc"),
    ),
];

/// `svg11-inc.rnc` itself, keyed under the directory-prefixed href
/// [`SVG_MATHML_PATCH`]'s module uses to include it
/// (`"svg11/svg11-inc.rnc"`).
const SVG11_ROOT_MODULE: (&str, &str) = (
    "svg11/svg11-inc.rnc",
    include_str!("../schema/svg11/svg11-inc.rnc"),
);

/// The 4 vendored `schema/mml3/*.rnc` sibling modules reachable from
/// `mathml3-inc.rnc`'s own `include` graph, keyed bare (same reasoning as
/// [`SVG11_FILES`]).
const MML3_FILES: &[(&str, &str)] = &[
    (
        "mathml3-common.rnc",
        include_str!("../schema/mml3/mathml3-common.rnc"),
    ),
    (
        "mathml3-content.rnc",
        include_str!("../schema/mml3/mathml3-content.rnc"),
    ),
    (
        "mathml3-presentation.rnc",
        include_str!("../schema/mml3/mathml3-presentation.rnc"),
    ),
    (
        "mathml3-strict-content.rnc",
        include_str!("../schema/mml3/mathml3-strict-content.rnc"),
    ),
];

/// `mathml3-inc.rnc` itself, keyed under the directory-prefixed href
/// [`SVG_MATHML_PATCH`]'s module uses to include it
/// (`"mml3/mathml3-inc.rnc"`).
const MML3_ROOT_MODULE: (&str, &str) = (
    "mml3/mathml3-inc.rnc",
    include_str!("../schema/mml3/mathml3-inc.rnc"),
);

/// A small, hand-composed (**not** vendored — no upstream file matches it
/// verbatim) entry point that just `include`s the real entry points of
/// the two vendored pieces, mirroring vnu's actual runtime schema
/// assembly (`http://s.validator.nu/html5-all.rnc`) semantically. It
/// can't be vendored verbatim: vnu's own literal driver chain
/// (`schema/.drivers/html5-all.rnc` -> `schema/html5/html5full-rdfa.rnc`)
/// references a `html5full-rdfa.rnc` file that doesn't exist anywhere in
/// the `validator/validator` source tree at the pinned commit (confirmed
/// by a full-repo file listing) — presumably assembled by a build step
/// outside the repo. This two-line composition is the smallest faithful
/// substitute: `html5/html5.rnc` unchanged, patched by
/// `html5-svg-mathml.rnc` unchanged, exactly like the real driver does.
///
/// Note on that missing file's own name: vnu's real default schema entry
/// point is literally `html5full-**rdfa**`, not `html5full` — i.e. vnu's
/// actual default schema does ship RDFa awareness baked in, which this
/// vendoring couldn't capture since the generating file isn't in the
/// source tree. `meta.rnc`'s own `RDFa Lite Property Metadata` block
/// (appended at the end of that file, not vendored) closes that specific,
/// confirmed gap for `<meta property>` (Open Graph) — see its own doc
/// comment for why it lives there instead of in a separate module here
/// (a cross-file `|=` combine for it did not actually take effect, A/B
/// tested directly; combining it into `meta.rnc` itself did).
const ROOT_ENTRY: &str = "include \"html5/html5.rnc\"\ninclude \"html5-svg-mathml.rnc\"\n";

/// Resolves every `include` href in the combined schema against the
/// embedded module groups above — no file-system access. Flat (no real
/// relative-URI joining against `base_uri`, same pragmatic shortcut this
/// resolver's predecessor already used for `html5/*.rnc` alone): safe
/// because none of these ~65 modules' basenames collide across `html5/`,
/// `svg11/`, and `mml3/` — checked directly (`ls .../*.rnc | xargs -n1
/// basename | sort | uniq -d` on all three directories, verified empty)
/// before relying on it, not assumed. Each `_ROOT_MODULE`'s own doc
/// comment explains why it additionally needs a directory-prefixed key.
struct EmbeddedResolver;

impl SchemaResolver for EmbeddedResolver {
    fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
        std::iter::once(HTML5_ROOT_MODULE)
            .chain(std::iter::once(SVG_MATHML_PATCH))
            .chain(std::iter::once(SVG11_ROOT_MODULE))
            .chain(std::iter::once(MML3_ROOT_MODULE))
            .chain(HTML5_FILES.iter().copied())
            .chain(SVG11_FILES.iter().copied())
            .chain(MML3_FILES.iter().copied())
            .find(|(name, _)| *name == href)
            .map(|(name, text)| {
                SchemaSource::new(text, format!("embedded:/{name}"), SchemaSyntax::Compact)
            })
            .ok_or_else(|| ResolveError::new(format!("no embedded schema module for {href:?}")))
    }
}

/// Compiles the vendored HTML5+SVG+MathML schema from the synthetic
/// [`ROOT_ENTRY`].
fn compile_html5_schema() -> Result<Schema, CompileError> {
    let source = SchemaSource::new(ROOT_ENTRY, "embedded:/root.rnc", SchemaSyntax::Compact);
    Schema::compile(&source, &EmbeddedResolver)
}

/// Compiled once (`CompileError` is `Clone`, but there's no need to clone
/// it — `LazyLock` caches the `Result` itself, so repeated access never
/// re-parses).
static HTML5_SCHEMA: LazyLock<Result<Schema, CompileError>> = LazyLock::new(compile_html5_schema);

/// The compiled, vendored HTML5 RELAX NG schema — compiled once (on first
/// access) and cached for the lifetime of the process. Later phases
/// (content-model validation) call [`Schema::validate`] against it.
pub(crate) fn html5_schema() -> Result<&'static Schema, &'static CompileError> {
    HTML5_SCHEMA.as_ref()
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
    let schema = html5_schema().map_err(ToString::to_string)?;
    let Some(root) = document.root_element() else {
        return Ok(Vec::new());
    };
    schema
        .validate(&DATATYPE_REGISTRY, &root)
        .map_err(|error| error.to_string())
}

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
            rule_id: "schema.html5".to_owned(),
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
        })
        .collect()
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

    #[test]
    fn broken_schema_produces_a_clean_error_not_a_panic() {
        struct NoResolver;
        impl SchemaResolver for NoResolver {
            fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
                Err(ResolveError::new(format!("no such resource: {href}")))
            }
        }

        let source = SchemaSource::new(
            "element broken { text",
            "embedded:/broken.rnc",
            SchemaSyntax::Compact,
        );

        let result = Schema::compile(&source, &NoResolver);

        assert!(result.is_err());
    }
}
