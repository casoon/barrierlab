//! Adapter from html5-parser's HTML5 parse tree to a normalized XML infoset.
//!
//! The vendored HTML5 RELAX NG schema (`schema/html5/html5.rnc`) is not a
//! schema for raw HTML5 parser nodes: it expects an XHTML-namespaced element
//! tree with expanded names, not html5-parser's `namespace: None` on plain
//! elements. This module bridges that gap. See
//! `plan/04a-input-normalization.md` for the phase plan and the
//! adapter-contract entry in `plan/DECISIONS.md` for the full, documented
//! contract this module implements (originally written against xmloxide;
//! see `plan/DECISIONS.md`'s Phase 08 migration entry for what changed
//! when this module switched to html5-parser — the tree shape this
//! module reads is deliberately close to identical, so the contract
//! itself carries over almost unchanged).
//!
//! [`normalize`] is wired into [`crate::check`]/[`crate::check_with_options`].
//!
//! ## Arena shape (Phase 06)
//!
//! [`NormalizedHtmlDocument`] is a flat arena (`Vec<NodeData>`, indexed by
//! [`NodeId`]) rather than the owned recursive tree Phase 04a originally
//! built. [`NormalizedNode`] is a small `Copy` handle
//! (`{ document: &'a NormalizedHtmlDocument, id: NodeId }`), not an owned
//! node — this is what lets it carry a working [`NormalizedNode::parent`],
//! which the owned-tree shape structurally could not provide (no
//! back-pointers). That gap — and this arena rebuild as its resolution —
//! was already anticipated in `xpath-eval/plan/03-document-trait.md`
//! ("Bekannte Kompatibilitätslücke") and `plan/05a-element-adapter.md`; see
//! `plan/DECISIONS.md` for the actual decision record. A synthetic
//! `Kind::Root` node (always [`NodeId`] `0`) is the document's XPath root —
//! its children are this document's top-level nodes (normally a single
//! `<html>` element, but a leading comment before it is preserved too).

use html5_parser::{Document, NodeId as SourceNodeId, NodeKind as SourceNodeKind};

use crate::finding::SourceLocation;

/// The XHTML namespace URI synthesized for plain (non-foreign) HTML elements.
///
/// html5-parser's tree builder assigns `namespace: None` to ordinary
/// elements; SVG and MathML elements already carry their correct
/// namespace URI, assigned during foreign-content insertion. See the
/// adapter-contract entry in `plan/DECISIONS.md`.
pub(crate) const XHTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";

/// The XML-spec-mandated namespace URI the `xml` prefix is always
/// pre-bound to (XML Namespaces 1.0 §3) — `relax_ng`'s compact-syntax
/// compiler binds it the same way (`relax-ng/src/syntax.rs`), so the
/// vendored schema's `attribute xml:lang { ... }` pattern compiles to
/// this namespace, not the literal, unsplit `"xml:lang"` local name this
/// crate's infoset otherwise keeps for HTML-parsing-spec-correctness (see
/// [`relax_ng::Element for NormalizedNode`]'s `attributes()`).
pub(crate) const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// XML Namespaces 1.0 §3's namespace URI for the `xmlns` prefix itself.
/// The HTML5 parsing spec's "adjust foreign attributes" step
/// (§13.2.6.1) assigns this to a bare `xmlns`/`xmlns:*` attribute
/// *only* on an SVG/MathML element's own attribute list (confirmed by
/// html5-parser's tree — the same attribute on a plain HTML element, or
/// an XHTML element embedded inside a `<foreignObject>`, keeps
/// `namespace: None` instead). Used by
/// [`relax_ng::Element::attributes()`]'s schema-layer drop below.
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// SVG and MathML namespace URIs — see the module doc comment / the
/// adapter-contract entry in `plan/DECISIONS.md` for how html5-parser's
/// tree builder assigns these during foreign-content insertion. Since the
/// Phase 08 SVG/MathML schema vendoring (`xtask/vendor-svg-mathml.sh`),
/// no production code here needs to single either namespace out anymore
/// — both validate through the ordinary `relax_ng::Element` path like any
/// other namespace — so these are only referenced from tests now, hence
/// the `cfg_attr`.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";

/// vnu's own synthetic namespace for matching custom elements
/// (`schema/html5/web-components.rnc`: `namespace c = "http://n.validator.nu/custom-elements/"`,
/// `common.elem.flow |= element c:* { ... }`). Not a real DOM/XML
/// namespace a custom element ever actually has (per the HTML5 parsing
/// algorithm, `<my-widget>` gets the ordinary XHTML namespace like any
/// other element) — it only exists inside the vendored RELAX NG schema
/// as a wildcard-matching trick, since RELAX NG's `NameClass` has no
/// "any element name matching this pattern" concept (the same
/// structural limitation already documented for `data-*` attributes in
/// [`relax_ng::Element for NormalizedNode`]'s `attributes()`). Used
/// schema-layer-only, by `name()` below — the XPath-facing
/// [`xpath_eval::Node::expanded_name`] keeps the real XHTML namespace
/// unchanged.
const CUSTOM_ELEMENT_NAMESPACE: &str = "http://n.validator.nu/custom-elements/";

/// An XML-infoset expanded name: an optional namespace URI plus a local name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpandedName {
    pub(crate) namespace: Option<String>,
    pub(crate) local_name: String,
}

/// A stable arena index into a [`NormalizedHtmlDocument`]'s nodes. `0` is
/// always the synthetic root (see the module doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(usize);

/// One arena slot's kind-specific data — see [`NormalizedNode`] for the
/// borrowed handle callers actually work with; only the node kinds the
/// vendored RNC content model needs are represented (as in Phase 04a).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// The synthetic document root (arena index `0`). Not itself part of
    /// the XML infoset — only its children are.
    Root,
    /// An element with an expanded name. `attributes` holds the ids of this
    /// element's [`Kind::Attribute`] nodes — materialized as their own
    /// arena nodes (not inline tuples) because `xpath_eval::Node` needs
    /// attributes to be independently addressable nodes with their own
    /// identity/parent/string-value, per the XPath 1.0 data model (§5.3).
    /// Not part of [`NormalizedNode::child_nodes`] — see that method's doc
    /// comment on why attributes and children are tracked separately.
    Element {
        name: ExpandedName,
        attributes: Vec<NodeId>,
    },
    /// An attribute node (XPath data model §5.3). Its arena `parent` is its
    /// owner element.
    Attribute { name: ExpandedName, value: String },
    /// A text node. Named/numeric character references are already resolved
    /// by the HTML5 tokenizer; `script`/`style` (RAWTEXT/script-data)
    /// content is plain text too, with no special representation — see the
    /// adapter-contract entry in `plan/DECISIONS.md`.
    Text { content: String },
    /// A comment node.
    Comment { content: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeData {
    kind: Kind,
    position: Option<SourceLocation>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

/// The root of a normalized HTML infoset, produced by [`normalize`]. A flat
/// arena — see the module doc comment for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedHtmlDocument {
    /// Index `0` is always the synthetic [`Kind::Root`].
    nodes: Vec<NodeData>,
}

impl NormalizedHtmlDocument {
    /// The synthetic document root. Its children are this document's
    /// top-level nodes, in document order.
    pub(crate) fn root(&self) -> NormalizedNode<'_> {
        NormalizedNode {
            document: self,
            id: NodeId(0),
        }
    }

    /// This document's top-level nodes (the root's children), in document
    /// order — normally a single `<html>` element. Convenience for callers
    /// that only care about the content, not the synthetic root itself
    /// (e.g. RELAX NG validation, which starts from an element, not a
    /// document wrapper).
    pub(crate) fn children(&self) -> impl Iterator<Item = NormalizedNode<'_>> {
        self.root().child_nodes()
    }

    /// This document's root *element* (`<html>`, per the HTML5 tree
    /// construction algorithm) — the first `Element`-kind top-level node,
    /// skipping any leading `Comment` (a plain `<!--...--> <html>...`
    /// document is valid and preserved by [`normalize`], but
    /// `relax_ng::Schema::validate` needs an actual element to start from,
    /// not the synthetic root or a comment). `None` only if [`normalize`]
    /// produced no element at all (malformed/empty input beyond what
    /// HTML5's error-recovery tree construction can synthesize — should
    /// not happen for real HTML5 parser output in practice).
    pub(crate) fn root_element(&self) -> Option<NormalizedNode<'_>> {
        self.children()
            .find(|node| matches!(node.data().kind, Kind::Element { .. }))
    }
}

