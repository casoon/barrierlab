//! Berechnete Stile für das Darstellungs-Tier.
//!
//! Was ohne Layout und Skripte zu bestimmen ist, kommt als `Some`; alles
//! andere als `None` — nie als geratener Wert. Der Kontrast hängt an
//! [`ComputedStyle::background_color`]: ein Hintergrundbild, ein Verlauf, eine
//! Mischung oder ein absolut positioniertes Element machen ihn unbestimmt.

use a11y_dom::{Color, ComputedStyle, Node, NodeKind, ancestors};

use crate::cascade::{Cascade, Specified};
use crate::color::{luminance, over, parse_color, teile_oberste_ebene};
use crate::value::{font_size_px, font_weight, laenge, tokens};

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

fn element_eltern<'a, N: Node<'a>>(n: N) -> Option<N> {
    n.parent().filter(|p| p.kind() == NodeKind::Element)
}

fn wurzel<'a, N: Node<'a>>(n: N) -> N {
    ancestors(n).last().unwrap_or(n)
}

impl Cascade {
    /// Die Schriftgröße in Pixeln; `None`, wenn sie von `calc()`, `vw` oder
    /// einer unbestimmten Regel abhängt.
    pub fn font_size_px<'a, N: Node<'a>>(&self, n: N) -> Option<f32> {
        if let Some(s) = self.sizes.borrow().get(&n.id()) {
            return *s;
        }
        let eltern = match element_eltern(n) {
            Some(p) => self.font_size_px(p),
            None => Some(16.0),
        };
        let rem = if element_eltern(n).is_none() {
            Some(16.0)
        } else {
            self.font_size_px(wurzel(n))
        };
        let groesse = match self.specified(n, "font-size") {
            Specified::Unknown => None,
            Specified::Unset => eltern,
            Specified::Set(v) if v == "inherit" => eltern,
            Specified::Set(v) => match (eltern, rem) {
                (Some(e), Some(r)) => font_size_px(&v, e, r).filter(|x| x.is_finite() && *x >= 0.0),
                _ => None,
            },
        };
        self.sizes.borrow_mut().insert(n.id(), groesse);
        groesse
    }

    /// Das Schriftgewicht, 1 bis 1000.
    pub fn font_weight<'a, N: Node<'a>>(&self, n: N) -> Option<u16> {
        if let Some(s) = self.weights.borrow().get(&n.id()) {
            return *s;
        }
        let eltern = match element_eltern(n) {
            Some(p) => self.font_weight(p),
            None => Some(400),
        };
        let gewicht = match self.specified(n, "font-weight") {
            Specified::Unknown => None,
            Specified::Unset => eltern,
            Specified::Set(v) if v == "inherit" => eltern,
            Specified::Set(v) => eltern.and_then(|e| font_weight(&v, e)),
        };
        self.weights.borrow_mut().insert(n.id(), gewicht);
        gewicht
    }

    /// Die Textfarbe samt Alpha. `None`, wenn sie sich nicht lesen lässt.
    pub fn color<'a, N: Node<'a>>(&self, n: N) -> Option<Color> {
        match self.specified(n, "color") {
            Specified::Unknown => None,
            Specified::Unset => Some(SCHWARZ),
            Specified::Set(v) if v.trim().eq_ignore_ascii_case("currentcolor") => {
                match element_eltern(n) {
                    Some(p) => self.color(p),
                    None => Some(SCHWARZ),
                }
            }
            Specified::Set(v) => parse_color(&v, self.environment().dark),
        }
    }

    /// Verändern Deckkraft, Filter oder ein Mischmodus an diesem Element oder
    /// einem Vorfahren die Farben, die die DOM-Kette ergibt? Das gilt für Text
    /// und Hintergrund.
    fn darstellung_unsicher<'a, N: Node<'a>>(&self, n: N) -> bool {
        std::iter::once(n).chain(ancestors(n)).any(|a| {
            if a.kind() != NodeKind::Element {
                return false;
            }
            let opazitaet = match self.specified(a, "opacity") {
                Specified::Unset => false,
                Specified::Unknown => true,
                Specified::Set(v) => v
                    .trim()
                    .trim_end_matches('%')
                    .parse::<f32>()
                    .map_or(true, |o| if v.contains('%') { o < 100.0 } else { o < 1.0 }),
            };
            opazitaet
                || self.ungleich(a, "filter", &["none"])
                || self.ungleich(a, "mix-blend-mode", &["normal"])
                || self.ungleich(a, "background-blend-mode", &["normal"])
        })
    }

    /// `backdrop-filter` mit mehr als Weichzeichnen: ein Weichzeichner mittelt
    /// nur, was dahinter liegt; Sättigung oder Helligkeit verschieben es.
    fn verschiebt_farben<'a, N: Node<'a>>(&self, a: N) -> bool {
        match self.specified(a, "backdrop-filter") {
            Specified::Unset => false,
            Specified::Unknown => true,
            Specified::Set(v) => {
                let v = v.trim().to_ascii_lowercase();
                v != "none" && !tokens(&v).iter().all(|t| t.starts_with("blur("))
            }
        }
    }

