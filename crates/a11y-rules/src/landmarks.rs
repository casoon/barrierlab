//! Landmarks, Tastaturerreichbarkeit und Dialogfokus nach WAI-ARIA 1.2
//! (Landmark-Rollen), HTML-AAM (`header`, `footer`, `aside`, `section`,
//! `form`) und WCAG 2.2 (1.3.1, 2.1.1, 2.4.3, 4.1.2).
//!
//! Portiert aus auditmysite (`region`, `landmark_granular`, `keyboard`, aus
//! `patterns` `accordion-no-controls` und `dialog-no-focusable`;
//! casoon/barrierlab#17). auditmysite ist der Vergleichspunkt, nicht die
//! Norm — Abweichungen stehen an der jeweiligen Stelle und im CHANGELOG.
//!
//! Alle Regeln hier sind Tier 2: Ob ein `<form>` oder `<section>` eine
//! Landmark ist, hängt an seinem Accessible Name (HTML-AAM, auditmysite#727),
//! und ob ein Element interaktiv ist, an seiner Rolle. Die Landmark-Rolle
//! selbst bestimmt [`landmark`] aus dem Markup und nicht aus der Rolle des
//! Hosts: `accname` gibt einem `<header>` in `<main>` noch `banner`, ältere
//! Chrome-Fassungen ebenso (auditmysite#639).

use std::collections::{HashMap, HashSet};

use a11y_dom::{Node, NodeId, NodeKind, Semantics, ancestors, descendants, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::semantics::named;
use crate::structure::{explizite_rolle, ist_dokumentweit};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- Landmark-Rolle ----------------------------------------------------------

/// Die Landmark-Rollen aus WAI-ARIA 1.2.
const LANDMARK_ROLLEN: &[&str] = &[
    "banner",
    "complementary",
    "contentinfo",
    "form",
    "main",
    "navigation",
    "region",
    "search",
];

/// Ob ein `<aside>` in Sectioning Content steht, das nicht `main` ist. Dann
/// ist es nach HTML-AAM nur mit Namen `complementary` (Korpus
/// `landmark_aside_scoping`).
fn in_sektion<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| {
        matches!(a.local_name(), "article" | "aside" | "nav" | "section")
            || matches!(
                explizite_rolle(a),
                Some("article" | "complementary" | "navigation" | "region")
            )
    })
}

/// Die Landmark-Rolle eines Elements, oder `None`, wenn es keine Landmark ist.
///
/// Die explizite Rolle zählt vor dem Tag. `form` und `region` sind nur mit
/// Accessible Name Landmarks (WAI-ARIA 1.2, HTML-AAM) — Chrome meldet ein
/// unbenanntes `<form>` trotzdem mit der Rolle `form` (auditmysite#727).
/// `<header>`/`<footer>` sind nur dokumentweit `banner`/`contentinfo`,
/// `<aside>` in einem Abschnitt nur mit Namen `complementary`.
fn landmark<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> Option<&'static str> {
    let rolle: &'static str = match explizite_rolle(n) {
        Some(r) => LANDMARK_ROLLEN.iter().copied().find(|l| *l == r)?,
        None => match n.local_name() {
            "main" => "main",
            "nav" => "navigation",
            "search" => "search",
            "header" if ist_dokumentweit(n) => "banner",
            "footer" if ist_dokumentweit(n) => "contentinfo",
            "aside" if !in_sektion(n) || named(doc, n) => "complementary",
            "section" => "region",
            "form" => "form",
            _ => return None,
        },
    };
    if matches!(rolle, "form" | "region") && !named(doc, n) {
        return None;
    }
    Some(rolle)
}

/// Alle Landmarks in Dokumentreihenfolge.
fn landmarks_von<'n, D: Semantics>(doc: &'n D) -> Vec<(D::N<'n>, &'static str)> {
    elements(doc)
        .filter(|n| !doc.is_ignored(*n))
        .filter_map(|n| landmark(doc, n).map(|r| (n, r)))
        .collect()
}

