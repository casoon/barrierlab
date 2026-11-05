//! Die Kaskade über kleine Seiten: Spezifität, Reihenfolge, Wichtigkeit,
//! Vererbung, `var()`, Medien, Ebenen und der effektive Hintergrund.

use a11y_dom::{Arena, Color, Node, elements};
use style_cascade::{Cascade, Environment, Specified};
use stylesheet_parse::parse_stylesheet;

fn seite(inneres: &str) -> Arena {
    // Ein Mini-Parser reicht nicht für beliebiges HTML: der Aufbau steht hier
    // von Hand, je Test eine Kette `<body><p id=...>`.
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html><body>{inneres}</body></html>"
    ))
    .document;
    fn kopiere(
        d: &html5_parser::Document,
        id: html5_parser::NodeId,
        b: a11y_dom::ArenaBuilder,
    ) -> a11y_dom::ArenaBuilder {
        use html5_parser::NodeKind as H;
        match &d.node(id).kind {
            H::Element {
                name, attributes, ..
            } => {
                let mut b = b.open(name);
                for a in attributes {
                    b = b.attr(&a.name, &a.value);
                }
                for k in d.children(id) {
                    b = kopiere(d, k, b);
                }
                b.close()
            }
            H::DocumentFragment => d.children(id).fold(b, |b, k| kopiere(d, k, b)),
            H::Text { content } => b.text(content),
            _ => b,
        }
    }
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, html5_parser::NodeKind::Element { .. }))
        .unwrap();
    kopiere(&doc, wurzel, Arena::builder()).build()
}

fn kaskade(css: &str) -> Cascade {
    Cascade::new(&[parse_stylesheet(css)], Environment::default())
}

fn finde<'a>(d: &'a Arena, id: &str) -> a11y_dom::ArenaNode<'a> {
    elements(d).find(|n| n.attr("id") == Some(id)).unwrap()
}

fn farbe(c: &Cascade, d: &Arena, id: &str) -> Option<Color> {
    c.color(finde(d, id))
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 255 }
}

#[test]
fn spezifitaet_schlaegt_reihenfolge() {
    let d = seite(r#"<p id="a" class="x">t</p>"#);
    let c = kaskade("#a { color: #010101 } .x { color: #020202 } p { color: #030303 }");
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(1, 1, 1)));
    let c = kaskade(".x { color: #020202 } p { color: #030303 }");
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(2, 2, 2)));
}

#[test]
fn gleiche_spezifitaet_spaeter_gewinnt() {
    let d = seite(r#"<p id="a" class="x y">t</p>"#);
    let c = kaskade(".x { color: #010101 } .y { color: #020202 }");
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(2, 2, 2)));
}

#[test]
fn wichtig_und_stilattribut() {
    let d =
        seite(r#"<p id="a" style="color: #020202">t</p><p id="b" style="color: #020202">t</p>"#);
    let c = kaskade("p { color: #010101 } #b { color: #030303 !important }");
    // Das Stilattribut schlägt eine Regel ohne !important …
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(2, 2, 2)));
    // … aber nicht eine mit.
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(3, 3, 3)));
}

#[test]
fn vererbung_und_ua_link() {
    let d = seite(
        r##"<div id="a" style="color: #333"><span id="b">x</span><a id="l" href="#">l</a></div>"##,
    );
    let c = kaskade("");
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(0x33, 0x33, 0x33)));
    // Ein Link behält die UA-Farbe, auch wenn der Elternteil eine setzt.
    assert_eq!(farbe(&c, &d, "l"), Some(rgb(0, 0, 0xee)));
    let c = kaskade("a { color: inherit }");
    assert_eq!(farbe(&c, &d, "l"), Some(rgb(0x33, 0x33, 0x33)));
}

