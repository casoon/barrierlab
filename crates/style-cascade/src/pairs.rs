//! Farbpaare hinter einem Text aus Schichten, die ein Host kennt: für
//! [`a11y_dom::Rendering::color_pairs`].
//!
//! Die Kaskade ([`crate::Cascade::color_pairs`]) baut die Schichten aus den
//! Stylesheets; ein Browser-Host baut sie aus den berechneten Stilen. Die
//! Verrechnung ist dieselbe, damit beide Hosts für dieselbe Seite dieselben
//! Paare liefern.

use a11y_dom::{Color, ColorPairs};

use crate::color::{over, parse_color, teile_oberste_ebene};
use crate::value::tokens;

/// Ein Element der Kette vom Text nach außen: seine Hintergrundfarbe, die
/// Stufen eines Verlaufs darüber und seine `opacity`.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintLayer {
    /// `background-color`, `None` ohne Angabe (durchsichtig).
    pub background_color: Option<Color>,
    /// Die Stufen eines `background-image` aus Verläufen, siehe
    /// [`gradient_stops`]; `None` ohne Bild.
    pub gradient: Option<Vec<Color>>,
    /// `opacity`, 0 bis 1.
    pub opacity: f64,
}

/// Die Farbstufen eines `background-image`, das nur aus Verläufen besteht
/// (CSS Images 3 §3). `None` bei einem Bild (`url()`, `image-set()` …) oder
/// einer nicht lesbaren Stufe: Dann ist der Hintergrund nicht bekannt.
/// Nimmt angegebene wie berechnete Werte (`rgb(…)` statt Namen).
pub fn gradient_stops(value: &str, dark: bool) -> Option<Vec<Color>> {
    let mut out = Vec::new();
    for schicht in teile_oberste_ebene(value, ',') {
        let schicht = schicht.trim();
        if schicht.eq_ignore_ascii_case("none") {
            continue;
        }
        let lower = schicht.to_ascii_lowercase();
        let (name, rest) = lower.split_once('(')?;
        if !matches!(
            name.trim(),
            "linear-gradient"
                | "radial-gradient"
                | "conic-gradient"
                | "repeating-linear-gradient"
                | "repeating-radial-gradient"
                | "repeating-conic-gradient"
        ) {
            return None;
        }
        let inhalt = rest.strip_suffix(')')?;
        for teil in teile_oberste_ebene(inhalt, ',') {
            let t = tokens(teil);
            let erstes = t.first()?;
            // Richtung, Form, Ort, Farbraum und Farbhinweise sind keine Stufen.
            let keine_stufe = erstes.starts_with("to")
                || *erstes == "circle"
                || *erstes == "ellipse"
                || *erstes == "closest-side"
                || *erstes == "farthest-side"
                || *erstes == "closest-corner"
                || *erstes == "farthest-corner"
                || *erstes == "from"
                || *erstes == "at"
                || *erstes == "in"
                || erstes.ends_with("deg")
                || erstes.ends_with("turn")
                || erstes.ends_with("rad")
                || erstes.ends_with("grad")
                || erstes.ends_with('%')
                || erstes.parse::<f32>().is_ok();
            if keine_stufe {
                continue;
            }
            out.push(parse_color(erstes, dark)?);
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Die Farbe je Schatten eines `text-shadow`; `None` an einer Stelle heißt
/// `currentcolor` (keine Farbangabe oder ausdrücklich).
pub(crate) fn shadow_tokens(value: &str) -> Vec<Option<&str>> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Vec::new();
    }
    teile_oberste_ebene(value, ',')
        .into_iter()
        .map(|schatten| {
            tokens(schatten)
                .into_iter()
                .find(|t| {
                    !t.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))
                })
                .filter(|t| !t.eq_ignore_ascii_case("currentcolor"))
        })
        .collect()
}

/// Die Farben eines `text-shadow` (CSS Text Decoration 3 §4): leer ohne
/// Schatten, `None`, wenn eine Farbe nicht lesbar ist. `current` ist die
/// Textfarbe, die ein Schatten ohne eigene Farbe annimmt.
pub fn shadow_colors(value: &str, current: Color, dark: bool) -> Option<Vec<Color>> {
    shadow_tokens(value)
        .into_iter()
        .map(|t| t.map_or(Some(current), |t| parse_color(t, dark)))
        .collect()
}

