use super::*;

/// A (namespace, local name) pair, used to describe the scope-check
/// algorithms' target element-type lists (§13.2.4.2) as plain data,
/// without needing a live node to compare against.
#[derive(Debug, Clone, Copy)]
pub(super) struct ElementType {
    namespace: &'static str,
    name: &'static str,
}

pub(super) fn element_type_matches(list: &[ElementType], namespace: &str, name: &str) -> bool {
    list.iter()
        .any(|element_type| element_type.namespace == namespace && element_type.name == name)
}

/// The element-type list for "has a particular element in scope"
/// (§13.2.4.2, the default/unqualified scope) — also the common base list
/// every other named scope (list item, button) extends.
pub(super) const DEFAULT_SCOPE: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "applet",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "caption",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "html",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "table",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "td",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "th",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "marquee",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "object",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "select",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "template",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mi",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mo",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mn",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "ms",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mtext",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "annotation-xml",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "foreignObject",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "desc",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "title",
    },
];

/// Extra element types "has a particular element in list item scope"
/// adds on top of [`DEFAULT_SCOPE`].
pub(super) const LIST_ITEM_SCOPE_EXTRA: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "ol",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "ul",
    },
];

/// Extra element types "has a particular element in button scope" adds on
/// top of [`DEFAULT_SCOPE`].
pub(super) const BUTTON_SCOPE_EXTRA: &[ElementType] = &[ElementType {
    namespace: HTML_NAMESPACE,
    name: "button",
}];

/// The element-type list for "has a particular element in table scope" —
/// deliberately *not* an extension of [`DEFAULT_SCOPE`], per spec (a much
/// shorter, unrelated list).
pub(super) const TABLE_SCOPE: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "html",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "table",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "template",
    },
];

/// The "special" category (§13.2.4.2) — elements with varying levels of
/// special parsing rules. Used by the adoption agency algorithm
/// (§13.2.6.4.7) to find the "furthest block" between a misnested
/// formatting element and the current node. Transcribed directly from
/// the spec's element list (83 HTML elements, 6 MathML, 3 SVG) —
/// extracted from the raw spec markup, not assumed. Note `title`
/// legitimately appears twice: once for the HTML element, once more for
/// the (distinctly namespaced) SVG element of the same name.
pub(super) const SPECIAL_CATEGORY: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "address",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "applet",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "area",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "article",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "aside",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "base",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "basefont",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "bgsound",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "blockquote",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "body",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "br",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "button",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "caption",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "center",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "col",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "colgroup",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dd",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "details",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dir",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "div",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dl",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dt",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "embed",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "fieldset",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "figcaption",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "figure",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "footer",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "form",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "frame",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "frameset",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h1",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h2",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h3",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h4",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h5",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "h6",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "head",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "header",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "hgroup",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "hr",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "html",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "iframe",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "img",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "input",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "keygen",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "li",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "link",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "listing",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "main",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "marquee",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "menu",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "meta",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "nav",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "noembed",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "noframes",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "noscript",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "object",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "ol",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "p",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "param",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "plaintext",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "pre",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "script",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "search",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "section",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "select",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "source",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "style",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "summary",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "table",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tbody",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "td",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "template",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "textarea",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tfoot",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "th",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "thead",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "title",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tr",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "track",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "ul",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "wbr",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "xmp",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mi",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mo",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mn",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "ms",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "mtext",
    },
    ElementType {
        namespace: MATHML_NAMESPACE,
        name: "annotation-xml",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "foreignObject",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "desc",
    },
    ElementType {
        namespace: SVG_NAMESPACE,
        name: "title",
    },
];

pub(super) fn is_special(document: &Document, node: NodeId) -> bool {
    let NodeKind::Element {
        name, namespace, ..
    } = &document.node(node).kind
    else {
        return false;
    };
    element_type_matches(SPECIAL_CATEGORY, namespace.as_deref().unwrap_or(""), name)
}

/// The element-type list "generate implied end tags" pops through
/// (§13.2.6.3).
pub(super) const IMPLIED_END_TAGS: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dd",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dt",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "li",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "optgroup",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "option",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "p",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rb",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rp",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rt",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rtc",
    },
];

