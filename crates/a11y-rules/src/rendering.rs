//! Tier-3-Regeln: brauchen berechnete Stile und Geometrie.
//!
//! Warum das strukturell nicht geht: Die Vordergrundfarbe eines Textes steht
//! fast nie an dem Element, das den Text trägt, und die Hintergrundfarbe fast
//! nie am selben Element wie die Vordergrundfarbe. Beides entsteht erst aus
//! Kaskade, Vererbung und Transparenz — ein statischer Parser kann es nicht
//! rekonstruieren, auch nicht näherungsweise.
//!
//! Hosts ohne [`Rendering`] melden diese Regeln als `UNTESTED` — siehe
//! [`crate::run`].
//!
//! # Was der Host liefern muss
//!
//! [`ComputedStyle::background_color`] ist die **effektive** Farbe: Der Host
//! löst Transparenz über die Vorfahren auf. Kann er das nicht — weil ein
//! Hintergrundbild, ein Verlauf oder ein `background-blend-mode` im Spiel ist
//! —, liefert er `None`. Die Regel meldet dann `UNTESTED` und nicht etwa eine
//! Prüfung gegen Weiß. Eine geratene Hintergrundfarbe wäre schlimmer als keine
//! Aussage: Sie erzeugt ein `PASS`, auf das sich jemand verlässt.
//!
//! [`Rendering`]: a11y_dom::Rendering
//! [`ComputedStyle::background_color`]: a11y_dom::ComputedStyle::background_color

use a11y_dom::{
    Color, ComputedStyle, Node, NodeId, NodeKind, Rendering, Tier, ancestors, elements,
};
use a11y_report::{Evidence, Finding, Location, Severity};

