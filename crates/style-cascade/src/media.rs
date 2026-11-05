//! Medienbedingungen gegen eine feste Umgebung auswerten.
//!
//! Die Grammatik liest `media-query-parse`; hier steht nur die Frage, ob
//! die Bedingung in der angenommenen Umgebung gilt. Was die Umgebung nicht
//! beantwortet (Auflösung, Farbtiefe …), ist [`Match::Unknown`].

use media_query_parse::{
    MediaCondition, MediaConditionWithoutOr, MediaFeature, MediaInParens, MediaModifier,
    MediaQuery, MfComparison, MfRange, MfValue, parse_media_query_list,
};

use crate::selector::Match;

/// Die Umgebung, in der die Seite gedacht wird.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    pub width: f32,
    pub height: f32,
    /// `prefers-color-scheme: dark`.
    pub dark: bool,
    /// `prefers-reduced-motion: reduce`.
    pub reduced_motion: bool,
    /// `hover: hover`; `None`, wenn die Umgebung es nicht festlegt.
    pub hover: Option<bool>,
    /// `pointer: fine`; `None`, wenn die Umgebung es nicht festlegt.
    pub fine_pointer: Option<bool>,
    /// `prefers-contrast: more`; `None`, wenn die Umgebung es nicht festlegt.
    pub contrast_more: Option<bool>,
    /// `forced-colors: active`; `None`, wenn die Umgebung es nicht festlegt.
    pub forced_colors: Option<bool>,
}

impl Default for Environment {
    /// Ein Desktop-Fenster, helles Schema, Bewegung erlaubt.
    fn default() -> Self {
        Environment {
            width: 1280.0,
            height: 800.0,
            dark: false,
            reduced_motion: false,
            hover: None,
            fine_pointer: None,
            contrast_more: None,
            forced_colors: None,
        }
    }
}

/// Gilt die Medienbedingung (eine Liste, durch Kommas getrennt) in `env`?
pub(crate) fn eval(condition: &str, env: &Environment) -> Match {
    let mut ergebnis = Match::No;
    for q in parse_media_query_list(condition) {
        let t = match q {
            Ok(q) => query(&q, env),
            Err(_) => Match::Unknown,
        };
        ergebnis = ergebnis.oder(t);
    }
    ergebnis
}

fn query(q: &MediaQuery, env: &Environment) -> Match {
    match q {
        MediaQuery::Condition(c) => bedingung(c, env),
        MediaQuery::TypeQuery {
            modifier,
            media_type,
            condition,
        } => {
            let typ = match media_type.0.to_ascii_lowercase().as_str() {
                "all" | "screen" => Match::Yes,
                "print" | "speech" | "tty" | "tv" | "projection" | "handheld" | "braille"
                | "embossed" | "aural" => Match::No,
                _ => Match::Unknown,
            };
            let t = match condition {
                Some(MediaConditionWithoutOr::Not(p)) => typ.und(in_klammern(p, env).nicht()),
                Some(MediaConditionWithoutOr::And(ps)) => {
                    ps.iter().fold(typ, |t, p| t.und(in_klammern(p, env)))
                }
                Some(_) => Match::Unknown,
                None => typ,
            };
            match modifier {
                Some(MediaModifier::Not) => t.nicht(),
                _ => t,
            }
        }
        _ => Match::Unknown,
    }
}

fn bedingung(c: &MediaCondition, env: &Environment) -> Match {
    match c {
        MediaCondition::Not(p) => in_klammern(p, env).nicht(),
        MediaCondition::And(ps) => ps
            .iter()
            .fold(Match::Yes, |t, p| t.und(in_klammern(p, env))),
        MediaCondition::Or(ps) => ps
            .iter()
            .fold(Match::No, |t, p| t.oder(in_klammern(p, env))),
        _ => Match::Unknown,
    }
}

fn in_klammern(p: &MediaInParens, env: &Environment) -> Match {
    match p {
        MediaInParens::Condition(c) => bedingung(c, env),
        MediaInParens::Feature(f) => merkmal(f, env),
        _ => Match::Unknown,
    }
}

fn px(v: &MfValue) -> Option<f64> {
    match v {
        MfValue::Dimension { value, unit } => match unit.to_ascii_lowercase().as_str() {
            "px" => Some(*value),
            "em" | "rem" => Some(*value * 16.0),
            _ => None,
        },
        MfValue::Number(n) if *n == 0.0 => Some(0.0),
        _ => None,
    }
}