/// The element-type list "generate all implied end tags thoroughly" pops
/// through (§13.2.6.3) — a superset of [`IMPLIED_END_TAGS`].
pub(super) const IMPLIED_END_TAGS_THOROUGHLY: &[ElementType] = &[
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "caption",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "colgroup",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dd",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "dt",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "li",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "optgroup",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "option",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "p",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rb",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rp",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rt",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "rtc",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tbody",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "td",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tfoot",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "th",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "thead",
    },
    ElementType {
        namespace: HTML_NAMESPACE,
        name: "tr",
    },
];

/// The stack of open elements (§13.2.4.2). Grows downwards in spec terms —
/// modeled here as a `Vec` where the topmost node (the `html` element) is
/// index 0 and the current node (bottommost) is the last entry, so
/// "push"/"pop" are plain `Vec` operations.
///
/// Note there is no "select scope" here: despite `<select>`/`<option>`
/// needing special parsing behavior, the spec has no dedicated "in
/// select" insertion mode or "select scope" algorithm (verified against
/// the raw spec text, not assumed) — that handling lives inline in the
/// "in body" insertion mode instead.
#[derive(Debug, Default)]
pub(crate) struct OpenElementsStack {
    pub(super) entries: Vec<NodeId>,
}

impl OpenElementsStack {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, node: NodeId) {
        self.entries.push(node);
    }

    pub(crate) fn pop(&mut self) -> Option<NodeId> {
        self.entries.pop()
    }

    /// The current node (§13.2.4.2): the bottommost node on the stack.
    pub(crate) fn current_node(&self) -> Option<NodeId> {
        self.entries.last().copied()
    }

    /// True if `node` is currently on the stack — used by "reconstruct
    /// the active formatting elements" (§13.2.4.3) to check whether a
    /// formatting entry's element is still open.
    pub(crate) fn contains(&self, node: NodeId) -> bool {
        self.entries.contains(&node)
    }

    /// §13.2.4.2's generic "have an element *target node* in a specific
    /// scope consisting of a list of element types *list*" algorithm.
    /// `target_namespace`/`target_name` identify the element **by tag
    /// name**, not by a specific node instance — matching how every named
    /// scope check is actually invoked in the spec (e.g. "the stack of
    /// open elements has a `p` element in button scope"). `extra_boundary`
    /// lets [`has_element_in_list_item_scope`](Self::has_element_in_list_item_scope)/
    /// [`has_element_in_button_scope`](Self::has_element_in_button_scope) extend
    /// [`DEFAULT_SCOPE`] without duplicating it; pass `&[]` for scopes
    /// that don't extend it.
    pub(super) fn has_element_in_specific_scope(
        &self,
        document: &Document,
        target_namespace: &str,
        target_name: &str,
        boundary: &[ElementType],
        extra_boundary: &[ElementType],
    ) -> bool {
        for &node in self.entries.iter().rev() {
            let NodeKind::Element {
                name, namespace, ..
            } = &document.node(node).kind
            else {
                continue;
            };
            let namespace = namespace.as_deref().unwrap_or("");
            if namespace == target_namespace && name == target_name {
                return true;
            }
            if element_type_matches(boundary, namespace, name)
                || element_type_matches(extra_boundary, namespace, name)
            {
                return false;
            }
        }
        // Unreachable for a well-formed stack: the `html` element is
        // always index 0, and `html` is itself always in every scope's
        // boundary list, so the loop always terminates via one of the
        // branches above before running out of entries.
        false
    }

    /// "Have a particular element in scope" (default scope, §13.2.4.2).
    pub(crate) fn has_element_in_scope(&self, document: &Document, name: &str) -> bool {
        self.has_element_in_specific_scope(document, HTML_NAMESPACE, name, DEFAULT_SCOPE, &[])
    }

    /// "Have a particular element in list item scope" (§13.2.4.2).
    pub(crate) fn has_element_in_list_item_scope(&self, document: &Document, name: &str) -> bool {
        self.has_element_in_specific_scope(
            document,
            HTML_NAMESPACE,
            name,
            DEFAULT_SCOPE,
            LIST_ITEM_SCOPE_EXTRA,
        )
    }

    /// "Have a particular element in button scope" (§13.2.4.2).
    pub(crate) fn has_element_in_button_scope(&self, document: &Document, name: &str) -> bool {
        self.has_element_in_specific_scope(
            document,
            HTML_NAMESPACE,
            name,
            DEFAULT_SCOPE,
            BUTTON_SCOPE_EXTRA,
        )
    }

    /// "Have a particular element in table scope" (§13.2.4.2).
    pub(crate) fn has_element_in_table_scope(&self, document: &Document, name: &str) -> bool {
        self.has_element_in_specific_scope(document, HTML_NAMESPACE, name, TABLE_SCOPE, &[])
    }

    /// The topmost node on the stack — the `html` element, once one has
    /// been pushed. Needed by foster parenting's "no last table" fallback
    /// (§13.2.6.1).
    pub(super) fn topmost(&self) -> Option<NodeId> {
        self.entries.first().copied()
    }

    /// The last (most recently pushed, i.e. bottommost) element on the
    /// stack matching `namespace`/`name`, if any — "the last `table`
    /// element in the stack of open elements" and similar phrasing used
    /// throughout §13.2.6.1.
    pub(super) fn last_matching(
        &self,
        document: &Document,
        namespace: &str,
        name: &str,
    ) -> Option<NodeId> {
        self.entries.iter().rev().copied().find(|&node| {
            let NodeKind::Element {
                name: node_name,
                namespace: node_namespace,
                ..
            } = &document.node(node).kind
            else {
                return false;
            };
            node_namespace.as_deref() == Some(namespace) && node_name == name
        })
    }

    /// True if `a` is lower (more recently added, i.e. further from the
    /// top) than `b` in the stack. Both must currently be on the stack.
    pub(super) fn is_lower(&self, a: NodeId, b: NodeId) -> bool {
        let position_of = |target: NodeId| {
            self.entries
                .iter()
                .position(|&node| node == target)
                .expect("is_lower called with a node that isn't on the stack")
        };
        position_of(a) > position_of(b)
    }

    /// The element immediately above `node` on the stack (i.e. pushed
    /// immediately before it) — §13.2.6.1's foster-parenting fallback for
    /// a table with no parent (unreachable in this single-pass parser,
    /// since nothing ever removes a node from its parent once inserted,
    /// but implemented for spec fidelity regardless).
    pub(super) fn element_immediately_above(&self, node: NodeId) -> Option<NodeId> {
        let position = self.entries.iter().position(|&entry| entry == node)?;
        position.checked_sub(1).map(|above| self.entries[above])
    }

    /// The topmost node lower in the stack than `formatting_element` that
    /// is an element in the [`SPECIAL_CATEGORY`] (§13.2.4.2) — the
    /// adoption agency algorithm's "furthest block" (§13.2.6.4.7, step
    /// 7). `None` if there is no such node.
    pub(super) fn topmost_special_element_below(
        &self,
        document: &Document,
        formatting_element: NodeId,
    ) -> Option<NodeId> {
        let position = self
            .entries
            .iter()
            .position(|&entry| entry == formatting_element)?;
        self.entries[position + 1..]
            .iter()
            .copied()
            .find(|&node| is_special(document, node))
    }

    /// "Generate implied end tags" (§13.2.6.3): pop elements off the stack
    /// while the current node is a `dd`, `dt`, `li`, `optgroup`, `option`,
    /// `p`, `rb`, `rp`, `rt`, or `rtc` element. If `exclude` is given, that
    /// element name is treated as if it were not in the list (so popping
    /// stops there instead of past it), per the spec's "lists an element
    /// to exclude from the process" clause.
    pub(crate) fn generate_implied_end_tags(&mut self, document: &Document, exclude: Option<&str>) {
        self.generate_implied_end_tags_impl(document, IMPLIED_END_TAGS, exclude);
    }

    /// "Generate all implied end tags thoroughly" (§13.2.6.3) — the same
    /// idea over the larger [`IMPLIED_END_TAGS_THOROUGHLY`] list. The spec
    /// never invokes this variant with an exclusion, so there's no
    /// `exclude` parameter here.
    pub(crate) fn generate_implied_end_tags_thoroughly(&mut self, document: &Document) {
        self.generate_implied_end_tags_impl(document, IMPLIED_END_TAGS_THOROUGHLY, None);
    }

    pub(super) fn generate_implied_end_tags_impl(
        &mut self,
        document: &Document,
        list: &[ElementType],
        exclude: Option<&str>,
    ) {
        while let Some(node) = self.current_node() {
            let NodeKind::Element {
                name, namespace, ..
            } = &document.node(node).kind
            else {
                break;
            };
            let namespace = namespace.as_deref().unwrap_or("");
            if Some(name.as_str()) == exclude {
                break;
            }
            if !element_type_matches(list, namespace, name) {
                break;
            }
            self.pop();
        }
    }
}
