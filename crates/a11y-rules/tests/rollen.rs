//! Rollen-, Namens- und Tooltip-Hinweise (casoon/barrierlab#20).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`tests/fixtures/detection_corpus/`), aus auditmysite#644 und aus echten
//! Seiten (Abruf 2026-10-03); der Name jedes Tests nennt die Quelle. Alle
//! Regeln sind Tier 1 und laufen ohne Semantik.

use a11y_dom::{Arena, ArenaBuilder, Node, elements};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn body(src: &str) -> Arena {
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
        _ => b,
    }
}

/// Die `id`-Attribute der Elemente, an denen `rule` meldet.
fn an(doc: &Arena, r: &Report, rule: &str) -> Vec<String> {
    let knoten: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| f.location.node.clone())
        .collect();
    elements(doc)
        .filter(|n| knoten.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

fn urteile(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

// --- aria/role-redundant ----------------------------------------------------

/// Korpus `redundant_role_list_style`, auditmysite#644 (barrierlab.eu):
/// `role="list"` auf einer Liste mit `list-style: none` stellt die Semantik
/// wieder her, die WebKit/VoiceOver ihr nimmt. Ohne `list-style-type` im
/// berechneten Stil prüft die Regel `<ul>`/`<ol>` mit `role="list"` gar nicht
/// — auch `ol#numbered-list` nicht, das auditmysite mit Stil meldet.
#[test]
fn korpus_redundant_role_list_style_issue_644() {
    let doc = body(
        r#"<ul role="list" class="plain" id="plain-list"><li>First</li><li>Second</li></ul>
           <ol role="list" id="numbered-list"><li>One</li><li>Two</li></ol>
           <button type="button" role="button" id="send-button">Send</button>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "aria/role-redundant"), ["send-button"]);
    assert_eq!(
        urteile(&r, "aria/role-redundant"),
        [(Outcome::Fail, Severity::Low)]
    );
}

/// bahn.de und spiegel.de (`<button role="button">` als Akkordeon-Auslöser),
/// wetter.com (`li[role=menuitem] > a[href][role=link]`), sparkasse.de
/// (`<a tabindex="0" role="link" href>`).
#[test]
fn echte_seiten_bahn_de_wetter_com_sparkasse_de() {
    let doc = body(
        r#"<h3><button id="bahn" class="js-accordion-trigger" role="button" aria-expanded="false">Service</button></h3>
           <ul role="menubar"><li role="menuitem"><a id="wetter" href="/wetter" role="link">Wetter</a></li></ul>
           <a id="sparkasse" tabindex="0" role="link" href="https://www.sparkasse.de/standorte/filialen">Filialen</a>
           <a id="ohne-href" role="link" tabindex="0">Kein Link ohne href</a>"#,
    );
    assert_eq!(
        an(&doc, &run(&doc), "aria/role-redundant"),
        ["bahn", "wetter", "sparkasse"]
    );
}

/// lidl.de: `ul[role=list] > li[role=listitem]` in einem Masonry-Grid. Die
/// Rolle des Eintrags gehört zur selben WebKit-Abhilfe wie die der Liste.
#[test]
fn listeneintrag_in_rollenliste_lidl_de() {
    let doc = body(
        r#"<ul role="list" class="ux-masonry-grid"><li id="lidl" class="ux-masonry-grid__item" role="listitem">Deal Days</li></ul>
           <ul><li id="schlicht" role="listitem">Ohne Listenrolle</li></ul>
           <ul role="menu"><li id="menue" role="listitem">Fremde Rolle</li></ul>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "aria/role-redundant"), ["schlicht"]);
}

#[test]
fn tabelle_auditmysite_redundant_role() {
    let doc = body(
        r#"<input id="text" role="textbox">
           <input id="liste" list="vorschlaege" role="textbox"><datalist id="vorschlaege"></datalist>
           <input id="absenden" type="submit" role="button">
           <img id="bild" src="a.png" alt="Logo" role="img">
           <img id="deko" src="b.png" alt="" role="img">
           <h2 id="ueberschrift" role="heading">Titel</h2>
           <table id="tabelle" role="table"><tr><th>A</th></tr><tr><td>1</td></tr></table>
           <header id="kopf" role="banner">Kontextabhängig</header>"#,
    );
    assert_eq!(
        an(&doc, &run(&doc), "aria/role-redundant"),
        ["text", "absenden", "bild", "ueberschrift", "tabelle"]
    );
}

