//! Links- und Zeigerregeln (casoon/barrierlab#18).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`tests/fixtures/detection_corpus/`) und aus echten Seiten (Abruf
//! 2026-10-03); der Name jedes Tests nennt die Quelle. Alle drei Regeln sind
//! Tier 1 und laufen ohne Semantik.

use a11y_dom::{Arena, ArenaBuilder, Node, elements};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn page(head_and_body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head>{head_and_body}</html>"
    ))
    .document;
    let root = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    adopt(&doc, root, Arena::builder()).build()
}

fn body(src: &str) -> Arena {
    page(&format!("<body><main>{src}</main></body>"))
}

fn adopt(doc: &html5_parser::Document, id: html5_parser::NodeId, b: ArenaBuilder) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = adopt(doc, k, b);
            }
            b.close()
        }
        H::Text { content } => b.text(content),
        _ => b,
    }
}

/// Die `id`-Attribute der Elemente, an denen `rule` meldet.
fn findings_at(doc: &Arena, r: &Report, rule: &str) -> Vec<String> {
    let node: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| f.location.node.clone())
        .collect();
    elements(doc)
        .filter(|n| node.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

fn verdicts(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

// --- keyboard/click-handler-not-focusable ----------------------------------

#[test]
fn corpus_keyboard_and_targets() {
    let doc = body(
        r#"<div id="klick" onclick="console.log('clicked')">Clickable div with no keyboard handler</div>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "keyboard/click-handler-not-focusable"),
        ["klick"]
    );
    assert_eq!(
        verdicts(&r, "keyboard/click-handler-not-focusable"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn corpus_audit_exclude_cap() {
    let doc = body(
        r##"<div id="real-click" onclick="void 0">Real fake button</div>
           <p><a id="real-link" href="#" onclick="return false">Real fake link</a></p>"##,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "keyboard/click-handler-not-focusable"),
        ["real-click"]
    );
    assert_eq!(findings_at(&doc, &r, "links/used-as-button"), ["real-link"]);
}

#[test]
fn elements_made_operable_auditmysite_click_handlers() {
    let doc = body(
        r#"<div id="rolle" role="button" onclick="x()">Rolle</div>
           <span id="tab" tabindex="0" onclick="x()">Tabfolge</span>
           <li id="minus" tabindex="-1" onclick="x()">Nur programmatisch</li>
           <button id="knopf" onclick="x()">Knopf</button>
           <td id="zelle" onclick="x()">Nicht in der Liste</td>
           <a id="ohne-href" onclick="x()">Platzhalter</a>
           <a id="mit-href" href="/x" onclick="track()">Ziel</a>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "keyboard/click-handler-not-focusable"),
        ["minus", "ohne-href"]
    );
}

#[test]
fn data_onclick_is_not_a_handler_weather_com() {
    // Echte Seite: wetter.com hängt Handler über data-onclick an.
    let doc = body(
        r#"<div id="w" class="scroll-btn" data-WetterAction data-target="body" data-onclick="scrollToTarget">↑</div>"#,
    );
    assert!(findings_at(&doc, &run(&doc), "keyboard/click-handler-not-focusable").is_empty());
}

// --- links/used-as-button ---------------------------------------------------

#[test]
fn consent_call_weather_com() {
    let doc = body(
        r##"<a id="uc" data-label="Privatsphäre Einstellungen" href="#" onclick="UC_UI_recall();" aria-label="Privatsphäre Einstellungen" class="cmp-lnk-recall">Privatsphäre</a>"##,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "links/used-as-button"), ["uc"]);
    assert_eq!(
        verdicts(&r, "links/used-as-button"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn reload_craigslist() {
    let doc = body(
        r##"<a id="cl-unrecoverable-hard-refresh" href="#" onclick="location.reload(true);">reload</a>"##,
    );
    assert_eq!(
        findings_at(&doc, &run(&doc), "links/used-as-button"),
        ["cl-unrecoverable-hard-refresh"]
    );
}

#[test]
fn targets_auditmysite_fake_navigation_link() {
    let doc = body(
        r##"<a id="js" href=" JavaScript:void(0)" onclick="x()">js</a>
            <a id="leer" href="" onclick="x()">leer</a>
            <a id="knopf" href="#" role="button" onclick="x()">Rolle button</a>
            <a id="anker" href="#abschnitt" onclick="x()">Anker</a>
            <a id="ohne-handler" href="#">kein onclick</a>
            <a id="wetter" href="javascript:void(0);" role="button" data-onclick="openNext">Favoriten</a>"##,
    );
    assert_eq!(
        findings_at(&doc, &run(&doc), "links/used-as-button"),
        ["js", "leer"]
    );
}

// --- navigation/location-missing --------------------------------------------

#[test]
fn home_page_without_location() {
    // Wie die Startseiten von spiegel.de, zeit.de und tagesschau.de:
    // Navigation, kein aria-current, kein Brotkrumenpfad.
    let doc = page(
        r#"<body><header><nav aria-label="Hauptnavigation"><a href="/politik">Politik</a><a href="/sport">Sport</a></nav></header><main></main></body>"#,
    );
    let r = run(&doc);
    assert_eq!(
        verdicts(&r, "navigation/location-missing"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn breadcrumb_gov_uk_and_current_page_bundesregierung() {
    let breadcrumb = page(
        r#"<body><nav aria-label="Menu"><a href="/browse">Browse</a></nav>
           <main><nav class="govuk-breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/">Home</a></li></ol></nav></main></body>"#,
    );
    assert!(verdicts(&run(&breadcrumb), "navigation/location-missing").is_empty());

    let current = page(
        r#"<body><div role="navigation"><a href="/breg-de/bundesregierung" aria-current="page">Bundesregierung</a></div><main></main></body>"#,
    );
    assert!(verdicts(&run(&current), "navigation/location-missing").is_empty());
}

#[test]
fn carousel_dot_is_not_a_location_t_online() {
    let doc = page(
        r#"<body><nav><a href="/a">A</a></nav><main><ul><li aria-label="Navigation Reizartikel 1" aria-current="true">1</li></ul></main></body>"#,
    );
    assert_eq!(verdicts(&run(&doc), "navigation/location-missing").len(), 1);
}

#[test]
fn without_navigation_no_hint() {
    let doc = body(r#"<a href="/x">Nur ein Link</a>"#);
    assert!(verdicts(&run(&doc), "navigation/location-missing").is_empty());
}

#[cfg(feature = "de")]
#[test]
fn findings_follow_the_language() {
    let doc = body(r#"<div onclick="x()">Klick</div>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "keyboard/click-handler-not-focusable")
        .unwrap();
    assert!(f.message.contains("Tastatur"), "{}", f.message);
}
