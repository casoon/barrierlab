//! Kurzschreibweisen in die Einzeleigenschaften zerlegen, die die Kaskade
//! auswertet. Was nicht sicher zu zerlegen ist (`var()`, Systemschriften),
//! wird zu [`UNKNOWN`]: die Einzeleigenschaften sind dann unbestimmt, nicht
//! geraten.

use crate::color::{parse_color, teile_oberste_ebene};
use crate::value::{font_size_px, tokens};

/// Wert, der „hier steht etwas, das wir nicht lesen können" heißt.
pub(crate) const UNKNOWN: &str = "\u{1}unknown";

/// Beginn eines Werts, der eine Kurzschreibweise mit `var()` trägt: erst am
/// Element lässt sich die Variable einsetzen und die Kurzschreibweise zerlegen.
/// Aufbau: `\u{2}<kurzschreibweise>\u{2}<wert>`.
pub(crate) const PENDING: char = '\u{2}';

const KURZSCHREIBWEISEN: &[&str] = &[
    "background",
    "font",
    "border",
    "border-top",
    "border-bottom",
    "border-style",
    "border-width",
    "border-color",
    "text-decoration",
    "list-style",
];

const RAHMEN_STILE: &[&str] = &[
    "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];
const LINIEN: &[&str] = &["underline", "overline", "line-through", "blink", "none"];
const LISTEN_TYPEN: &[&str] = &[
    "disc",
    "circle",
    "square",
    "decimal",
    "decimal-leading-zero",
    "lower-roman",
    "upper-roman",
    "lower-alpha",
    "upper-alpha",
    "lower-latin",
    "upper-latin",
    "lower-greek",
    "disclosure-open",
    "disclosure-closed",
    "none",
];

fn alle(props: &[&str], v: &str) -> Vec<(String, String)> {
    props
        .iter()
        .map(|p| ((*p).to_string(), v.to_string()))
        .collect()
}

fn ist_farbe(t: &str) -> bool {
    t.eq_ignore_ascii_case("currentcolor") || parse_color(t, false).is_some()
}

fn ist_bild(t: &str) -> bool {
    let t = t.to_ascii_lowercase();
    t.starts_with("url(")
        || t.contains("gradient(")
        || t.starts_with("image-set(")
        || t.starts_with("image(")
        || t.starts_with("element(")
        || t.starts_with("-webkit-image-set(")
}

/// Zerlegt eine Deklaration in (Eigenschaft, Wert)-Paare. Eine Eigenschaft,
/// die keine Kurzschreibweise ist, kommt unverändert zurück.
pub(crate) fn expand(name: &str, value: &str) -> Vec<(String, String)> {
    let v = value.trim();
    let hat_var = v.to_ascii_lowercase().contains("var(");
    if hat_var && KURZSCHREIBWEISEN.contains(&name) {
        return expand(name, "")
            .into_iter()
            .map(|(p, _)| (p, format!("{PENDING}{name}{PENDING}{v}")))
            .collect();
    }
    match name {
        "background" => {
            if hat_var {
                return alle(&["background-color", "background-image"], UNKNOWN);
            }
            let schichten = teile_oberste_ebene(v, ',');
            let bilder: Vec<&str> = schichten
                .iter()
                .map(|s| {
                    tokens(s)
                        .into_iter()
                        .find(|t| ist_bild(t))
                        .unwrap_or("none")
                })
                .collect();
            let bild = bilder.iter().any(|b| *b != "none");
            let farbe = schichten
                .last()
                .and_then(|s| tokens(s).into_iter().find(|t| ist_farbe(t)))
                .unwrap_or("transparent");
            vec![
                ("background-color".into(), farbe.to_string()),
                (
                    "background-image".into(),
                    if bild {
                        bilder.join(", ")
                    } else {
                        "none".to_string()
                    },
                ),
            ]
        }
        "font" => {
            let props = [
                "font-style",
                "font-weight",
                "font-size",
                "line-height",
                "font-family",
            ];
            if hat_var
                || matches!(
                    v,
                    "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar"
                )
            {
                return alle(&props, UNKNOWN);
            }
            let t = tokens(v);
            let (mut stil, mut gewicht) = ("normal", "normal");
            let mut i = 0;
            while i < t.len() {
                let k = t[i].to_ascii_lowercase();
                let groesse = k.split('/').next().unwrap_or("");
                if font_size_px(groesse, 16.0, 16.0).is_some() && !k.starts_with("normal") {
                    break;
                }
                match k.as_str() {
                    "italic" | "oblique" => stil = t[i],
                    "bold" | "bolder" | "lighter" => gewicht = t[i],
                    k if k.parse::<u16>().is_ok() => gewicht = t[i],
                    _ => {}
                }
                i += 1;
            }
            let Some(groesse_zeile) = t.get(i) else {
                return alle(&props, UNKNOWN);
            };
            let (groesse, zeilenhoehe) = match groesse_zeile.split_once('/') {
                Some((g, l)) => (g, l),
                None => (*groesse_zeile, "normal"),
            };
            let familie = t[i + 1..].join(" ");
            if familie.is_empty() {
                return alle(&props, UNKNOWN);
            }
            vec![
                ("font-style".into(), stil.into()),
                ("font-weight".into(), gewicht.into()),
                ("font-size".into(), groesse.into()),
                ("line-height".into(), zeilenhoehe.into()),
                ("font-family".into(), familie),
            ]
        }
        "border" | "border-top" | "border-bottom" => {
            let seiten: &[&str] = match name {
                "border" => &["top", "bottom"],
                "border-top" => &["top"],
                _ => &["bottom"],
            };
            let teile: Vec<(String, String)> = if hat_var {
                vec![
                    ("width".into(), UNKNOWN.into()),
                    ("style".into(), UNKNOWN.into()),
                    ("color".into(), UNKNOWN.into()),
                ]
            } else {
                let (mut breite, mut stil, mut farbe) = ("medium", "none", "currentcolor");
                for t in tokens(v) {
                    let l = t.to_ascii_lowercase();
                    if RAHMEN_STILE.contains(&l.as_str()) {
                        stil = t;
                    } else if matches!(l.as_str(), "thin" | "medium" | "thick")
                        || l.starts_with(|c: char| c.is_ascii_digit() || c == '.')
                    {
                        breite = t;
                    } else {
                        farbe = t;
                    }
                }
                vec![
                    ("width".into(), breite.into()),
                    ("style".into(), stil.into()),
                    ("color".into(), farbe.into()),
                ]
            };
            seiten
                .iter()
                .flat_map(|s| {
                    teile
                        .iter()
                        .map(move |(k, w)| (format!("border-{s}-{k}"), w.clone()))
                })
                .collect()
        }
        "border-style" | "border-width" | "border-color" => {
            let art = &name["border-".len()..];
            let t = if hat_var { vec![UNKNOWN] } else { tokens(v) };
            let (oben, unten) = match t.as_slice() {
                [a] | [a, _] => (*a, *a),
                [a, _, c] | [a, _, c, _] => (*a, *c),
                _ => (UNKNOWN, UNKNOWN),
            };
            vec![
                (format!("border-top-{art}"), oben.to_string()),
                (format!("border-bottom-{art}"), unten.to_string()),
            ]
        }
        "text-decoration" => {
            if hat_var {
                return alle(&["text-decoration-line"], UNKNOWN);
            }
            let linien: Vec<&str> = tokens(v)
                .into_iter()
                .filter(|t| LINIEN.contains(&t.to_ascii_lowercase().as_str()))
                .collect();
            let wert = if linien.is_empty() {
                "none".to_string()
            } else {
                linien.join(" ")
            };
            vec![("text-decoration-line".into(), wert)]
        }
        "list-style" => {
            if hat_var {
                return alle(&["list-style-type"], UNKNOWN);
            }
            let typ = tokens(v)
                .into_iter()
                .find(|t| LISTEN_TYPEN.contains(&t.to_ascii_lowercase().as_str()))
                .unwrap_or("disc");
            vec![("list-style-type".into(), typ.to_string())]
        }
        _ => vec![(name.to_string(), v.to_string())],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(p: &[(String, String)], k: &str) -> String {
        p.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    #[test]
    fn hintergrund() {
        let p = expand("background", "#fff url(a.png) no-repeat");
        assert_eq!(get(&p, "background-color"), "#fff");
        assert_ne!(get(&p, "background-image"), "none");
        let p = expand("background", "linear-gradient(red, blue)");
        assert_eq!(get(&p, "background-color"), "transparent");
        assert_ne!(get(&p, "background-image"), "none");
        let p = expand("background", "var(--bg)");
        assert!(get(&p, "background-color").starts_with(PENDING));
        assert!(get(&p, "background-image").starts_with(PENDING));
    }

    #[test]
    fn schrift() {
        let p = expand("font", "italic bold 14px/1.5 Georgia, serif");
        assert_eq!(get(&p, "font-style"), "italic");
        assert_eq!(get(&p, "font-weight"), "bold");
        assert_eq!(get(&p, "font-size"), "14px");
        assert_eq!(get(&p, "line-height"), "1.5");
        assert_eq!(get(&p, "font-family"), "Georgia, serif");
        let p = expand("font", "menu");
        assert_eq!(get(&p, "font-size"), UNKNOWN);
    }

    #[test]
    fn rahmen() {
        let p = expand("border", "2px solid #333");
        assert_eq!(get(&p, "border-top-width"), "2px");
        assert_eq!(get(&p, "border-bottom-style"), "solid");
        assert_eq!(get(&p, "border-top-color"), "#333");
        let p = expand("border-top", "none");
        assert_eq!(get(&p, "border-top-style"), "none");
        assert_eq!(get(&p, "border-bottom-style"), "");
        let p = expand("border-style", "solid none");
        assert_eq!(get(&p, "border-top-style"), "solid");
        assert_eq!(get(&p, "border-bottom-style"), "solid");
    }

    #[test]
    fn dekoration_und_liste() {
        assert_eq!(
            get(
                &expand("text-decoration", "underline dotted red"),
                "text-decoration-line"
            ),
            "underline"
        );
        assert_eq!(
            get(&expand("text-decoration", "none"), "text-decoration-line"),
            "none"
        );
        assert_eq!(
            get(&expand("list-style", "none"), "list-style-type"),
            "none"
        );
        assert_eq!(
            get(&expand("list-style", "square inside"), "list-style-type"),
            "square"
        );
    }

    #[test]
    fn einzeleigenschaft_bleibt() {
        assert_eq!(
            expand("color", " red "),
            vec![("color".to_string(), "red".to_string())]
        );
    }
}
