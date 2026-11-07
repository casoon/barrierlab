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

use crate::locale::{Locale, de_text, pick};
use crate::registry::Meta;
use crate::visibility::Scope;

/// Ein Punkt der Checkliste.
struct Point {
    id: &'static str,
    wcag: &'static [&'static str],
    severity: Severity,
    /// Ob es auf der Seite etwas gibt, für das der Punkt gilt.
    applies: fn(&Trigger) -> bool,
    en: &'static str,
    #[cfg(feature = "de")]
    de: &'static str,
}

/// Was die Seite enthält, einmal über alle Elemente gezählt.
#[derive(Default)]
struct Trigger {
    media: bool,
    images: bool,
    text: bool,
    links: bool,
    fields: bool,
    auto_refresh: bool,
    login: bool,
    gestures: bool,
    interruptions: bool,
    shortcut: bool,
    pointer_events: bool,
    hover_content: bool,
}

impl Trigger {
    fn survey<D: Document>(doc: &D) -> Self {
        let mut a = Trigger::default();
        for n in elements(doc) {
            match n.local_name() {
                "video" | "audio" => a.media = true,
                "iframe" if n.attr("src").is_some_and(video_platform) => a.media = true,
                "img" | "area" => a.images = true,
                "a" if n.has_attr("href") => a.links = true,
                "select" | "textarea" => a.fields = true,
                "input" => match input_type(n).as_str() {
                    "hidden" | "submit" | "button" | "reset" => {}
                    "image" => a.images = true,
                    "password" => {
                        a.fields = true;
                        a.login = true;
                    }
                    _ => a.fields = true,
                },
                "meta"
                    if n.attr("http-equiv")
                        .is_some_and(|h| h.eq_ignore_ascii_case("refresh")) =>
                {
                    a.auto_refresh = true;
                    a.interruptions = true;
                }
                "dialog" => a.interruptions = true,
                _ => {}
            }
            if n.attr("role") == Some("img") {
                a.images = true;
            }
            if matches!(
                n.attr("role"),
                Some("alert" | "alertdialog" | "status" | "log" | "marquee" | "timer")
            ) || n
                .attr("aria-live")
                .is_some_and(|v| !v.trim().eq_ignore_ascii_case("off"))
            {
                a.interruptions = true;
            }
            if looks_like_gestures(n) {
                a.gestures = true;
            }
            if n.has_attr("accesskey")
                || ["onkeydown", "onkeypress", "onkeyup"]
                    .iter()
                    .any(|h| n.has_attr(h))
                || (n.is_element("script")
                    && n.children().any(|k| {
                        k.kind() == NodeKind::Text
                            && ["keydown", "keypress", "keyup"]
                                .iter()
                                .any(|e| k.text().contains(e))
                    }))
            {
                a.shortcut = true;
            }
            if ["onmousedown", "onpointerdown", "ontouchstart"]
                .iter()
                .any(|h| n.has_attr(h))
            {
                a.pointer_events = true;
            }
            if n.attr("role") == Some("tooltip")
                || n.has_attr("popover")
                || n.has_attr("onmouseover")
                || n.attr("aria-haspopup")
                    .is_some_and(|v| !v.trim().eq_ignore_ascii_case("false"))
            {
                a.hover_content = true;
            }
            if looks_like_captcha(n) {
                a.login = true;
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

fn input_type<'a, N: Node<'a>>(n: N) -> String {
    n.attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "text".into())
}

/// Die Player der Videoplattformen, die auditmysite (`media_rules`,
/// `VIDEO_EMBED_HOSTS`) kennt. Ihre Untertitel steuert der Player der
/// Plattform; im Markup der Seite steht davon nichts. Beleg: das
/// YouTube-Video auf w3.org/WAI.
fn video_platform(src: &str) -> bool {
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
fn looks_like_captcha<'a, N: Node<'a>>(n: N) -> bool {
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
fn looks_like_gestures<'a, N: Node<'a>>(n: N) -> bool {
    const HANDLER: &[&str] = &[
        "ontouchstart",
        "ontouchmove",
        "onpointermove",
        "ongesturestart",
        "ondragstart",
    ];
    const CLASSES: &[&str] = &[
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
                CLASSES
                    .iter()
                    .any(|w| k == *w || k.starts_with(&format!("{w}-")))
            })
        })
}

const POINTS: &[Point] = &[
    Point {
        id: "manual/media-alternatives",
        wcag: &["1.2.1", "1.2.2", "1.2.3", "1.2.5"],
        severity: Severity::High,
        applies: |a| a.media,
        en: "Check by hand: audio and video need captions, a transcript and, where the \
             picture carries information, audio description.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Audio und Video brauchen Untertitel, ein Transkript und, wo \
             das Bild Information trägt, Audiodeskription.",
    },
    Point {
        id: "manual/image-alternatives",
        wcag: &["1.1.1"],
        severity: Severity::High,
        applies: |a| a.images,
        en: "Check by hand: every alt text says what the image means in its place, and \
             only purely decorative images have alt=\"\".",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Jeder Alt-Text sagt, was das Bild an seiner Stelle bedeutet, \
             und nur rein dekorative Bilder haben alt=\"\".",
    },
    Point {
        id: "manual/visual-structure",
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        applies: |a| a.text,
        en: "Check by hand: what looks like a heading, list, table or menu is marked up as \
             one — not just styled to look like it.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Was wie eine Überschrift, Liste, Tabelle oder ein Menü \
             aussieht, ist auch so ausgezeichnet — nicht nur so gestaltet.",
    },
    Point {
        id: "manual/use-of-color",
        wcag: &["1.4.1"],
        severity: Severity::Medium,
        applies: |a| a.links || a.fields,
        en: "Check by hand: colour is never the only way to tell something — links in \
             text, required fields, errors, states.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Farbe ist nie das einzige Unterscheidungsmerkmal — Links im \
             Text, Pflichtfelder, Fehler, Zustände.",
    },
    Point {
        id: "manual/timing",
        wcag: &["2.2.1"],
        severity: Severity::High,
        applies: |a| a.fields || a.auto_refresh,
        en: "Check by hand: any time limit can be turned off, adjusted or extended.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Jede Zeitbegrenzung lässt sich abschalten, anpassen oder \
             verlängern.",
    },
    Point {
        id: "manual/error-handling",
        wcag: &["3.3.1", "3.3.3"],
        severity: Severity::High,
        applies: |a| a.fields,
        en: "Check by hand: submit the form with errors — each error is named in text, next \
             to its field, with a suggestion how to fix it.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Formular fehlerhaft absenden — jeder Fehler wird in Textform \
             am Feld benannt, mit einem Vorschlag zur Korrektur.",
    },
    Point {
        id: "manual/authentication",
        wcag: &["3.3.8"],
        severity: Severity::High,
        applies: |a| a.login,
        en: "Check by hand: signing in needs no cognitive test — no puzzle captcha, paste \
             and password managers work.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Die Anmeldung verlangt keinen kognitiven Test — kein \
             Rätsel-Captcha, Einfügen und Passwortmanager funktionieren.",
    },
    Point {
        id: "manual/pointer-gestures",
        wcag: &["2.5.1"],
        severity: Severity::Medium,
        applies: |a| a.gestures,
        en: "Check by hand: everything done by swiping, dragging a path or with two \
             fingers also works with a single tap or click — for example arrow buttons \
             on a carousel, zoom buttons on a map.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Alles, was per Wischen, Ziehen entlang eines Pfads oder mit \
             zwei Fingern geht, geht auch mit einem einzelnen Tippen oder Klick — etwa \
             Pfeiltasten am Karussell, Zoomknöpfe an der Karte.",
    },
    Point {
        id: "manual/no-timing",
        wcag: &["2.2.3"],
        severity: Severity::Low,
        applies: |a| a.fields || a.auto_refresh,
        en: "Check by hand: no task depends on timing at all, except synchronised media \
             and real-time events.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Keine Aufgabe hängt überhaupt von Zeit ab, außer \
             synchronisierte Medien und Echtzeit-Ereignisse.",
    },
    Point {
        id: "manual/interruptions",
        wcag: &["2.2.4"],
        severity: Severity::Low,
        applies: |a| a.interruptions,
        en: "Check by hand: updates, alerts and pop-ups can be postponed or turned off, \
             unless they concern an emergency.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Aktualisierungen, Meldungen und Einblendungen lassen sich \
             aufschieben oder abschalten, außer sie betreffen einen Notfall.",
    },
    Point {
        id: "manual/re-authenticating",
        wcag: &["2.2.5"],
        severity: Severity::Low,
        applies: |a| a.login,
        en: "Check by hand: when a session expires, signing in again continues without \
             losing entered data.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Läuft die Sitzung ab, geht es nach erneuter Anmeldung ohne \
             Verlust der eingegebenen Daten weiter.",
    },
    Point {
        id: "manual/unusual-words",
        wcag: &["3.1.3"],
        severity: Severity::Low,
        applies: |a| a.text,
        en: "Check by hand: jargon, idioms and unusual words are explained — inline, \
             with <dfn> or in a glossary.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Fachbegriffe, Redewendungen und ungewöhnliche Wörter werden \
             erklärt — im Text, mit <dfn> oder in einem Glossar.",
    },
    Point {
        id: "manual/help",
        wcag: &["3.3.5"],
        severity: Severity::Low,
        applies: |a| a.fields,
        en: "Check by hand: fields that need it have context-sensitive help — \
             instructions, examples or a link to help.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Felder, die es brauchen, haben kontextbezogene Hilfe — \
             Anleitung, Beispiele oder einen Link zur Hilfe.",
    },
    Point {
        id: "manual/dragging-movements",
        wcag: &["2.5.7"],
        severity: Severity::Medium,
        applies: |a| a.gestures,
        en: "Check by hand: everything done by dragging also works with a single pointer without dragging — for example buttons to move an item or a slider you can click.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Alles, was per Ziehen geht, geht auch mit einem einzelnen Zeiger ohne Ziehen — etwa Knöpfe zum Verschieben eines Eintrags oder ein Regler, den man anklicken kann.",
    },
    Point {
        id: "manual/consistent-help",
        wcag: &["3.2.6"],
        severity: Severity::Low,
        applies: |a| a.text,
        en: "Check by hand: where help is offered — contact, chat, a help page — it sits in the same place on every page of the set.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Wo Hilfe angeboten wird — Kontakt, Chat, Hilfeseite —, steht sie auf jeder Seite des Angebots an derselben Stelle.",
    },
    Point {
        id: "manual/keyboard-trap",
        wcag: &["2.1.2"],
        severity: Severity::High,
        applies: |a| a.links || a.fields || a.interruptions,
        en: "Check by hand: with the keyboard alone you can move into and out of every control, dialog and embedded widget again.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Mit der Tastatur allein kommt man in jedes Bedienelement, jeden Dialog und jedes eingebettete Widget hinein und wieder heraus.",
    },
    Point {
        id: "manual/character-shortcuts",
        wcag: &["2.1.4"],
        severity: Severity::Medium,
        applies: |a| a.shortcut,
        en: "Check by hand: a shortcut made of a single character key can be turned off, remapped, or only works while the control has focus.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Ein Tastenkürzel aus einer einzelnen Zeichentaste lässt sich abschalten, umbelegen oder gilt nur, solange das Bedienelement den Fokus hat.",
    },
    Point {
        id: "manual/error-prevention",
        wcag: &["3.3.4"],
        severity: Severity::Medium,
        applies: |a| a.fields,
        en: "Check by hand: submissions with legal or financial effect, or that change or delete data, can be reversed, are checked for errors, or are confirmed before they are final.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Absendungen mit rechtlicher oder finanzieller Wirkung oder die Daten ändern oder löschen, lassen sich rückgängig machen, werden auf Fehler geprüft oder vor dem Abschluss bestätigt.",
    },
    Point {
        id: "manual/pointer-cancellation",
        wcag: &["2.5.2"],
        severity: Severity::Medium,
        applies: |a| a.pointer_events,
        en: "Check by hand: a single-pointer action does not fire on the down event alone — it can be aborted or undone, or fires on release.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Eine Aktion mit einem einzelnen Zeiger löst nicht schon beim Herunterdrücken aus — sie lässt sich abbrechen oder rückgängig machen, oder sie löst beim Loslassen aus.",
    },
    Point {
        id: "manual/consistent-navigation",
        wcag: &["3.2.3"],
        severity: Severity::Medium,
        applies: |a| a.links,
        en: "Check by hand: navigation that repeats across the pages of the set keeps the same relative order.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Navigation, die sich auf den Seiten des Angebots wiederholt, behält dieselbe relative Reihenfolge.",
    },
    Point {
        id: "manual/consistent-identification",
        wcag: &["3.2.4"],
        severity: Severity::Medium,
        applies: |a| a.links || a.fields,
        en: "Check by hand: components with the same function carry the same name and the same icon on every page of the set.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Komponenten mit gleicher Funktion tragen auf jeder Seite des Angebots denselben Namen und dasselbe Symbol.",
    },
    Point {
        id: "manual/sensory-characteristics",
        wcag: &["1.3.3"],
        severity: Severity::Medium,
        applies: |a| a.text,
        en: "Check by hand: instructions do not rely on shape, size, position or sound alone — not just \"the round button\" or \"the box on the right\".",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Anweisungen verlassen sich nicht allein auf Form, Größe, Lage oder Klang — nicht nur „der runde Knopf“ oder „das Feld rechts“.",
    },
    Point {
        id: "manual/multiple-ways",
        wcag: &["2.4.5"],
        severity: Severity::Medium,
        applies: |a| a.links,
        en: "Check by hand: a page of the set can be reached in more than one way — for example navigation, search, site map or related links.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Eine Seite des Angebots ist auf mehr als einem Weg erreichbar — etwa über Navigation, Suche, Seitenübersicht oder verwandte Links.",
    },
    Point {
        id: "manual/text-spacing",
        wcag: &["1.4.12"],
        severity: Severity::Medium,
        applies: |a| a.text,
        en: "Check by hand: with line height 1.5, paragraph spacing 2, letter spacing 0.12 and word spacing 0.16 times the font size applied, no text is cut off, overlapped or lost.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Mit Zeilenhöhe 1,5, Absatzabstand 2, Zeichenabstand 0,12 und Wortabstand 0,16 der Schriftgröße geht kein Text verloren, wird abgeschnitten oder überlagert.",
    },
    Point {
        id: "manual/content-on-hover-focus",
        wcag: &["1.4.13"],
        severity: Severity::Medium,
        applies: |a| a.hover_content,
        en: "Check by hand: content that appears on hover or focus can be dismissed without moving the pointer or focus, can be hovered itself, and stays until dismissed or no longer valid.",
        #[cfg(feature = "de")]
        de: "Von Hand prüfen: Inhalt, der bei Hover oder Fokus erscheint, lässt sich schließen, ohne Zeiger oder Fokus zu bewegen, lässt sich selbst überfahren und bleibt, bis er geschlossen wird oder nicht mehr gilt.",
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
        "manual/dragging-movements",
        "manual/consistent-help",
        "manual/keyboard-trap",
        "manual/character-shortcuts",
        "manual/error-prevention",
        "manual/pointer-cancellation",
        "manual/consistent-navigation",
        "manual/consistent-identification",
        "manual/sensory-characteristics",
        "manual/multiple-ways",
        "manual/text-spacing",
        "manual/content-on-hover-focus",
    ],
    tier: Tier::Structure,
    // Was dargestellt wird, auch unter `aria-hidden`: Untertitel (1.2.2),
    // Farbe (1.4.1) und Zeitbegrenzung (2.2.1) betreffen auch Inhalt, den die
    // Assistenztechnik nicht sieht — ein sichtbares Video unter
    // `aria-hidden="true"` braucht trotzdem Untertitel.
    scope: Scope::Rendered,
    wcag: &[
        "1.1.1", "1.2.1", "1.2.2", "1.2.3", "1.2.5", "1.3.1", "1.3.3", "1.4.1", "1.4.12", "1.4.13",
        "2.1.2", "2.1.4", "2.2.1", "2.2.3", "2.2.4", "2.2.5", "2.4.5", "2.5.1", "2.5.2", "2.5.7",
        "3.1.3", "3.2.3", "3.2.4", "3.2.6", "3.3.1", "3.3.3", "3.3.4", "3.3.5", "3.3.8",
    ],
    severity: Severity::Medium,
    help: "Criteria no machine can decide. Each appears once per page when the page contains \
           something it applies to, as UNTESTED — a checklist, not a verdict.",
    help_de: de_text!(
        "Kriterien, die keine Maschine entscheiden kann. Jedes erscheint einmal je Seite, \
              wenn die Seite etwas enthält, für das es gilt, als UNTESTED — eine Checkliste, \
              kein Urteil."
    ),
};

pub(crate) fn checklist<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let trigger = Trigger::survey(doc);
    let root = Location::node(doc.root().id().to_string());
    for p in POINTS.iter().filter(|p| (p.applies)(&trigger)) {
        out.push(
            Finding::untested(p.id, pick!(locale, p.en, p.de))
                .with_severity(p.severity)
                .with_wcag(p.wcag.iter().copied())
                .at(root.clone()),
        );
    }
}

/// Bewegungssteuerung (2.5.4) steht nicht im Accessibility-Tree, sondern im
/// Skript: Das einzige Zeichen im Dokument ist ein Inline-`<script>`, der
/// `devicemotion` oder `deviceorientation` abonniert. Deshalb ein eigener
/// Punkt mit [`Scope::Markup`] — `<script>` ist nie dargestellt. Was ein
/// externes Skript abonniert, sieht keine Prüfung am Dokument.
pub(crate) const META_MOTION: Meta = Meta {
    ids: &["manual/motion-actuation"],
    tier: Tier::Structure,
    scope: Scope::Markup,
    wcag: &["2.5.4"],
    severity: Severity::Medium,
    help: "Checklist item for motion actuation, shown when an inline script listens to \
           device motion or orientation.",
    help_de: de_text!(
        "Checklistenpunkt für Bewegungssteuerung, erscheint, wenn ein Inline-Skript \
              auf Gerätebewegung oder -ausrichtung hört."
    ),
};

pub(crate) fn motion<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let listens_for_motion = elements(doc).any(|n| {
        n.is_element("script")
            && n.children().any(|k| {
                k.kind() == NodeKind::Text
                    && (k.text().contains("devicemotion") || k.text().contains("deviceorientation"))
            })
    });
    if listens_for_motion {
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
    fn every_point_is_declared() {
        let ids: Vec<&str> = POINTS.iter().map(|p| p.id).collect();
        assert_eq!(ids, META.ids);
    }
}
