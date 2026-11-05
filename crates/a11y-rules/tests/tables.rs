//! Tabellenregeln (casoon/barrierlab#20).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`tests/fixtures/detection_corpus/`) und aus den Fehlalarmen, die
//! auditmysite bis 1.7.0 behoben hat (#638, #639, #654, #659); der Name jedes
//! Tests nennt die Quelle. Beide Regeln sind Tier 1 und laufen ohne Semantik.

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

/// Die Befunde der neuen Tabellenregeln.
fn new(r: &Report) -> Vec<&str> {
    r.findings
        .iter()
        .map(|f| f.rule_id.as_str())
        .filter(|id| {
            matches!(
                *id,
                "tables/header-without-data"
                    | "tables/data-undetermined"
                    | "tables/headers-attr-invalid"
            )
        })
        .collect()
}

// --- tables/header-without-data ---------------------------------------------

#[test]
fn corpus_table_headers_no_data() {
    let doc = body(
        r#"<table aria-label="Empty results">
             <thead><tr><th id="a">Name</th><th id="b">Score</th></tr></thead>
             <tbody><tr><th id="c">No results</th><th id="d">—</th></tr></tbody>
           </table>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "tables/header-without-data"),
        ["a", "b", "c", "d"]
    );
    assert!(
        verdicts(&r, "tables/header-without-data")
            .iter()
            .all(|u| *u == (Outcome::Fail, Severity::High))
    );
}

/// auditmysite#654 und Korpus `table_grid_rows_unrendered`: Der Zeilenvorrat
/// eines virtualisierten Grids steht im DOM, ist aber noch nicht dargestellt.
/// Hier mit `hidden` statt `display: none` — Tier 1 kennt nur das Attribut;
/// mit Darstellung fällt der Stil in dieselbe Sicht.
#[test]
fn corpus_table_grid_rows_unrendered_issue_654() {
    let doc = body(
        r#"<table id="pending" role="grid" aria-label="Orders">
             <thead><tr><th scope="col">ID</th><th scope="col">Customer</th></tr></thead>
             <tbody><tr hidden><td tabindex="-1"></td><td tabindex="-1"></td></tr>
                    <tr hidden><td tabindex="-1"></td><td tabindex="-1"></td></tr></tbody>
           </table>
           <table id="loaded" role="grid" aria-label="Orders">
             <thead><tr><th scope="col">ID</th><th scope="col">Customer</th></tr></thead>
             <tbody><tr><td tabindex="-1">1</td><td tabindex="-1">Customer 1</td></tr></tbody>
           </table>
           <table id="header-only" aria-label="Header only">
             <tr><th id="name" scope="col">Name</th><th id="score" scope="col">Score</th></tr>
           </table>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "tables/header-without-data"),
        ["name", "score"]
    );
    assert_eq!(
        findings_at(&doc, &r, "tables/data-undetermined"),
        ["pending"]
    );
    assert_eq!(
        verdicts(&r, "tables/data-undetermined"),
        [(Outcome::Untested, Severity::High)]
    );
}

/// auditmysite#654, zweiter Fall (og-vanilla.casoon.dev/table): eine native
/// Tabelle mit `th scope="col"` und gefülltem Körper.
#[test]
fn native_table_with_body_issue_654() {
    let doc = body(
        r#"<table><caption>Orders</caption>
             <thead><tr><th scope="col">ID</th><th scope="col">Customer</th></tr></thead>
             <tbody><tr><td>1</td><td>Customer 1</td></tr><tr><td>2</td><td>Customer 2</td></tr></tbody>
           </table>"#,
    );
    assert!(new(&run(&doc)).is_empty());
}

/// Korpus `table_headers_tbody_ignored`, auditmysite#638 (geographia.eu,
/// Hebel-Kapitel): Leere Zellen und Zellen, die mit einem `aria-hidden`-Kind
/// beginnen, sind trotzdem Datenzellen.
#[test]
fn corpus_table_headers_tbody_ignored_issue_638() {
    let doc = body(
        r#"<div class="scroll" role="region" aria-label="Lever map" tabindex="0">
           <table class="map">
             <caption class="sr-only">Lever map: sectors and the levers that apply</caption>
             <thead><tr><th scope="col">Sector</th><th scope="col" class="num">Share</th><th scope="col">Generate clean</th><th scope="col">Electrify</th></tr></thead>
             <tbody>
               <tr><th scope="row">Power and heat</th><td class="num"><span class="bar" aria-hidden="true"><span style="width:81.8%"></span></span>33 %</td><td><span aria-hidden="true">●</span><span class="sr-only">applies</span></td><td></td></tr>
               <tr><th scope="row">Transport</th><td class="num"><span class="bar" aria-hidden="true"><span style="width:40.8%"></span></span>16 %</td><td></td><td><span aria-hidden="true">●</span><span class="sr-only">applies</span></td></tr>
             </tbody>
           </table></div>"#,
    );
    assert!(new(&run(&doc)).is_empty());
}

