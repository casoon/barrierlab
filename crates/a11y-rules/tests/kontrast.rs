//! Kontrast nach WCAG 1.4.3 und 1.4.6 und feldweise optionales `Layout`
//! (casoon/barrierlab#47).
//!
//! Die Fälle stammen aus dem Vergleich in casoon/auditmysite#698 (gov.uk,
//! 2026-10-05) und aus auditmysites eigener Kontrastregel (#716: überdeckter
//! Text wird Hinweis, kein Verstoß).

use a11y_dom::{
    Arena, ArenaNode, Backdrop, Color, ComputedStyle, Document, Layout, Node, Rect, Rendering,
};
use a11y_report::{Outcome, Report};
use a11y_rules::run_with_rendering;

/// `data-fg`/`data-bg` als `rrggbb`, `data-px` die Schriftgröße,
/// `data-hidden` für optisch verborgen, `data-covered` für überdeckt;
/// `data-appearance`, `data-border` (Farbe des oberen Rahmens, 1 px) und
/// `data-own` (eigene Fläche) für selbst gezeichnete Bedienelemente.
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
            background_color: match node.attr("data-bg") {
                Some("none") => None,
                b => Some(farbe(b.unwrap_or("ffffff"))),
            },
            font_size_px: Some(node.attr("data-px").map_or(16.0, |p| p.parse().unwrap())),
            font_weight: Some(400),
            display: Some("block".into()),
            visibility: Some("visible".into()),
            appearance: node.attr("data-appearance").map(Into::into),
            border_top_style: node.attr("data-border").map(|_| "solid".into()),
            border_top_width_px: node.attr("data-border").map(|_| 1.0),
            border_top_color: node.attr("data-border").map(farbe),
            own_background_color: node.attr("data-own").map(farbe),
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

    fn sampled_backdrop<'n>(&'n self, node: Self::N<'n>) -> Option<Backdrop> {
        node.attr("data-backdrop").map(abtastung)
    }
}

fn lum(r: u8, g: u8, b: u8) -> f64 {
    let k = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * k(r) + 0.7152 * k(g) + 0.0722 * k(b)
}

