//! Tier-1-Regeln: entscheidbar allein aus Tags, Attributen, Text und Hierarchie.

use std::collections::{HashMap, HashSet};

use a11y_dom::{
    Document, Node, NodeId, ScopedId, Tier, ancestors, closest, elements, has_text, scoped_id,
    self_and_descendants, subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, de_text, pick, tr};
use crate::registry::{Meta, StructureRule};
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Rollen aus den ARIA-Modulen: WAI-ARIA Graphics Module 1.0 und DPUB-ARIA 1.1.
/// Gültige Rollen, aber ohne Einträge in den Attribut-Tabellen von WAI-ARIA
/// 1.2 — `aria/role-invalid` erkennt sie an, die Attributregeln urteilen über
/// sie nicht. Beleg: `<svg role="graphics-document">` im auditmysite-Korpus
/// (`svg_graphics_role_no_name`).
pub(crate) const MODULE_ROLES: &[&str] = &[
    "graphics-document",
    "graphics-object",
    "graphics-symbol",
    "doc-abstract",
    "doc-acknowledgments",
    "doc-afterword",
    "doc-appendix",
    "doc-backlink",
    "doc-biblioentry",
    "doc-bibliography",
    "doc-biblioref",
    "doc-chapter",
    "doc-colophon",
    "doc-conclusion",
    "doc-cover",
    "doc-credit",
    "doc-credits",
    "doc-dedication",
    "doc-endnote",
    "doc-endnotes",
    "doc-epigraph",
    "doc-epilogue",
    "doc-errata",
    "doc-example",
    "doc-footnote",
    "doc-foreword",
    "doc-glossary",
    "doc-glossref",
    "doc-index",
    "doc-introduction",
    "doc-noteref",
    "doc-notice",
    "doc-pagebreak",
    "doc-pagefooter",
    "doc-pageheader",
    "doc-pagelist",
    "doc-part",
    "doc-preface",
    "doc-prologue",
    "doc-pullquote",
    "doc-qna",
    "doc-subtitle",
    "doc-tip",
    "doc-toc",
];

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

