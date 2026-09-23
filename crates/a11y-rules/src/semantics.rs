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

use a11y_dom::{elements, Node, NodeId, Semantics, Tier};
use a11y_report::{Finding, Location, Severity};

use crate::registry::{Meta, SemanticsRule};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn named<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> bool {
    doc.accessible_name(n).is_some_and(|s| !s.trim().is_empty())
}

fn link_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("a") || !n.has_attr("href") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail("links/name-missing", "The link has no accessible name.")
                    .with_severity(Severity::Critical)
                    .with_wcag(["2.4.4", "4.1.2"])
                    .at(at(n.id())),
            );
        }
    }
}

fn button_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let ist_button = n.is_element("button") || doc.role(n).as_deref() == Some("button");
        if !ist_button || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail("buttons/name-missing", "The button has no accessible name.")
                    .with_severity(Severity::Critical)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
            );
        }
    }
}

fn svg_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("svg") || doc.is_ignored(n) {
            continue;
        }
        // Rein dekoratives SVG ist korrekt ausgezeichnet und gemeint.
        if matches!(n.attr("role"), Some("presentation") | Some("none"))
            || n.attr("aria-hidden") == Some("true")
        {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "svg/name-missing",
                    "The SVG has no accessible name and is not marked decorative.",
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
fn generic_link_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
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
                    format!(
                        "The link text \"{}\" says nothing about its target.",
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

fn ambiguous_link_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
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
                    format!("Several links are named \"{name}\" but point to different targets."),
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
        wcag: &["2.4.4", "4.1.2"],
        severity: Severity::Critical,
        help: "Every link needs a name that describes its target.",
    },
    Meta {
        ids: &["buttons/name-missing"],
        tier: Tier::Semantics,
        wcag: &["4.1.2"],
        severity: Severity::Critical,
        help: "Every button needs a name that describes what it does.",
    },
    Meta {
        ids: &["svg/name-missing"],
        tier: Tier::Semantics,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Informative SVGs need a name, decorative ones role=\"presentation\".",
    },
    Meta {
        ids: &["links/ambiguous-name"],
        tier: Tier::Semantics,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Links with the same name should point to the same target.",
    },
    Meta {
        ids: &["links/generic-name"],
        tier: Tier::Semantics,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Link text should say where it leads without the surrounding sentence.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
fn funktionen<D: Semantics>() -> [fn(&D, &mut Vec<Finding>); 5] {
    [
        link_names,
        button_names,
        svg_names,
        ambiguous_link_names,
        generic_link_names,
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
