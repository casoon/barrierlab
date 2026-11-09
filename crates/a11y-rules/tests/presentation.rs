//! Darstellungsregeln mit gemessenen Werten (casoon/barrierlab#21, zweiter
//! Teil): `lists/role-redundant`, `color/link-indistinct`,
//! `keyboard/scrollable-region-not-focusable`.
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`redundant_role_list_style`, `link_in_text_block_context`,
//! `scrollable_region_focusable`). Der Host hier rechnet die Stile so aus,
//! wie Chrome es für diese Seiten täte: `display` nach dem UA-Stylesheet,
//! die Werte aus dem CSS des Korpusfalls über `data-*`-Attribute.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, Color, ComputedStyle, Document, Layout, Node, Rect, Rendering,
    Semantics, elements,
};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run_full;
use accname::IdIndex;
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

struct Host<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    /// Ob der Host die neuen Felder liefert.
    full: bool,
}

impl<'a> Host<'a> {
    fn new(doc: &'a Arena, full: bool) -> Self {
        Host {
            ids: IdIndex::build(doc.root()),
            doc,
            full,
        }
    }
}

impl Document for Host<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for Host<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }
}

/// `display` nach dem UA-Stylesheet, soweit diese Fälle es brauchen.
fn ua_display(n: ArenaNode<'_>) -> &'static str {
    match n.local_name() {
        "a" | "span" | "strong" | "em" | "b" | "i" | "code" => "inline",
        "li" => "list-item",
        "head" | "title" | "style" | "script" => "none",
        _ => "block",
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        let mut style = ComputedStyle {
            color: Some(Color {
                r: 34,
                g: 34,
                b: 34,
                a: 255,
            }),
            background_color: None,
            font_size_px: Some(16.0),
            font_weight: Some(400),
            display: Some(node.attr("data-display").unwrap_or(ua_display(node)).into()),
            visibility: Some("visible".into()),
            ..Default::default()
        };
        if self.full {
            style.list_style_type = Some(
                node.attr("data-list-style")
                    .unwrap_or(match node.local_name() {
                        "ol" => "decimal",
                        _ => "disc",
                    })
                    .into(),
            );
            style.text_decoration_line = Some(node.attr("data-deco").unwrap_or("none").into());
            style.font_style = Some("normal".into());
            style.font_family = Some("sans-serif".into());
            style.border_bottom_style = Some("none".into());
            if let Some(w) = node.attr("data-weight") {
                style.font_weight = Some(w.parse().unwrap());
            }
        }
        Some(style)
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 30.0,
        })
    }

    fn layout<'n>(&'n self, _node: Self::N<'n>) -> Option<Layout> {
        Some(Layout {
            focus_visible: Some(true),
            ..Default::default()
        })
    }

    fn scroll_overflow_px<'n>(&'n self, node: Self::N<'n>) -> Option<f32> {
        self.full
            .then(|| node.attr("data-scroll").map_or(0.0, |s| s.parse().unwrap()))
    }
}

fn check(body: &str, full: bool) -> (Arena, Report) {
    let doc = page(body);
    let r = run_full(&Host::new(&doc, full));
    (doc, r)
}