    fn ungleich<'a, N: Node<'a>>(&self, a: N, prop: &str, harmlos: &[&str]) -> bool {
        match self.specified(a, prop) {
            Specified::Unset => false,
            Specified::Unknown => true,
            Specified::Set(v) => !harmlos.contains(&v.trim().to_ascii_lowercase().as_str()),
        }
    }

    /// Steht das Element so, dass der Hintergrund der DOM-Kette nicht der
    /// ist, der hinter ihm liegt? Aus dem Fluss genommene Kästen
    /// (`position: absolute|fixed`) und `backdrop-filter` zählen — aber nur
    /// bis zur ersten deckenden Fläche; die gehört dem Element selbst und
    /// verdeckt, was dahinter liegt.
    fn lage_unsicher<'a, N: Node<'a>>(&self, n: N) -> bool {
        for a in std::iter::once(n).chain(ancestors(n)) {
            if a.kind() != NodeKind::Element {
                continue;
            }
            let ohne_bild = match self.specified(a, "background-image") {
                Specified::Unset => true,
                Specified::Set(v) => v.trim().eq_ignore_ascii_case("none"),
                Specified::Unknown => false,
            };
            let deckend = ohne_bild
                && matches!(
                    self.specified(a, "background-color"),
                    Specified::Set(v)
                        if parse_color(&v, self.environment().dark).is_some_and(|c| c.a == 255)
                );
            if deckend {
                return false;
            }
            if self.ungleich(a, "position", &["static", "relative", "sticky"])
                || self.verschiebt_farben(a)
            {
                return true;
            }
        }
        false
    }

    /// Der Hintergrund hinter dem Text des Elements: Farben von innen nach
    /// außen übereinandergelegt bis zur ersten deckenden, darunter Weiß.
    /// `None` bei Bild, Verlauf, Mischung oder unbestimmter Regel.
    pub fn effective_background<'a, N: Node<'a>>(&self, n: N) -> Option<Color> {
        if self.darstellung_unsicher(n) || self.lage_unsicher(n) {
            return None;
        }
        let mut schichten = Vec::new();
        for a in std::iter::once(n).chain(ancestors(n)) {
            if a.kind() != NodeKind::Element {
                continue;
            }
            match self.specified(a, "background-image") {
                Specified::Unknown => return None,
                Specified::Set(v) if !v.trim().eq_ignore_ascii_case("none") => return None,
                _ => {}
            }
            match self.specified(a, "background-color") {
                Specified::Unknown => return None,
                Specified::Unset => {}
                Specified::Set(v) => {
                    let c = if v.trim().eq_ignore_ascii_case("currentcolor") {
                        self.color(a)?
                    } else {
                        parse_color(&v, self.environment().dark)?
                    };
                    schichten.push(c);
                    if c.a == 255 {
                        break;
                    }
                }
            }
        }
        Some(
            schichten
                .into_iter()
                .rev()
                .fold(WEISS, |unten, c| over(c, unten)),
        )
    }

    /// Abgetasteter Hintergrund hinter dem Text, wenn dort ein Verlauf liegt:
    /// die Leuchtdichte jeder Farbstufe und der Mitte zwischen je zwei Stufen,
    /// über den Untergrund gelegt. Das sind die Ränder des Verlaufs, nicht jeder
    /// seiner Pixel.
    ///
    /// `None`, wenn der Hintergrund nicht aus Verläufen besteht (ein Bild,
    /// eine unlesbare Stufe, `var()` ohne Wert), schon eine Farbe feststeht
    /// oder die Kette unsicher ist (Deckkraft, Filter, absolute Lage).
    pub fn backdrop_luminances<'a, N: Node<'a>>(&self, n: N) -> Option<Vec<f64>> {
        if self.darstellung_unsicher(n) || self.lage_unsicher(n) {
            return None;
        }
        // Von innen nach außen bis zum Element mit dem Bild. Was davor
        // liegt, muss unsichtbar sein, sonst liegt es vor dem Verlauf.
        let mut chain = std::iter::once(n)
            .chain(ancestors(n))
            .filter(|a| a.kind() == NodeKind::Element);
        let mut bild = None;
        // Halbdurchsichtige Farbflächen vor dem Verlauf, von innen nach außen.
        let mut vorn: Vec<Color> = Vec::new();
        for a in chain.by_ref() {
            let hat_bild = match self.specified(a, "background-image") {
                Specified::Unknown => return None,
                Specified::Set(v) if !v.trim().eq_ignore_ascii_case("none") => Some(v),
                _ => None,
            };
            let farbe = match self.specified(a, "background-color") {
                Specified::Unknown => return None,
                Specified::Unset => None,
                Specified::Set(v) if v.trim().eq_ignore_ascii_case("currentcolor") => self.color(a),
                Specified::Set(v) => Some(parse_color(&v, self.environment().dark)?),
            };
            if let Some(v) = hat_bild {
                bild = Some((a, v, farbe));
                break;
            }
            if let Some(c) = farbe.filter(|c| c.a > 0) {
                vorn.push(c);
            }
        }
        let (element, wert, eigene_farbe) = bild?;
        let stufen = verlaufsstufen(&wert, self.environment().dark)?;
        // Untergrund: die eigene Farbe des Elements, dann die Vorfahren.
        let mut unten = vec![];
        if let Some(c) = eigene_farbe {
            unten.push(c);
        }
        if eigene_farbe.is_none_or(|c| c.a < 255) {
            for a in ancestors(element).filter(|a| a.kind() == NodeKind::Element) {
                match self.specified(a, "background-image") {
                    Specified::Unknown => return None,
                    Specified::Set(v) if !v.trim().eq_ignore_ascii_case("none") => return None,
                    _ => {}
                }
                match self.specified(a, "background-color") {
                    Specified::Unknown => return None,
                    Specified::Unset => {}
                    Specified::Set(v) => {
                        let c = if v.trim().eq_ignore_ascii_case("currentcolor") {
                            self.color(a)?
                        } else {
                            parse_color(&v, self.environment().dark)?
                        };
                        unten.push(c);
                        if c.a == 255 {
                            break;
                        }
                    }
                }
            }
        }
        let grund = unten.into_iter().rev().fold(WEISS, |u, c| over(c, u));
        let farben: Vec<Color> = stufen
            .iter()
            .map(|c| vorn.iter().rev().fold(over(*c, grund), |u, v| over(*v, u)))
            .collect();
        let mut proben: Vec<f64> = farben.iter().map(|c| luminance(*c)).collect();
        for w in farben.windows(2) {
            let m = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
            proben.push(luminance(Color {
                r: m(w[0].r, w[1].r),
                g: m(w[0].g, w[1].g),
                b: m(w[0].b, w[1].b),
                a: 255,
            }));
        }
        Some(proben)
    }