use crate::heuristics;
use crate::locale::{Locale, de_text, pick, tr};
use crate::presentation;
use crate::probes;
use crate::registry::{Meta, RenderingRule};
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Relative Leuchtdichte nach WCAG 2.x, Definition „relative luminance".
fn luminance(c: Color) -> f64 {
    fn channel(v: u8) -> f64 {
        let s = f64::from(v) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// Kontrastverhältnis nach WCAG 2.x, 1,0 bis 21,0.
pub(crate) fn ratio(front: Color, back: Color) -> f64 {
    let (a, b) = (luminance(front), luminance(back));
    let (light, dark) = if a >= b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

/// Legt eine teildurchsichtige Farbe über eine deckende.
///
/// Der Host löst die Transparenz des *Hintergrunds* auf, nicht die des
/// Vordergrunds: `color: rgba(0,0,0,.5)` bleibt als solches stehen und wird
/// erst hier verrechnet.
pub(crate) fn over(front: Color, back: Color) -> Color {
    if front.a == 255 {
        return front;
    }
    let a = f64::from(front.a) / 255.0;
    let mix = |v: u8, h: u8| (f64::from(v) * a + f64::from(h) * (1.0 - a)).round() as u8;
    Color {
        r: mix(front.r, back.r),
        g: mix(front.g, back.g),
        b: mix(front.b, back.b),
        a: 255,
    }
}

/// Großer Text im Sinne von WCAG 1.4.3: ab 18 pt, oder ab 14 pt bei fett.
///
/// Gerechnet wird in CSS-Pixeln, weil der berechnete Stil sie liefert:
/// 18 pt sind 24 px, 14 pt sind 18,66 px.
fn is_large_text(style: &ComputedStyle) -> bool {
    let Some(px) = style.font_size_px else {
        return false;
    };
    let bold = style.font_weight.is_some_and(|w| w >= 700);
    px >= 24.0 || (bold && px >= 18.66)
}

/// Ob dieses Element selbst Text trägt — nicht, ob irgendwo darunter Text
/// steht. Der Kontrast gehört an das Element, dessen Farbe gilt.
fn carries_text<'a, N: Node<'a>>(n: N) -> bool {
    n.children()
        .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
}

/// Ob das Element überhaupt dargestellt wird. `display: none` und
/// `visibility: hidden` sind kein Kontrastproblem.
fn is_rendered(style: &ComputedStyle) -> bool {
    style.display.as_deref() != Some("none") && style.visibility.as_deref() != Some("hidden")
}

/// Teil eines inaktiven Bedienelements: `disabled` an einem Formularelement
/// oder einem `fieldset` darüber, oder `aria-disabled="true"`. WCAG 1.4.3,
/// 1.4.6 und 1.4.11 nehmen inaktive Bedienelemente ausdrücklich aus (ACT
/// afw4f7: „not part of a disabled widget").
fn inactive<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n).chain(ancestors(n)).any(|a| {
        a.attr("aria-disabled")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            || (a.has_attr("disabled")
                && matches!(
                    a.local_name(),
                    "button" | "input" | "select" | "textarea" | "option" | "optgroup" | "fieldset"
                ))
    })
}

/// Was eine Messung am Element ergibt.
enum Measurement {
    /// Eine Hintergrundfarbe gilt: Verhältnis und ob der Text groß ist.
    Value(f64, bool),
    /// Abgetasteter Hintergrund: Median und 40. Perzentil der Verhältnisse
    /// über die Stichprobe, und ob der Text groß ist.
    Span { median: f64, p40: f64, large: bool },
    /// Nicht bestimmbar, mit Grund.
    Undetermined(Reason),
}

enum Reason {
    /// Vorder- oder Hintergrundfarbe fehlt, und keine Abtastung hilft.
    Color,
    /// Ein fixiertes oder klebendes fremdes Element überdeckt die Mitte.
    Covered,
}

fn ratio_lum(a: f64, b: f64) -> f64 {
    let (light, dark) = if a >= b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

/// Median und 40. Perzentil der Verhältnisse zwischen Textfarbe und den
/// abgetasteten Pixeln — wie auditmysite vor der Umstellung: Die Glyphen
/// selbst liegen in der Stichprobe und ziehen die unteren Werte nach unten,
/// deshalb nicht das Minimum.
fn span(front: Color, sample: &[f64]) -> Option<(f64, f64)> {
    if sample.is_empty() {
        return None;
    }
    // Halbdurchsichtige Schrift wird gegen Weiß verrechnet; der genaue
    // Untergrund ist hier gerade unbekannt.
    let front = luminance(over(
        front,
        Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    ));
    let mut v: Vec<f64> = sample
        .iter()
        .map(|l| ratio_lum(front, l.clamp(0.0, 1.0)))
        .collect();
    v.sort_by(f64::total_cmp);
    let quantile = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    Some((quantile(0.5), quantile(0.4)))
}

/// Misst den Kontrast eines Elements, das selbst Text trägt. `None`, wenn es
/// nichts zu messen gibt: kein eigener Text, nicht dargestellt oder optisch
/// verborgen (`Rendering::visually_hidden`).
///
/// Text unter `aria-hidden` wird gemessen: WCAG 1.4.3 gilt für sichtbaren
/// Text, unabhängig vom Accessibility-Tree. auditmysite nahm ihn aus (#395).
fn measure<'d, D: Rendering>(doc: &'d D, n: D::N<'d>) -> Option<Measurement> {
    if !carries_text(n) || inactive(n) {
        return None;
    }
    let style = doc.computed_style(n)?;
    if !is_rendered(&style) || doc.visually_hidden(n) == Some(true) {
        return None;
    }
    if doc.layout(n).and_then(|l| l.obscured) == Some(true) {
        return Some(Measurement::Undetermined(Reason::Covered));
    }
    let large = is_large_text(&style);
    Some(match (style.color, style.background_color) {
        (Some(front), Some(back)) => Measurement::Value(ratio(over(front, back), back), large),
        (Some(front), None) => match doc
            .sampled_backdrop(n)
            .and_then(|b| span(front, &b.luminance))
        {
            Some((median, p40)) => Measurement::Span { median, p40, large },
            None => Measurement::Undetermined(Reason::Color),
        },
        _ => Measurement::Undetermined(Reason::Color),
    })
}

/// A number for the running text of a message: with a comma in German. The
/// evidence (`Evidence`) stays language-independent with a point.
fn number(locale: Locale, value: f64, decimals: usize) -> String {
    let point = format!("{value:.decimals$}");
    pick!(locale, point, point.replace('.', ","))
}

fn text_kind(locale: Locale, large: bool) -> &'static str {
    if large {
        pick!(locale, "large text", "großen Text")
    } else {
        pick!(locale, "normal text", "normalen Text")
    }
}

fn evidences(value: f64, threshold: f64) -> [Evidence; 2] {
    [
        Evidence::computed("contrast_ratio", format!("{value:.2}")),
        Evidence::computed("required_ratio", format!("{threshold:.1}")),
    ]
}

/// Eine Prüfung gegen eine Schwelle — für 1.4.3 und 1.4.6 dieselbe.
enum Verdict {
    Passes,
    Fails(f64),
    /// Abgetastet: Median besteht, das 40. Perzentil nicht.
    Borderline {
        median: f64,
        p40: f64,
    },
}

