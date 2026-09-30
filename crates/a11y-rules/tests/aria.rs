//! ARIA-Attribute und -Beziehungen (casoon/barrierlab#14).
//!
//! Die Fälle stammen aus dem Detection-Korpus von auditmysite
//! (`tests/fixtures/detection_corpus/`) und aus dessen Issues; der Name jedes
//! Tests nennt die Quelle.

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, Node, Semantics, elements};
use a11y_report::{Outcome, Report};
use a11y_rules::{run, run_with_semantics};
use accname::IdIndex;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

/// Ein HTML-Fragment als Arena, über den echten Parser.
fn html(src: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body><main>{src}</main></body></html>"
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

/// Tier-2-Host: die Arena mit Rolle und Name aus `accname`.
struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    /// Verhält sich wie Chrome: `<li>` in einer Liste mit fremder Rolle wird
    /// `generic`, ein schlichtes `<tbody>` ausgeblendet.
    chrome: bool,
}

impl<'a> MitSemantik<'a> {
    fn new(doc: &'a Arena) -> Self {
        MitSemantik {
            ids: IdIndex::build(doc.root()),
            doc,
            chrome: false,
        }
    }

    fn wie_chrome(doc: &'a Arena) -> Self {
        MitSemantik {
            chrome: true,
            ..Self::new(doc)
        }
    }
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
        if self.chrome && node.is_element("li") && !node.has_attr("role") {
            let liste = node.parent().and_then(|p| p.attr("role"));
            if liste.is_some_and(|r| r != "list") {
                return Some("generic".into());
            }
        }
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
            || (self.chrome && node.is_element("tbody") && !node.has_attr("role"))
    }
}

fn mit_semantik(src: &str) -> Report {
    let a = html(src);
    run_with_semantics(&MitSemantik::new(&a))
}

fn wie_chrome(src: &str) -> Report {
    let a = html(src);
    run_with_semantics(&MitSemantik::wie_chrome(&a))
}

fn anzahl(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

fn outcome(r: &Report, id: &str) -> Outcome {
    r.findings.iter().find(|f| f.rule_id == id).unwrap().outcome
}

/// Die Befunde einer Kennung, verortet an der `id` des Elements.
fn an(src: &str, r: &Report, rule: &str) -> Vec<String> {
    let a = html(src);
    let mut v: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| {
            let knoten = f.location.node.as_deref()?;
            elements(&a)
                .find(|n| n.id().to_string() == knoten)
                .map(|n| n.attr("id").unwrap_or(n.local_name()).to_string())
        })
        .collect();
    v.sort();
    v
}

// --- aria/attribute-unknown ------------------------------------------------

#[test]
fn unbekanntes_aria_attribut_korpus_aria_attribute_validation() {
    let r = run(&html(
        r#"<div aria-nonexistentattr="true">x</div>
           <div aria-labeledby="a">Tippfehler</div>
           <div aria-description="ARIA 1.3">ok</div>
           <button aria-pressed="true">ok</button>"#,
    ));
    assert_eq!(anzahl(&r, "aria/attribute-unknown"), 2);
}

// --- aria/attribute-value-invalid ------------------------------------------

#[test]
fn ungueltiger_wert_korpus_aria_invalid_attr_value() {
    let r = run(&html(
        r#"<button aria-expanded="maybe" aria-controls="panel1">Toggle</button>
           <div id="panel1">Panel</div>"#,
    ));
    assert_eq!(anzahl(&r, "aria/attribute-value-invalid"), 1);
}

#[test]
fn gueltige_und_leere_werte_bleiben_still() {
    let r = run(&html(
        r#"<button aria-expanded="TRUE" aria-haspopup="menu">a</button>
           <div role="checkbox" aria-checked="mixed" tabindex="0">b</div>
           <div role="slider" aria-valuenow="1e2" aria-valuemin="-5" aria-valuemax="200.5" tabindex="0">c</div>
           <div aria-live="polite" aria-relevant="additions text">d</div>
           <h2 aria-level="2">e</h2>
           <button aria-pressed="">f</button>
           <a href="/" aria-current="yes">g</a>
           <input aria-invalid="bogus" aria-label="h">"#,
    ));
    assert_eq!(
        anzahl(&r, "aria/attribute-value-invalid"),
        0,
        "{:?}",
        r.findings
    );
}

