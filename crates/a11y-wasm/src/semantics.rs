//! Tier 2 über der Arena: Rolle und Accessible Name, berechnet von `accname`.
//!
//! In-Page-JavaScript kommt nicht an den nativen Accessibility-Tree —
//! `getComputedRole`/`getComputedLabel` sind WebDriver-Befehle, keine
//! In-Page-APIs. Rolle und Name entstehen deshalb in Rust über der Arena.
//!
//! Der [`IdIndex`] wird **einmal je Scan** gebaut und hier gehalten. Er löst
//! `aria-labelledby`, `aria-describedby` und `label[for]` auf; ihn je Knoten neu
//! aufzubauen machte die Namensauflösung quadratisch.
//!
//! Dass dieser Host überhaupt möglich ist, liegt an der Lebenszeit-Kopplung der
//! Tier-Traits in `a11y-dom` (`&'n self` an `Self::N<'n>`): Der Host *leiht* die
//! Arena aus und hält den daraus abgeleiteten Index daneben.

use a11y_dom::{Document, NameSource, Semantics};
use accname::IdIndex;

use crate::arena::{Arena, ArenaNode};

/// Die Arena plus der einmal je Scan gebaute ID-Index.
pub struct SemanticArena<'a> {
    arena: &'a Arena,
    pub(crate) ids: IdIndex<'a, ArenaNode<'a>>,
}

impl<'a> SemanticArena<'a> {
    pub fn new(arena: &'a Arena) -> Self {
        let ids = IdIndex::build(arena.root());
        SemanticArena { arena, ids }
    }
}

impl Document for SemanticArena<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.arena.root()
    }

    fn node_count(&self) -> Option<usize> {
        Some(self.arena.len())
    }
}

impl Semantics for SemanticArena<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    /// Ohne nativen Accessibility-Tree ist nicht bekannt, *woher* ein Name
    /// stammt. `None` ist hier die ehrliche Antwort, nicht eine Lücke.
    fn name_source<'n>(&'n self, _node: Self::N<'n>) -> Option<NameSource> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::testing::build;
    use a11y_dom::{Node, elements};

    #[test]
    fn name_kommt_ueber_aria_labelledby() {
        let arena = build(|b| {
            b.open("form")
                .open("span")
                .attr("id", "l")
                .text("Vorname")
                .close()
                .open("input")
                .attr("aria-labelledby", "l")
                .close()
                .close();
        });
        let host = SemanticArena::new(&arena);
        let feld = elements(&host).find(|n| n.local_name() == "input").unwrap();
        assert_eq!(host.accessible_name(feld).as_deref(), Some("Vorname"));
    }

    #[test]
    fn rolle_wird_implizit_abgeleitet() {
        let arena = build(|b| {
            b.open("nav").open("button").text("Menü").close().close();
        });
        let host = SemanticArena::new(&arena);
        let button = elements(&host)
            .find(|n| n.local_name() == "button")
            .unwrap();
        assert_eq!(host.role(button).as_deref(), Some("button"));
        assert_eq!(host.accessible_name(button).as_deref(), Some("Menü"));
    }

    #[test]
    fn label_for_wird_ueber_den_index_aufgeloest() {
        let arena = build(|b| {
            b.open("form")
                .open("label")
                .attr("for", "e")
                .text("E-Mail")
                .close()
                .open("input")
                .attr("id", "e")
                .attr("type", "email")
                .close()
                .close();
        });
        let host = SemanticArena::new(&arena);
        let feld = elements(&host).find(|n| n.local_name() == "input").unwrap();
        assert_eq!(host.accessible_name(feld).as_deref(), Some("E-Mail"));
    }

    /// Das Dokument trägt `l` und `doc`; die Shadow Root trägt ihr eigenes `l`
    /// und `inner`. Jedes Feld verweist auf eine ID; welche trifft, bestimmt
    /// der Bereich.
    fn schatten() -> Arena {
        build(|b| {
            b.open("body")
                .open("span")
                .attr("id", "l")
                .text("Dokument")
                .close()
                .open("span")
                .attr("id", "doc")
                .text("Nur außen")
                .close()
                .open("input")
                .attr("id", "aussen-l")
                .attr("aria-labelledby", "l")
                .close()
                .open("input")
                .attr("id", "aussen-inner")
                .attr("aria-labelledby", "inner")
                .close()
                .open("div") // Host
                .shadow_root()
                .open("span")
                .attr("id", "l")
                .text("Schatten")
                .close()
                .open("span")
                .attr("id", "inner")
                .text("Nur innen")
                .close()
                .open("input")
                .attr("id", "innen-l")
                .attr("aria-labelledby", "l")
                .close()
                .open("input")
                .attr("id", "innen-doc")
                .attr("aria-labelledby", "doc")
                .close()
                .open("slot")
                .slotted()
                .open("span")
                .attr("id", "geschlitzt")
                .text("Aus dem Slot")
                .close()
                .end_scope()
                .open("input")
                .attr("id", "innen-slot")
                .attr("aria-labelledby", "geschlitzt")
                .close()
                .close()
                .end_scope()
                .close()
                .close();
        })
    }

    fn name_von(host: &SemanticArena, id: &str) -> Option<String> {
        let feld = elements(host).find(|n| n.attr("id") == Some(id)).unwrap();
        host.accessible_name(feld).filter(|n| !n.is_empty())
    }

    /// Eine ID trifft nur im eigenen Bereich: dieselbe ID in zwei Bereichen
    /// löst je Bereich auf das eigene Ziel auf.
    #[test]
    fn id_verweis_trifft_das_ziel_im_eigenen_bereich() {
        let arena = schatten();
        let host = SemanticArena::new(&arena);
        assert_eq!(name_von(&host, "aussen-l").as_deref(), Some("Dokument"));
        assert_eq!(name_von(&host, "innen-l").as_deref(), Some("Schatten"));
    }

    /// Kein Durchgriff in keine Richtung: ein Ziel nur innen ist von außen
    /// nicht zu erreichen und ein Ziel nur außen nicht von innen.
    #[test]
    fn id_verweis_greift_nicht_ueber_die_bereichsgrenze() {
        let arena = schatten();
        let host = SemanticArena::new(&arena);
        assert_eq!(name_von(&host, "aussen-inner"), None);
        assert_eq!(name_von(&host, "innen-doc"), None);
    }

    /// Eingesetzter Inhalt gehört zum äußeren Bereich, auch wenn er im flachen
    /// Baum unter dem Slot der Shadow Root hängt.
    #[test]
    fn eingesetzter_inhalt_gehoert_zum_aeusseren_bereich() {
        let arena = schatten();
        let host = SemanticArena::new(&arena);
        assert_eq!(name_von(&host, "innen-slot"), None);
    }

    /// Ohne Bereichsspalte bleibt alles im Dokument: dieselbe ID zweimal ist
    /// ein Duplikat. Mit ihr nicht.
    #[test]
    fn doppelte_id_in_zwei_bereichen_ist_kein_duplikat() {
        let mit = schatten();
        let ids = |a: &Arena| -> bool {
            a11y_rules::run_with_semantics(&SemanticArena::new(a))
                .findings
                .iter()
                .any(|f| f.rule_id == "ids/duplicate")
        };
        assert!(!ids(&mit));
        let mut flach = schatten();
        flach.set_scopes(vec![0; flach.len()]).unwrap();
        assert!(ids(&flach));
    }
}
