//! Rollen-, Namens- und Tooltip-Hinweise nach WCAG 2.2 (1.4.13, 4.1.2), ARIA
//! in HTML (überflüssige Rollen) und dem APG-Muster „Tooltip".
//!
//! Portiert aus auditmysite (`redundant_role`, `content_on_hover` mit
//! `title-only-description` und `content-on-hover-focus`;
//! casoon/barrierlab#20). auditmysite ist der Vergleichspunkt, nicht die Norm
//! — Abweichungen stehen an der jeweiligen Stelle und im CHANGELOG.
//!
//! Alles hier liest Tags und Attribute: Tier 1.

use std::collections::HashSet;

use a11y_dom::{Document, Node, NodeId, ancestors, descendants, elements, has_text};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn nicht_leer(v: Option<&str>) -> bool {
    v.is_some_and(|v| !v.trim().is_empty())
}

fn input_typ<'a, N: Node<'a>>(n: N) -> String {
    n.attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_default()
}

// --- aria/role-redundant ----------------------------------------------------

/// Die implizite Rolle eines Elements, wo sie nicht vom Kontext abhängt —
/// dieselbe Tabelle wie in auditmysite. `<header>`/`<footer>` fehlen, weil ihre
/// Rolle von den Vorfahren abhängt.
///
/// `<ul>`/`<ol>` mit `role="list"` fehlen ebenfalls: Bei `list-style: none`
/// nimmt WebKit/VoiceOver der Liste ihre Semantik, und `role="list"` ist die
/// übliche Abhilfe (auditmysite#644, barrierlab.eu). Ob die Liste so gestaltet
/// ist, steht im berechneten Stil, und den liefert `a11y-dom` nicht
/// (`ComputedStyle` kennt kein `list-style-type`). Das Paar wird deshalb gar
/// nicht geprüft, statt geraten.
fn implizite_rolle<'a, N: Node<'a>>(n: N) -> Option<&'static str> {
    Some(match n.local_name() {
        "button" => "button",
        "a" if n.has_attr("href") => "link",
        "textarea" => "textbox",
        "input" => match input_typ(n).as_str() {
            "" | "text" if !n.has_attr("list") => "textbox",
            "checkbox" => "checkbox",
            "radio" => "radio",
            "button" | "submit" | "reset" => "button",
            _ => return None,
        },
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        // Ein `<li>` ist nur in einer Liste ohne eigene Rolle ein `listitem`.
        // Trägt die Liste `role="list"`, gehört `role="listitem"` zur selben
        // WebKit-Abhilfe wie oben (lidl.de: `ul[role=list] > li[role=listitem]`
        // in einem Masonry-Grid).
        "li" if n.parent().is_some_and(|p| {
            matches!(p.local_name(), "ul" | "ol" | "menu") && !p.has_attr("role")
        }) =>
        {
            "listitem"
        }
        "table" => "table",
        "img" if nicht_leer(n.attr("alt")) => "img",
        _ => return None,
    })
}

