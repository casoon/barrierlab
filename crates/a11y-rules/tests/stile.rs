//! Regeln über Stylesheets (casoon/barrierlab#21).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`media_and_motion`, `reduced_motion_transform`,
//! `reduced_motion_override`, `text_and_layout`) und aus den Stylesheets
//! echter Seiten (Abruf 2026-10-03), nachgebaut auf die Regel, die den Fall
//! trägt; der Name jedes Tests nennt die Quelle.

use a11y_dom::{Arena, ArenaBuilder};
use a11y_report::{NotRun, Outcome, Report, Severity};
use a11y_rules::{run, run_stylesheets};
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn seite(body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body>{body}</body></html>"
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

fn pruefe(body: &str, css: &str) -> Report {
    let doc = seite(body);
    run_stylesheets(run(&doc), &doc, &[stylesheet_parse::parse_stylesheet(css)])
}

fn urteile(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

// --- Vermerke -------------------------------------------------------------

#[test]
fn ohne_stylesheets_nicht_gelaufen_mit_stylesheets_gelaufen() {
    let doc = seite("<p>Text</p>");
    let ohne = run(&doc);
    for id in [
        "focus/outline-removed",
        "motion/reduced-motion-ignored",
        "orientation/content-hidden",
        "text/justified",
        "text/line-height-tight",
    ] {
        let r = ohne.rule_runs.iter().find(|r| r.rule_id == id).unwrap();
        assert!(!r.did_run(), "{id}");
        assert_eq!(r.not_run, Some(NotRun::CapabilityMissing), "{id}");
    }
    let mit = run_stylesheets(ohne, &doc, &[]);
    for id in ["focus/outline-removed", "text/justified"] {
        let laeufe: Vec<_> = mit.rule_runs.iter().filter(|r| r.rule_id == id).collect();
        assert_eq!(laeufe.len(), 1, "{id}");
        assert!(laeufe[0].did_run(), "{id}");
    }
    assert!(
        mit.findings
            .iter()
            .all(|f| !f.rule_id.starts_with("motion/"))
    );
}

// --- focus/outline-removed --------------------------------------------------

const MEDIA_AND_MOTION_CSS: &str = "
    @media (orientation: landscape) { .landscape-only-hide { display: none; } }
    .spinner { animation: spin 2s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }
    *:focus { outline: none; }
";

const MEDIA_AND_MOTION_BODY: &str = r#"<main><h1>Motion</h1>
    <div class="landscape-only-hide">Only shown in portrait orientation</div>
    <div class="spinner">Loading…</div>
    <button>Focusable button with no visible focus indicator</button></main>"#;

#[test]
fn korpus_media_and_motion() {
    let r = pruefe(MEDIA_AND_MOTION_BODY, MEDIA_AND_MOTION_CSS);
    assert_eq!(
        urteile(&r, "focus/outline-removed"),
        [(Outcome::Fail, Severity::High)]
    );
    assert_eq!(
        urteile(&r, "motion/reduced-motion-ignored"),
        [(Outcome::Review, Severity::Medium)]
    );
    assert_eq!(
        urteile(&r, "orientation/content-hidden"),
        [(Outcome::Review, Severity::Medium)]
    );
}

#[test]
fn fokus_fuer_die_tastatur_wieder_gesetzt_sueddeutsche_de() {
    let css = "[data-whatinput] *:focus{outline:none;}
               [data-whatintent='keyboard'] *:focus{outline:2px solid #009990;}";
    assert!(urteile(&pruefe("<button>x</button>", css), "focus/outline-removed").is_empty());
}

#[test]
fn ersatz_oder_focus_visible_nehmen_den_befund_zurueck() {
    for css in [
        "a:focus { outline: none } button:focus { box-shadow: 0 0 0 3px #005fcc }",
        "a:focus { outline: 0 } a:focus-visible { outline: 2px solid }",
        ".btn:focus:not(:focus-visible) { outline: none }",
        "a:focus-within { outline: none }",
    ] {
        assert!(
            urteile(&pruefe("<a href=\"/\">x</a>", css), "focus/outline-removed").is_empty(),
            "{css}"
        );
    }
    let ohne = pruefe(
        "<a href=\"/\">x</a>",
        "a:focus { outline-style: none } a:focus { outline: initial }",
    );
    assert_eq!(urteile(&ohne, "focus/outline-removed").len(), 1);
}

// --- motion/reduced-motion-ignored ------------------------------------------

const KARTE: &str = "
    .card { transition: transform 0.3s ease; }
    .card:hover { transform: translateY(-4px); }
    .slide-in { animation: slide-in 1s ease-out; }
    @keyframes slide-in { from { transform: translateX(-100%); } to { transform: none; } }
";
const KARTE_BODY: &str =
    r#"<main><div class="card">Card</div><div class="slide-in">Panel</div></main>"#;

#[test]
fn korpus_reduced_motion_transform() {
    let r = pruefe(KARTE_BODY, KARTE);
    assert_eq!(urteile(&r, "motion/reduced-motion-ignored").len(), 1);
    assert!(
        r.findings[0..]
            .iter()
            .any(|f| f.message.contains("2 style rules"))
    );
}

#[test]
fn korpus_reduced_motion_override() {
    let css = format!(
        "{KARTE} @media (prefers-reduced-motion: reduce) {{
            *, *::before, *::after {{ transition-duration: 0.01ms !important; }}
            .slide-in {{ animation: none; }}
        }}"
    );
    assert!(urteile(&pruefe(KARTE_BODY, &css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn nur_bewegung_die_auf_der_seite_laufen_kann_auditmysite_712() {
    // bundesregierung.de: Keyframes eines Lade-Kreisels, den die Seite nie
    // einbindet.
    let css = ".player-spinner { animation: spin 1s infinite } @keyframes spin { to { transform: rotate(1turn) } }";
    assert!(urteile(&pruefe("<p>Text</p>", css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn farbe_deckkraft_und_dauer_allein_bewegen_nichts() {
    let css = "a { transition: color .2s, background-color .2s }
               .fade { transition: opacity 1s }
               .duration-200 { transition-duration: 200ms }
               .pulse { animation: pulse 2s infinite } @keyframes pulse { 50% { opacity: .5 } }";
    let body = r#"<a href="/">a</a><div class="fade duration-200 pulse">x</div>"#;
    assert!(urteile(&pruefe(body, css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn transition_all_nur_mit_bewegtem_zustand() {
    let body = r#"<a class="nav" href="/">x</a>"#;
    assert!(
        urteile(
            &pruefe(
                body,
                ".nav { transition: all .3s } .nav:hover { color: red }"
            ),
            "motion/reduced-motion-ignored"
        )
        .is_empty()
    );
    assert_eq!(
        urteile(
            &pruefe(
                body,
                ".nav { transition: .3s } .nav:hover { transform: scale(1.1) }"
            ),
            "motion/reduced-motion-ignored"
        )
        .len(),
        1
    );
}

#[test]
fn abgeschirmt_unter_no_preference_casoon_de() {
    let css = "@media (hover:hover) and (prefers-reduced-motion:no-preference) {
        .row { transition: opacity .6s, transform .6s; transform: translateY(24px) } }";
    assert!(
        urteile(
            &pruefe(r#"<div class="row">x</div>"#, css),
            "motion/reduced-motion-ignored"
        )
        .is_empty()
    );
}

// --- orientation/content-hidden ---------------------------------------------

#[test]
fn breakpoint_marker_bundesregierung_de() {
    let css =
        "@media only screen and (min-width:0) and (max-width:479px) and (orientation:landscape) {
        body:before { display: none } }";
    assert!(urteile(&pruefe("<p>x</p>", css), "orientation/content-hidden").is_empty());
}

#[test]
fn dialog_bittet_ums_drehen_welt_de() {
    let css = "@media only screen and (max-width: 767px) and (orientation: landscape) and (max-height: 375px) {
        .has-orientation-notification .c-modal__content-body { display: none } }";
    let offen = r#"<div class="c-modal has-orientation-notification"><div class="c-modal__content-body">Artikel</div></div>"#;
    assert_eq!(
        urteile(&pruefe(offen, css), "orientation/content-hidden").len(),
        1
    );
    // Ohne den Dialog im Dokument trifft die Regel nichts.
    assert!(urteile(&pruefe("<p>x</p>", css), "orientation/content-hidden").is_empty());
}

#[test]
fn ganzer_inhalt_ist_schwer() {
    let css = "@media (orientation: portrait) { body { transform: rotate(90deg) } }";
    assert_eq!(
        urteile(&pruefe("<p>x</p>", css), "orientation/content-hidden"),
        [(Outcome::Review, Severity::High)]
    );
}

// --- text/justified, text/line-height-tight ---------------------------------

#[test]
fn korpus_text_and_layout() {
    let css = ".justified { text-align: justify; } .tight-box { line-height: 1.1; }";
    let body = r#"<main><p class="justified">This paragraph uses justified text alignment.</p>
        <div class="tight-box">Clipped box</div><p>Die Bundesrepublik</p></main>"#;
    let r = pruefe(body, css);
    assert_eq!(
        urteile(&r, "text/justified"),
        [(Outcome::Review, Severity::Low)]
    );
    // Die enge Zeilenhöhe trifft ein div, keinen Absatz.
    assert!(urteile(&r, "text/line-height-tight").is_empty());
}

#[test]
fn normalize_wird_von_den_absaetzen_ueberschrieben() {
    let css = "html { line-height: 1.15 } .prose { line-height: 1.75 }";
    let body = r#"<article class="prose"><p>Absatz</p></article>"#;
    assert!(urteile(&pruefe(body, css), "text/line-height-tight").is_empty());
}

#[test]
fn govuk_fliesstext_unter_eineinhalb() {
    // gov.uk: .govuk-body { line-height: 1.3157894737 } (25px auf 19px).
    let css = "html { line-height: 1.15 } .govuk-body { line-height: 1.3157894737 }";
    let r = pruefe(r#"<p class="govuk-body">Text</p>"#, css);
    assert_eq!(
        urteile(&r, "text/line-height-tight"),
        [(Outcome::Review, Severity::Low)]
    );
    assert!(
        r.findings
            .iter()
            .any(|f| f.message.contains("1.3157894737"))
    );
}

#[test]
fn zeilenhoehe_in_pixeln_bleibt_unbeurteilt() {
    let r = pruefe("<p>Text</p>", "p { line-height: 18px }");
    assert!(urteile(&r, "text/line-height-tight").is_empty());
}