/// A borrowed, `Copy` handle to one node in a [`NormalizedHtmlDocument`]'s
/// arena — not an owned node. See the module doc comment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NormalizedNode<'a> {
    document: &'a NormalizedHtmlDocument,
    id: NodeId,
}

impl PartialEq for NormalizedNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.document, other.document) && self.id == other.id
    }
}

impl Eq for NormalizedNode<'_> {}

impl<'a> NormalizedNode<'a> {
    /// This node's arena slot. Takes `self` by value (not `&self`) so the
    /// returned reference is tied to the arena's lifetime `'a` (borrowed
    /// out of `self.document`, a `Copy` field), not to however long `self`
    /// itself happens to be borrowed for — the same reasoning
    /// `xpath_eval::Node`'s own methods document for why they take `self`
    /// by value.
    fn data(self) -> &'a NodeData {
        &self.document.nodes[self.id.0]
    }

    /// This node's parent, or `None` only for the synthetic root itself.
    pub(crate) fn parent(self) -> Option<Self> {
        self.data().parent.map(|id| NormalizedNode {
            document: self.document,
            id,
        })
    }

    /// This node's raw children (including `Text`/`Comment` nodes), in
    /// document order. Named `child_nodes` rather than `children` so it
    /// doesn't collide with [`relax_ng::Element::children`]'s
    /// `Content<Self>`-yielding method of the same conventional name —
    /// inherent methods shadow trait methods in Rust's method resolution,
    /// so keeping both named `children` would silently pick this one
    /// wherever `relax_ng::Element` was meant, with a type mismatch as the
    /// only symptom.
    pub(crate) fn child_nodes(self) -> impl Iterator<Item = Self> + 'a {
        self.data().children.iter().map(move |&id| NormalizedNode {
            document: self.document,
            id,
        })
    }

    /// This node's attribute nodes — non-empty only for [`Kind::Element`].
    /// Not included in [`NormalizedNode::child_nodes`], matching the XPath
    /// 1.0 data model (§5.3): attribute nodes have a parent (their owner
    /// element) but are never anyone's `children`.
    pub(crate) fn attribute_nodes(self) -> impl Iterator<Item = Self> + 'a {
        let attributes: &'a [NodeId] = match &self.data().kind {
            Kind::Element { attributes, .. } => attributes,
            Kind::Root | Kind::Attribute { .. } | Kind::Text { .. } | Kind::Comment { .. } => &[],
        };
        attributes.iter().map(move |&id| NormalizedNode {
            document: self.document,
            id,
        })
    }

    /// This node's source position, if any (currently always `None` — see
    /// [`normalize`]'s doc comment).
    pub(crate) fn position(self) -> Option<&'a SourceLocation> {
        self.data().position.as_ref()
    }

    /// Whether this is the synthetic document root (arena index `0`).
    /// Currently only exercised by tests (`parent()` returning the root is
    /// otherwise self-evident from context) — kept as real, reachable API
    /// rather than inlined into a test assertion, since any future caller
    /// that walks upward via `parent()` needs a way to detect the top.
    #[allow(dead_code)]
    pub(crate) fn is_document_root(self) -> bool {
        self.id.0 == 0
    }
}

/// Normalizes an html5-parser HTML5 parse tree into the infoset shape the
/// vendored HTML5 RELAX NG schema expects.
///
/// Document order is preserved. `Doctype` and `ProcessingInstruction` nodes
/// are dropped: the vendored schema's content model does not reference
/// DOCTYPE/PI representation. html5-parser never produces `EntityRef` or
/// `CData` nodes at all (character references and RAWTEXT content are
/// resolved straight to `Text`) — see the adapter-contract entry in
/// `plan/DECISIONS.md`. `DocumentFragment` (a `<template>` element's
/// synthetic "template contents" wrapper, its sole real tree child) is
/// transparent here: not represented as a node of its own, its children
/// are spliced directly into its parent's (the `<template>` element's)
/// children instead — matching what the vendored schema's content model
/// actually expects to see under `<template>`.
///
/// Every produced node's source position comes straight from
/// html5-parser's own `Position` (`None` only for a node html5-parser
/// itself synthesizes, e.g. an implied `<html>`/`<head>`/`<body>` —
/// closes the "no source position available" gap this module's
/// xmloxide-based predecessor had, documented in `plan/DECISIONS.md`'s
/// Phase 04a entries and resolved by the Phase 08 migration entry).
pub(crate) fn normalize(document: &Document, source: &str) -> NormalizedHtmlDocument {
    let _ = source;

    // Reserve index 0 for the synthetic root before recursing, so every
    // top-level node's `parent` can point at it immediately.
    let mut nodes = vec![NodeData {
        kind: Kind::Root,
        position: None,
        parent: None,
        children: Vec::new(),
    }];
    let root_id = NodeId(0);

    let root_children = normalized_children(document, document.root(), &mut nodes, root_id);
    nodes[0].children = root_children;

    NormalizedHtmlDocument { nodes }
}

/// Normalizes every child of `id`, in document order, splicing a
/// `DocumentFragment` child's own children in transparently instead of
/// representing the fragment itself — see [`normalize`]'s doc comment.
/// Shared by [`normalize`] (the synthetic root's children) and
/// [`normalize_node`] (every other node's children), so the splicing
/// behavior is identical everywhere a `<template>` might appear.
fn normalized_children(
    document: &Document,
    id: SourceNodeId,
    nodes: &mut Vec<NodeData>,
    parent: NodeId,
) -> Vec<NodeId> {
    let mut result = Vec::new();
    for child in document.children(id) {
        if matches!(&document.node(child).kind, SourceNodeKind::DocumentFragment) {
            result.extend(normalized_children(document, child, nodes, parent));
        } else if let Some(normalized) = normalize_node(document, child, nodes, parent) {
            result.push(normalized);
        }
    }
    result
}

/// Builds one node (and, recursively, its subtree) into `nodes`, returning
/// its [`NodeId`] — or `None` for a source node kind [`normalize`] drops
/// entirely (`Doctype`/`ProcessingInstruction`, or a nested `Document`
/// node, which never occurs below the root in practice). Never called
/// directly on a `DocumentFragment` — [`normalized_children`] intercepts
/// those before they'd reach here.
///
/// Checks whether `id` survives normalization *before* reserving its
/// [`NodeId`] (a placeholder push, overwritten once the real [`Kind`] is
/// known) — reserving unconditionally would leak an orphaned, unreachable
/// arena slot for every dropped node. The placeholder itself is needed
/// because an element's attribute nodes need their owner's `NodeId` as
/// their own `parent` *before* that element's final [`Kind::Element`]
/// (which references those attribute nodes' ids) can itself be
/// constructed — a small chicken-and-egg the placeholder breaks.
fn normalize_node(
    document: &Document,
    id: SourceNodeId,
    nodes: &mut Vec<NodeData>,
    parent: NodeId,
) -> Option<NodeId> {
    if matches!(
        &document.node(id).kind,
        SourceNodeKind::Document
            | SourceNodeKind::ProcessingInstruction { .. }
            | SourceNodeKind::Doctype { .. }
    ) {
        return None;
    }
    debug_assert!(
        !matches!(&document.node(id).kind, SourceNodeKind::DocumentFragment),
        "normalized_children should have intercepted any DocumentFragment before it reached here"
    );

    let position = document.node(id).position.map(|position| SourceLocation {
        line: position.line,
        column: position.column,
        byte_offset: position.byte_offset,
    });

    let this_id = NodeId(nodes.len());
    nodes.push(NodeData {
        kind: Kind::Comment {
            content: String::new(),
        }, // placeholder, overwritten below before this function returns
        position,
        parent: Some(parent),
        children: Vec::new(),
    });

    let kind = match &document.node(id).kind {
        SourceNodeKind::Element {
            name,
            namespace,
            attributes,
        } => Kind::Element {
            name: ExpandedName {
                namespace: Some(
                    namespace
                        .clone()
                        .unwrap_or_else(|| XHTML_NAMESPACE.to_owned()),
                ),
                local_name: name.clone(),
            },
            attributes: attributes
                .iter()
                .map(|attribute| {
                    let attribute_id = NodeId(nodes.len());
                    nodes.push(NodeData {
                        kind: Kind::Attribute {
                            name: ExpandedName {
                                namespace: attribute.namespace.clone(),
                                local_name: attribute.name.clone(),
                            },
                            value: attribute.value.clone(),
                        },
                        position: None,
                        parent: Some(this_id),
                        children: Vec::new(),
                    });
                    attribute_id
                })
                .collect(),
        },
        SourceNodeKind::Text { content } => Kind::Text {
            content: content.clone(),
        },
        SourceNodeKind::Comment { content } => Kind::Comment {
            content: content.clone(),
        },
        SourceNodeKind::Document
        | SourceNodeKind::ProcessingInstruction { .. }
        | SourceNodeKind::Doctype { .. } => {
            unreachable!("filtered out by the early check above")
        }
        SourceNodeKind::DocumentFragment => {
            unreachable!("normalized_children intercepts DocumentFragment before normalize_node")
        }
    };
    nodes[this_id.0].kind = kind;

    let children = normalized_children(document, id, nodes, this_id);
    nodes[this_id.0].children = children;

    Some(this_id)
}