// --- landmarks/not-unique ------------------------------------------------------

/// Landmarks derselben Rolle mit demselben Namen (auch beide ohne Namen) sind
/// nicht zu unterscheiden. Gemeldet wird jedes beteiligte Element.
///
/// `REVIEW` statt auditmysites Verstoß: WAI-ARIA 1.2 und die APG verlangen
/// unterscheidbare Namen nur als SHOULD, und WCAG 1.3.1 setzt keine Namen an
/// Landmarks voraus (axe: best practice). Schwere wie in auditmysite.
pub(crate) fn not_unique<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let alle: Vec<_> = landmarks_von(doc)
        .into_iter()
        .map(|(n, rolle)| {
            let name = doc
                .accessible_name(n)
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            (n, rolle, name)
        })
        .collect();
    let mut anzahl: HashMap<(&str, &str), usize> = HashMap::new();
    for (_, rolle, name) in &alle {
        *anzahl.entry((rolle, name.as_str())).or_default() += 1;
    }
    for (n, rolle, name) in &alle {
        if anzahl[&(*rolle, name.as_str())] < 2 {
            continue;
        }
        let text = if name.is_empty() {
            tr!(
                locale,
                "Several \"{rolle}\" landmarks have no name and cannot be told apart.",
                "Mehrere \"{rolle}\"-Landmarks haben keinen Namen und sind nicht zu unterscheiden.",
            )
        } else {
            tr!(
                locale,
                "Several \"{rolle}\" landmarks share the name \"{name}\" and cannot be told apart.",
                "Mehrere \"{rolle}\"-Landmarks heißen \"{name}\" und sind nicht zu unterscheiden.",
            )
        };
        out.push(
            Finding::review("landmarks/not-unique", text)
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
        );
    }
}

// --- landmarks/not-top-level, *-duplicate ------------------------------------

/// `banner`, `contentinfo` und `main` gehören auf die oberste Ebene (WAI-ARIA
/// 1.2 bei den drei Rollen); `banner` und `contentinfo` je höchstens einmal.
/// Wie in auditmysite `FAIL`, mittel. Ein unbenanntes `<form>` oder
/// `<section>` ist keine Landmark und macht nichts darin verschachtelt
/// (auditmysite#727). Der Duplikatbefund zeigt wie `landmarks/main-duplicate`
/// auf die zweite Landmark, nicht wie auditmysite auf die erste.
pub(crate) fn structure<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let alle = landmarks_von(doc);
    for (n, rolle) in &alle {
        if !matches!(*rolle, "banner" | "contentinfo" | "main") {
            continue;
        }
        let Some(aussen) = ancestors(*n).find_map(|a| landmark(doc, a)) else {
            continue;
        };
        out.push(
            Finding::fail(
                "landmarks/not-top-level",
                tr!(
                    locale,
                    "The \"{rolle}\" landmark is nested inside a \"{aussen}\" landmark.",
                    "Die \"{rolle}\"-Landmark steckt in einer \"{aussen}\"-Landmark.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1"])
            .at(at(n.id())),
        );
    }
    for (rolle, kennung) in [
        ("banner", "landmarks/banner-duplicate"),
        ("contentinfo", "landmarks/contentinfo-duplicate"),
    ] {
        let gleiche: Vec<_> = alle.iter().filter(|(_, r)| *r == rolle).collect();
        if gleiche.len() < 2 {
            continue;
        }
        let zahl = gleiche.len();
        out.push(
            Finding::fail(
                kennung,
                tr!(
                    locale,
                    "The document has {zahl} \"{rolle}\" landmarks; at most one is allowed.",
                    "Das Dokument hat {zahl} \"{rolle}\"-Landmarks; höchstens eine ist vorgesehen.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1"])
            .at(at(gleiche[1].0.id())),
        );
    }
}

// --- landmarks/content-outside -----------------------------------------------