#[test]
fn falsche_typen_werden_gemeldet() {
    let r = run(&html(
        r#"<div role="heading" aria-level="2.5">a</div>
           <div role="slider" aria-valuenow="viel" tabindex="0">b</div>
           <div aria-live="loud">c</div>
           <div aria-relevant="additions everything">d</div>
           <div role="checkbox" aria-checked="yes" tabindex="0">e</div>"#,
    ));
    assert_eq!(anzahl(&r, "aria/attribute-value-invalid"), 5);
}

// --- aria/owns-conflict ----------------------------------------------------

#[test]
fn doppelter_besitz_korpus_parsing_duplicate_id_aria() {
    let r = run(&html(
        r#"<div role="combobox" aria-expanded="false" aria-owns="liste">a</div>
           <div role="combobox" aria-expanded="false" aria-owns="liste">b</div>
           <div role="group" aria-owns="x y x">c</div>
           <ul id="liste" role="listbox"><li role="option">o</li></ul>
           <span id="x">x</span><span id="y">y</span>"#,
    ));
    assert_eq!(anzahl(&r, "aria/owns-conflict"), 1);
}

// --- aria/attribute-not-allowed, aria/attribute-prohibited -----------------

#[test]
fn nicht_unterstuetzt_und_verboten_korpus_aria_attribute_validation() {
    let src = r#"<span id="deko" role="presentation" aria-label="Redundant">Deko</span>
                 <div id="icon" role="img" aria-checked="true" aria-label="Icon">Icon</div>
                 <div id="generisch" aria-label="Kein Name erlaubt">x</div>
                 <div id="auf" aria-expanded="false">Akkordeon aus div</div>"#;
    let r = mit_semantik(src);
    assert_eq!(
        an(src, &r, "aria/attribute-prohibited"),
        ["deko", "generisch"]
    );
    assert_eq!(an(src, &r, "aria/attribute-not-allowed"), ["auf", "icon"]);
}

#[test]
fn aria_checked_an_nativer_checkbox_aria_in_html() {
    let src = r#"<input id="c" type="checkbox" aria-checked="true" aria-label="a">
                 <input id="r" type="radio" aria-checked="false" aria-label="b">
                 <input id="n" type="number" min="0" aria-valuemin="0" aria-label="c">
                 <button id="d" disabled aria-disabled="false">d</button>
                 <button id="ok" disabled aria-disabled="true">e</button>"#;
    let r = mit_semantik(src);
    assert_eq!(
        an(src, &r, "aria/attribute-not-allowed"),
        ["c", "d", "n", "r"]
    );
}

#[test]
fn treegrid_zeilen_und_native_eingaben_bleiben_still_auditmysite_655_656_673() {
    let r = mit_semantik(
        r#"<table role="treegrid" aria-label="Orders">
             <thead><tr><th>Country</th><th>Amount</th></tr></thead>
             <tbody>
               <tr aria-level="1" aria-expanded="true" tabindex="0"><td>AT</td><td>300</td></tr>
               <tr aria-level="2"><td>Vienna</td><td>100</td></tr>
             </tbody>
           </table>
           <label for="v">Volume</label><input id="v" type="range" min="0" max="10">
           <input type="date" aria-label="Order date value">
           <div class="track" role="slider" tabindex="0" aria-label="Jahr"
                aria-valuemin="1850" aria-valuemax="2099" aria-valuenow="2025"
                aria-valuetext="2025: +1,43 °C">x</div>
           <table aria-label="Keys"><tbody><tr><th scope="row">Home</th><td>First</td></tr></tbody></table>
           <ul role="tree" aria-label="Files">
             <li role="treeitem" aria-expanded="false" tabindex="0">src</li>
           </ul>"#,
    );
    let aria: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("aria/"))
        .map(|f| (&f.rule_id, &f.message))
        .collect();
    assert!(aria.is_empty(), "{aria:?}");
}

