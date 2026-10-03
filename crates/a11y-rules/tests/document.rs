//! Dokument-, Sprach-, Zeit- und Gliederungsregeln (casoon/barrierlab#20).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`tests/fixtures/detection_corpus/`) und aus echten Seiten (Abruf
//! 2026-10-03); der Name jedes Tests nennt die Quelle. Alle Regeln sind
//! Tier 1 und laufen ohne Semantik.

use a11y_dom::{Arena, ArenaBuilder, Node, elements};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

/// Ein ganzes Dokument als Arena, über den echten Parser.
fn dokument(src: &str) -> Arena {
    let doc = html5_parser::parse(src).document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    uebernehmen(&doc, wurzel, Arena::builder()).build()
}

fn body(src: &str) -> Arena {
    dokument(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body><main>{src}</main></body></html>"
    ))
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

// --- document/lang-mismatch, language/abbreviation-unexpanded ---------------

/// Korpus `misc_content_checks`: `lang` und `xml:lang` widersprechen sich,
/// und `<abbr>W3C</abbr>` hat kein `title`.
#[test]
fn korpus_misc_content_checks() {
    let doc = dokument(
        r#"<!DOCTYPE html><html id="html" lang="en" xml:lang="de"><head><title>T</title></head>
           <body><main><p><abbr id="w3c">W3C</abbr> publishes web standards.</p></main></body></html>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "document/lang-mismatch"), ["html"]);
    assert_eq!(
        urteile(&r, "document/lang-mismatch"),
        [(Outcome::Fail, Severity::Medium)]
    );
    assert_eq!(an(&doc, &r, "language/abbreviation-unexpanded"), ["w3c"]);
    assert_eq!(
        urteile(&r, "language/abbreviation-unexpanded"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn gleiche_sprache_bing_com() {
    let doc = dokument(
        r#"<!DOCTYPE html><html dir="ltr" lang="de" xml:lang="de" xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body></body></html>"#,
    );
    assert!(urteile(&run(&doc), "document/lang-mismatch").is_empty());
    // Nur die Primärkennung zählt, wie in auditmysite.
    let regional = dokument(
        r#"<!DOCTYPE html><html lang="de-DE" xml:lang="de"><head><title>T</title></head><body></body></html>"#,
    );
    assert!(urteile(&run(&regional), "document/lang-mismatch").is_empty());
}

#[test]
fn abkuerzung_mit_title_w3_org_wai() {
    let doc = body(r#"<abbr title="World Wide Web Consortium">W3C</abbr>"#);
    assert!(urteile(&run(&doc), "language/abbreviation-unexpanded").is_empty());
}

// --- timing/meta-refresh ----------------------------------------------------

#[test]
fn korpus_meta_refresh_present() {
    let doc = dokument(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta id="refresh" http-equiv="refresh" content="120"><title>T</title></head>
           <body><main><h1>Page with a short meta-refresh</h1></main></body></html>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "timing/meta-refresh"), ["refresh"]);
    assert_eq!(
        urteile(&r, "timing/meta-refresh"),
        [(Outcome::Fail, Severity::High)]
    );
}

/// web.de und gmx.net leiten ohne JavaScript sofort weiter. Eine sofortige
/// Weiterleitung ist keine Frist (H76).
#[test]
fn sofortige_weiterleitung_web_de_gmx_net() {
    let doc = dokument(
        r#"<!DOCTYPE html><html lang="de"><head><title>T</title><meta http-equiv="refresh" content="0;url=/?status=no-script" /></head><body></body></html>"#,
    );
    assert!(urteile(&run(&doc), "timing/meta-refresh").is_empty());
}

#[test]
fn zwanzig_stunden_und_ohne_ziffern_html_standard() {
    let doc = dokument(
        r#"<!DOCTYPE html><html lang="en"><head><title>T</title>
           <meta id="lang" http-equiv="Refresh" content="72000">
           <meta id="kaputt" http-equiv="refresh" content="url=/x">
           <meta id="kurz" http-equiv="REFRESH" content=" 5; url=/weiter"></head><body></body></html>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "timing/meta-refresh"), ["kurz"]);
}

// --- language/part-unmarked -------------------------------------------------

#[test]
fn korpus_text_and_layout() {
    let doc = body(
        r#"<p>This paragraph uses justified text alignment, which creates uneven spacing
              between words and can make reading more difficult for some users.</p>
           <p id="de">
            Die Bundesrepublik Deutschland ist ein demokratischer und sozialer Bundesstaat.
            Alle Staatsgewalt geht vom Volke aus und wird durch Wahlen und Abstimmungen sowie
            durch besondere Organe der Gesetzgebung ausgeübt.
           </p>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "language/part-unmarked"), ["de"]);
    assert_eq!(
        urteile(&r, "language/part-unmarked"),
        [(Outcome::Review, Severity::Medium)]
    );
}

const GRUNDGESETZ: &str = "Die Bundesrepublik Deutschland ist ein demokratischer und sozialer \
    Bundesstaat. Alle Staatsgewalt geht vom Volke aus und wird durch Wahlen und Abstimmungen \
    sowie durch besondere Organe der Gesetzgebung ausgeübt.";