/// Rollen, die mit Namen als Inhalt zählen — dieselbe Liste wie auditmysite
/// `region`.
const INHALT_ROLLEN: &[&str] = &[
    "heading",
    "paragraph",
    "link",
    "button",
    "textbox",
    "checkbox",
    "radio",
    "img",
    "image",
    "listitem",
    "list",
    "table",
    "cell",
    "row",
];

/// Elemente, deren Text kein Inhalt der Seite ist.
fn kein_inhalt<'a, N: Node<'a>>(n: N) -> bool {
    matches!(
        n.local_name(),
        "script" | "style" | "template" | "noscript" | "svg"
    )
}

/// Die Sprunglinks der Seite, am Verhalten erkannt: Fragmentlinks vor dem
/// ersten Link, der die Seite verlässt (axe `isSkipLink`, auditmysite#642).
/// Sie stehen absichtlich vor jeder Landmark.
fn sprunglinks<D: Semantics>(doc: &D) -> HashSet<NodeId> {
    elements(doc)
        .filter(|n| n.is_element("a"))
        .filter(|n| n.attr("href").is_some_and(|h| !h.trim().is_empty()))
        .take_while(|n| {
            n.attr("href")
                .is_some_and(|h| h.starts_with('#') && h.len() > 1)
        })
        .map(|n| n.id())
        .collect()
}

/// Ob unter `n` (einschließlich) etwas steht, das als Inhalt zählt: Text oder
/// ein benanntes Element einer [`INHALT_ROLLEN`]. Sprunglinks zählen nicht.
fn hat_inhalt<'n, D: Semantics>(doc: &'n D, n: D::N<'n>, sprung: &HashSet<NodeId>) -> bool {
    match n.kind() {
        NodeKind::Text => !n.text().trim().is_empty(),
        NodeKind::Element => {
            if kein_inhalt(n) || sprung.contains(&n.id()) {
                return false;
            }
            let benannt = doc
                .role(n)
                .is_some_and(|r| INHALT_ROLLEN.contains(&r.as_str()))
                && named(doc, n);
            benannt || n.children().any(|k| hat_inhalt(doc, k, sprung))
        }
    }
}

/// Inhalt außerhalb jeder Landmark. Wer per Landmark springt, erreicht ihn
/// nicht.
///
/// Gemeldet wird das äußerste Element ohne Landmark darin, nicht jeder
/// Textknoten und jedes benannte Element einzeln wie in auditmysite — ein
/// Block, ein Befund (wie axe `region`). Steht Text unmittelbar neben einer
/// Landmark, trägt der umgebende Container den Befund. `FAIL`, mittel wie in
/// auditmysite.
pub(crate) fn content_outside<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let sprung = sprunglinks(doc);
    let mut mit_landmark: HashSet<NodeId> = HashSet::new();
    for (n, _) in landmarks_von(doc) {
        mit_landmark.insert(n.id());
        mit_landmark.extend(ancestors(n).map(|a| a.id()));
    }
    let Some(body) = elements(doc).find(|n| n.is_element("body")) else {
        return;
    };
    let mut stapel = vec![body];
    while let Some(n) = stapel.pop() {
        let mut gemeldet = false;
        for k in n.children() {
            match k.kind() {
                NodeKind::Text if !gemeldet && !k.text().trim().is_empty() => {
                    melden(n.id(), locale, out);
                    gemeldet = true;
                }
                NodeKind::Element => {
                    if mit_landmark.contains(&k.id()) {
                        // Eine Landmark selbst ist erledigt; ein Container mit
                        // Landmarks darin wird weiter zerlegt.
                        if landmark(doc, k).is_none() {
                            stapel.push(k);
                        }
                    } else if hat_inhalt(doc, k, &sprung) {
                        melden(k.id(), locale, out);
                    }
                }
                _ => {}
            }
        }
    }
}