/// auditmysite#638, Kommentar (barrierlab.eu/de/learn/accessible-website/):
/// ein Spaltenkopf über einer Spalte aus Zeilenköpfen.
#[test]
fn column_header_over_row_headers_barrierlab_eu_issue_638() {
    let doc = body(
        r#"<table><caption>Häufigste Verstöße</caption>
             <thead><tr><th scope="col">Verstoß</th><th scope="col">WCAG</th><th scope="col">Anteil der Seiten</th></tr></thead>
             <tbody><tr><th scope="row">Textkontrast zu gering</th><td>1.4.3</td><td>83,9 %</td></tr></tbody>
           </table>"#,
    );
    assert!(new(&run(&doc)).is_empty());
}

/// auditmysite#639 (geographia.eu, Hebel-Kapitel): Dieselbe Seite wie #638
/// hat ein `<header>` im Artikel in `<main>`. Den Landmark-Teil prüft
/// `tests/landmarks.rs` (`corpus_landmark_header_in_main_issue_639`) mit
/// Semantik; hier steht fest, dass die Tabelle in diesem Gefüge keinen
/// Tabellenbefund erzeugt und Tier 1 kein zweites Banner meldet.
#[test]
fn table_in_chapter_geographia_issue_639() {
    let doc = page(
        r#"<body><header class="site-header"><a href="/">geographia</a></header>
           <main><div class="stage-content"><div class="chapter"><article>
             <header class="grid gap-8"><h1>Hebel</h1></header>
             <table class="map"><caption>Hebel</caption>
               <thead><tr><th scope="col">Sektor</th><th scope="col">Anteil</th></tr></thead>
               <tbody><tr><th scope="row">Strom und Wärme</th><td>33 %</td></tr></tbody>
             </table>
           </article></div></div></main></body>"#,
    );
    let r = run(&doc);
    assert!(new(&r).is_empty());
    assert!(
        !r.findings
            .iter()
            .any(|f| f.rule_id.starts_with("landmarks/") && f.rule_id.ends_with("-duplicate")),
        "{:?}",
        r.findings
    );
}

/// auditmysite#659 (og-vanilla.casoon.dev/accessibility) und Korpus
/// `table_required_rows_tbody`: `tbody > tr` mit Zeilenkopf und Datenzelle,
/// daneben eine Tabelle nur mit Beschriftung. Die Rollenregel
/// (`aria/required-children-missing`) prüft `tests/aria.rs`.
#[test]
fn corpus_table_required_rows_tbody_issue_659() {
    let doc = body(
        r#"<table id="keys" aria-label="Keyboard shortcuts"><tbody>
             <tr><th scope="row"><kbd>Home</kbd> / <kbd>End</kbd></th><td>First / last cell of the row</td></tr>
             <tr><th scope="row"><kbd>Page Up</kbd></th><td>Scroll one page up</td></tr>
           </tbody></table>
           <table id="empty" aria-label="Customers by revenue"><caption>Customers by revenue</caption></table>"#,
    );
    assert!(new(&run(&doc)).is_empty());
}

#[test]
fn nested_table_counts_on_its_own() {
    // Die Datenzellen der inneren Tabelle gehören nicht der äußeren.
    let doc = body(
        r#"<table aria-label="Außen"><tr><th id="aussen">Kopf</th></tr>
             <tr><th id="rahmen"><table aria-label="Innen"><tr><th>K</th></tr><tr><td>D</td></tr></table></th></tr>
           </table>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "tables/header-without-data"),
        ["aussen", "rahmen"]
    );
}

// --- tables/headers-attr-invalid --------------------------------------------

#[test]
fn corpus_forms_extended() {
    let doc = body(
        r#"<table>
             <thead><tr><th id="col-name">Name</th></tr></thead>
             <tbody><tr><td id="alice" headers="does-not-exist">Alice</td></tr></tbody>
           </table>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "tables/headers-attr-invalid"),
        ["alice"]
    );
    assert_eq!(
        verdicts(&r, "tables/headers-attr-invalid"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn references_per_html_standard_auditmysite_td_headers_attr() {
    let doc = body(
        r#"<table><tr><th id="h1">A</th><th id="h2">B</th><td id="d0">x</td></tr>
             <tr><td id="gut" headers="h1 h2">1</td><td id="leer" headers=" ">2</td>
                 <td id="auf-td" headers="d0">3</td><td id="fremd" headers="h3">4</td></tr></table>
           <table><tr><th id="h3">C</th></tr><tr><td>5</td></tr></table>
           <p id="kein-cell" headers="nirgends">Kein Zellelement</p>"#,
    );
    let r = run(&doc);
    assert_eq!(
        findings_at(&doc, &r, "tables/headers-attr-invalid"),
        ["leer", "auf-td", "fremd"]
    );
}

#[cfg(feature = "de")]
#[test]
fn findings_follow_the_language() {
    let doc = body(r#"<table><tr><td headers="x">1</td></tr></table>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "tables/headers-attr-invalid")
        .unwrap();
    assert!(f.message.contains("Kopfzelle"), "{}", f.message);
}
