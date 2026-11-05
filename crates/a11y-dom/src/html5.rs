//! [`Document`] für den Parsebaum von `html5-parser`, ohne Kopie in eine
//! [`crate::Arena`]. Nur mit dem Feature `html5-parser`.
//!
//! Abbildung auf das Trait:
//!
//! - **Wurzel** ist das erste Element unter dem Dokumentknoten (`<html>`), wie
//!   bei der [`crate::Arena`].
//! - **Elemente und Text** erscheinen, wie der Parser sie ablegt: Tagnamen so
//!   gespeichert (HTML kleingeschrieben, SVG mit der Schreibweise der
//!   Spezifikation, etwa `foreignObject`), Attribute mit ihrem gespeicherten
//!   Namen. Kommentare, Processing Instructions und Doctype fehlen, wie
//!   [`NodeKind`] es vorsieht.
//! - **`<template>`**: Der Parser hängt den Inhalt unter ein
//!   `DocumentFragment`. Das Fragment wird übersprungen, der Inhalt erscheint
//!   als Kinder des `<template>`-Elements. Ein `shadowrootmode` wird nicht als
//!   Shadow Root gedeutet; jeder Knoten liegt im Bereich des Dokuments.
//! - **[`Node::id`]** ist der Arena-Index des Parsers. Er folgt der
//!   Anlegereihenfolge, nicht der Dokumentreihenfolge; [`Node::document_order`]
//!   ist deshalb überschrieben und läuft über den Baum.

use std::cmp::Ordering;

use html5_parser::{Document as Html5Document, NodeId as Html5Id, NodeKind as H};

use crate::tree::{Document, Node, NodeId, NodeKind};

/// Ein Handle auf einen Knoten eines `html5_parser::Document`.
#[derive(Debug, Clone, Copy)]
pub struct Html5Node<'a> {
    doc: &'a Html5Document,
    id: Html5Id,
}

impl PartialEq for Html5Node<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.doc, other.doc) && self.id == other.id
    }
}

impl Eq for Html5Node<'_> {}

fn is_visible(doc: &Html5Document, id: Html5Id) -> bool {
    matches!(doc.node(id).kind, H::Element { .. } | H::Text { .. })
}

/// Die sichtbaren Kinder von `id`: Fragmente aufgelöst, nur Elemente und Text.
fn visible_children<'a>(doc: &'a Html5Document, id: Html5Id) -> impl Iterator<Item = Html5Id> + 'a {
    doc.children(id)
        .flat_map(move |c| {
            let fragment = matches!(doc.node(c).kind, H::DocumentFragment);
            let single = (!fragment).then_some(c);
            single
                .into_iter()
                .chain(fragment.then(|| doc.children(c)).into_iter().flatten())
        })
        .filter(move |&c| is_visible(doc, c))
}

impl<'a> Html5Node<'a> {
    /// Der Weg von der Wurzel bis zum Knoten, den Knoten eingeschlossen.
    fn path(self) -> Vec<Html5Node<'a>> {
        let mut path = vec![self];
        let mut cur = self;
        while let Some(p) = cur.parent() {
            path.push(p);
            cur = p;
        }
        path.reverse();
        path
    }
}

