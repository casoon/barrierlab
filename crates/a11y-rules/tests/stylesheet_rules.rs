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

fn page(body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body>{body}</body></html>"
    ))
    .document;
    let root = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    adopt(&doc, root, Arena::builder()).build()
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

fn check(body: &str, css: &str) -> Report {
    let doc = page(body);
    run_stylesheets(run(&doc), &doc, &[stylesheet_parse::parse_stylesheet(css)])
}

fn verdicts(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

// --- Vermerke -------------------------------------------------------------

#[test]
fn without_stylesheets_not_run_with_stylesheets_run() {
    let doc = page("<p>Text</p>");
    let without = run(&doc);
    for id in [
        "focus/outline-removed",
        "motion/reduced-motion-ignored",
        "orientation/content-hidden",
        "text/justified",
        "text/line-height-tight",
    ] {
        let r = without.rule_runs.iter().find(|r| r.rule_id == id).unwrap();
        assert!(!r.did_run(), "{id}");
        assert_eq!(r.not_run, Some(NotRun::CapabilityMissing), "{id}");
    }
    let with = run_stylesheets(without, &doc, &[]);
    for id in ["focus/outline-removed", "text/justified"] {
        let runs: Vec<_> = with.rule_runs.iter().filter(|r| r.rule_id == id).collect();
        assert_eq!(runs.len(), 1, "{id}");
        assert!(runs[0].did_run(), "{id}");
    }
    assert!(
        with.findings
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
fn corpus_media_and_motion() {
    let r = check(MEDIA_AND_MOTION_BODY, MEDIA_AND_MOTION_CSS);
    assert_eq!(
        verdicts(&r, "focus/outline-removed"),
        [(Outcome::Fail, Severity::High)]
    );
    assert_eq!(
        verdicts(&r, "motion/reduced-motion-ignored"),
        [(Outcome::Review, Severity::Medium)]
    );
    assert_eq!(
        verdicts(&r, "orientation/content-hidden"),
        [(Outcome::Review, Severity::Medium)]
    );
}

#[test]
fn focus_restored_for_keyboard_sueddeutsche_de() {
    let css = "[data-whatinput] *:focus{outline:none;}
               [data-whatintent='keyboard'] *:focus{outline:2px solid #009990;}";
    assert!(verdicts(&check("<button>x</button>", css), "focus/outline-removed").is_empty());
}

#[test]
fn replacement_or_focus_visible_withdraw_the_finding() {
    for css in [
        "a:focus { outline: none } button:focus { box-shadow: 0 0 0 3px #005fcc }",
        "a:focus { outline: 0 } a:focus-visible { outline: 2px solid }",
        ".btn:focus:not(:focus-visible) { outline: none }",
        "a:focus-within { outline: none }",
    ] {
        assert!(
            verdicts(&check("<a href=\"/\">x</a>", css), "focus/outline-removed").is_empty(),
            "{css}"
        );
    }
    let without = check(
        "<a href=\"/\">x</a>",
        "a:focus { outline-style: none } a:focus { outline: initial }",
    );
    assert_eq!(verdicts(&without, "focus/outline-removed").len(), 1);
}

// --- motion/reduced-motion-ignored ------------------------------------------

const CARD: &str = "
    .card { transition: transform 0.3s ease; }
    .card:hover { transform: translateY(-4px); }
    .slide-in { animation: slide-in 1s ease-out; }
    @keyframes slide-in { from { transform: translateX(-100%); } to { transform: none; } }
";
const CARD_BODY: &str =
    r#"<main><div class="card">Card</div><div class="slide-in">Panel</div></main>"#;

#[test]
fn corpus_reduced_motion_transform() {
    let r = check(CARD_BODY, CARD);
    assert_eq!(verdicts(&r, "motion/reduced-motion-ignored").len(), 1);
    assert!(
        r.findings[0..]
            .iter()
            .any(|f| f.message.contains("2 style rules"))
    );
}

#[test]
fn corpus_reduced_motion_override() {
    let css = format!(
        "{CARD} @media (prefers-reduced-motion: reduce) {{
            *, *::before, *::after {{ transition-duration: 0.01ms !important; }}
            .slide-in {{ animation: none; }}
        }}"
    );
    assert!(verdicts(&check(CARD_BODY, &css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn only_motion_that_can_run_on_the_page_auditmysite_712() {
    // bundesregierung.de: Keyframes eines Lade-Kreisels, den die Seite nie
    // einbindet.
    let css = ".player-spinner { animation: spin 1s infinite } @keyframes spin { to { transform: rotate(1turn) } }";
    assert!(verdicts(&check("<p>Text</p>", css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn color_opacity_and_duration_alone_move_nothing() {
    let css = "a { transition: color .2s, background-color .2s }
               .fade { transition: opacity 1s }
               .duration-200 { transition-duration: 200ms }
               .pulse { animation: pulse 2s infinite } @keyframes pulse { 50% { opacity: .5 } }";
    let body = r#"<a href="/">a</a><div class="fade duration-200 pulse">x</div>"#;
    assert!(verdicts(&check(body, css), "motion/reduced-motion-ignored").is_empty());
}

#[test]
fn transition_all_only_with_moving_state() {
    let body = r#"<a class="nav" href="/">x</a>"#;
    assert!(
        verdicts(
            &check(
                body,
                ".nav { transition: all .3s } .nav:hover { color: red }"
            ),
            "motion/reduced-motion-ignored"
        )
        .is_empty()
    );
    assert_eq!(
        verdicts(
            &check(
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
fn shielded_under_no_preference_casoon_de() {
    let css = "@media (hover:hover) and (prefers-reduced-motion:no-preference) {
        .row { transition: opacity .6s, transform .6s; transform: translateY(24px) } }";
    assert!(
        verdicts(
            &check(r#"<div class="row">x</div>"#, css),
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
    assert!(verdicts(&check("<p>x</p>", css), "orientation/content-hidden").is_empty());
}

#[test]
fn dialog_asks_to_rotate_welt_de() {
    let css = "@media only screen and (max-width: 767px) and (orientation: landscape) and (max-height: 375px) {
        .has-orientation-notification .c-modal__content-body { display: none } }";
    let open = r#"<div class="c-modal has-orientation-notification"><div class="c-modal__content-body">Artikel</div></div>"#;
    assert_eq!(
        verdicts(&check(open, css), "orientation/content-hidden").len(),
        1
    );
    // Ohne den Dialog im Dokument trifft die Regel nichts.
    assert!(verdicts(&check("<p>x</p>", css), "orientation/content-hidden").is_empty());
}

#[test]
fn whole_content_is_hard() {
    let css = "@media (orientation: portrait) { body { transform: rotate(90deg) } }";
    assert_eq!(
        verdicts(&check("<p>x</p>", css), "orientation/content-hidden"),
        [(Outcome::Review, Severity::High)]
    );
}

// --- text/justified, text/line-height-tight ---------------------------------

#[test]
fn corpus_text_and_layout() {
    let css = ".justified { text-align: justify; } .tight-box { line-height: 1.1; }";
    let body = r#"<main><p class="justified">This paragraph uses justified text alignment.</p>
        <div class="tight-box">Clipped box</div><p>Die Bundesrepublik</p></main>"#;
    let r = check(body, css);
    assert_eq!(
        verdicts(&r, "text/justified"),
        [(Outcome::Review, Severity::Low)]
    );
    // Die enge Zeilenhöhe trifft ein div, keinen Absatz.
    assert!(verdicts(&r, "text/line-height-tight").is_empty());
}

#[test]
fn normalize_is_overridden_by_the_paragraphs() {
    let css = "html { line-height: 1.15 } .prose { line-height: 1.75 }";
    let body = r#"<article class="prose"><p>Absatz</p></article>"#;
    assert!(verdicts(&check(body, css), "text/line-height-tight").is_empty());
}

#[test]
fn govuk_running_text_below_one_and_a_half() {
    // gov.uk: .govuk-body { line-height: 1.3157894737 } (25px auf 19px).
    let css = "html { line-height: 1.15 } .govuk-body { line-height: 1.3157894737 }";
    let r = check(r#"<p class="govuk-body">Text</p>"#, css);
    assert_eq!(
        verdicts(&r, "text/line-height-tight"),
        [(Outcome::Review, Severity::Low)]
    );
    assert!(
        r.findings
            .iter()
            .any(|f| f.message.contains("1.3157894737"))
    );
}

#[test]
fn line_height_in_pixels_stays_unjudged() {
    let r = check("<p>Text</p>", "p { line-height: 18px }");
    assert!(verdicts(&r, "text/line-height-tight").is_empty());
}

/// auditmysite#743: Chrome schreibt die Kurzschreibweise in `cssText` voll aus,
/// mit `none` als Füllmodus vor dem Namen. Korpusfall `media_and_motion`, wie
/// ihn auditmysite aus `document.styleSheets` liest.
#[test]
fn spelled_out_shorthand_from_chrome() {
    let css = ".spinner { animation: 2s linear 0s infinite normal none running spin; }
               @keyframes spin { 100% { transform: rotate(360deg); } }";
    let r = check(r#"<div class="spinner">Loading…</div>"#, css);
    assert_eq!(verdicts(&r, "motion/reduced-motion-ignored").len(), 1);
    // Ein Name, der wie ein Schlüsselwort aussieht, bleibt Name, sobald die
    // Eigenschaft schon vergeben ist.
    let double = ".x { animation: 1s forwards both } @keyframes both { to { left: 10px } }";
    assert_eq!(
        verdicts(
            &check(r#"<div class="x">x</div>"#, double),
            "motion/reduced-motion-ignored"
        )
        .len(),
        1
    );
    // `animation: none` ist keine Animation.
    let no = ".x { animation: none } @keyframes none { to { left: 10px } }";
    assert!(
        verdicts(
            &check(r#"<div class="x">x</div>"#, no),
            "motion/reduced-motion-ignored"
        )
        .is_empty()
    );
}
