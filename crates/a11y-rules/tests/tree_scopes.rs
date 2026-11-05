//! IDs gelten je Baumbereich (casoon/barrierlab#23): Dokument und jede Shadow
//! Root haben ihre eigenen. Die Fälle sind die Abnahmekriterien des Issues.

use a11y_dom::{Arena, ArenaBuilder, Document, Node, TreeScope, elements};
use a11y_report::Report;
use a11y_rules::run;

fn page(content: impl FnOnce(ArenaBuilder) -> ArenaBuilder) -> Arena {
    content(
        Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("T")
            .close()
            .close()
            .open("body"),
    )
    .close()
    .close()
    .build()
}

fn count(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

/// Zwei Komponenten derselben Art, jede mit `id="hilfe"` und einem Feld, das
/// darauf zeigt — wie zwei Instanzen eines Web-Components.
#[test]
fn same_id_in_two_roots_is_not_a_duplicate() {
    let component = |b: ArenaBuilder| {
        b.open("x-feld")
            .shadow_root()
            .open("input")
            .attr("aria-describedby", "hilfe")
            .attr("aria-label", "Name")
            .close()
            .open("p")
            .attr("id", "hilfe")
            .text("Hilfe")
            .close()
            .end_scope()
            .close()
    };
    let r = run(&page(|b| component(component(b))));
    assert_eq!(count(&r, "ids/duplicate"), 0);
    assert_eq!(count(&r, "aria/reference-missing"), 0);
}

#[test]
fn same_id_in_same_scope_stays_duplicate() {
    let r = run(&page(|b| {
        b.open("input")
            .attr("aria-describedby", "hilfe")
            .attr("aria-label", "Name")
            .close()
            .open("p")
            .attr("id", "hilfe")
            .close()
            .open("p")
            .attr("id", "hilfe")
            .close()
    }));
    assert_eq!(count(&r, "ids/duplicate"), 1);
}

/// Ein Verweis aus dem Light DOM in eine Shadow Root trifft nichts.
#[test]
fn labelledby_into_a_shadow_root_is_missing() {
    let arena = page(|b| {
        b.open("button")
            .attr("aria-labelledby", "innen")
            .close()
            .open("x-text")
            .shadow_root()
            .open("span")
            .attr("id", "innen")
            .text("Senden")
            .close()
            .end_scope()
            .close()
    });
    assert_eq!(count(&run(&arena), "aria/reference-missing"), 1);

    let button = elements(&arena).find(|n| n.is_element("button")).unwrap();
    let ids = accname::IdIndex::build(arena.root());
    assert_eq!(accname::name(button, &ids), None);
}

#[test]
fn label_for_across_the_boundary_does_not_label() {
    let r = run(&page(|b| {
        b.open("label")
            .attr("for", "feld")
            .text("Name")
            .close()
            .open("x-feld")
            .shadow_root()
            .open("input")
            .attr("id", "feld")
            .close()
            .end_scope()
            .close()
    }));
    assert_eq!(count(&r, "forms/label-missing"), 1);
}

/// Per `<slot>` eingesetzter Inhalt gehört zum äußeren Bereich: Sein Verweis
/// auf ein Element im Light DOM trifft, auch wenn er im flachen Baum unter
/// der Shadow Root hängt.
#[test]
fn slotted_content_belongs_to_the_outer_scope() {
    let arena = page(|b| {
        b.open("p")
            .attr("id", "aussen")
            .text("Hinweis")
            .close()
            .open("x-rahmen")
            .shadow_root()
            .open("div")
            .open("slot")
            .slotted()
            .open("input")
            .attr("aria-label", "Name")
            .attr("aria-describedby", "aussen")
            .close()
            .end_scope()
            .close()
            .close()
            .end_scope()
            .close()
    });
    assert_eq!(count(&run(&arena), "aria/reference-missing"), 0);

    let area = |name: &str| {
        elements(&arena)
            .find(|n| n.is_element(name))
            .unwrap()
            .tree_scope()
    };
    assert_eq!(area("input"), TreeScope(0));
    assert_eq!(area("x-rahmen"), TreeScope(0));
    assert_ne!(area("slot"), TreeScope(0));
}