// --- aria/required-parent-missing, aria/required-children-missing ----------

/// auditmysite#715 (magyarorszag.hu): axe meldet `aria-required-children` an
/// der Tabliste und `aria-required-parent` an den Tabs; der `listitem`-Fall
/// kommt dazu.
const TABLISTE_AUS_LI: &str = r##"<ul id="tabs" role="tablist"><li id="li1"><a id="t1" role="tab" href="#a">A</a></li><li id="li2"><a id="t2" role="tab" href="#b">B</a></li></ul>"##;

#[test]
fn tabliste_aus_listeneintraegen_auditmysite_715() {
    let r = mit_semantik(TABLISTE_AUS_LI);
    assert_eq!(
        an(TABLISTE_AUS_LI, &r, "aria/required-children-missing"),
        ["tabs"]
    );
    assert_eq!(
        an(TABLISTE_AUS_LI, &r, "aria/required-parent-missing"),
        ["li1", "li2", "t1", "t2"]
    );
}

#[test]
fn tabliste_aus_listeneintraegen_auch_wenn_der_host_das_li_glaettet_auditmysite_715() {
    let r = wie_chrome(TABLISTE_AUS_LI);
    assert_eq!(
        an(TABLISTE_AUS_LI, &r, "aria/required-children-missing"),
        ["tabs"]
    );
    assert_eq!(
        an(TABLISTE_AUS_LI, &r, "aria/required-parent-missing"),
        ["li1", "li2", "t1", "t2"]
    );
}

#[test]
fn richtig_gebaute_tabliste_und_praesentationale_listen_bleiben_still() {
    let src = r##"<div role="tablist" aria-label="T">
                    <button role="tab" aria-selected="true" aria-controls="p">A</button>
                    <span><button role="tab" aria-selected="false">B</button></span>
                  </div>
                  <div role="tabpanel" id="p">P</div>
                  <ul role="tablist"><li role="presentation"><a role="tab" href="#x" aria-selected="true">X</a></li></ul>
                  <ul role="menu"><li role="none"><a role="menuitem" href="#m">M</a></li></ul>
                  <ul role="none"><li><a href="#n">N</a></li></ul>
                  <ul role="list"><li>L</li></ul>"##;
    for r in [mit_semantik(src), wie_chrome(src)] {
        assert_eq!(
            anzahl(&r, "aria/required-parent-missing"),
            0,
            "{:?}",
            r.findings
        );
        assert_eq!(
            anzahl(&r, "aria/required-children-missing"),
            0,
            "{:?}",
            r.findings
        );
    }
}

#[test]
fn verwaister_tab_korpus_aria_attribute_validation() {
    let src = r#"<div id="t" role="tab" tabindex="0">Orphan tab</div>"#;
    assert_eq!(
        an(src, &mit_semantik(src), "aria/required-parent-missing"),
        ["t"]
    );
}

#[test]
fn native_tabelle_mit_tbody_auditmysite_659_674() {
    // Ohne role: HTML gibt die Bestandteile vor. Mit role="grid"/"table":
    // Das tbody ist eine rowgroup (accname) oder ausgeblendet (Chrome) —
    // die Zeilen zählen in beiden Fällen.
    let src = r#"<table id="keys" aria-label="Keyboard shortcuts"><tbody>
                   <tr><th scope="row"><kbd>Home</kbd></th><td>First</td></tr></tbody></table>
                 <table role="grid" aria-label="Orders">
                   <thead><tr><th scope="col">ID</th></tr></thead>
                   <tbody><tr><td role="gridcell" tabindex="-1">1</td></tr></tbody></table>
                 <table role="table" aria-label="Plain"><tbody><tr role="row"><td role="cell">1</td></tr></tbody></table>"#;
    for r in [mit_semantik(src), wie_chrome(src)] {
        assert_eq!(
            anzahl(&r, "aria/required-children-missing"),
            0,
            "{:?}",
            r.findings
        );
        assert_eq!(
            anzahl(&r, "aria/required-parent-missing"),
            0,
            "{:?}",
            r.findings
        );
    }
}

