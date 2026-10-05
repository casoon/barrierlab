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

use a11y_dom::{Color, ComputedStyle, Node, NodeId, NodeKind, Rendering, Tier, elements};
use a11y_report::{Evidence, Finding, Location, Severity};

use crate::darstellung;
use crate::heuristik;
use crate::locale::{Locale, pick, tr};
use crate::registry::{Meta, RenderingRule};
use crate::sicht::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Relative Leuchtdichte nach WCAG 2.x, Definition „relative luminance".
fn luminanz(c: Color) -> f64 {
    fn kanal(v: u8) -> f64 {
        let s = f64::from(v) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * kanal(c.r) + 0.7152 * kanal(c.g) + 0.0722 * kanal(c.b)
}

/// Kontrastverhältnis nach WCAG 2.x, 1,0 bis 21,0.
fn verhaeltnis(vorn: Color, hinten: Color) -> f64 {
    let (a, b) = (luminanz(vorn), luminanz(hinten));
    let (hell, dunkel) = if a >= b { (a, b) } else { (b, a) };
    (hell + 0.05) / (dunkel + 0.05)
}

/// Legt eine teildurchsichtige Farbe über eine deckende.
///
/// Der Host löst die Transparenz des *Hintergrunds* auf, nicht die des
/// Vordergrunds: `color: rgba(0,0,0,.5)` bleibt als solches stehen und wird
/// erst hier verrechnet.
fn ueber(vorn: Color, hinten: Color) -> Color {
    if vorn.a == 255 {
        return vorn;
    }
    let a = f64::from(vorn.a) / 255.0;
    let misch = |v: u8, h: u8| (f64::from(v) * a + f64::from(h) * (1.0 - a)).round() as u8;
    Color {
        r: misch(vorn.r, hinten.r),
        g: misch(vorn.g, hinten.g),
        b: misch(vorn.b, hinten.b),
        a: 255,
    }
}

/// Großer Text im Sinne von WCAG 1.4.3: ab 18 pt, oder ab 14 pt bei fett.
///
/// Gerechnet wird in CSS-Pixeln, weil der berechnete Stil sie liefert:
/// 18 pt sind 24 px, 14 pt sind 18,66 px.
fn ist_grosser_text(stil: &ComputedStyle) -> bool {
    let Some(px) = stil.font_size_px else {
        return false;
    };
    let fett = stil.font_weight.is_some_and(|w| w >= 700);
    px >= 24.0 || (fett && px >= 18.66)
}

/// Ob dieses Element selbst Text trägt — nicht, ob irgendwo darunter Text
/// steht. Der Kontrast gehört an das Element, dessen Farbe gilt.
fn traegt_text<'a, N: Node<'a>>(n: N) -> bool {
    n.children()
        .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
}

/// Ob das Element überhaupt dargestellt wird. `display: none` und
/// `visibility: hidden` sind kein Kontrastproblem.
fn wird_dargestellt(stil: &ComputedStyle) -> bool {
    stil.display.as_deref() != Some("none") && stil.visibility.as_deref() != Some("hidden")
}

/// Was eine Messung am Element ergibt.
enum Messung {
    /// Eine Hintergrundfarbe gilt: Verhältnis und ob der Text groß ist.
    Wert(f64, bool),
    /// Abgetasteter Hintergrund: Median und 40. Perzentil der Verhältnisse
    /// über die Stichprobe, und ob der Text groß ist.
    Spanne { median: f64, p40: f64, gross: bool },
    /// Nicht bestimmbar, mit Grund.
    Unbestimmt(Grund),
}

enum Grund {
    /// Vorder- oder Hintergrundfarbe fehlt, und keine Abtastung hilft.
    Farbe,
    /// Ein fixiertes oder klebendes fremdes Element überdeckt die Mitte.
    Verdeckt,
}

fn verhaeltnis_lum(a: f64, b: f64) -> f64 {
    let (hell, dunkel) = if a >= b { (a, b) } else { (b, a) };
    (hell + 0.05) / (dunkel + 0.05)
}

/// Median und 40. Perzentil der Verhältnisse zwischen Textfarbe und den
/// abgetasteten Pixeln — wie auditmysite vor der Umstellung: Die Glyphen
/// selbst liegen in der Stichprobe und ziehen die unteren Werte nach unten,
/// deshalb nicht das Minimum.
fn spanne(vorn: Color, stichprobe: &[f64]) -> Option<(f64, f64)> {
    if stichprobe.is_empty() {
        return None;
    }
    // Halbdurchsichtige Schrift wird gegen Weiß verrechnet; der genaue
    // Untergrund ist hier gerade unbekannt.
    let vorn = luminanz(ueber(
        vorn,
        Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    ));
    let mut v: Vec<f64> = stichprobe
        .iter()
        .map(|l| verhaeltnis_lum(vorn, l.clamp(0.0, 1.0)))
        .collect();
    v.sort_by(f64::total_cmp);
    let bei = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    Some((bei(0.5), bei(0.4)))
}

/// Misst den Kontrast eines Elements, das selbst Text trägt. `None`, wenn es
/// nichts zu messen gibt: kein eigener Text, nicht dargestellt oder optisch
/// verborgen (`Rendering::visually_hidden`).
///
/// Text unter `aria-hidden` wird gemessen: WCAG 1.4.3 gilt für sichtbaren
/// Text, unabhängig vom Accessibility-Tree. auditmysite nahm ihn aus (#395).
fn messe<'d, D: Rendering>(doc: &'d D, n: D::N<'d>) -> Option<Messung> {
    if !traegt_text(n) {
        return None;
    }
    let stil = doc.computed_style(n)?;
    if !wird_dargestellt(&stil) || doc.visually_hidden(n) == Some(true) {
        return None;
    }
    if doc.layout(n).and_then(|l| l.obscured) == Some(true) {
        return Some(Messung::Unbestimmt(Grund::Verdeckt));
    }
    let gross = ist_grosser_text(&stil);
    Some(match (stil.color, stil.background_color) {
        (Some(vorn), Some(hinten)) => {
            Messung::Wert(verhaeltnis(ueber(vorn, hinten), hinten), gross)
        }
        (Some(vorn), None) => match doc
            .sampled_backdrop(n)
            .and_then(|b| spanne(vorn, &b.luminance))
        {
            Some((median, p40)) => Messung::Spanne { median, p40, gross },
            None => Messung::Unbestimmt(Grund::Farbe),
        },
        _ => Messung::Unbestimmt(Grund::Farbe),
    })
}

