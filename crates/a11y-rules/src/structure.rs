//! Tier-1-Regeln: entscheidbar allein aus Tags, Attributen, Text und Hierarchie.

use std::collections::{HashMap, HashSet};

use a11y_dom::{closest, elements, subtree_text, Document, Node, NodeId, Tier};
use a11y_report::{Finding, Location, Severity};

use crate::registry::{Meta, StructureRule};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Alle gültigen ARIA-Rollen aus WAI-ARIA 1.2, ohne die abstrakten.
pub(crate) const VALID_ROLES: &[&str] = &[
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

/// Abstrakte Rollen. Sie beschreiben die Taxonomie und dürfen nie im Markup stehen.
const ABSTRACT_ROLES: &[&str] = &[
    "command",
    "composite",
    "input",
    "landmark",
    "range",
    "roletype",
    "section",
    "sectionhead",
    "select",
    "structure",
    "widget",
    "window",
];

fn heading_level(tag: &str) -> Option<u8> {
    let b = tag.as_bytes();
    (b.len() == 2 && b[0] == b'h' && b[1].is_ascii_digit() && (b'1'..=b'6').contains(&b[1]))
        .then(|| b[1] - b'0')
}

// --- Dokument -------------------------------------------------------------

fn lang<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let root = doc.root();
    if !root.is_element("html") {
        return;
    }
    match root.attr("lang") {
        None => out.push(
            Finding::fail(
                "document/lang-missing",
                "Das <html>-Element hat kein lang-Attribut.",
            )
            .with_severity(Severity::High)
            .with_wcag(["3.1.1"])
            .at(at(root.id())),
        ),
        Some(l) if !is_plausible_lang(l) => out.push(
            Finding::fail(
                "document/lang-invalid",
                format!("Der Sprachcode \"{l}\" ist kein gültiges BCP-47-Kürzel."),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.1.1"])
            .at(at(root.id())),
        ),
        _ => {}
    }
}

/// Grobprüfung auf BCP 47: Primärkennung aus zwei oder drei Buchstaben,
/// danach nur alphanumerische Untertags. Keine Registry-Prüfung — dafür
/// bräuchte es die IANA-Liste, und die gehört nicht in dieses Crate.
fn is_plausible_lang(l: &str) -> bool {
    let l = l.trim();
    let mut parts = l.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn title<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    match elements(doc).find(|n| n.is_element("title")) {
        None => out.push(
            Finding::fail(
                "document/title-missing",
                "Das Dokument hat kein <title>-Element.",
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.2"]),
        ),
        Some(t) if subtree_text(t).trim().is_empty() => out.push(
            Finding::fail("document/title-empty", "Das <title>-Element ist leer.")
                .with_severity(Severity::High)
                .with_wcag(["2.4.2"])
                .at(at(t.id())),
        ),
        _ => {}
    }
}

fn viewport<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let mut gefunden = false;
    for n in elements(doc).filter(|n| n.is_element("meta")) {
        if n.attr("name") != Some("viewport") {
            continue;
        }
        gefunden = true;
        let Some(content) = n.attr("content") else {
            continue;
        };
        // Kleinschreiben vor dem Vergleich: HTML-Attributwerte sind nicht
        // normiert, und `user-scalable=NO` sperrt den Zoom genauso.
        let c = content.to_ascii_lowercase().replace(' ', "");
        let gesperrt = c.contains("user-scalable=no") || c.contains("user-scalable=0");
        let max = c.split(',').find_map(|p| {
            p.strip_prefix("maximum-scale=")
                .and_then(|v| v.parse::<f32>().ok())
        });

        // Unter 200 %: verletzt 1.4.4 unmittelbar.
        if gesperrt || max.is_some_and(|v| v < 2.0) {
            out.push(
                Finding::fail(
                    "zoom/viewport-locked",
                    "Der Viewport verhindert oder begrenzt das Zoomen.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.4.4"])
                .at(at(n.id())),
            );
            continue;
        }

        // Zwischen 200 % und 500 %: 1.4.4 ist erfüllt, aber die Begrenzung
        // trifft alle, die stärker vergrößern müssen. Eigene Kennung statt
        // derselben, weil der eine Fall ein Verstoß ist und der andere nicht.
        if max.is_some_and(|v| v < 5.0) {
            out.push(
                Finding::fail(
                    "zoom/viewport-scale-limited",
                    "Der Viewport begrenzt die Vergrößerung auf weniger als 500 %.",
                )
                .with_severity(Severity::Low)
                .with_wcag(["1.4.4"])
                .at(at(n.id())),
            );
        }
    }

    // Ohne Viewport-Angabe legen mobile Browser eine Desktop-Breite zugrunde
    // und verkleinern die Seite. Der Text landet damit unter jeder lesbaren
    // Größe, und Zoomen holt ihn nur teilweise zurück.
    if !gefunden {
        out.push(
            Finding::fail(
                "zoom/viewport-missing",
                "Das Dokument hat keine Viewport-Angabe.",
            )
            .with_severity(Severity::High)
            .with_wcag(["1.4.4", "1.4.10"])
            .at(at(doc.root().id())),
        );
    }
}

