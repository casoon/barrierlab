//! Per-document memo for evaluations that only depend on the document
//! (barrierlab#69): the nodes on the `descendant` axis of the root that
//! pass a node test, and, over such a list, which nodes carry an attribute
//! with a given value.
//!
//! Both are pure functions of the document, so caching them never changes
//! a result. A cache binds itself to the first root it sees; evaluating
//! against another document simply bypasses it.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::axes;
use crate::document::{Node, NodeKind};

/// A node test with its prefix resolved, in a form usable as a map key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TestKey {
    Node,
    Kind(NodeKind),
    ProcessingInstruction(String),
    NamespaceWildcard(NodeKind, String),
    Name(NodeKind, Option<String>, String),
}

/// Over one cached node list: the positions (ascending) of the nodes with
/// an attribute of one name, overall and by the attribute's value.
#[derive(Default)]
pub(crate) struct AttributeIndex {
    pub(crate) present: Vec<u32>,
    pub(crate) by_value: HashMap<String, Vec<u32>>,
}

/// Memo for one document, shared by every evaluation against it. Hand it
/// to [`crate::EvaluationContext::cache`]; drop it when the document goes
/// away.
pub struct DocumentCache<N> {
    root: OnceCell<N>,
    all: OnceCell<Rc<[N]>>,
    lists: RefCell<HashMap<TestKey, Rc<[N]>>>,
    indexes: RefCell<HashMap<(TestKey, TestKey), Rc<AttributeIndex>>>,
}

impl<N> Default for DocumentCache<N> {
    fn default() -> Self {
        DocumentCache {
            root: OnceCell::new(),
            all: OnceCell::new(),
            lists: RefCell::new(HashMap::new()),
            indexes: RefCell::new(HashMap::new()),
        }
    }
}