fn melden(id: NodeId, locale: Locale, out: &mut Vec<Finding>) {
    out.push(
        Finding::fail(
            "landmarks/content-outside",
            pick!(
                locale,
                "Content is not contained in any landmark.",
                "Inhalt steht außerhalb jeder Landmark.",
            ),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["1.3.1"])
        .at(at(id)),
    );
}

// --- keyboard/focusable-no-role, keyboard/interactive-not-focusable ----------

/// Ob das Element ohne `tabindex` fokussierbar ist (HTML, „focusable area").
fn nativ_fokussierbar<'a, N: Node<'a>>(n: N) -> bool {
    if n.has_attr("contenteditable")
        && !n
            .attr("contenteditable")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"))
    {
        return true;
    }
    match n.local_name() {
        "a" | "area" => n.has_attr("href"),
        "input" => !n
            .attr("type")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden")),
        "audio" | "video" => n.has_attr("controls"),
        "button" | "select" | "textarea" | "summary" | "iframe" => true,
        _ => false,
    }
}

fn tabindex<'a, N: Node<'a>>(n: N) -> Option<i32> {
    n.attr("tabindex")
        .and_then(|t| t.trim().parse::<i32>().ok())
}

/// Ob das Element fokussiert werden kann, auch nur per Skript
/// (`tabindex="-1"`) — das, was Chrome als `focusable` meldet.
fn fokussierbar<'a, N: Node<'a>>(n: N) -> bool {
    tabindex(n).is_some() || nativ_fokussierbar(n)
}

fn deaktiviert<'a, N: Node<'a>>(n: N) -> bool {
    n.has_attr("disabled")
        && matches!(
            n.local_name(),
            "button" | "input" | "select" | "textarea" | "fieldset" | "optgroup" | "option"
        )
}

fn inert<'a, N: Node<'a>>(n: N) -> bool {
    n.has_attr("inert") || ancestors(n).any(|a| a.has_attr("inert"))
}

/// Nicht interaktive Rollen aus auditmysite `keyboard`, ohne `region`: Ein
/// benannter, fokussierbarer Bereich ist das empfohlene Muster für scrollbare
/// Bereiche (axe `scrollable-region-focusable`, auditmysite-Korpus
/// `scrollable_region_focusable`).
const NICHT_INTERAKTIV: &[&str] = &[
    "generic",
    "group",
    "article",
    "section",
    "paragraph",
    "none",
    "presentation",
];

/// Interaktive Rollen, die fokussierbar sein müssen (auditmysite `keyboard`).
const INTERAKTIV: &[&str] = &[
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

/// Ob ein Vorfahre den Fokus per `aria-activedescendant` verwaltet. Dann sind
/// seine Optionen, Einträge und Menüpunkte zu Recht nicht selbst fokussierbar
/// (WAI-ARIA 1.2, `aria-activedescendant`).
fn fokus_verwaltet<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| {
        a.attr("aria-activedescendant")
            .is_some_and(|v| !v.trim().is_empty())
    })
}