// --- Überschriften --------------------------------------------------------

fn headings<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let mut last = 0u8;
    let mut any = false;
    let mut h1s: Vec<NodeId> = Vec::new();

    for n in elements(doc) {
        let Some(level) = heading_level(n.local_name()) else {
            continue;
        };
        any = true;
        if level == 1 {
            h1s.push(n.id());
        }
        if subtree_text(n).trim().is_empty() && !n.has_attr("aria-label") {
            out.push(
                Finding::fail("headings/empty", "Die Überschrift hat keinen Text.")
                    .with_severity(Severity::Medium)
                    .with_wcag(["1.3.1", "2.4.6"])
                    .at(at(n.id())),
            );
        }
        if last > 0 && level > last + 1 {
            out.push(
                Finding::fail(
                    "headings/skip-level",
                    format!("Die Gliederung springt von h{last} auf h{level}."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
        last = level;
    }

    if any && h1s.is_empty() {
        out.push(
            Finding::fail(
                "headings/h1-missing",
                "Das Dokument hat keine h1-Überschrift.",
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1"]),
        );
    }

    // Mehrere h1 sind in HTML zulässig und je nach Gliederung sogar richtig.
    // Meist sind sie es nicht — das ist eine Erwartung, kein Verstoß, also
    // REVIEW. Der Befund zeigt auf die zweite, die überzählige.
    if h1s.len() > 1 {
        out.push(
            Finding::review(
                "headings/h1-multiple",
                format!("Das Dokument hat {} h1-Überschriften.", h1s.len()),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.3.1"])
            .at(at(h1s[1])),
        );
    }
}

// --- ARIA: erforderliche Attribute ---------------------------------------

/// Welche Attribute eine Rolle zwingend braucht, damit ihr Zustand überhaupt
/// übermittelt wird. Eine Checkbox ohne `aria-checked` wird als Checkbox
/// angesagt, deren Zustand niemand erfährt.
const ERFORDERLICH: &[(&str, &[&str])] = &[
    ("checkbox", &["aria-checked"]),
    ("switch", &["aria-checked"]),
    ("radio", &["aria-checked"]),
    ("combobox", &["aria-expanded"]),
    (
        "slider",
        &["aria-valuenow", "aria-valuemin", "aria-valuemax"],
    ),
    ("spinbutton", &["aria-valuenow"]),
    ("scrollbar", &["aria-controls", "aria-valuenow"]),
    ("option", &["aria-selected"]),
];

fn aria_required_attributes<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(rolle) = n.attr("role").map(str::trim) else {
            continue;
        };
        // Bei mehreren Rollen gilt die erste, die der Browser kennt.
        let Some(erste) = rolle.split_whitespace().next() else {
            continue;
        };
        let Some((_, noetig)) = ERFORDERLICH
            .iter()
            .find(|(r, _)| r.eq_ignore_ascii_case(erste))
        else {
            continue;
        };

        let fehlend: Vec<&str> = noetig.iter().copied().filter(|a| !n.has_attr(a)).collect();
        if fehlend.is_empty() {
            continue;
        }
        out.push(
            Finding::fail(
                "aria/required-attribute-missing",
                format!(
                    "role=\"{erste}\" braucht {}, aber {} fehlt.",
                    noetig.join(", "),
                    fehlend.join(", ")
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- Landmarks ------------------------------------------------------------

/// Ob dieser Knoten eine Landmark der gesuchten Art ist.
///
/// Die Rolle zählt vor dem Tag: `<div role="main">` ist eine Main-Landmark,
/// `<main role="presentation">` ist keine.
fn ist_landmark<'a, N: Node<'a>>(n: N, tag: &str, rolle: &str) -> bool {
    match n.attr("role").map(str::trim) {
        Some(r) => r.split_whitespace().any(|x| x.eq_ignore_ascii_case(rolle)),
        None => n.is_element(tag),
    }
}

/// `<header>` und `<footer>` sind nur dann banner bzw. contentinfo, wenn sie
/// nicht in einem Sectioning-Element stecken — ein `<footer>` in einem
/// `<article>` gehört zu diesem Artikel, nicht zum Dokument. Ein `<div>`
/// dazwischen disqualifiziert dagegen nicht.
const SECTIONING: &[&str] = &["article", "aside", "main", "nav", "section"];

fn ist_dokumentweit<'a, N: Node<'a>>(n: N) -> bool {
    a11y_dom::ancestors(n).all(|a| !SECTIONING.contains(&a.local_name()))
}

fn landmarks<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let wurzel = doc.root().id();
    let mains: Vec<_> = elements(doc)
        .filter(|n| ist_landmark(*n, "main", "main"))
        .collect();

    match mains.len() {
        0 => out.push(
            Finding::fail(
                "landmarks/main-missing",
                "Das Dokument hat keine main-Landmark.",
            )
            .with_severity(Severity::High)
            .with_wcag(["1.3.1", "2.4.1"])
            .at(at(wurzel)),
        ),
        1 => {}
        n => out.push(
            Finding::fail(
                "landmarks/main-duplicate",
                format!("Das Dokument hat {n} main-Landmarks; genau eine ist zulässig."),
            )
            .with_severity(Severity::High)
            .with_wcag(["1.3.1"])
            // Die zweite ist die überzählige — dorthin zeigt der Befund.
            .at(at(mains[1].id())),
        ),
    }

    // Navigation, banner und contentinfo sind Erwartungen, keine beweisbaren
    // Verstöße: Eine Seite darf ohne Navigation auskommen. Deshalb REVIEW und
    // nicht FAIL -- eine dritte Achse „Gewissheit" gibt es bewusst nicht.
    for (tag, rolle, kennung, text) in [
        (
            "nav",
            "navigation",
            "landmarks/navigation-missing",
            "Das Dokument hat keine navigation-Landmark.",
        ),
        (
            "header",
            "banner",
            "landmarks/banner-missing",
            "Das Dokument hat keine banner-Landmark.",
        ),
        (
            "footer",
            "contentinfo",
            "landmarks/contentinfo-missing",
            "Das Dokument hat keine contentinfo-Landmark.",
        ),
    ] {
        let vorhanden = elements(doc).any(|n| {
            ist_landmark(n, tag, rolle) && (n.attr("role").is_some() || ist_dokumentweit(n))
        });
        if !vorhanden {
            out.push(
                Finding::review(kennung, text)
                    .with_severity(Severity::Low)
                    .with_wcag(["1.3.1", "2.4.1"])
                    .at(at(wurzel)),
            );
        }
    }
}

// --- Sprunglink -----------------------------------------------------------

/// Ob dieser Link nach Textlage ein Sprunglink ist.
///
/// Das ist eine Heuristik über Linktext, `class` und `id` — es gibt kein
/// Merkmal, an dem ein Sprunglink sicher zu erkennen wäre. Der Befund ist
/// deshalb `REVIEW`, nicht `FAIL`.
fn sieht_aus_wie_sprunglink<'a, N: Node<'a>>(n: N) -> bool {
    const MARKER: &[&str] = &[
        "skip",
        "sprung",
        "zum inhalt",
        "zum hauptinhalt",
        "direkt zum inhalt",
    ];
    let text = subtree_text(n).to_lowercase();
    let marker = format!(
        "{} {}",
        n.attr("class").unwrap_or(""),
        n.attr("id").unwrap_or("")
    )
    .to_lowercase();
    MARKER
        .iter()
        .any(|m| text.contains(m) || marker.contains(m))
}

fn skip_link<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let vorhanden = elements(doc).any(|n| {
        n.is_element("a")
            && n.attr("href")
                .is_some_and(|h| h.starts_with('#') && h.len() > 1)
            && sieht_aus_wie_sprunglink(n)
    });
    if !vorhanden {
        out.push(
            Finding::review(
                "keyboard/skip-link-missing",
                "Kein Sprunglink gefunden, der wiederkehrende Bereiche überspringt.",
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.4.1"])
            .at(at(doc.root().id())),
        );
    }
}

// --- Bilder ---------------------------------------------------------------

fn images<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| n.is_element("img")) {
        match n.attr("alt") {
            None => out.push(
                Finding::fail("images/alt-missing", "Das Bild hat kein alt-Attribut.")
                    .with_severity(Severity::High)
                    .with_wcag(["1.1.1"])
                    .at(at(n.id())),
            ),
            Some(alt) if suspicious_alt(alt) => out.push(
                // Heuristisch: der Text ist da, aber vermutlich nichtssagend.
                // Deshalb Review, nicht Fail.
                Finding::review(
                    "images/alt-suspicious",
                    format!("Der Alt-Text \"{alt}\" beschreibt das Bild vermutlich nicht."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            ),
            _ => {}
        }
    }
}

fn suspicious_alt(alt: &str) -> bool {
    let a = alt.trim().to_ascii_lowercase();
    if a.is_empty() {
        // Leeres alt ist die korrekte Auszeichnung für dekorative Bilder.
        return false;
    }
    const ENDINGS: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".webp", ".svg", ".avif"];
    const FILLERS: &[&str] = &[
        "bild",
        "image",
        "img",
        "foto",
        "photo",
        "grafik",
        "graphic",
        "icon",
        "logo",
        "picture",
        "spacer",
        "platzhalter",
        "placeholder",
    ];
    ENDINGS.iter().any(|e| a.ends_with(e)) || FILLERS.contains(&a.as_str())
}

// --- Formulare ------------------------------------------------------------

/// Die Label-Zuordnung ist vollständig strukturell entscheidbar: `label[for]`,
/// verschachteltes `<label>`, `aria-label`, `aria-labelledby`, `title`. Dafür
/// braucht es keine Accessible-Name-Berechnung.
fn form_labels<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let label_targets: HashSet<&str> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for"))
        .collect();

    for n in elements(doc) {
        let tag = n.local_name();
        if !matches!(tag, "input" | "select" | "textarea") {
            continue;
        }
        let ty = n.attr("type").unwrap_or("text").to_ascii_lowercase();
        if matches!(
            ty.as_str(),
            "hidden" | "submit" | "button" | "reset" | "image"
        ) {
            continue;
        }

        let aria_named = n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
            || n.has_attr("aria-labelledby");
        let for_labelled = n.attr("id").is_some_and(|id| label_targets.contains(id));
        let wrapped = closest(n, "label").is_some();
        let titled = n.attr("title").is_some_and(|v| !v.trim().is_empty());

        if !(aria_named || for_labelled || wrapped || titled) {
            out.push(
                Finding::fail("forms/label-missing", "Das Eingabefeld hat kein Label.")
                    .with_severity(Severity::Critical)
                    .with_wcag(["1.3.1", "3.3.2", "4.1.2"])
                    .at(at(n.id())),
            );
        } else if n.has_attr("placeholder") && !aria_named && !for_labelled && !wrapped {
            out.push(
                Finding::fail(
                    "forms/placeholder-as-label",
                    "Das Feld nutzt den Platzhalter anstelle eines Labels.",
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.3.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- ARIA -----------------------------------------------------------------

fn aria_roles<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(role) = n.attr("role") else { continue };
        for r in role.split_whitespace() {
            if ABSTRACT_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-abstract",
                        format!(
                            "\"{r}\" ist eine abstrakte Rolle und darf nicht ausgezeichnet werden."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            } else if !VALID_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-invalid",
                        format!("\"{r}\" ist keine gültige ARIA-Rolle."),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

fn aria_references<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let ids: HashSet<&str> = elements(doc).filter_map(|n| n.attr("id")).collect();

    for n in elements(doc) {
        for rel in [
            "aria-labelledby",
            "aria-describedby",
            "aria-controls",
            "aria-owns",
        ] {
            let Some(v) = n.attr(rel) else { continue };
            let fehlend: Vec<&str> = v
                .split_whitespace()
                .filter(|id| !ids.contains(id))
                .collect();
            if !fehlend.is_empty() {
                out.push(
                    Finding::fail(
                        "aria/reference-missing",
                        format!(
                            "{rel} verweist auf nicht vorhandene IDs: {}",
                            fehlend.join(", ")
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["1.3.1", "4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

// --- IDs ------------------------------------------------------------------

fn duplicate_ids<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for n in elements(doc) {
        let Some(id) = n.attr("id") else { continue };
        if id.is_empty() {
            continue;
        }
        let count = seen.entry(id).or_insert(0);
        *count += 1;
        if *count == 2 {
            out.push(
                Finding::fail(
                    "ids/duplicate",
                    format!("Die ID \"{id}\" kommt mehrfach vor."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["4.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Tastatur -------------------------------------------------------------

fn tabindex<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(raw) = n.attr("tabindex") else {
            continue;
        };
        let Ok(value) = raw.trim().parse::<i32>() else {
            continue;
        };
        if value > 0 {
            out.push(
                Finding::fail(
                    "keyboard/positive-tabindex",
                    format!("tabindex=\"{value}\" bricht die natürliche Tabreihenfolge."),
                )
                // Hoch, nicht mittel: Ein positiver tabindex bricht die
                // Tabreihenfolge reproduzierbar und für jeden, der mit der
                // Tastatur navigiert — das ist kein Schönheitsfehler.
                .with_severity(Severity::High)
                .with_wcag(["2.4.3"])
                .at(at(n.id())),
            );
        }
    }
}

/// Fokussierbar und zugleich vor dem Accessibility-Tree versteckt — Nutzer
/// landen mit der Tabtaste auf etwas, das ihnen nicht angesagt wird.
fn hidden_focusable<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if n.attr("aria-hidden") != Some("true") {
            continue;
        }
        let natively_focusable = matches!(
            n.local_name(),
            "a" | "button" | "input" | "select" | "textarea" | "summary" | "iframe"
        ) && !n.has_attr("disabled");
        let tab_focusable = n
            .attr("tabindex")
            .and_then(|t| t.trim().parse::<i32>().ok())
            .is_some_and(|v| v >= 0);

        if natively_focusable || tab_focusable {
            out.push(
                Finding::fail(
                    "keyboard/hidden-focusable",
                    "Das Element ist fokussierbar, aber per aria-hidden versteckt.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Listen und Tabellen --------------------------------------------------

/// Ob dieser Knoten eine Liste ist -- als Tag oder per `role`-Attribut.
///
/// Das `role`-Attribut gehoert zu Tier 1: Es steht im Markup und braucht keine
/// berechnete Semantik. Ohne diese Zeile waere `<div role="list">` fuer die
/// Regel unsichtbar, obwohl es fuer die Assistenztechnik eine Liste ist.
fn ist_liste<'a, N: Node<'a>>(n: N) -> bool {
    matches!(n.local_name(), "ul" | "ol") || n.attr("role") == Some("list")
}

/// Ob dieser Knoten als Listeneintrag zaehlt.
fn ist_listeneintrag<'a, N: Node<'a>>(n: N) -> bool {
    n.local_name() == "li" || n.attr("role") == Some("listitem")
}

fn list_structure<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !ist_liste(n) {
            continue;
        }
        // Der Autor sagt ausdruecklich, dass hier keine Liste gemeint ist.
        if matches!(n.attr("role"), Some("presentation") | Some("none")) {
            continue;
        }
        let tag = n.local_name();
        let kinder: Vec<_> = n
            .children()
            .filter(|c| c.kind() == a11y_dom::NodeKind::Element)
            .collect();

        let fremd = kinder
            .iter()
            .any(|c| !ist_listeneintrag(*c) && !matches!(c.local_name(), "script" | "template"));
        if fremd {
            out.push(
                Finding::fail(
                    "lists/invalid-structure",
                    format!("<{tag}> enthält direkte Kinder, die kein <li> sind."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }

        // Eine Liste ohne Eintraege kuendigt der Assistenztechnik eine
        // Struktur an, die es nicht gibt.
        if !kinder.iter().any(|c| ist_listeneintrag(*c)) {
            out.push(
                Finding::fail(
                    "lists/empty",
                    format!("<{tag}> enthält keine Listeneinträge."),
                )
                .with_severity(Severity::Low)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }

    beschreibungslisten(doc, out);
    verwaiste_eintraege(doc, out);
}

/// Ein Listeneintrag ohne Liste.
///
/// Die Prüfung oben läuft über Listen und sieht deshalb nur, was *in* einer
/// steht. Ein `<li>`, das gar keine Liste über sich hat, wird dabei nie
/// besucht — für die Assistenztechnik kündigt es aber eine Aufzählung an, die
/// es nicht gibt.
fn verwaiste_eintraege<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !ist_listeneintrag(n) {
            continue;
        }
        let in_liste = a11y_dom::ancestors(n).any(ist_liste);
        if !in_liste {
            out.push(
                Finding::fail(
                    "lists/item-outside-list",
                    "Der Listeneintrag steht außerhalb einer Liste.",
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ob dieser Knoten ein Definitionsbegriff ist — als `<dt>` oder per `role`.
fn ist_begriff<'a, N: Node<'a>>(n: N) -> bool {
    n.local_name() == "dt" || n.attr("role") == Some("term")
}

/// Ob dieser Knoten eine Definition ist.
fn ist_definition<'a, N: Node<'a>>(n: N) -> bool {
    n.local_name() == "dd" || n.attr("role") == Some("definition")
}

/// Ein Begriff ohne Definition.
///
/// In einer Beschreibungsliste gehört zu jedem `<dt>` mindestens ein `<dd>`.
/// Fehlt es, kündigt die Auszeichnung eine Zuordnung an, die es nicht gibt —
/// die Assistenztechnik liest einen Begriff vor, zu dem nichts folgt.
///
/// Geprüft wird unter demselben Elternknoten, weil HTML seit einiger Zeit auch
/// `<div>`-Gruppen innerhalb einer `<dl>` erlaubt. Ein `<dt>` in einer solchen
/// Gruppe braucht sein `<dd>` dort, nicht irgendwo in der Liste.
fn beschreibungslisten<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !ist_begriff(n) {
            continue;
        }
        let Some(eltern) = n.parent() else { continue };
        let hat_definition = eltern
            .children()
            .filter(|c| c.kind() == a11y_dom::NodeKind::Element)
            .any(ist_definition);

        if !hat_definition {
            out.push(
                Finding::fail(
                    "lists/term-without-definition",
                    "Zu diesem Begriff gibt es keine Definition.",
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ob im Teilbaum eine Kopfzelle steckt — als `<th>` oder per Rolle.
fn hat_kopfzelle<'a, N: Node<'a>>(n: N) -> bool {
    a11y_dom::descendants(n).any(|d| {
        d.is_element("th") || matches!(d.attr("role"), Some("columnheader") | Some("rowheader"))
    })
}

fn table_headers<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc)
        .filter(|n| n.is_element("table") || matches!(n.attr("role"), Some("table") | Some("grid")))
    {
        // Als präsentational ausgezeichnet: keine Datentabelle. Dann dürfen
        // dort aber auch keine Kopfzellen stehen — die Auszeichnung sagt „das
        // ist keine Tabelle", die Kopfzellen sagen das Gegenteil, und die
        // Assistenztechnik bekommt widersprüchliche Angaben.
        if matches!(n.attr("role"), Some("presentation") | Some("none")) {
            if hat_kopfzelle(n) {
                out.push(
                    Finding::fail(
                        "tables/presentational-with-headers",
                        "Die Tabelle ist als präsentational ausgezeichnet, enthält aber Kopfzellen.",
                    )
                    .with_severity(Severity::Medium)
                    .with_wcag(["1.3.1"])
                    .at(at(n.id())),
                );
            }
            continue;
        }

        // Ein Name macht aus einer Tabelle erst eine auffindbare Tabelle: Wer
        // sich eine Tabellenliste ausgeben lässt, sieht sonst nur „Tabelle".
        // Heuristisch, weil eine Tabelle ohne Namen nicht zwingend falsch ist
        // — deshalb Review, nicht Fail.
        let benannt = n.children().any(|c| c.is_element("caption"))
            || n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
            || n.has_attr("aria-labelledby");
        if !benannt {
            out.push(
                Finding::review(
                    "tables/name-missing",
                    "Die Tabelle hat weder <caption> noch aria-label.",
                )
                .with_severity(Severity::Low)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
        if !hat_kopfzelle(n) {
            out.push(
                Finding::fail(
                    "tables/header-missing",
                    "Die Tabelle hat keine <th>-Kopfzellen.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Die Deklarationen. Getrennt von der Zuordnung zu Funktionen, damit sie auch
/// ohne konkreten Host lesbar sind — ein Host, der diesen Tier nicht bedient,
/// muss die Kennungen trotzdem benennen können.
pub const METAS: &[Meta] = &[
    Meta {
        ids: &["document/lang-missing", "document/lang-invalid"],
        tier: Tier::Structure,
        wcag: &["3.1.1"],
        severity: Severity::High,
        help: "Das <html>-Element braucht ein gültiges lang-Attribut.",
    },
    Meta {
        ids: &["document/title-missing", "document/title-empty"],
        tier: Tier::Structure,
        wcag: &["2.4.2"],
        severity: Severity::High,
        help: "Jede Seite braucht einen aussagekräftigen <title>.",
    },
    Meta {
        ids: &[
            "zoom/viewport-locked",
            "zoom/viewport-scale-limited",
            "zoom/viewport-missing",
        ],
        tier: Tier::Structure,
        wcag: &["1.4.4", "1.4.10"],
        severity: Severity::High,
        help: "Der Viewport muss vorhanden sein und darf Zoomen nicht verhindern.",
    },
    Meta {
        ids: &[
            "headings/empty",
            "headings/skip-level",
            "headings/h1-missing",
            "headings/h1-multiple",
        ],
        tier: Tier::Structure,
        wcag: &["1.3.1", "2.4.6"],
        severity: Severity::Medium,
        help: "Überschriften bilden die Gliederung; Ebenen nicht überspringen.",
    },
    Meta {
        ids: &["images/alt-missing", "images/alt-suspicious"],
        tier: Tier::Structure,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Informative Bilder brauchen einen beschreibenden Alt-Text.",
    },
    Meta {
        ids: &["forms/label-missing", "forms/placeholder-as-label"],
        tier: Tier::Structure,
        wcag: &["1.3.1", "3.3.2", "4.1.2"],
        severity: Severity::Critical,
        help: "Jedes Eingabefeld braucht ein zugeordnetes Label.",
    },
    Meta {
        ids: &["aria/role-invalid", "aria/role-abstract"],
        tier: Tier::Structure,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Nur Rollen aus der ARIA-Spezifikation verwenden.",
    },
    Meta {
        ids: &["aria/reference-missing"],
        tier: Tier::Structure,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "ARIA-Verweise müssen auf vorhandene IDs zeigen.",
    },
    Meta {
        ids: &["aria/required-attribute-missing"],
        tier: Tier::Structure,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Eine Rolle, die einen Zustand ansagt, braucht das Attribut, das ihn trägt.",
    },
    Meta {
        ids: &["ids/duplicate"],
        tier: Tier::Structure,
        wcag: &["4.1.1"],
        severity: Severity::Medium,
        help: "IDs müssen im Dokument eindeutig sein.",
    },
    Meta {
        ids: &["keyboard/positive-tabindex"],
        tier: Tier::Structure,
        wcag: &["2.4.3"],
        severity: Severity::High,
        help: "Positive tabindex-Werte brechen die Tabreihenfolge.",
    },
    Meta {
        ids: &["keyboard/hidden-focusable"],
        tier: Tier::Structure,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "Fokussierbare Elemente dürfen nicht aria-hidden sein.",
    },
    Meta {
        ids: &[
            "lists/invalid-structure",
            "lists/empty",
            "lists/term-without-definition",
        ],
        tier: Tier::Structure,
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        help: "<ul> und <ol> dürfen als direkte Kinder nur <li> haben und nicht leer sein.",
    },
    Meta {
        ids: &[
            "tables/header-missing",
            "tables/name-missing",
            "tables/presentational-with-headers",
        ],
        tier: Tier::Structure,
        wcag: &["1.3.1"],
        severity: Severity::High,
        help: "Datentabellen brauchen <th>-Kopfzellen.",
    },
    Meta {
        ids: &[
            "landmarks/main-missing",
            "landmarks/main-duplicate",
            "landmarks/navigation-missing",
            "landmarks/banner-missing",
            "landmarks/contentinfo-missing",
        ],
        tier: Tier::Structure,
        wcag: &["1.3.1", "2.4.1"],
        severity: Severity::High,
        help: "Landmarks gliedern die Seite für alle, die sie nicht sehen können.",
    },
    Meta {
        ids: &["keyboard/skip-link-missing"],
        tier: Tier::Structure,
        wcag: &["2.4.1"],
        severity: Severity::Medium,
        help: "Ein Sprunglink überspringt wiederkehrende Bereiche vor dem Inhalt.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
fn funktionen<D: Document>() -> [fn(&D, &mut Vec<Finding>); 16] {
    [
        lang,
        title,
        viewport,
        headings,
        images,
        form_labels,
        aria_roles,
        aria_references,
        aria_required_attributes,
        duplicate_ids,
        tabindex,
        hidden_focusable,
        list_structure,
        table_headers,
        landmarks,
        skip_link,
    ]
}

/// Alle Tier-1-Regeln.
pub fn rules<D: Document>() -> Vec<StructureRule<D>> {
    METAS
        .iter()
        .zip(funktionen::<D>())
        .map(|(meta, run)| StructureRule { meta: *meta, run })
        .collect()
}
