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
    measures_hidden: bool,
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

fn make_color(hex: &str) -> Color {
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
            color: Some(make_color(node.attr("data-fg").unwrap_or("000000"))),
            background_color: match node.attr("data-bg") {
                Some("none") => None,
                b => Some(make_color(b.unwrap_or("ffffff"))),
            },
            font_size_px: Some(node.attr("data-px").map_or(16.0, |p| p.parse().unwrap())),
            font_weight: Some(400),
            display: Some("block".into()),
            visibility: Some("visible".into()),
            appearance: node.attr("data-appearance").map(Into::into),
            border_top_style: node.attr("data-border").map(|_| "solid".into()),
            border_top_width_px: node.attr("data-border").map(|_| 1.0),
            border_top_color: node.attr("data-border").map(make_color),
            own_background_color: node.attr("data-own").map(make_color),
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
        self.measures_hidden.then(|| node.has_attr("data-hidden"))
    }

    fn sampled_backdrop<'n>(&'n self, node: Self::N<'n>) -> Option<Backdrop> {
        node.attr("data-backdrop").map(sampling)
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
fn sampling(kind: &str) -> Backdrop {
    let mix = |a: (u8, u8, u8), b: (u8, u8, u8), t: f64| {
        let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
        lum(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
    };
    let mut l: Vec<f64> = (0..90)
        .map(|i| {
            let t = f64::from(i) / 89.0;
            match kind {
                "dunkel" => mix((15, 23, 42), (30, 41, 59), t),
                "hell" => mix((240, 240, 240), (255, 255, 255), t),
                // Dunkel bis 65 %, danach Weiß.
                _ if t < 0.65 => lum(15, 23, 42),
                _ => 1.0,
            }
        })
        .collect();
    l.extend(std::iter::repeat_n(1.0, 10));
    Backdrop { luminance: l }
}

fn page(element: impl FnOnce(a11y_dom::ArenaBuilder) -> a11y_dom::ArenaBuilder) -> Arena {
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

fn check(arena: &Arena, measures_hidden: bool) -> Report {
    run_with_rendering(&Host {
        doc: arena,
        measures_hidden,
    })
}

fn verdicts(r: &Report, id: &str) -> Vec<Outcome> {
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
fn visually_hidden_text_govuk_search_button() {
    let arena = page(|b| {
        b.open("button")
            .attr("class", "gem-c-search__submit")
            .attr("data-fg", "1d70b8")
            .attr("data-bg", "d2e2f1")
            .attr("data-hidden", "")
            .text("Search GOV.UK")
            .close()
    });
    assert!(verdicts(&check(&arena, true), "contrast/text-insufficient").is_empty());
    // Misst der Host es nicht, bleibt es beim Verstoß wie bisher.
    assert_eq!(
        verdicts(&check(&arena, false), "contrast/text-insufficient"),
        [Outcome::Fail]
    );
}

/// auditmysite #716: Text unter einem Cookie-Banner ist nicht bestimmbar.
#[test]
fn covered_text_is_not_determinable() {
    let arena = page(|b| {
        b.open("p")
            .attr("data-fg", "777777")
            .attr("data-covered", "")
            .text("Hero text under a cookie banner")
            .close()
    });
    let r = check(&arena, true);
    assert!(verdicts(&r, "contrast/text-insufficient").is_empty());
    assert_eq!(
        verdicts(&r, "contrast/text-undetermined"),
        [Outcome::Untested]
    );
}

/// gov.uk-Linkfarbe #1d70b8 auf Weiß: 5,17:1 — besteht AA, verfehlt AAA.
#[test]
fn enhanced_contrast_govuk_link_color() {
    let arena = page(|b| {
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
    let r = check(&arena, true);
    assert_eq!(verdicts(&r, "contrast/text-enhanced"), [Outcome::Fail]);
    assert_eq!(verdicts(&r, "contrast/text-insufficient"), [Outcome::Fail]);
}

/// Ein Host, der von `Layout` nur `obscured` misst: `focus/obscured` läuft,
/// die übrigen Heuristiken stehen mit `NotRun::CapabilityMissing` im Bericht,
/// ohne Befund.
#[test]
fn partial_layout_leaves_unmeasured_unchecked() {
    let arena = page(|b| b.open("p").text("Text").close());
    let r = check(&arena, true);
    for id in [
        "order/visual-mismatch",
        "motion/infinite-animation",
        "reflow/min-width",
        "keyboard/pointer-only",
    ] {
        assert!(verdicts(&r, id).is_empty(), "{id}");
        let run = r.rule_runs.iter().find(|x| x.rule_id == id).unwrap();
        assert_eq!(
            run.not_run,
            Some(a11y_report::NotRun::CapabilityMissing),
            "{id}"
        );
    }
    assert!(verdicts(&r, "focus/obscured").is_empty());
    let run = r
        .rule_runs
        .iter()
        .find(|x| x.rule_id == "focus/obscured")
        .unwrap();
    assert!(run.did_run());
}

fn gradient(kind: &'static str) -> Arena {
    page(move |b| {
        b.open("p")
            .attr("data-fg", "ffffff")
            .attr("data-bg", "none")
            .attr("data-backdrop", kind)
            .text("Weißer Text über einem Verlauf")
            .close()
    })
}

/// `image_contrast.html`, `.dark-gradient-pass`: Weiß auf dunklem Verlauf.
#[test]
fn sampled_darker_gradient_passes() {
    let r = check(&gradient("dunkel"), true);
    for id in ["contrast/text-insufficient", "contrast/text-undetermined"] {
        assert!(verdicts(&r, id).is_empty(), "{id}");
    }
}

/// `.light-gradient-fail`: Weiß auf hellem Verlauf, schon der Median verfehlt.
#[test]
fn sampled_lighter_gradient_fails_with_evidence() {
    let r = check(&gradient("hell"), true);
    assert_eq!(verdicts(&r, "contrast/text-insufficient"), [Outcome::Fail]);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "contrast/text-insufficient")
        .unwrap();
    let fields: Vec<_> = f
        .evidence
        .iter()
        .filter_map(|e| e.field.as_deref())
        .collect();
    assert_eq!(fields, ["contrast_ratio", "required_ratio"]);
}

/// `.split-gradient-warn`: Der Median besteht, das 40. Perzentil nicht.
#[test]
fn sampled_split_gradient_is_hint() {
    let r = check(&gradient("geteilt"), true);
    assert!(verdicts(&r, "contrast/text-insufficient").is_empty());
    assert_eq!(
        verdicts(&r, "contrast/text-undetermined"),
        [Outcome::Review]
    );
}

/// Ohne Abtastung bleibt Text über einem Verlauf nicht bestimmbar.
#[test]
fn without_sampling_not_determinable() {
    let arena = page(|b| {
        b.open("p")
            .attr("data-fg", "ffffff")
            .attr("data-bg", "none")
            .text("Text")
            .close()
    });
    assert_eq!(
        verdicts(&check(&arena, true), "contrast/text-undetermined"),
        [Outcome::Untested]
    );
}

// --- 1.4.11 Bedienelemente (auditmysite `non_text_contrast_css`) ------------

fn checkbox_arena(attrs: &[(&str, &str)]) -> Arena {
    page(|b| {
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
fn paler_border_of_own_box_fails() {
    let r = check(
        &checkbox_arena(&[("data-appearance", "none"), ("data-border", "cccccc")]),
        false,
    );
    assert_eq!(
        verdicts(&r, "contrast/control-insufficient"),
        [Outcome::Fail]
    );
}

#[test]
fn darker_border_passes() {
    let r = check(
        &checkbox_arena(&[("data-appearance", "none"), ("data-border", "767676")]),
        false,
    );
    assert!(verdicts(&r, "contrast/control-insufficient").is_empty());
}

#[test]
fn without_border_own_surface_counts() {
    let r = check(
        &checkbox_arena(&[("data-appearance", "none"), ("data-own", "eeeeee")]),
        false,
    );
    assert_eq!(
        verdicts(&r, "contrast/control-insufficient"),
        [Outcome::Fail]
    );
}

/// Ein natives Kästchen zeichnet der Browser; sein Kontrast steht in keinem
/// berechneten Stil.
#[test]
fn native_box_is_not_measured() {
    let r = check(&checkbox_arena(&[("data-border", "cccccc")]), false);
    assert!(verdicts(&r, "contrast/control-insufficient").is_empty());
    assert!(verdicts(&r, "contrast/control-undetermined").is_empty());
}

#[test]
fn without_background_behind_not_determinable() {
    let arena = page(|b| {
        b.open("div")
            .attr("data-bg", "none")
            .open("span")
            .attr("role", "switch")
            .attr("data-appearance", "none")
            .attr("data-border", "cccccc")
            .close()
            .close()
    });
    let r = check(&arena, false);
    assert_eq!(
        verdicts(&r, "contrast/control-undetermined"),
        [Outcome::Untested]
    );
    assert!(verdicts(&r, "contrast/control-insufficient").is_empty());
}