fn text_art(locale: Locale, gross: bool) -> &'static str {
    if gross {
        pick!(locale, "large text", "großen Text")
    } else {
        pick!(locale, "normal text", "normalen Text")
    }
}

fn belege(wert: f64, schwelle: f64) -> [Evidence; 2] {
    [
        Evidence::computed("contrast_ratio", format!("{wert:.2}")),
        Evidence::computed("required_ratio", format!("{schwelle:.1}")),
    ]
}

/// Eine Prüfung gegen eine Schwelle — für 1.4.3 und 1.4.6 dieselbe.
enum Urteil {
    Besteht,
    Verfehlt(f64),
    /// Abgetastet: Median besteht, das 40. Perzentil nicht.
    Grenzwertig {
        median: f64,
        p40: f64,
    },
}

fn urteile(m: &Messung, schwelle_klein: f64, schwelle_gross: f64) -> Option<(Urteil, f64, bool)> {
    let knapp = |w: f64, s: f64| w + 0.005 < s;
    match *m {
        Messung::Wert(w, gross) => {
            let s = if gross {
                schwelle_gross
            } else {
                schwelle_klein
            };
            Some((
                if knapp(w, s) {
                    Urteil::Verfehlt(w)
                } else {
                    Urteil::Besteht
                },
                s,
                gross,
            ))
        }
        Messung::Spanne { median, p40, gross } => {
            let s = if gross {
                schwelle_gross
            } else {
                schwelle_klein
            };
            let u = if knapp(median, s) {
                Urteil::Verfehlt(median)
            } else if knapp(p40, s) {
                Urteil::Grenzwertig { median, p40 }
            } else {
                Urteil::Besteht
            };
            Some((u, s, gross))
        }
        Messung::Unbestimmt(_) => None,
    }
}