fn vergleich(ist: f64, op: MfComparison, soll: f64) -> Match {
    let b = match op {
        MfComparison::Lt => ist < soll,
        MfComparison::Le => ist <= soll,
        MfComparison::Gt => ist > soll,
        MfComparison::Ge => ist >= soll,
        MfComparison::Eq => ist == soll,
        _ => return Match::Unknown,
    };
    if b { Match::Yes } else { Match::No }
}

fn umgekehrt(op: MfComparison) -> MfComparison {
    match op {
        MfComparison::Lt => MfComparison::Gt,
        MfComparison::Le => MfComparison::Ge,
        MfComparison::Gt => MfComparison::Lt,
        MfComparison::Ge => MfComparison::Le,
        o => o,
    }
}

fn abmessung(name: &str, env: &Environment) -> Option<f64> {
    match name {
        "width" => Some(f64::from(env.width)),
        "height" => Some(f64::from(env.height)),
        _ => None,
    }
}

/// Die Antwort, wenn die Umgebung den Wert festlegt, sonst `Unknown`.
fn festgelegt(wert: Option<bool>, frage: impl Fn(bool) -> bool) -> Match {
    match wert {
        Some(b) if frage(b) => Match::Yes,
        Some(_) => Match::No,
        None => Match::Unknown,
    }
}

