//! Die Checkliste: Kriterien, die keine Maschine entscheiden kann.
//!
//! Ob ein Video Untertitel braucht, ob `alt=""` an einem Bild stimmt, ob ein
//! fett gesetzter Absatz eigentlich eine Überschrift ist — das sieht nur ein
//! Mensch. Schweigen wäre hier die falsche Antwort: Es sähe aus wie bestanden.
//! Deshalb entsteht je Kriterium und Seite genau ein `UNTESTED`-Befund, sobald
//! es auf der Seite etwas gibt, für das das Kriterium gilt. Er sagt, was von
//! Hand zu prüfen ist, und steht am Wurzelknoten — er gilt der Seite, nicht
//! einem Element.
//!
//! Anlass: LiveAudit schwieg am 29.09.2026 auf barrierlab.eu zu acht
//! absichtlich eingebauten Barrieren dieser Art, nicht einmal `UNTESTED`
//! (liveaudit#7).
//!
//! Ein Befund je Seite, nicht je Element: Auf einer Seite mit zweihundert
//! Bildern wären zweihundert gleiche Einträge keine Checkliste mehr.

use a11y_dom::{Document, Node, NodeKind, Tier, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick};
use crate::registry::Meta;
use crate::sicht::Scope;

/// Ein Punkt der Checkliste.
struct Punkt {
    id: &'static str,
    wcag: &'static [&'static str],
    severity: Severity,
    /// Ob es auf der Seite etwas gibt, für das der Punkt gilt.
    gilt: fn(&Anlass) -> bool,
    en: &'static str,
    #[cfg(feature = "de")]
    de: &'static str,
}

/// Was die Seite enthält, einmal über alle Elemente gezählt.
#[derive(Default)]
struct Anlass {
    medien: bool,
    bilder: bool,
    text: bool,
    links: bool,
    felder: bool,
    auto_refresh: bool,
    anmeldung: bool,
}

impl Anlass {
    fn erhebe<D: Document>(doc: &D) -> Self {
        let mut a = Anlass::default();
        for n in elements(doc) {
            match n.local_name() {
                "video" | "audio" => a.medien = true,
                "img" | "area" => a.bilder = true,
                "a" if n.has_attr("href") => a.links = true,
                "select" | "textarea" => a.felder = true,
                "input" => match input_typ(n).as_str() {
                    "hidden" | "submit" | "button" | "reset" => {}
                    "image" => a.bilder = true,
                    "password" => {
                        a.felder = true;
                        a.anmeldung = true;
                    }
                    _ => a.felder = true,
                },
                "meta"
                    if n.attr("http-equiv")
                        .is_some_and(|h| h.eq_ignore_ascii_case("refresh")) =>
                {
                    a.auto_refresh = true;
                }
                _ => {}
            }
            if n.attr("role") == Some("img") {
                a.bilder = true;
            }
            if sieht_aus_wie_captcha(n) {
                a.anmeldung = true;
            }
            if !a.text
                && n.children()
                    .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
                && !matches!(n.local_name(), "title" | "script" | "style" | "noscript")
            {
                a.text = true;
            }
        }
        a
    }
}

fn input_typ<'a, N: Node<'a>>(n: N) -> String {
    n.attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "text".into())
}

/// Die verbreiteten Captcha-Dienste binden sich über Klassen, IDs oder die
/// Quelle eines Frames ein. Eine Heuristik — sie entscheidet nur, ob der Punkt
/// auf der Checkliste erscheint, nicht über einen Befund.
fn sieht_aus_wie_captcha<'a, N: Node<'a>>(n: N) -> bool {
    ["class", "id", "src"].iter().any(|a| {
        n.attr(a).is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("captcha") || v.contains("turnstile")
        })
    })
}

