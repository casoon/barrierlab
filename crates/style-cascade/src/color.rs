//! CSS-Farben lesen (CSS Color Level 4, soweit ohne Farbraum-Umrechnung
//! möglich) und Farbflächen übereinanderlegen.

use a11y_dom::Color;

/// Liest eine Farbe. `None` für alles, was sich nicht ohne Weiteres in sRGB
/// ausdrücken lässt (`color-mix()`, `oklch()`, `var()` …) und für
/// `currentcolor`, das der Aufrufer auflösen muss.
///
/// `dark` wählt den Zweig von `light-dark(hell, dunkel)`.
pub fn parse_color(text: &str, dark: bool) -> Option<Color> {
    let v = text.trim().to_ascii_lowercase();
    let v = v.as_str();
    if let Some(inner) = funktion(v, "light-dark") {
        let teile = teile_oberste_ebene(inner, ',');
        let [hell, dunkel] = teile.as_slice() else {
            return None;
        };
        return parse_color(if dark { dunkel } else { hell }, dark);
    }
    if let Some(hex) = v.strip_prefix('#') {
        return hex_farbe(hex);
    }
    if let Some(inner) = funktion(v, "rgb").or_else(|| funktion(v, "rgba")) {
        return rgb(inner);
    }
    if let Some(inner) = funktion(v, "hsl").or_else(|| funktion(v, "hsla")) {
        return hsl(inner);
    }
    if let Some(inner) = funktion(v, "oklab") {
        return oklab(inner, false);
    }
    if let Some(inner) = funktion(v, "oklch") {
        return oklab(inner, true);
    }
    if let Some(inner) = funktion(v, "lab") {
        return lab(inner, false);
    }
    if let Some(inner) = funktion(v, "lch") {
        return lab(inner, true);
    }
    if let Some(inner) = funktion(v, "hwb") {
        return hwb(inner);
    }
    if let Some(inner) = funktion(v, "color") {
        return color_funktion(inner);
    }
    if let Some(inner) = funktion(v, "color-mix") {
        return color_mix(inner, dark);
    }
    if v == "transparent" {
        return Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    benannt(v)
}

/// Relative Leuchtdichte nach WCAG 2.2 (sRGB), 0,0 bis 1,0. Alpha bleibt
/// unberücksichtigt: erst über einen Untergrund legen ([`over`]).
pub fn luminance(c: Color) -> f64 {
    let k = |v: u8| f64::from(linear(f32::from(v) / 255.0));
    0.2126 * k(c.r) + 0.7152 * k(c.g) + 0.0722 * k(c.b)
}

/// `a` über `b` gelegt; das Ergebnis ist deckend.
pub fn over(a: Color, b: Color) -> Color {
    let alpha = f32::from(a.a) / 255.0;
    let m = |v: u8, h: u8| (f32::from(v) * alpha + f32::from(h) * (1.0 - alpha)).round() as u8;
    Color {
        r: m(a.r, b.r),
        g: m(a.g, b.g),
        b: m(a.b, b.b),
        a: 255,
    }
}

fn funktion<'a>(v: &'a str, name: &str) -> Option<&'a str> {
    v.strip_prefix(name)?
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')
}

