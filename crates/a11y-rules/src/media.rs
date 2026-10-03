//! Bild- und Medienregeln nach WCAG 2.2 (1.1.1, 1.4.2, 2.4.1, 4.1.2) und dem
//! HTML-Standard (`<area>`, `<input type="image">`, `<object>`, `ismap`,
//! `<iframe>`).
//!
//! Portiert aus auditmysite (`image_input_rules`, `server_side_image_map`,
//! `background_audio` und der Rahmenteil von `media_rules`;
//! casoon/barrierlab#19). auditmysite ist der Vergleichspunkt, nicht die Norm
//! — Abweichungen stehen an der jeweiligen Stelle und im CHANGELOG.
//!
//! Alles außer `frames/name-missing` liest nur Attribute und Textinhalt und
//! läuft auf Tier 1. Der Rahmenname kann über `aria-labelledby` kommen; dafür
//! braucht es die Namensberechnung, also Tier 2.

use a11y_dom::{Document, Node, NodeId, NodeKind, Semantics, descendants, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick};
use crate::semantics::named;
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn gefuellt<'a, N: Node<'a>>(n: N, attr: &str) -> bool {
    n.attr(attr).is_some_and(|v| !v.trim().is_empty())
}

/// `aria-label` oder `aria-labelledby` — beide gehen in der Namensberechnung
/// dem `alt` vor. auditmysite zählt nur `aria-label` (und an `<area>` gar
/// nichts außer `alt`); ein Element, das per `aria-labelledby` benannt ist,
/// meldet es zu Unrecht.
fn aria_benannt<'a, N: Node<'a>>(n: N) -> bool {
    gefuellt(n, "aria-label") || gefuellt(n, "aria-labelledby")
}

// --- images/area-alt-missing ------------------------------------------------

