//! Normative Zuordnung (auditmysite#779): Wo eine Regel nur eine Best
//! Practice oder eine Vermutung belegt, trägt ihr Befund kein WCAG-Kriterium
//! als Verstoß — sondern das Schlagwort `best-practice` ohne Kriterium oder
//! `REVIEW`. Jeder Fall hier scheiterte an 0.25.0, weil der Befund dort
//! `FAIL` mit Kriterium war. Die Entscheidungen samt Begründung stehen im
//! Changelog des Crates.

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, Node, Semantics};
use a11y_report::{Finding, Outcome};
use a11y_rules::run_with_semantics;
use accname::IdIndex;
use html5_parser::NodeKind as H;

fn html(body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"de\"><head><title>T</title></head><body>{body}</body></html>"
    ))
    .document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    uebernehmen(&doc, wurzel, Arena::builder()).build()
}

fn uebernehmen(
    doc: &html5_parser::Document,
    id: html5_parser::NodeId,
    b: ArenaBuilder,
) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.close()
        }
        H::Text { content } => b.text(content),
        H::DocumentFragment => doc.children(id).fold(b, |b, k| uebernehmen(doc, k, b)),
        _ => b,
    }
}

struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

/// Die Befunde einer Kennung, Tier 1 und 2.
fn befunde(body: &str, id: &str) -> Vec<Finding> {
    let a = html(body);
    let r = run_with_semantics(&MitSemantik {
        doc: &a,
        ids: IdIndex::build(a.root()),
    });
    r.findings.into_iter().filter(|f| f.rule_id == id).collect()
}

fn einer(body: &str, id: &str) -> Finding {
    let mut v = befunde(body, id);
    assert_eq!(v.len(), 1, "{id} in {body}: {v:?}");
    v.remove(0)
}

/// Best Practice: `FAIL` bleibt das Urteil über die Empfehlung, aber ohne
/// Kriterium — wie `zoom/viewport-scale-limited` seit 0.24.1.
fn best_practice(body: &str, id: &str) {
    let f = einer(body, id);
    assert!(f.wcag.is_empty(), "{id}: {:?}", f.wcag);
    assert_eq!(f.tags, ["best-practice"], "{id}");
}

fn review(body: &str, id: &str, wcag: &[&str]) {
    let f = einer(body, id);
    assert_eq!(f.outcome, Outcome::Review, "{id}");
    assert_eq!(f.wcag, wcag, "{id}");
}

// --- Überschriften und Landmarks ----------------------------------------------

/// Der Fall aus dem Issue: eine korrekt ausgezeichnete h2, keine h1. Die h2
/// ist als Überschrift bestimmbar, 1.3.1 ist erfüllt.
#[test]
fn h2_ohne_h1_ist_best_practice() {
    best_practice(
        "<main><h2>Abschnitt</h2><p>Text</p></main>",
        "headings/h1-missing",
    );
}

#[test]
fn uebersprungene_ebene_ist_best_practice() {
    best_practice(
        "<main><h1>Titel</h1><h3>Unterpunkt</h3></main>",
        "headings/skip-level",
    );
}

/// Ohne main, aber mit Überschriften: 2.4.1 ist über H69 erfüllt, 1.3.1
/// verlangt keine Landmarks.
#[test]
fn fehlende_main_ist_best_practice() {
    let body = "<div><h1>Titel</h1><p>Text</p></div>";
    best_practice(body, "landmarks/main-missing");
    assert!(befunde(body, "headings/none").is_empty());
}

#[test]
fn doppelte_main_ist_best_practice() {
    best_practice(
        "<main><h1>A</h1></main><main><p>B</p></main>",
        "landmarks/main-duplicate",
    );
}

#[test]
fn verschachtelte_und_doppelte_landmarks_sind_best_practice() {
    best_practice(
        r#"<main><div role="banner">Kopf</div><h1>A</h1></main>"#,
        "landmarks/not-top-level",
    );
    best_practice(
        r#"<div role="banner">A</div><div role="banner">B</div><main><h1>x</h1></main>"#,
        "landmarks/banner-duplicate",
    );
    best_practice(
        "<p>Draußen</p><main><h1>Drinnen</h1></main>",
        "landmarks/content-outside",
    );
}

/// Ohne Viewport-Angabe bleibt Zoomen erlaubt; 1.4.10 wird über die
/// Vergrößerung im Browser geprüft.
#[test]
fn fehlender_viewport_ist_best_practice() {
    best_practice("<main><h1>A</h1></main>", "zoom/viewport-missing");
}

