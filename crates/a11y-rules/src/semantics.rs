//! Tier-2-Regeln: brauchen einen echten Accessible Name.
//!
//! Warum das nicht strukturell geht: Der Accessible Name eines Links oder
//! Buttons entsteht nach einem mehrstufigen Verfahren — `aria-labelledby` löst
//! Verweisketten auf, versteckte Teilbäume zählen unter bestimmten Bedingungen
//! doch mit, `alt` eines eingebetteten Bildes fließt ein. Eine Näherung aus
//! Teilbaumtext plus `aria-label` liegt in genau den Fällen falsch, in denen
//! es darauf ankommt.
//!
//! Hosts ohne [`Semantics`] melden diese Regeln als `UNTESTED` — siehe
//! [`crate::run`].
//!
//! [`Semantics`]: a11y_dom::Semantics

use a11y_dom::{Node, NodeId, Semantics, Tier, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::registry::{Meta, SemanticsRule};
use crate::sicht::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

pub(crate) fn named<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> bool {
    doc.accessible_name(n).is_some_and(|s| !s.trim().is_empty())
}

fn link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("a") || !n.has_attr("href") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "links/name-missing",
                    pick!(
                        locale,
                        "The link has no accessible name.",
                        "Der Link hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["2.4.4", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn button_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let ist_button = n.is_element("button") || doc.role(n).as_deref() == Some("button");
        // Eine `<summary>` meldet `summary/name-missing` — `accname` gibt ihr
        // die Rolle `button`, Chrome nicht; so meldet jeder Host sie einmal.
        if !ist_button || n.is_element("summary") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "buttons/name-missing",
                    pick!(
                        locale,
                        "The button has no accessible name.",
                        "Der Button hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn svg_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("svg") || doc.is_ignored(n) {
            continue;
        }
        // Rein dekoratives SVG ist korrekt ausgezeichnet und gemeint.
        if n.attr("role").is_some_and(|r| {
            r.split_whitespace().next().is_some_and(|x| {
                x.eq_ignore_ascii_case("presentation") || x.eq_ignore_ascii_case("none")
            })
        }) || n.attr("aria-hidden") == Some("true")
        {
            continue;
        }
        // Das Icon in einem benannten Link oder Button: Der Name des
        // Bedienelements trägt die Aussage, die Grafik braucht keinen eigenen.
        // Beleg: auditmysite `is_graphic_in_named_control` (mit.edu).
        if a11y_dom::ancestors(n).any(|a| {
            (a.is_element("a")
                || a.is_element("button")
                || a.attr("role").is_some_and(|r| {
                    r.split_whitespace().next().is_some_and(|x| {
                        x.eq_ignore_ascii_case("link") || x.eq_ignore_ascii_case("button")
                    })
                }))
                && named(doc, a)
        }) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "svg/name-missing",
                    pick!(locale, "The SVG has no accessible name and is not marked decorative.", "Das SVG hat keinen zugänglichen Namen und ist nicht als dekorativ ausgezeichnet."),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Gleicher Linktext, unterschiedliches Ziel — für Screenreader-Nutzer, die
/// sich eine Linkliste ausgeben lassen, nicht unterscheidbar.
/// Namen, die für sich genommen nichts über das Ziel sagen.
///
/// Wer eine Liste aller Links abruft — Screenreader können das —, hört eine
/// Folge von „mehr", „hier", „weiterlesen". Welcher wohin führt, steht nur im
/// umgebenden Text, den diese Liste nicht mitliefert.
/// Bewusst eng gehalten. „Start" oder „Info" sind als Navigationsbeschriftung
/// völlig in Ordnung — aufgenommen ist nur, was auf den umgebenden Satz
/// angewiesen ist („hier", „dieser Link") oder gar nichts benennt („mehr",
/// „weiterlesen").
const NICHTSSAGEND: &[&str] = &[
    "hier",
    "hier klicken",
    "klick hier",
    "klicken sie hier",
    "siehe hier",
    "mehr",
    "mehr dazu",
    "mehr erfahren",
    "mehr lesen",
    "weiterlesen",
    "weiter",
    "dieser link",
    "link",
    "click here",
    "click",
    "here",
    "more",
    "read more",
    "learn more",
    "see more",
    "continue",
    "this link",
    "link here",
];

/// Heuristisch: Die Liste kann einen Namen treffen, der im Zusammenhang doch
/// eindeutig ist. Deshalb `REVIEW`, nicht `FAIL`.
fn generic_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("a") || !n.has_attr("href") || doc.is_ignored(n) {
            continue;
        }
        let Some(name) = doc.accessible_name(n) else {
            continue;
        };
        let key = name
            .trim()
            .trim_end_matches(['.', '!', '…', '>', '›', '→'])
            .trim()
            .to_lowercase();
        if NICHTSSAGEND.contains(&key.as_str()) {
            out.push(
                Finding::review(
                    "links/generic-name",
                    tr!(
                        locale,
                        "The link text \"{}\" says nothing about its target.",
                        "Der Linktext \"{}\" sagt nichts über das Ziel.",
                        name.trim()
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.4"])
                .at(at(n.id())),
            );
        }
    }
}

fn ambiguous_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    use std::collections::HashMap;
    let mut nach_name: HashMap<String, Vec<(NodeId, String)>> = HashMap::new();

    for n in elements(doc) {
        if !n.is_element("a") || doc.is_ignored(n) {
            continue;
        }
        let (Some(name), Some(href)) = (doc.accessible_name(n), n.attr("href")) else {
            continue;
        };
        let key = name.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        nach_name
            .entry(key)
            .or_default()
            .push((n.id(), href.to_string()));
    }

    for (name, treffer) in nach_name {
        if treffer.len() < 2 {
            continue;
        }
        let ziele: std::collections::HashSet<&str> =
            treffer.iter().map(|(_, h)| h.as_str()).collect();
        if ziele.len() < 2 {
            continue; // gleicher Text, gleiches Ziel: unproblematisch
        }
        for (id, _) in &treffer {
            out.push(
                Finding::review(
                    "links/ambiguous-name",
                    tr!(
                        locale,
                        "Several links are named \"{name}\" but point to different targets.",
                        "Mehrere Links heißen \"{name}\", zeigen aber auf verschiedene Ziele."
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.4"])
                .at(at(*id)),
            );
        }
    }
}

/// Die Deklarationen. Siehe [`crate::structure::METAS`] zur Begründung der
/// Trennung von den Funktionen.
pub const METAS: &[Meta] = &[
    Meta {
        ids: &["links/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4", "4.1.2"],
        severity: Severity::Critical,
        help: "Every link needs a name that describes its target.",
        #[cfg(feature = "de")]
        help_de: "Jeder Link braucht einen Namen, der sein Ziel beschreibt.",
    },
    Meta {
        ids: &["buttons/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Critical,
        help: "Every button needs a name that describes what it does.",
        #[cfg(feature = "de")]
        help_de: "Jeder Button braucht einen Namen, der seine Wirkung beschreibt.",
    },
    Meta {
        ids: &["svg/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Informative SVGs need a name, decorative ones role=\"presentation\".",
        #[cfg(feature = "de")]
        help_de: "Informative SVGs brauchen einen Namen, dekorative role=\"presentation\".",
    },
    Meta {
        ids: &["links/ambiguous-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Links with the same name should point to the same target.",
        #[cfg(feature = "de")]
        help_de: "Gleich benannte Links sollten auf dasselbe Ziel zeigen.",
    },
    Meta {
        ids: &["links/generic-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Link text should say where it leads without the surrounding sentence.",
        #[cfg(feature = "de")]
        help_de: "Der Linktext soll auch ohne den umgebenden Satz sagen, wohin er führt.",
    },
    Meta {
        ids: &["aria/attribute-not-allowed", "aria/attribute-prohibited"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Use only ARIA attributes the element's role supports.",
        #[cfg(feature = "de")]
        help_de: "Nur ARIA-Attribute verwenden, die die Rolle des Elements unterstützt.",
    },
    Meta {
        ids: &[
            "aria/required-parent-missing",
            "aria/required-children-missing",
        ],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "Roles that belong together must be nested as WAI-ARIA requires.",
        #[cfg(feature = "de")]
        help_de: "Zusammengehörige Rollen müssen verschachtelt sein, wie WAI-ARIA es verlangt.",
    },
    Meta {
        ids: &["names/required-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1", "4.1.2"],
        severity: Severity::High,
        help: "Elements whose role requires a name (fields, toggles, menu items, meters, …) need one.",
        #[cfg(feature = "de")]
        help_de: "Elemente, deren Rolle einen Namen verlangt (Felder, Schalter, Menüpunkte, Messanzeigen, …), brauchen einen.",
    },
    Meta {
        ids: &["names/symbol-only"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "An accessible name should be words, not a single symbol.",
        #[cfg(feature = "de")]
        help_de: "Ein zugänglicher Name sollte aus Wörtern bestehen, nicht aus einem einzelnen Symbol.",
    },
    Meta {
        ids: &["dialog/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Every dialog needs a name, usually via aria-labelledby on its heading.",
        #[cfg(feature = "de")]
        help_de: "Jeder Dialog braucht einen Namen, meist per aria-labelledby auf seine Überschrift.",
    },
    Meta {
        ids: &["dialog/modal-unmarked"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "A modal dialog should carry aria-modal=\"true\".",
        #[cfg(feature = "de")]
        help_de: "Ein modaler Dialog sollte aria-modal=\"true\" tragen.",
    },
    Meta {
        ids: &["summary/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "The <summary> of a <details> element needs text.",
        #[cfg(feature = "de")]
        help_de: "Die <summary> eines <details>-Elements braucht Text.",
    },
    Meta {
        ids: &["status/live-overridden"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.3"],
        severity: Severity::High,
        help: "Live regions should keep the aria-live their role implies.",
        #[cfg(feature = "de")]
        help_de: "Live-Regionen sollten das aria-live behalten, das ihre Rolle vorgibt.",
    },
    Meta {
        ids: &["label-in-name/mismatch"],
        tier: Tier::Semantics,
        scope: Scope::Rendered,
        wcag: &["2.5.3"],
        severity: Severity::Medium,
        help: "The accessible name must contain the visible label text.",
        #[cfg(feature = "de")]
        help_de: "Der zugängliche Name muss den sichtbaren Beschriftungstext enthalten.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
fn funktionen<D: Semantics>() -> [fn(&D, Locale, &mut Vec<Finding>); 14] {
    [
        link_names,
        button_names,
        svg_names,
        ambiguous_link_names,
        generic_link_names,
        crate::aria::attributes_allowed,
        crate::aria::required_context,
        crate::names::required_names,
        crate::names::symbol_names,
        crate::names::dialog_names,
        crate::names::dialog_modal,
        crate::names::summary_names,
        crate::names::live_regions,
        crate::names::label_in_name,
    ]
}

/// Alle Tier-2-Regeln.
pub fn rules<D: Semantics>() -> Vec<SemanticsRule<D>> {
    METAS
        .iter()
        .zip(funktionen::<D>())
        .map(|(meta, run)| SemanticsRule { meta: *meta, run })
        .collect()
}
