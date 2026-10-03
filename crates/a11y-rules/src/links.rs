//! Links- und Zeigerregeln nach WCAG 2.2 (2.1.1, 2.4.8, 4.1.2), dem
//! HTML-Standard (`<a>` ohne `href`) und WAI-ARIA 1.2 (`aria-current`).
//!
//! Portiert aus auditmysite (`click_handlers`, `fake_navigation_link`,
//! `location`; casoon/barrierlab#18). auditmysite ist der Vergleichspunkt,
//! nicht die Norm — Abweichungen stehen an der jeweiligen Stelle und im
//! CHANGELOG.
//!
//! Sichtbar ist nur das Inline-Attribut `onclick`. Ein per
//! `addEventListener` angehängter Handler steht nicht im Markup; dafür gibt es
//! auf dem Rendering-Tier `keyboard/pointer-only`, das `cursor: pointer`
//! auswertet.

use a11y_dom::{Document, Node, NodeId, ancestors, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick};
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Rollen, die ein Element bedienbar machen. Dieselbe Menge wie in
/// auditmysite.
const INTERAKTIVE_ROLLEN: &[&str] = &[
    "button",
    "link",
    "checkbox",
    "radio",
    "switch",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "tab",
    "option",
    "treeitem",
];

fn in_tabfolge<'a, N: Node<'a>>(n: N) -> bool {
    n.attr("tabindex")
        .and_then(|t| t.trim().parse::<i64>().ok())
        .is_some_and(|t| t >= 0)
}

// --- keyboard/click-handler-not-focusable -----------------------------------

/// Ein Inline-`onclick` an einem Element, das weder von sich aus noch über
/// Rolle und `tabindex` bedienbar ist.
///
/// Die Elemente sind die aus auditmysite, dazu `<a>` ohne `href`: Nach dem
/// HTML-Standard ist das kein Link, sondern ein Platzhalter — nicht
/// fokussierbar, ohne Rolle. auditmysite meldet es als
/// `fake_navigation_link`, obwohl es gar nicht als Link angesagt wird.
pub(crate) fn klick_ohne_tastatur<'a, N: Node<'a>>(n: N) -> bool {
    let nicht_interaktiv = matches!(
        n.local_name(),
        "div" | "span" | "p" | "li" | "section" | "article"
    ) || (n.is_element("a") && !n.has_attr("href"));
    n.has_attr("onclick")
        && nicht_interaktiv
        && !explizite_rolle(n).is_some_and(|r| INTERAKTIVE_ROLLEN.contains(&r))
        && !in_tabfolge(n)
}

/// 2.1.1: Klickbar, aber per Tastatur weder erreichbar noch auslösbar.
pub(crate) fn click_handler<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| klick_ohne_tastatur(*n)) {
        out.push(
            Finding::fail(
                "keyboard/click-handler-not-focusable",
                pick!(
                    locale,
                    "This element has an onclick handler but is neither a control nor in the tab \
                     order; keyboard users cannot trigger it.",
                    "Das Element hat einen onclick-Handler, ist aber weder Bedienelement noch in \
                     der Tabfolge; per Tastatur lässt es sich nicht auslösen.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.1.1"])
            .at(at(n.id())),
        );
    }
}

// --- links/used-as-button ----------------------------------------------------

/// Ein `href`, das nirgendwohin führt: leer, nur `#` oder eine
/// `javascript:`-Adresse.
fn fuehrt_nirgendwohin(href: &str) -> bool {
    let href = href.trim();
    href.is_empty()
        || href == "#"
        || href
            .get(..11)
            .is_some_and(|s| s.eq_ignore_ascii_case("javascript:"))
}

/// 4.1.2: Ein Link, der eine Aktion auslöst statt zu navigieren. Angesagt
/// wird „Link", die Leertaste löst ihn nicht aus.
///
/// Beleg aus echten Seiten: Der Aufruf der Consent-Einstellungen
/// (`<a href="#" onclick="UC_UI_recall();">`, wetter.com) und
/// `<a href="#" onclick="location.reload(true);">` (craigslist.org).
pub(crate) fn used_as_button<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(href) = n.attr("href") else {
            continue;
        };
        if n.is_element("a")
            && n.has_attr("onclick")
            && fuehrt_nirgendwohin(href)
            && explizite_rolle(n) != Some("button")
        {
            out.push(
                Finding::fail(
                    "links/used-as-button",
                    pick!(
                        locale,
                        "This link has no real target and triggers an action via onclick. It is \
                         announced as a link and does not respond to the Space key; use a \
                         <button>.",
                        "Der Link hat kein echtes Ziel und löst per onclick eine Aktion aus. \
                         Angesagt wird er als Link, auf die Leertaste reagiert er nicht; ein \
                         <button> gehört hierher.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- navigation/location-missing --------------------------------------------

fn ist_navigation<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("nav") || explizite_rolle(n) == Some("navigation")
}

/// 2.4.8 (AAA): Die Seite hat eine Navigation, zeigt aber weder einen
/// Brotkrumenpfad noch die aktuelle Seite mit `aria-current="page"` an.
///
/// `REVIEW` statt auditmysites Verstoß: 2.4.8 lässt sich auch mit Titel,
/// Überschriften oder einer Sitemap erfüllen, und auf einer Startseite ist der
/// Ort selbsterklärend. Der Brotkrumenpfad wird wie in auditmysite am Wort
/// „breadcrumb" im `aria-label` erkannt; `aria-current="true"` zählt nicht,
/// weil es auch Karussellpunkte markiert (t-online.de). Beleg: Die Startseiten
/// von spiegel.de, zeit.de und tagesschau.de lösen den Hinweis aus, die
/// Unterseiten von gov.uk und bundesregierung.de mit Brotkrumenpfad nicht.
pub(crate) fn location<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mit_navigation =
        elements(doc).any(|n| n.is_element("a") && ancestors(n).any(ist_navigation));
    let ort_angezeigt = elements(doc).any(|n| {
        n.attr("aria-current")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("page"))
            || n.attr("aria-label")
                .is_some_and(|l| l.to_ascii_lowercase().contains("breadcrumb"))
    });
    if mit_navigation && !ort_angezeigt {
        out.push(
            Finding::review(
                "navigation/location-missing",
                pick!(
                    locale,
                    "The page has navigation but neither a breadcrumb trail nor a link marked \
                     aria-current=\"page\". Check that users can tell where they are in the site.",
                    "Die Seite hat eine Navigation, aber weder einen Brotkrumenpfad noch einen \
                     mit aria-current=\"page\" ausgezeichneten Link. Prüfen, ob erkennbar ist, \
                     wo man sich auf der Website befindet.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["2.4.8"])
            .at(at(doc.root().id())),
        );
    }
}