/// 1.1.1: Ein aktives `<area>` (mit `href`) ohne Textalternative. Es ist ein
/// Link; ohne `alt` hat er keinen Namen.
///
/// Beleg: auditmysite-Korpus `misc_content_checks`.
pub(crate) fn area_alt<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if n.is_element("area") && n.has_attr("href") && !gefuellt(n, "alt") && !aria_benannt(n) {
            out.push(
                Finding::fail(
                    "images/area-alt-missing",
                    pick!(
                        locale,
                        "This image map area is a link but has no alt text.",
                        "Der Bereich der Imagemap ist ein Link, hat aber keinen Alt-Text.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- images/input-alt-missing -----------------------------------------------

/// 1.1.1: Ein Grafik-Button `<input type="image">` ohne `alt`. Die
/// Namensberechnung fällt dann auf einen Ersatztext des Browsers zurück
/// („Submit Query"), der nichts über die Aktion sagt — deshalb meldet
/// `buttons/name-missing` ihn nicht.
///
/// Beleg: auditmysite-Korpus `misc_content_checks`.
pub(crate) fn input_alt<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let grafik_button = n.is_element("input")
            && n.attr("type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("image"));
        if grafik_button && !gefuellt(n, "alt") && !aria_benannt(n) {
            out.push(
                Finding::fail(
                    "images/input-alt-missing",
                    pick!(
                        locale,
                        "This image button has no alt text describing its action.",
                        "Der Grafik-Button hat keinen Alt-Text, der seine Aktion beschreibt.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- images/server-side-map -------------------------------------------------

/// 1.1.1: Eine serverseitige Imagemap (`<img ismap>`). Die Ziele stecken in
/// Koordinaten, die erst der Server auswertet; per Tastatur und für
/// Screenreader sind sie nicht erreichbar.
///
/// Beleg: auditmysite-Korpus `forms_and_misc`.
pub(crate) fn server_side_map<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if n.is_element("img") && n.has_attr("ismap") {
            out.push(
                Finding::fail(
                    "images/server-side-map",
                    pick!(
                        locale,
                        "This image is a server-side image map (ismap); its targets cannot be \
                         reached by keyboard or screen reader. Use a client-side map or text \
                         links.",
                        "Das Bild ist eine serverseitige Imagemap (ismap); ihre Ziele sind per \
                         Tastatur und Screenreader nicht erreichbar. Eine clientseitige Imagemap \
                         oder Textlinks gehören hierher.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- objects/alt-missing ----------------------------------------------------

/// 1.1.1: Ein `<object>` ohne Textalternative — kein Ersatzinhalt, kein
/// `aria-label`/`aria-labelledby`, kein `title`.
///
/// Ersatzinhalt ist wie in auditmysite der Text im Element. `<embed>` prüft
/// auditmysite mit, hier nicht: Weder Korpus noch die 48 echten Seiten
/// enthalten eines.
///
/// Beleg: auditmysite-Korpus `object_no_alt` (ein PDF ohne Ersatzinhalt).
pub(crate) fn object_alt<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("object") {
            continue;
        }
        let ersatztext =
            descendants(n).any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty());
        if !ersatztext && !aria_benannt(n) && !gefuellt(n, "title") {
            out.push(
                Finding::fail(
                    "objects/alt-missing",
                    pick!(
                        locale,
                        "This <object> has no text alternative: no fallback content, \
                         aria-label or title.",
                        "Das <object> hat keine Textalternative: keinen Ersatzinhalt, kein \
                         aria-label und keinen title.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- media/audio-autoplay ---------------------------------------------------

/// 1.4.2: Ein `<audio autoplay>` ohne `muted` spielt beim Laden los.
///
/// `REVIEW` statt auditmysites Verstoß: Ob der Ton länger als drei Sekunden
/// läuft und ob die Seite ihn anhalten lässt, steht nicht im Markup. Als
/// Kriterium 1.4.2 (Audio Control) statt auditmysites 1.4.7 (Hintergrundton
/// unter Sprache, AAA): Selbststartender Ton ist der Gegenstand von 1.4.2.
///
/// Beleg: auditmysite-Korpus `media_and_visual`.
pub(crate) fn audio_autoplay<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if n.is_element("audio") && n.has_attr("autoplay") && !n.has_attr("muted") {
            out.push(
                Finding::review(
                    "media/audio-autoplay",
                    pick!(
                        locale,
                        "This audio starts playing on its own. Check that it stops within three \
                         seconds or that it can be paused or turned down right away.",
                        "Der Ton startet von selbst. Prüfen, ob er nach spätestens drei Sekunden \
                         endet oder sich sofort anhalten oder leiser stellen lässt.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- frames/name-missing ----------------------------------------------------

/// Ob ein Rahmen nur Technik transportiert und nie zu sehen ist: Breite und
/// Höhe als Attribut 0 oder 1 (wie in auditmysite) oder ein Inline-Stil
/// `display: none`. Ohne berechnete Stile ist das der einzige Hinweis darauf;
/// craigslist.org lädt so einen Speicherrahmen, duckduckgo.com einen leeren.
fn unsichtbarer_rahmen<'a, N: Node<'a>>(n: N) -> bool {
    let winzig = |a: &str| n.attr(a).is_some_and(|v| matches!(v.trim(), "0" | "1"));
    let ausgeblendet = n.attr("style").is_some_and(|s| {
        s.split(';').any(|d| {
            let mut teile = d.splitn(2, ':');
            let name = teile.next().unwrap_or("").trim();
            let wert = teile.next().unwrap_or("").trim();
            name.eq_ignore_ascii_case("display")
                && wert
                    .trim_end_matches("!important")
                    .trim()
                    .eq_ignore_ascii_case("none")
        })
    });
    (winzig("width") && winzig("height")) || ausgeblendet
}

/// 2.4.1, 4.1.2: Ein `<iframe>` ohne Namen. Screenreader nennen beim Betreten
/// des Rahmens dessen Namen; ohne ihn bleibt offen, was darin steckt.
///
/// Ausgenommen sind Rahmen mit `role="none"`/`"presentation"` und solche, die
/// nie zu sehen sind ([`unsichtbarer_rahmen`]). Der Teil von auditmysites
/// `media_rules`, der fremde Rahmen als ungeprüft meldet (`frame-tested`),
/// bleibt im Host — ob ein Rahmen fremd ist, weiß nur der Browser.
///
/// Belege: auditmysite-Korpus `frame_missing_title`; die Sportdaten-Rahmen
/// auf spiegel.de (ohne `title`) und n-tv.de (`title=""`).
pub(crate) fn frame_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("iframe")
            || doc.is_ignored(n)
            || explizite_rolle(n).is_some_and(|r| matches!(r, "none" | "presentation"))
            || unsichtbarer_rahmen(n)
            || named(doc, n)
        {
            continue;
        }
        out.push(
            Finding::fail(
                "frames/name-missing",
                pick!(
                    locale,
                    "The iframe has no accessible name; give it a title that says what it \
                     contains.",
                    "Der Rahmen hat keinen zugänglichen Namen; ein title, der sagt, was er \
                     enthält, gehört hierher.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.1", "4.1.2"])
            .at(at(n.id())),
        );
    }
}
