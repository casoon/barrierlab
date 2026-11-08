use super::*;

/// The XLink namespace URI (https://infra.spec.whatwg.org/#xlink-namespace).
pub(super) const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";
/// The XML namespace URI.
pub(super) const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
/// The XMLNS namespace URI.
pub(super) const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// "Adjust foreign attributes" (§13.2.6.1): attribute names that get
/// reassigned to a namespaced identity (a prefix, local name, and
/// namespace) rather than staying a single unqualified name — mostly
/// XLink attributes on SVG elements. This crate's `document::Attribute`
/// has no separate prefix/local-name split, so this table maps the full
/// original name straight to the namespace URI, keeping `name` as-is
/// (e.g. `"xlink:href"` stays `"xlink:href"`, gains
/// `namespace: Some(XLINK_NAMESPACE)`).
pub(super) const FOREIGN_ATTRIBUTE_NAMESPACES: &[(&str, &str)] = &[
    ("xlink:actuate", XLINK_NAMESPACE),
    ("xlink:arcrole", XLINK_NAMESPACE),
    ("xlink:href", XLINK_NAMESPACE),
    ("xlink:role", XLINK_NAMESPACE),
    ("xlink:show", XLINK_NAMESPACE),
    ("xlink:title", XLINK_NAMESPACE),
    ("xlink:type", XLINK_NAMESPACE),
    ("xml:lang", XML_NAMESPACE),
    ("xml:space", XML_NAMESPACE),
    ("xmlns", XMLNS_NAMESPACE),
    ("xmlns:xlink", XMLNS_NAMESPACE),
];

/// "Adjust SVG attributes" (§13.2.6.1): attribute names that are
/// case-fixed when found on an SVG-namespace element's token (token
/// attribute names always arrive all-lowercase from the tokenizer,
/// which has no namespace awareness).
pub(super) const SVG_ATTRIBUTE_ADJUSTMENTS: &[(&str, &str)] = &[
    ("attributename", "attributeName"),
    ("attributetype", "attributeType"),
    ("basefrequency", "baseFrequency"),
    ("baseprofile", "baseProfile"),
    ("calcmode", "calcMode"),
    ("clippathunits", "clipPathUnits"),
    ("diffuseconstant", "diffuseConstant"),
    ("edgemode", "edgeMode"),
    ("filterunits", "filterUnits"),
    ("glyphref", "glyphRef"),
    ("gradienttransform", "gradientTransform"),
    ("gradientunits", "gradientUnits"),
    ("kernelmatrix", "kernelMatrix"),
    ("kernelunitlength", "kernelUnitLength"),
    ("keypoints", "keyPoints"),
    ("keysplines", "keySplines"),
    ("keytimes", "keyTimes"),
    ("lengthadjust", "lengthAdjust"),
    ("limitingconeangle", "limitingConeAngle"),
    ("markerheight", "markerHeight"),
    ("markerunits", "markerUnits"),
    ("markerwidth", "markerWidth"),
    ("maskcontentunits", "maskContentUnits"),
    ("maskunits", "maskUnits"),
    ("numoctaves", "numOctaves"),
    ("pathlength", "pathLength"),
    ("patterncontentunits", "patternContentUnits"),
    ("patterntransform", "patternTransform"),
    ("patternunits", "patternUnits"),
    ("pointsatx", "pointsAtX"),
    ("pointsaty", "pointsAtY"),
    ("pointsatz", "pointsAtZ"),
    ("preservealpha", "preserveAlpha"),
    ("preserveaspectratio", "preserveAspectRatio"),
    ("primitiveunits", "primitiveUnits"),
    ("refx", "refX"),
    ("refy", "refY"),
    ("repeatcount", "repeatCount"),
    ("repeatdur", "repeatDur"),
    ("requiredextensions", "requiredExtensions"),
    ("requiredfeatures", "requiredFeatures"),
    ("specularconstant", "specularConstant"),
    ("specularexponent", "specularExponent"),
    ("spreadmethod", "spreadMethod"),
    ("startoffset", "startOffset"),
    ("stddeviation", "stdDeviation"),
    ("stitchtiles", "stitchTiles"),
    ("surfacescale", "surfaceScale"),
    ("systemlanguage", "systemLanguage"),
    ("tablevalues", "tableValues"),
    ("targetx", "targetX"),
    ("targety", "targetY"),
    ("textlength", "textLength"),
    ("viewbox", "viewBox"),
    ("viewtarget", "viewTarget"),
    ("xchannelselector", "xChannelSelector"),
    ("ychannelselector", "yChannelSelector"),
    ("zoomandpan", "zoomAndPan"),
];