/// Die `id`-Attribute der Elemente, an denen `rule` meldet.
fn findings_at(doc: &Arena, r: &Report, rule: &str) -> Vec<String> {
    let node: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule && f.outcome != Outcome::Untested)
        .filter_map(|f| f.location.node.clone())
        .collect();
    elements(doc)
        .filter(|n| node.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

/// `id` steht mit `NotRun::CapabilityMissing` und Grund im Bericht.
fn assert_not_run(r: &Report, id: &str) {
    let run = r.rule_runs.iter().find(|x| x.rule_id == id).unwrap();
    assert_eq!(
        run.not_run,
        Some(a11y_report::NotRun::CapabilityMissing),
        "{id}"
    );
    assert!(run.reason.is_some(), "{id}");
}

fn verdicts(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

// --- lists/role-redundant ---------------------------------------------------

const LISTS: &str = r#"<main>
    <ul role="list" class="plain" id="plain-list" data-list-style="none"><li>First</li><li>Second</li></ul>
    <ol role="list" id="numbered-list"><li>One</li><li>Two</li></ol>
    <button type="button" role="button" id="send-button">Send</button></main>"#;

#[test]
fn corpus_redundant_role_list_style_auditmysite_644() {
    let (doc, r) = check(LISTS, true);
    assert_eq!(
        findings_at(&doc, &r, "lists/role-redundant"),
        ["numbered-list"]
    );
    assert_eq!(
        verdicts(&r, "lists/role-redundant"),
        [(Outcome::Review, Severity::Low)]
    );
    // Die Rolle stimmt, sie ist nur überflüssig: Best Practice ohne 4.1.2
    // (auditmysite#779).
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "lists/role-redundant")
        .unwrap();
    assert!(f.wcag.is_empty(), "{:?}", f.wcag);
    assert_eq!(f.tags, ["best-practice"]);
}

#[test]
fn without_list_style_type_unchecked() {
    let (_, r) = check(LISTS, false);
    assert!(verdicts(&r, "lists/role-redundant").is_empty());
    assert_not_run(&r, "lists/role-redundant");
}

// --- color/link-indistinct --------------------------------------------------

/// Korpusfall `link_in_text_block_context` (auditmysite#710): `a {
/// text-decoration: none }`, Listen ohne Zeichen.
const LINKS: &str = r#"
    <header>
      <div class="logo"><a id="logo-link" href="/">Government Portal</a></div>
      <div class="header-links"><a id="header-help" href="/help">Help</a> | <a id="header-contact" href="/contact">Contact</a></div>
      <nav><ul><li><a id="nav-home" href="/">Home</a></li><li><a id="nav-services" href="/services">Services</a></li></ul></nav>
    </header>
    <main>
      <h1>Link in text block</h1>
      <ul class="exposed-links"><li><a id="list-link" href="/topics">Topics</a></li><li><a id="list-link-2" href="/news">News</a></li></ul>
      <p>Please read our <a id="inline-link" href="/policy">privacy policy</a> before you submit the form.</p>
    </main>
    <footer>
      <div id="footer-links"><a id="footer-imprint" href="/imprint">Imprint</a> · <a id="footer-privacy" href="/privacy">Privacy</a> · <a id="footer-a11y" href="/accessibility">Accessibility</a></div>
      <ul><li><a id="footer-list-link" href="/sitemap">Sitemap</a></li></ul>
    </footer>"#;

#[test]
fn corpus_link_in_text_block_context_auditmysite_710() {
    let (doc, r) = check(LINKS, true);
    assert_eq!(
        findings_at(&doc, &r, "color/link-indistinct"),
        ["inline-link"]
    );
    assert_eq!(
        verdicts(&r, "color/link-indistinct"),
        [(Outcome::Fail, Severity::Medium)]
    );
}

#[test]
fn underline_or_weight_set_apart() {
    let body = r#"<p>Please read our <a id="u" href="/p" data-deco="underline">policy</a> and the
        <a id="b" href="/t" data-weight="700">terms</a> before you go.</p>"#;
    let (doc, r) = check(body, true);
    assert!(findings_at(&doc, &r, "color/link-indistinct").is_empty());
}

#[test]
fn link_as_block_is_not_running_text() {
    let body = r#"<p>Some running text here <a id="card" href="/c" data-display="block">card</a> more words.</p>"#;
    let (doc, r) = check(body, true);
    assert!(findings_at(&doc, &r, "color/link-indistinct").is_empty());
}

#[test]
fn without_text_markup_unchecked() {
    let (_, r) = check(LINKS, false);
    assert!(verdicts(&r, "color/link-indistinct").is_empty());
    assert_not_run(&r, "color/link-indistinct");
}

// --- keyboard/scrollable-region-not-focusable -------------------------------

/// Korpusfall `scrollable_region_focusable` (auditmysite#717, gov.si
/// `div.menus`): `.box { height: 60px; overflow: auto }`, `.tall { height:
/// 400px }` — 340 px Überhang.
const AREAS: &str = r##"<main><h1>Scrollable regions</h1>
    <div class="box menus" id="menus" data-scroll="340"><div class="tall">Only text that needs scrolling to read.</div></div>
    <div class="box" id="terms" tabindex="0" role="region" aria-label="Terms" data-scroll="340"><div class="tall">Terms text.</div></div>
    <div class="box" id="links" data-scroll="340"><div class="tall">Links: <a href="#a">first link</a></div></div>
    <div class="box" id="clipped" style="overflow: hidden"><div class="tall">Clipped text.</div></div>
    <div class="box" id="short">Short text.</div>
    <div class="box" id="knapp" data-scroll="12">Almost fits.</div>
    <div class="box" id="leer" data-scroll="340"><div class="tall"></div></div>
    <div inert><div class="box" id="inert" data-scroll="340">Inert text.</div></div></main>"##;

#[test]
fn corpus_scrollable_region_focusable_auditmysite_717() {
    let (doc, r) = check(AREAS, true);
    assert_eq!(
        findings_at(&doc, &r, "keyboard/scrollable-region-not-focusable"),
        ["menus"]
    );
    assert_eq!(
        verdicts(&r, "keyboard/scrollable-region-not-focusable"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn without_scroll_overhang_unchecked() {
    let (_, r) = check(AREAS, false);
    assert!(verdicts(&r, "keyboard/scrollable-region-not-focusable").is_empty());
    assert_not_run(&r, "keyboard/scrollable-region-not-focusable");
}
