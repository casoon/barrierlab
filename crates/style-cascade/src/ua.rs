//! Das Stylesheet des Browsers (HTML, „Rendering"), soweit es die
//! Eigenschaften betrifft, die die Kaskade auswertet. Es steht unter jeder
//! Angabe der Seite.

use a11y_dom::{Node, ancestors};

use crate::color::parse_color;
use crate::shorthand::Expanded;

/// Was das UA-Stylesheet für `prop` an diesem Element festlegt.
pub(crate) fn declared<'a, N: Node<'a>>(n: N, prop: &str) -> Option<String> {
    let tag = n.local_name().to_ascii_lowercase();
    let t = tag.as_str();
    let link = matches!(t, "a" | "area") && n.has_attr("href");
    let s = |v: &str| Some(v.to_string());
    match prop {
        "display" => s(display(n, t)),
        "color" if link => s("#0000ee"),
        "color" if matches!(t, "input" | "textarea" | "select" | "button") => s("#000000"),
        "color" if t == "mark" => s("#000000"),
        "background-color" if t == "mark" => s("#ffff00"),
        "background-color" if matches!(t, "input" | "textarea" | "select") => s("#ffffff"),
        "background-color" if t == "button" => s("#efefef"),
        "text-decoration-line" if link => s("underline"),
        "text-decoration-line" if matches!(t, "u" | "ins") => s("underline"),
        "text-decoration-line" if matches!(t, "s" | "strike" | "del") => s("line-through"),
        "font-weight"
            if matches!(
                t,
                "b" | "strong" | "th" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            ) =>
        {
            s("bold")
        }
        "font-style" if matches!(t, "i" | "em" | "cite" | "dfn" | "var" | "address") => s("italic"),
        "font-size" => match t {
            "h1" => s("2em"),
            "h2" => s("1.5em"),
            "h3" => s("1.17em"),
            "h5" => s("0.83em"),
            "h6" => s("0.67em"),
            "small" | "sub" | "sup" => s("smaller"),
            "big" => s("larger"),
            "input" | "textarea" | "select" | "button" => s("13.3333px"),
            _ => None,
        },
        "font-family" if matches!(t, "code" | "kbd" | "samp" | "pre" | "tt") => s("monospace"),
        "list-style-type" => match t {
            "ol" => s("decimal"),
            "ul" | "menu" | "dir" => Some(
                match a11y_dom::ancestors(n)
                    .filter(|a| matches!(a.local_name(), "ul" | "menu" | "dir"))
                    .count()
                {
                    0 => "disc",
                    1 => "circle",
                    _ => "square",
                }
                .to_string(),
            ),
            _ => None,
        },
        _ => None,
    }
}

/// Eine Farbe in einem Darstellungsattribut (`bgcolor="#ff6600"`). Nur die
/// Schreibungen, die sich ohne den alten Parse-Algorithmus (HTML „rules for
/// parsing a legacy colour value") eindeutig lesen lassen; sonst unbekannt.
fn alte_farbe(v: &str) -> Expanded {
    let v = v.trim();
    if parse_color(v, false).is_some() {
        return Expanded::Set(v.to_string());
    }
    let hex = v.strip_prefix('#').unwrap_or(v);
    if matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Expanded::Set(format!("#{hex}"));
    }
    Expanded::Unknown
}

/// Darstellungsattribute (HTML „Rendering", Presentational hints): wirken
/// wie eine Angabe des Autors mit Spezifität 0, unter jeder Regel.
pub(crate) fn hint<'a, N: Node<'a>>(n: N, prop: &str) -> Option<Expanded> {
    let tag = n.local_name().to_ascii_lowercase();
    let t = tag.as_str();
    match prop {
        "background-color"
            if matches!(
                t,
                "body" | "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th"
            ) =>
        {
            n.attr("bgcolor").map(alte_farbe)
        }
        // Ein Hintergrundbild: nicht abbildbar, aber vorhanden.
        "background-image" if matches!(t, "body" | "table" | "td" | "th") => n
            .attr("background")
            .filter(|b| !b.trim().is_empty())
            .map(|_| Expanded::Set("url(#)".to_string())),
        "color" if t == "font" => n.attr("color").map(alte_farbe),
        "color" if t == "body" => n.attr("text").map(alte_farbe),
        "color" if matches!(t, "a" | "area") && n.has_attr("href") => ancestors(n)
            .find(|a| a.is_element("body"))
            .and_then(|b| b.attr("link"))
            .map(alte_farbe),
        "font-size" if t == "font" => n.attr("size").map(|v| match v.trim() {
            "1" => Expanded::Set("10px".to_string()),
            "2" => Expanded::Set("13px".to_string()),
            "3" => Expanded::Set("16px".to_string()),
            "4" => Expanded::Set("18px".to_string()),
            "5" => Expanded::Set("24px".to_string()),
            "6" => Expanded::Set("32px".to_string()),
            "7" => Expanded::Set("48px".to_string()),
            _ => Expanded::Unknown,
        }),
        _ => None,
    }
}

fn display<'a, N: Node<'a>>(n: N, t: &str) -> &'static str {
    if n.has_attr("hidden") && n.attr("hidden") != Some("until-found") {
        return "none";
    }
    match t {
        "head" | "title" | "style" | "script" | "meta" | "link" | "template" | "noscript"
        | "datalist" | "area" | "param" | "source" | "track" | "base" => "none",
        "audio" if !n.has_attr("controls") => "none",
        "dialog" if !n.has_attr("open") => "none",
        "li" => "list-item",
        "table" => "table",
        "tr" => "table-row",
        "td" | "th" => "table-cell",
        "thead" => "table-header-group",
        "tbody" => "table-row-group",
        "tfoot" => "table-footer-group",
        "caption" => "table-caption",
        "col" => "table-column",
        "colgroup" => "table-column-group",
        "html" | "body" | "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol"
        | "dl" | "dt" | "dd" | "section" | "article" | "aside" | "nav" | "header" | "footer"
        | "main" | "address" | "blockquote" | "figure" | "figcaption" | "form" | "fieldset"
        | "legend" | "hr" | "pre" | "details" | "summary" | "dialog" | "menu" | "dir"
        | "center" | "hgroup" | "search" | "optgroup" | "option" | "listing" | "xmp"
        | "plaintext" => "block",
        _ => "inline",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a11y_dom::{Arena, elements};

    #[test]
    fn standardwerte() {
        let d = Arena::builder()
            .open("html")
            .open("body")
            .open("a")
            .attr("href", "/x")
            .close()
            .open("a")
            .close()
            .open("h1")
            .close()
            .open("span")
            .attr("hidden", "")
            .close()
            .close()
            .close()
            .build();
        let el: Vec<_> = elements(&d).collect();
        assert_eq!(declared(el[2], "color").as_deref(), Some("#0000ee"));
        assert_eq!(declared(el[3], "color"), None);
        assert_eq!(declared(el[4], "font-size").as_deref(), Some("2em"));
        assert_eq!(declared(el[4], "display").as_deref(), Some("block"));
        assert_eq!(declared(el[5], "display").as_deref(), Some("none"));
        assert_eq!(declared(el[3], "display").as_deref(), Some("inline"));
    }
}