/// "Adjust MathML attributes" (§13.2.6.1) — trivial compared to SVG's:
/// only `definitionurl` is case-fixed.
pub(super) const MATHML_ATTRIBUTE_ADJUSTMENTS: &[(&str, &str)] =
    &[("definitionurl", "definitionURL")];

/// Applies "adjust MathML attributes"/"adjust SVG attributes"/"adjust
/// foreign attributes" (§13.2.6.1) to `tag`'s attributes for an element
/// being inserted into `namespace` (MathML or SVG — never called for
/// HTML-namespace insertions, see
/// [`create_element_for_token`](TreeBuilder::create_element_for_token)).
/// The MathML/SVG name-casing fixups apply only when `namespace`
/// matches; the foreign-attribute namespacing always applies,
/// regardless of which foreign namespace this is.
pub(super) fn adjust_attributes_for_foreign_element(
    tag: &TagToken,
    namespace: &str,
) -> Vec<Attribute> {
    let name_adjustments: &[(&str, &str)] = if namespace == MATHML_NAMESPACE {
        MATHML_ATTRIBUTE_ADJUSTMENTS
    } else if namespace == SVG_NAMESPACE {
        SVG_ATTRIBUTE_ADJUSTMENTS
    } else {
        &[]
    };
    tag.attributes
        .iter()
        .map(|attribute| {
            let name = name_adjustments
                .iter()
                .find(|&&(from, _)| from == attribute.name)
                .map_or_else(|| attribute.name.clone(), |&(_, to)| to.to_owned());
            let namespace = FOREIGN_ATTRIBUTE_NAMESPACES
                .iter()
                .find(|&&(from, _)| from == name)
                .map(|&(_, ns)| ns.to_owned());
            Attribute {
                name,
                value: attribute.value.clone(),
                namespace,
            }
        })
        .collect()
}

/// "Adjust SVG tag names" — the table foreign content's "any other
/// start tag" rule (§13.2.6.5) uses to case-fix an SVG element's tag
/// name (token tag names always arrive all-lowercase from the
/// tokenizer). Returns `name` unchanged if it isn't in the table.
pub(super) const SVG_TAG_NAME_ADJUSTMENTS: &[(&str, &str)] = &[
    ("altglyph", "altGlyph"),
    ("altglyphdef", "altGlyphDef"),
    ("altglyphitem", "altGlyphItem"),
    ("animatecolor", "animateColor"),
    ("animatemotion", "animateMotion"),
    ("animatetransform", "animateTransform"),
    ("clippath", "clipPath"),
    ("feblend", "feBlend"),
    ("fecolormatrix", "feColorMatrix"),
    ("fecomponenttransfer", "feComponentTransfer"),
    ("fecomposite", "feComposite"),
    ("feconvolvematrix", "feConvolveMatrix"),
    ("fediffuselighting", "feDiffuseLighting"),
    ("fedisplacementmap", "feDisplacementMap"),
    ("fedistantlight", "feDistantLight"),
    ("fedropshadow", "feDropShadow"),
    ("feflood", "feFlood"),
    ("fefunca", "feFuncA"),
    ("fefuncb", "feFuncB"),
    ("fefuncg", "feFuncG"),
    ("fefuncr", "feFuncR"),
    ("fegaussianblur", "feGaussianBlur"),
    ("feimage", "feImage"),
    ("femerge", "feMerge"),
    ("femergenode", "feMergeNode"),
    ("femorphology", "feMorphology"),
    ("feoffset", "feOffset"),
    ("fepointlight", "fePointLight"),
    ("fespecularlighting", "feSpecularLighting"),
    ("fespotlight", "feSpotLight"),
    ("fetile", "feTile"),
    ("feturbulence", "feTurbulence"),
    ("foreignobject", "foreignObject"),
    ("glyphref", "glyphRef"),
    ("lineargradient", "linearGradient"),
    ("radialgradient", "radialGradient"),
    ("textpath", "textPath"),
];

