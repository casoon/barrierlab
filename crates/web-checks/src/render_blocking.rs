//! Was den Aufbau einer Seite aufhält: synchrone Skripte und Stylesheets im
//! `<head>`.
//!
//! Der Host sammelt die Elemente und ihre Attribute, dieses Modul urteilt.
//! Keine Formulierung, keine Schwere.

/// MIME-Typen, die ein `<script>` als klassisches JavaScript ausführen lässt
/// (WHATWG HTML, „JavaScript MIME type essence match").
const JAVASCRIPT: [&str; 16] = [
    "application/ecmascript",
    "application/javascript",
    "application/x-ecmascript",
    "application/x-javascript",
    "text/ecmascript",
    "text/javascript",
    "text/javascript1.0",
    "text/javascript1.1",
    "text/javascript1.2",
    "text/javascript1.3",
    "text/javascript1.4",
    "text/javascript1.5",
    "text/jscript",
    "text/livescript",
    "text/x-ecmascript",
    "text/x-javascript",
];

/// Hält ein `<script src>` im `<head>` den Aufbau an?
///
/// Ja, wenn es ein klassisches Skript ist (kein `type` oder ein
/// JavaScript-MIME-Typ, Groß- und Kleinschreibung gleichgültig) und weder
/// `async` noch `defer` trägt. `type="module"` wird wie `defer` ausgeführt und
/// hält nie auf; ein Datenblock (`application/ld+json`, `importmap`,
/// `speculationrules`, …) wird nicht ausgeführt und hält ebenfalls nicht auf.
///
/// ```
/// use web_checks::render_blocking::is_blocking_script;
/// assert!(is_blocking_script(None, false, false));
/// assert!(!is_blocking_script(Some("MODULE"), false, false));
/// assert!(!is_blocking_script(None, true, false));
/// ```
pub fn is_blocking_script(type_attr: Option<&str>, has_async: bool, has_defer: bool) -> bool {
    if has_async || has_defer {
        return false;
    }
    match type_attr.map(str::trim) {
        None | Some("") => true,
        Some(t) => JAVASCRIPT.iter().any(|js| t.eq_ignore_ascii_case(js)),
    }
}

/// Hält ein `<link rel="stylesheet">` den Aufbau an, nach seinem
/// `media`-Attribut? Ja bei fehlendem oder leerem Attribut, `all` und
/// `screen`; jede andere Angabe (`print`, `(min-width: 800px)`, …) zählt hier
/// als nicht aufhaltend. Eine Medienabfrage auszuwerten ist Sache des Hosts.
///
/// ```
/// use web_checks::render_blocking::is_blocking_stylesheet_media;
/// assert!(is_blocking_stylesheet_media(None));
/// assert!(is_blocking_stylesheet_media(Some(" Screen ")));
/// assert!(!is_blocking_stylesheet_media(Some("print")));
/// ```
pub fn is_blocking_stylesheet_media(media: Option<&str>) -> bool {
    match media.map(str::trim) {
        None | Some("") => true,
        Some(m) => m.eq_ignore_ascii_case("all") || m.eq_ignore_ascii_case("screen"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jede_attributkombination() {
        for (typ, a, d, erwartet) in [
            (None, false, false, true),
            (None, true, false, false),
            (None, false, true, false),
            (None, true, true, false),
            (Some(""), false, false, true),
            (Some("text/javascript"), false, false, true),
            (Some("text/javascript"), true, false, false),
            (Some("module"), false, false, false),
            (Some("module"), true, true, false),
        ] {
            assert_eq!(is_blocking_script(typ, a, d), erwartet, "{typ:?} {a} {d}");
        }
    }

    #[test]
    fn type_module_in_jeder_schreibung() {
        for t in ["module", "MODULE", "Module", " module "] {
            assert!(!is_blocking_script(Some(t), false, false), "{t}");
        }
    }

    #[test]
    fn javascript_typen_und_datenbloecke() {
        assert!(is_blocking_script(
            Some("Application/JavaScript"),
            false,
            false
        ));
        assert!(is_blocking_script(Some(" text/ecmascript "), false, false));
        for t in [
            "application/ld+json",
            "importmap",
            "speculationrules",
            "text/template",
        ] {
            assert!(!is_blocking_script(Some(t), false, false), "{t}");
        }
    }

    #[test]
    fn stylesheet_media() {
        for m in [
            None,
            Some(""),
            Some("  "),
            Some("all"),
            Some("ALL"),
            Some("screen"),
        ] {
            assert!(is_blocking_stylesheet_media(m), "{m:?}");
        }
        for m in ["print", "(min-width: 800px)", "screen and (min-width: 1px)"] {
            assert!(!is_blocking_stylesheet_media(Some(m)), "{m}");
        }
    }
}