/// Teilt an `sep`, aber nicht innerhalb von Klammern.
pub(crate) fn teile_oberste_ebene(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut tiefe = 0u32;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => tiefe += 1,
            ')' => tiefe = tiefe.saturating_sub(1),
            c if c == sep && tiefe == 0 => {
                out.push(s[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

fn hex_farbe(hex: &str) -> Option<Color> {
    let k = |s: &str| u8::from_str_radix(s, 16).ok();
    let doppelt = |i: usize| k(&hex[i..=i].repeat(2));
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        3 => Some(Color {
            r: doppelt(0)?,
            g: doppelt(1)?,
            b: doppelt(2)?,
            a: 255,
        }),
        4 => Some(Color {
            r: doppelt(0)?,
            g: doppelt(1)?,
            b: doppelt(2)?,
            a: doppelt(3)?,
        }),
        6 => Some(Color {
            r: k(&hex[0..2])?,
            g: k(&hex[2..4])?,
            b: k(&hex[4..6])?,
            a: 255,
        }),
        8 => Some(Color {
            r: k(&hex[0..2])?,
            g: k(&hex[2..4])?,
            b: k(&hex[4..6])?,
            a: k(&hex[6..8])?,
        }),
        _ => None,
    }
}

/// Kanäle und Alpha einer `rgb()`/`hsl()`-Liste, alt (Kommas) oder neu
/// (Leerzeichen und `/`).
fn komponenten(inner: &str) -> Option<(Vec<&str>, Option<&str>)> {
    if inner.contains(',') {
        let t = teile_oberste_ebene(inner, ',');
        match t.len() {
            3 => Some((t, None)),
            4 => Some((t[..3].to_vec(), Some(t[3]))),
            _ => None,
        }
    } else {
        let (farbe, alpha) = match inner.split_once('/') {
            Some((f, a)) => (f, Some(a.trim())),
            None => (inner, None),
        };
        let t: Vec<&str> = farbe.split_whitespace().collect();
        (t.len() == 3).then_some((t, alpha))
    }
}

fn alpha(a: Option<&str>) -> Option<u8> {
    let Some(a) = a else { return Some(255) };
    let wert = match a.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok()? / 100.0,
        None => a.trim().parse::<f32>().ok()?,
    };
    Some((wert.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn rgb(inner: &str) -> Option<Color> {
    let (k, a) = komponenten(inner)?;
    let kanal = |s: &str| -> Option<u8> {
        let wert = match s.strip_suffix('%') {
            Some(p) => p.trim().parse::<f32>().ok()? * 2.55,
            None => s.trim().parse::<f32>().ok()?,
        };
        Some(wert.clamp(0.0, 255.0).round() as u8)
    };
    Some(Color {
        r: kanal(k[0])?,
        g: kanal(k[1])?,
        b: kanal(k[2])?,
        a: alpha(a)?,
    })
}

fn hsl(inner: &str) -> Option<Color> {
    let (k, a) = komponenten(inner)?;
    let h = k[0].trim().trim_end_matches("deg").parse::<f32>().ok()?;
    let proz = |s: &str| {
        s.trim()
            .strip_suffix('%')?
            .parse::<f32>()
            .ok()
            .map(|p| p / 100.0)
    };
    let (s, l) = (proz(k[1])?.clamp(0.0, 1.0), proz(k[2])?.clamp(0.0, 1.0));
    let h = h.rem_euclid(360.0) / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let k8 = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Color {
        r: k8(r),
        g: k8(g),
        b: k8(b),
        a: alpha(a)?,
    })
}

/// Eine Zahl oder ein Prozentwert; `prozent_von` ist der Wert bei 100 %.
fn zahl_oder_prozent(s: &str, prozent_von: f32) -> Option<f32> {
    let s = s.trim();
    if s == "none" {
        return Some(0.0);
    }
    match s.strip_suffix('%') {
        Some(p) => Some(p.trim().parse::<f32>().ok()? / 100.0 * prozent_von),
        None => s.trim().parse().ok(),
    }
}

fn winkel(s: &str) -> Option<f32> {
    let s = s.trim();
    if s == "none" {
        return Some(0.0);
    }
    for (suffix, faktor) in [
        ("deg", 1.0),
        ("grad", 0.9),
        ("rad", 180.0 / std::f32::consts::PI),
        ("turn", 360.0),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return n.trim().parse::<f32>().ok().map(|n| n * faktor);
        }
    }
    s.parse().ok()
}

fn gamma(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn linear(v: f32) -> f32 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn aus_linear(r: f32, g: f32, b: f32, a: u8) -> Color {
    let k = |v: f32| (gamma(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
    Color {
        r: k(r),
        g: k(g),
        b: k(b),
        a,
    }
}

/// Linear-sRGB aus OKLab (Björn Ottosson).
fn oklab_zu_linear(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    (
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_93 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s,
        -0.004_196_086 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    )
}

fn linear_zu_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = (0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    (
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    )
}

fn oklab(inner: &str, polar: bool) -> Option<Color> {
    let (farbe, a) = match inner.split_once('/') {
        Some((f, a)) => (f, Some(a.trim())),
        None => (inner, None),
    };
    let t: Vec<&str> = farbe.split_whitespace().collect();
    let [x, y, z] = t.as_slice() else { return None };
    let l = zahl_oder_prozent(x, 1.0)?;
    let (a_, b_) = if polar {
        let c = zahl_oder_prozent(y, 0.4)?.max(0.0);
        let h = winkel(z)?.to_radians();
        (c * h.cos(), c * h.sin())
    } else {
        (zahl_oder_prozent(y, 0.4)?, zahl_oder_prozent(z, 0.4)?)
    };
    let (r, g, b) = oklab_zu_linear(l.clamp(0.0, 1.0), a_, b_);
    Some(aus_linear(r, g, b, alpha(a)?))
}

/// CIE-Lab und -LCH (D50) nach sRGB, mit Bradford-Anpassung auf D65.
fn lab(inner: &str, polar: bool) -> Option<Color> {
    let (farbe, a) = match inner.split_once('/') {
        Some((f, a)) => (f, Some(a.trim())),
        None => (inner, None),
    };
    let t: Vec<&str> = farbe.split_whitespace().collect();
    let [x, y, z] = t.as_slice() else { return None };
    let l = zahl_oder_prozent(x, 100.0)?.clamp(0.0, 100.0);
    let (a_, b_) = if polar {
        let c = zahl_oder_prozent(y, 150.0)?.max(0.0);
        let h = winkel(z)?.to_radians();
        (c * h.cos(), c * h.sin())
    } else {
        (zahl_oder_prozent(y, 125.0)?, zahl_oder_prozent(z, 125.0)?)
    };
    const EPS: f32 = 216.0 / 24389.0;
    const KAPPA: f32 = 24389.0 / 27.0;
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a_ / 500.0;
    let fz = fy - b_ / 200.0;
    let kub = |f: f32| {
        if f * f * f > EPS {
            f * f * f
        } else {
            (116.0 * f - 16.0) / KAPPA
        }
    };
    let yr = if l > KAPPA * EPS {
        fy * fy * fy
    } else {
        l / KAPPA
    };
    let (xd50, yd50, zd50) = (kub(fx) * 0.964_22, yr, kub(fz) * 0.825_21);
    let xd65 = 0.955_576_6 * xd50 - 0.023_039_3 * yd50 + 0.063_163_6 * zd50;
    let yd65 = -0.028_289_5 * xd50 + 1.009_941_6 * yd50 + 0.021_007_7 * zd50;
    let zd65 = 0.012_298_2 * xd50 - 0.020_483 * yd50 + 1.329_909_8 * zd50;
    let r = 3.240_97 * xd65 - 1.537_383_2 * yd65 - 0.498_610_76 * zd65;
    let g = -0.969_243_6 * xd65 + 1.875_967_5 * yd65 + 0.041_555_06 * zd65;
    let b = 0.055_630_08 * xd65 - 0.203_976_96 * yd65 + 1.056_971_5 * zd65;
    Some(aus_linear(r, g, b, alpha(a)?))
}

fn hwb(inner: &str) -> Option<Color> {
    let (farbe, a) = match inner.split_once('/') {
        Some((f, a)) => (f, Some(a.trim())),
        None => (inner, None),
    };
    let t: Vec<&str> = farbe.split_whitespace().collect();
    let [h, w, b] = t.as_slice() else { return None };
    let (w, b) = (zahl_oder_prozent(w, 1.0)?, zahl_oder_prozent(b, 1.0)?);
    let (w, b) = if w + b >= 1.0 {
        (w / (w + b), b / (w + b))
    } else {
        (w, b)
    };
    let reiner = hsl(&format!("{} 100% 50%", winkel(h)?))?;
    let k = |v: u8| ((f32::from(v) / 255.0 * (1.0 - w - b) + w) * 255.0).round() as u8;
    Some(Color {
        r: k(reiner.r),
        g: k(reiner.g),
        b: k(reiner.b),
        a: alpha(a)?,
    })
}

/// `color(srgb r g b / a)` und `color(srgb-linear …)`. Weitere Farbräume
/// (Display P3 …) sind nicht ohne Gamut-Abbildung auszudrücken.
fn color_funktion(inner: &str) -> Option<Color> {
    let (farbe, a) = match inner.split_once('/') {
        Some((f, a)) => (f, Some(a.trim())),
        None => (inner, None),
    };
    let t: Vec<&str> = farbe.split_whitespace().collect();
    let [raum, x, y, z] = t.as_slice() else {
        return None;
    };
    let (r, g, b) = (
        zahl_oder_prozent(x, 1.0)?,
        zahl_oder_prozent(y, 1.0)?,
        zahl_oder_prozent(z, 1.0)?,
    );
    match *raum {
        "srgb" => {
            let k = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            Some(Color {
                r: k(r),
                g: k(g),
                b: k(b),
                a: alpha(a)?,
            })
        }
        "srgb-linear" => Some(aus_linear(r, g, b, alpha(a)?)),
        _ => None,
    }
}

/// `color-mix(in srgb|oklab, farbe [p%], farbe [p%])`.
fn color_mix(inner: &str, dark: bool) -> Option<Color> {
    let teile = teile_oberste_ebene(inner, ',');
    let [raum, f1, f2] = teile.as_slice() else {
        return None;
    };
    let raum = raum.trim().strip_prefix("in")?.trim();
    let ok = match raum {
        "srgb" => false,
        "oklab" => true,
        _ => return None,
    };
    let zerlege = |s: &str| -> Option<(Color, Option<f32>)> {
        let t = crate::value::tokens(s);
        let (farbe, p) = match t.as_slice() {
            [f] => (*f, None),
            [f, p] if p.ends_with('%') => (
                *f,
                Some(p.trim_end_matches('%').parse::<f32>().ok()? / 100.0),
            ),
            [p, f] if p.ends_with('%') => (
                *f,
                Some(p.trim_end_matches('%').parse::<f32>().ok()? / 100.0),
            ),
            _ => return None,
        };
        Some((parse_color(farbe, dark)?, p))
    };
    let (c1, p1) = zerlege(f1)?;
    let (c2, p2) = zerlege(f2)?;
    let (p1, p2) = match (p1, p2) {
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => (a, 1.0 - a),
        (None, Some(b)) => (1.0 - b, b),
        (None, None) => (0.5, 0.5),
    };
    let summe = p1 + p2;
    if summe <= 0.0 {
        return None;
    }
    let (n1, n2) = (p1 / summe, p2 / summe);
    let alpha_aus = (n1 * f32::from(c1.a) + n2 * f32::from(c2.a)) / 255.0;
    let faktor = summe.min(1.0);
    let k = |c: Color| {
        let (r, g, b) = (
            linear(f32::from(c.r) / 255.0),
            linear(f32::from(c.g) / 255.0),
            linear(f32::from(c.b) / 255.0),
        );
        if ok {
            linear_zu_oklab(r, g, b)
        } else {
            (
                f32::from(c.r) / 255.0,
                f32::from(c.g) / 255.0,
                f32::from(c.b) / 255.0,
            )
        }
    };
    let (x1, x2) = (k(c1), k(c2));
    let (a1, a2) = (f32::from(c1.a) / 255.0, f32::from(c2.a) / 255.0);
    // Vormultipliziert mischen; bei Alpha 0 bleibt nur die Farbe, die sichtbar ist.
    let gewicht = n1 * a1 + n2 * a2;
    let mische = |u: f32, v: f32| {
        if gewicht == 0.0 {
            0.0
        } else {
            (n1 * a1 * u + n2 * a2 * v) / gewicht
        }
    };
    let (u, v, w) = (mische(x1.0, x2.0), mische(x1.1, x2.1), mische(x1.2, x2.2));
    let a8 = (alpha_aus * faktor * 255.0).round() as u8;
    if ok {
        let (r, g, b) = oklab_zu_linear(u, v, w);
        Some(aus_linear(r, g, b, a8))
    } else {
        let k8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        Some(Color {
            r: k8(u),
            g: k8(v),
            b: k8(w),
            a: a8,
        })
    }
}

fn benannt(name: &str) -> Option<Color> {
    let i = FARBEN.binary_search_by_key(&name, |(n, _)| n).ok()?;
    let v = FARBEN[i].1;
    Some(Color {
        r: (v >> 16) as u8,
        g: (v >> 8) as u8,
        b: v as u8,
        a: 255,
    })
}

/// Die benannten Farben (CSS Color Level 4 §6.1), alphabetisch für die
/// Binärsuche.
const FARBEN: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("grey", 0x808080),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn c(r: u8, g: u8, b: u8, a: u8) -> Option<Color> {
        Some(Color { r, g, b, a })
    }

    #[test]
    fn hex() {
        assert_eq!(parse_color("#fff", false), c(255, 255, 255, 255));
        assert_eq!(parse_color("#336699", false), c(0x33, 0x66, 0x99, 255));
        assert_eq!(parse_color("#33669980", false), c(0x33, 0x66, 0x99, 0x80));
        assert_eq!(parse_color("#12", false), None);
    }

    #[test]
    fn rgb_alt_und_neu() {
        assert_eq!(parse_color("rgb(10, 20, 30)", false), c(10, 20, 30, 255));
        assert_eq!(parse_color("rgba(10,20,30,.5)", false), c(10, 20, 30, 128));
        assert_eq!(
            parse_color("rgb(10 20 30 / 50%)", false),
            c(10, 20, 30, 128)
        );
        assert_eq!(parse_color("rgb(100% 0% 0%)", false), c(255, 0, 0, 255));
    }

    #[test]
    fn hsl_und_namen() {
        assert_eq!(parse_color("hsl(0, 100%, 50%)", false), c(255, 0, 0, 255));
        assert_eq!(
            parse_color("hsl(120deg 100% 25%)", false),
            c(0, 128, 0, 255)
        );
        assert_eq!(
            parse_color("RebeccaPurple", false),
            c(0x66, 0x33, 0x99, 255)
        );
        assert_eq!(parse_color("transparent", false), c(0, 0, 0, 0));
    }

    #[test]
    fn light_dark() {
        assert_eq!(
            parse_color("light-dark(#fff, #000)", false),
            c(255, 255, 255, 255)
        );
        assert_eq!(parse_color("light-dark(#fff, #000)", true), c(0, 0, 0, 255));
    }

    #[test]
    fn nicht_lesbares_ist_none() {
        assert_eq!(parse_color("currentcolor", false), None);
        assert_eq!(parse_color("color-mix(in hsl, red, blue)", false), None);
        assert_eq!(parse_color("var(--x)", false), None);
        assert_eq!(parse_color("color(display-p3 1 0 0)", false), None);
    }

    fn nah(a: Option<Color>, r: u8, g: u8, b: u8) {
        let a = a.expect("Farbe");
        let d = |x: u8, y: u8| (i16::from(x) - i16::from(y)).abs() <= 1;
        assert!(
            d(a.r, r) && d(a.g, g) && d(a.b, b),
            "{a:?} != ({r}, {g}, {b})"
        );
    }

    #[test]
    fn moderne_farbraeume() {
        // Referenzwerte aus CSS Color 4 (Chrome-Ausgabe).
        nah(parse_color("oklch(100% 0 0)", false), 255, 255, 255);
        nah(parse_color("oklch(0% 0 0)", false), 0, 0, 0);
        nah(parse_color("oklch(62.8% 0.2577 29.23)", false), 255, 0, 0);
        nah(parse_color("oklab(0.5 0 0)", false), 99, 99, 99);
        nah(parse_color("lab(50% 0 0)", false), 119, 119, 119);
        nah(parse_color("lab(54.29% 80.8 69.89)", false), 255, 0, 0);
        nah(parse_color("lch(50% 0 0)", false), 119, 119, 119);
        nah(parse_color("hwb(0 0% 0%)", false), 255, 0, 0);
        nah(parse_color("hwb(0 100% 0%)", false), 255, 255, 255);
        nah(parse_color("color(srgb 1 0 0)", false), 255, 0, 0);
    }

    #[test]
    fn farbmischung() {
        nah(
            parse_color("color-mix(in srgb, red, blue)", false),
            128,
            0,
            128,
        );
        nah(
            parse_color("color-mix(in srgb, white 25%, black)", false),
            64,
            64,
            64,
        );
        let halb = parse_color("color-mix(in oklab, white 50%, transparent)", false).unwrap();
        assert_eq!((halb.r, halb.g, halb.b, halb.a), (255, 255, 255, 128));
    }

    #[test]
    fn leuchtdichte() {
        let l = |r, g, b| luminance(Color { r, g, b, a: 255 });
        assert!((l(255, 255, 255) - 1.0).abs() < 1e-6);
        assert!(l(0, 0, 0).abs() < 1e-9);
        assert!((l(119, 119, 119) - 0.184).abs() < 0.002);
    }

    #[test]
    fn ueberlagerung() {
        let halb = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 128,
        };
        let weiss = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        assert_eq!(
            over(halb, weiss),
            Color {
                r: 127,
                g: 127,
                b: 127,
                a: 255
            }
        );
    }

    #[test]
    fn farbtabelle_ist_sortiert() {
        assert!(FARBEN.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