/// 4.1.2: `role` wiederholt nur die Rolle, die das Element ohnehin hat. Ohne
/// Wirkung, aber manche Kombinationen aus Browser und Screenreader sagen sie
/// doppelt an; ARIA in HTML rät davon ab. Beleg: bahn.de und spiegel.de
/// (`<button role="button">`), wetter.com, lidl.de und sparkasse.de
/// (`<a href role="link">`) und der auditmysite-Korpus
/// `redundant_role_list_style` (`button#send-button`).
pub(crate) fn redundant<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(rolle) = explizite_rolle(n) else {
            continue;
        };
        if implizite_rolle(n) != Some(rolle) {
            continue;
        }
        let tag = n.local_name();
        out.push(
            Finding::fail(
                "aria/role-redundant",
                tr!(
                    locale,
                    "role=\"{rolle}\" only repeats the implicit role of <{tag}>; remove it.",
                    "role=\"{rolle}\" wiederholt nur die implizite Rolle von <{tag}>; \
                     entfernen."
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- names/title-only -------------------------------------------------------

/// Ob ein Element einen Namen aus einer Quelle vor `title` bekommt: ARIA,
/// Text, ein `alt` darin. Strukturell genähert, wie `headings/empty`.
fn name_vor_title<'a, N: Node<'a>>(n: N) -> bool {
    nicht_leer(n.attr("aria-label"))
        || nicht_leer(n.attr("aria-labelledby"))
        || has_text(n)
        || descendants(n).any(|d| {
            nicht_leer(d.attr("alt"))
                || nicht_leer(d.attr("aria-label"))
                || nicht_leer(d.attr("aria-labelledby"))
        })
}

/// Ein Eingabefeld mit zugeordnetem `<label>` — umschließend oder über `for`.
fn hat_label<'a, N: Node<'a>>(n: N, label_for: &HashSet<&str>) -> bool {
    ancestors(n).any(|a| a.is_element("label"))
        || n.attr("id").is_some_and(|id| label_for.contains(id))
}

/// Bedienelemente, deren Name nur aus `title` stammen kann. Textfelder meldet
/// `forms/title-only-label`; `submit` und `reset` haben einen Vorgabenamen.
fn kandidat<'a, N: Node<'a>>(n: N, label_for: &HashSet<&str>) -> bool {
    match n.local_name() {
        "button" => !name_vor_title(n),
        "a" => n.has_attr("href") && !name_vor_title(n),
        "input" => match input_typ(n).as_str() {
            "button" => !nicht_leer(n.attr("value")) && !name_vor_title(n),
            "image" => !nicht_leer(n.attr("alt")) && !name_vor_title(n),
            "checkbox" | "radio" => !hat_label(n, label_for) && !name_vor_title(n),
            _ => false,
        },
        _ => false,
    }
}

/// 4.1.2 (Best Practice): Ein Bedienelement, dessen einziger Name das
/// `title`-Attribut ist. Den Browser-Tooltip sieht nur, wer mit der Maus
/// darüberfährt — nicht per Tastatur, nicht auf Touch-Geräten.
///
/// Kein Fall von 1.4.13: Den `title`-Tooltip zeichnet der Browser
/// (auditmysite#711). `REVIEW` statt auditmysites Verstoß, wie bei
/// `forms/title-only-label`: `title` ist eine gültige Namensquelle (H65).
/// Beleg: auditmysite-Korpus `name_description_best_practice`
/// (`<button title="Search">` mit verstecktem SVG).
pub(crate) fn title_only<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let label_for: HashSet<&str> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for"))
        .collect();
    for n in elements(doc) {
        if !nicht_leer(n.attr("title")) || !kandidat(n, &label_for) {
            continue;
        }
        out.push(
            Finding::review(
                "names/title-only",
                pick!(
                    locale,
                    "The control is named only by its title attribute; the tooltip appears on \
                     mouse hover only, not on keyboard focus or touch. Give it visible text or \
                     an aria-label.",
                    "Das Bedienelement ist nur über sein title-Attribut benannt; der Tooltip \
                     erscheint nur beim Überfahren mit der Maus, nicht bei Tastaturfokus oder \
                     Touch. Sichtbarer Text oder ein aria-label gehört hierher.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- patterns/tooltip-unreferenced -----------------------------------------

/// 1.4.13: Ein `role="tooltip"`, auf das nichts verweist. Ohne Verweis sagt
/// die Assistenztechnik ihn nie an, wenn er erscheint.
///
/// Anders als auditmysite zählt neben `aria-describedby` auch
/// `aria-labelledby`: Ein Symbolknopf, der seinen Namen aus dem Tooltip
/// bekommt, sagt ihn an. Versteckte Tooltips zählen mit — sichtbar werden sie
/// erst beim Überfahren. Beleg: auditmysite-Korpus `forms_and_misc`
/// (`<div role="tooltip" id="orphan-tip">`).
pub(crate) fn tooltip<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let verwiesen: HashSet<&str> = elements(doc)
        .flat_map(|n| {
            ["aria-describedby", "aria-labelledby"]
                .into_iter()
                .filter_map(move |a| n.attr(a))
        })
        .flat_map(str::split_whitespace)
        .collect();
    for n in elements(doc) {
        if explizite_rolle(n) != Some("tooltip")
            || n.attr("id").is_some_and(|id| verwiesen.contains(id))
        {
            continue;
        }
        out.push(
            Finding::fail(
                "patterns/tooltip-unreferenced",
                pick!(
                    locale,
                    "No aria-describedby references this tooltip, so assistive technology never \
                     announces it. Reference it from its trigger.",
                    "Kein aria-describedby verweist auf diesen Tooltip; die Assistenztechnik \
                     sagt ihn nie an. Vom auslösenden Element aus darauf verweisen.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.4.13"])
            .at(at(n.id())),
        );
    }
}