#[test]
fn tabelle_nur_mit_beschriftung_korpus_table_required_rows_tbody() {
    // Der Korpusfall ist eine native Tabelle; hier mit expliziter Rolle,
    // weil native Tabellen bewusst nicht geprüft werden (siehe CHANGELOG).
    let src = r#"<div id="leer" role="table" aria-label="Customers"><div role="caption">Customers</div></div>
                 <table id="nativ" aria-label="Customers"><caption>Customers</caption></table>"#;
    assert_eq!(
        an(src, &mit_semantik(src), "aria/required-children-missing"),
        ["leer"]
    );
}

#[test]
fn bestandteile_ausnahmen_leer_beschaeftigt_eingeklappt() {
    let src = r#"<div role="tablist" aria-label="leer"></div>
                 <div role="listbox" aria-busy="true" aria-label="lädt"><p>Lädt …</p></div>
                 <div role="menu" aria-expanded="false" aria-label="zu"><p>zu</p></div>
                 <div role="list"><div><div role="listitem">durch generic hindurch</div></div></div>
                 <div role="listbox" aria-label="g"><div role="group"><div role="option">o</div></div></div>"#;
    let r = mit_semantik(src);
    assert_eq!(
        anzahl(&r, "aria/required-children-missing"),
        0,
        "{:?}",
        r.findings
    );
    assert_eq!(
        anzahl(&r, "aria/required-parent-missing"),
        0,
        "{:?}",
        r.findings
    );
}

#[test]
fn falsche_bestandteile_werden_gemeldet() {
    let src = r#"<div id="liste" role="list"><p>kein Eintrag</p></div>
                 <div id="menue" role="menubar"><ul><li><a id="punkt" role="menuitem" href="/">M</a></li></ul></div>"#;
    let r = mit_semantik(src);
    assert_eq!(
        an(src, &r, "aria/required-children-missing"),
        ["liste", "menue"]
    );
    assert_eq!(an(src, &r, "aria/required-parent-missing"), ["punkt"]);
}

#[test]
fn aria_owns_stiftet_kontext_und_bestandteile() {
    let src = r#"<div id="lb" role="listbox" aria-label="L" aria-owns="o1"></div>
                 <div><div id="o1" role="option">o</div></div>"#;
    let r = mit_semantik(src);
    assert_eq!(
        anzahl(&r, "aria/required-children-missing"),
        0,
        "{:?}",
        r.findings
    );
    assert_eq!(
        anzahl(&r, "aria/required-parent-missing"),
        0,
        "{:?}",
        r.findings
    );
}

// --- Widgets ---------------------------------------------------------------

#[test]
fn tab_ohne_auswahl_korpus_aria_and_widgets() {
    let src = r#"<div id="tl" role="tablist">
                   <div role="tab" id="tab1" aria-controls="panel1">First</div>
                   <div role="tab" id="tab2" aria-selected="false" aria-controls="panel1">Second</div>
                 </div>
                 <div role="tabpanel" id="panel1">Panel</div>"#;
    let r = run(&html(src));
    assert_eq!(an(src, &r, "aria/tab-selected-missing"), ["tl"]);
    assert_eq!(outcome(&r, "aria/tab-selected-missing"), Outcome::Review);
    assert_eq!(anzahl(&r, "aria/tabpanel-missing"), 0);
}

#[test]
fn ein_gewaehlter_tab_genuegt() {
    let r = run(&html(
        r#"<div role="tablist"><div role="tab" aria-selected="true">A</div><div role="tab">B</div></div>
           <div role="tabpanel">P</div>"#,
    ));
    assert_eq!(anzahl(&r, "aria/tab-selected-missing"), 0);
}

