//! Die Arena: der Baum der geprüften Seite im WASM-Linearspeicher.
//!
//! Der Collector überträgt den Baum einmal je Scan spaltenweise — Typed Arrays
//! plus ein einziger UTF-8-Blob für alle Texte und Attributwerte. Darüber
//! implementiert dieses Modul [`a11y_dom::Document`] und [`a11y_dom::Node`],
//! sodass der Regelbestand aus `a11y-rules` unverändert darüber läuft.
//!
//! Warum spaltenweise und nicht als Rückrufmodell: WASM kann nicht pro
//! Trait-Methode nach JavaScript zurückrufen — jedes `children()` wäre ein
//! FFI-Übergang. Gemessen kostet der Aufbau 0,5 ms bei 31.000 Knoten
//! (siehe `spike/ERGEBNIS.md`).

use std::cmp::Ordering;

use a11y_dom::{Document, Node, NodeId, NodeKind, TreeScope};

/// Der Tagname, unter dem der Collector Textknoten interniert. Elemente können
/// so nicht heißen — `localName` ist nie mit `#` präfigiert.
pub const TEXT_TAG: &str = "#text";

/// Ein Knoten. Attribute liegen als Bereich `[attr_start, attr_end)` in den
/// Attributspalten der Arena, nicht pro Knoten als eigene Allokation.
struct Entry {
    tag: u32,
    parent: i32,
    text_off: u32,
    text_len: u32,
    attr_start: u32,
    attr_end: u32,
}

/// Der Baum eines Scans.
pub struct Arena {
    entries: Vec<Entry>,
    /// Kindindizes, pro Knoten zusammenhängend über `child_start` adressiert.
    children: Vec<u32>,
    child_start: Vec<u32>,
    attr_name: Vec<u32>,
    attr_val_off: Vec<u32>,
    attr_val_len: Vec<u32>,
    /// Ein einziger UTF-8-Blob für alle Texte und Attributwerte.
    blob: Vec<u8>,
    tag_names: Vec<String>,
    attr_names: Vec<String>,
    /// Die internierte Kennung von [`TEXT_TAG`], sofern das Dokument Text enthält.
    text_tag: Option<u32>,
    /// Der Baumbereich je Knoten (`TreeScope`, 0 = Dokument). Leer heißt: alle
    /// Knoten im Dokument — der Fall ohne Shadow DOM.
    scopes: Vec<u32>,
}

impl Arena {
    /// Baut die Arena aus den Spalten des Collectors.
    ///
    /// Prüft vorher die Invarianten der Spalten (`validate_columns`); ein
    /// Collector mit einem Fehler um eins bekommt eine Meldung statt eines
    /// Traps im WASM-Modul.
    #[allow(clippy::too_many_arguments)]
    pub fn from_columns(
        tag: &[u32],
        parent: &[i32],
        text_off: &[u32],
        text_len: &[u32],
        attr_start: &[u32],
        attr_name: Vec<u32>,
        attr_val_off: Vec<u32>,
        attr_val_len: Vec<u32>,
        blob: Vec<u8>,
        tag_names: Vec<String>,
        attr_names: Vec<String>,
    ) -> Result<Self, String> {
        validate_columns(
            tag,
            parent,
            text_off,
            text_len,
            attr_start,
            &attr_name,
            &attr_val_off,
            &attr_val_len,
            blob.len(),
            tag_names.len(),
            attr_names.len(),
        )?;
        let n = tag.len();
        let mut entries = Vec::with_capacity(n);
        for i in 0..n {
            entries.push(Entry {
                tag: tag[i],
                parent: parent[i],
                text_off: text_off[i],
                text_len: text_len[i],
                attr_start: attr_start[i],
                attr_end: attr_start[i + 1],
            });
        }
        let (children, child_start) = index_children(&entries);
        let text_tag = tag_names
            .iter()
            .position(|t| t == TEXT_TAG)
            .map(|i| i as u32);

        Ok(Arena {
            entries,
            children,
            child_start,
            attr_name,
            attr_val_off,
            attr_val_len,
            blob,
            tag_names,
            attr_names,
            text_tag,
            scopes: Vec::new(),
        })
    }