/// Die Paare (Text, Hintergrund) hinter einem Text, wenn ein Verlauf,
/// `opacity` oder ein Schatten im Spiel ist: je Stufe eines Verlaufs und je
/// Mitte zwischen zwei Stufen ein Paar, `opacity` als lineare Mischung mit
/// dem, was dahinter liegt (CSS Color 4 §15.1, Compositing 1 §3.3).
///
/// `layers` geht vom Element des Texts nach außen; gelesen wird bis zur ersten
/// deckenden Farbe ohne Verlauf, hinter der keine `opacity` unter 1 mehr
/// folgt, sonst bis `canvas`. `halo`: die Farben aus [`shadow_colors`],
/// `None`, wenn sie nicht lesbar sind.
///
/// `None`, wenn nichts davon im Spiel ist — dann gelten die einfachen
/// Farben. Leere Paare: nötig, aber nicht bestimmbar (mehr als ein Verlauf,
/// Text- oder Leinwandfarbe unbekannt, Schatten unlesbar).
pub fn compose_color_pairs(
    text: Option<Color>,
    layers: &[PaintLayer],
    canvas: Option<Color>,
    halo: Option<Vec<Color>>,
) -> Option<ColorPairs> {
    let mut schichten: Vec<&PaintLayer> = Vec::new();
    let mut grund = None;
    for (i, l) in layers.iter().enumerate() {
        schichten.push(l);
        let deckend = l.gradient.is_none() && l.background_color.is_some_and(|c| c.a == 255);
        if deckend && layers[i..].iter().all(|l| l.opacity >= 1.0) {
            grund = l.background_color;
            break;
        }
    }
    let verlaeufe = schichten.iter().filter(|s| s.gradient.is_some()).count();
    let gemischt = schichten.iter().any(|s| s.opacity < 1.0);
    if verlaeufe == 0 && !gemischt && halo.as_ref().is_some_and(Vec::is_empty) {
        return None;
    }
    let (Some(halo), true) = (halo, verlaeufe <= 1) else {
        return Some(ColorPairs::default());
    };
    let Some(text) = text else {
        return Some(ColorPairs::default());
    };
    let Some(grund) = grund.or(canvas) else {
        return Some(ColorPairs::default());
    };
    schichten.reverse();
    let mische = |hinten: Color, vorn: Color, o: f64| {
        let m = |h: u8, v: u8| (f64::from(h) * (1.0 - o) + f64::from(v) * o).round() as u8;
        Color {
            r: m(hinten.r, vorn.r),
            g: m(hinten.g, vorn.g),
            b: m(hinten.b, vorn.b),
            a: 255,
        }
    };
    let varianten = schichten
        .iter()
        .find_map(|s| s.gradient.as_ref())
        .map_or(1, |st| (2 * st.len()).saturating_sub(1).max(1));
    let mut pairs = Vec::with_capacity(varianten);
    for v in 0..varianten {
        let mut c = grund;
        let mut hinten = Vec::with_capacity(schichten.len());
        for s in &schichten {
            hinten.push(c);
            if let Some(f) = s.background_color {
                c = over(f, c);
            }
            if let Some(st) = &s.gradient {
                let farben: Vec<Color> = st.iter().map(|s| over(*s, c)).collect();
                let mitte = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
                let mut proben = farben.clone();
                proben.extend(farben.windows(2).map(|w| Color {
                    r: mitte(w[0].r, w[1].r),
                    g: mitte(w[0].g, w[1].g),
                    b: mitte(w[0].b, w[1].b),
                    a: 255,
                }));
                c = *proben.get(v)?;
            }
        }
        let (mut t, mut b) = (over(text, c), c);
        for (i, s) in schichten.iter().enumerate().rev() {
            if s.opacity < 1.0 {
                t = mische(hinten[i], t, s.opacity);
                b = mische(hinten[i], b, s.opacity);
            }
        }
        pairs.push((t, b));
    }
    let opacity = schichten.iter().map(|s| s.opacity).product();
    Some(ColorPairs {
        pairs,
        halo,
        opacity: Some(opacity),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }
    const WHITE: Color = rgb(255, 255, 255);
    const BLACK: Color = rgb(0, 0, 0);

    fn layer(background_color: Option<Color>, opacity: f64) -> PaintLayer {
        PaintLayer {
            background_color,
            gradient: None,
            opacity,
        }
    }

    /// So schreibt Chrome einen berechneten Verlauf: Funktionsfarben mit
    /// Leerzeichen, Längen hinter den Stufen.
    #[test]
    fn verlauf_in_berechneter_schreibweise() {
        assert_eq!(
            gradient_stops(
                "linear-gradient(to right, rgb(255, 255, 255), rgb(0, 0, 255))",
                false
            ),
            Some(vec![WHITE, rgb(0, 0, 255)])
        );
        let stufen = gradient_stops(
            "linear-gradient(90deg, rgba(0, 0, 0, 0), rgba(0, 0, 0, 0) 3.3em, rgb(0, 0, 0) 3.3em, rgb(0, 0, 0) 6em)",
            false,
        )
        .unwrap();
        assert_eq!(stufen.len(), 4);
        assert_eq!(gradient_stops("url(\"a.png\")", false), None);
    }

    #[test]
    fn schattenfarben_in_berechneter_schreibweise() {
        assert_eq!(
            shadow_colors("rgb(255, 255, 255) 0px 0px 3px", BLACK, false),
            Some(vec![WHITE])
        );
        assert_eq!(
            shadow_colors("2px 2px 1px", WHITE, false),
            Some(vec![WHITE])
        );
        assert_eq!(shadow_colors("none", BLACK, false), Some(vec![]));
    }

    /// ACT afw4f7 / 09o5cg: Schwarz unter `opacity: .3` auf Weiß ist ein
    /// helles Grau auf Weiß.
    #[test]
    fn opacity_mischt_text_und_hintergrund() {
        let p = compose_color_pairs(
            Some(BLACK),
            &[layer(None, 0.3), layer(Some(WHITE), 1.0)],
            Some(WHITE),
            Some(vec![]),
        )
        .unwrap();
        assert_eq!(p.pairs, vec![(rgb(179, 179, 179), WHITE)]);
    }

    /// ACT afw4f7: ein Verlauf ergibt je Stufe und Mitte ein Paar.
    #[test]
    fn verlauf_ergibt_paare_je_stufe_und_mitte() {
        let p = compose_color_pairs(
            Some(rgb(0x33, 0x33, 0x33)),
            &[
                PaintLayer {
                    background_color: None,
                    gradient: Some(vec![WHITE, rgb(0, 0, 255)]),
                    opacity: 1.0,
                },
                layer(None, 1.0),
            ],
            Some(WHITE),
            Some(vec![]),
        )
        .unwrap();
        let hinten: Vec<Color> = p.pairs.iter().map(|(_, b)| *b).collect();
        assert_eq!(hinten, vec![WHITE, rgb(0, 0, 255), rgb(127, 127, 255)]);
    }

    /// ACT afw4f7: Der Schatten kommt als Halo mit, die Paare sind die
    /// einfachen Farben.
    #[test]
    fn schatten_kommt_als_halo_mit() {
        let grau = rgb(0x73, 0x73, 0x73);
        let p = compose_color_pairs(
            Some(BLACK),
            &[layer(Some(grau), 1.0)],
            Some(WHITE),
            Some(vec![WHITE]),
        )
        .unwrap();
        assert_eq!(p.pairs, vec![(BLACK, grau)]);
        assert_eq!(p.halo, vec![WHITE]);
    }

    #[test]
    fn ohne_verlauf_deckkraft_und_schatten_gelten_die_einfachen_farben() {
        assert_eq!(
            compose_color_pairs(
                Some(BLACK),
                &[layer(Some(WHITE), 1.0)],
                Some(WHITE),
                Some(vec![])
            ),
            None
        );
    }

    #[test]
    fn unlesbarer_schatten_oder_zwei_verlaeufe_sind_nicht_bestimmbar() {
        let p = compose_color_pairs(Some(BLACK), &[layer(Some(WHITE), 1.0)], Some(WHITE), None);
        assert_eq!(p, Some(ColorPairs::default()));
        let verlauf = || PaintLayer {
            background_color: None,
            gradient: Some(vec![WHITE, BLACK]),
            opacity: 1.0,
        };
        let p = compose_color_pairs(
            Some(BLACK),
            &[verlauf(), verlauf()],
            Some(WHITE),
            Some(vec![]),
        );
        assert_eq!(p, Some(ColorPairs::default()));
    }
}
