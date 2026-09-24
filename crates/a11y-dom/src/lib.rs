//! Dokumentmodell-Abstraktion für Accessibility-Regeln.
//!
//! Regeln werden einmal geschrieben und laufen über drei sehr verschiedene
//! Substrate: statisches HTML aus einem Build, ein per CDP ferngesteuerter
//! Chrome, und der DOM einer laufenden Seite. Dieses Crate definiert, was diese
//! drei gemeinsam haben — und, wichtiger, wie sie sich unterscheiden.
//!
//! # Der Baum ist DOM-förmig
//!
//! [`Node`] bildet Tags, Attribute, Text und Hierarchie ab, nicht Rollen und
//! Accessible Names. Das ist bewusst: Die Mehrzahl der Regeln braucht
//! Attribute (`tabindex`, `id`, `role`, `alt`, `for`), und der native
//! Accessibility-Tree des Browsers gibt die gar nicht her — `tabindex` etwa
//! taucht dort nicht auf. Rolle und Name kommen als eigene Fähigkeit obendrauf.
//!
//! # Fähigkeiten statt Optionen
//!
//! Die drei Substrate unterscheiden sich nicht in der Darstellung derselben
//! Daten, sondern darin, welche Daten überhaupt existieren. Ein flaches Trait
//! mit `Option`-Rückgaben würde dazu führen, dass Regeln je nach Host
//! stillschweigend nicht laufen. Stattdessen gibt es [`Semantics`],
//! [`Rendering`] und [`Interaction`] als eigene Traits, die ein Host
//! implementiert oder eben nicht — und eine Regel, deren [`Tier`] nicht erfüllt
//! ist, meldet `UNTESTED` statt zu schweigen.
//!
//! ```
//! use a11y_dom::{elements, subtree_text, Arena, Document, Node};
//!
//! let doc = Arena::builder()
//!     .open("html").attr("lang", "de")
//!         .open("body")
//!             .open("h1").text("Bericht").close()
//!             .open("img").attr("src", "logo.png").close()
//!         .close()
//!     .close()
//!     .build();
//!
//! let img = elements(&doc).find(|n| n.local_name() == "img").unwrap();
//! assert!(img.attr("alt").is_none());
//!
//! let h1 = elements(&doc).find(|n| n.local_name() == "h1").unwrap();
//! assert_eq!(subtree_text(h1), "Bericht");
//! ```

#![forbid(unsafe_code)]

mod arena;
mod tiers;
mod tree;

pub use arena::{Arena, ArenaBuilder, ArenaNode};
pub use tiers::{
    Caps, Color, ComputedStyle, Interaction, NameSource, Rect, Rendering, Semantics, Tier,
};
pub use tree::{
    Document, Node, NodeId, NodeKind, ancestors, closest, descendants, elements, has_text,
    self_and_descendants, subtree_text,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Arena {
        Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("Seite")
            .close()
            .close()
            .open("body")
            .open("main")
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
            .close()
            .close()
            .build()
    }

    #[test]
    fn wurzel_und_attribute() {
        let d = doc();
        assert_eq!(d.root().local_name(), "html");
        assert_eq!(d.root().attr("lang"), Some("de"));
        assert!(d.root().has_attr("lang"));
        assert!(!d.root().has_attr("dir"));
    }

    #[test]
    fn nachfahren_kommen_in_dokumentreihenfolge() {
        let d = doc();
        let namen: Vec<&str> = elements(&d).map(|n| n.local_name()).collect();
        assert_eq!(
            namen,
            vec!["html", "head", "title", "body", "main", "h1", "p", "a"]
        );
    }

    #[test]
    fn subtree_text_sammelt_ueber_verschachtelung() {
        let d = doc();
        let p = elements(&d).find(|n| n.local_name() == "p").unwrap();
        assert_eq!(subtree_text(p), "Ein Link.");
        assert!(has_text(p));
    }

    #[test]
    fn eigener_text_ist_nicht_teilbaumtext() {
        let d = doc();
        let p = elements(&d).find(|n| n.local_name() == "p").unwrap();
        // Ein Element traegt selbst keinen Text; der steckt in seinen Textkindern.
        assert_eq!(p.text(), "");
        assert_eq!(subtree_text(p), "Ein Link.");
    }

    #[test]
    fn vorfahren_und_closest() {
        let d = doc();
        let a = elements(&d).find(|n| n.local_name() == "a").unwrap();
        let kette: Vec<&str> = ancestors(a).map(|n| n.local_name()).collect();
        assert_eq!(kette, vec!["p", "main", "body", "html"]);
        assert_eq!(closest(a, "main").map(|n| n.local_name()), Some("main"));
        // closest schliesst den Knoten selbst ein
        assert_eq!(closest(a, "a").map(|n| n.local_name()), Some("a"));
        assert!(closest(a, "form").is_none());
    }

    #[test]
    fn knoten_lassen_sich_ueber_die_kennung_zurueckholen() {
        let d = doc();
        let h1 = elements(&d).find(|n| n.local_name() == "h1").unwrap();
        let wieder = d.get(h1.id()).unwrap();
        assert_eq!(wieder, h1);
        assert!(d.get(NodeId(9999)).is_none());
    }

    #[test]
    fn caps_kennt_struktur_immer() {
        let nur_struktur = Caps::STRUCTURE_ONLY;
        assert!(nur_struktur.has(Tier::Structure));
        assert!(!nur_struktur.has(Tier::Semantics));

        let voll = Caps::default()
            .with_semantics()
            .with_rendering()
            .with_interaction();
        assert!(voll.has(Tier::Rendering));
        assert!(voll.has(Tier::Interaction));
    }

    #[test]
    fn node_count_ist_optional_aber_hier_bekannt() {
        let d = doc();
        assert_eq!(d.node_count(), Some(d.len()));
    }
}
