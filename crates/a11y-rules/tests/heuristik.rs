//! Die heuristischen Tier-3-Regeln (liveaudit#6): Jede meldet `REVIEW`, nie
//! `FAIL`, und ohne Layout-Daten des Hosts nichts — außer dem ungemessenen
//! Fokus, der als `UNTESTED` stehen bleibt.
//!
//! Der Test-Host liest Layout und Geometrie aus `data-*`-Attributen.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, ComputedStyle, Document, Layout, Node, Rect, Rendering,
    Semantics,
};
use a11y_report::{Outcome, Report};
use a11y_rules::run_full;
use accname::IdIndex;

struct Host<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    /// Ob der Host überhaupt Layout liefert.
    layout: bool,
}

impl<'a> Host<'a> {
    fn new(doc: &'a Arena) -> Self {
        Host {
            ids: IdIndex::build(doc.root()),
            doc,
            layout: true,
        }
    }
}

impl Document for Host<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for Host<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, _node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: None,
            font_weight: None,
            display: Some("block".into()),
            visibility: Some("visible".into()),
        })
    }

    /// `data-size="BxH"` oder `"BxH@X,Y"`; ohne Position bei (0, 0).
    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect> {
        let wert = node.attr("data-size")?;
        let (groesse, ort) = wert.split_once('@').unwrap_or((wert, "0,0"));
        let (w, h) = groesse.split_once('x')?;
        let (x, y) = ort.split_once(',')?;
        Some(Rect {
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            width: w.parse().ok()?,
            height: h.parse().ok()?,
        })
    }

    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        if !self.layout {
            return None;
        }
        Some(Layout {
            flex_reversed: node.has_attr("data-reversed"),
            order: node.attr("data-order").map_or(0, |o| o.parse().unwrap()),
            min_width_px: node
                .attr("data-min-width")
                .map_or(0.0, |o| o.parse().unwrap()),
            cursor_pointer: node.has_attr("data-pointer"),
            infinite_animation: node.has_attr("data-endlos"),
            obscured: node.has_attr("data-verdeckt"),
            hides_focus: node.has_attr("data-leiste"),
            focus_visible: node.attr("data-fokus").map(|f| f == "sichtbar"),
        })
    }
}

fn seite() -> ArenaBuilder {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .close()
        .open("body")
}

fn befunde<'r>(r: &'r Report, id: &str) -> Vec<&'r a11y_report::Finding> {
    r.findings.iter().filter(|f| f.rule_id == id).collect()
}

fn nur_review(r: &Report, id: &str) -> usize {
    let b = befunde(r, id);
    assert!(
        b.iter().all(|f| f.outcome == Outcome::Review),
        "{id}: {b:?}"
    );
    b.len()
}

#[test]
fn umgekehrte_reihenfolge_ueber_bedienbarem_inhalt() {
    let arena = seite()
        .open("div")
        .attr("data-reversed", "")
        .open("a")
        .attr("href", "/1")
        .text("Eins")
        .close()
        .open("a")
        .attr("href", "/2")
        .text("Zwei")
        .close()
        .close()
        // Umgekehrt, aber ohne Bedienbares: harmlos.
        .open("div")
        .attr("data-reversed", "")
        .open("p")
        .text("a")
        .close()
        .open("p")
        .text("b")
        .close()
        .close()
        .open("div")
        .open("button")
        .attr("data-order", "-1")
        .text("Vor")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "order/visual-mismatch"),
        2
    );
}

#[test]
fn endlos_animation_nur_am_aeussersten_element() {
    let arena = seite()
        .open("div")
        .attr("data-endlos", "")
        .open("span")
        .attr("data-endlos", "")
        .text("Laufschrift")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        1
    );
}

#[test]
fn min_width_ueber_320_ausser_tabellen() {
    let arena = seite()
        .open("div")
        .attr("data-min-width", "480")
        .text("Breit")
        .close()
        .open("div")
        .attr("data-min-width", "300")
        .text("Schmal genug")
        .close()
        .open("table")
        .attr("data-min-width", "900")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(nur_review(&r, "reflow/min-width"), 1);
    assert!(befunde(&r, "reflow/min-width")[0].message.contains("480"));
}

#[test]
fn anklickbares_div_ohne_rolle() {
    let arena = seite()
        // Das „Suchen" auf barrierlab.eu: sieht aus wie ein Knopf.
        .open("div")
        .attr("data-pointer", "")
        .open("span")
        .attr("data-pointer", "")
        .text("Suchen")
        .close()
        .close()
        .open("div")
        .attr("onclick", "ablehnen()")
        .text("Ablehnen")
        .close()
        // Richtig gebaut und Zeiger darin: kein Befund.
        .open("button")
        .open("span")
        .attr("data-pointer", "")
        .text("Senden")
        .close()
        .close()
        .open("div")
        .attr("role", "button")
        .attr("tabindex", "0")
        .attr("data-pointer", "")
        .text("Ok")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    // Das „Ablehnen" mit Inline-onclick meldet die Tier-1-Regel, nicht diese.
    assert_eq!(nur_review(&r, "keyboard/pointer-only"), 1);
    assert_eq!(befunde(&r, "keyboard/click-handler-not-focusable").len(), 1);
}