/// WCAG 2.1.1, aus dem Markup erschlossen, beides `REVIEW` wie in
/// auditmysite (#569): Ob ein Element per Tastatur bedienbar ist, hängt an
/// Skripten, die keine Datenschicht zeigt.
///
/// - `keyboard/focusable-no-role` (niedrig): per Tab erreichbar
///   (`tabindex` ≥ 0), aber ohne interaktive Rolle. Abweichend von auditmysite
///   zählt nur die Tabfolge: Ein `tabindex="-1"` (Ziel eines Sprunglinks)
///   erreicht niemand mit der Tabtaste.
/// - `keyboard/interactive-not-focusable` (hoch): interaktive Rolle, aber
///   nicht fokussierbar. Ausgenommen sind deaktivierte Felder, native
///   `<option>` (Chrome führt sie nicht als `option`) und Elemente unter
///   `aria-activedescendant`.
pub(crate) fn keyboard<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || inert(n) {
            continue;
        }
        let rolle = doc.role(n);
        let rolle = rolle.as_deref();
        if tabindex(n).is_some_and(|t| t >= 0)
            && !nativ_fokussierbar(n)
            && rolle.is_none_or(|r| NICHT_INTERAKTIV.contains(&r))
        {
            out.push(
                Finding::review(
                    "keyboard/focusable-no-role",
                    pick!(
                        locale,
                        "The element is in the tab order but has no interactive role; check that it is operable by keyboard.",
                        "Das Element liegt in der Tabfolge, hat aber keine interaktive Rolle; prüfen, ob es per Tastatur bedienbar ist.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["2.1.1"])
                .at(at(n.id())),
            );
        }
        let Some(r) = rolle.filter(|r| INTERAKTIV.contains(r)) else {
            continue;
        };
        if fokussierbar(n) || deaktiviert(n) || n.is_element("option") || fokus_verwaltet(n) {
            continue;
        }
        out.push(
            Finding::review(
                "keyboard/interactive-not-focusable",
                tr!(
                    locale,
                    "The element has role \"{r}\" but cannot receive keyboard focus; check for tabindex or script handling.",
                    "Das Element hat die Rolle \"{r}\", ist aber nicht per Tastatur fokussierbar; tabindex oder Skriptbehandlung prüfen.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.1.1"])
            .at(at(n.id())),
        );
    }
}

// --- dialog/focusable-missing -------------------------------------------------

/// Ein offener Dialog ohne ein einziges fokussierbares Element darin: Wer mit
/// der Tastatur arbeitet, kann ihn weder bedienen noch schließen. `FAIL`,
/// mittel, 2.4.3 wie auditmysite `dialog-no-focusable`. Abweichend zählt jeder
/// Nachfahre, nicht nur die direkten Kinder.
pub(crate) fn dialog_focusable<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || (n.is_element("dialog") && !n.has_attr("open")) {
            continue;
        }
        if !matches!(doc.role(n).as_deref(), Some("dialog" | "alertdialog")) {
            continue;
        }
        if descendants(n)
            .any(|d| d.kind() == NodeKind::Element && fokussierbar(d) && !deaktiviert(d))
        {
            continue;
        }
        out.push(
            Finding::fail(
                "dialog/focusable-missing",
                pick!(
                    locale,
                    "The dialog contains no focusable element.",
                    "Der Dialog enthält kein fokussierbares Element.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.4.3"])
            .at(at(n.id())),
        );
    }
}

// --- patterns/accordion-controls-missing -----------------------------------------

/// Ein aufgeklappter Button (`aria-expanded="true"`) ohne `aria-controls`:
/// Welcher Bereich aufgeklappt ist, erfährt niemand (APG, Accordion).
///
/// Nur aufgeklappt, wie auditmysite `accordion-no-controls`; ausgenommen sind
/// `<summary>` (der Bereich ist das umgebende `<details>`) und Buttons in
/// `navigation`/`banner` (Aufklappmenüs). `REVIEW` statt auditmysites Verstoß:
/// WAI-ARIA 1.2 verlangt `aria-controls` am Button nicht, das
/// Disclosure-Muster der APG nennt es optional. Schwere wie in auditmysite.
pub(crate) fn accordion_controls<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || n.is_element("summary") {
            continue;
        }
        if !n
            .attr("aria-expanded")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            || n.attr("aria-controls")
                .is_some_and(|v| !v.trim().is_empty())
        {
            continue;
        }
        if doc.role(n).as_deref() != Some("button") {
            continue;
        }
        if ancestors(n).any(|a| matches!(landmark(doc, a), Some("navigation" | "banner"))) {
            continue;
        }
        out.push(
            Finding::review(
                "patterns/accordion-controls-missing",
                pick!(
                    locale,
                    "The expanded button does not reference the region it controls with aria-controls.",
                    "Der aufgeklappte Button verweist nicht per aria-controls auf den Bereich, den er steuert.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}