fn verdicts(
    m: &Measurement,
    threshold_small: f64,
    threshold_large: f64,
) -> Option<(Verdict, f64, bool)> {
    // Ohne Toleranz: Understanding 1.4.3 verlangt, das Verhältnis nicht zu
    // runden — 4,499:1 erreicht 4,5:1 nicht.
    let narrowly = |w: f64, s: f64| w < s;
    match *m {
        Measurement::Value(w, large) => {
            let s = if large {
                threshold_large
            } else {
                threshold_small
            };
            Some((
                if narrowly(w, s) {
                    Verdict::Fails(w)
                } else {
                    Verdict::Passes
                },
                s,
                large,
            ))
        }
        Measurement::Span { median, p40, large } => {
            let s = if large {
                threshold_large
            } else {
                threshold_small
            };
            let u = if narrowly(median, s) {
                Verdict::Fails(median)
            } else if narrowly(p40, s) {
                Verdict::Borderline { median, p40 }
            } else {
                Verdict::Passes
            };
            Some((u, s, large))
        }
        Measurement::Undetermined(_) => None,
    }
}

fn borderline(
    id: &str,
    median: f64,
    p40: f64,
    threshold: f64,
    wcag: &str,
    locale: Locale,
    n: NodeId,
) -> Finding {
    let (median_s, p40_s, threshold_s) = (
        number(locale, median, 2),
        number(locale, p40, 2),
        number(locale, threshold, 1),
    );
    Finding::review(
        id,
        tr!(
            locale,
            "The text lies over an image or gradient. Over most of its box the contrast \
             reaches {median_s}:1, in darker or lighter parts only {p40_s}:1 \
             ({threshold_s}:1 required). Check it by hand.",
            "Der Text liegt über einem Bild oder Verlauf. Über den größten Teil seines Kastens \
             erreicht der Kontrast {median_s}:1, in helleren oder dunkleren Teilen nur \
             {p40_s}:1 (gefordert {threshold_s}:1). Von Hand prüfen."
        ),
    )
    .with_severity(Severity::Medium)
    .with_wcag([wcag])
    .with_evidence([
        Evidence::computed("contrast_ratio", format!("{median:.2}")),
        Evidence::computed("contrast_ratio_p40", format!("{p40:.2}")),
        Evidence::computed("required_ratio", format!("{threshold:.1}")),
    ])
    .at(at(n))
}