/// Adapts a normalized HTML node to [`relax_ng::Element`], so a
/// [`relax_ng::Schema`] can validate a [`NormalizedHtmlDocument`] without
/// cloning subtrees.
///
/// Implemented for `NormalizedNode<'a>` directly (not a reference to it):
/// it is already a small `Copy` handle, not an owned node — see the module
/// doc comment.
impl<'a> relax_ng::Element for NormalizedNode<'a> {
    /// `Phase 08`: `relax_ng::Element::Location` became generic (was a
    /// hardcoded `Option<String>`) precisely so a caller with real
    /// structured positions — this crate, since the html5-parser
    /// migration — doesn't have to downgrade them to a formatted string
    /// and back. `SourceLocation` already exists (`crate::finding`) and
    /// already has exactly the shape [`Self::position`] returns.
    type Location = crate::finding::SourceLocation;

    /// Returns this node's expanded name.
    ///
    /// Only meaningful for an element node: the `relax-ng` validator only
    /// ever calls `name()` on the `E` carried by `Content::Element(E)`, and
    /// [`children`](Self::children) below only ever produces
    /// `Content::Element` for element children (`Text` becomes
    /// `Content::Text`, `Comment` is skipped entirely, and the synthetic
    /// root is never itself passed to a validator). So a `Text`/`Comment`/
    /// root node reaching `name()` would mean either this adapter or
    /// `relax-ng`'s own dispatch has a real bug — there is no meaningful
    /// expanded name to invent for those variants, and silently returning a
    /// bogus one (e.g. an empty local name) would corrupt validation
    /// results instead of surfacing the bug. Panicking documents and
    /// enforces the invariant.
    fn name(&self) -> relax_ng::ExpandedName {
        match &self.data().kind {
            Kind::Element { name, .. } => {
                if name.namespace.as_deref() == Some(XHTML_NAMESPACE)
                    && crate::datatypes::structural::check_custom_element_name(&name.local_name)
                        .is_ok()
                {
                    // Schema-layer-only remap — see CUSTOM_ELEMENT_NAMESPACE's
                    // doc comment. Confirmed necessary against the vendored
                    // corpus (plan/DECISIONS.md's Phase 08 entry): custom
                    // elements like `<view-source>`/`<streaming-element>`
                    // were rejected outright as "unexpected element" without
                    // this remap.
                    relax_ng::ExpandedName {
                        namespace: Some(CUSTOM_ELEMENT_NAMESPACE.to_owned()),
                        local: name.local_name.clone(),
                    }
                } else {
                    relax_ng::ExpandedName {
                        namespace: name.namespace.clone(),
                        local: name.local_name.clone(),
                    }
                }
            }
            Kind::Text { .. } | Kind::Comment { .. } | Kind::Root | Kind::Attribute { .. } => {
                panic!(
                    "relax_ng::Element::name() called on a non-Element NormalizedNode; \
                     only Content::Element(NormalizedNode::Element) should ever reach this"
                )
            }
        }
    }

    /// Schema-layer-only view of this element's attributes — remaps `lang`
    /// and literal `xml:lang` onto the single, namespace-split form the
    /// compiled schema's `attribute xml:lang { ... }` pattern actually
    /// expects: `{namespace: XML_NAMESPACE, local: "lang"}`.
    ///
    /// Two distinct issues, both traced empirically against the vendored
    /// corpus (`plan/DECISIONS.md`'s Phase 08 entry):
    ///
    /// 1. `schema/html5/common.rnc`'s `common.attrs.lang` is `& XMLonly`,
    ///    and `html5.rnc`'s HTML-mode override sets `XMLonly = notAllowed`
    ///    — so the vendored schema, taken literally, makes a plain `lang`
    ///    attribute `notAllowed` on *every* element, always.
    ///    `common.rnc`'s own comment on that definition explains why:
    ///    `"This lang definition is a hack for environments where the
    ///    HTML5 parser maps lang to xml:lang. Sameness check left to
    ///    Schematron"` — vnu's own HTML5 parser presents `lang`'s value to
    ///    the schema layer as `xml:lang`, leaving a literal `lang`/`xml:lang`
    ///    pair's agreement to a separate Schematron rule, not the RELAX NG
    ///    schema check.
    /// 2. A literal `xml:lang` attribute doesn't validate either, even
    ///    though [`xml_lang_attribute_keeps_literal_local_name_on_html_elements`]
    ///    confirms this crate's infoset correctly keeps it as a single,
    ///    unsplit `"xml:lang"` local name with no namespace (HTML5-parsing-
    ///    spec-correct: `adjustForeignAttributes` only applies inside
    ///    SVG/MathML, see the Phase 04a adapter-contract entry) — but
    ///    `relax_ng`'s compact-syntax compiler pre-binds the `xml` prefix to
    ///    [`XML_NAMESPACE`] like any spec-conformant XML processor
    ///    (`relax-ng/src/syntax.rs`), so `attribute xml:lang { ... }`
    ///    compiles to the *namespace-split* expanded name, not the literal
    ///    unsplit local name. The unsplit form is correct for the
    ///    XPath-facing assertions layer (real HTML documents never split
    ///    it), but a mismatch against what the schema pattern expects.
    ///
    /// Without this remap, `lang` (one of the single most common HTML
    /// attributes) made the vendored schema reject essentially every
    /// real-world document.
    ///
    /// When both a literal `xml:lang` and a plain `lang` are present, only
    /// the literal `xml:lang` is remapped and `lang` is dropped from this
    /// schema-facing view entirely, to avoid ever yielding the same
    /// expanded name twice (which `relax_ng::Schema::validate` has no
    /// defined behavior for). This dual-attribute case isn't
    /// corpus-verified; cross-checking the two for agreement is left to a
    /// future Schematron rule, per the schema's own comment above.
    ///
    /// This is a `relax_ng::Element`-only adapter: the XPath-facing
    /// [`Attribute::name`]/[`Attribute::local_name`] used by the assertion
    /// layer (Schematron) is untouched, so a rule cross-checking a literal
    /// `@lang` against `@xml:lang` still sees both under their real names.
    ///
    /// This same adapter also drops any `data-<name>` custom-data
    /// attribute entirely (schema layer only) — RELAX NG's `NameClass`
    /// has no prefix-wildcard concept (only exact names, `anyName`, or
    /// `nsName` with `except`, none of which can express "any attribute
    /// starting with `data-`"), and neither `schema/html5/*.rnc` nor
    /// `relax-ng` model or accept one; confirmed nowhere in the vendored
    /// `.rnc` sources. vnu itself special-cases `data-*` outside its own
    /// RelaxNG grammar the same way. Without dropping these, the schema
    /// rejected `data-*` attributes unconditionally — confirmed against
    /// `html/attributes/data/value-isvalid.html`/
    /// `colon-in-name-isvalid.html` (both expected clean).
    ///
    /// Only `data-` followed by *at least one* more character is
    /// dropped — bare `data-` (nothing after the hyphen) is left
    /// unmapped so the schema's existing "unexpected attribute"
    /// rejection keeps firing for it, matching
    /// `html/attributes/data/no-characters-after-hyphen-novalid.html`
    /// (`<p data-="">`, expected an error). The other `data-*`
    /// conformance requirements (XML-compatible, no uppercase ASCII,
    /// must not start case-insensitively with "xml") aren't
    /// corpus-verified here and are left unchecked — a documented,
    /// accepted residual gap, not silently claimed as covered.
    ///
    /// Also drops a `th` element's `abbr` attribute (schema layer only).
    /// `abbr` is still a current, non-obsolete WHATWG content attribute of
    /// `th` (unlike on `td`, where vnu's `Assertions.java` registers it as
    /// obsolete-with-warning via `OBSOLETE_ATTRIBUTES` — a separate,
    /// unimplemented warning-level concern, left as an accepted residual
    /// gap since no corpus fixture exercises it), but `schema/html5/
    /// tables.rnc`'s `th.attrs` never includes `tables.attrs.abbr` (a
    /// pattern the same file defines but never wires in — confirmed
    /// identical against the live `validator/validator` source, not a
    /// vendoring mistake). vnu's real default schema entry point
    /// (`http://s.validator.nu/html5-all.rnc`, used by its own test
    /// runner) resolves through driver files this project doesn't vendor
    /// (`schema/.drivers/`, itself referencing a generated `html5full-
    /// rdfa.rnc` not present in the source tree) that patch `th.attrs`
    /// with `&= th.attrs.abbr?` — i.e. vnu accepts this outside the
    /// modular `schema/html5/*.rnc` slice this crate vendors, the same
    /// situation as `data-*` above. Confirmed against
    /// `html-aria/misc/td-with-aria-selected-isvalid.html` (`<th
    /// scope="col" abbr="Sunday">`, expected clean).
    ///
    /// Also drops any namespace-declaration attribute (`xmlns` / `xmlns:*`
    /// — [`XMLNS_NAMESPACE`], or the same local name with no namespace at
    /// all when HTML5's "adjust foreign attributes" step doesn't apply,
    /// see that constant's doc comment) — a genuine XML-infoset-level
    /// concept this crate's `NormalizedNode` never models as such (no
    /// `xmlns` declarations/namespace nodes at all, only already-resolved
    /// `namespace` fields, see this file's module doc comment), so
    /// html5-parser leaves them as ordinary, literal attributes instead —
    /// which `schema/svg11/*.rnc`/`schema/mml3/*.rnc` (Phase 08's SVG/
    /// MathML vendoring) never model as content attributes either, since
    /// a properly namespace-processing infoset would have already
    /// consumed them into namespace bindings before schema validation
    /// ever sees the element. Confirmed against `html/svg/svg-transform-
    /// origin-transform-box-isvalid.html` (`<svg xmlns="...">` at the SVG
    /// root, `<div xmlns="...">` inside a `<foreignObject>`, expected
    /// clean).
    fn attributes(&self) -> impl Iterator<Item = (relax_ng::ExpandedName, String)> {
        let is_th = matches!(&self.data().kind, Kind::Element { name, .. }
            if name.namespace.as_deref() == Some(XHTML_NAMESPACE) && name.local_name == "th");

        let raw: Vec<(relax_ng::ExpandedName, String)> = (*self)
            .attribute_nodes()
            .map(|attribute| match &attribute.data().kind {
                Kind::Attribute { name, value } => (
                    relax_ng::ExpandedName {
                        namespace: name.namespace.clone(),
                        local: name.local_name.clone(),
                    },
                    value.clone(),
                ),
                Kind::Root | Kind::Element { .. } | Kind::Text { .. } | Kind::Comment { .. } => {
                    unreachable!("NormalizedNode::attribute_nodes only ever yields Attribute nodes")
                }
            })
            .collect();

        let has_literal_xml_lang = raw
            .iter()
            .any(|(name, _)| name.namespace.is_none() && name.local == "xml:lang");

        let split_xml_lang_name = || relax_ng::ExpandedName {
            namespace: Some(XML_NAMESPACE.to_owned()),
            local: "lang".to_owned(),
        };

        raw.into_iter().filter_map(move |(name, value)| {
            if name.namespace.is_none() && name.local == "xml:lang" {
                Some((split_xml_lang_name(), value))
            } else if name.namespace.is_none() && name.local == "lang" {
                if has_literal_xml_lang {
                    None
                } else {
                    Some((split_xml_lang_name(), value))
                }
            } else if name.namespace.as_deref() == Some(XMLNS_NAMESPACE)
                || (name.namespace.is_none()
                    && (name.local == "xmlns"
                        || name.local.starts_with("xmlns:")
                        || (name.local.starts_with("data-") && name.local.len() > "data-".len())
                        || (is_th && name.local == "abbr")))
            {
                None
            } else {
                Some((name, value))
            }
        })
    }

