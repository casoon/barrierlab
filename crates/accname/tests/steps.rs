//! Schritt für Schritt: je ein Fall für die Entscheidungen der
//! Namensberechnung, die der WPT-Korpus (`wpt.rs`) und die Unit-Tests nicht
//! allein festnageln — von `tests/mutations/*.toml` als Töter genannt
//! (`xtask/mutate.py crates/accname`).
//!
//! Die Fälle sind HTML-Schnipsel; das Element mit `id="t"` wird benannt.
//! `name_rendered` bekommt `display` und `visibility` aus dem `style`-Attribut,
//! ohne Angabe ist ein Element `inline`.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, ComputedStyle, Document, Node, Rect, Rendering, elements,
};
use accname::{IdIndex, description, name, name_rendered, role};
use html5_parser::NodeKind as H;

struct Host<'d>(&'d Arena);

impl Document for Host<'_> {
    type N<'a>
        = ArenaNode<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

fn inline_style(n: ArenaNode<'_>, prop: &str) -> Option<String> {
    n.attr("style")?.split(';').find_map(|d| {
        let (k, v) = d.split_once(':')?;
        (k.trim() == prop).then(|| v.trim().to_string())
    })
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            display: Some(inline_style(node, "display").unwrap_or_else(|| "inline".into())),
            visibility: Some(inline_style(node, "visibility").unwrap_or_else(|| "visible".into())),
            ..Default::default()
        })
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }
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
        _ => b,
    }
}

fn parse(html: &str) -> Arena {
    let geparst = html5_parser::parse(html).document;
    let wurzel = geparst
        .children(geparst.root())
        .find(|&c| matches!(geparst.node(c).kind, H::Element { .. }))
        .expect("Dokument ohne Wurzelelement");
    uebernehmen(&geparst, wurzel, Arena::builder()).build()
}

fn ziel<'a>(doc: &'a Arena) -> ArenaNode<'a> {
    elements(doc)
        .find(|n| n.attr("id") == Some("t"))
        .expect("Element mit id=t fehlt")
}

/// Name ohne Stil (`name`).
fn n(html: &str) -> Option<String> {
    let doc = parse(html);
    let ids = IdIndex::build(doc.root());
    name(ziel(&doc), &ids)
}

/// Name mit Stil (`name_rendered`).
fn nr(html: &str) -> Option<String> {
    let doc = parse(html);
    let host = Host(&doc);
    let ids = IdIndex::build(doc.root());
    name_rendered(&host, ziel(&doc), &ids)
}

fn beschreibung(html: &str) -> Option<String> {
    let doc = parse(html);
    let ids = IdIndex::build(doc.root());
    description(ziel(&doc), &ids)
}

fn rolle(html: &str) -> Option<&'static str> {
    let doc = parse(html);
    role(ziel(&doc))
}

// --- 2A: versteckt ------------------------------------------------------