/// 1.4.3 (AA): 4,5:1, bei großem Text 3:1.
fn text_contrast<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(measurement) = measure(doc, n) else {
            continue;
        };
        if let Measurement::Undetermined(reason) = measurement {
            // Rule 3: Nicht prüfbar ist nicht bestanden.
            out.push(
                Finding::untested(
                    "contrast/text-undetermined",
                    match reason {
                        Reason::Color => pick!(
                            locale,
                            "The contrast cannot be determined automatically — the host could \
                             not resolve the foreground or background colour. Check it by hand.",
                            "Der Kontrast ist automatisiert nicht bestimmbar — der Host konnte \
                             Vorder- oder Hintergrundfarbe nicht auflösen. Von Hand prüfen.",
                        ),
                        Reason::Covered => pick!(
                            locale,
                            "A fixed or sticky element covers this text, so its contrast cannot \
                             be determined. Check it by hand with the overlay closed.",
                            "Ein fixiertes oder klebendes Element überdeckt diesen Text, sein \
                             Kontrast ist so nicht bestimmbar. Mit geschlossener Überlagerung von \
                             Hand prüfen.",
                        ),
                    },
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.3"])
                .at(at(n.id())),
            );
            continue;
        }
        let Some((verdict, threshold, large)) = verdicts(&measurement, 4.5, 3.0) else {
            continue;
        };
        match verdict {
            Verdict::Passes => {}
            Verdict::Fails(value) => out.push(
                Finding::fail(
                    "contrast/text-insufficient",
                    tr!(
                        locale,
                        "The text reaches a contrast ratio of {}:1; \
                         {}:1 is required for {}.",
                        "Der Text erreicht ein Kontrastverhältnis von {}:1, \
                         gefordert sind {}:1 für {}.",
                        number(locale, value, 2),
                        number(locale, threshold, 1),
                        text_kind(locale, large)
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.4.3"])
                .with_evidence(evidences(value, threshold))
                .at(at(n.id())),
            ),
            Verdict::Borderline { median, p40 } => out.push(borderline(
                "contrast/text-undetermined",
                median,
                p40,
                threshold,
                "1.4.3",
                locale,
                n.id(),
            )),
        }
    }
}

/// 1.4.6 (AAA): 7:1, bei großem Text 4,5:1. Gemeldet wird nur, was AA
/// besteht — was schon 1.4.3 verfehlt oder dort offen ist, steht dort und
/// nicht doppelt.
fn text_contrast_enhanced<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(measurement) = measure(doc, n) else {
            continue;
        };
        if !matches!(
            verdicts(&measurement, 4.5, 3.0),
            Some((Verdict::Passes, _, _))
        ) {
            continue;
        }
        let Some((verdict, aaa, large)) = verdicts(&measurement, 7.0, 4.5) else {
            continue;
        };
        match verdict {
            Verdict::Passes => {}
            Verdict::Fails(value) => out.push(
                Finding::fail(
                    "contrast/text-enhanced",
                    tr!(
                        locale,
                        "The text reaches {}:1; enhanced contrast (AAA) requires \
                         {}:1 for {}.",
                        "Der Text erreicht {}:1; erhöhter Kontrast (AAA) verlangt \
                         {}:1 für {}.",
                        number(locale, value, 2),
                        number(locale, aaa, 1),
                        text_kind(locale, large)
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.6"])
                .with_evidence(evidences(value, aaa))
                .at(at(n.id())),
            ),
            Verdict::Borderline { median, p40 } => out.push(borderline(
                "contrast/text-enhanced",
                median,
                p40,
                aaa,
                "1.4.6",
                locale,
                n.id(),
            )),
        }
    }
}

/// Ob das Element ein Kontrollkästchen, Optionsfeld, Schalter oder
/// Schieberegler ist — die Bedienelemente, deren Grenze 1.4.11 betrifft und
/// die Autoren mit `appearance: none` gern selbst zeichnen.
fn is_select_element<'a, N: Node<'a>>(n: N) -> bool {
    let ty = n
        .attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_default();
    (n.is_element("input") && matches!(ty.as_str(), "checkbox" | "radio" | "range"))
        || matches!(n.attr("role"), Some("checkbox" | "radio" | "switch"))
}

/// 1.4.11: Die Grenze eines selbst gezeichneten Bedienelements braucht 3:1
/// gegenüber dem Hintergrund, auf dem es steht.
///
/// Nur `appearance: none`: Ein natives Element zeichnet der Browser, sein
/// Kontrast steht in keinem berechneten Stil. Die Grenze ist der obere
/// Rahmen, wenn er gezeichnet wird, sonst die eigene Fläche. Ist beides
/// durchsichtig, gibt es keine Grenze zu messen. Kein Hintergrund am
/// Elternelement heißt `UNTESTED`, nicht ein Vergleich gegen Weiß.
fn control_contrast<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !is_select_element(n) || inactive(n) || doc.bounds(n).is_some_and(|b| b.is_empty()) {
            continue;
        }
        let Some(style) = doc.computed_style(n) else {
            continue;
        };
        if !is_rendered(&style) || style.appearance.as_deref() != Some("none") {
            continue;
        }
        let frame = style.border_top_width_px.is_some_and(|w| w > 0.0)
            && !matches!(style.border_top_style.as_deref(), Some("none" | "hidden"));
        let limit = if frame {
            style.border_top_color
        } else {
            style.own_background_color
        };
        let Some(limit) = limit.filter(|c| c.a > 0) else {
            continue;
        };
        let back = n
            .parent()
            .and_then(|p| doc.computed_style(p))
            .and_then(|s| s.background_color);
        let Some(back) = back else {
            out.push(
                Finding::untested(
                    "contrast/control-undetermined",
                    pick!(
                        locale,
                        "The contrast of this custom control cannot be determined — the host \
                         could not resolve the background behind it. Check it by hand.",
                        "Der Kontrast dieses selbst gezeichneten Bedienelements ist nicht \
                         bestimmbar — der Host konnte den Hintergrund dahinter nicht auflösen. \
                         Von Hand prüfen.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.11"])
                .at(at(n.id())),
            );
            continue;
        };
        let value = ratio(over(limit, back), back);
        if value >= 3.0 {
            continue;
        }
        out.push(
            Finding::fail(
                "contrast/control-insufficient",
                tr!(
                    locale,
                    "The boundary of this custom control reaches {}:1 against its \
                     background; 3:1 is required.",
                    "Die Grenze dieses selbst gezeichneten Bedienelements erreicht {}:1 \
                     gegenüber ihrem Hintergrund, gefordert sind 3:1.",
                    number(locale, value, 2),
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.4.11"])
            .with_evidence(evidences(value, 3.0))
            .at(at(n.id())),
        );
    }
}

const CONTRAST_CONTROL: Meta = Meta {
    ids: &[
        "contrast/control-insufficient",
        "contrast/control-undetermined",
    ],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &["1.4.11"],
    severity: Severity::Medium,
    help: "A custom-drawn checkbox, radio button, switch or slider (appearance: none) needs \
           a boundary with a contrast ratio of at least 3:1 against its background.",
    help_de: de_text!(
        "Ein selbst gezeichnetes Kontrollkästchen, Optionsfeld, Schalter oder \
              Schieberegler (appearance: none) braucht eine Grenze mit mindestens 3:1 \
              gegenüber seinem Hintergrund."
    ),
};

const CONTRAST: Meta = Meta {
    ids: &["contrast/text-insufficient", "contrast/text-undetermined"],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &["1.4.3"],
    severity: Severity::High,
    help: "Text needs a contrast ratio against its background of at least 4.5:1, \
           or 3:1 for large text. Large text starts at 18 pt, or 14 pt when bold.",
    help_de: de_text!(
        "Text braucht gegenüber seinem Hintergrund ein Kontrastverhältnis von \
              mindestens 4,5:1, bei großem Text 3:1. Großer Text ist ab 18 pt, bei \
              fettem Schnitt ab 14 pt."
    ),
};

const CONTRAST_ENHANCED: Meta = Meta {
    ids: &["contrast/text-enhanced"],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &["1.4.6"],
    severity: Severity::Medium,
    help: "Enhanced contrast (AAA): at least 7:1, or 4.5:1 for large text.",
    help_de: de_text!("Erhöhter Kontrast (AAA): mindestens 7:1, bei großem Text 4,5:1."),
};

pub(crate) const METAS: &[Meta] = &[
    CONTRAST,
    CONTRAST_ENHANCED,
    CONTRAST_CONTROL,
    heuristics::METAS[0],
    heuristics::METAS[1],
    heuristics::METAS[2],
    heuristics::METAS[3],
    heuristics::METAS[4],
    heuristics::METAS[5],
    heuristics::METAS[6],
    heuristics::METAS[7],
    presentation::METAS[0],
    presentation::METAS[1],
    presentation::METAS[2],
    probes::METAS[0],
];

/// Drei eigene Regeln, dazu die Heuristiken, die Darstellungsregeln und die
/// Regeln über Messungen des Hosts.
const _: () = assert!(
    METAS.len() == 3 + heuristics::METAS.len() + presentation::METAS.len() + probes::METAS.len()
);

pub(crate) fn rules<D: Rendering>() -> Vec<RenderingRule<D>> {
    let functions = [
        text_contrast as fn(&D, Locale, &mut Vec<Finding>),
        text_contrast_enhanced,
        control_contrast,
    ]
    .into_iter()
    .chain(heuristics::functions::<D>())
    .chain(presentation::functions::<D>())
    .chain(probes::functions::<D>());
    METAS
        .iter()
        .zip(functions)
        .map(|(meta, run)| RenderingRule { meta: *meta, run })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[test]
    fn ratio_black_on_white_is_21() {
        assert!((ratio(BLACK, WHITE) - 21.0).abs() < 0.01);
    }

    #[test]
    fn ratio_is_symmetric() {
        assert!((ratio(BLACK, WHITE) - ratio(WHITE, BLACK)).abs() < 1e-9);
    }

    #[test]
    fn same_color_yields_one() {
        assert!((ratio(WHITE, WHITE) - 1.0).abs() < 1e-9);
    }

    /// Die Referenzwerte stammen aus WCAG 1.4.3: #767676 ist die dunkelste
    /// Graustufe, die auf Weiß noch 4,5:1 erreicht.
    #[test]
    fn edge_case_767676_on_white_passes_narrowly() {
        let gray = Color {
            r: 0x76,
            g: 0x76,
            b: 0x76,
            a: 255,
        };
        let v = ratio(gray, WHITE);
        assert!(v >= 4.5, "{v} sollte 4,5 erreichen");
        assert!(v < 4.6, "{v} sollte knapp darüber liegen");
    }

    #[test]
    fn semi_transparent_black_on_white_becomes_gray() {
        let half = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 128,
        };
        let mixed = over(half, WHITE);
        assert_eq!(mixed.a, 255);
        assert_eq!(mixed.r, 127);
    }

    #[test]
    fn large_text_from_24px_or_bold_from_18_66px() {
        let style = |px: f32, w: u16| ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: Some(px),
            font_weight: Some(w),
            display: None,
            visibility: None,
            ..Default::default()
        };
        assert!(is_large_text(&style(24.0, 400)));
        assert!(!is_large_text(&style(23.0, 400)));
        assert!(is_large_text(&style(19.0, 700)));
        assert!(!is_large_text(&style(19.0, 400)));
    }
}