pub(super) fn adjust_svg_tag_name(name: &str) -> &str {
    SVG_TAG_NAME_ADJUSTMENTS
        .iter()
        .find(|&&(from, _)| from == name)
        .map_or(name, |&(_, to)| to)
}

/// A node is a "MathML text integration point" (§13.2.6.5) if it's one
/// of these five MathML elements.
pub(super) fn is_mathml_text_integration_point(document: &Document, node: NodeId) -> bool {
    let NodeKind::Element {
        name, namespace, ..
    } = &document.node(node).kind
    else {
        return false;
    };
    namespace.as_deref() == Some(MATHML_NAMESPACE)
        && matches!(name.as_str(), "mi" | "mo" | "mn" | "ms" | "mtext")
}

/// A node is an "HTML integration point" (§13.2.6.5) if it's an SVG
/// `foreignObject`/`desc`/`title`, or a MathML `annotation-xml` whose
/// start tag had an `encoding` attribute (ASCII case-insensitive)
/// matching `"text/html"` or `"application/xhtml+xml"`. Checked against
/// the element's current attributes rather than a separately stored
/// flag — this crate never mutates attributes after element creation
/// (no script execution), so they're equivalent.
pub(super) fn is_html_integration_point(document: &Document, node: NodeId) -> bool {
    let NodeKind::Element {
        name,
        namespace,
        attributes,
    } = &document.node(node).kind
    else {
        return false;
    };
    match namespace.as_deref() {
        Some(MATHML_NAMESPACE) if name == "annotation-xml" => attributes.iter().any(|attribute| {
            attribute.name == "encoding"
                && (attribute.value.eq_ignore_ascii_case("text/html")
                    || attribute
                        .value
                        .eq_ignore_ascii_case("application/xhtml+xml"))
        }),
        Some(SVG_NAMESPACE) => matches!(name.as_str(), "foreignObject" | "desc" | "title"),
        _ => false,
    }
}

impl TreeBuilder {
    /// The tree construction dispatcher's own test (§13.2.6): true if
    /// `kind` should be processed "in foreign content" (§13.2.6.5)
    /// rather than by the current insertion mode's HTML-content rules.
    /// The "stack of open elements is empty" case returns `false`
    /// (HTML content) implicitly, since [`adjusted_current_node`](Self::adjusted_current_node)
    /// is `None` then.
    pub(super) fn should_process_as_foreign_content(&self, kind: &TokenKind) -> bool {
        let Some(node) = self.adjusted_current_node() else {
            return false;
        };
        if self.is_html_namespace_element(node) {
            return false;
        }
        if is_mathml_text_integration_point(&self.document, node) {
            match kind {
                TokenKind::StartTag(tag)
                    if !matches!(tag.name.as_str(), "mglyph" | "malignmark") =>
                {
                    return false;
                }
                TokenKind::Character(_) => return false,
                _ => {}
            }
        }
        if self.node_has_namespace_and_name(node, MATHML_NAMESPACE, "annotation-xml")
            && matches!(kind, TokenKind::StartTag(tag) if tag.name == "svg")
        {
            return false;
        }
        if is_html_integration_point(&self.document, node)
            && matches!(kind, TokenKind::StartTag(_) | TokenKind::Character(_))
        {
            return false;
        }
        if matches!(kind, TokenKind::Eof) {
            return false;
        }
        true
    }