fn lang<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let root = doc.root();
    if !root.is_element("html") {
        return;
    }
    // Ein leeres oder nur aus Leerraum bestehendes `lang` nennt keine Sprache:
    // ACT b5c3f8 verlangt einen nicht leeren Wert, ACT bf051a (gültiger Wert)
    // gilt erst für nicht leere. Deshalb `lang-missing`, nicht `lang-invalid`.
    match root.attr("lang").filter(|l| !l.trim().is_empty()) {
        None => out.push(
            Finding::fail(
                "document/lang-missing",
                pick!(
                    locale,
                    "The <html> element has no lang attribute.",
                    "Das <html>-Element hat kein lang-Attribut.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["3.1.1"])
            .at(at(root.id())),
        ),
        Some(l) if !is_plausible_lang(l) => out.push(
            Finding::fail(
                "document/lang-invalid",
                tr!(
                    locale,
                    "The language code \"{l}\" is not a valid BCP-47 tag.",
                    "Der Sprachcode \"{l}\" ist kein gültiges BCP-47-Kürzel."
                ),
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
    // BCP 47 (RFC 5646, 2.1): jedes weitere Untertag hat 1 bis 8 Zeichen.
    parts.all(|p| (1..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn title<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Nur ein HTML-Dokument braucht einen `<title>` (ACT 2779a5: „html
    // element"); ein SVG- oder anderes Fremddokument nicht.
    if !doc.root().is_element("html") {
        return;
    }
    // Nur der HTML-`<title>` zählt (ACT 2779a5), nicht der eines eingebetteten
    // SVG: Der benennt die Grafik, nicht die Seite.
    let html_title = elements(doc)
        .find(|n| n.is_element("title") && !ancestors(*n).any(|a| a.is_element("svg")));
    match html_title {
        None => out.push(
            Finding::fail(
                "document/title-missing",
                pick!(
                    locale,
                    "The document has no <title> element.",
                    "Das Dokument hat kein <title>-Element.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.2"]),
        ),
        Some(t) if subtree_text(t).trim().is_empty() => out.push(
            Finding::fail(
                "document/title-empty",
                pick!(
                    locale,
                    "The <title> element is empty.",
                    "Das <title>-Element ist leer.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.2"])
            .at(at(t.id())),
        ),
        _ => {}
    }
}

/// Der Wert zu `key` im `content` einer Viewport-Angabe.
///
/// Trenner sind nach CSS Viewport („parsing the content attribute") Komma,
/// Semikolon und Leerraum; Leerraum um `=` gehört zum Paar.
/// `width=device-width; maximum-scale=1` ist verbreitet, und Browser werten
/// es aus — wer nur an Kommas trennt, übersieht die Sperre.
fn viewport_value<'c>(content: &'c str, key: &str) -> Option<&'c str> {
    let mut rest = content;
    while !rest.is_empty() {
        rest = rest.trim_start_matches(|c: char| c == ',' || c == ';' || c.is_whitespace());
        let name_end = rest
            .find(|c: char| c == '=' || c == ',' || c == ';' || c.is_whitespace())
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        rest = rest[name_end..].trim_start();
        let Some(after_eq) = rest.strip_prefix('=') else {
            continue;
        };
        rest = after_eq.trim_start();
        let value_end = rest
            .find(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .unwrap_or(rest.len());
        let value = &rest[..value_end];
        rest = &rest[value_end..];
        if name == key {
            return Some(value);
        }
    }
    None
}

fn viewport<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Die Viewport-Angabe ist ein HTML-`<meta>`; ein Fremddokument hat keine.
    if !doc.root().is_element("html") {
        return;
    }
    let mut found = false;
    for n in elements(doc).filter(|n| n.is_element("meta")) {
        if n.attr("name") != Some("viewport") {
            continue;
        }
        found = true;
        let Some(content) = n.attr("content") else {
            continue;
        };
        // Kleinschreiben vor dem Vergleich: HTML-Attributwerte sind nicht
        // normiert, und `user-scalable=NO` sperrt den Zoom genauso.
        let c = content.to_ascii_lowercase();
        let locked = matches!(viewport_value(&c, "user-scalable"), Some("no" | "0"));
        let max = viewport_value(&c, "maximum-scale").and_then(|v| v.parse::<f32>().ok());

        // Unter 200 %: verletzt 1.4.4 unmittelbar.
        if locked || max.is_some_and(|v| v < 2.0) {
            out.push(
                Finding::fail(
                    "zoom/viewport-locked",
                    pick!(
                        locale,
                        "The viewport prevents or limits zooming.",
                        "Der Viewport verhindert oder begrenzt das Zoomen.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.4.4"])
                .at(at(n.id())),
            );
            continue;
        }

        // Zwischen 200 % und 500 %: 1.4.4 ist erfüllt, aber die Begrenzung
        // trifft alle, die stärker vergrößern müssen. Eigene Kennung statt
        // derselben, weil der eine Fall ein Verstoß ist und der andere nicht —
        // und deshalb ohne 1.4.4, als Best Practice. Mit 1.4.4 zählten Hosts
        // ihn als WCAG-Verstoß (auditmysite, inros-lackner.de mit
        // `maximum-scale=2`).
        if max.is_some_and(|v| v < 5.0) {
            out.push(
                Finding::best_practice(
                    "zoom/viewport-scale-limited",
                    pick!(
                        locale,
                        "The viewport limits scaling to less than 500%.",
                        "Der Viewport begrenzt die Vergrößerung auf weniger als 500 %.",
                    ),
                )
                .with_severity(Severity::Low)
                .at(at(n.id())),
            );
        }
    }

    // Ohne Viewport-Angabe legen mobile Browser eine Desktop-Breite zugrunde
    // und verkleinern die Seite. Ein Verstoß ist das nicht: Zoomen bleibt
    // erlaubt (1.4.4), und 1.4.10 wird über die Vergrößerung im Browser
    // geprüft, die die Angabe nicht berührt. Best Practice ohne Kriterium
    // (auditmysite#779).
    if !found {
        out.push(
            Finding::best_practice(
                "zoom/viewport-missing",
                pick!(
                    locale,
                    "The document has no viewport declaration.",
                    "Das Dokument hat keine Viewport-Angabe.",
                ),
            )
            .with_severity(Severity::High)
            .at(at(doc.root().id())),
        );
    }
}

// --- Überschriften --------------------------------------------------------

/// Ob eine Überschrift leer ist — entschieden allein aus der Struktur.
///
/// „Leer" hieß bis 0.10.0: kein Text im Teilbaum und kein `aria-label`. Das
/// übersah zwei Wege, auf denen eine Überschrift sehr wohl einen Namen trägt —
/// `aria-labelledby` und ein Bild mit Alternativtext. Eine Überschrift, die aus
/// einem beschrifteten Logo besteht, ist keine leere Überschrift, und der
/// Befund dagegen war schlicht falsch.
///
/// Der Accessible Name wäre die genauere Auskunft, aber er ist Tier 2. Diese
/// Regel gilt Tier 1 und nähert ihn deshalb strukturell an: Sie fragt nicht,
/// *wie* der Name lautet, sondern ob überhaupt einer gestiftet wird.
fn heading_is_empty<'a, N: Node<'a>>(n: N) -> bool {
    // Ein leeres `aria-label` stiftet keinen Namen (accname 1.2, 2D).
    if has_text(n)
        || n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
        || n.has_attr("aria-labelledby")
    {
        return false;
    }
    !self_and_descendants(n).any(|d| {
        d.attr("alt").is_some_and(|v| !v.trim().is_empty())
            || d.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
            || d.has_attr("aria-labelledby")
    })
}

/// Die Überschriftenebene: `h1`–`h6` nach HTML-AAM, `role="heading"` mit
/// `aria-level` (WAI-ARIA 1.2: ohne Angabe Ebene 2). Eine andere explizite
/// Rolle nimmt einem `hN` die Überschrift (`<h2 role="tab">`).
fn heading_level_of<'a, N: Node<'a>>(n: N) -> Option<u8> {
    match explicit_role(n) {
        Some("heading") => Some(
            n.attr("aria-level")
                .and_then(|v| v.trim().parse::<u8>().ok())
                .filter(|l| *l >= 1)
                .or_else(|| heading_level(n.local_name()))
                .unwrap_or(2),
        ),
        Some(_) => None,
        None => heading_level(n.local_name()),
    }
}

fn headings<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut last = 0u8;
    let mut any = false;
    let mut h1s: Vec<NodeId> = Vec::new();

    for n in elements(doc) {
        let Some(level) = heading_level_of(n) else {
            continue;
        };
        any = true;
        if level == 1 {
            h1s.push(n.id());
        }
        if heading_is_empty(n) {
            out.push(
                Finding::fail(
                    "headings/empty",
                    pick!(
                        locale,
                        "The heading has no text.",
                        "Die Überschrift hat keinen Text.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1", "2.4.6"])
                .at(at(n.id())),
            );
        }
        // Eine übersprungene Ebene ist kein Verstoß: 1.3.1 verlangt, dass die
        // Überschriften als solche bestimmbar sind, nicht, dass ihre Ebenen
        // lückenlos aufeinander folgen (G141 ist Empfehlung, axe
        // `heading-order` Best Practice). Deshalb ohne Kriterium.
        if last > 0 && level > last + 1 {
            out.push(
                Finding::best_practice(
                    "headings/skip-level",
                    tr!(
                        locale,
                        "The outline skips from h{last} to h{level}.",
                        "Die Gliederung springt von h{last} auf h{level}."
                    ),
                )
                .with_severity(Severity::Medium)
                .at(at(n.id())),
            );
        }
        last = level;
    }

    // Gar keine Überschrift ist für sich kein Verstoß gegen 2.4.1: Eine
    // main-Landmark (ARIA11) und ein Sprunglink (G1) überspringen
    // wiederholte Blöcke genauso, jede Technik reicht allein. Gemeldet wird
    // deshalb nur, wenn keiner der drei Mechanismen da ist — als Verstoß,
    // wenn Navigation oder Kopfbereich als Landmark ausgezeichnet sind (die
    // Blöcke, die Understanding 2.4.1 als wiederholt nennt), sonst als
    // Hinweis: Ob sich Blöcke über mehrere Seiten wiederholen, zeigt ein
    // einzelnes Dokument nicht (auditmysite#779). Gezählt wird auch
    // `role="heading"`. Hinter einem offenen Dialog fehlen die Mechanismen
    // für den Moment der Messung, nicht für die Seite (auditmysite#709).
    let heading = any || elements(doc).any(|n| explicit_role(n) == Some("heading"));
    let main = elements(doc).any(|n| is_landmark(n, "main", "main"));
    if !heading && !main && !skip_link_present(doc) {
        let repeated = elements(doc).any(|n| {
            is_landmark(n, "nav", "navigation")
                || (is_landmark(n, "header", "banner")
                    && (n.attr("role").is_some() || is_document_wide(n)))
        });
        let f = if modal_open(doc) {
            Finding::review(
                "headings/none",
                pick!(
                    locale,
                    "No headings reachable while a dialog is open; check the page with the dialog closed.",
                    "Keine Überschriften erreichbar, solange ein Dialog offen ist; die Seite mit geschlossenem Dialog prüfen.",
                ),
            )
            .with_severity(Severity::Low)
        } else if repeated {
            Finding::fail(
                "headings/none",
                pick!(
                    locale,
                    "Navigation or header blocks have no way to be bypassed: no headings, no main landmark, no skip link.",
                    "Navigation oder Kopfbereich lassen sich nicht überspringen: keine Überschriften, keine main-Landmark, kein Sprunglink.",
                ),
            )
            .with_severity(Severity::Medium)
        } else {
            Finding::review(
                "headings/none",
                pick!(
                    locale,
                    "No headings, no main landmark and no skip link; if blocks repeat across pages, nothing bypasses them.",
                    "Keine Überschriften, keine main-Landmark und kein Sprunglink; wiederholen sich Blöcke über mehrere Seiten, überspringt sie nichts.",
                ),
            )
            .with_severity(Severity::Low)
        };
        out.push(f.with_wcag(["2.4.1"]).at(at(doc.root().id())));
    }

    // Eine fehlende h1 belegt keinen Verstoß: Eine korrekt ausgezeichnete h2
    // ist als Überschrift bestimmbar (1.3.1), und WCAG verlangt keine
    // bestimmte Ebene. Best Practice ohne Kriterium, wie axe
    // `page-has-heading-one` (auditmysite#779).
    if any && h1s.is_empty() {
        out.push(
            Finding::best_practice(
                "headings/h1-missing",
                pick!(
                    locale,
                    "The document has no h1 heading.",
                    "Das Dokument hat keine h1-Überschrift.",
                ),
            )
            .with_severity(Severity::Medium),
        );
    }

    // Mehrere h1 sind in HTML zulässig und je nach Gliederung sogar richtig.
    // Meist sind sie es nicht — das ist eine Erwartung, kein Verstoß, also
    // REVIEW. Der Befund zeigt auf die zweite, die überzählige.
    if h1s.len() > 1 {
        out.push(
            Finding::review(
                "headings/h1-multiple",
                tr!(
                    locale,
                    "The document has {} h1 headings.",
                    "Das Dokument hat {} h1-Überschriften.",
                    h1s.len()
                ),
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
const REQUIRED: &[(&str, &[&str])] = &[
    ("checkbox", &["aria-checked"]),
    ("switch", &["aria-checked"]),
    ("radio", &["aria-checked"]),
    ("combobox", &["aria-expanded"]),
    // ARIA 1.2: `aria-valuemin`/`aria-valuemax` haben Vorgaben (0 und 100)
    // und sind nicht mehr erforderlich, `aria-selected` an `option` ebenso.
    ("slider", &["aria-valuenow"]),
    ("spinbutton", &["aria-valuenow"]),
    ("scrollbar", &["aria-controls", "aria-valuenow"]),
    ("meter", &["aria-valuenow"]),
    // Nur der fokussierbare Trenner ist ein Widget mit Wert; der statische
    // braucht keinen.
    ("separator", &["aria-valuenow"]),
];

/// Ob das Element seinen Zustand nativ übermittelt, sodass die Rolle ihn
/// nicht per ARIA braucht: `<input type="range" role="slider">` hat seinen
/// Wert, `<input type="checkbox" role="switch">` seinen Haken (ARIA in HTML
/// verbietet dort sogar `aria-checked`). Beleg: auditmysite#656.
fn state_native<'a, N: Node<'a>>(n: N) -> bool {
    if n.is_element("meter") || n.is_element("progress") {
        return true;
    }
    n.is_element("input")
        && n.attr("type").is_some_and(|t| {
            ["checkbox", "radio", "range", "number"]
                .iter()
                .any(|x| t.trim().eq_ignore_ascii_case(x))
        })
}

fn aria_required_attributes<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(role) = n.attr("role").map(str::trim) else {
            continue;
        };
        // Bei mehreren Rollen gilt die erste, die der Browser kennt.
        let Some(first) = role.split_whitespace().next() else {
            continue;
        };
        let Some((_, needed)) = REQUIRED.iter().find(|(r, _)| r.eq_ignore_ascii_case(first)) else {
            continue;
        };
        if state_native(n) {
            continue;
        }
        if first.eq_ignore_ascii_case("separator") && !n.has_attr("tabindex") {
            continue;
        }

        let missing: Vec<&str> = needed.iter().copied().filter(|a| !n.has_attr(a)).collect();
        if missing.is_empty() {
            continue;
        }
        out.push(
            Finding::fail(
                "aria/required-attribute-missing",
                tr!(
                    locale,
                    "role=\"{first}\" requires {}, but {} is missing.",
                    "role=\"{first}\" braucht {}, aber {} fehlt.",
                    needed.join(", "),
                    missing.join(", ")
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
fn is_landmark<'a, N: Node<'a>>(n: N, tag: &str, role: &str) -> bool {
    match n.attr("role").map(str::trim) {
        Some(r) => r.split_whitespace().any(|x| x.eq_ignore_ascii_case(role)),
        None => n.is_element(tag),
    }
}

/// `<header>` und `<footer>` sind nur dann banner bzw. contentinfo, wenn sie
/// nicht in einem Sectioning-Element stecken — ein `<footer>` in einem
/// `<article>` gehört zu diesem Artikel, nicht zum Dokument. Ein `<div>`
/// dazwischen disqualifiziert dagegen nicht.
const SECTIONING: &[&str] = &["article", "aside", "main", "nav", "section"];

/// Dieselbe Grenze über die Rolle (HTML-AAM, `header`/`footer`): Ein
/// `<div role="main">` schließt ein `<header>` genauso ein wie ein `<main>`.
/// Beleg: auditmysite#639.
const SECTIONING_ROLES: &[&str] = &["article", "complementary", "main", "navigation", "region"];

pub(crate) fn is_document_wide<'a, N: Node<'a>>(n: N) -> bool {
    a11y_dom::ancestors(n).all(|a| {
        !SECTIONING.contains(&a.local_name())
            && !a.attr("role").is_some_and(|r| {
                r.split_whitespace()
                    .next()
                    .is_some_and(|x| SECTIONING_ROLES.iter().any(|s| x.eq_ignore_ascii_case(s)))
            })
    })
}

/// Ob ein Dialog offen ist: ein dargestelltes `role="dialog"`/`"alertdialog"`
/// oder ein `<dialog open>`. Solange er offen ist, blenden Seiten den Rest oft
/// aus (`aria-hidden` oder `inert`) — die Landmarks fehlen dann für den
/// Moment, nicht für die Seite. `aria-modal` ist keine Voraussetzung: Der
/// Consent-Dialog von administracion.gob.es trägt es nicht und blendet
/// `<main>` trotzdem aus (auditmysite#709). Die Sicht der Regel enthält nur
/// Dargestelltes; ein per Stil geschlossener Dialog zählt bei Hosts mit
/// Stilen also nicht.
fn modal_open<D: Document>(doc: &D) -> bool {
    elements(doc).any(|n| {
        let dialog_role = n.attr("role").is_some_and(|r| {
            r.split_whitespace().next().is_some_and(|x| {
                x.eq_ignore_ascii_case("dialog") || x.eq_ignore_ascii_case("alertdialog")
            })
        });
        dialog_role || (n.is_element("dialog") && n.has_attr("open"))
    })
}

/// Ob im `<body>` etwas steht, das in eine Landmark gehören könnte: Text
/// oder ein ersetztes bzw. Bedienelement. Die Sicht der Regel enthält nur,
/// was im Accessibility-Tree steht; Verstecktes zählt also nicht.
fn page_has_content<D: Document>(doc: &D) -> bool {
    elements(doc)
        .filter(|n| n.is_element("body") || a11y_dom::ancestors(*n).any(|a| a.is_element("body")))
        .filter(|n| !matches!(n.local_name(), "script" | "style" | "template" | "noscript"))
        .any(|n| {
            n.children()
                .any(|k| k.kind() == a11y_dom::NodeKind::Text && !k.text().trim().is_empty())
                || matches!(
                    n.local_name(),
                    "img"
                        | "svg"
                        | "canvas"
                        | "video"
                        | "audio"
                        | "iframe"
                        | "object"
                        | "embed"
                        | "input"
                        | "button"
                        | "select"
                        | "textarea"
                )
        })
}

fn landmarks<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Landmarks fassen Inhalt (WAI-ARIA 1.2, „landmark“). Eine Seite ohne
    // jeden Inhalt — leer oder alles versteckt — erwartet keine; die fünf
    // Befunde hier wären über nichts.
    if !page_has_content(doc) {
        return;
    }
    let root = doc.root().id();
    let mains: Vec<_> = elements(doc)
        .filter(|n| is_landmark(*n, "main", "main"))
        .collect();

    // Fehlende und doppelte main-Landmark sind Best Practice ohne Kriterium
    // (axe `landmark-one-main`, `landmark-no-duplicate-main`): 1.3.1
    // verlangt keine Landmarks, und für 2.4.1 genügen Überschriften oder ein
    // Sprunglink genauso — fehlt alles zugleich, meldet das
    // `headings/none` (auditmysite#779). Mehrere `<main>` verbietet HTML,
    // nicht WCAG, seit 2.2 4.1.1 entfallen ist.
    match mains.len() {
        // Hinter einem offenen Modal ist die fehlende main-Landmark kein
        // Befund über die Seite, sondern über den Moment der Messung — ein
        // Hinweis, kein Verstoß. Beleg: fünf von 21 EU-Portalen mit offenem
        // Consent-Dialog (auditmysite#709).
        0 if modal_open(doc) => out.push(
            Finding::best_practice(
                "landmarks/main-missing",
                pick!(
                    locale,
                    "No main landmark reachable while a modal dialog is open; check the page with the dialog closed.",
                    "Keine main-Landmark erreichbar, solange ein modaler Dialog offen ist; die Seite mit geschlossenem Dialog prüfen.",
                ),
            )
            .with_severity(Severity::Low)
            .at(at(root)),
        ),
        0 => out.push(
            Finding::best_practice(
                "landmarks/main-missing",
                pick!(
                    locale,
                    "The document has no main landmark.",
                    "Das Dokument hat keine main-Landmark.",
                ),
            )
            .with_severity(Severity::High)
            .at(at(root)),
        ),
        1 => {}
        n => out.push(
            Finding::best_practice(
                "landmarks/main-duplicate",
                tr!(
                    locale,
                    "The document has {n} main landmarks; exactly one is allowed.",
                    "Das Dokument hat {n} main-Landmarks; genau eine ist zulässig."
                ),
            )
            .with_severity(Severity::High)
            // Die zweite ist die überzählige — dorthin zeigt der Befund.
            .at(at(mains[1].id())),
        ),
    }

    // Navigation, banner und contentinfo sind Erwartungen, keine beweisbaren
    // Verstöße: Eine Seite darf ohne Navigation auskommen. Deshalb REVIEW und
    // nicht FAIL -- eine dritte Achse „Gewissheit" gibt es bewusst nicht.
    for (tag, role, id, text) in [
        (
            "nav",
            "navigation",
            "landmarks/navigation-missing",
            pick!(
                locale,
                "The document has no navigation landmark.",
                "Das Dokument hat keine navigation-Landmark.",
            ),
        ),
        (
            "header",
            "banner",
            "landmarks/banner-missing",
            pick!(
                locale,
                "The document has no banner landmark.",
                "Das Dokument hat keine banner-Landmark.",
            ),
        ),
        (
            "footer",
            "contentinfo",
            "landmarks/contentinfo-missing",
            pick!(
                locale,
                "The document has no contentinfo landmark.",
                "Das Dokument hat keine contentinfo-Landmark.",
            ),
        ),
    ] {
        let present = elements(doc).any(|n| {
            is_landmark(n, tag, role) && (n.attr("role").is_some() || is_document_wide(n))
        });
        if !present {
            out.push(
                Finding::review(id, text)
                    .with_severity(Severity::Low)
                    .with_wcag(["1.3.1", "2.4.1"])
                    .at(at(root)),
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
fn looks_like_skip_link<'a, N: Node<'a>>(n: N) -> bool {
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

fn is_fragment_link(href: &str) -> bool {
    href.starts_with('#') && href.len() > 1
}

/// Ob `target` der Anfang des Hauptinhalts ist: die main-Landmark selbst oder
/// ein Element, das an ihrem Anfang steht (jeder Vorfahre bis zur Landmark
/// ist das erste Element-Kind). `<main><h1 id="inhalt">` gehört dazu, ein
/// Abschnitt mitten im Inhalt nicht — ein Inhaltsverzeichnis ist kein
/// Sprunglink.
fn starts_main_content<'a, N: Node<'a>>(target: N) -> bool {
    let mut n = target;
    loop {
        if is_landmark(n, "main", "main") {
            return true;
        }
        let Some(parent) = n.parent() else {
            return false;
        };
        let first = parent
            .children()
            .find(|k| k.kind() == a11y_dom::NodeKind::Element);
        if first != Some(n) {
            return false;
        }
        n = parent;
    }
}

/// Ob die Seite einen Sprunglink hat — am Ziel, an der Lage vor dem ersten
/// Link, der die Seite verlässt, oder an Text und Klasse erkannt.
fn skip_link_present<D: Document>(doc: &D) -> bool {
    let links: Vec<_> = elements(doc)
        .filter(|n| n.is_element("a") && n.has_attr("href"))
        .collect();
    // Ein Fragmentlink auf den Anfang des Hauptinhalts ist ein Sprunglink,
    // wo immer er steht (G1, ARIA11) — auch hinter den Links eines
    // Cookie-Banners. Beleg: bund.de, „Inhalt" → `#main` (barrierlab#26).
    let ids: HashMap<ScopedId, _> = elements(doc)
        .filter_map(|n| n.attr("id").map(|id| (scoped_id(n, id), n)))
        .collect();
    let to_the_main_content = links.iter().any(|n| {
        n.attr("href")
            .filter(|h| is_fragment_link(h))
            .and_then(|h| ids.get(&scoped_id(*n, &h[1..])))
            .is_some_and(|target| starts_main_content(*target))
    });
    // Am Ziel erkannt, wie axe `isSkipLink`: Links vor dem ersten, der die
    // Seite verlässt, springen innerhalb der Seite — in jeder Sprache. Nur
    // wenn danach überhaupt etwas zu überspringen kommt; eine Seite aus
    // lauter Ankern hat keinen Block, den ein Sprunglink umgeht.
    // Beleg: auditmysite#642, „Aller au contenu" auf jeder /fr/-Seite.
    // `href=""` verlässt die Seite nicht; es zählt weder als Sprunglink noch
    // beendet es die Folge davor.
    let ahead = links
        .iter()
        .filter(|n| n.attr("href").is_some_and(|h| !h.trim().is_empty()))
        .take_while(|n| n.attr("href").is_some_and(is_fragment_link))
        .count();
    let real = links
        .iter()
        .filter(|n| n.attr("href").is_some_and(|h| !h.trim().is_empty()))
        .count();
    to_the_main_content
        || (ahead > 0 && ahead < real)
        || links
            .iter()
            .any(|n| n.attr("href").is_some_and(is_fragment_link) && looks_like_skip_link(*n))
}

fn skip_link<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // 2.4.1 betrifft Blöcke, die sich über Seiten wiederholen — Understanding
    // nennt Navigationslinks und Kopfbereiche. Eine Seite ohne Link und ohne
    // Navigations- oder Kopf-Landmark hat nichts, das ein Sprunglink umgehen
    // könnte; dort ist die Regel nicht anwendbar.
    let to_skip = elements(doc).any(|n| {
        (n.is_element("a") && n.attr("href").is_some_and(|h| !h.trim().is_empty()))
            || is_landmark(n, "nav", "navigation")
            || (is_landmark(n, "header", "banner")
                && (n.attr("role").is_some() || is_document_wide(n)))
    });
    if to_skip && !skip_link_present(doc) {
        out.push(
            Finding::review(
                "keyboard/skip-link-missing",
                pick!(
                    locale,
                    "No skip link found that bypasses repeated blocks.",
                    "Kein Sprunglink gefunden, der wiederkehrende Bereiche überspringt.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.4.1"])
            .at(at(doc.root().id())),
        );
    }
}

// --- Bilder ---------------------------------------------------------------

/// Ob dieses Element ausdrücklich aus dem Accessibility-Tree genommen wurde.
///
/// Ein Bild mit `role="presentation"` oder `aria-hidden="true"` ist erklärt
/// dekorativ. Ihm ein `alt` abzuverlangen hieße, eine bewusste Angabe des
/// Autors zu ignorieren und einen Befund zu melden, den niemand beheben kann,
/// ohne die Angabe zurückzunehmen.
fn explicitly_decorative<'a, N: Node<'a>>(n: N) -> bool {
    if n.attr("aria-hidden") == Some("true") {
        return true;
    }
    n.attr("role").is_some_and(|r| {
        r.split_whitespace()
            .any(|x| x.eq_ignore_ascii_case("presentation") || x.eq_ignore_ascii_case("none"))
    })
}

/// Ob ein `<img>` ohne `alt` seine Textalternative anders bekommt:
/// `aria-label`, `aria-labelledby` oder `title` sind nach ARIA6, ARIA10 und
/// H67 hinreichende Techniken, und die Namensberechnung zieht sie heran.
fn alternative_without_alt<'a, N: Node<'a>>(n: N) -> bool {
    ["aria-label", "aria-labelledby", "title"]
        .iter()
        .any(|a| n.attr(a).is_some_and(|v| !v.trim().is_empty()))
}

fn images<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| n.is_element("img")) {
        if explicitly_decorative(n) {
            continue;
        }
        if n.attr("alt").is_none() && alternative_without_alt(n) {
            continue;
        }
        match n.attr("alt") {
            None => out.push(
                Finding::fail(
                    "images/alt-missing",
                    pick!(
                        locale,
                        "The image has no alt attribute.",
                        "Das Bild hat kein alt-Attribut.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            ),
            Some(alt) if suspicious_alt(alt) => out.push(
                // Heuristisch: der Text ist da, aber vermutlich nichtssagend.
                // Deshalb Review, nicht Fail.
                Finding::review(
                    "images/alt-suspicious",
                    tr!(
                        locale,
                        "The alt text \"{alt}\" probably does not describe the image.",
                        "Der Alt-Text \"{alt}\" beschreibt das Bild vermutlich nicht."
                    ),
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
    // Ein oder zwei Zeichen beschreiben kein Bild. Der Befund ist ohnehin
    // REVIEW — wenn `alt="5"` am Bild einer Fünf steht, bestätigt das der
    // Mensch in einem Schritt.
    if a.chars().count() < 3 {
        return true;
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
fn form_labels<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let label_targets: HashSet<ScopedId> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for").map(|f| scoped_id(n, f)))
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
        let for_labelled = n
            .attr("id")
            .is_some_and(|id| label_targets.contains(&scoped_id(n, id)));
        let wrapped = closest(n, "label").is_some();
        let titled = n.attr("title").is_some_and(|v| !v.trim().is_empty());
        // Ein Platzhalter stiftet nach HTML-AAM den Namen (ACT e086e5 besteht
        // damit); ein Feld mit ihm ist nicht unbeschriftet, sondern ein Fall
        // für `forms/placeholder-as-label` (auditmysite#779).
        let placeholder = n.attr("placeholder").is_some_and(|v| !v.trim().is_empty());

        if !(aria_named || for_labelled || wrapped || titled || placeholder) {
            out.push(
                Finding::fail(
                    "forms/label-missing",
                    pick!(
                        locale,
                        "The input has no label.",
                        "Das Eingabefeld hat kein Label."
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["1.3.1", "3.3.2", "4.1.2"])
                .at(at(n.id())),
            );
        } else if placeholder && !n.has_attr("aria-labelledby") && !for_labelled && !wrapped {
            // `aria-label` ist nicht sichtbar: Neben ihm bleibt der Platzhalter
            // die einzige sichtbare Beschriftung (3.3.2).
            // Der Platzhalter ist sichtbar, solange das Feld leer ist, und
            // stiftet den Namen; 3.3.2 ist damit nicht sicher verletzt. Ob er
            // als Beschriftung genügt, ist eine Frage an den Prüfer
            // (auditmysite#779).
            out.push(
                Finding::review(
                    "forms/placeholder-as-label",
                    pick!(
                        locale,
                        "The field uses the placeholder instead of a label.",
                        "Das Feld nutzt den Platzhalter anstelle eines Labels.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.3.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- ARIA -----------------------------------------------------------------

fn aria_roles<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(role) = n.attr("role") else { continue };
        // Weitere Token sind Rückfallrollen (WAI-ARIA 1.2, „Handling Author
        // Errors"): Der Browser nimmt das erste, das er kennt. Ungültig ist
        // die Angabe erst, wenn kein Token eine gültige Rolle ist (ACT 674b10).
        let has_valid = role
            .split_whitespace()
            .any(|r| VALID_ROLES.contains(&r) || MODULE_ROLES.contains(&r));
        for r in role.split_whitespace() {
            if ABSTRACT_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-abstract",
                        tr!(
                            locale,
                            "\"{r}\" is an abstract role and must not be used as a value.",
                            "\"{r}\" ist eine abstrakte Rolle und darf nicht ausgezeichnet werden."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            } else if !has_valid && !VALID_ROLES.contains(&r) && !MODULE_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-invalid",
                        tr!(
                            locale,
                            "\"{r}\" is not a valid ARIA role.",
                            "\"{r}\" ist keine gültige ARIA-Rolle."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

fn aria_references<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids: HashSet<ScopedId> = elements(doc)
        .filter_map(|n| n.attr("id").map(|id| scoped_id(n, id)))
        .collect();

    for n in elements(doc) {
        for rel in [
            "aria-labelledby",
            "aria-describedby",
            "aria-controls",
            "aria-owns",
            "aria-activedescendant",
            "aria-details",
            "aria-flowto",
            "aria-errormessage",
        ] {
            let Some(v) = n.attr(rel) else { continue };
            // Die Fehlermeldung darf fehlen, solange nichts ungültig ist —
            // sie entsteht oft erst mit dem Fehler (ARIA 1.2,
            // `aria-errormessage`: nur bei `aria-invalid` maßgeblich).
            if rel == "aria-errormessage"
                && n.attr("aria-invalid")
                    .is_none_or(|i| i.trim().is_empty() || i.trim().eq_ignore_ascii_case("false"))
            {
                continue;
            }
            // Ein leerer Verweis auf Steuerung, Besitz oder aktiven Nachfahren
            // behauptet eine Beziehung, die es nicht gibt. Leeres
            // `aria-labelledby` dagegen fällt auf die übrigen Namensquellen
            // zurück und schadet nicht.
            if v.trim().is_empty() {
                if matches!(rel, "aria-controls" | "aria-owns" | "aria-activedescendant") {
                    out.push(
                        Finding::fail(
                            "aria/reference-missing",
                            tr!(
                                locale,
                                "{rel} is present but empty.",
                                "{rel} ist vorhanden, aber leer.",
                            ),
                        )
                        .with_severity(Severity::High)
                        .with_wcag(["1.3.1", "4.1.2"])
                        .at(at(n.id())),
                    );
                }
                continue;
            }
            // Ein eingeklapptes Bedienelement darf auf ein Ziel zeigen, das
            // erst beim Öffnen entsteht — Menüs aus Web-Komponenten tun das
            // regelmäßig (bund.de, sachsen-anhalt.de). axe-core nimmt diesen
            // Fall bei `aria-controls` genauso aus.
            if rel == "aria-controls" && n.attr("aria-expanded") == Some("false") {
                continue;
            }
            let missing: Vec<&str> = v
                .split_whitespace()
                .filter(|id| !ids.contains(&scoped_id(n, id)))
                .collect();
            if !missing.is_empty() {
                out.push(
                    Finding::fail(
                        "aria/reference-missing",
                        tr!(
                            locale,
                            "{rel} references IDs that do not exist: {}",
                            "{rel} verweist auf nicht vorhandene IDs: {}",
                            missing.join(", ")
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

/// Attribute, die per IDREF auf ein anderes Element zeigen.
const IDREF_ATTRS: &[&str] = &[
    "for",
    "form",
    "list",
    "headers",
    "aria-labelledby",
    "aria-describedby",
    "aria-controls",
    "aria-owns",
    "aria-activedescendant",
    "aria-details",
    "aria-errormessage",
    "aria-flowto",
];

/// Doppelte IDs, auf die ein IDREF zeigt.
///
/// WCAG 2.2 hat 4.1.1 (Parsing) gestrichen: Eine doppelte ID ist für sich
/// genommen kein Verstoß mehr. Einer bleibt sie dort, wo ein IDREF auf sie
/// zeigt — dann ist nicht bestimmbar, welches Element gemeint ist, und Name,
/// Rolle oder Wert der Beziehung bricht. Gemeldet als 4.1.2, dieselbe Zuordnung,
/// die axe-core für `duplicate-id-aria` führt.
fn duplicate_ids<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut referenced: HashSet<ScopedId> = HashSet::new();
    for n in elements(doc) {
        for attr in IDREF_ATTRS {
            if let Some(value) = n.attr(attr) {
                referenced.extend(value.split_whitespace().map(|id| scoped_id(n, id)));
            }
        }
    }
    if referenced.is_empty() {
        return;
    }

    let mut seen: HashMap<ScopedId, usize> = HashMap::new();
    for n in elements(doc) {
        let Some(id) = n.attr("id") else { continue };
        if id.is_empty() || !referenced.contains(&scoped_id(n, id)) {
            continue;
        }
        let count = seen.entry(scoped_id(n, id)).or_insert(0);
        *count += 1;
        // `REVIEW`: Der Browser löst den Verweis auf das erste Element auf.
        // Ob das das gemeinte ist, zeigt das Markup nicht (auditmysite#779).
        if *count == 2 {
            out.push(
                Finding::review(
                    "ids/duplicate",
                    tr!(
                        locale,
                        "The ID \"{id}\" occurs more than once and is referenced, so the reference is ambiguous.",
                        "Die ID \"{id}\" kommt mehrfach vor und wird referenziert — der Verweis ist nicht eindeutig.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Tastatur -------------------------------------------------------------

fn tabindex<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(raw) = n.attr("tabindex") else {
            continue;
        };
        let Ok(value) = raw.trim().parse::<i32>() else {
            continue;
        };
        // Ein positiver tabindex verletzt 2.4.3 erst, wenn die entstehende
        // Reihenfolge Sinn oder Bedienung bricht (F44) — das zeigt das Markup
        // nicht. Deshalb `REVIEW` (auditmysite#779).
        if value > 0 {
            out.push(
                Finding::review(
                    "keyboard/positive-tabindex",
                    tr!(
                        locale,
                        "tabindex=\"{value}\" breaks the natural tab order.",
                        "tabindex=\"{value}\" bricht die natürliche Tabreihenfolge."
                    ),
                )
                // Hoch, nicht mittel: Ein positiver tabindex verschiebt die
                // Tabreihenfolge reproduzierbar und für jeden, der mit der
                // Tastatur navigiert — das ist kein Schönheitsfehler.
                .with_severity(Severity::High)
                .with_wcag(["2.4.3"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ob die Tabtaste dieses Element erreicht: nativ fokussierbar oder mit
/// `tabindex` ≥ 0, und nicht per `tabindex="-1"` oder `disabled` ausgenommen.
/// `inert` und Nicht-Dargestelltes prüft der Aufrufer.
pub(crate) fn per_tab_reachable<'a, N: Node<'a>>(n: N) -> bool {
    let tabindex = n
        .attr("tabindex")
        .and_then(|t| t.trim().parse::<i32>().ok());
    if let Some(t) = tabindex {
        return t >= 0;
    }
    (match n.local_name() {
        "a" | "area" => n.has_attr("href"),
        "input" => !n
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        "button" | "select" | "textarea" | "summary" | "iframe" => true,
        _ => false,
    }) && !(n.has_attr("disabled")
        // `disabled` gibt es nur an Formularfeldern; an einem Link ist es
        // wirkungslos, der Link bleibt per Tab erreichbar.
        && matches!(n.local_name(), "button" | "input" | "select" | "textarea"))
}

/// Fokussierbar und zugleich vor dem Accessibility-Tree versteckt — Nutzer
/// landen mit der Tabtaste auf etwas, das ihnen nicht angesagt wird.
///
/// `aria-hidden` wirkt auf den ganzen Teilbaum. Geprüft wird deshalb jedes
/// fokussierbare Element darunter, nicht nur das, das das Attribut trägt —
/// der übliche Fall ist ein versteckter Container voller Links. Ausgenommen
/// ist, was `inert` ist oder per `tabindex="-1"` aus der Tabfolge genommen
/// wurde: So wird ein versteckter Bereich richtig gebaut. Nicht Dargestelltes
/// fehlt in der Sicht dieser Regel ohnehin (`Scope::Rendered`).
fn hidden_focusable<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let with_ancestors = || std::iter::once(n).chain(ancestors(n));
        if !with_ancestors().any(|a| a.attr("aria-hidden") == Some("true"))
            || with_ancestors().any(|a| a.has_attr("inert"))
        {
            continue;
        }
        if per_tab_reachable(n) {
            out.push(
                Finding::fail(
                    "keyboard/hidden-focusable",
                    pick!(
                        locale,
                        "The element is focusable but hidden with aria-hidden, on itself or an ancestor.",
                        "Das Element ist fokussierbar, aber per aria-hidden versteckt, selbst oder über einen Vorfahren.",
                    ),
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
fn is_list<'a, N: Node<'a>>(n: N) -> bool {
    // `<menu>` ist nach HTML eine Liste aus `<li>` und nach HTML-AAM `list`.
    matches!(n.local_name(), "ul" | "ol" | "menu") || n.attr("role") == Some("list")
}

/// Ob dieser Knoten als Listeneintrag zaehlt.
fn is_list_item<'a, N: Node<'a>>(n: N) -> bool {
    n.local_name() == "li" || n.attr("role") == Some("listitem")
}

/// Die wirksame explizite Rolle: das erste gültige Token aus `role`.
pub(crate) fn explicit_role<'a, N: Node<'a>>(n: N) -> Option<&'a str> {
    n.attr("role")?
        .split_whitespace()
        .find(|r| VALID_ROLES.contains(r))
}

/// Die Element-Kinder einer Liste im flachen Baum.
///
/// Ein `<slot>` ist `display: contents` und für die Assistenztechnik
/// durchlässig: Was ihm zugewiesen ist (oder sein Ersatzinhalt), steht unter
/// ihm und gilt als Kind der Liste. So baut eine Web-Komponente ihre Liste im
/// Shadow Root (`<ul><slot></slot></ul>`) und bekommt die Einträge aus dem
/// Light DOM.
fn list_children<'a, N: Node<'a>>(n: N, out: &mut Vec<N>) {
    for c in n.children() {
        if c.kind() != a11y_dom::NodeKind::Element {
            continue;
        }
        if c.local_name() == "slot" {
            list_children(c, out);
        } else {
            out.push(c);
        }
    }
}

fn list_structure<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !is_list(n) {
            continue;
        }
        // Eine andere Rolle macht aus `<ul>`/`<ol>` etwas anderes als eine
        // Liste -- `presentation`/`none` ebenso wie `listbox` oder `menu`.
        if explicit_role(n).is_some_and(|r| r != "list") {
            continue;
        }
        let tag = n.local_name();
        let mut children = Vec::new();
        list_children(n, &mut children);

        // Ein `<li>` mit einer anderen expliziten Rolle (`role="group"` bei
        // Slidern, `tab`, `menuitem`) ist kein Listeneintrag mehr — die Liste
        // hat dann Kinder, die die Assistenztechnik nicht als Einträge sieht
        // (axe `list`). Beleg: berlin.de, `ul.swiper-wrapper > li[role=group]`.
        let foreign = children.iter().any(|c| {
            let entry = is_list_item(*c) && explicit_role(*c).is_none_or(|r| r == "listitem");
            !entry && !matches!(c.local_name(), "script" | "template")
        });
        if foreign {
            out.push(
                Finding::fail(
                    "lists/invalid-structure",
                    tr!(
                        locale,
                        "<{tag}> has direct children that are not list items.",
                        "<{tag}> enthält direkte Kinder, die keine Listeneinträge sind."
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }

        // Eine Liste ohne Eintraege kuendigt der Assistenztechnik eine
        // Struktur an, die es nicht gibt. Verloren geht dabei keine
        // Information, also kein Verstoss gegen 1.3.1: Best Practice
        // (auditmysite#779).
        // Kinder, die keine Einträge sind, meldet schon `lists/invalid-structure`;
        // „leer" wäre eine zweite, falsche Aussage über denselben Mangel.
        if !foreign && !children.iter().any(|c| is_list_item(*c)) {
            out.push(
                Finding::best_practice(
                    "lists/empty",
                    tr!(
                        locale,
                        "<{tag}> has no list items.",
                        "<{tag}> enthält keine Listeneinträge."
                    ),
                )
                .with_severity(Severity::Low)
                .at(at(n.id())),
            );
        }
    }

    description_lists(doc, locale, out);
    orphaned_entries(doc, locale, out);
}

/// Ein Listeneintrag ohne Liste.
///
/// Die Prüfung oben läuft über Listen und sieht deshalb nur, was *in* einer
/// steht. Ein `<li>`, das gar keine Liste über sich hat, wird dabei nie
/// besucht — für die Assistenztechnik kündigt es aber eine Aufzählung an, die
/// es nicht gibt.
fn orphaned_entries<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        // Ein `<li>` mit eigener Rolle (`none` im Menü-Muster, `tab`) ist kein
        // Listeneintrag; über seinen Kontext urteilen die ARIA-Regeln.
        if !is_list_item(n) || explicit_role(n).is_some_and(|r| r != "listitem") {
            continue;
        }
        // Die nächste Liste darüber. Ist sie per `role` etwas anderes
        // geworden (`ul[role=tablist]`), steht der Eintrag trotzdem außerhalb
        // einer Liste; ist sie `none`/`presentation`, erbt er das und ist
        // keiner (auditmysite#715, magyarorszag.hu).
        let in_list = match a11y_dom::ancestors(n).find(|a| is_list(*a)) {
            None => false,
            Some(l) => match explicit_role(l) {
                None | Some("list" | "directory") => true,
                Some("none" | "presentation") => continue,
                Some(_) => false,
            },
        };
        if !in_list {
            out.push(
                Finding::fail(
                    "lists/item-outside-list",
                    pick!(
                        locale,
                        "The list item is outside a list.",
                        "Der Listeneintrag steht außerhalb einer Liste.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ob dieser Knoten ein Definitionsbegriff ist — als `<dt>` oder per `role`.
fn is_term<'a, N: Node<'a>>(n: N) -> bool {
    n.local_name() == "dt" || n.attr("role") == Some("term")
}

/// Ob dieser Knoten eine Definition ist.
fn is_definition<'a, N: Node<'a>>(n: N) -> bool {
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
/// Gruppe braucht sein `<dd>` dort, nicht irgendwo in der Liste — und nach dem
/// Inhaltsmodell von `<dl>` *danach*: Auf eine Folge von `<dt>` folgt
/// mindestens ein `<dd>`. Ein `<dd>` vor dem Begriff gehört zu einer anderen
/// Gruppe.
fn description_lists<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !is_term(n) {
            continue;
        }
        let Some(parent) = n.parent() else { continue };
        let has_definition = parent
            .children()
            .filter(|c| c.kind() == a11y_dom::NodeKind::Element)
            .skip_while(|c| *c != n)
            .filter(|c| !matches!(c.local_name(), "script" | "template"))
            .find(|c| !is_term(*c))
            .is_some_and(is_definition);

        if !has_definition {
            out.push(
                Finding::fail(
                    "lists/term-without-definition",
                    pick!(
                        locale,
                        "This term has no definition.",
                        "Zu diesem Begriff gibt es keine Definition.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ob im Teilbaum eine Kopfzelle steckt — als `<th>` oder per Rolle.
fn has_header_cell<'a, N: Node<'a>>(n: N) -> bool {
    a11y_dom::descendants(n).any(|d| {
        d.is_element("th") || matches!(d.attr("role"), Some("columnheader") | Some("rowheader"))
    })
}

fn table_headers<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc)
        .filter(|n| n.is_element("table") || matches!(n.attr("role"), Some("table") | Some("grid")))
    {
        // Als präsentational ausgezeichnet: keine Datentabelle. Dann dürfen
        // dort aber auch keine Kopfzellen stehen — die Auszeichnung sagt „das
        // ist keine Tabelle", die Kopfzellen sagen das Gegenteil, und die
        // Assistenztechnik bekommt widersprüchliche Angaben.
        if matches!(n.attr("role"), Some("presentation") | Some("none")) {
            if has_header_cell(n) {
                out.push(
                    Finding::fail(
                        "tables/presentational-with-headers",
                        pick!(locale, "The table is marked presentational but contains header cells.", "Die Tabelle ist als präsentational ausgezeichnet, enthält aber Kopfzellen."),
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
        // Namensquellen nach HTML-AAM („table Element Accessible Name
        // Computation"): aria-labelledby, aria-label, caption, title.
        let is_named = n.children().any(|c| c.is_element("caption"))
            || ["aria-label", "aria-labelledby", "title"]
                .iter()
                .any(|a| n.attr(a).is_some_and(|v| !v.trim().is_empty()));
        if !is_named {
            out.push(
                Finding::review(
                    "tables/name-missing",
                    pick!(
                        locale,
                        "The table has neither a <caption> nor an aria-label.",
                        "Die Tabelle hat weder <caption> noch aria-label.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
        // Ohne `<th>` fehlt die Kopfzeile nur einer Datentabelle mit
        // sichtbaren Köpfen (F91); eine Layouttabelle oder eine Tabelle ohne
        // Köpfe ist zulässig. Was davon vorliegt, zeigt das Markup nicht —
        // `REVIEW` (auditmysite#779).
        if !has_header_cell(n) {
            out.push(
                Finding::review(
                    "tables/header-missing",
                    pick!(
                        locale,
                        "The table has no <th> header cells.",
                        "Die Tabelle hat keine <th>-Kopfzellen.",
                    ),
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
        scope: Scope::Markup,
        wcag: &["3.1.1"],
        severity: Severity::High,
        help: "The <html> element needs a valid lang attribute.",
        help_de: de_text!("Das <html>-Element braucht ein gültiges lang-Attribut."),
    },
    Meta {
        ids: &["document/title-missing", "document/title-empty"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["2.4.2"],
        severity: Severity::High,
        help: "Every page needs a meaningful <title>.",
        help_de: de_text!("Jede Seite braucht einen aussagekräftigen <title>."),
    },
    Meta {
        ids: &[
            "zoom/viewport-locked",
            "zoom/viewport-scale-limited",
            "zoom/viewport-missing",
        ],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.4.4"],
        severity: Severity::High,
        help: "A viewport must be present and must not prevent zooming.",
        help_de: de_text!("Der Viewport muss vorhanden sein und darf Zoomen nicht verhindern."),
    },
    Meta {
        ids: &[
            "headings/empty",
            "headings/skip-level",
            "headings/h1-missing",
            "headings/h1-multiple",
            "headings/none",
        ],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "2.4.1", "2.4.6"],
        severity: Severity::Medium,
        help: "Headings form the outline; do not skip levels.",
        help_de: de_text!("Überschriften bilden die Gliederung; Ebenen nicht überspringen."),
    },
    Meta {
        ids: &["images/alt-missing", "images/alt-suspicious"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Informative images need descriptive alt text.",
        help_de: de_text!("Informative Bilder brauchen einen beschreibenden Alt-Text."),
    },
    Meta {
        ids: &["forms/label-missing", "forms/placeholder-as-label"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "3.3.2", "4.1.2"],
        severity: Severity::Critical,
        help: "Every input needs an associated label.",
        help_de: de_text!("Jedes Eingabefeld braucht ein zugeordnetes Label."),
    },
    Meta {
        ids: &["aria/role-invalid", "aria/role-abstract"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Use only roles from the ARIA specification.",
        help_de: de_text!("Nur Rollen aus der ARIA-Spezifikation verwenden."),
    },
    Meta {
        ids: &["aria/reference-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "ARIA references must point to IDs that exist.",
        help_de: de_text!("ARIA-Verweise müssen auf vorhandene IDs zeigen."),
    },
    Meta {
        ids: &["aria/required-attribute-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "A role that announces a state needs the attribute carrying it.",
        help_de: de_text!(
            "Eine Rolle, die einen Zustand ansagt, braucht das Attribut, das ihn trägt."
        ),
    },
    Meta {
        ids: &["ids/duplicate"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "An ID that another element references must be unique.",
        help_de: de_text!("Eine ID, auf die ein anderes Element verweist, muss eindeutig sein."),
    },
    Meta {
        ids: &["keyboard/positive-tabindex"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["2.4.3"],
        severity: Severity::High,
        help: "Positive tabindex values break the tab order.",
        help_de: de_text!("Positive tabindex-Werte brechen die Tabreihenfolge."),
    },
    Meta {
        ids: &["keyboard/hidden-focusable"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "Focusable elements must not be aria-hidden.",
        help_de: de_text!("Fokussierbare Elemente dürfen nicht aria-hidden sein."),
    },
    Meta {
        ids: &[
            "lists/invalid-structure",
            "lists/empty",
            "lists/item-outside-list",
            "lists/term-without-definition",
        ],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        help: "<ul> and <ol> may only have <li> as direct children and must not be empty; \
               a list item must stand inside a list.",
        help_de: de_text!(
            "<ul> und <ol> dürfen als direkte Kinder nur <li> haben und nicht leer sein; \
                  ein Listeneintrag muss in einer Liste stehen."
        ),
    },
    Meta {
        ids: &[
            "tables/header-missing",
            "tables/name-missing",
            "tables/presentational-with-headers",
        ],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::High,
        help: "Data tables need <th> header cells.",
        help_de: de_text!("Datentabellen brauchen <th>-Kopfzellen."),
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
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "2.4.1"],
        severity: Severity::High,
        help: "Landmarks structure the page for everyone who cannot see it.",
        help_de: de_text!("Landmarks gliedern die Seite für alle, die sie nicht sehen können."),
    },
    Meta {
        ids: &["keyboard/skip-link-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.1"],
        severity: Severity::Medium,
        help: "A skip link bypasses blocks that repeat before the content.",
        help_de: de_text!("Ein Sprunglink überspringt wiederkehrende Bereiche vor dem Inhalt."),
    },
    Meta {
        ids: &["aria/attribute-unknown"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Use only aria-* attributes defined by WAI-ARIA.",
        help_de: de_text!("Nur aria-*-Attribute verwenden, die WAI-ARIA definiert."),
    },
    Meta {
        ids: &["aria/attribute-value-invalid"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "ARIA attributes need a value of their type.",
        help_de: de_text!("ARIA-Attribute brauchen einen Wert ihres Typs."),
    },
    Meta {
        ids: &["aria/owns-conflict"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "An element may be owned by only one aria-owns.",
        help_de: de_text!("Ein Element darf nur einem aria-owns gehören."),
    },
    Meta {
        ids: &[
            "aria/tab-selected-missing",
            "aria/tabpanel-missing",
            "aria/combobox-popup-missing",
        ],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::Medium,
        help: "Tabs mark the selected tab; an expanded combobox references its popup.",
        help_de: de_text!(
            "Tabs zeichnen den gewählten Tab aus; eine aufgeklappte Combobox verweist auf ihr Popup."
        ),
    },
    Meta {
        ids: &["popover/target-missing", "popover/target-invalid"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "popovertarget must reference an element with the popover attribute.",
        help_de: de_text!("popovertarget muss auf ein Element mit popover-Attribut verweisen."),
    },
    Meta {
        ids: &["inert/dialog-inert"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["2.1.1", "4.1.2"],
        severity: Severity::High,
        help: "An open dialog must not be inert.",
        help_de: de_text!("Ein offener Dialog darf nicht inert sein."),
    },
    crate::checklist::META,
    crate::checklist::META_MOTION,
    Meta {
        ids: &["forms/autocomplete-invalid"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.5"],
        severity: Severity::Low,
        help: "autocomplete must use the autofill tokens of the HTML standard.",
        help_de: de_text!("autocomplete muss die Autofill-Angaben des HTML-Standards verwenden."),
    },
    Meta {
        ids: &["forms/error-unidentified"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.3.1"],
        severity: Severity::Medium,
        help: "A field marked aria-invalid needs an error description via aria-describedby or aria-errormessage.",
        help_de: de_text!(
            "Ein als aria-invalid ausgezeichnetes Feld braucht eine Fehlerbeschreibung per aria-describedby oder aria-errormessage."
        ),
    },
    Meta {
        ids: &["forms/group-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        help: "Radio buttons and checkboxes sharing a name belong in a fieldset with legend or a named group.",
        help_de: de_text!(
            "Optionsfelder und gleichnamige Kontrollkästchen gehören in ein fieldset mit legend oder eine benannte Gruppe."
        ),
    },
    Meta {
        ids: &["forms/no-submit"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.2.2"],
        severity: Severity::Medium,
        help: "A form that collects input needs an explicit submit control.",
        help_de: de_text!(
            "Ein Formular, das Eingaben sammelt, braucht ein ausdrückliches Absende-Element."
        ),
    },
    Meta {
        ids: &["forms/redundant-entry"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.3.7"],
        severity: Severity::Medium,
        help: "Information entered once is prefilled, autofillable or selectable when asked again.",
        help_de: de_text!(
            "Einmal Eingegebenes ist bei erneuter Abfrage vorbelegt, automatisch ausfüllbar oder auswählbar."
        ),
    },
    Meta {
        ids: &["context/on-input"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.2.2"],
        severity: Severity::Medium,
        help: "Changing a selection must not submit, navigate or open a window by itself.",
        help_de: de_text!(
            "Eine Auswahl zu ändern darf nicht von selbst absenden, navigieren oder ein Fenster öffnen."
        ),
    },
    Meta {
        ids: &["context/on-focus", "context/autofocus"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["3.2.1"],
        severity: Severity::High,
        help: "Receiving focus must not change the context.",
        help_de: de_text!("Der Fokus allein darf den Kontext nicht wechseln."),
    },
    Meta {
        ids: &["auth/captcha"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["3.3.8"],
        severity: Severity::Medium,
        help: "A CAPTCHA in a sign-in form needs an alternative without a cognitive function test.",
        help_de: de_text!(
            "Ein Captcha im Anmeldeformular braucht eine Alternative ohne kognitiven Test."
        ),
    },
    Meta {
        ids: &["keyboard/click-handler-not-focusable"],
        tier: Tier::Structure,
        scope: Scope::Rendered,
        wcag: &["2.1.1"],
        severity: Severity::High,
        help: "An element with a click handler must be a control: a <button> or <a href>, or a \
               role and tabindex with keyboard handling.",
        help_de: de_text!(
            "Ein Element mit Klick-Handler muss ein Bedienelement sein: <button> oder \
                  <a href>, oder Rolle und tabindex mit Tastaturbedienung."
        ),
    },
    Meta {
        ids: &["links/used-as-button"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Low,
        help: "A link that triggers an action instead of navigating should be a <button>.",
        help_de: de_text!(
            "Ein Link, der eine Aktion auslöst statt zu navigieren, sollte ein <button> sein."
        ),
    },
    Meta {
        ids: &["navigation/location-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.8"],
        severity: Severity::Low,
        help: "Show users where they are: a breadcrumb trail or aria-current=\"page\" in the \
               navigation.",
        help_de: de_text!(
            "Zeigen, wo man sich befindet: ein Brotkrumenpfad oder aria-current=\"page\" in \
                  der Navigation."
        ),
    },
    Meta {
        ids: &["images/area-alt-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "An image map <area> with href needs alt text naming its target.",
        help_de: de_text!("Ein <area> mit href braucht einen Alt-Text, der sein Ziel nennt."),
    },
    Meta {
        ids: &["images/input-alt-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "An <input type=\"image\"> needs alt text describing its action.",
        help_de: de_text!(
            "Ein <input type=\"image\"> braucht einen Alt-Text, der seine Aktion beschreibt."
        ),
    },
    Meta {
        ids: &["images/server-side-map"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["2.1.1"],
        severity: Severity::Medium,
        help: "Do not use server-side image maps (ismap); use a client-side map or text links.",
        help_de: de_text!(
            "Keine serverseitigen Imagemaps (ismap); stattdessen eine clientseitige \
                  Imagemap oder Textlinks."
        ),
    },
    Meta {
        ids: &["objects/alt-missing"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "An <object> needs a text alternative: fallback content, aria-label or title.",
        help_de: de_text!(
            "Ein <object> braucht eine Textalternative: Ersatzinhalt, aria-label oder title."
        ),
    },
    Meta {
        ids: &["media/audio-autoplay"],
        tier: Tier::Structure,
        // Ton spielt auch, wenn das Element versteckt oder aus dem
        // Accessibility-Tree genommen ist (`hidden`, `aria-hidden`).
        scope: Scope::Markup,
        wcag: &["1.4.2"],
        severity: Severity::Medium,
        help: "Audio that plays automatically must stop within three seconds or be pausable.",
        help_de: de_text!(
            "Selbststartender Ton muss nach drei Sekunden enden oder sich anhalten lassen."
        ),
    },
    Meta {
        ids: &["viz/text-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Every visualisation (figure[data-viz]) needs a non-empty text layer \
               [data-viz-text] with its statement.",
        help_de: de_text!(
            "Jede Visualisierung (figure[data-viz]) braucht eine nicht leere Textschicht \
                  [data-viz-text] mit ihrer Aussage."
        ),
    },
    Meta {
        ids: &["display/text-hidden"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Never put hidden, aria-hidden=\"true\" or inert on [data-viz-text] or its \
               ancestors; hide it visually instead.",
        help_de: de_text!(
            "Kein hidden, aria-hidden=\"true\" oder inert an [data-viz-text] oder seinen \
                  Vorfahren; die Schicht nur visuell ausblenden."
        ),
    },
    Meta {
        ids: &["viz/caption-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.1.1"],
        severity: Severity::Low,
        help: "Every visualisation (figure[data-viz]) needs a <figcaption> that names it.",
        help_de: de_text!(
            "Jede Visualisierung (figure[data-viz]) braucht ein <figcaption>, das sie \
                  benennt."
        ),
    },
    Meta {
        ids: &["viz/static-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["2.2.2"],
        severity: Severity::Medium,
        help: "A 3d or interactive visualisation needs a still image [data-viz-static] for calm \
               mode.",
        help_de: de_text!(
            "Eine 3D- oder interaktive Visualisierung braucht ein Standbild \
                  [data-viz-static] für den ruhigen Modus."
        ),
    },
    Meta {
        ids: &["viz/table-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["1.3.1"],
        severity: Severity::Low,
        help: "A chart (data-viz=\"chart\") should give its values as a <table>.",
        help_de: de_text!(
            "Ein Diagramm (data-viz=\"chart\") sollte seine Werte als <table> angeben."
        ),
    },
    Meta {
        ids: &["display/toggle-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["2.2.2"],
        severity: Severity::Medium,
        help: "A page with visualisations needs a display-mode toggle marked \
               [data-display-toggle].",
        help_de: de_text!(
            "Eine Seite mit Visualisierungen braucht einen mit [data-display-toggle] \
                  markierten Umschalter für den Darstellungsmodus."
        ),
    },
    Meta {
        ids: &["display/init-missing"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["2.2.2"],
        severity: Severity::Low,
        help: "Set html[data-display] in a small blocking script in <head>, before <body>.",
        help_de: de_text!(
            "html[data-display] in einem kleinen blockierenden Skript in <head> setzen, vor \
                  <body>."
        ),
    },
    Meta {
        ids: &["tables/header-without-data", "tables/data-undetermined"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::High,
        help: "Header cells need data cells to label; a table of headers only labels nothing.",
        help_de: de_text!(
            "Kopfzellen brauchen Datenzellen, die sie beschriften; eine Tabelle nur aus \
                  Kopfzellen beschriftet nichts."
        ),
    },
    Meta {
        ids: &["tables/headers-attr-invalid"],
        tier: Tier::Structure,
        // Nur Zellen, die dargestellt und im Accessibility-Tree sind (ACT
        // a25f45: Tabelle sichtbar und im Baum); eine versteckte Tabelle
        // vermittelt in diesem Zustand keine Zuordnung.
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::High,
        help: "The headers attribute of a cell must list the ids of header cells in the same \
               table.",
        help_de: de_text!(
            "Das headers-Attribut einer Zelle muss die IDs von Kopfzellen derselben Tabelle \
                  nennen."
        ),
    },
    Meta {
        ids: &["document/lang-mismatch"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["3.1.1"],
        severity: Severity::Medium,
        help: "lang and xml:lang on <html> must name the same language.",
        help_de: de_text!("lang und xml:lang am <html>-Element müssen dieselbe Sprache nennen."),
    },
    Meta {
        ids: &["language/part-unmarked", "language/part-undetermined"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.1.2"],
        severity: Severity::Medium,
        help: "A passage in another language than the page needs its own lang attribute.",
        help_de: de_text!(
            "Eine Passage in einer anderen Sprache als die Seite braucht ein eigenes \
                  lang-Attribut."
        ),
    },
    Meta {
        ids: &["language/abbreviation-unexpanded"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["3.1.4"],
        severity: Severity::Low,
        help: "Make the expansion of an abbreviation available, e.g. in the title of <abbr>.",
        help_de: de_text!(
            "Die Langform einer Abkürzung verfügbar machen, etwa im title von <abbr>."
        ),
    },
    Meta {
        ids: &["timing/meta-refresh"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &["2.2.1"],
        severity: Severity::High,
        help: "Do not reload or redirect with a delayed <meta http-equiv=\"refresh\">; redirect \
               on the server or let users trigger it.",
        help_de: de_text!(
            "Nicht mit verzögertem <meta http-equiv=\"refresh\"> neu laden oder \
                  weiterleiten; serverseitig weiterleiten oder Nutzer auslösen lassen."
        ),
    },
    Meta {
        ids: &["headings/section-without-heading"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.10"],
        severity: Severity::Low,
        help: "Give articles and named sections a heading so users can find their content.",
        help_de: de_text!(
            "Artikeln und benannten Abschnitten eine Überschrift geben, damit man ihren \
                  Inhalt findet."
        ),
    },
    Meta {
        ids: &["aria/role-redundant"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &[],
        severity: Severity::Low,
        help: "Do not set a role the element already has implicitly.",
        help_de: de_text!("Keine Rolle setzen, die das Element ohnehin implizit hat."),
    },
    Meta {
        ids: &["names/title-only"],
        tier: Tier::Structure,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "Do not rely on the title attribute as a control's only name.",
        help_de: de_text!(
            "Das title-Attribut nicht zum einzigen Namen eines Bedienelements machen."
        ),
    },
    Meta {
        ids: &["patterns/tooltip-unreferenced"],
        tier: Tier::Structure,
        scope: Scope::Markup,
        wcag: &[],
        severity: Severity::Low,
        help: "Reference a role=\"tooltip\" from its trigger with aria-describedby.",
        help_de: de_text!(
            "Auf ein role=\"tooltip\" vom auslösenden Element aus mit aria-describedby \
                  verweisen."
        ),
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
fn functions<D: Document>() -> [fn(&D, Locale, &mut Vec<Finding>); 57] {
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
        crate::aria::attribute_names,
        crate::aria::attribute_values,
        crate::aria::owns_conflict,
        crate::aria::widgets,
        crate::aria::popover,
        crate::aria::inert_dialog,
        crate::checklist::checklist,
        crate::checklist::motion,
        crate::forms::autocomplete_invalid,
        crate::forms::error_unidentified,
        crate::forms::group_missing,
        crate::forms::no_submit,
        crate::forms::redundant_entry,
        crate::forms::on_input,
        crate::forms::on_focus,
        crate::forms::captcha,
        crate::links::click_handler,
        crate::links::used_as_button,
        crate::links::location,
        crate::media::area_alt,
        crate::media::input_alt,
        crate::media::server_side_map,
        crate::media::object_alt,
        crate::media::audio_autoplay,
        crate::viz::text,
        crate::viz::text_hidden,
        crate::viz::caption,
        crate::viz::static_layer,
        crate::viz::table,
        crate::viz::toggle,
        crate::viz::init,
        crate::tables::header_without_data,
        crate::tables::headers_attr,
        crate::document::lang_mismatch,
        crate::document::part_unmarked,
        crate::document::abbreviations,
        crate::document::meta_refresh,
        crate::document::section_without_heading,
        crate::roles::redundant,
        crate::roles::title_only,
        crate::roles::tooltip,
    ]
}

const _: () = assert!(METAS.len() == 57);

/// Alle Tier-1-Regeln.
pub fn rules<D: Document>() -> Vec<StructureRule<D>> {
    METAS
        .iter()
        .zip(functions::<D>())
        .map(|(meta, run)| StructureRule { meta: *meta, run })
        .collect()
}