fn grenzwertig(
    id: &str,
    median: f64,
    p40: f64,
    schwelle: f64,
    wcag: &str,
    locale: Locale,
    n: NodeId,
) -> Finding {
    Finding::review(
        id,
        tr!(
            locale,
            "The text lies over an image or gradient. Over most of its box the contrast \
             reaches {median:.2}:1, in darker or lighter parts only {p40:.2}:1 \
             ({schwelle:.1}:1 required). Check it by hand.",
            "Der Text liegt über einem Bild oder Verlauf. Über den größten Teil seines Kastens \
             erreicht der Kontrast {median:.2}:1, in helleren oder dunkleren Teilen nur \
             {p40:.2}:1 (gefordert {schwelle:.1}:1). Von Hand prüfen."
        ),
    )
    .with_severity(Severity::Medium)
    .with_wcag([wcag])
    .with_evidence([
        Evidence::computed("contrast_ratio", format!("{median:.2}")),
        Evidence::computed("contrast_ratio_p40", format!("{p40:.2}")),
        Evidence::computed("required_ratio", format!("{schwelle:.1}")),
    ])
    .at(at(n))
}

/// 1.4.3 (AA): 4,5:1, bei großem Text 3:1.
fn text_kontrast<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(messung) = messe(doc, n) else {
            continue;
        };
        if let Messung::Unbestimmt(grund) = messung {
            // Rule 3: Nicht prüfbar ist nicht bestanden.
            out.push(
                Finding::untested(
                    "contrast/text-undetermined",
                    match grund {
                        Grund::Farbe => pick!(
                            locale,
                            "The contrast cannot be determined automatically — the host could \
                             not resolve the foreground or background colour. Check it by hand.",
                            "Der Kontrast ist automatisiert nicht bestimmbar — der Host konnte \
                             Vorder- oder Hintergrundfarbe nicht auflösen. Von Hand prüfen.",
                        ),
                        Grund::Verdeckt => pick!(
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
        let Some((urteil, schwelle, gross)) = urteile(&messung, 4.5, 3.0) else {
            continue;
        };
        match urteil {
            Urteil::Besteht => {}
            Urteil::Verfehlt(wert) => out.push(
                Finding::fail(
                    "contrast/text-insufficient",
                    tr!(
                        locale,
                        "The text reaches a contrast ratio of {wert:.2}:1; \
                         {schwelle:.1}:1 is required for {}.",
                        "Der Text erreicht ein Kontrastverhältnis von {wert:.2}:1, \
                         gefordert sind {schwelle:.1}:1 für {}.",
                        text_art(locale, gross)
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.4.3"])
                .with_evidence(belege(wert, schwelle))
                .at(at(n.id())),
            ),
            Urteil::Grenzwertig { median, p40 } => out.push(grenzwertig(
                "contrast/text-undetermined",
                median,
                p40,
                schwelle,
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
fn text_kontrast_erhoeht<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(messung) = messe(doc, n) else {
            continue;
        };
        if !matches!(urteile(&messung, 4.5, 3.0), Some((Urteil::Besteht, _, _))) {
            continue;
        }
        let Some((urteil, aaa, gross)) = urteile(&messung, 7.0, 4.5) else {
            continue;
        };
        match urteil {
            Urteil::Besteht => {}
            Urteil::Verfehlt(wert) => out.push(
                Finding::fail(
                    "contrast/text-enhanced",
                    tr!(
                        locale,
                        "The text reaches {wert:.2}:1; enhanced contrast (AAA) requires \
                         {aaa:.1}:1 for {}.",
                        "Der Text erreicht {wert:.2}:1; erhöhter Kontrast (AAA) verlangt \
                         {aaa:.1}:1 für {}.",
                        text_art(locale, gross)
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.6"])
                .with_evidence(belege(wert, aaa))
                .at(at(n.id())),
            ),
            Urteil::Grenzwertig { median, p40 } => out.push(grenzwertig(
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

const KONTRAST: Meta = Meta {
    ids: &["contrast/text-insufficient", "contrast/text-undetermined"],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &["1.4.3"],
    severity: Severity::High,
    help: "Text needs a contrast ratio against its background of at least 4.5:1, \
           or 3:1 for large text. Large text starts at 18 pt, or 14 pt when bold.",
    #[cfg(feature = "de")]
    help_de: "Text braucht gegenüber seinem Hintergrund ein Kontrastverhältnis von \
              mindestens 4,5:1, bei großem Text 3:1. Großer Text ist ab 18 pt, bei \
              fettem Schnitt ab 14 pt.",
};

const KONTRAST_ERHOEHT: Meta = Meta {
    ids: &["contrast/text-enhanced"],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &["1.4.6"],
    severity: Severity::Medium,
    help: "Enhanced contrast (AAA): at least 7:1, or 4.5:1 for large text.",
    #[cfg(feature = "de")]
    help_de: "Erhöhter Kontrast (AAA): mindestens 7:1, bei großem Text 4,5:1.",
};

pub(crate) const METAS: &[Meta] = &[
    KONTRAST,
    KONTRAST_ERHOEHT,
    heuristik::METAS[0],
    heuristik::METAS[1],
    heuristik::METAS[2],
    heuristik::METAS[3],
    heuristik::METAS[4],
    heuristik::METAS[5],
    heuristik::METAS[6],
    darstellung::METAS[0],
    darstellung::METAS[1],
    darstellung::METAS[2],
];

pub(crate) fn rules<D: Rendering>() -> Vec<RenderingRule<D>> {
    let funktionen = [
        text_kontrast as fn(&D, Locale, &mut Vec<Finding>),
        text_kontrast_erhoeht,
    ]
    .into_iter()
    .chain(heuristik::funktionen::<D>())
    .chain(darstellung::funktionen::<D>());
    METAS
        .iter()
        .zip(funktionen)
        .map(|(meta, run)| RenderingRule { meta: *meta, run })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHWARZ: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    const WEISS: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[test]
    fn verhaeltnis_schwarz_auf_weiss_ist_21() {
        assert!((verhaeltnis(SCHWARZ, WEISS) - 21.0).abs() < 0.01);
    }

    #[test]
    fn verhaeltnis_ist_symmetrisch() {
        assert!((verhaeltnis(SCHWARZ, WEISS) - verhaeltnis(WEISS, SCHWARZ)).abs() < 1e-9);
    }

    #[test]
    fn gleiche_farbe_ergibt_eins() {
        assert!((verhaeltnis(WEISS, WEISS) - 1.0).abs() < 1e-9);
    }

    /// Die Referenzwerte stammen aus WCAG 1.4.3: #767676 ist die dunkelste
    /// Graustufe, die auf Weiß noch 4,5:1 erreicht.
    #[test]
    fn grenzfall_767676_auf_weiss_besteht_knapp() {
        let grau = Color {
            r: 0x76,
            g: 0x76,
            b: 0x76,
            a: 255,
        };
        let v = verhaeltnis(grau, WEISS);
        assert!(v >= 4.5, "{v} sollte 4,5 erreichen");
        assert!(v < 4.6, "{v} sollte knapp darüber liegen");
    }

    #[test]
    fn halbdurchsichtiges_schwarz_auf_weiss_wird_grau() {
        let halb = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 128,
        };
        let gemischt = ueber(halb, WEISS);
        assert_eq!(gemischt.a, 255);
        assert_eq!(gemischt.r, 127);
    }

    #[test]
    fn grosser_text_ab_24px_oder_fett_ab_18_66px() {
        let stil = |px: f32, w: u16| ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: Some(px),
            font_weight: Some(w),
            display: None,
            visibility: None,
            ..Default::default()
        };
        assert!(ist_grosser_text(&stil(24.0, 400)));
        assert!(!ist_grosser_text(&stil(23.0, 400)));
        assert!(ist_grosser_text(&stil(19.0, 700)));
        assert!(!ist_grosser_text(&stil(19.0, 400)));
    }
}