    /// Setzt den Baumbereich je Knoten, parallel zu den Knotenspalten.
    ///
    /// `0` ist das Dokument, jede andere Zahl eine Shadow Root; welche, bestimmt
    /// der Collector, sie muss nur je Dokument eindeutig sein. Ein per `<slot>`
    /// eingesetzter Knoten trägt den Bereich des **äußeren** Baums, nicht den
    /// der Shadow Root, unter der er im flachen Baum hängt (siehe
    /// [`a11y_dom::Node::tree_scope`]). IDs und ihre Verweise gelten je Bereich.
    ///
    /// Prüft: so viele Einträge wie Knoten, Wurzel im Dokument, jeder Bereich
    /// kleiner als die Knotenzahl (mehr Shadow Roots als Knoten gibt es nicht).
    pub fn set_scopes(&mut self, scopes: Vec<u32>) -> Result<(), String> {
        let n = self.entries.len();
        if scopes.len() != n {
            return Err(format!("scopes has {} entries, tag has {n}", scopes.len()));
        }
        if scopes[0] != 0 {
            return Err(format!(
                "scopes[0] is {}, but the root belongs to the document (scope 0)",
                scopes[0]
            ));
        }
        if let Some(i) = scopes.iter().position(|s| *s as usize >= n) {
            return Err(format!(
                "scopes[{i}] = {} is not below the node count ({n})",
                scopes[i]
            ));
        }
        self.scopes = scopes;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn str_at(&self, off: u32, len: u32) -> &str {
        let (a, b) = (off as usize, (off + len) as usize);
        std::str::from_utf8(&self.blob[a..b]).unwrap_or("")
    }
}

/// Prüft die elf Spalten des Collectors, bevor die Arena sie indiziert.
///
/// Geprüft wird: gleiche Länge der Knotenspalten, `attr_start` mit einem Eintrag
/// mehr als Knoten, monoton und im Bereich der Attributspalten, Eltern vor dem
/// Kind, Text- und Attributwerte innerhalb des Blobs, Namensindizes im
/// Wörterbuch. Reine Rust-Funktion, ohne WASM testbar.
#[allow(clippy::too_many_arguments)]
pub fn validate_columns(
    tag: &[u32],
    parent: &[i32],
    text_off: &[u32],
    text_len: &[u32],
    attr_start: &[u32],
    attr_name: &[u32],
    attr_val_off: &[u32],
    attr_val_len: &[u32],
    blob_len: usize,
    tag_count: usize,
    attr_name_count: usize,
) -> Result<(), String> {
    let n = tag.len();
    if n == 0 {
        return Err("tag is empty: the arena needs at least a root node".into());
    }
    for (name, len) in [
        ("parent", parent.len()),
        ("text_off", text_off.len()),
        ("text_len", text_len.len()),
    ] {
        if len != n {
            return Err(format!("{name} has {len} entries, tag has {n}"));
        }
    }
    if attr_start.len() != n + 1 {
        return Err(format!(
            "attr_start has {} entries, expected {} (node count + 1)",
            attr_start.len(),
            n + 1
        ));
    }
    let m = attr_name.len();
    for (name, len) in [
        ("attr_val_off", attr_val_off.len()),
        ("attr_val_len", attr_val_len.len()),
    ] {
        if len != m {
            return Err(format!("{name} has {len} entries, attr_name has {m}"));
        }
    }
    if attr_start[0] != 0 {
        return Err(format!("attr_start[0] is {}, expected 0", attr_start[0]));
    }
    for (i, w) in attr_start.windows(2).enumerate() {
        if w[1] < w[0] {
            return Err(format!(
                "attr_start is not monotonic at {}: {} follows {}",
                i + 1,
                w[1],
                w[0]
            ));
        }
    }
    if attr_start[n] as usize != m {
        return Err(format!(
            "attr_start ends at {}, but there are {m} attributes",
            attr_start[n]
        ));
    }
    for i in 0..n {
        if tag[i] as usize >= tag_count {
            return Err(format!(
                "tag[{i}] = {} is outside the tag dictionary ({tag_count} names)",
                tag[i]
            ));
        }
        if parent[i] < -1 || (parent[i] >= 0 && parent[i] as usize >= i) {
            return Err(format!(
                "parent[{i}] = {} must be -1 or the index of an earlier node",
                parent[i]
            ));
        }
        if u64::from(text_off[i]) + u64::from(text_len[i]) > blob_len as u64 {
            return Err(format!(
                "text of node {i} reaches beyond the blob ({blob_len} bytes)"
            ));
        }
    }
    for a in 0..m {
        if attr_name[a] as usize >= attr_name_count {
            return Err(format!(
                "attr_name[{a}] = {} is outside the attribute dictionary ({attr_name_count} names)",
                attr_name[a]
            ));
        }
        if u64::from(attr_val_off[a]) + u64::from(attr_val_len[a]) > blob_len as u64 {
            return Err(format!(
                "value of attribute {a} reaches beyond the blob ({blob_len} bytes)"
            ));
        }
    }
    Ok(())
}

/// Baut `children`/`child_start` aus der Elternspalte. Zwei Durchläufe, keine
/// Zwischenallokation pro Knoten.
fn index_children(entries: &[Entry]) -> (Vec<u32>, Vec<u32>) {
    let n = entries.len();
    let mut counts = vec![0u32; n];
    for e in entries {
        if e.parent >= 0 {
            counts[e.parent as usize] += 1;
        }
    }
    let mut start = vec![0u32; n + 1];
    let mut acc = 0u32;
    for i in 0..n {
        start[i] = acc;
        acc += counts[i];
    }
    start[n] = acc;

    let mut cursor = start.clone();
    let mut children = vec![0u32; acc as usize];
    for (i, e) in entries.iter().enumerate() {
        if e.parent >= 0 {
            let p = e.parent as usize;
            children[cursor[p] as usize] = i as u32;
            cursor[p] += 1;
        }
    }
    (children, start)
}

/// Ein Handle auf einen Knoten. `Copy`, weil die Traversierungs-Iteratoren aus
/// `a11y-dom` viele kurzlebige Handles erzeugen.
#[derive(Clone, Copy)]
pub struct ArenaNode<'a> {
    arena: &'a Arena,
    idx: u32,
}