    /// Pops elements off the stack of open elements while the current
    /// node is foreign content that isn't an integration point — the
    /// shared "pop back out of foreign content" step used by both of
    /// foreign content's implicit-close-and-reprocess cases (the block-
    /// element start-tag group and the `br`/`p` end tags, §13.2.6.5).
    pub(super) fn pop_out_of_foreign_content(&mut self) {
        while let Some(current) = self.open_elements.current_node() {
            if is_mathml_text_integration_point(&self.document, current)
                || is_html_integration_point(&self.document, current)
                || self.is_html_namespace_element(current)
            {
                break;
            }
            self.open_elements.pop();
        }
    }

    /// True if the current node is an SVG `script` element — used by
    /// the `</script>` special case (§13.2.6.5).
    pub(super) fn current_node_is_svg_script(&self) -> bool {
        self.open_elements
            .current_node()
            .is_some_and(|node| self.node_has_namespace_and_name(node, SVG_NAMESPACE, "script"))
    }

    /// Pops elements off the stack of open elements until `target` has
    /// been popped, by identity.
    pub(super) fn pop_until_node_popped(&mut self, target: NodeId) {
        loop {
            let popped = self.open_elements.pop();
            if popped == Some(target) {
                break;
            }
        }
    }

    /// "The rules for parsing tokens in foreign content" (§13.2.6.5).
    /// Script-execution machinery (the speculative parser, script
    /// nesting level, insertion-point save/restore, actually executing
    /// SVG `<script>`) is entirely out of scope, same as everywhere
    /// else in this crate — the `</script>` special case (whose real
    /// purpose is running the script at exactly the right moment)
    /// reduces to a plain pop once that's stripped out, same as the
    /// generic "any other end tag" fallback right below it.
    pub(super) fn process_token_foreign_content(
        &mut self,
        kind: &TokenKind,
        position: Position,
    ) -> TokenOutcome {
        match kind {
            TokenKind::Character(c) if *c == '\0' => {
                self.insert_character('\u{FFFD}', Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::Character(c) if is_whitespace(*c) => {
                self.insert_character(*c, Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::Character(c) => {
                self.insert_character(*c, Some(position));
                self.frameset_ok = false;
                TokenOutcome::Consumed(None)
            }
            TokenKind::Comment(data) => {
                self.insert_comment(data.clone(), Some(position));
                TokenOutcome::Consumed(None)
            }
            TokenKind::ProcessingInstruction(pi) => {
                self.insert_processing_instruction(
                    pi.target.clone(),
                    pi.data.clone(),
                    Some(position),
                );
                TokenOutcome::Consumed(None)
            }
            TokenKind::Doctype(_) => {
                // "A DOCTYPE token: Parse error. Ignore the token."
                self.error(ParseErrorKind::StrayDoctype, position);
                TokenOutcome::Consumed(None)
            }
            TokenKind::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "b" | "big"
                        | "blockquote"
                        | "body"
                        | "br"
                        | "center"
                        | "code"
                        | "dd"
                        | "div"
                        | "dl"
                        | "dt"
                        | "em"
                        | "embed"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "head"
                        | "hr"
                        | "i"
                        | "img"
                        | "li"
                        | "listing"
                        | "menu"
                        | "meta"
                        | "nobr"
                        | "ol"
                        | "p"
                        | "pre"
                        | "ruby"
                        | "s"
                        | "small"
                        | "span"
                        | "strong"
                        | "strike"
                        | "sub"
                        | "sup"
                        | "table"
                        | "tt"
                        | "u"
                        | "ul"
                        | "var"
                ) || (tag.name == "font"
                    && tag.attributes.iter().any(|attribute| {
                        matches!(attribute.name.as_str(), "color" | "face" | "size")
                    })) =>
            {
                // "Parse error. While the current node is not a MathML
                // text integration point, [...] pop" (§13.2.6.5).
                self.error(ParseErrorKind::ForeignContentBreakout, position);
                self.pop_out_of_foreign_content();
                TokenOutcome::Reprocess
            }
            TokenKind::EndTag(tag) if matches!(tag.name.as_str(), "br" | "p") => {
                self.error(ParseErrorKind::ForeignContentBreakout, position);
                self.pop_out_of_foreign_content();
                TokenOutcome::Reprocess
            }
            TokenKind::StartTag(tag) => {
                let namespace = self
                    .adjusted_current_node()
                    .and_then(|node| self.namespace_of(node))
                    .expect("foreign content dispatch always has a foreign adjusted current node");
                let adjusted_tag = if namespace == SVG_NAMESPACE {
                    TagToken {
                        name: adjust_svg_tag_name(&tag.name).to_owned(),
                        ..tag.clone()
                    }
                } else {
                    tag.clone()
                };
                self.insert_foreign_element(&adjusted_tag, &namespace, false, Some(position));
                if adjusted_tag.self_closing {
                    // Both self-closing branches (SVG <script>, acting
                    // as its own end tag below, vs. every other
                    // element) reduce to the same pop once script
                    // execution is stripped out.
                    self.open_elements.pop();
                    self.acknowledge_self_closing_flag();
                }
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) if tag.name == "script" && self.current_node_is_svg_script() => {
                self.open_elements.pop();
                TokenOutcome::Consumed(None)
            }
            TokenKind::EndTag(tag) => {
                let mut node = self.open_elements.current_node();
                loop {
                    let current = node.expect(
                        "the stack of open elements always contains at least html by the time \
                         this runs",
                    );
                    if self.open_elements.topmost() == Some(current) {
                        return TokenOutcome::Consumed(None);
                    }
                    if self.element_name_matches_ignore_case(current, &tag.name) {
                        // Step 2: "If node's tag name, converted to ASCII
                        // lowercase, is not the same as the tag name of
                        // the token, then this is a parse error." Only
                        // reported when a deeper match closes the
                        // intervening elements; with no match at all the
                        // HTML rules below report the stray end tag.
                        if self.open_elements.current_node() != Some(current) {
                            self.error(ParseErrorKind::ImpliedEndTagWithUnclosedElements, position);
                        }
                        self.pop_until_node_popped(current);
                        return TokenOutcome::Consumed(None);
                    }
                    let next = self
                        .open_elements
                        .element_immediately_above(current)
                        .expect("current is not topmost, so an element always sits above it");
                    if self.is_html_namespace_element(next) {
                        // Step 7: process the token per the current
                        // insertion mode's HTML content rules — a direct
                        // call, not `TokenOutcome::Reprocess` (which
                        // would just re-run `should_process_as_foreign_content`
                        // against the still-unchanged, still-foreign
                        // current node and dispatch back here forever;
                        // see `process_token_by_insertion_mode`'s doc
                        // comment).
                        return self.process_token_by_insertion_mode(kind, position);
                    }
                    node = Some(next);
                }
            }
            TokenKind::Eof => unreachable!(
                "should_process_as_foreign_content always returns false for an EOF token, \
                 so process_token never dispatches one here"
            ),
        }
    }

    /// True if `node`'s tag name matches `tag_name` under ASCII
    /// case-insensitive comparison — foreign content's "any other end
    /// tag" rule (§13.2.6.5) explicitly lowercases the (possibly
    /// mixed-case, e.g. SVG) node name before comparing it against the
    /// token's tag name (which always arrives all-lowercase from the
    /// tokenizer).
    pub(super) fn element_name_matches_ignore_case(&self, node: NodeId, tag_name: &str) -> bool {
        matches!(
            &self.document.node(node).kind,
            NodeKind::Element { name, .. } if name.eq_ignore_ascii_case(tag_name)
        )
    }
}
