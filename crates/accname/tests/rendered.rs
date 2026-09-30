//! `name_rendered`: Trenner und Verstecktheit aus dem berechneten Stil.
//!
//! Der Test-Host liest `display` und `visibility` aus `data-display` /
//! `data-visibility`; ohne Angabe ist ein Element `inline`. `data-kein-stil`
//! simuliert ein Element, für das der Host keinen Stil kennt.

use a11y_dom::{Arena, ArenaNode, ComputedStyle, Document, Node, Rect, Rendering, elements};
use accname::{IdIndex, name, name_rendered};

struct Gestylt<'d>(&'d Arena);

impl Document for Gestylt<'_> {
    type N<'a>
        = ArenaNode<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

impl Rendering for Gestylt<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        if node.has_attr("data-kein-stil") {
            return None;
        }
        Some(ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: None,
            font_weight: None,
            display: Some(node.attr("data-display").unwrap_or("inline").to_string()),
            visibility: Some(
                node.attr("data-visibility")
                    .unwrap_or("visible")
                    .to_string(),
            ),
        })
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }
}

fn finde<'a>(doc: &'a Arena, tag: &str) -> ArenaNode<'a> {
    elements(doc).find(|n| n.local_name() == tag).unwrap()
}

fn gerendert(doc: &Arena, tag: &str) -> Option<String> {
    let host = Gestylt(doc);
    let ids = IdIndex::build(doc.root());
    name_rendered(&host, finde(doc, tag), &ids)
}

#[test]
fn inline_elemente_schliessen_direkt_an() {
    // Korpusfälle aus auditmysite; Chrome: „EU-Arktis", „Rechenpower".
    let abbr = Arena::builder()
        .open("a")
        .attr("href", "/")
        .open("abbr")
        .text("EU")
        .close()
        .text("-Arktis")
        .close()
        .build();
    assert_eq!(gerendert(&abbr, "a").as_deref(), Some("EU-Arktis"));

    let span = Arena::builder()
        .open("a")
        .attr("href", "/")
        .text("Rechen")
        .open("span")
        .text("power")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&span, "a").as_deref(), Some("Rechenpower"));
}

#[test]
fn block_spans_bleiben_getrennt() {
    // Der Fall, an dem die Tag-Regel aus 0.11.1 scheiterte.
    let doc = Arena::builder()
        .open("a")
        .attr("href", "/")
        .open("span")
        .attr("data-display", "block")
        .text("Cloud & Hosting")
        .close()
        .open("span")
        .attr("data-display", "block")
        .text("Edge-Hosting")
        .close()
        .close()
        .build();
    assert_eq!(
        gerendert(&doc, "a").as_deref(),
        Some("Cloud & Hosting Edge-Hosting")
    );
}

#[test]
fn inline_block_wird_abgesetzt() {
    // WPT comp_name_from_content: „for each child (display:inline-block)".
    let doc = Arena::builder()
        .open("a")
        .attr("href", "/")
        .open("span")
        .attr("data-display", "inline-block")
        .text("one")
        .close()
        .open("span")
        .attr("data-display", "inline-block")
        .text("two")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "a").as_deref(), Some("one two"));
}

#[test]
fn ersetzte_elemente_bleiben_abgesetzt() {
    let doc = Arena::builder()
        .open("a")
        .attr("href", "/")
        .text("Foto")
        .open("img")
        .attr("alt", "Logo")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "a").as_deref(), Some("Foto Logo"));
}

#[test]
fn ohne_stil_fuer_ein_element_gilt_der_rueckfall() {
    let doc = Arena::builder()
        .open("a")
        .attr("href", "/")
        .text("Rechen")
        .open("span")
        .attr("data-kein-stil", "")
        .text("power")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "a").as_deref(), Some("Rechen power"));
}

#[test]
fn label_mit_inline_stern_bleibt_zusammen() {
    let doc = Arena::builder()
        .open("form")
        .attr("data-display", "block")
        .open("label")
        .attr("for", "f")
        .text("abholen")
        .open("span")
        .text("*")
        .close()
        .close()
        .open("input")
        .attr("type", "checkbox")
        .attr("id", "f")
        .attr("data-display", "inline-block")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "input").as_deref(), Some("abholen*"));
}

#[test]
fn per_css_versteckter_inhalt_zaehlt_nicht() {
    let none = Arena::builder()
        .open("button")
        .text("Senden")
        .open("span")
        .attr("data-display", "none")
        .text(" (Formular)")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&none, "button").as_deref(), Some("Senden"));

    let hidden = Arena::builder()
        .open("button")
        .text("Senden")
        .open("span")
        .attr("data-visibility", "hidden")
        .text(" (Formular)")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&hidden, "button").as_deref(), Some("Senden"));
}

#[test]
fn per_verweis_zaehlt_auch_versteckter_text() {
    // 2A: ausdrücklich per aria-labelledby herangezogen, zählt er trotzdem.
    let doc = Arena::builder()
        .open("div")
        .attr("data-display", "block")
        .open("span")
        .attr("id", "l")
        .attr("data-display", "none")
        .text("Suche")
        .close()
        .open("input")
        .attr("aria-labelledby", "l")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "input").as_deref(), Some("Suche"));
}

#[test]
fn knoten_unter_verstecktem_vorfahren_hat_keinen_namen() {
    let doc = Arena::builder()
        .open("nav")
        .attr("data-display", "none")
        .open("a")
        .attr("href", "/")
        .text("Start")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "a"), None);
    // Ohne Stil bleibt es beim Namen — die Verstecktheit ist nicht erkennbar.
    let ids = IdIndex::build(doc.root());
    assert_eq!(name(finde(&doc, "a"), &ids).as_deref(), Some("Start"));
}

#[test]
fn zeilenumbruch_trennt() {
    let doc = Arena::builder()
        .open("h2")
        .text("Austauschbar.")
        .open("br")
        .close()
        .open("span")
        .text("Oder unverwechselbar.")
        .close()
        .close()
        .build();
    assert_eq!(
        gerendert(&doc, "h2").as_deref(),
        Some("Austauschbar. Oder unverwechselbar.")
    );
}

/// Ein Label mit `display: none` benennt nichts — Chrome gibt dem Feld dann
/// den Namen `""` (per CDP nachgemessen auf barrierlab.eu/experience/, wo ein
/// Schalter die Labels ausblendet).
#[test]
fn nicht_dargestelltes_label_benennt_nicht() {
    let doc = Arena::builder()
        .open("form")
        .open("label")
        .attr("for", "e")
        .attr("data-display", "none")
        .text("E-Mail")
        .close()
        .open("input")
        .attr("id", "e")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "input"), None);

    // Ohne Stil ist die Verstecktheit nicht erkennbar; das Label zählt.
    let ids = IdIndex::build(doc.root());
    assert_eq!(name(finde(&doc, "input"), &ids).as_deref(), Some("E-Mail"));
}

/// Per `aria-labelledby` zählt versteckter Inhalt weiterhin.
#[test]
fn verstecktes_label_zaehlt_ueber_labelledby() {
    let doc = Arena::builder()
        .open("form")
        .open("label")
        .attr("id", "l")
        .attr("data-display", "none")
        .text("E-Mail")
        .close()
        .open("input")
        .attr("aria-labelledby", "l")
        .close()
        .close()
        .build();
    assert_eq!(gerendert(&doc, "input").as_deref(), Some("E-Mail"));
}
