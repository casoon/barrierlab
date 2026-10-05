//! Kontrast nach WCAG 1.4.3 und 1.4.6 und feldweise optionales `Layout`
//! (casoon/barrierlab#47).
//!
//! Die Fälle stammen aus dem Vergleich in casoon/auditmysite#698 (gov.uk,
//! 2026-10-05) und aus auditmysites eigener Kontrastregel (#716: überdeckter
//! Text wird Hinweis, kein Verstoß).

use a11y_dom::{Arena, ArenaNode, Color, ComputedStyle, Document, Layout, Node, Rect, Rendering};
use a11y_report::{Outcome, Report};
use a11y_rules::run_with_rendering;

/// `data-fg`/`data-bg` als `rrggbb`, `data-px` die Schriftgröße,
/// `data-hidden` für optisch verborgen, `data-covered` für überdeckt.
struct Host<'a> {
    doc: &'a Arena,
    /// Ob der Host `visually_hidden` misst.
    misst_verborgen: bool,
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

fn farbe(hex: &str) -> Color {
    let k = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
    Color {
        r: k(0),
        g: k(2),
        b: k(4),
        a: 255,
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            color: Some(farbe(node.attr("data-fg").unwrap_or("000000"))),
            background_color: Some(farbe(node.attr("data-bg").unwrap_or("ffffff"))),
            font_size_px: Some(node.attr("data-px").map_or(16.0, |p| p.parse().unwrap())),
            font_weight: Some(400),
            display: Some("block".into()),
            visibility: Some("visible".into()),
            ..Default::default()
        })
    }

    fn bounds<'n>(&'n self, _: Self::N<'n>) -> Option<Rect> {
        Some(Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 30.0,
        })
    }

    /// Nur ein Feld gemessen: ob die Mitte überdeckt ist.
    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        Some(Layout {
            obscured: Some(node.has_attr("data-covered")),
            ..Default::default()
        })
    }

    fn visually_hidden<'n>(&'n self, node: Self::N<'n>) -> Option<bool> {
        self.misst_verborgen.then(|| node.has_attr("data-hidden"))
    }
}

fn seite(element: impl FnOnce(a11y_dom::ArenaBuilder) -> a11y_dom::ArenaBuilder) -> Arena {
    element(
        Arena::builder()
            .open("html")
            .attr("lang", "en")
            .open("head")
            .open("title")
            .text("T")
            .close()
            .close()
            .open("body"),
    )
    .close()
    .close()
    .build()
}

fn pruefe(arena: &Arena, misst_verborgen: bool) -> Report {
    run_with_rendering(&Host {
        doc: arena,
        misst_verborgen,
    })
}

fn urteile(r: &Report, id: &str) -> Vec<Outcome> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(|f| f.outcome)
        .collect()
}

/// gov.uk: Der Suchknopf trägt „Search GOV.UK" als Text, versteckt per
/// `text-indent: -5000px` und `overflow: hidden`; sichtbar ist nur die Lupe.
/// #1d70b8 auf #d2e2f1 sind 3,91:1 — für Text, den niemand sieht.
#[test]
fn optisch_verborgener_text_govuk_suchknopf() {
    let arena = seite(|b| {
        b.open("button")
            .attr("class", "gem-c-search__submit")
            .attr("data-fg", "1d70b8")
            .attr("data-bg", "d2e2f1")
            .attr("data-hidden", "")
            .text("Search GOV.UK")
            .close()
    });
    assert!(urteile(&pruefe(&arena, true), "contrast/text-insufficient").is_empty());
    // Misst der Host es nicht, bleibt es beim Verstoß wie bisher.
    assert_eq!(
        urteile(&pruefe(&arena, false), "contrast/text-insufficient"),
        [Outcome::Fail]
    );
}

/// auditmysite #716: Text unter einem Cookie-Banner ist nicht bestimmbar.
#[test]
fn ueberdeckter_text_ist_nicht_bestimmbar() {
    let arena = seite(|b| {
        b.open("p")
            .attr("data-fg", "777777")
            .attr("data-covered", "")
            .text("Hero text under a cookie banner")
            .close()
    });
    let r = pruefe(&arena, true);
    assert!(urteile(&r, "contrast/text-insufficient").is_empty());
    assert_eq!(
        urteile(&r, "contrast/text-undetermined"),
        [Outcome::Untested]
    );
}

/// gov.uk-Linkfarbe #1d70b8 auf Weiß: 5,17:1 — besteht AA, verfehlt AAA.
#[test]
fn erhoehter_kontrast_govuk_linkfarbe() {
    let arena = seite(|b| {
        b.open("a")
            .attr("href", "/")
            .attr("data-fg", "1d70b8")
            .text("Benefits")
            .close()
            // Großer Text braucht für AAA nur 4,5:1.
            .open("h1")
            .attr("data-fg", "1d70b8")
            .attr("data-px", "32")
            .text("Welcome")
            .close()
            // Schon AA verfehlt: steht unter 1.4.3, nicht doppelt.
            .open("p")
            .attr("data-fg", "999999")
            .text("Grey")
            .close()
    });
    let r = pruefe(&arena, true);
    assert_eq!(urteile(&r, "contrast/text-enhanced"), [Outcome::Fail]);
    assert_eq!(urteile(&r, "contrast/text-insufficient"), [Outcome::Fail]);
}

/// Ein Host, der von `Layout` nur `obscured` misst: `focus/obscured` läuft,
/// die übrigen Heuristiken melden „nicht geprüft" statt still nichts.
#[test]
fn teilweises_layout_laesst_ungemessenes_ungeprueft() {
    let arena = seite(|b| b.open("p").text("Text").close());
    let r = pruefe(&arena, true);
    for id in [
        "order/visual-mismatch",
        "motion/infinite-animation",
        "reflow/min-width",
        "keyboard/pointer-only",
    ] {
        assert_eq!(urteile(&r, id), [Outcome::Untested], "{id}");
    }
    assert!(urteile(&r, "focus/obscured").is_empty());
}