    /// Schema-layer-only: a `<selectedcontent>` element's children are
    /// always discarded here, never handed to
    /// [`merge_text_and_comment_runs`]. `html5-parser` implements
    /// WHATWG HTML's "maybe clone an option into selectedcontent"
    /// insertion step (`tree_builder.rs`'s
    /// `maybe_clone_option_into_selectedcontent`) — genuine, spec-correct
    /// *runtime DOM* behavior for a live browser, but not something
    /// vnu's own (purely static) parser reproduces, and not something a
    /// conformance checker should validate against: authors always write
    /// `<selectedcontent>` empty in source (`schema/html5/
    /// web-forms.rnc`'s `selectedcontent.inner = ( empty )` encodes
    /// exactly that), the mirrored content only exists because
    /// `html5-parser`'s tree builder runs that DOM insertion step during
    /// parsing. Every child a `<selectedcontent>` element can have is
    /// therefore always a synthesized clone, never literal source
    /// content — safe to drop unconditionally, not just the
    /// position-less ones. Confirmed against `html/elements/select/
    /// button-selectedcontent-isvalid.html` (expected clean; without
    /// this, the mirrored option text tripped "unexpected text" against
    /// the empty content model).
    fn children(&self) -> impl Iterator<Item = relax_ng::Content<Self>> {
        let is_selectedcontent = matches!(&self.data().kind, Kind::Element { name, .. }
            if name.namespace.as_deref() == Some(XHTML_NAMESPACE) && name.local_name == "selectedcontent");
        if is_selectedcontent {
            Vec::new().into_iter()
        } else {
            merge_text_and_comment_runs((*self).child_nodes()).into_iter()
        }
    }

    fn location(&self) -> Option<Self::Location> {
        self.position().copied()
    }
}

/// Builds the [`relax_ng::Content`] sequence for a run of `NormalizedNode`
/// children, honoring [`relax_ng::Content::Text`]'s contract: adjacent text
/// nodes — including ones only adjacent because a `Comment` between them is
/// dropped from this model — must be merged into a single `Text` before
/// reaching the caller. `Comment` children are otherwise skipped and never
/// yield a `Content` of their own; a comment run with no adjacent text
/// yields nothing at all, never an empty `Content::Text`.
///
/// SVG/MathML element children are validated for real now (Phase 08,
/// `xtask/vendor-svg-mathml.sh`) — `schema/html5/*.rnc` alone never
/// mentions either namespace (vnu's own modular `html5.rnc` doesn't
/// either, not a vendoring omission), but vnu's *real* default schema
/// entry point (`http://s.validator.nu/html5-all.rnc`) resolves through a
/// separate driver, `schema/.drivers/html5-svg-mathml.rnc`, that patches
/// `<svg>`/`<math>` in via the vendored SVG 1.1/MathML 3 modules
/// (`schema/svg11/`, `schema/mml3/`) — see `src/schema.rs`'s `ROOT_ENTRY`
/// and `plan/DECISIONS.md`'s SVG/MathML entry for the full vendoring and
/// integration story, including three genuine `relax-ng` parser gaps
/// (RNC annotations, `default namespace PREFIX = "..."` prefix
/// registration, single-quoted string literals) discovered and fixed
/// along the way. An earlier version of this function skipped both
/// namespaces' subtrees entirely at this layer; that workaround is gone.
fn merge_text_and_comment_runs<'a>(
    children: impl Iterator<Item = NormalizedNode<'a>>,
) -> Vec<relax_ng::Content<NormalizedNode<'a>>> {
    let mut result = Vec::new();
    let mut pending_text: Option<String> = None;
    for child in children {
        match &child.data().kind {
            Kind::Element { .. } => {
                if let Some(text) = pending_text.take() {
                    result.push(relax_ng::Content::Text(text));
                }
                result.push(relax_ng::Content::Element(child));
            }
            Kind::Text { content } => match &mut pending_text {
                Some(existing) => existing.push_str(content),
                None => pending_text = Some(content.clone()),
            },
            Kind::Comment { .. } => {}
            Kind::Root => unreachable!("the synthetic root is never anyone's child"),
            Kind::Attribute { .. } => {
                unreachable!("attribute nodes are never part of child_nodes()")
            }
        }
    }
    if let Some(text) = pending_text.take() {
        result.push(relax_ng::Content::Text(text));
    }
    result
}