#[test]
fn widget_muster_korpus_widget_patterns() {
    let src = r#"<div id="tl" role="tablist" aria-label="Sections"><div role="tab" id="tab1">Tab 1</div></div>
                 <div id="cb" role="combobox" aria-expanded="true" aria-label="Fruit" tabindex="0"></div>
                 <ul id="listbox1" role="listbox"><li role="option">Apple</li></ul>"#;
    let r = run(&html(src));
    assert_eq!(an(src, &r, "aria/tabpanel-missing"), ["tl"]);
    assert_eq!(outcome(&r, "aria/tabpanel-missing"), Outcome::Review);
    assert_eq!(an(src, &r, "aria/tab-selected-missing"), ["tl"]);
    assert_eq!(an(src, &r, "aria/combobox-popup-missing"), ["cb"]);
    assert_eq!(outcome(&r, "aria/combobox-popup-missing"), Outcome::Fail);
}

#[test]
fn combobox_mit_popup_oder_eingeklappt_www_gov_uk() {
    let r = run(&html(
        r#"<input role="combobox" aria-expanded="true" aria-controls="lb" aria-label="Suche">
           <ul id="lb" role="listbox"><li role="option">a</li></ul>
           <input role="combobox" aria-expanded="false" aria-label="zu">
           <div role="combobox" aria-expanded="true" aria-label="alt"><div role="listbox"><div role="option">o</div></div></div>"#,
    ));
    assert_eq!(anzahl(&r, "aria/combobox-popup-missing"), 0);
}

// --- popover, inert -------------------------------------------------------

#[test]
fn popover_ziel_fehlt_oder_ist_keins_modern_attributes() {
    let src = r#"<button id="fehlt" popovertarget="nirgends">a</button>
                 <button id="falsch" popovertarget="kein-popover">b</button>
                 <button id="gut" popovertarget="pop">c</button>
                 <div id="kein-popover">x</div>
                 <div id="pop" popover>y</div>"#;
    let r = run(&html(src));
    assert_eq!(an(src, &r, "popover/target-missing"), ["fehlt"]);
    assert_eq!(an(src, &r, "popover/target-invalid"), ["falsch"]);
}

#[test]
fn inerter_offener_dialog_modern_attributes() {
    let src = r#"<div inert><dialog id="offen" open><button>OK</button></dialog></div>
                 <dialog id="zu" inert><button>OK</button></dialog>
                 <div id="aria" role="dialog" aria-label="D" inert><button>OK</button></div>
                 <dialog id="frei" open><button>OK</button></dialog>"#;
    let r = run(&html(src));
    assert_eq!(an(src, &r, "inert/dialog-inert"), ["aria", "offen"]);
    let f: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "inert/dialog-inert")
        .map(|f| f.outcome)
        .collect();
    assert!(f.contains(&Outcome::Fail) && f.contains(&Outcome::Review));
}

// --- aria/reference-missing (erweitert) ------------------------------------

#[test]
fn details_flowto_und_fehlermeldung_als_verweise() {
    let src = r#"<div id="d" aria-details="weg">a</div>
                 <div id="f" aria-flowto="weg">b</div>
                 <input id="still" aria-errormessage="kommt-noch" aria-label="c">
                 <input id="laut" aria-invalid="true" aria-errormessage="weg" aria-label="d">"#;
    let r = run(&html(src));
    assert_eq!(an(src, &r, "aria/reference-missing"), ["d", "f", "laut"]);
}

// --- Sprache ---------------------------------------------------------------

#[cfg(feature = "de")]
#[test]
fn deutsche_texte() {
    use a11y_rules::{Locale, run_with_semantics_in};
    let a = html(r#"<div role="tab" tabindex="0">T</div>"#);
    let r = run_with_semantics_in(&MitSemantik::new(&a), Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "aria/required-parent-missing")
        .unwrap();
    assert!(f.message.contains("muss in einem Element"), "{}", f.message);
}