impl<N> DocumentCache<N> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a, N: Node<'a>> DocumentCache<N> {
    /// Whether this cache belongs to the document whose root is `root`.
    /// Only a root node identifies a document; anything else is not served.
    fn serves(&self, root: N) -> bool {
        root.kind() == NodeKind::Root && *self.root.get_or_init(|| root) == root
    }

    /// The descendants of `root` that pass `matches`, in document order.
    pub(crate) fn descendants(
        &self,
        root: N,
        key: &TestKey,
        matches: impl Fn(N) -> bool,
    ) -> Option<Rc<[N]>> {
        if !self.serves(root) {
            return None;
        }
        if let Some(list) = self.lists.borrow().get(key) {
            return Some(Rc::clone(list));
        }
        let all = self.all.get_or_init(|| axes::descendant(root).collect());
        let list: Rc<[N]> = if *key == TestKey::Node {
            Rc::clone(all)
        } else {
            all.iter().copied().filter(|&n| matches(n)).collect()
        };
        self.lists
            .borrow_mut()
            .insert(key.clone(), Rc::clone(&list));
        Some(list)
    }

    /// For the list cached under `list_key`, the positions of the nodes
    /// grouped by the value of their attributes that pass `attribute`.
    pub(crate) fn attribute_index(
        &self,
        list_key: &TestKey,
        list: &[N],
        attribute_key: &TestKey,
        attribute: impl Fn(N) -> bool,
    ) -> Rc<AttributeIndex> {
        let key = (list_key.clone(), attribute_key.clone());
        if let Some(index) = self.indexes.borrow().get(&key) {
            return Rc::clone(index);
        }
        let mut index = AttributeIndex::default();
        for (position, &n) in list.iter().enumerate() {
            let position = u32::try_from(position).expect("node list exceeds u32::MAX");
            for a in axes::attribute(n).filter(|&a| attribute(a)) {
                if index.present.last() != Some(&position) {
                    index.present.push(position);
                }
                a.with_string_value(|value| {
                    if let Some(positions) = index.by_value.get_mut(value) {
                        if positions.last() != Some(&position) {
                            positions.push(position);
                        }
                    } else {
                        index.by_value.insert(value.to_owned(), vec![position]);
                    }
                });
            }
        }
        let index = Rc::new(index);
        self.indexes.borrow_mut().insert(key, Rc::clone(&index));
        index
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentCache;
    use crate::document::fixture::{TestNode, build, build_roles};
    use crate::document::{Document, Node};
    use crate::{EvaluationContext, QName, Value, evaluate, evaluate_boolean, parse};

    /// Every node of `doc` reachable through `children()` and `attributes()`.
    fn all_nodes(root: TestNode<'_>) -> Vec<TestNode<'_>> {
        let mut out = vec![root];
        let mut i = 0;
        while i < out.len() {
            let n = out[i];
            out.extend(n.attributes());
            out.extend(n.children());
            i += 1;
        }
        out
    }

    /// Evaluates `src` from every node, with and without a cache, as a
    /// value and as a boolean, and requires all four to agree.
    fn assert_cache_transparent(src: &str) {
        let doc = build_roles();
        let expr = parse(src).expect("test expression must parse");
        let lookup = |name: &QName| match name.local.as_str() {
            "v" => Some(Value::String("a".to_owned())),
            _ => None,
        };
        let cache = DocumentCache::new();
        for node in all_nodes(doc.root()) {
            let mut plain = EvaluationContext::new(node);
            plain.variables = Some(&lookup);
            let mut cached = plain;
            cached.cache = Some(&cache);

            let expected = evaluate(&expr, &plain);
            assert_eq!(evaluate(&expr, &cached), expected, "{src}");
            let truth = expected.map(|v| v.to_boolean());
            assert_eq!(evaluate_boolean(&expr, &plain), truth, "{src}");
            assert_eq!(evaluate_boolean(&expr, &cached), truth, "{src}");
        }
    }

    #[test]
    fn cached_evaluation_selects_the_same_nodes() {
        for src in [
            "//*",
            "//p",
            "//*[@id = 'a']",
            "count(//*[@id = 'a'])",
            "//*[@id = $v]",
            "//*[@id = 'a'][2]",
            "//*[@id = //p/@id]",
            "//*[@role = 'tab' or @role = 'menu']",
            "//*[@role and @id]",
            "//*[(@role = 'tab')]",
            "//*[@role = 'tab' and ancestor::*[@role = 'menu']]",
            "//*[@role = 'tab' and position() = 1]",
            "//*[@role][1]",
            "//*[@role][last()]",
            "//*[@id = 1]",
            "//*[@id = @role]",
            "//*[@nope = 'x']",
            "//*[not(@role)]",
            "//*[ancestor::*[@role = 'menu' or @role = 'list']]",
            "ancestor::*[@role = 'menu']",
            "ancestor-or-self::*[@role = 'tab' or @id = 'b']",
            "boolean(//a)",
            "//li[1]",
            "//p[1]",
            "/descendant::p[1]",
        ] {
            assert_cache_transparent(src);
        }
    }

    #[test]
    fn index_errors_surface_like_plain_evaluation() {
        // `$missing` is unbound: the index cannot answer, and the ordinary
        // path raises the same error it always did.
        assert_cache_transparent("//*[@id = $missing]");
        assert_cache_transparent("//*[@role = 'tab' and $missing]");
    }

    #[test]
    fn positional_double_slash_is_not_rewritten() {
        // §2.5: `//p[1]` is the first `p` child of each parent, not the
        // first `p` in the document.
        let doc = build_roles();
        let ctx = EvaluationContext::new(doc.root());
        let count = |src: &str| match evaluate(&parse(src).unwrap(), &ctx) {
            Ok(Value::NodeSet(nodes)) => nodes.len(),
            other => panic!("expected a node-set, got {other:?}"),
        };
        assert_eq!(count("//p[1]"), 2);
        assert_eq!(count("/descendant::p[1]"), 1);
    }

    #[test]
    fn a_cache_serves_only_the_first_document() {
        let first = build_roles();
        let second = build();
        let cache = DocumentCache::new();
        let expr = parse("count(//*[@id = 'a'])").unwrap();
        for (doc_root, expected) in [(first.root(), 2.0), (second.doc.root(), 0.0)] {
            let mut ctx = EvaluationContext::new(doc_root);
            ctx.cache = Some(&cache);
            assert_eq!(evaluate(&expr, &ctx), Ok(Value::Number(expected)));
        }
    }
}