#[test]
fn ausgezeichnete_und_ausgenommene_passagen_auditmysite_language_of_parts() {
    let doc = body(&format!(
        r#"<p id="lang" lang="de">{GRUNDGESETZ}</p>
           <blockquote><p id="zitat">{GRUNDGESETZ}</p></blockquote>
           <p id="code" translate="no">{GRUNDGESETZ}</p>
           <ul><li id="li"><p id="innen">{GRUNDGESETZ}</p></li></ul>"#
    ));
    // Gemeldet wird nur der innerste Textblock.
    assert_eq!(an(&doc, &run(&doc), "language/part-unmarked"), ["innen"]);
}

/// sparkasse.de setzt ein `<style>` von Emotion in jeden Listeneintrag. Sein
/// CSS las sich als englischer Text.
#[test]
fn eingebettetes_style_sparkasse_de() {
    let doc = dokument(
        r#"<!DOCTYPE html><html lang="de"><head><title>T</title></head><body><ul><li id="filialen"><style data-emotion="css lqkmdh">.css-lqkmdh{font-size:13px;line-height:1.385;font-family:var(--font-sparkasse-b1, SparkasseWeb, Sparkasse B1, Sparkasse Web);font-weight:400;text-transform:none;letter-spacing:-0.02em;min-width:64px;padding:6px 16px;border:0;border-radius:6px;transition:background-color 250ms cubic-bezier(0.4, 0, 0.2, 1) 0ms,box-shadow 250ms;--variant-textColor:#EE0000;box-shadow:none;}.css-15os96o:hover:not(.Mui-disabled):not(.Mui-focusVisible):not(:disabled):not(.MuiButton-loading):not(.Mui-active){background-color:#B60000;}</style><a href="/privatkunden">Privatkunden</a></li></ul></body></html>"#,
    );
    // Ohne das Auslassen von <style>: fünf „not" aus `:not(` und kein
    // deutsches Funktionswort — als Englisch erkannt.
    assert!(urteile(&run(&doc), "language/part-unmarked").is_empty());
}

#[test]
fn andere_seitensprache_wird_nicht_geprueft() {
    let doc = dokument(&format!(
        r#"<!DOCTYPE html><html lang="fr"><head><title>T</title></head><body><p>{GRUNDGESETZ}</p></body></html>"#
    ));
    let r = run(&doc);
    assert!(urteile(&r, "language/part-unmarked").is_empty());
    assert_eq!(
        urteile(&r, "language/part-undetermined"),
        [(Outcome::Untested, Severity::Medium)]
    );
}

// --- headings/section-without-heading ---------------------------------------

/// Teaser-Karten als `<article>` ohne Überschrift, wie auf t-online.de (61),
/// faz.net (39) und bild.de (25). Ein Hinweis je Seite.
#[test]
fn teaser_artikel_t_online_de() {
    let doc = dokument(
        r#"<!DOCTYPE html><html id="html" lang="de"><head><title>T</title></head><body>
           <nav aria-label="Hauptnavigation"><a href="/">Start</a></nav>
           <main><h1>Nachrichten</h1>
             <article class="relative min-w-0"><a href="/a">Tagesanbruch – Der Newsletter</a></article>
             <article class="relative min-w-0"><a href="/b">Denken gegen die Uhr</a></article>
             <article><h2>Mit Überschrift</h2></article>
             <section aria-label="Wetter"><p>Wählen Sie eine Region aus.</p></section>
             <section><p>Unbenannt: keine region.</p></section>
           </main></body></html>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "headings/section-without-heading"), ["html"]);
    assert_eq!(
        urteile(&r, "headings/section-without-heading"),
        [(Outcome::Review, Severity::Low)]
    );
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "headings/section-without-heading")
        .unwrap();
    assert!(f.message.contains(": 3."), "{}", f.message);
}

#[cfg(feature = "de")]
#[test]
fn befunde_folgen_der_sprache() {
    let doc = body(r#"<abbr>W3C</abbr>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "language/abbreviation-unexpanded")
        .unwrap();
    assert!(f.message.contains("Langform"), "{}", f.message);
}

/// Source citations with their own lang (geographia.eu, astro-post-audit#77).
#[test]
fn geographia_marked_citation_does_not_change_block_language() {
    let german = "Die Daten und die Informationen sind für die Menschen und werden mit den Quellen auf der Seite als Grundlage für die weiteren Untersuchungen bereitgestellt.";
    let doc = body(&format!(
        r#"<ul>
        <li id="marked"><cite lang="de">{german}</cite> · licence: CC BY 4.0 · retrieved 29 September 2026</li>
        <li id="unmarked"><cite>{german}</cite> · licence: CC BY 4.0</li>
        <li id="mixed"><cite lang="de">{german}</cite> {german}</li>
        </ul>"#
    ));
    let r = run(&doc);
    assert_eq!(
        an(&doc, &r, "language/part-unmarked"),
        ["unmarked", "mixed"]
    );
}