    /// Die berechneten Werte, die Regeln des Darstellungs-Tiers lesen.
    ///
    /// Jedes Feld ist einzeln `None`, wenn es nicht bestimmbar ist; die
    /// Kontrastregel meldet das als „nicht bestimmbar", nicht als bestanden.
    pub fn computed_style<'a, N: Node<'a>>(&self, n: N) -> ComputedStyle {
        let eigen = |prop: &str, vorgabe: &str| -> Option<String> {
            match self.specified(n, prop) {
                Specified::Unknown => None,
                Specified::Unset => Some(vorgabe.to_string()),
                Specified::Set(v) => Some(v.trim().to_ascii_lowercase()),
            }
        };
        let unsicher = self.darstellung_unsicher(n);
        let bg = self.effective_background(n);
        let farbe = if unsicher {
            None
        } else {
            self.color(n).and_then(|c| match (c.a, bg) {
                (255, _) => Some(c),
                (_, Some(b)) => Some(over(c, b)),
                _ => None,
            })
        };
        let em = self.font_size_px(n);
        let stil_oben = eigen("border-top-style", "none");
        let breite_oben = match (&stil_oben, self.specified(n, "border-top-width")) {
            (Some(s), _) if s == "none" || s == "hidden" => Some(0.0),
            (Some(_), Specified::Unset) => Some(3.0),
            (Some(_), Specified::Set(v)) => match v.trim().to_ascii_lowercase().as_str() {
                "thin" => Some(1.0),
                "medium" => Some(3.0),
                "thick" => Some(5.0),
                w => em.and_then(|e| laenge(w, e, 16.0)),
            },
            _ => None,
        };
        let farbwert = |prop: &str, vorgabe: Option<Color>| -> Option<Color> {
            match self.specified(n, prop) {
                Specified::Unknown => None,
                Specified::Unset => vorgabe,
                Specified::Set(v) if v.trim().eq_ignore_ascii_case("currentcolor") => self.color(n),
                Specified::Set(v) => parse_color(&v, self.environment().dark),
            }
        };
        ComputedStyle {
            color: farbe,
            background_color: bg,
            font_size_px: em,
            font_weight: self.font_weight(n),
            display: eigen("display", "inline"),
            visibility: eigen("visibility", "visible"),
            list_style_type: eigen("list-style-type", "disc"),
            text_decoration_line: eigen("text-decoration-line", "none"),
            font_style: eigen("font-style", "normal"),
            font_family: eigen("font-family", "serif"),
            border_bottom_style: eigen("border-bottom-style", "none"),
            appearance: eigen("appearance", "auto")
                .filter(|a| a != "auto")
                .or_else(|| eigen("-webkit-appearance", "auto")),
            border_top_style: stil_oben,
            border_top_width_px: breite_oben,
            border_top_color: farbwert("border-top-color", self.color(n)),
            own_background_color: farbwert(
                "background-color",
                Some(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                }),
            ),
        }
    }
}

/// Die Farbstufen eines `background-image`, das nur aus Verläufen besteht.
fn verlaufsstufen(wert: &str, dark: bool) -> Option<Vec<Color>> {
    let mut out = Vec::new();
    for schicht in teile_oberste_ebene(wert, ',') {
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