impl<'a> ArenaNode<'a> {
    fn entry(self) -> &'a Entry {
        &self.arena.entries[self.idx as usize]
    }

    pub fn index(self) -> u32 {
        self.idx
    }
}

impl PartialEq for ArenaNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.arena, other.arena) && self.idx == other.idx
    }
}

impl Eq for ArenaNode<'_> {}

impl<'a> Node<'a> for ArenaNode<'a> {
    fn id(self) -> NodeId {
        NodeId(self.idx)
    }

    fn kind(self) -> NodeKind {
        if Some(self.entry().tag) == self.arena.text_tag {
            NodeKind::Text
        } else {
            NodeKind::Element
        }
    }

    fn parent(self) -> Option<Self> {
        let p = self.entry().parent;
        (p >= 0).then_some(ArenaNode {
            arena: self.arena,
            idx: p as u32,
        })
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let arena = self.arena;
        let a = arena.child_start[self.idx as usize] as usize;
        let b = arena.child_start[self.idx as usize + 1] as usize;
        arena.children[a..b]
            .iter()
            .map(move |&idx| ArenaNode { arena, idx })
    }

    fn local_name(self) -> &'a str {
        match self.kind() {
            NodeKind::Text => "",
            NodeKind::Element => &self.arena.tag_names[self.entry().tag as usize],
        }
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        let arena = self.arena;
        let e = self.entry();
        (e.attr_start..e.attr_end).map(move |a| {
            let a = a as usize;
            (
                arena.attr_names[arena.attr_name[a] as usize].as_str(),
                arena.str_at(arena.attr_val_off[a], arena.attr_val_len[a]),
            )
        })
    }

    fn text(self) -> &'a str {
        match self.kind() {
            NodeKind::Text => {
                let e = self.entry();
                self.arena.str_at(e.text_off, e.text_len)
            }
            NodeKind::Element => "",
        }
    }

    fn tree_scope(self) -> TreeScope {
        TreeScope(
            self.arena
                .scopes
                .get(self.idx as usize)
                .copied()
                .unwrap_or(0),
        )
    }

    fn document_order(self, other: Self) -> Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl Document for Arena {
    type N<'a> = ArenaNode<'a>;

    fn root(&self) -> Self::N<'_> {
        ArenaNode {
            arena: self,
            idx: 0,
        }
    }

    fn node_count(&self) -> Option<usize> {
        Some(self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::build;
    use a11y_dom::{elements, subtree_text};

    fn doc() -> Arena {
        build(|b| {
            b.open("html")
                .attr("lang", "de")
                .open("body")
                .open("h1")
                .text("Titel")
                .close()
                .open("p")
                .text("Ein ")
                .open("a")
                .attr("href", "/x")
                .text("Link")
                .close()
                .text(".")
                .close()
                .close()
                .close();
        })
    }

    #[test]
    fn wurzel_und_attribute() {
        let d = doc();
        assert_eq!(d.root().local_name(), "html");
        assert_eq!(d.root().attr("lang"), Some("de"));
        assert!(!d.root().has_attr("dir"));
    }

    #[test]
    fn nachfahren_kommen_in_dokumentreihenfolge() {
        let d = doc();
        let namen: Vec<&str> = elements(&d).map(|n| n.local_name()).collect();
        assert_eq!(namen, vec!["html", "body", "h1", "p", "a"]);
    }

    #[test]
    fn textknoten_sind_text_und_haben_keinen_tagnamen() {
        let d = doc();
        let p = elements(&d).find(|n| n.local_name() == "p").unwrap();
        assert_eq!(p.text(), "");
        assert_eq!(subtree_text(p), "Ein Link.");
    }

    #[test]
    fn dokument_ohne_text_hat_keine_textkennung() {
        let d = build(|b| {
            b.open("html")
                .open("img")
                .attr("src", "a.png")
                .close()
                .close();
        });
        assert!(d.text_tag.is_none());
        assert_eq!(elements(&d).count(), 2);
    }

    /// Gültige Spalten für `<a href="x">t</a>`: Knoten 0 Element, 1 Text.
    struct Cols {
        tag: Vec<u32>,
        parent: Vec<i32>,
        text_off: Vec<u32>,
        text_len: Vec<u32>,
        attr_start: Vec<u32>,
        attr_name: Vec<u32>,
        attr_val_off: Vec<u32>,
        attr_val_len: Vec<u32>,
        blob: usize,
        tags: usize,
        attrs: usize,
    }

    fn gut() -> Cols {
        Cols {
            tag: vec![0, 1],
            parent: vec![-1, 0],
            text_off: vec![1, 0],
            text_len: vec![0, 1],
            attr_start: vec![0, 1, 1],
            attr_name: vec![0],
            attr_val_off: vec![1],
            attr_val_len: vec![1],
            blob: 2,
            tags: 2,
            attrs: 1,
        }
    }

    fn pruefe(c: &Cols) -> Result<(), String> {
        validate_columns(
            &c.tag,
            &c.parent,
            &c.text_off,
            &c.text_len,
            &c.attr_start,
            &c.attr_name,
            &c.attr_val_off,
            &c.attr_val_len,
            c.blob,
            c.tags,
            c.attrs,
        )
    }

    fn fehler(c: &Cols) -> String {
        pruefe(c).expect_err("Spalten müssen abgelehnt werden")
    }

    #[test]
    fn gueltige_spalten_werden_angenommen() {
        assert_eq!(pruefe(&gut()), Ok(()));
    }

    #[test]
    fn leere_arena_wird_abgelehnt() {
        let mut c = gut();
        c.tag = vec![];
        c.parent = vec![];
        c.text_off = vec![];
        c.text_len = vec![];
        c.attr_start = vec![0];
        assert!(fehler(&c).contains("tag is empty"));
    }

    #[test]
    fn knotenspalten_muessen_so_lang_sein_wie_tag() {
        let mut c = gut();
        c.parent.pop();
        assert!(fehler(&c).contains("parent has 1 entries, tag has 2"));
        let mut c = gut();
        c.text_off.push(0);
        assert!(fehler(&c).contains("text_off has 3"));
        let mut c = gut();
        c.text_len.pop();
        assert!(fehler(&c).contains("text_len has 1"));
    }

    #[test]
    fn attr_start_braucht_knotenzahl_plus_eins() {
        let mut c = gut();
        c.attr_start.pop();
        assert!(fehler(&c).contains("attr_start has 2 entries, expected 3"));
    }

    #[test]
    fn attributspalten_muessen_gleich_lang_sein() {
        let mut c = gut();
        c.attr_val_off.push(0);
        assert!(fehler(&c).contains("attr_val_off has 2 entries, attr_name has 1"));
        let mut c = gut();
        c.attr_val_len.clear();
        assert!(fehler(&c).contains("attr_val_len has 0"));
    }

    #[test]
    fn attr_start_muss_bei_null_beginnen_monoton_sein_und_bei_der_attributzahl_enden() {
        let mut c = gut();
        c.attr_start = vec![1, 1, 1];
        assert!(fehler(&c).contains("attr_start[0]"));
        let mut c = gut();
        c.attr_start = vec![0, 1, 0];
        assert!(fehler(&c).contains("not monotonic"));
        let mut c = gut();
        c.attr_start = vec![0, 1, 5];
        assert!(fehler(&c).contains("attr_start ends at 5"));
    }

    #[test]
    fn indizes_muessen_im_woerterbuch_liegen() {
        let mut c = gut();
        c.tag[1] = 2;
        assert!(fehler(&c).contains("tag[1] = 2"));
        let mut c = gut();
        c.attr_name[0] = 1;
        assert!(fehler(&c).contains("attr_name[0] = 1"));
    }

    #[test]
    fn eltern_muessen_vor_dem_kind_stehen() {
        let mut c = gut();
        c.parent[1] = 1;
        assert!(fehler(&c).contains("parent[1] = 1"));
        let mut c = gut();
        c.parent[1] = -2;
        assert!(fehler(&c).contains("parent[1] = -2"));
        let mut c = gut();
        c.parent[0] = 0;
        assert!(fehler(&c).contains("parent[0] = 0"));
    }

    #[test]
    fn texte_und_werte_muessen_im_blob_liegen() {
        let mut c = gut();
        c.text_len[1] = 3;
        assert!(fehler(&c).contains("text of node 1"));
        let mut c = gut();
        c.attr_val_off[0] = u32::MAX;
        assert!(fehler(&c).contains("value of attribute 0"));
    }

    #[test]
    fn bereiche_werden_geprueft_und_ohne_aufruf_ist_alles_dokument() {
        let mut d = doc();
        assert!(d.root().tree_scope() == TreeScope(0));
        let n = d.len();
        assert!(
            d.set_scopes(vec![0; n - 1])
                .unwrap_err()
                .contains("scopes has")
        );
        let mut s = vec![0; n];
        s[0] = 1;
        assert!(d.set_scopes(s).unwrap_err().contains("scopes[0] is 1"));
        let mut s = vec![0; n];
        s[2] = n as u32;
        assert!(d.set_scopes(s).unwrap_err().contains("scopes[2]"));
        let mut s = vec![0; n];
        s[2] = 2;
        d.set_scopes(s).unwrap();
        let drei = elements(&d).nth(2).unwrap();
        assert!(drei.tree_scope() == TreeScope(2));
    }

    #[test]
    fn from_columns_meldet_statt_zu_paniken() {
        let r = Arena::from_columns(
            &[0],
            &[-1],
            &[0],
            &[0],
            &[0], // zu kurz: braucht zwei Einträge
            vec![],
            vec![],
            vec![],
            vec![],
            vec!["html".into()],
            vec![],
        );
        assert!(r.is_err());
    }
}
