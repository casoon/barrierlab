//! Eine schlichte Arena, die [`Document`] und [`Node`] erfüllt.
//!
//! Zwei Zwecke: Sie beweist, dass die Traits implementierbar sind, und sie ist
//! die Form, die ein WASM-Host tatsächlich braucht — dort kann nicht pro
//! Trait-Methode nach JavaScript zurückgerufen werden, also wird der Baum
//! einmal in den Linearspeicher materialisiert.
//!
//! Für Hosts mit eigener Baumdarstellung (`scraper::Html`, ein AXTree) ist sie
//! nicht gedacht — die implementieren die Traits direkt über ihre eigenen Typen.

use crate::tree::{Document, Node, NodeId, NodeKind};

#[derive(Debug)]
struct Entry {
    kind: NodeKind,
    name: String,
    text: String,
    parent: Option<u32>,
    children: Vec<u32>,
    attrs: Vec<(String, String)>,
}

/// Ein Dokument als flache Knotenliste.
#[derive(Debug, Default)]
pub struct Arena {
    nodes: Vec<Entry>,
}

impl Arena {
    pub fn builder() -> ArenaBuilder {
        ArenaBuilder {
            arena: Arena::default(),
            open: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Knoten über seine Kennung, etwa um einen Befund zurück aufzulösen.
    pub fn get(&self, id: NodeId) -> Option<ArenaNode<'_>> {
        ((id.0 as usize) < self.nodes.len()).then_some(ArenaNode {
            arena: self,
            idx: id.0,
        })
    }
}

/// Ein Handle auf einen Arena-Knoten. `Copy`, wie das Trait es verlangt.
#[derive(Debug, Clone, Copy)]
pub struct ArenaNode<'a> {
    arena: &'a Arena,
    idx: u32,
}

impl PartialEq for ArenaNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.arena, other.arena) && self.idx == other.idx
    }
}

impl Eq for ArenaNode<'_> {}

impl<'a> ArenaNode<'a> {
    fn entry(&self) -> &'a Entry {
        &self.arena.nodes[self.idx as usize]
    }
}

impl<'a> Node<'a> for ArenaNode<'a> {
    fn id(self) -> NodeId {
        NodeId(self.idx)
    }

    fn kind(self) -> NodeKind {
        self.entry().kind
    }

    fn parent(self) -> Option<Self> {
        self.entry().parent.map(|p| ArenaNode {
            arena: self.arena,
            idx: p,
        })
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let arena = self.arena;
        self.entry()
            .children
            .iter()
            .map(move |&idx| ArenaNode { arena, idx })
    }

    fn local_name(self) -> &'a str {
        &self.entry().name
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.entry()
            .attrs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }

    fn text(self) -> &'a str {
        &self.entry().text
    }
}

impl Document for Arena {
    type N<'a>
        = ArenaNode<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        ArenaNode {
            arena: self,
            idx: 0,
        }
    }

    fn node_count(&self) -> Option<usize> {
        Some(self.nodes.len())
    }
}

/// Baut eine [`Arena`] auf. Elemente werden geöffnet und geschlossen, Text wird
/// in das gerade offene Element gelegt.
///
/// ```
/// use a11y_dom::{Arena, Document, Node};
///
/// let doc = Arena::builder()
///     .open("html").attr("lang", "de")
///         .open("body")
///             .open("h1").text("Überschrift").close()
///             .open("img").attr("src", "a.png").close()
///         .close()
///     .close()
///     .build();
///
/// assert_eq!(doc.root().local_name(), "html");
/// assert_eq!(doc.root().attr("lang"), Some("de"));
/// ```
pub struct ArenaBuilder {
    arena: Arena,
    open: Vec<u32>,
}

impl ArenaBuilder {
    fn push(&mut self, kind: NodeKind, name: &str, text: &str) -> u32 {
        let idx = self.arena.nodes.len() as u32;
        let parent = self.open.last().copied();
        self.arena.nodes.push(Entry {
            kind,
            name: name.to_string(),
            text: text.to_string(),
            parent,
            children: Vec::new(),
            attrs: Vec::new(),
        });
        if let Some(p) = parent {
            self.arena.nodes[p as usize].children.push(idx);
        }
        idx
    }

    /// Öffnet ein Element. Folgende `attr`, `text` und `open` landen darin, bis
    /// [`ArenaBuilder::close`] es schließt.
    pub fn open(mut self, local_name: &str) -> Self {
        let idx = self.push(NodeKind::Element, local_name, "");
        self.open.push(idx);
        self
    }

    /// Setzt ein Attribut am gerade offenen Element.
    pub fn attr(mut self, name: &str, value: &str) -> Self {
        if let Some(&idx) = self.open.last() {
            self.arena.nodes[idx as usize]
                .attrs
                .push((name.to_string(), value.to_string()));
        }
        self
    }

    /// Fügt einen Textknoten in das gerade offene Element ein.
    pub fn text(mut self, text: &str) -> Self {
        self.push(NodeKind::Text, "#text", text);
        self
    }

    /// Schließt das gerade offene Element.
    pub fn close(mut self) -> Self {
        self.open.pop();
        self
    }

    /// Kurzform für ein Element ohne Kinder.
    pub fn leaf(self, local_name: &str) -> Self {
        self.open(local_name).close()
    }

    pub fn build(self) -> Arena {
        self.arena
    }
}
