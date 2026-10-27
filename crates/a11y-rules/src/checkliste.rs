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
    gesten: bool,
    unterbrechungen: bool,
}

impl Anlass {
    fn erhebe<D: Document>(doc: &D) -> Self {
        let mut a = Anlass::default();
        for n in elements(doc) {
            match n.local_name() {
                "video" | "audio" => a.medien = true,
                "iframe" if n.attr("src").is_some_and(videoplattform) => a.medien = true,
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
                    a.unterbrechungen = true;
                }
                "dialog" => a.unterbrechungen = true,
                _ => {}
            }
            if n.attr("role") == Some("img") {
                a.bilder = true;
            }
            if matches!(
                n.attr("role"),
                Some("alert" | "alertdialog" | "status" | "log" | "marquee" | "timer")
            ) || n
                .attr("aria-live")
                .is_some_and(|v| !v.trim().eq_ignore_ascii_case("off"))
            {
                a.unterbrechungen = true;
            }
            if sieht_aus_wie_gesten(n) {
                a.gesten = true;
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

/// Die Player der Videoplattformen, die auditmysite (`media_rules`,
/// `VIDEO_EMBED_HOSTS`) kennt. Ihre Untertitel steuert der Player der
/// Plattform; im Markup der Seite steht davon nichts. Beleg: das
/// YouTube-Video auf w3.org/WAI.
fn videoplattform(src: &str) -> bool {
    const HOSTS: &[&str] = &[
        "youtube.com",
        "youtube-nocookie.com",
        "vimeo.com",
        "dailymotion.com",
        "wistia.com",
        "wistia.net",
    ];
    let Some((_, rest)) = src.trim().split_once("//") else {
        return false;
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    HOSTS
        .iter()
        .any(|h| host == *h || host.strip_suffix(h).is_some_and(|v| v.ends_with('.')))
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

/// Wischen, Ziehen, Zoomen: `draggable`, Touch- und Zeiger-Handler im Markup
/// und die verbreiteten Karussell- und Kartenbibliotheken, die sich über
/// Klassen einbinden. Wie beim Captcha entscheidet die Heuristik nur, ob der
/// Punkt auf der Checkliste erscheint.
fn sieht_aus_wie_gesten<'a, N: Node<'a>>(n: N) -> bool {
    const HANDLER: &[&str] = &[
        "ontouchstart",
        "ontouchmove",
        "onpointermove",
        "ongesturestart",
        "ondragstart",
    ];
    const KLASSEN: &[&str] = &[
        "swiper",
        "slick",
        "splide",
        "glide",
        "flickity",
        "carousel",
        "leaflet",
        "mapboxgl",
        "maplibregl",
        "ol-viewport",
    ];
    n.attr("draggable")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
        || HANDLER.iter().any(|h| n.has_attr(h))
        || n.attr("class").is_some_and(|c| {
            c.split_ascii_whitespace().any(|k| {
                let k = k.to_ascii_lowercase();
                KLASSEN
                    .iter()
                    .any(|w| k == *w || k.starts_with(&format!("{w}-")))
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
    Punkt {
        id: "manual/pointer-gestures",
        wcag: &["2.5.1"],
        severity: Severity::Medium,
        gilt: |a| a.gesten,
        en: "Check by hand: everything done by swiping, dragging a path or with two \
             fingers also works with a single tap or click — for example arrow buttons \
             on a carousel, zoom buttons on a map.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Alles, was per Wischen, Ziehen entlang eines Pfads oder mit \
             zwei Fingern geht, geht auch mit einem einzelnen Tippen oder Klick — etwa \
             Pfeiltasten am Karussell, Zoomknöpfe an der Karte.",
    },
    Punkt {
        id: "manual/no-timing",
        wcag: &["2.2.3"],
        severity: Severity::Low,
        gilt: |a| a.felder || a.auto_refresh,
        en: "Check by hand: no task depends on timing at all, except synchronised media \
             and real-time events.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Keine Aufgabe hängt überhaupt von Zeit ab, außer \
             synchronisierte Medien und Echtzeit-Ereignisse.",
    },
    Punkt {
        id: "manual/interruptions",
        wcag: &["2.2.4"],
        severity: Severity::Low,
        gilt: |a| a.unterbrechungen,
        en: "Check by hand: updates, alerts and pop-ups can be postponed or turned off, \
             unless they concern an emergency.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Aktualisierungen, Meldungen und Einblendungen lassen sich \
             aufschieben oder abschalten, außer sie betreffen einen Notfall.",
    },
    Punkt {
        id: "manual/re-authenticating",
        wcag: &["2.2.5"],
        severity: Severity::Low,
        gilt: |a| a.anmeldung,
        en: "Check by hand: when a session expires, signing in again continues without \
             losing entered data.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Läuft die Sitzung ab, geht es nach erneuter Anmeldung ohne \
             Verlust der eingegebenen Daten weiter.",
    },
    Punkt {
        id: "manual/unusual-words",
        wcag: &["3.1.3"],
        severity: Severity::Low,
        gilt: |a| a.text,
        en: "Check by hand: jargon, idioms and unusual words are explained — inline, \
             with <dfn> or in a glossary.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Fachbegriffe, Redewendungen und ungewöhnliche Wörter werden \
             erklärt — im Text, mit <dfn> oder in einem Glossar.",
    },
    Punkt {
        id: "manual/help",
        wcag: &["3.3.5"],
        severity: Severity::Low,
        gilt: |a| a.felder,
        en: "Check by hand: fields that need it have context-sensitive help — \
             instructions, examples or a link to help.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Felder, die es brauchen, haben kontextbezogene Hilfe — \
             Anleitung, Beispiele oder einen Link zur Hilfe.",
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
        "manual/pointer-gestures",
        "manual/no-timing",
        "manual/interruptions",
        "manual/re-authenticating",
        "manual/unusual-words",
        "manual/help",
    ],
    tier: Tier::Structure,
    // Was dargestellt wird, auch unter `aria-hidden`: Untertitel (1.2.2),
    // Farbe (1.4.1) und Zeitbegrenzung (2.2.1) betreffen auch Inhalt, den die
    // Assistenztechnik nicht sieht — ein sichtbares Video unter
    // `aria-hidden="true"` braucht trotzdem Untertitel.
    scope: Scope::Rendered,
    wcag: &[
        "1.1.1", "1.2.1", "1.2.2", "1.2.3", "1.2.5", "1.3.1", "1.4.1", "2.2.1", "2.2.3", "2.2.4",
        "2.2.5", "2.5.1", "3.1.3", "3.3.1", "3.3.3", "3.3.5", "3.3.8",
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

/// Bewegungssteuerung (2.5.4) steht nicht im Accessibility-Tree, sondern im
/// Skript: Das einzige Zeichen im Dokument ist ein Inline-`<script>`, der
/// `devicemotion` oder `deviceorientation` abonniert. Deshalb ein eigener
/// Punkt mit [`Scope::Markup`] — `<script>` ist nie dargestellt. Was ein
/// externes Skript abonniert, sieht keine Prüfung am Dokument.
pub(crate) const META_BEWEGUNG: Meta = Meta {
    ids: &["manual/motion-actuation"],
    tier: Tier::Structure,
    scope: Scope::Markup,
    wcag: &["2.5.4"],
    severity: Severity::Medium,
    help: "Checklist item for motion actuation, shown when an inline script listens to \
           device motion or orientation.",
    #[cfg(feature = "de")]
    help_de: "Checklistenpunkt für Bewegungssteuerung, erscheint, wenn ein Inline-Skript \
              auf Gerätebewegung oder -ausrichtung hört.",
};

pub(crate) fn bewegung<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let hoert_auf_bewegung = elements(doc).any(|n| {
        n.is_element("script")
            && n.children().any(|k| {
                k.kind() == NodeKind::Text
                    && (k.text().contains("devicemotion") || k.text().contains("deviceorientation"))
            })
    });
    if hoert_auf_bewegung {
        out.push(
            Finding::untested(
                "manual/motion-actuation",
                pick!(
                    locale,
                    "Check by hand: whatever reacts to shaking or tilting the device also \
                     works with a control on the page, and the motion response can be \
                     turned off.",
                    "Von Hand prüfen: Was auf Schütteln oder Neigen des Geräts reagiert, geht \
                     auch über ein Bedienelement auf der Seite, und die Reaktion auf Bewegung \
                     lässt sich abschalten."
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.5.4"])
            .at(Location::node(doc.root().id().to_string())),
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