// --- names/title-only -------------------------------------------------------

#[test]
fn korpus_name_description_best_practice() {
    let doc = body(
        r#"<button id="suche" type="button" title="Search"><svg aria-hidden="true" width="10" height="10"></svg></button>
           <button type="button" title="Opens the help page in a new window">Help</button>
           <label for="kw">Keywords</label>
           <input id="kw" type="search" title="Search the whole portal">
           <input type="submit" value="Search" title="Start the search">"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "names/title-only"), ["suche"]);
    assert_eq!(
        urteile(&r, "names/title-only"),
        [(Outcome::Review, Severity::Medium)]
    );
}

/// focus.de (Symbol-Link ohne Text), spiegel.de (Teaser-Link um ein Bild mit
/// leerem `alt`), bahn.de (Suchknopf).
#[test]
fn echte_seiten_focus_de_spiegel_de_bahn_de() {
    let doc = body(
        r#"<a id="focus" class="Footer-Icon mb-xs" href="https://www.facebook.com/focus.de" title="FOCUS online Facebook"><svg class="Footer-Icon-Svg"><use href="/icons.svg#facebook"></use></svg></a>
           <a class="Footer-Icon mb-xs" href="https://www.facebook.com/focus.de" title="FOCUS online Facebook"><svg><use href="/icons.svg#facebook"></use></svg>FOCUS online Facebook</a>
           <a id="spiegel" href="https://www.spiegel.de/ausland/terror" class="block" title="Terror in Europa"><div><picture><img src="t.jpg" title="Terror in Europa - Fotos" alt></picture></div></a>
           <form><button id="bahn" type="submit" class="search-form__search-icon" title="Suchen"></button></form>"#,
    );
    assert_eq!(
        an(&doc, &run(&doc), "names/title-only"),
        ["focus", "spiegel", "bahn"]
    );
}

#[test]
fn andere_namensquellen_auditmysite_711() {
    let doc = body(
        r#"<a href="/a" title="t" aria-label="Name">x</a>
           <a href="/b" title="t"><img src="i.png" alt="Logo"></a>
           <a href="/c" title="t"><span aria-hidden="true">Versteckt</span></a>
           <a title="t">Kein Link ohne href</a>
           <label><input type="checkbox" title="t"> Bedingungen</label>
           <input id="cb" type="checkbox" title="Nur title">
           <input type="button" value="Los" title="t">
           <input id="bild" type="image" src="go.png" title="Los">"#,
    );
    // Das versteckte <span> gibt keinen Namen: Der dritte Link meldet mit.
    assert_eq!(
        an(&doc, &run(&doc), "names/title-only"),
        ["?", "cb", "bild"]
    );
}

// --- patterns/tooltip-unreferenced ------------------------------------------

#[test]
fn korpus_forms_and_misc() {
    let doc = body(r#"<div role="tooltip" id="orphan-tip">Never announced</div>"#);
    let r = run(&doc);
    assert_eq!(
        an(&doc, &r, "patterns/tooltip-unreferenced"),
        ["orphan-tip"]
    );
    assert_eq!(
        urteile(&r, "patterns/tooltip-unreferenced"),
        [(Outcome::Fail, Severity::Low)]
    );
}

#[test]
fn verwiesene_tooltips_apg_tooltip() {
    let doc = body(
        r#"<button aria-describedby="tip1">Speichern</button><div role="tooltip" id="tip1" hidden>Strg+S</div>
           <button aria-labelledby="tip2"><svg aria-hidden="true"></svg></button><div role="tooltip" id="tip2">Löschen</div>
           <div role="tooltip">Ohne id</div>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "patterns/tooltip-unreferenced"), ["?"]);
}

#[cfg(feature = "de")]
#[test]
fn befunde_folgen_der_sprache() {
    let doc = body(r#"<button role="button">x</button>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "aria/role-redundant")
        .unwrap();
    assert!(f.message.contains("wiederholt"), "{}", f.message);
}