// --- Rollen, Listen, Tabellen -------------------------------------------------

#[test]
fn ueberfluessige_rolle_ist_best_practice() {
    best_practice(
        r#"<main><h1>A</h1><button role="button">Los</button></main>"#,
        "aria/role-redundant",
    );
}

/// 1.4.13 regelt Erscheinen und Verschwinden, nicht die Verknüpfung.
#[test]
fn unverknuepfter_tooltip_ist_best_practice() {
    best_practice(
        r#"<main><h1>A</h1><div role="tooltip" id="t">Hilfe</div></main>"#,
        "patterns/tooltip-unreferenced",
    );
}

#[test]
fn leere_liste_ist_best_practice() {
    best_practice("<main><h1>A</h1><ul></ul></main>", "lists/empty");
}

/// Layouttabelle oder Datentabelle ohne Köpfe — das Markup sagt es nicht.
#[test]
fn tabelle_ohne_th_ist_review() {
    review(
        "<main><h1>A</h1><table><tr><td>a</td><td>b</td></tr></table></main>",
        "tables/header-missing",
        &["1.3.1"],
    );
}

// --- Tastatur und Verweise ----------------------------------------------------

/// F44 verlangt ein Urteil über die entstehende Reihenfolge.
#[test]
fn positiver_tabindex_ist_review() {
    review(
        r#"<main><h1>A</h1><button tabindex="2">Los</button></main>"#,
        "keyboard/positive-tabindex",
        &["2.4.3"],
    );
}

/// Der Browser nimmt das erste Element; ob das gemeint ist, sagt das Markup
/// nicht.
#[test]
fn doppelte_referenzierte_id_ist_review() {
    review(
        r#"<main><h1>A</h1><label for="x">Name</label><input id="x"><input id="x" aria-label="B"></main>"#,
        "ids/duplicate",
        &["4.1.2"],
    );
}

/// Eine Karte um ihren Link: Die Funktion ist per Tastatur über den Link da.
/// Ohne fokussierbares Element darin bleibt es ein Verstoß.
#[test]
fn klick_handler_um_fokussierbares_ist_review() {
    review(
        r#"<main><h1>A</h1><div id="k" onclick="go()"><a href="/x">Mehr</a></div></main>"#,
        "keyboard/click-handler-not-focusable",
        &["2.1.1"],
    );
    let f = einer(
        r#"<main><h1>A</h1><div id="k" onclick="go()">Mehr</div></main>"#,
        "keyboard/click-handler-not-focusable",
    );
    assert_eq!(f.outcome, Outcome::Fail);
    assert_eq!(f.wcag, ["2.1.1"]);
}

/// ACT cae760 führt nur 4.1.2; für 2.4.1 ist der Titel eine Technik unter
/// mehreren (H64).
#[test]
fn rahmen_ohne_namen_nur_4_1_2() {
    let f = einer(
        r#"<main><h1>A</h1><iframe src="x.html"></iframe></main>"#,
        "frames/name-missing",
    );
    assert_eq!(f.outcome, Outcome::Fail);
    assert_eq!(f.wcag, ["4.1.2"]);
}

// --- Namen und Formulare ------------------------------------------------------

/// ACT 7d6734 gilt nur für SVG mit ausdrücklicher Bildrolle.
#[test]
fn svg_ohne_rolle_ist_review() {
    review(
        r#"<main><h1>A</h1><svg><circle r="1"></circle></svg></main>"#,
        "svg/name-missing",
        &["1.1.1"],
    );
    let f = einer(
        r#"<main><h1>A</h1><svg role="img"><circle r="1"></circle></svg></main>"#,
        "svg/name-missing",
    );
    assert_eq!(f.outcome, Outcome::Fail);
}

/// Der Platzhalter stiftet den Namen (ACT e086e5 besteht): kein
/// unbeschriftetes Feld, sondern eine Frage an den Prüfer.
#[test]
fn platzhalter_ist_kein_fehlendes_label() {
    let body = r#"<main><h1>A</h1><input id="p" placeholder="Suche"></main>"#;
    assert!(befunde(body, "forms/label-missing").is_empty());
    review(body, "forms/placeholder-as-label", &["3.3.2"]);
}

/// Ein `fieldset` kann auch nur gestalten.
#[test]
fn fieldset_ohne_legend_ist_review() {
    review(
        r#"<main><h1>A</h1><fieldset><input type="checkbox" aria-label="Ja"></fieldset></main>"#,
        "forms/group-name-missing",
        &["1.3.1", "3.3.2"],
    );
}