fn merkmal(f: &MediaFeature, env: &Environment) -> Match {
    match f {
        MediaFeature::Boolean(n) => match n.0.to_ascii_lowercase().as_str() {
            "color" => Match::Yes,
            "monochrome" | "grid" => Match::No,
            _ => Match::Unknown,
        },
        MediaFeature::Plain { name, value } => {
            let name = name.0.to_ascii_lowercase();
            if let Some(dim) = name.strip_prefix("min-") {
                return match (abmessung(dim, env), px(value)) {
                    (Some(ist), Some(soll)) => vergleich(ist, MfComparison::Ge, soll),
                    _ => Match::Unknown,
                };
            }
            if let Some(dim) = name.strip_prefix("max-") {
                return match (abmessung(dim, env), px(value)) {
                    (Some(ist), Some(soll)) => vergleich(ist, MfComparison::Le, soll),
                    _ => Match::Unknown,
                };
            }
            if let Some(ist) = abmessung(&name, env) {
                return px(value).map_or(Match::Unknown, |s| vergleich(ist, MfComparison::Eq, s));
            }
            let MfValue::Ident(v) = value else {
                return Match::Unknown;
            };
            let v = v.to_ascii_lowercase();
            let ja = |b: bool| if b { Match::Yes } else { Match::No };
            match (name.as_str(), v.as_str()) {
                ("prefers-color-scheme", "dark") => ja(env.dark),
                ("prefers-color-scheme", "light") => ja(!env.dark),
                ("prefers-reduced-motion", "reduce") => ja(env.reduced_motion),
                ("prefers-reduced-motion", "no-preference") => ja(!env.reduced_motion),
                ("orientation", "landscape") => ja(env.width >= env.height),
                ("orientation", "portrait") => ja(env.width < env.height),
                ("hover" | "any-hover", "hover") => festgelegt(env.hover, |b| b),
                ("hover" | "any-hover", "none") => festgelegt(env.hover, |b| !b),
                ("pointer" | "any-pointer", "fine") => festgelegt(env.fine_pointer, |b| b),
                // „grob" und „keiner" unterscheidet die Umgebung nicht.
                ("pointer" | "any-pointer", "coarse" | "none") => match env.fine_pointer {
                    Some(true) => Match::No,
                    _ => Match::Unknown,
                },
                ("prefers-contrast", "no-preference") => festgelegt(env.contrast_more, |b| !b),
                ("prefers-contrast", "more") => festgelegt(env.contrast_more, |b| b),
                ("prefers-contrast", "less" | "custom") => match env.contrast_more {
                    Some(false) => Match::No,
                    _ => Match::Unknown,
                },
                ("forced-colors", "none") => festgelegt(env.forced_colors, |b| !b),
                ("forced-colors", "active") => festgelegt(env.forced_colors, |b| b),
                ("display-mode", "browser") => Match::Yes,
                ("display-mode", _) => Match::No,
                ("scripting", "enabled") => Match::Yes,
                _ => Match::Unknown,
            }
        }
        MediaFeature::Range(r) => match r {
            MfRange::NameFirst {
                name,
                operator,
                value,
            } => match (abmessung(&name.0.to_ascii_lowercase(), env), px(value)) {
                (Some(ist), Some(soll)) => vergleich(ist, *operator, soll),
                _ => Match::Unknown,
            },
            MfRange::ValueFirst {
                value,
                operator,
                name,
            } => match (abmessung(&name.0.to_ascii_lowercase(), env), px(value)) {
                (Some(ist), Some(soll)) => vergleich(ist, umgekehrt(*operator), soll),
                _ => Match::Unknown,
            },
            MfRange::Interval {
                lower,
                lower_inclusive,
                name,
                upper_inclusive,
                upper,
                ..
            } => {
                let (Some(ist), Some(lo), Some(hi)) = (
                    abmessung(&name.0.to_ascii_lowercase(), env),
                    px(lower),
                    px(upper),
                ) else {
                    return Match::Unknown;
                };
                let (lo_op, hi_op) = (
                    if *lower_inclusive {
                        MfComparison::Ge
                    } else {
                        MfComparison::Gt
                    },
                    if *upper_inclusive {
                        MfComparison::Le
                    } else {
                        MfComparison::Lt
                    },
                );
                vergleich(ist, lo_op, lo).und(vergleich(ist, hi_op, hi))
            }
            _ => Match::Unknown,
        },
        _ => Match::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(c: &str) -> Match {
        eval(c, &Environment::default())
    }

    #[test]
    fn typen() {
        assert_eq!(e("screen"), Match::Yes);
        assert_eq!(e("print"), Match::No);
        assert_eq!(e("not print"), Match::Yes);
        assert_eq!(e("print, screen"), Match::Yes);
    }

    #[test]
    fn breiten() {
        assert_eq!(e("(min-width: 768px)"), Match::Yes);
        assert_eq!(e("(max-width: 600px)"), Match::No);
        assert_eq!(
            e("screen and (min-width: 640px) and (max-width: 1440px)"),
            Match::Yes
        );
        assert_eq!(e("(width >= 1281px)"), Match::No);
        assert_eq!(e("(600px <= width <= 1000px)"), Match::No);
        assert_eq!(e("(min-width: 40em)"), Match::Yes);
    }

    #[test]
    fn praeferenzen() {
        assert_eq!(e("(prefers-color-scheme: dark)"), Match::No);
        assert_eq!(e("(prefers-reduced-motion: no-preference)"), Match::Yes);
        let dunkel = Environment {
            dark: true,
            ..Default::default()
        };
        assert_eq!(eval("(prefers-color-scheme: dark)", &dunkel), Match::Yes);
    }

    #[test]
    fn eingabe_und_kontrast_nur_wenn_die_umgebung_sie_festlegt() {
        assert_eq!(e("(hover: hover)"), Match::Unknown);
        assert_eq!(e("(pointer: fine)"), Match::Unknown);
        assert_eq!(e("(prefers-contrast: more)"), Match::Unknown);
        assert_eq!(e("(forced-colors: none)"), Match::Unknown);
        let maus = Environment {
            hover: Some(true),
            fine_pointer: Some(true),
            contrast_more: Some(false),
            forced_colors: Some(false),
            ..Default::default()
        };
        assert_eq!(eval("(hover: hover)", &maus), Match::Yes);
        assert_eq!(eval("(hover: none)", &maus), Match::No);
        assert_eq!(eval("(pointer: coarse)", &maus), Match::No);
        assert_eq!(eval("(prefers-contrast: more)", &maus), Match::No);
        assert_eq!(eval("(prefers-contrast: no-preference)", &maus), Match::Yes);
        assert_eq!(eval("(forced-colors: active)", &maus), Match::No);
    }

    #[test]
    fn unbekanntes_bleibt_unbekannt() {
        assert_eq!(e("(resolution: 2dppx)"), Match::Unknown);
        assert_eq!(e("(min-resolution: 192dpi)"), Match::Unknown);
        assert_eq!(e("("), Match::Unknown);
    }
}