/// Adapts a normalized HTML node to `xpath_eval::Node`/`Document`, so a
/// [`schematron_engine::evaluate`]-style call can run XPath `context`/
/// `test` expressions against a [`NormalizedHtmlDocument`] (Phase 06).
///
/// Scope deliberately narrower than the full XPath 1.0 data model, matching
/// what this crate's own infoset actually represents:
/// - No namespace nodes ([`Node::namespaces`] is always empty) — mirrors
///   `relax_ng::Element::namespace_bindings`'s existing Stage-1 scope
///   decision (`plan/05a-element-adapter.md`): `NormalizedNode` has no
///   `xmlns` declarations in its model, only already-resolved `namespace`
///   fields on elements/attributes.
/// - No `ProcessingInstruction` nodes — [`normalize`] already drops them
///   from the source tree entirely (see its doc comment).
/// - [`Node::is_id_attribute`] keeps the trait's default (`false`
///   everywhere) — this crate has no DTD/schema ID information to report,
///   same rationale `xpath_eval::Node`'s own doc comment gives.
///
/// **On the `children()` naming collision:** both this trait and
/// `relax_ng::Element` (implemented above) declare a `children()` method
/// on the same type, with different, incompatible return types
/// (`impl Iterator<Item = Self>` here vs. `impl Iterator<Item =
/// Content<Self>>` there). Unlike [`NormalizedNode::child_nodes`] vs.
/// `relax_ng::Element::children`, this is a collision between two *trait*
/// methods, which Rust cannot resolve via inherent-method shadowing —
/// calling `.children()` on a `NormalizedNode` with both `relax_ng::Element`
/// and `xpath_eval::Node` in scope at once is a compile-time ambiguity
/// error (not a silent bug, unlike the inherent/trait case). Avoid it by
/// keeping the two traits' call sites in different scopes (as this crate
/// does today — the RELAX NG adapter above and any future Schematron/XPath
/// integration are separate concerns) or by using fully-qualified syntax
/// (`xpath_eval::Node::children(node)`) if a caller ever genuinely needs
/// both in one scope.
impl<'a> xpath_eval::Node<'a> for NormalizedNode<'a> {
    fn kind(self) -> xpath_eval::NodeKind {
        match &self.data().kind {
            Kind::Root => xpath_eval::NodeKind::Root,
            Kind::Element { .. } => xpath_eval::NodeKind::Element,
            Kind::Attribute { .. } => xpath_eval::NodeKind::Attribute,
            Kind::Text { .. } => xpath_eval::NodeKind::Text,
            Kind::Comment { .. } => xpath_eval::NodeKind::Comment,
        }
    }

    fn parent(self) -> Option<Self> {
        NormalizedNode::parent(self)
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        self.child_nodes()
    }

    fn attributes(self) -> impl Iterator<Item = Self> + 'a {
        self.attribute_nodes()
    }

    fn namespaces(self) -> impl Iterator<Item = Self> + 'a {
        // See this impl block's doc comment ("No namespace nodes").
        std::iter::empty()
    }

    fn expanded_name(self) -> Option<xpath_eval::ExpandedName> {
        let name = match &self.data().kind {
            Kind::Element { name, .. } | Kind::Attribute { name, .. } => name,
            Kind::Root | Kind::Text { .. } | Kind::Comment { .. } => return None,
        };
        Some(xpath_eval::ExpandedName {
            namespace_uri: name.namespace.clone(),
            local_name: name.local_name.clone(),
        })
    }

    fn string_value(self) -> String {
        match &self.data().kind {
            Kind::Attribute { value, .. } => value.clone(),
            Kind::Text { content } | Kind::Comment { content } => content.clone(),
            Kind::Root | Kind::Element { .. } => {
                let mut value = String::new();
                collect_descendant_text(self, &mut value);
                value
            }
        }
    }

    fn document_order(self, other: Self) -> std::cmp::Ordering {
        // The arena is built in a single top-down, depth-first pass in
        // exactly document order (an element's own id precedes its
        // attribute nodes' ids, which precede its children's ids — see
        // `normalize_node`), so arena index order already *is* document
        // order; no separate traversal/comparison needed.
        self.id.0.cmp(&other.id.0)
    }
}

/// The string-value of an `Element`/`Root` node (XPath 1.0 §5.1/§5.2): the
/// concatenation, in document order, of every descendant `Text` node's
/// content — attribute values are not included (matches
/// [`NormalizedNode::child_nodes`] already excluding attribute nodes).
fn collect_descendant_text(node: NormalizedNode<'_>, out: &mut String) {
    match &node.data().kind {
        Kind::Text { content } => out.push_str(content),
        Kind::Root | Kind::Element { .. } => {
            for child in node.child_nodes() {
                collect_descendant_text(child, out);
            }
        }
        Kind::Attribute { .. } | Kind::Comment { .. } => {}
    }
}

impl xpath_eval::Document for NormalizedHtmlDocument {
    type N<'a>
        = NormalizedNode<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        NormalizedHtmlDocument::root(self)
    }
}

#[cfg(test)]
mod xpath_node_tests {
    // Phase 06: xpath_eval::Node/Document for NormalizedNode/
    // NormalizedHtmlDocument. See this module's doc comment on the
    // `impl xpath_eval::Node` block for scope (no namespace nodes, no PI
    // nodes, `is_id_attribute` stays the trait default).
    use xpath_eval::{Document, Node, NodeKind};

    use super::{NormalizedHtmlDocument, normalize};
    use crate::parse::parse;

    fn normalize_html(html: &str) -> NormalizedHtmlDocument {
        let parsed = parse(html);
        normalize(parsed.document(), parsed.source())
    }

