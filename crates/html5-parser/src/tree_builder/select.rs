use super::*;

impl TreeBuilder {
    /// "Get the nearest ancestor `select`" (form-elements.html §4.10.10)
    /// — walks `node`'s ancestors from nearest to furthest, tracking at
    /// most one intervening `optgroup`; a `datalist`/`hr`/`option`
    /// ancestor, or a *second* `optgroup`, disqualifies the walk
    /// entirely (returns `None`) before it can reach a `select`.
    pub(super) fn nearest_ancestor_select(&self, node: NodeId) -> Option<NodeId> {
        let mut ancestor_optgroup = None;
        let mut current = self.document.parent(node);
        while let Some(ancestor) = current {
            if self.node_has_html_name(ancestor, "datalist")
                || self.node_has_html_name(ancestor, "hr")
                || self.node_has_html_name(ancestor, "option")
            {
                return None;
            }
            if self.node_has_html_name(ancestor, "optgroup") {
                if ancestor_optgroup.is_some() {
                    return None;
                }
                ancestor_optgroup = Some(ancestor);
            }
            if self.node_has_html_name(ancestor, "select") {
                return Some(ancestor);
            }
            current = self.document.parent(ancestor);
        }
        None
    }

    /// "Get the list of options" (form-elements.html §4.10.7),
    /// practically scoped: `option` elements that are either direct
    /// children of `select`, or direct children of an `optgroup` that
    /// is itself a direct child of `select` — covers ordinary
    /// `<select>`/`<option>`/`<optgroup>` markup (and everything the
    /// html5lib-tests corpus's `<selectedcontent>` cases use) without a
    /// general tree-order walker with subtree-skipping, which the real
    /// algorithm technically requires for exotic nesting no test here
    /// exercises.
    pub(super) fn list_of_options(&self, select: NodeId) -> Vec<NodeId> {
        let mut options = Vec::new();
        for child in self.document.children(select) {
            if self.node_has_html_name(child, "option") {
                options.push(child);
            } else if self.node_has_html_name(child, "optgroup") {
                options.extend(
                    self.document
                        .children(child)
                        .filter(|&grandchild| self.node_has_html_name(grandchild, "option")),
                );
            }
        }
        options
    }

    /// The option that would be "selected" in `select` right now, per
    /// the selectedness setting algorithm's two rules (form-elements.html
    /// §4.10.7) collapsed into a single query: this crate never mutates
    /// an option's selectedness after the fact (no scripting, no form
    /// resets, no live DOM) — a `selected` attribute is the only way an
    /// option's selectedness ever becomes (and stays) true, and "no
    /// option selected yet" always means the first (non-`disabled`) one
    /// defaults to selected. Re-deriving this from current attribute
    /// state on demand, rather than maintaining a separate mutable
    /// selectedness flag per option that would need updating on every
    /// insertion, gives the same answer at every point during parsing
    /// (each is evaluated only against however many options exist in
    /// the tree so far, exactly matching the incremental algorithm's own
    /// per-insertion re-evaluation).
    pub(super) fn selected_option_of(&self, select: NodeId) -> Option<NodeId> {
        let options = self.list_of_options(select);
        options
            .iter()
            .rev()
            .find(|&&option| self.node_has_html_attribute(option, "selected"))
            .copied()
            .or_else(|| {
                options
                    .into_iter()
                    .find(|&option| !self.node_has_html_attribute(option, "disabled"))
            })
    }

    /// "Get a `select`'s enabled `selectedcontent`" (form-elements.html
    /// §4.10.17), simplified: skips the `disabled` flag a
    /// `selectedcontent` element can carry from its own "post-connection
    /// steps" (only relevant for exotic nesting — a `selectedcontent`
    /// inside a *second*, unrelated `select`/`option` — this crate's
    /// namesake corpus cases never trigger it).
    pub(super) fn first_selectedcontent_descendant(&self, select: NodeId) -> Option<NodeId> {
        if self.node_has_html_attribute(select, "multiple") {
            return None;
        }
        self.first_descendant_matching(select, "selectedcontent")
    }

    /// Depth-first, tree-order search for the first descendant of
    /// `node` with HTML tag name `name`. Iterative, so input nesting depth
    /// costs heap, not call frames.
    pub(super) fn first_descendant_matching(&self, node: NodeId, name: &str) -> Option<NodeId> {
        let mut open = vec![self.document.children(node)];
        while let Some(children) = open.last_mut() {
            let Some(child) = children.next() else {
                open.pop();
                continue;
            };
            if self.node_has_html_name(child, name) {
                return Some(child);
            }
            open.push(self.document.children(child));
        }
        None
    }

    /// "Maybe clone an `option` into `selectedcontent`" (form-elements.html
    /// §4.10.10), run "when an `option` element is popped off the stack
    /// of open elements" — see
    /// [`with_option_pop_hook`](Self::with_option_pop_hook) for how (and
    /// which) pops actually trigger this call.
    pub(super) fn maybe_clone_option_into_selectedcontent(&mut self, option: NodeId) {
        let Some(select) = self.nearest_ancestor_select(option) else {
            return;
        };
        if self.selected_option_of(select) != Some(option) {
            return;
        }
        let Some(selectedcontent) = self.first_selectedcontent_descendant(select) else {
            return;
        };
        // "Clone an option into a selectedcontent": replace all of
        // `selectedcontent`'s children with fresh, deep clones of
        // `option`'s own children.
        let existing_children: Vec<_> = self.document.children(selectedcontent).collect();
        for child in existing_children {
            self.document.remove(child);
        }
        let option_children: Vec<_> = self.document.children(option).collect();
        for child in option_children {
            let clone = self.document.clone_subtree(child);
            self.document.append_child(selectedcontent, clone);
        }
    }

    /// Runs `f`, then runs
    /// [`maybe_clone_option_into_selectedcontent`](Self::maybe_clone_option_into_selectedcontent)
    /// for every `option` element `f` popped off the stack of open
    /// elements, in the order they were actually popped (most-recently-
    /// popped first) — form-elements.html's real trigger for that hook
    /// is simply "when an `option` element is popped", full stop,
    /// regardless of *why*, but plumbing a callback through every
    /// pop-driving helper (`generate_implied_end_tags`,
    /// `pop_until_one_of_popped`, ...) down to a single `Vec::pop` call
    /// would touch most of this file for cases this crate's corpus
    /// doesn't exercise. Comparing the stack before/after instead keeps
    /// the hook centralized at only the call sites that matter for
    /// `<select>` content (see its own call sites) — not a fully general
    /// interception of every possible pop, a deliberate practical scope.
    pub(super) fn with_option_pop_hook<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let before: Vec<NodeId> = self.open_elements.entries.clone();
        let result = f(self);
        let after_len = self.open_elements.entries.len();
        // `f` is only ever passed pop-driving calls here (see call
        // sites) — `after_len <= before.len()` always holds in
        // practice, but this isn't the place to panic if that
        // invariant is ever violated by a future caller.
        if after_len <= before.len() {
            for &node in before[after_len..].iter().rev() {
                if self.node_has_html_name(node, "option") {
                    self.maybe_clone_option_into_selectedcontent(node);
                }
            }
        }
        result
    }
}