/// Eine Abtastung wie aus dem Screenshot: 100 Pixel über den Verlauf von
/// links nach rechts, dazu 10 % Glyphenpixel in Weiß. Die Verläufe stammen
/// aus auditmysites `tests/fixtures/image_contrast.html`.
fn abtastung(art: &str) -> Backdrop {
    let mischen = |a: (u8, u8, u8), b: (u8, u8, u8), t: f64| {
        let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
        lum(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
    };
    let mut l: Vec<f64> = (0..90)
        .map(|i| {
            let t = f64::from(i) / 89.0;
            match art {
                "dunkel" => mischen((15, 23, 42), (30, 41, 59), t),
                "hell" => mischen((240, 240, 240), (255, 255, 255), t),
                // Dunkel bis 65 %, danach Weiß.
                _ if t < 0.65 => lum(15, 23, 42),
                _ => 1.0,
            }
        })
        .collect();
    l.extend(std::iter::repeat_n(1.0, 10));
    Backdrop { luminance: l }
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

fn verlauf(art: &'static str) -> Arena {
    seite(move |b| {
        b.open("p")
            .attr("data-fg", "ffffff")
            .attr("data-bg", "none")
            .attr("data-backdrop", art)
            .text("Weißer Text über einem Verlauf")
            .close()
    })
}

/// `image_contrast.html`, `.dark-gradient-pass`: Weiß auf dunklem Verlauf.
#[test]
fn abgetastet_dunkler_verlauf_besteht() {
    let r = pruefe(&verlauf("dunkel"), true);
    for id in ["contrast/text-insufficient", "contrast/text-undetermined"] {
        assert!(urteile(&r, id).is_empty(), "{id}");
    }
}

/// `.light-gradient-fail`: Weiß auf hellem Verlauf, schon der Median verfehlt.
#[test]
fn abgetastet_heller_verlauf_verfehlt_mit_beleg() {
    let r = pruefe(&verlauf("hell"), true);
    assert_eq!(urteile(&r, "contrast/text-insufficient"), [Outcome::Fail]);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "contrast/text-insufficient")
        .unwrap();
    let felder: Vec<_> = f
        .evidence
        .iter()
        .filter_map(|e| e.field.as_deref())
        .collect();
    assert_eq!(felder, ["contrast_ratio", "required_ratio"]);
}

/// `.split-gradient-warn`: Der Median besteht, das 40. Perzentil nicht.
#[test]
fn abgetastet_geteilter_verlauf_ist_hinweis() {
    let r = pruefe(&verlauf("geteilt"), true);
    assert!(urteile(&r, "contrast/text-insufficient").is_empty());
    assert_eq!(urteile(&r, "contrast/text-undetermined"), [Outcome::Review]);
}

/// Ohne Abtastung bleibt Text über einem Verlauf nicht bestimmbar.
#[test]
fn ohne_abtastung_nicht_bestimmbar() {
    let arena = seite(|b| {
        b.open("p")
            .attr("data-fg", "ffffff")
            .attr("data-bg", "none")
            .text("Text")
            .close()
    });
    assert_eq!(
        urteile(&pruefe(&arena, true), "contrast/text-undetermined"),
        [Outcome::Untested]
    );
}

// --- 1.4.11 Bedienelemente (auditmysite `non_text_contrast_css`) ------------

fn kaestchen(attrs: &[(&str, &str)]) -> Arena {
    seite(|b| {
        let mut b = b.open("input").attr("type", "checkbox");
        for (k, v) in attrs {
            b = b.attr(k, v);
        }
        b.close()
    })
}

/// #cccccc auf Weiß sind 1,61:1 — das blasse Kästchen, das axe und Pa11y in
/// der Praxis finden.
#[test]
fn blasser_rahmen_eines_eigenen_kaestchens_verfehlt() {
    let r = pruefe(
        &kaestchen(&[("data-appearance", "none"), ("data-border", "cccccc")]),
        false,
    );
    assert_eq!(
        urteile(&r, "contrast/control-insufficient"),
        [Outcome::Fail]
    );
}

#[test]
fn dunkler_rahmen_besteht() {
    let r = pruefe(
        &kaestchen(&[("data-appearance", "none"), ("data-border", "767676")]),
        false,
    );
    assert!(urteile(&r, "contrast/control-insufficient").is_empty());
}

#[test]
fn ohne_rahmen_zaehlt_die_eigene_flaeche() {
    let r = pruefe(
        &kaestchen(&[("data-appearance", "none"), ("data-own", "eeeeee")]),
        false,
    );
    assert_eq!(
        urteile(&r, "contrast/control-insufficient"),
        [Outcome::Fail]
    );
}

/// Ein natives Kästchen zeichnet der Browser; sein Kontrast steht in keinem
/// berechneten Stil.
#[test]
fn natives_kaestchen_wird_nicht_gemessen() {
    let r = pruefe(&kaestchen(&[("data-border", "cccccc")]), false);
    assert!(urteile(&r, "contrast/control-insufficient").is_empty());
    assert!(urteile(&r, "contrast/control-undetermined").is_empty());
}

#[test]
fn ohne_hintergrund_dahinter_nicht_bestimmbar() {
    let arena = seite(|b| {
        b.open("div")
            .attr("data-bg", "none")
            .open("span")
            .attr("role", "switch")
            .attr("data-appearance", "none")
            .attr("data-border", "cccccc")
            .close()
            .close()
    });
    let r = pruefe(&arena, false);
    assert_eq!(
        urteile(&r, "contrast/control-undetermined"),
        [Outcome::Untested]
    );
    assert!(urteile(&r, "contrast/control-insufficient").is_empty());
}