impl<'a> Node<'a> for Html5Node<'a> {
    fn id(self) -> NodeId {
        NodeId(self.id.as_u32())
    }

    fn kind(self) -> NodeKind {
        match self.doc.node(self.id).kind {
            H::Text { .. } => NodeKind::Text,
            _ => NodeKind::Element,
        }
    }

    fn parent(self) -> Option<Self> {
        let mut parent = self.doc.parent(self.id)?;
        if matches!(self.doc.node(parent).kind, H::DocumentFragment) {
            parent = self.doc.parent(parent)?;
        }
        is_visible(self.doc, parent).then_some(Html5Node {
            doc: self.doc,
            id: parent,
        })
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let doc = self.doc;
        visible_children(doc, self.id).map(move |id| Html5Node { doc, id })
    }

    fn local_name(self) -> &'a str {
        match &self.doc.node(self.id).kind {
            H::Element { name, .. } => name,
            _ => "",
        }
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        let attributes: &'a [html5_parser::Attribute] = match &self.doc.node(self.id).kind {
            H::Element { attributes, .. } => attributes,
            _ => &[],
        };
        attributes
            .iter()
            .map(|a| (a.name.as_str(), a.value.as_str()))
    }

    fn text(self) -> &'a str {
        match &self.doc.node(self.id).kind {
            H::Text { content } => content,
            _ => "",
        }
    }

    fn document_order(self, other: Self) -> Ordering {
        let (a, b) = (self.path(), other.path());
        let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        match (a.get(common), b.get(common)) {
            (Some(&x), Some(&y)) if common > 0 => {
                let first = a[common - 1].children().find(|&s| s == x || s == y);
                if first == Some(x) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl Document for Html5Document {
    type N<'a>
        = Html5Node<'a>
    where
        Self: 'a;

    /// Das erste Element unter dem Dokumentknoten. Ein vollständig geparstes
    /// Dokument hat immer eines: der Parser ergänzt `<html>`.
    fn root(&self) -> Self::N<'_> {
        let id = visible_children(self, html5_parser::Document::root(self))
            .find(|&c| matches!(self.node(c).kind, H::Element { .. }))
            .expect("geparstes Dokument ohne Wurzelelement");
        Html5Node { doc: self, id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arena, ArenaBuilder, ancestors, elements, subtree_text};

    const SRC: &str = "<!DOCTYPE html><html lang=\"de\"><head><title>Seite</title></head>\
        <body><!-- Kommentar --><main><h1>Titel</h1>\
        <p>Ein <a href=\"/x\" class=\"l\">Link</a>.</p>\
        <template><b>inert</b></template></main></body></html>";

    /// `Document::root`; auf dem Parser-Typ verdeckt `html5_parser::Document::root`
    /// (Dokumentknoten) die Trait-Methode.
    fn trait_root(doc: &Html5Document) -> Html5Node<'_> {
        <Html5Document as Document>::root(doc)
    }

    fn names<D: Document>(doc: &D) -> Vec<&str> {
        elements(doc).map(|n| n.local_name()).collect()
    }

    #[test]
    fn elemente_in_dokumentreihenfolge() {
        let doc = html5_parser::parse(SRC).document;
        assert_eq!(
            names(&doc),
            vec![
                "html", "head", "title", "body", "main", "h1", "p", "a", "template", "b"
            ]
        );
        assert_eq!(trait_root(&doc).local_name(), "html");
        assert!(trait_root(&doc).parent().is_none());
    }

    #[test]
    fn attribute_text_und_vorfahren() {
        let doc = html5_parser::parse(SRC).document;
        let a = elements(&doc).find(|n| n.local_name() == "a").unwrap();
        assert_eq!(a.attr("href"), Some("/x"));
        assert!(a.has_attr("class"));
        assert!(a.attr("alt").is_none());
        assert_eq!(trait_root(&doc).attr("lang"), Some("de"));
        assert_eq!(subtree_text(a), "Link");
        let p = a.parent().unwrap();
        assert_eq!(subtree_text(p), "Ein Link.");
        let up: Vec<&str> = ancestors(a).map(|n| n.local_name()).collect();
        assert_eq!(up, vec!["p", "main", "body", "html"]);
    }

    #[test]
    fn kommentare_und_doctype_fehlen_template_inhalt_ist_kind() {
        let doc = html5_parser::parse(SRC).document;
        let body = elements(&doc).find(|n| n.local_name() == "body").unwrap();
        assert!(body.children().all(|c| c.kind() == NodeKind::Element));
        let template = elements(&doc)
            .find(|n| n.local_name() == "template")
            .unwrap();
        let kids: Vec<&str> = template.children().map(|n| n.local_name()).collect();
        assert_eq!(kids, vec!["b"]);
        assert_eq!(kids.len(), 1);
        let b = template.children().next().unwrap();
        assert_eq!(b.parent(), Some(template));
        assert_eq!(subtree_text(b), "inert");
    }

    /// Die Kopie, wie ein Host sie bisher schreibt (Korpus-Host): Elemente,
    /// Attribute und Text in eine Arena, das Template-Fragment aufgelöst.
    fn kopieren(doc: &Html5Document, id: Html5Id, b: ArenaBuilder) -> ArenaBuilder {
        match &doc.node(id).kind {
            H::Element {
                name, attributes, ..
            } => {
                let mut b = b.open(name);
                for a in attributes {
                    b = b.attr(&a.name, &a.value);
                }
                for k in doc.children(id) {
                    b = kopieren(doc, k, b);
                }
                b.close()
            }
            H::DocumentFragment => {
                let mut b = b;
                for k in doc.children(id) {
                    b = kopieren(doc, k, b);
                }
                b
            }
            H::Text { content } => b.text(content),
            _ => b,
        }
    }

    type Zeichen = (String, Vec<(String, String)>, String);

    fn zeichen<D: Document>(doc: &D) -> Vec<Zeichen> {
        crate::self_and_descendants(doc.root())
            .map(|n| {
                (
                    // Die Arena nennt Textknoten „#text", das Trait verlangt leer.
                    if n.kind() == NodeKind::Element {
                        n.local_name().to_string()
                    } else {
                        String::new()
                    },
                    n.attributes()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                    n.text().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn gleich_der_kopie_in_eine_arena() {
        let doc = html5_parser::parse(SRC).document;
        let wurzel = doc
            .children(doc.root())
            .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
            .unwrap();
        let arena: Arena = kopieren(&doc, wurzel, Arena::builder()).build();
        assert_eq!(zeichen(&doc), zeichen(&arena));
    }

    #[test]
    fn dokumentreihenfolge_trotz_umgehaengter_knoten() {
        // „foo" wird vor die Tabelle umgehängt (Foster Parenting), ist aber
        // nach ihr angelegt worden: Arena-Index und Dokumentreihenfolge
        // weichen ab.
        let doc = html5_parser::parse("<table><tr><td>a</td></tr>foo</table>").document;
        let table = elements(&doc).find(|n| n.local_name() == "table").unwrap();
        let foo = crate::self_and_descendants(trait_root(&doc))
            .find(|n| n.kind() == NodeKind::Text && n.text() == "foo")
            .unwrap();
        assert!(foo.id() > table.id());
        assert_eq!(foo.document_order(table), Ordering::Less);
        assert_eq!(table.document_order(foo), Ordering::Greater);
        assert_eq!(table.document_order(table), Ordering::Equal);
        let td = elements(&doc).find(|n| n.local_name() == "td").unwrap();
        assert_eq!(table.document_order(td), Ordering::Less);
        assert_eq!(td.document_order(table), Ordering::Greater);
    }
}