    /// The element children of `node`, in document order — filters out
    /// `Text`/`Comment` children so callers can navigate by position
    /// without tripping over implicit whitespace text nodes.
    fn element_children<'a>(
        node: super::NormalizedNode<'a>,
    ) -> impl Iterator<Item = super::NormalizedNode<'a>> {
        node.children().filter(|n| n.kind() == NodeKind::Element)
    }

    fn html_element(document: &NormalizedHtmlDocument) -> super::NormalizedNode<'_> {
        element_children(document.root())
            .next()
            .expect("expected <html>")
    }

    /// `<html>`'s second element child — `<head>` is always synthesized
    /// first, `<body>` second.
    fn body_element(document: &NormalizedHtmlDocument) -> super::NormalizedNode<'_> {
        element_children(html_element(document))
            .nth(1)
            .expect("expected <body> as html's second element child")
    }

    #[test]
    fn root_kind_and_parentless() {
        let document = normalize_html("<p>hi</p>");
        let root = document.root();
        assert_eq!(root.kind(), NodeKind::Root);
        assert_eq!(root.parent(), None);
    }

    /// `NormalizedHtmlDocument::root()` is also an inherent method (used
    /// throughout this file's other tests) that happens to shadow the
    /// `Document::root()` trait method with identical behavior — see the
    /// `impl xpath_eval::Node` block's doc comment on this kind of
    /// collision. Calling it through `Document::root(...)` explicitly here
    /// proves the *trait* impl itself is correct, not just the inherent
    /// method it shadows.
    #[test]
    fn document_root_is_reachable_through_the_trait_too() {
        let document = normalize_html("<p>hi</p>");
        assert_eq!(Document::root(&document), document.root());
    }

    /// Exercises `NormalizedNode` in a genuinely generic `N: Node<'a>`
    /// context, the way `schematron_engine::evaluate<D: Document>` will
    /// use it — a trait-bound mismatch here would not necessarily show up
    /// in the other, concrete-type tests in this module.
    #[test]
    fn satisfies_the_node_trait_bound_generically() {
        fn generic_kind<'a, N: Node<'a>>(node: N) -> NodeKind {
            node.kind()
        }

        let document = normalize_html("<p>hi</p>");
        assert_eq!(generic_kind(document.root()), NodeKind::Root);
    }

    #[test]
    fn element_kind_and_expanded_name() {
        let document = normalize_html("<p>hi</p>");
        let html = html_element(&document);
        assert_eq!(html.kind(), NodeKind::Element);
        let name = html.expanded_name().expect("element should have a name");
        assert_eq!(name.namespace_uri.as_deref(), Some(super::XHTML_NAMESPACE));
        assert_eq!(name.local_name, "html");
    }

    #[test]
    fn attribute_node_kind_name_value_and_parent() {
        let document = normalize_html(r#"<p id="x">hi</p>"#);
        let body = body_element(&document);
        let p = element_children(body).next().expect("expected <p>");

        let attribute = p
            .attributes()
            .next()
            .expect("<p id=\"x\"> should have one attribute node");
        assert_eq!(attribute.kind(), NodeKind::Attribute);
        let name = attribute
            .expanded_name()
            .expect("attribute should have a name");
        assert_eq!(name.local_name, "id");
        assert_eq!(attribute.string_value(), "x");
        assert_eq!(attribute.parent(), Some(p));
    }

    #[test]
    fn text_node_string_value_is_its_content() {
        let document = normalize_html("<p>hi</p>");
        let body = body_element(&document);
        let p = element_children(body).next().expect("expected <p>");
        let text = p
            .children()
            .find(|n| n.kind() == NodeKind::Text)
            .expect("expected a text child");
        assert_eq!(text.string_value(), "hi");
    }

    #[test]
    fn comment_node_string_value_is_its_content() {
        let document = normalize_html("<p><!--hello--></p>");
        let body = body_element(&document);
        let p = element_children(body).next().expect("expected <p>");
        let comment = p
            .children()
            .find(|n| n.kind() == NodeKind::Comment)
            .expect("expected a comment child");
        assert_eq!(comment.string_value(), "hello");
    }

    #[test]
    fn element_string_value_concatenates_descendant_text_and_skips_comments() {
        let document = normalize_html("<div>a<!--x-->b<span>c</span></div>");
        let body = body_element(&document);
        let div = element_children(body).next().expect("expected <div>");
        assert_eq!(div.string_value(), "abc");
    }

    #[test]
    fn namespaces_are_always_empty() {
        let document = normalize_html("<p>hi</p>");
        let html = html_element(&document);
        assert_eq!(html.namespaces().count(), 0);
    }

    #[test]
    fn document_order_matches_arena_build_order() {
        let document = normalize_html(r#"<p id="x">hi</p>"#);
        let html = html_element(&document);
        let body = body_element(&document);
        let p = element_children(body).next().expect("expected <p>");
        let attribute = p.attributes().next().expect("expected id attribute");
        let text = p
            .children()
            .find(|n| n.kind() == NodeKind::Text)
            .expect("expected text child");

        use std::cmp::Ordering;
        assert_eq!(html.document_order(body), Ordering::Less);
        // An element's own node precedes its attribute nodes, which precede
        // its children — see `normalize_node`'s doc comment.
        assert_eq!(p.document_order(attribute), Ordering::Less);
        assert_eq!(attribute.document_order(text), Ordering::Less);
        assert_eq!(p.document_order(p), Ordering::Equal);
    }
}

#[cfg(test)]
mod tests {
    // Phase 04a input-normalization spike: matrix of HTML shapes the
    // adapter must normalize correctly, plus the provenance invariant
    // ("never invents a position"). See plan/04a-input-normalization.md.
    // Phase 06: rewritten against the arena/handle shape — see this
    // module's doc comment.
    use super::{
        CUSTOM_ELEMENT_NAMESPACE, MATHML_NAMESPACE, NormalizedHtmlDocument, NormalizedNode,
        SVG_NAMESPACE, XHTML_NAMESPACE, XML_NAMESPACE, normalize,
    };
    use crate::parse::parse;

    fn normalize_html(html: &str) -> NormalizedHtmlDocument {
        let parsed = parse(html);
        normalize(parsed.document(), parsed.source())
    }

    fn only_element<'a>(document: &'a NormalizedHtmlDocument) -> NormalizedNode<'a> {
        let mut children = document.children();
        let only = children.next().expect("document should have one child");
        assert!(children.next().is_none(), "expected exactly one child");
        only
    }

    fn expect_element<'a>(
        node: NormalizedNode<'a>,
        namespace: &str,
        local_name: &str,
    ) -> Vec<NormalizedNode<'a>> {
        use relax_ng::Element;
        assert_eq!(node.name().namespace.as_deref(), Some(namespace));
        assert_eq!(node.name().local, local_name);
        node.child_nodes().collect()
    }

    fn find_element<'a>(children: &[NormalizedNode<'a>], local_name: &str) -> NormalizedNode<'a> {
        use relax_ng::Element;
        *children
            .iter()
            .find(|child| child.name().local == local_name)
            .unwrap_or_else(|| panic!("expected to find element {local_name:?}"))
    }

    fn expect_text(node: NormalizedNode<'_>, expected: &str) {
        match &node.data().kind {
            super::Kind::Text { content } => assert_eq!(content, expected),
            other => panic!("expected text {expected:?}, got {other:?}"),
        }
    }

    #[test]
    fn document_root_is_document_root() {
        let document = normalize_html("<p>hi</p>");
        let html = only_element(&document);
        assert!(!html.is_document_root());
        let root = document.children().next().unwrap();
        assert_eq!(root.id.0, 1);
    }

    #[test]
    fn implicit_html_head_body_are_synthesized_with_xhtml_namespace() {
        use relax_ng::Element;
        let document = normalize_html("<p>hi</p>");

        let html = only_element(&document);
        assert_eq!(html.name().local, "html");
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");

        let head = find_element(&html_children, "head");
        assert_eq!(head.name().local, "head");
        expect_element(head, XHTML_NAMESPACE, "head");

        let body = find_element(&html_children, "body");
        assert_eq!(body.name().local, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");

        let p = find_element(&body_children, "p");
        assert_eq!(p.name().local, "p");
        let p_children = expect_element(p, XHTML_NAMESPACE, "p");
        expect_text(p_children[0], "hi");
    }

    #[test]
    fn optional_end_tags_produce_sibling_elements() {
        let document = normalize_html("<ul><li>a<li>b</ul>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let ul = find_element(&body_children, "ul");
        let ul_children = expect_element(ul, XHTML_NAMESPACE, "ul");

        use relax_ng::Element;
        let li_elements: Vec<_> = ul_children
            .iter()
            .filter(|child| child.name().local == "li")
            .collect();
        assert_eq!(li_elements.len(), 2);

        let first_li_children = expect_element(*li_elements[0], XHTML_NAMESPACE, "li");
        expect_text(first_li_children[0], "a");

        let second_li_children = expect_element(*li_elements[1], XHTML_NAMESPACE, "li");
        expect_text(second_li_children[0], "b");
    }

    #[test]
    fn svg_elements_keep_svg_namespace() {
        use relax_ng::Element;
        let document = normalize_html("<svg><circle/></svg>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let svg = find_element(&body_children, "svg");
        assert_eq!(svg.name().local, "svg");
        let svg_children = expect_element(svg, SVG_NAMESPACE, "svg");
        let circle = find_element(&svg_children, "circle");
        assert_eq!(circle.name().local, "circle");
        expect_element(circle, SVG_NAMESPACE, "circle");
    }

    #[test]
    fn mathml_elements_keep_mathml_namespace() {
        use relax_ng::Element;
        let document = normalize_html("<math><mi>x</mi></math>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let math = find_element(&body_children, "math");
        assert_eq!(math.name().local, "math");
        let math_children = expect_element(math, MATHML_NAMESPACE, "math");
        let mi = find_element(&math_children, "mi");
        assert_eq!(mi.name().local, "mi");
        let mi_children = expect_element(mi, MATHML_NAMESPACE, "mi");
        expect_text(mi_children[0], "x");
    }

    #[test]
    fn script_and_style_content_normalize_to_plain_text() {
        use relax_ng::Element;
        let script_document = normalize_html("<script>1 < 2;</script>");
        let script_html = only_element(&script_document);
        let script_html_children = expect_element(script_html, XHTML_NAMESPACE, "html");
        let head = find_element(&script_html_children, "head");
        let head_children = expect_element(head, XHTML_NAMESPACE, "head");
        let script = find_element(&head_children, "script");
        assert_eq!(script.name().local, "script");
        let script_children = expect_element(script, XHTML_NAMESPACE, "script");
        expect_text(script_children[0], "1 < 2;");

        let style_document = normalize_html("<style>a{color:red}</style>");
        let style_html = only_element(&style_document);
        let style_html_children = expect_element(style_html, XHTML_NAMESPACE, "html");
        let style_head = find_element(&style_html_children, "head");
        let style_head_children = expect_element(style_head, XHTML_NAMESPACE, "head");
        let style = find_element(&style_head_children, "style");
        assert_eq!(style.name().local, "style");
        let style_children = expect_element(style, XHTML_NAMESPACE, "style");
        expect_text(style_children[0], "a{color:red}");
    }

    #[test]
    fn named_entities_resolve_to_decoded_text() {
        use relax_ng::Element;
        let document = normalize_html("<p>&amp; &copy;</p>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        assert_eq!(p.name().local, "p");
        let p_children = expect_element(p, XHTML_NAMESPACE, "p");
        expect_text(p_children[0], "& \u{a9}");
    }

    #[test]
    fn xml_lang_attribute_keeps_literal_local_name_on_html_elements() {
        // The XPath-facing view (what the assertion/Schematron layer sees)
        // keeps a literal `xml:lang` attribute as a single, unsplit local
        // name with no namespace — matching real HTML5 parsing (foreign-
        // attribute namespace-splitting only applies inside SVG/MathML).
        // The *schema*-facing view intentionally diverges from this (see
        // `attributes_are_remapped_to_the_split_xml-namespace_form_for_the_schema_layer`
        // below) — this test is specifically about the XPath layer.
        let document = normalize_html(r#"<p xml:lang="de">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        let attribute = p
            .attribute_nodes()
            .next()
            .expect("p should have one attribute");

        assert_eq!(
            xpath_eval::Node::expanded_name(attribute),
            Some(xpath_eval::ExpandedName {
                namespace_uri: None,
                local_name: "xml:lang".to_owned(),
            })
        );
    }

    #[test]
    fn schema_layer_remaps_plain_lang_to_the_split_xml_namespace_form() {
        use relax_ng::Element;
        let document = normalize_html(r#"<p lang="de">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");

        let attributes: Vec<_> = p.attributes().collect();
        assert_eq!(
            attributes,
            vec![(
                relax_ng::ExpandedName {
                    namespace: Some(XML_NAMESPACE.to_owned()),
                    local: "lang".to_owned(),
                },
                "de".to_owned(),
            )]
        );
    }

    #[test]
    fn schema_layer_remaps_literal_xml_lang_to_the_split_xml_namespace_form() {
        use relax_ng::Element;
        let document = normalize_html(r#"<p xml:lang="de">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");

        let attributes: Vec<_> = p.attributes().collect();
        assert_eq!(
            attributes,
            vec![(
                relax_ng::ExpandedName {
                    namespace: Some(XML_NAMESPACE.to_owned()),
                    local: "lang".to_owned(),
                },
                "de".to_owned(),
            )]
        );
    }

    #[test]
    fn schema_layer_prefers_literal_xml_lang_value_over_plain_lang_when_both_present() {
        use relax_ng::Element;
        let document = normalize_html(r#"<p lang="de" xml:lang="fr">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        let attributes: Vec<_> = p.attributes().collect();

        // Both attributes are present in the source; the schema-facing
        // view yields the split xml:lang name exactly once, with the
        // literal `xml:lang`'s value — never `lang`'s, and never both (see
        // this `attributes()` impl's doc comment for what's
        // corpus-verified here and what isn't).
        assert_eq!(
            attributes,
            vec![(
                relax_ng::ExpandedName {
                    namespace: Some(XML_NAMESPACE.to_owned()),
                    local: "lang".to_owned(),
                },
                "fr".to_owned(),
            )]
        );
    }

    #[test]
    fn schema_layer_drops_data_star_attributes() {
        use relax_ng::Element;
        let document = normalize_html(r#"<p data-z="" data-z:foo="">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        let attributes: Vec<_> = p.attributes().collect();

        assert_eq!(attributes, Vec::new());
    }

    #[test]
    fn schema_layer_drops_xmlns_attributes() {
        use relax_ng::Element;
        // html/svg/svg-transform-origin-transform-box-isvalid.html: an
        // `xmlns` declaration on the `svg` root itself (adjusted into
        // the XMLNS namespace by HTML5 parsing's "adjust foreign
        // attributes" step) and a plain, unadjusted `xmlns` on an XHTML
        // `div` inside a `foreignObject` — both must be invisible to the
        // schema layer, neither is a real content attribute.
        let document = normalize_html(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><div xmlns="http://www.w3.org/1999/xhtml"></div></foreignObject></svg>"#,
        );

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let svg = find_element(&body_children, "svg");
        assert_eq!(svg.attributes().collect::<Vec<_>>(), Vec::new());

        let svg_children = expect_element(svg, SVG_NAMESPACE, "svg");
        let foreign_object = find_element(&svg_children, "foreignObject");
        let foreign_object_children =
            expect_element(foreign_object, SVG_NAMESPACE, "foreignObject");
        let div = find_element(&foreign_object_children, "div");
        assert_eq!(div.attributes().collect::<Vec<_>>(), Vec::new());
    }

    #[test]
    fn schema_layer_keeps_bare_data_hyphen_attribute() {
        use relax_ng::Element;
        let document = normalize_html(r#"<p data-="">hi</p>"#);

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        let attributes: Vec<_> = p.attributes().collect();

        // Bare `data-` (nothing after the hyphen) is NOT a valid custom
        // data attribute — left unmapped so the schema's own
        // "unexpected attribute" rejection still fires for it.
        assert_eq!(
            attributes,
            vec![(
                relax_ng::ExpandedName {
                    namespace: None,
                    local: "data-".to_owned(),
                },
                String::new(),
            )]
        );
    }

    #[test]
    fn schema_layer_validates_svg_subtree_for_real() {
        use relax_ng::Element;
        let document = normalize_html("<p>before<svg><path/><circle/></svg>after</p>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");

        // Since the Phase 08 SVG/MathML schema vendoring
        // (`xtask/vendor-svg-mathml.sh`), the `svg` element is a real,
        // present `Content::Element` here — no longer merged away like a
        // `Comment`. `before`/`after` stay as two *separate* text runs
        // (not concatenated into one "beforeafter"), since the `svg`
        // element genuinely sits between them now.
        let children: Vec<_> = p.children().collect();
        assert_eq!(children.len(), 3, "expected before-text, svg, after-text");
        assert_eq!(children[0], relax_ng::Content::Text("before".to_owned()));
        assert!(matches!(children[1], relax_ng::Content::Element(_)));
        assert_eq!(children[2], relax_ng::Content::Text("after".to_owned()));
        let relax_ng::Content::Element(svg) = children[1] else {
            unreachable!("just matched Content::Element above");
        };
        assert_eq!(svg.name().namespace.as_deref(), Some(SVG_NAMESPACE));
        assert_eq!(svg.name().local, "svg");
    }

    #[test]
    fn schema_layer_validates_mathml_subtree_for_real() {
        use relax_ng::Element;
        let document = normalize_html("<p>before<math><mi>x</mi></math>after</p>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");

        let children: Vec<_> = p.children().collect();
        assert_eq!(children.len(), 3, "expected before-text, math, after-text");
        assert_eq!(children[0], relax_ng::Content::Text("before".to_owned()));
        assert!(matches!(children[1], relax_ng::Content::Element(_)));
        assert_eq!(children[2], relax_ng::Content::Text("after".to_owned()));
        let relax_ng::Content::Element(math) = children[1] else {
            unreachable!("just matched Content::Element above");
        };
        assert_eq!(math.name().namespace.as_deref(), Some(MATHML_NAMESPACE));
        assert_eq!(math.name().local, "math");
    }

    #[test]
    fn custom_element_keeps_xhtml_namespace_for_xpath_but_not_schema() {
        // XPath-facing view (xpath_eval::Node::expanded_name, what the
        // assertion/Schematron layer sees): a custom element gets the
        // ordinary XHTML namespace like any other plain element, matching
        // real HTML5 parsing (there is no such thing as a "custom
        // element namespace" in the DOM/parsing algorithm itself).
        let document = normalize_html("<my-widget>hi</my-widget>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let widget = find_element(&body_children, "my-widget");
        assert_eq!(
            xpath_eval::Node::expanded_name(widget),
            Some(xpath_eval::ExpandedName {
                namespace_uri: Some(XHTML_NAMESPACE.to_owned()),
                local_name: "my-widget".to_owned(),
            })
        );
        let widget_children: Vec<_> = xpath_eval::Node::children(widget).collect();
        expect_text(widget_children[0], "hi");
    }

    #[test]
    fn custom_element_gets_the_vnu_custom_element_namespace_for_schema() {
        // Schema-facing view (relax_ng::Element::name): remapped to
        // CUSTOM_ELEMENT_NAMESPACE so the vendored schema's `element c:*`
        // wildcard (schema/html5/web-components.rnc) can match it —
        // confirmed necessary against the vendored corpus, see
        // CUSTOM_ELEMENT_NAMESPACE's own doc comment.
        use relax_ng::Element;
        let document = normalize_html("<my-widget>hi</my-widget>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let widget = find_element(&body_children, "my-widget");
        assert_eq!(
            widget.name().namespace.as_deref(),
            Some(CUSTOM_ELEMENT_NAMESPACE)
        );
        assert_eq!(widget.name().local, "my-widget");
    }

    #[test]
    fn plain_element_with_a_hyphenless_name_keeps_xhtml_namespace_for_schema_too() {
        // Sanity check for the custom-element-name gate itself: an
        // ordinary element (no hyphen in its name) must never be
        // misidentified as a custom element and remapped.
        use relax_ng::Element;
        let document = normalize_html("<p>hi</p>");

        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        assert_eq!(p.name().namespace.as_deref(), Some(XHTML_NAMESPACE));
    }

    #[test]
    fn synthesized_nodes_have_no_position_but_explicit_ones_do() {
        // Bare `<p>hi</p>` triggers both explicit (`p`, its text content)
        // and parser-inserted implicit (`html`/`head`/`body`) elements,
        // exercising both cases — since the Phase 08 html5-parser
        // migration (see plan/DECISIONS.md), explicit nodes carry a real
        // position; only html5-parser's own synthesized nodes still have
        // none (matching html5_parser::Node's own documented contract).
        let document = normalize_html("<p>hi</p>");

        let html = only_element(&document);
        assert!(html.position().is_none(), "implicit <html> has no position");
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let head = find_element(&html_children, "head");
        assert!(head.position().is_none(), "implicit <head> has no position");
        let body = find_element(&html_children, "body");
        assert!(body.position().is_none(), "implicit <body> has no position");

        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let p = find_element(&body_children, "p");
        assert!(
            p.position().is_some(),
            "explicit <p> should have a position"
        );

        let p_children = expect_element(p, XHTML_NAMESPACE, "p");
        let text = p_children[0];
        assert!(
            text.position().is_some(),
            "explicit text content should have a position"
        );
    }

    #[test]
    fn parent_of_top_level_node_is_the_synthetic_root() {
        let document = normalize_html("<p>hi</p>");
        let html = only_element(&document);
        let parent = html.parent().expect("html should have a parent");
        assert!(parent.is_document_root());
        assert!(parent.parent().is_none(), "the root itself has no parent");
    }

    #[test]
    fn parent_of_nested_element_round_trips_to_the_child() {
        use relax_ng::Element;
        let document = normalize_html("<div><p>hi</p></div>");
        let html = only_element(&document);
        let html_children = expect_element(html, XHTML_NAMESPACE, "html");
        let body = find_element(&html_children, "body");
        let body_children = expect_element(body, XHTML_NAMESPACE, "body");
        let div = find_element(&body_children, "div");
        let div_children: Vec<_> = div.child_nodes().collect();
        let p = find_element(&div_children, "p");

        let parent = p.parent().expect("p should have a parent");
        assert_eq!(parent.name().local, "div");
    }
}

#[cfg(test)]
mod element_adapter_tests {
    // Phase 05a: relax_ng::Element impl for NormalizedNode. See
    // plan/05a-element-adapter.md. Phase 06: rewritten against the
    // arena/handle shape — see this module's doc comment.
    use relax_ng::{Content, Element};

    use super::{NormalizedHtmlDocument, NormalizedNode, normalize};
    use crate::parse::parse;

    fn normalize_html(html: &str) -> NormalizedHtmlDocument {
        let parsed = parse(html);
        normalize(parsed.document(), parsed.source())
    }

    /// Finds the first descendant element (in document order, including
    /// `node` itself) with the given local name. Panics if none is found.
    fn find_by_local_name<'a>(node: NormalizedNode<'a>, local_name: &str) -> NormalizedNode<'a> {
        fn search<'a>(node: NormalizedNode<'a>, local_name: &str) -> Option<NormalizedNode<'a>> {
            if matches!(&node.data().kind, super::Kind::Element{ name, .. } if name.local_name == local_name)
            {
                return Some(node);
            }
            node.child_nodes()
                .find_map(|child| search(child, local_name))
        }

        search(node, local_name)
            .unwrap_or_else(|| panic!("expected to find element {local_name:?}"))
    }

    fn first_child(document: &NormalizedHtmlDocument) -> NormalizedNode<'_> {
        document
            .children()
            .next()
            .expect("document should have a child")
    }

    #[test]
    fn element_with_child_elements_yields_content_element_per_child() {
        let document = normalize_html("<div><p>a</p><p>b</p></div>");
        let div = find_by_local_name(first_child(&document), "div");

        let content: Vec<_> = div.children().collect();
        assert_eq!(content.len(), 2);
        for item in &content {
            match item {
                Content::Element(element) => assert_eq!(element.name().local, "p"),
                Content::Text(text) => panic!("expected element child, got text {text:?}"),
            }
        }
    }

    #[test]
    fn element_with_text_content_yields_content_text() {
        let document = normalize_html("<p>hi</p>");
        let p = find_by_local_name(first_child(&document), "p");

        let content: Vec<_> = p.children().collect();
        assert_eq!(content, vec![Content::Text("hi".to_owned())]);
    }

    #[test]
    fn mixed_content_yields_correct_content_sequence() {
        let document = normalize_html("<p>a<b>x</b>c</p>");
        let p = find_by_local_name(first_child(&document), "p");

        let content: Vec<_> = p.children().collect();
        assert_eq!(content.len(), 3);
        assert_eq!(content[0], Content::Text("a".to_owned()));
        match &content[1] {
            Content::Element(element) => assert_eq!(element.name().local, "b"),
            other => panic!("expected element child, got {other:?}"),
        }
        assert_eq!(content[2], Content::Text("c".to_owned()));
    }

    #[test]
    fn comment_child_is_skipped_and_surrounding_text_is_merged() {
        let document = normalize_html("<p>a<!--x-->b</p>");
        let p = find_by_local_name(first_child(&document), "p");

        let underlying_child_count = p.child_nodes().count();
        assert_eq!(underlying_child_count, 3, "expected Text, Comment, Text");

        // Per relax_ng::Content::Text's contract, text nodes adjacent around
        // a dropped comment must be merged into a single Text before
        // reaching the validator — not yielded as two separate items.
        let content: Vec<_> = p.children().collect();
        assert_eq!(content, vec![Content::Text("ab".to_owned())]);
    }

    #[test]
    fn comment_between_elements_with_no_adjacent_text_yields_nothing() {
        let document = normalize_html("<div><p>a</p><!--x--><p>b</p></div>");
        let div = find_by_local_name(first_child(&document), "div");

        let underlying_child_count = div.child_nodes().count();
        assert_eq!(
            underlying_child_count, 3,
            "expected Element, Comment, Element"
        );

        let content: Vec<_> = div.children().collect();
        assert_eq!(
            content.len(),
            2,
            "the comment must not produce an empty Content::Text"
        );
        for item in &content {
            match item {
                Content::Element(element) => assert_eq!(element.name().local, "p"),
                other => panic!("expected only element children, got {other:?}"),
            }
        }
    }

    #[test]
    fn attributes_map_namespace_and_local_name() {
        let document = normalize_html(r#"<a xlink:href="https://example.com">x</a>"#);
        // Not a realistic HTML5 parse (the HTML parser wouldn't produce an
        // `xlink:href`-namespaced attribute on a plain `<a>`), so build the
        // assertion against a real parsed document's `<a>` instead, keeping
        // the same attribute-mapping intent as the Phase 05a original: a
        // namespaced attribute's namespace/local-name both round-trip.
        let a = find_by_local_name(first_child(&document), "a");
        let attributes: Vec<_> = a.attributes().collect();
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].0.local, "xlink:href");
        assert_eq!(attributes[0].1, "https://example.com");
    }

    #[test]
    fn location_is_some_for_an_explicit_element_since_the_html5_parser_migration() {
        // Was `None` unconditionally under the old xmloxide-based parser
        // (it never tracked per-node positions at all, see
        // plan/DECISIONS.md's Phase 04a entries) — the Phase 08 migration
        // to html5-parser closes that gap for explicit (non-synthesized)
        // nodes; see `synthesized_nodes_have_no_position_but_explicit_ones_do`
        // in the sibling `tests` module for the synthesized-node case.
        // Structured since the later Phase 08 `relax_ng::Element::Location`
        // API change — was a formatted `"1:1"` string before that.
        let document = normalize_html("<p>hi</p>");
        let p = find_by_local_name(first_child(&document), "p");

        assert_eq!(
            p.location(),
            Some(crate::finding::SourceLocation {
                line: 1,
                column: 1,
                byte_offset: 0,
            })
        );
    }

    #[test]
    fn normalize_never_produces_two_adjacent_text_siblings() {
        // A comment sitting between two text runs must remain a real,
        // separate Comment sibling — not be dropped by normalize() in a way
        // that would leave two Text children directly adjacent to each
        // other.
        let document = normalize_html("<p>a<!--x-->b</p>");
        let p = find_by_local_name(first_child(&document), "p");

        fn assert_no_adjacent_text_siblings(node: NormalizedNode<'_>) {
            let children: Vec<_> = node.child_nodes().collect();
            for window in children.windows(2) {
                assert!(
                    !matches!(
                        (&window[0].data().kind, &window[1].data().kind),
                        (super::Kind::Text { .. }, super::Kind::Text { .. })
                    ),
                    "found two adjacent Text siblings"
                );
            }
            for child in children {
                assert_no_adjacent_text_siblings(child);
            }
        }

        assert_no_adjacent_text_siblings(p);
    }
}
