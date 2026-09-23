//! Rollenberechnung nach [HTML-AAM](https://www.w3.org/TR/html-aam-1.0/).
//!
//! Die explizite `role`-Auszeichnung hat Vorrang, sofern sie eine gültige,
//! nicht abstrakte Rolle benennt. Andernfalls gilt die implizite Rolle des
//! HTML-Elements.

use a11y_dom::{ancestors, Node};

/// Gültige, nicht abstrakte Rollen aus WAI-ARIA 1.2.
const VALID: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

/// Rollen, deren Name sich aus dem eigenen Inhalt speist
/// ([ARIA: name from author *and* contents](https://w3c.github.io/aria/#namefromcontent)).
const NAME_FROM_CONTENT: &[&str] = &[
    "button",
    "cell",
    "checkbox",
    "columnheader",
    "gridcell",
    "heading",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "radio",
    "row",
    "rowheader",
    "sectionhead",
    "switch",
    "tab",
    "tooltip",
    "treeitem",
];

/// Rollen, für die ein Accessible Name laut ARIA **verboten** ist. Autoren
/// dürfen sie nicht benennen; ein `aria-label` daran wird ignoriert.
const NAME_PROHIBITED: &[&str] = &[
    "caption",
    "code",
    "deletion",
    "emphasis",
    "generic",
    "insertion",
    "paragraph",
    "presentation",
    "none",
    "strong",
    "subscript",
    "superscript",
    "term",
    "time",
];

/// Ob der Name dieser Rolle aus dem Inhalt des Elements gebildet wird.
pub fn allows_name_from_content(role: &str) -> bool {
    NAME_FROM_CONTENT.contains(&role)
}

/// Ob ein Accessible Name für diese Rolle verboten ist.
pub fn name_is_prohibited(role: &str) -> bool {
    NAME_PROHIBITED.contains(&role)
}

/// Die berechnete Rolle eines Knotens.
///
/// `None` bedeutet: keine Rolle abgeleitet. Das ist etwas anderes als
/// `Some("none")` — Letzteres ist die ausdrückliche Auszeichnung als
/// präsentational.
pub fn role<'a, N: Node<'a>>(node: N) -> Option<&'static str> {
    if let Some(explicit) = node.attr("role") {
        // Mehrere Rollen sind ein Fallback-Stapel: die erste gültige gewinnt.
        for token in explicit.split_whitespace() {
            if let Some(known) = VALID.iter().find(|v| **v == token) {
                return Some(known);
            }
        }
    }
    implicit(node)
}

/// Die implizite Rolle des HTML-Elements, ohne Rücksicht auf `role`.
pub fn implicit<'a, N: Node<'a>>(node: N) -> Option<&'static str> {
    let tag = node.local_name();
    Some(match tag {
        "a" | "area" => {
            if node.has_attr("href") {
                "link"
            } else {
                "generic"
            }
        }
        "article" => "article",
        "aside" => "complementary",
        "blockquote" => "blockquote",
        "button" => "button",
        "caption" => "caption",
        "code" => "code",
        "datalist" => "listbox",
        "dd" => "definition",
        "del" => "deletion",
        "details" => "group",
        "dfn" => "term",
        "dialog" => "dialog",
        "dt" => "term",
        "em" => "emphasis",
        "fieldset" => "group",
        "figure" => "figure",
        "form" => "form",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "hr" => "separator",
        "html" => "document",
        "img" => match node.attr("alt") {
            // Leeres alt ist die ausdrückliche Auszeichnung als dekorativ.
            Some("") => "presentation",
            _ => "img",
        },
        "input" => return input_role(node),
        "ins" => "insertion",
        "li" => "listitem",
        "main" => "main",
        "mark" => "mark",
        "menu" | "ul" | "ol" => "list",
        "meter" => "meter",
        "nav" => "navigation",
        "optgroup" => "group",
        "option" => "option",
        "output" => "status",
        "p" => "paragraph",
        "progress" => "progressbar",
        "search" => "search",
        "select" => {
            // Mehrfachauswahl oder aufgeklappt: Listbox. Sonst Combobox.
            if node.has_attr("multiple")
                || node
                    .attr("size")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .is_some_and(|s| s > 1)
            {
                "listbox"
            } else {
                "combobox"
            }
        }
        "strong" => "strong",
        "sub" => "subscript",
        "summary" => "button",
        "sup" => "superscript",
        "table" => "table",
        "tbody" | "tfoot" | "thead" => "rowgroup",
        "td" => "cell",
        "textarea" => "textbox",
        "th" => {
            // scope entscheidet; ohne scope ist die Spaltenüberschrift die
            // häufigere und vom Browser bevorzugte Auslegung.
            match node.attr("scope") {
                Some("row") | Some("rowgroup") => "rowheader",
                _ => "columnheader",
            }
        }
        "time" => "time",
        "tr" => "row",
        // header und footer sind nur dann Landmarks, wenn sie nicht in einem
        // sektionierenden Element stecken.
        "header" => {
            if in_sectioning_content(node) {
                "generic"
            } else {
                "banner"
            }
        }
        "footer" => {
            if in_sectioning_content(node) {
                "generic"
            } else {
                "contentinfo"
            }
        }
        // section ist nur dann eine Landmark, wenn sie benannt ist.
        "section" => {
            if node.has_attr("aria-label") || node.has_attr("aria-labelledby") {
                "region"
            } else {
                "generic"
            }
        }
        "div" | "span" => "generic",
        _ => return None,
    })
}

fn in_sectioning_content<'a, N: Node<'a>>(node: N) -> bool {
    ancestors(node).any(|a| matches!(a.local_name(), "article" | "aside" | "nav" | "section"))
}

fn input_role<'a, N: Node<'a>>(node: N) -> Option<&'static str> {
    let ty = node
        .attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "text".into());
    Some(match ty.as_str() {
        "button" | "image" | "reset" | "submit" => "button",
        "checkbox" => "checkbox",
        "email" | "tel" | "text" | "url" => {
            // Mit Vorschlagsliste wird daraus eine Combobox.
            if node.has_attr("list") {
                "combobox"
            } else {
                "textbox"
            }
        }
        "number" => "spinbutton",
        "radio" => "radio",
        "range" => "slider",
        "search" => {
            if node.has_attr("list") {
                "combobox"
            } else {
                "searchbox"
            }
        }
        // password, color, date, datetime-local, file, month, time, week und
        // hidden haben laut HTML-AAM keine Rolle.
        _ => return None,
    })
}