#[test]
fn verdecktes_bedienelement() {
    let arena = seite()
        .open("a")
        .attr("href", "/x")
        .attr("data-verdeckt", "")
        .text("Unter der Leiste")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "focus/obscured"),
        1
    );
}

#[test]
fn leiste_ohne_scroll_padding() {
    let arena = seite()
        .open("header")
        .attr("data-leiste", "")
        .text("Leiste")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(nur_review(&r, "focus/obscured"), 1);
    assert!(
        befunde(&r, "focus/obscured")[0]
            .message
            .contains("scroll-padding-top")
    );
}

#[test]
fn zu_kleine_ziele_ausser_im_fliesstext() {
    let arena = seite()
        .open("button")
        .attr("data-size", "16x16")
        .text("x")
        .close()
        .open("button")
        .attr("data-size", "44x44")
        .text("groß")
        .close()
        .open("p")
        .text("Mehr im ")
        .open("a")
        .attr("href", "/g")
        .attr("data-size", "40x14")
        .text("Glossar")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 1);
}

#[test]
fn fokus_ohne_indikator_und_ungemessen() {
    let gemessen = seite()
        .open("button")
        .attr("data-fokus", "unsichtbar")
        .text("A")
        .close()
        .open("button")
        .attr("data-fokus", "sichtbar")
        .text("B")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&gemessen));
    assert_eq!(nur_review(&r, "focus/indicator-missing"), 1);
    assert!(befunde(&r, "focus/indicator-unmeasured").is_empty());

    // Nicht gemessen ist nicht bestanden: ein UNTESTED für die Seite.
    let ungemessen = seite()
        .open("button")
        .text("A")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&ungemessen));
    let b = befunde(&r, "focus/indicator-unmeasured");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].outcome, Outcome::Untested);
}

#[test]
fn ohne_layout_melden_die_heuristiken_nichts() {
    let arena = seite()
        .open("div")
        .attr("data-reversed", "")
        .attr("data-endlos", "")
        .attr("data-min-width", "999")
        .attr("data-pointer", "")
        .text("Alles")
        .close()
        .close()
        .close()
        .build();
    let mut host = Host::new(&arena);
    host.layout = false;
    let r = run_full(&host);
    for id in [
        "order/visual-mismatch",
        "motion/infinite-animation",
        "reflow/min-width",
        "keyboard/pointer-only",
    ] {
        assert!(befunde(&r, id).is_empty(), "{id}");
    }
}

#[test]
fn kleines_ziel_mit_genug_abstand_ist_kein_befund() {
    // Zwei 16-px-Ziele, 40 px Mittenabstand: Die 24-px-Kreise berühren sich nicht.
    let arena = seite()
        .open("button")
        .attr("data-size", "16x16@0,0")
        .text("a")
        .close()
        .open("button")
        .attr("data-size", "16x16@40,0")
        .text("b")
        .close()
        // Zwei 16-px-Ziele, 4 px Lücke (20 px Mittenabstand): zu eng.
        .open("button")
        .attr("data-size", "16x16@0,100")
        .text("c")
        .close()
        .open("button")
        .attr("data-size", "16x16@20,100")
        .text("d")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 2);
}

#[test]
fn checkbox_im_label_hat_das_label_als_ziel() {
    let arena = seite()
        .open("label")
        .open("input")
        .attr("type", "checkbox")
        .attr("data-size", "16x16@0,0")
        .close()
        .text("Pause motion")
        .close()
        .open("button")
        .attr("data-size", "44x44@20,0")
        .text("Daneben")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 0);
}

#[test]
fn animiertes_svg_ist_eine_bewegung() {
    let arena = seite()
        .open("svg")
        .open("circle")
        .attr("data-endlos", "")
        .close()
        .open("circle")
        .attr("data-endlos", "")
        .close()
        .open("rect")
        .attr("data-endlos", "")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        1
    );
}

#[test]
fn mit_pause_schalter_keine_bewegungsmeldung() {
    let arena = seite()
        .open("span")
        .attr("data-endlos", "")
        .text("*")
        .close()
        .open("label")
        .open("input")
        .attr("type", "checkbox")
        .close()
        .text("Pause motion")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        0
    );
}
