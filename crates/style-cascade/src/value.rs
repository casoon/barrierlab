//! Längen, Schriftgewichte und Zerlegen von Werten.

/// Zerlegt an Leerraum außerhalb von Klammern und Anführungszeichen.
pub(crate) fn tokens(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut tiefe = 0u32;
    let mut anf: Option<char> = None;
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if let Some(q) = anf {
            if c == q {
                anf = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                anf = Some(c);
                start.get_or_insert(i);
            }
            '(' => {
                tiefe += 1;
                start.get_or_insert(i);
            }
            ')' => tiefe = tiefe.saturating_sub(1),
            c if c.is_whitespace() && tiefe == 0 => {
                if let Some(st) = start.take() {
                    out.push(&s[st..i]);
                }
            }
            _ => {
                start.get_or_insert(i);
            }
        }
    }
    if let Some(st) = start {
        out.push(&s[st..]);
    }
    out
}

/// Die benannten Schriftgrößen (CSS Fonts 4 §2.5) in Pixeln bei 16 px Basis.
fn schluesselwort(v: &str) -> Option<f32> {
    Some(match v {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => 16.0,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "xxx-large" => 48.0,
        _ => return None,
    })
}

/// Eine Schriftgröße in Pixeln. `parent` ist die Größe des Elternelements,
/// `root` die des Wurzelelements. `None` für `calc()`, `vw`, `ex` und alles,
/// was ohne Layout nicht zu bestimmen ist.
pub(crate) fn font_size_px(v: &str, parent: f32, root: f32) -> Option<f32> {
    let v = v.trim().to_ascii_lowercase();
    if let Some(px) = schluesselwort(&v) {
        return Some(px);
    }
    match v.as_str() {
        "larger" => return Some(parent * 1.2),
        "smaller" => return Some(parent / 1.2),
        _ => {}
    }
    laenge(&v, parent, root)
}

/// Eine Länge in Pixeln; `em` und `%` beziehen sich auf `em`.
pub(crate) fn laenge(v: &str, em: f32, root: f32) -> Option<f32> {
    let v = v.trim();
    let zahl = |s: &str| s.trim().parse::<f32>().ok();
    if let Some(p) = v.strip_suffix('%') {
        return zahl(p).map(|p| em * p / 100.0);
    }
    for (suffix, faktor) in [
        ("px", 1.0),
        ("pt", 4.0 / 3.0),
        ("pc", 16.0),
        ("in", 96.0),
        ("cm", 96.0 / 2.54),
        ("mm", 96.0 / 25.4),
    ] {
        if let Some(n) = v.strip_suffix(suffix) {
            return zahl(n).map(|n| n * faktor);
        }
    }
    if let Some(n) = v.strip_suffix("rem") {
        return zahl(n).map(|n| n * root);
    }
    if let Some(n) = v.strip_suffix("em") {
        return zahl(n).map(|n| n * em);
    }
    // 0 darf ohne Einheit stehen.
    zahl(v).filter(|n| *n == 0.0)
}

/// Ein Schriftgewicht, `parent` ist das des Elternelements.
pub(crate) fn font_weight(v: &str, parent: u16) -> Option<u16> {
    let v = v.trim().to_ascii_lowercase();
    match v.as_str() {
        "normal" => Some(400),
        "bold" => Some(700),
        "bolder" => Some(match parent {
            ..=399 => 400,
            400..=599 => 700,
            _ => 900,
        }),
        "lighter" => Some(match parent {
            ..=549 => 100,
            550..=749 => 400,
            _ => 700,
        }),
        n => n
            .parse::<f32>()
            .ok()
            .filter(|w| (1.0..=1000.0).contains(w))
            .map(|w| w as u16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerlegen() {
        assert_eq!(
            tokens("1px solid rgb(0 0 0)"),
            vec!["1px", "solid", "rgb(0 0 0)"]
        );
        assert_eq!(tokens(r#"a "b c" d"#), vec!["a", r#""b c""#, "d"]);
    }

    #[test]
    fn groessen() {
        assert_eq!(font_size_px("14px", 16.0, 16.0), Some(14.0));
        assert_eq!(font_size_px("1.5em", 20.0, 16.0), Some(30.0));
        assert_eq!(font_size_px("150%", 20.0, 16.0), Some(30.0));
        assert_eq!(font_size_px("2rem", 20.0, 16.0), Some(32.0));
        assert_eq!(font_size_px("12pt", 16.0, 16.0), Some(16.0));
        assert_eq!(font_size_px("large", 10.0, 16.0), Some(18.0));
        assert_eq!(font_size_px("calc(1px + 2em)", 16.0, 16.0), None);
        assert_eq!(font_size_px("5vw", 16.0, 16.0), None);
    }

    #[test]
    fn gewichte() {
        assert_eq!(font_weight("bold", 400), Some(700));
        assert_eq!(font_weight("600", 400), Some(600));
        assert_eq!(font_weight("bolder", 400), Some(700));
        assert_eq!(font_weight("lighter", 700), Some(400));
    }
}