#[test]
fn knoten_unter_verstecktem_vorfahren_hat_keinen_namen_ohne_stil() {
    assert_eq!(
        n(r#"<div aria-hidden="true"><button id="t">X</button></div>"#),
        None
    );
    assert_eq!(n(r#"<div hidden><button id="t">X</button></div>"#), None);
}

#[test]
fn verweisziel_unter_verstecktem_vorfahren_ist_selbst_nicht_versteckt() {
    // Der Vorfahr zählt nur an der Wurzel der Berechnung, nicht für ein
    // Verweisziel: dessen eigener versteckter Teilbaum bleibt deshalb weg.
    // Festgehaltenes Ist-Verhalten, kein Normtext.
    let html = r#"<div style="display:none"><span id="l">a<span style="display:none">b</span></span></div>
        <button id="t" aria-labelledby="l"></button>"#;
    assert_eq!(nr(html).as_deref(), Some("a"));
}

// --- 2B/2C: Verweise und aria-label in der Beschreibung -----------------------

#[test]
fn labelledby_gilt_nicht_im_verweisdurchlauf() {
    let html = r#"<span id="a" aria-labelledby="b">A</span><span id="b">B</span>
        <button id="t" aria-labelledby="a"></button>"#;
    assert_eq!(n(html).as_deref(), Some("A"));
}

#[test]
fn beschreibung_folgt_weder_labelledby_noch_aria_label_im_ziel() {
    // Festgehaltenes Ist-Verhalten (siehe `Ziel` in name.rs).
    let html = r#"<span id="x">other</span>
        <div id="d"><span aria-labelledby="x">child</span></div>
        <button id="t" aria-describedby="d"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("child"));
    let html = r#"<div id="d"><button aria-label="L">child</button></div>
        <button id="t" aria-describedby="d"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("child"));
}

#[test]
fn rolle_mit_namensverbot_bekommt_auch_keinen_nativen_namen() {
    assert_eq!(n(r#"<img id="t" role="presentation" alt="Foto">"#), None);
}

#[test]
fn namensverbot_gilt_fuer_jede_verbotene_rolle() {
    for r in [
        "caption",
        "code",
        "deletion",
        "emphasis",
        "generic",
        "insertion",
        "paragraph",
        "presentation",
        "none",
        "strong",
        "subscript",
        "superscript",
        "term",
        "time",
    ] {
        assert_eq!(
            n(&format!(r#"<img id="t" role="{r}" alt="A">"#)),
            None,
            "{r} (alt)"
        );
        if r != "none" && r != "presentation" {
            // Mit aria-label hebt die präsentationale Rolle sich selbst auf.
            assert_eq!(
                n(&format!(r#"<div id="t" role="{r}" aria-label="L"></div>"#)),
                None,
                "{r} (aria-label)"
            );
        }
    }
}

#[test]
fn namen_aus_inhalt_fuer_jede_rolle_der_liste() {
    for r in [
        "button",
        "cell",
        "checkbox",
        "columnheader",
        "gridcell",
        "heading",
        "link",
        "menuitem",
        "menuitemcheckbox",
        "menuitemradio",
        "option",
        "radio",
        "row",
        "rowheader",
        "switch",
        "tab",
        "tooltip",
        "treeitem",
    ] {
        assert_eq!(
            n(&format!(r#"<div id="t" role="{r}">Text</div>"#)).as_deref(),
            Some("Text"),
            "{r}"
        );
    }
}

// --- 2D: native Textalternativen ----------------------------------------------

#[test]
fn leere_native_alternative_faellt_auf_title_durch() {
    assert_eq!(
        n(r#"<input id="t" placeholder="" title="T">"#).as_deref(),
        Some("T")
    );
}

#[test]
fn area_nutzt_alt() {
    assert_eq!(
        n(r#"<map><area id="t" href="/" alt="Alt"></map>"#).as_deref(),
        Some("Alt")
    );
}

#[test]
fn input_typ_wird_getrimmt_und_kleingeschrieben() {
    assert_eq!(
        n(r#"<input id="t" type=" submit ">"#).as_deref(),
        Some("Submit")
    );
    assert_eq!(
        n(r#"<input id="t" type="SUBMIT">"#).as_deref(),
        Some("Submit")
    );
    assert_eq!(
        n(r#"<input id="t" type="reset">"#).as_deref(),
        Some("Reset")
    );
}

#[test]
fn bildschaltflaeche_nimmt_label_vor_dem_vorgabenamen() {
    assert_eq!(
        n(r#"<input id="t" type="image"><label for="t">Lbl</label>"#).as_deref(),
        Some("Lbl")
    );
    assert_eq!(
        n(r#"<input id="t" type="image">"#).as_deref(),
        Some("Submit Query")
    );
}

#[test]
fn labels_fuer_textarea_meter_progress_output() {
    for tag in ["textarea", "meter", "progress", "output"] {
        assert_eq!(
            n(&format!(
                r#"<{tag} id="t"></{tag}><label for="t">L</label>"#
            ))
            .as_deref(),
            Some("L"),
            "{tag}"
        );
    }
}

#[test]
fn figure_nutzt_figcaption() {
    assert_eq!(
        n(r#"<figure id="t"><figcaption>Cap</figcaption>x</figure>"#).as_deref(),
        Some("Cap")
    );
}

#[test]
fn legende_mit_block_kind_wird_im_stil_abgesetzt() {
    let html =
        r#"<fieldset id="t"><legend>a<span style="display:block">b</span></legend></fieldset>"#;
    assert_eq!(nr(html).as_deref(), Some("a b"));
}

#[test]
fn verstecktes_label_ueber_versteckten_vorfahren_benennt_nicht() {
    let html = r#"<div style="display:none"><label for="t">L</label></div><input id="t">"#;
    assert_eq!(nr(html), None);
}

#[test]
fn umschliessendes_label_nur_ohne_for_label() {
    let html = r#"<label for="t">For</label><label>Enc <input id="t"></label>"#;
    assert_eq!(n(html).as_deref(), Some("For"));
}

#[test]
fn verstecktes_kind_im_label_zaehlt_nicht() {
    let html = r#"<label for="t">a<span style="display:none">b</span></label><input id="t">"#;
    assert_eq!(nr(html).as_deref(), Some("a"));
}

#[test]
fn zeilenumbruch_im_label_trennt() {
    let html = r#"<label for="t">a<br>b</label><input id="t">"#;
    assert_eq!(nr(html).as_deref(), Some("a b"));
}

// --- 2E: eingebettete Steuerelemente --------------------------------------------

const EINGEBETTET: [&str; 11] = [
    "textbox",
    "searchbox",
    "combobox",
    "listbox",
    "slider",
    "spinbutton",
    "progressbar",
    "scrollbar",
    "checkbox",
    "radio",
    "switch",
];

#[test]
fn eingebettetes_steuerelement_gibt_sein_aria_label_nicht_her() {
    for r in EINGEBETTET {
        let html =
            format!(r#"<button id="t"><span role="{r}" aria-label="L">inner</span></button>"#);
        assert_eq!(n(&html).as_deref(), Some("inner"), "{r}");
    }
}

#[test]
fn eingebettetes_textfeld_nimmt_auch_versteckten_text() {
    // Der Wert ist der Teilbaumtext, ohne die Sichtbarkeitsregel von 2F.
    for r in ["textbox", "searchbox"] {
        let html =
            format!(r#"<button id="t"><span role="{r}">a<span hidden>b</span></span></button>"#);
        assert_eq!(n(&html).as_deref(), Some("ab"), "{r}");
    }
}

#[test]
fn eingebettete_auswahl_nimmt_gewaehlte_oder_erste_option() {
    let html = r#"<button id="t"><select><option>one</option><option selected>two</option></select></button>"#;
    assert_eq!(n(html).as_deref(), Some("two"));
    let html =
        r#"<button id="t"><select><option>one</option><option>two</option></select></button>"#;
    assert_eq!(n(html).as_deref(), Some("one"));
    let html = r#"<button id="t"><select multiple><option>one</option><option selected>two</option></select></button>"#;
    assert_eq!(n(html).as_deref(), Some("two"));
    let html = r#"<button id="t"><select multiple><option>one</option><option>two</option></select></button>"#;
    assert_eq!(n(html).as_deref(), Some("one"));
}

#[test]
fn eingebetteter_bereichswert_hat_eine_rangfolge() {
    for r in ["slider", "spinbutton", "progressbar", "scrollbar"] {
        let html = format!(
            r#"<button id="t"><span role="{r}" aria-valuetext="VT" aria-valuenow="5" value="V">x</span></button>"#
        );
        assert_eq!(n(&html).as_deref(), Some("VT"), "{r} valuetext");
        let html = format!(
            r#"<button id="t"><span role="{r}" aria-valuenow="5" value="V">x</span></button>"#
        );
        assert_eq!(n(&html).as_deref(), Some("5"), "{r} valuenow");
        let html = format!(r#"<button id="t"><span role="{r}" value="V">x</span></button>"#);
        assert_eq!(n(&html).as_deref(), Some("V"), "{r} value");
    }
}

#[test]
fn wert_eines_steuerelements_zaehlt_nur_in_der_rekursion() {
    let html = r#"<div id="t" role="slider" aria-valuenow="5" title="T"></div>"#;
    assert_eq!(n(html).as_deref(), Some("T"));
}

// --- Trenner (2F), mit Stil ---------------------------------------------------

#[test]
fn display_contents_schliesst_direkt_an() {
    let html = r#"<a id="t" href="/">a<span style="display:contents">b</span>c</a>"#;
    assert_eq!(nr(html).as_deref(), Some("abc"));
}

#[test]
fn ersetzte_elemente_werden_abgesetzt() {
    for el in [
        r#"<svg title="T"></svg>"#,
        r#"<canvas title="T"></canvas>"#,
        r#"<video title="T"></video>"#,
        r#"<audio title="T"></audio>"#,
        r#"<iframe title="T"></iframe>"#,
        r#"<object title="T"></object>"#,
        r#"<embed title="T">"#,
        r#"<img title="T">"#,
    ] {
        let html = format!(r#"<a id="t" href="/">x{el}y</a>"#);
        assert_eq!(nr(&html).as_deref(), Some("x T y"), "{el}");
    }
}

#[test]
fn leeres_blockkind_trennt_nicht() {
    let html = r#"<a id="t" href="/">a<span style="display:block"></span>b</a>"#;
    assert_eq!(nr(html).as_deref(), Some("ab"));
}

#[test]
fn blockkind_trennt_nach_beiden_seiten() {
    let html = r#"<a id="t" href="/">a<span style="display:block">b</span>c</a>"#;
    assert_eq!(nr(html).as_deref(), Some("a b c"));
}

// --- Beschreibung ---------------------------------------------------------

#[test]
fn beschreibung_setzt_verweise_mit_leerzeichen_zusammen() {
    let html = r#"<span id="a">A</span><span id="b">B</span><button id="t" aria-describedby="a b"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("A B"));
    // Ohne Auffüllung aus dem Inhaltsdurchlauf: native Alternativen.
    let html = r#"<img id="a" alt="A"><img id="b" alt="B"><button id="t" aria-describedby="a b"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("A B"));
}

#[test]
fn beschreibung_zieht_auch_versteckte_ziele_heran() {
    let html = r#"<span id="d" hidden>H</span><button id="t" aria-describedby="d"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("H"));
}

#[test]
fn beschreibung_auf_sich_selbst_ist_leer() {
    let html = r#"<button id="t" aria-describedby="t">Btn</button>"#;
    assert_eq!(beschreibung(html), None);
}

#[test]
fn leere_beschreibung_faellt_auf_title_durch() {
    let html = r#"<span id="e"></span><button id="t" aria-describedby="e" aria-label="x" title="T"></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("T"));
}

#[test]
fn beschreibung_title_wird_getrimmt_und_blank_zaehlt_nicht() {
    let html = r#"<button id="t" aria-label="x" title="  T  "></button>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("T"));
    let html = r#"<button id="t" aria-label="x" title="  "></button>"#;
    assert_eq!(beschreibung(html), None);
}

#[test]
fn title_ist_beschreibung_wenn_labelledby_oder_label_den_namen_stellt() {
    let html = r#"<button id="t" aria-labelledby="l" title="T"></button><span id="l">L</span>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("T"));
    let html = r#"<input id="t" title="T"><label for="t">L</label>"#;
    assert_eq!(beschreibung(html).as_deref(), Some("T"));
}

// --- Rollen -----------------------------------------------------------------

#[test]
fn globales_aria_hebt_praesentationale_rolle_auf() {
    assert_eq!(rolle(r#"<h1 id="t" role="none">x</h1>"#), Some("none"));
    assert_eq!(
        rolle(r#"<h1 id="t" role="presentation">x</h1>"#),
        Some("presentation")
    );
    assert_eq!(
        rolle(r#"<h1 id="t" role="none" aria-busy="true">x</h1>"#),
        Some("heading")
    );
    assert_eq!(
        rolle(r#"<h1 id="t" role="presentation" aria-live="polite">x</h1>"#),
        Some("heading")
    );
    // Schon das Vorhandensein zählt, auch mit leerem Wert.
    assert_eq!(
        rolle(r#"<h1 id="t" role="none" aria-label="">x</h1>"#),
        Some("heading")
    );
}

#[test]
fn fokussierbares_element_hat_keine_praesentationale_rolle() {
    assert_eq!(
        rolle(r#"<button id="t" role="none">x</button>"#),
        Some("button")
    );
    assert_eq!(
        rolle(r#"<button id="t" role="none" disabled>x</button>"#),
        Some("none")
    );
    assert_eq!(
        rolle(r#"<fieldset disabled><button id="t" role="none">x</button></fieldset>"#),
        Some("none")
    );
    assert_eq!(
        rolle(r#"<input id="t" type="hidden" role="none">"#),
        Some("none")
    );
    assert_eq!(
        rolle(r#"<a id="t" href="/" role="none">x</a>"#),
        Some("link")
    );
    assert_eq!(rolle(r#"<a id="t" role="none">x</a>"#), Some("none"));
    assert_eq!(
        rolle(r#"<div id="t" role="none" tabindex="0">x</div>"#),
        Some("generic")
    );
    assert_eq!(
        rolle(r#"<div id="t" role="none" tabindex="x">x</div>"#),
        Some("none")
    );
    assert_eq!(
        rolle(r#"<div id="t" role="none" contenteditable>x</div>"#),
        Some("generic")
    );
    assert_eq!(
        rolle(r#"<div id="t" role="none" contenteditable="false">x</div>"#),
        Some("none")
    );
}

#[test]
fn implizite_rollen_nach_html_aam() {
    assert_eq!(
        rolle(r#"<section id="t" aria-labelledby="x">x</section>"#),
        Some("region")
    );
    assert_eq!(
        rolle(r#"<select id="t" multiple></select>"#),
        Some("listbox")
    );
    assert_eq!(
        rolle(r#"<select id="t" size="3"></select>"#),
        Some("listbox")
    );
    assert_eq!(
        rolle(r#"<select id="t" size="1"></select>"#),
        Some("combobox")
    );
    assert_eq!(
        rolle(r#"<table><tr><th id="t" scope="row">x</th></tr></table>"#),
        Some("rowheader")
    );
    assert_eq!(
        rolle(r#"<table><tr><th id="t" scope="rowgroup">x</th></tr></table>"#),
        Some("rowheader")
    );
    assert_eq!(rolle(r#"<input id="t" list="x">"#), Some("combobox"));
    assert_eq!(
        rolle(r#"<input id="t" type="search" list="x">"#),
        Some("combobox")
    );
}