const PUNKTE: &[Punkt] = &[
    Punkt {
        id: "manual/media-alternatives",
        wcag: &["1.2.1", "1.2.2", "1.2.3", "1.2.5"],
        severity: Severity::High,
        gilt: |a| a.medien,
        en: "Check by hand: audio and video need captions, a transcript and, where the \
             picture carries information, audio description.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Audio und Video brauchen Untertitel, ein Transkript und, wo \
             das Bild Information trägt, Audiodeskription.",
    },
    Punkt {
        id: "manual/image-alternatives",
        wcag: &["1.1.1"],
        severity: Severity::High,
        gilt: |a| a.bilder,
        en: "Check by hand: every alt text says what the image means in its place, and \
             only purely decorative images have alt=\"\".",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Jeder Alt-Text sagt, was das Bild an seiner Stelle bedeutet, \
             und nur rein dekorative Bilder haben alt=\"\".",
    },
    Punkt {
        id: "manual/visual-structure",
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        gilt: |a| a.text,
        en: "Check by hand: what looks like a heading, list, table or menu is marked up as \
             one — not just styled to look like it.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Was wie eine Überschrift, Liste, Tabelle oder ein Menü \
             aussieht, ist auch so ausgezeichnet — nicht nur so gestaltet.",
    },
    Punkt {
        id: "manual/use-of-color",
        wcag: &["1.4.1"],
        severity: Severity::Medium,
        gilt: |a| a.links || a.felder,
        en: "Check by hand: colour is never the only way to tell something — links in \
             text, required fields, errors, states.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Farbe ist nie das einzige Unterscheidungsmerkmal — Links im \
             Text, Pflichtfelder, Fehler, Zustände.",
    },
    Punkt {
        id: "manual/timing",
        wcag: &["2.2.1"],
        severity: Severity::High,
        gilt: |a| a.felder || a.auto_refresh,
        en: "Check by hand: any time limit can be turned off, adjusted or extended.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Jede Zeitbegrenzung lässt sich abschalten, anpassen oder \
             verlängern.",
    },
    Punkt {
        id: "manual/error-handling",
        wcag: &["3.3.1", "3.3.3"],
        severity: Severity::High,
        gilt: |a| a.felder,
        en: "Check by hand: submit the form with errors — each error is named in text, next \
             to its field, with a suggestion how to fix it.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Formular fehlerhaft absenden — jeder Fehler wird in Textform \
             am Feld benannt, mit einem Vorschlag zur Korrektur.",
    },
    Punkt {
        id: "manual/authentication",
        wcag: &["3.3.8"],
        severity: Severity::High,
        gilt: |a| a.anmeldung,
        en: "Check by hand: signing in needs no cognitive test — no puzzle captcha, paste \
             and password managers work.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Die Anmeldung verlangt keinen kognitiven Test — kein \
             Rätsel-Captcha, Einfügen und Passwortmanager funktionieren.",
    },
];

pub(crate) const META: Meta = Meta {
    ids: &[
        "manual/media-alternatives",
        "manual/image-alternatives",
        "manual/visual-structure",
        "manual/use-of-color",
        "manual/timing",
        "manual/error-handling",
        "manual/authentication",
    ],
    tier: Tier::Structure,
    scope: Scope::AccessibilityTree,
    wcag: &[
        "1.1.1", "1.2.1", "1.2.2", "1.2.3", "1.2.5", "1.3.1", "1.4.1", "2.2.1", "3.3.1", "3.3.3",
        "3.3.8",
    ],
    severity: Severity::Medium,
    help: "Criteria no machine can decide. Each appears once per page when the page contains \
           something it applies to, as UNTESTED — a checklist, not a verdict.",
    #[cfg(feature = "de")]
    help_de: "Kriterien, die keine Maschine entscheiden kann. Jedes erscheint einmal je Seite, \
              wenn die Seite etwas enthält, für das es gilt, als UNTESTED — eine Checkliste, \
              kein Urteil.",
};

pub(crate) fn checkliste<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let anlass = Anlass::erhebe(doc);
    let wurzel = Location::node(doc.root().id().to_string());
    for p in PUNKTE.iter().filter(|p| (p.gilt)(&anlass)) {
        out.push(
            Finding::untested(p.id, pick!(locale, p.en, p.de))
                .with_severity(p.severity)
                .with_wcag(p.wcag.iter().copied())
                .at(wurzel.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jeder_punkt_ist_deklariert() {
        let ids: Vec<&str> = PUNKTE.iter().map(|p| p.id).collect();
        assert_eq!(ids, META.ids);
    }
}