#[test]
fn schriftgroesse_relativ_und_vererbt() {
    let d = seite(
        r#"<div id="a" style="font-size: 20px"><p id="b" style="font-size: 1.5em"><span id="c">x</span></p></div>"#,
    );
    let c = kaskade("");
    assert_eq!(c.font_size_px(finde(&d, "b")), Some(30.0));
    // Der Enkel erbt 30 px, nicht noch einmal 1,5 em.
    assert_eq!(c.font_size_px(finde(&d, "c")), Some(30.0));
    let d = seite(r#"<h1 id="h">x</h1>"#);
    assert_eq!(kaskade("").font_size_px(finde(&d, "h")), Some(32.0));
}

#[test]
fn var_wird_aufgeloest() {
    let d = seite(r#"<div id="a"><p id="b">t</p></div>"#);
    let c = kaskade(
        ":root { --text: #101010 } #a { --akzent: #202020 } p { color: var(--text) } #a { background: var(--akzent, #fff) }",
    );
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(0x10, 0x10, 0x10)));
    // Ersatzwert, wenn die Variable fehlt.
    let c = kaskade("p { color: var(--fehlt, #303030) }");
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(0x30, 0x30, 0x30)));
    // Fehlt sie ganz, gilt die Vererbung.
    let c = kaskade("#a { color: #404040 } p { color: var(--fehlt) }");
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(0x40, 0x40, 0x40)));
}

#[test]
fn var_zyklus_ist_unbestimmt() {
    let d = seite(r#"<p id="b">t</p>"#);
    let c = kaskade("p { --a: var(--b); --b: var(--a); color: var(--a) }");
    assert_eq!(farbe(&c, &d, "b"), None);
}

#[test]
fn medien() {
    let d = seite(r#"<p id="b">t</p>"#);
    let css = "p { color: #010101 } @media (max-width: 600px) { p { color: #020202 } } @media print { p { color: #030303 } }";
    assert_eq!(farbe(&kaskade(css), &d, "b"), Some(rgb(1, 1, 1)));
    let schmal = Environment {
        width: 500.0,
        ..Default::default()
    };
    let c = Cascade::new(&[parse_stylesheet(css)], schmal);
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(2, 2, 2)));
}

#[test]
fn unbekannte_bedingung_macht_unbestimmt() {
    let d = seite(r#"<p id="b">t</p>"#);
    let c = kaskade("p { color: #010101 } @media (resolution: 2dppx) { p { color: #020202 } }");
    assert_eq!(farbe(&c, &d, "b"), None);
    // Gewinnt die sichere Regel ohnehin, bleibt der Wert bestimmt.
    let c = kaskade(
        "@media (resolution: 2dppx) { p { color: #020202 } } p { color: #010101 !important }",
    );
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(1, 1, 1)));
}

#[test]
fn unbekannter_selektor_macht_unbestimmt() {
    let d = seite(r#"<p id="b">t</p>"#);
    let c = kaskade("p { color: #010101 } p:has(b) { color: #020202 }");
    assert_eq!(farbe(&c, &d, "b"), None);
}

#[test]
fn formularzustaende_entscheiden_die_farbe() {
    // Bootstrap: `.btn:not(:disabled):not(.disabled)`
    let d = seite(
        r#"<button id="a" class="btn">t</button><button id="b" class="btn" disabled>t</button>"#,
    );
    let c =
        kaskade(".btn { color: #010101 } .btn:not(:disabled):not(.disabled) { color: #020202 }");
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(2, 2, 2)));
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(1, 1, 1)));
}

#[test]
fn nth_child_entscheidet_die_farbe() {
    let d = seite(r#"<ul><li id="a">1</li><li id="b">2</li><li id="c">3</li></ul>"#);
    let c = kaskade("li { color: #010101 } li:nth-child(odd) { color: #020202 }");
    assert_eq!(farbe(&c, &d, "a"), Some(rgb(2, 2, 2)));
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(1, 1, 1)));
    assert_eq!(farbe(&c, &d, "c"), Some(rgb(2, 2, 2)));
}

#[test]
fn hover_ohne_umgebung_ist_unbestimmt() {
    let d = seite(r#"<p id="b">t</p>"#);
    let css = "p { color: #010101 } @media (hover: hover) { p { color: #020202 } }";
    assert_eq!(farbe(&kaskade(css), &d, "b"), None);
    let maus = Environment {
        hover: Some(true),
        ..Default::default()
    };
    let c = Cascade::new(&[parse_stylesheet(css)], maus);
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(2, 2, 2)));
}

#[test]
fn zustaende_gelten_im_ruhezustand_nicht() {
    let d = seite(r#"<p id="b" class="k">t</p>"#);
    let c =
        kaskade(".k { color: #010101 } .k:hover { color: #020202 } .k::before { color: #030303 }");
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(1, 1, 1)));
}

