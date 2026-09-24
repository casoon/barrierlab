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
use a11y_report::{Finding, Location, Severity};

use crate::registry::{Meta, RenderingRule};

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

fn text_kontrast<D: Rendering>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !traegt_text(n) {
            continue;
        }
        let Some(stil) = doc.computed_style(n) else {
            continue;
        };
        if !wird_dargestellt(&stil) {
            continue;
        }

        let gross = ist_grosser_text(&stil);
        let schwelle = if gross { 3.0 } else { 4.5 };

        match (stil.color, stil.background_color) {
            (Some(vorn), Some(hinten)) => {
                let wert = verhaeltnis(ueber(vorn, hinten), hinten);
                if wert + 0.005 < schwelle {
                    out.push(
                        Finding::fail(
                            "contrast/text-insufficient",
                            format!(
                                "The text reaches a contrast ratio of {wert:.2}:1; \
                                 {schwelle:.1}:1 is required for {}.",
                                if gross { "large text" } else { "normal text" }
                            ),
                        )
                        .with_severity(Severity::High)
                        .with_wcag(["1.4.3"])
                        .at(at(n.id())),
                    );
                }
            }
            _ => {
                // Rule 3: Nicht prüfbar ist nicht bestanden. Der Host konnte
                // eine der beiden Farben nicht bestimmen — meist ein
                // Hintergrundbild oder ein Verlauf.
                out.push(
                    Finding::untested(
                        "contrast/text-undetermined",
                        "The contrast cannot be determined automatically — the host could not \
                         resolve the foreground or background colour. Check it by hand.",
                    )
                    .with_severity(Severity::Medium)
                    .with_wcag(["1.4.3"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

pub(crate) const METAS: &[Meta] = &[Meta {
    ids: &["contrast/text-insufficient", "contrast/text-undetermined"],
    tier: Tier::Rendering,
    wcag: &["1.4.3"],
    severity: Severity::High,
    help: "Text needs a contrast ratio against its background of at least 4.5:1, \
           or 3:1 for large text. Large text starts at 18 pt, or 14 pt when bold.",
}];

pub(crate) fn rules<D: Rendering>() -> Vec<RenderingRule<D>> {
    vec![RenderingRule {
        meta: METAS[0],
        run: text_kontrast,
    }]
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
        };
        assert!(ist_grosser_text(&stil(24.0, 400)));
        assert!(!ist_grosser_text(&stil(23.0, 400)));
        assert!(ist_grosser_text(&stil(19.0, 700)));
        assert!(!ist_grosser_text(&stil(19.0, 400)));
    }
}