#[test]
fn ebenen() {
    let d = seite(r#"<p id="b">t</p>"#);
    // Ohne Ebene schlägt jede Ebene, auch bei geringerer Spezifität.
    let c = kaskade("@layer base { #b { color: #010101 } } p { color: #020202 }");
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(2, 2, 2)));
    // Spätere Ebene schlägt frühere.
    let c = kaskade(
        "@layer a, b; @layer b { p { color: #020202 } } @layer a { #b { color: #010101 } }",
    );
    assert_eq!(farbe(&c, &d, "b"), Some(rgb(2, 2, 2)));
}

#[test]
fn kurzschreibweise_hintergrund() {
    let d = seite(r#"<div id="a"><p id="b">t</p></div>"#);
    let c = kaskade("#a { background: #fff url(x.png) }");
    assert_eq!(c.effective_background(finde(&d, "b")), None);
    let c = kaskade("#a { background: #102030 }");
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(0x10, 0x20, 0x30))
    );
}

#[test]
fn hintergrund_ueberlagert_und_standard_weiss() {
    let d = seite(r#"<div id="a"><p id="b">t</p></div>"#);
    let c = kaskade("");
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(255, 255, 255))
    );
    let c = kaskade("#a { background-color: rgba(0,0,0,0.5) }");
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(127, 127, 127))
    );
}

#[test]
fn deckkraft_und_position_machen_unbestimmt() {
    let d = seite(
        r#"<div id="a" style="opacity: .5"><p id="b">t</p></div><div id="c" style="position: absolute"><p id="e">t</p></div>"#,
    );
    let c = kaskade("");
    assert_eq!(c.effective_background(finde(&d, "b")), None);
    assert_eq!(c.computed_style(finde(&d, "b")).color, None);
    assert_eq!(c.effective_background(finde(&d, "e")), None);
}

#[test]
fn berechneter_stil_fuer_kontrast() {
    let d = seite(
        r#"<p id="b" style="color: #777; background: #fff; font-weight: bold; font-size: 14px">t</p>"#,
    );
    let s = kaskade("").computed_style(finde(&d, "b"));
    assert_eq!(s.color, Some(rgb(0x77, 0x77, 0x77)));
    assert_eq!(s.background_color, Some(rgb(255, 255, 255)));
    assert_eq!(s.font_size_px, Some(14.0));
    assert_eq!(s.font_weight, Some(700));
    assert_eq!(s.display.as_deref(), Some("block"));
}

#[test]
fn specified_unterscheidet_unset_und_unknown() {
    let d = seite(r#"<p id="b">t</p>"#);
    let c = kaskade("");
    assert_eq!(
        c.specified(finde(&d, "b"), "text-transform"),
        Specified::Unset
    );
}

#[test]
fn kurzschreibweise_mit_var() {
    let d = seite(r#"<div id="a"><p id="b">t</p></div>"#);
    let c = kaskade(":root { --bg: #102030; --bild: url(x.png) } #a { background: var(--bg) }");
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(0x10, 0x20, 0x30))
    );
    // Steckt ein Bild in der Variable, ist der Hintergrund unbestimmt.
    let c = kaskade(":root { --bild: url(x.png) } #a { background: #fff var(--bild) }");
    assert_eq!(c.effective_background(finde(&d, "b")), None);
}

#[test]
fn verlauf_gibt_proben() {
    let d = seite(r#"<div id="a"><p id="b">t</p></div>"#);
    let c = kaskade("#a { background: linear-gradient(160deg, #fff, #000 55%, #888) }");
    assert_eq!(c.effective_background(finde(&d, "b")), None);
    let l = c.backdrop_luminances(finde(&d, "b")).expect("Proben");
    // Drei Stufen und zwei Mitten.
    assert_eq!(l.len(), 5);
    assert!(l.iter().any(|v| *v > 0.99) && l.iter().any(|v| *v < 0.01));
    // Ein Bild ist nicht abtastbar.
    let c = kaskade("#a { background: url(x.png), linear-gradient(#fff, #000) }");
    assert_eq!(c.backdrop_luminances(finde(&d, "b")), None);
    // Eine feststehende Farbe braucht keine Proben.
    let c = kaskade("#a { background: #fff }");
    assert_eq!(c.backdrop_luminances(finde(&d, "b")), None);
}

#[test]
fn verlauf_mit_halbdurchsichtiger_flaeche() {
    let d = seite(r#"<div id="a"><div id="k"><p id="b">t</p></div></div>"#);
    let c = kaskade(
        "#a { background: linear-gradient(#000, #000) } #k { background: rgba(255,255,255,.5) }",
    );
    let l = c.backdrop_luminances(finde(&d, "b")).expect("Proben");
    // 50 % Weiß über Schwarz: mittleres Grau, nicht Schwarz.
    assert!(l.iter().all(|v| *v > 0.15 && *v < 0.25), "{l:?}");
}

#[test]
fn position_zaehlt_nur_bis_zur_deckenden_flaeche() {
    let d = seite(
        r#"<div id="a" style="position: fixed"><div id="k" style="background: #123456"><p id="b">t</p></div></div><div id="c" style="position: fixed"><p id="e">t</p></div>"#,
    );
    let c = kaskade("");
    // Die deckende Fläche steht vor dem fixierten Vorfahren.
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(0x12, 0x34, 0x56))
    );
    // Ohne eigene Fläche ist der Hintergrund hinter dem fixierten Kasten unbekannt.
    assert_eq!(c.effective_background(finde(&d, "e")), None);
}

#[test]
fn darstellungsattribute() {
    let d = seite(
        r##"<table><tr><td id="a" bgcolor="#ff6600"><font id="f" color="red">x</font></td></tr></table><table><tr><td id="b" bgcolor="#ff6600">y</td></tr></table>"##,
    );
    let c = kaskade("");
    assert_eq!(
        c.effective_background(finde(&d, "a")),
        Some(rgb(255, 102, 0))
    );
    assert_eq!(farbe(&c, &d, "f"), Some(rgb(255, 0, 0)));
    // Eine Regel der Seite schlägt das Attribut.
    let c = kaskade("#b { background-color: #112233 }");
    assert_eq!(
        c.effective_background(finde(&d, "b")),
        Some(rgb(0x11, 0x22, 0x33))
    );
}

#[test]
fn darstellungsattribut_verliert_gegen_ebene() {
    let d = seite(r##"<table id="b" bgcolor="#ff0000"><tr><td>t</td></tr></table>"##);
    let c = kaskade("@layer base { table { background-color: #00ff00 } }");
    assert_eq!(
        c.specified(finde(&d, "b"), "background-color"),
        Specified::Set("#00ff00".into())
    );
}

#[test]
fn style_attribut_wichtig_schlaegt_wichtig_in_ebene() {
    let d = seite(r#"<p id="p" style="color:#010101 !important">t</p>"#);
    let c = kaskade("@layer a { #p { color: #020202 !important } }");
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(1, 1, 1)));
}

#[test]
fn rangfolge_normal_bleibt() {
    let d = seite(r#"<p id="p" style="color:#010101">t</p>"#);
    let c = kaskade("#p { color: #020202 }");
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(1, 1, 1)));
    let d = seite(r#"<p id="p">t</p>"#);
    let c = kaskade("@layer a { #p { color: #010101 } } p { color: #020202 }");
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(2, 2, 2)));
}

#[test]
fn unaufgeloestes_import_macht_unbekannt() {
    let css = "@import url(theme.css); p { color: #010101 }";
    let d = seite(r#"<p id="p">t</p>"#);
    let c = kaskade(css);
    assert_eq!(c.unresolved_imports(), ["url(theme.css)"]);
    assert_eq!(c.specified(finde(&d, "p"), "color"), Specified::Unknown);
    // Nur ein `!important` im `style`-Attribut kann der Import nicht überstimmen.
    let d = seite(r#"<p id="p" style="color:#030303 !important">t</p>"#);
    let c = kaskade(css);
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(3, 3, 3)));
}

#[test]
fn ohne_import_unveraendert() {
    let d = seite(r#"<p id="p">t</p>"#);
    let c = kaskade("p { color: #010101 }");
    assert!(c.unresolved_imports().is_empty());
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(1, 1, 1)));
}

#[test]
fn aufgeloester_import_gibt_klare_antwort() {
    // Ein Host, der `@import` auflöst, reicht die importierten Regeln
    // stattdessen mit (hier einfach vorangestellt) und bekommt eine Antwort.
    let d = seite(r#"<p id="p">t</p>"#);
    let c = kaskade("p { color: #020202 } p { color: #010101 }");
    assert!(c.unresolved_imports().is_empty());
    assert_eq!(farbe(&c, &d, "p"), Some(rgb(1, 1, 1)));
}
