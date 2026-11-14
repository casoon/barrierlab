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

use std::collections::HashMap;

use a11y_dom::{
    Document, Node, NodeId, NodeKind, ScopedId, Semantics, ancestors, descendants, elements,
};
use a11y_report::{Finding, Location, Severity};

use crate::forms::id_index;
use crate::landmarks::tabindex;
use crate::locale::{Locale, pick};
use crate::semantics::named;
use crate::structure::{Naming, labelledby, presentational};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn filled<'a, N: Node<'a>>(n: N, attr: &str) -> bool {
    n.attr(attr).is_some_and(|v| !v.trim().is_empty())
}

/// `aria-label` oder `aria-labelledby` — beide gehen in der Namensberechnung
/// dem `alt` vor. auditmysite zählt nur `aria-label` (und an `<area>` gar
/// nichts außer `alt`); ein Element, das per `aria-labelledby` benannt ist,
/// meldet es zu Unrecht.
///
/// `aria-labelledby` zählt nur, wenn ein Ziel Text trägt: Ein Verweis auf ein
/// leeres Element stiftet keinen Namen (accname 1.2, 2B; ACT 8fc3b6). Ein
/// Ziel, das in der Sicht fehlt, kann versteckt oder nicht vorhanden sein
/// ([`Naming::Unknown`]).
fn aria_naming<'a, N: Node<'a>>(n: N, ids: &HashMap<ScopedId<'a>, N>) -> Naming {
    if filled(n, "aria-label") {
        Naming::Named
    } else {
        labelledby(n, ids)
    }
}

/// Der Befund zu einem Element ohne eigene Textalternative: `FAIL`, wenn auch
/// `aria-labelledby` nichts stiftet, `REVIEW`, wenn sein Ziel versteckt sein
/// könnte.
fn unnamed(
    naming: Naming,
    id: &str,
    fail: &'static str,
    unknown: &'static str,
    at_node: NodeId,
) -> Option<Finding> {
    let f = match naming {
        Naming::Named => return None,
        Naming::Empty => Finding::fail(id, fail),
        Naming::Unknown => Finding::review(id, unknown),
    };
    Some(
        f.with_severity(Severity::High)
            .with_wcag(["1.1.1"])
            .at(at(at_node)),
    )
}

// --- images/area-alt-missing ------------------------------------------------

/// 1.1.1: Ein aktives `<area>` (mit `href`) ohne Textalternative. Es ist ein
/// Link; ohne `alt` hat er keinen Namen.
///
/// Beleg: auditmysite-Korpus `misc_content_checks`.
pub(crate) fn area_alt<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    for n in elements(doc) {
        // Ein Ziel außerhalb der Sicht zählt hier als Name, wie bisher.
        if n.is_element("area")
            && n.has_attr("href")
            && !filled(n, "alt")
            && aria_naming(n, &ids) == Naming::Empty
        {
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
    let ids = id_index(doc);
    for n in elements(doc) {
        let graphic_button = n.is_element("input")
            && n.attr("type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("image"));
        // HTML-AAM (`input type="image"`): Nach `alt` kommt `title`, erst
        // dann der Ersatztext des Browsers (ACT 59796f).
        if !graphic_button || filled(n, "alt") || filled(n, "title") {
            continue;
        }
        out.extend(unnamed(
            aria_naming(n, &ids),
            "images/input-alt-missing",
            pick!(
                locale,
                "This image button has no alt text describing its action.",
                "Der Grafik-Button hat keinen Alt-Text, der seine Aktion beschreibt.",
            ),
            pick!(
                locale,
                "This image button has no alt text; its aria-labelledby points to an element \
                 that is hidden or does not exist.",
                "Der Grafik-Button hat keinen Alt-Text; sein aria-labelledby zeigt auf ein \
                 verstecktes oder nicht vorhandenes Element.",
            ),
            n.id(),
        ));
    }
}

// --- images/server-side-map -------------------------------------------------

/// 2.1.1: Eine serverseitige Imagemap (`<img ismap>`). Die Ziele stecken in
/// Koordinaten, die erst der Server auswertet; per Tastatur und für
/// Screenreader sind sie nicht erreichbar. `REVIEW`: Gleichwertige Textlinks
/// daneben erfüllen 2.1.1 (auditmysite#779).
///
/// Beleg: auditmysite-Korpus `forms_and_misc`.
pub(crate) fn server_side_map<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        // `ismap` wirkt nur an einem Nachfahren eines `a` mit `href` (HTML,
        // „The img element"); sonst gibt es keine serverseitige Imagemap.
        let in_link = ancestors(n).any(|a| a.is_element("a") && a.has_attr("href"));
        if n.is_element("img") && n.has_attr("ismap") && in_link {
            out.push(
                Finding::review(
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
                .with_wcag(["2.1.1"])
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
    let ids = id_index(doc);
    for n in elements(doc) {
        // Ausdrücklich dekorativ mit wirksamer `role="none"`: nicht im
        // Accessibility-Tree, kein Name nötig (ACT 8fc3b6).
        if !n.is_element("object") || presentational(n) {
            continue;
        }
        let replacement_text =
            descendants(n).any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty());
        if replacement_text || filled(n, "title") {
            continue;
        }
        out.extend(unnamed(
            aria_naming(n, &ids),
            "objects/alt-missing",
            pick!(
                locale,
                "This <object> has no text alternative: no fallback content, aria-label or title.",
                "Das <object> hat keine Textalternative: keinen Ersatzinhalt, kein aria-label \
                 und keinen title.",
            ),
            pick!(
                locale,
                "This <object> has no fallback content, aria-label or title; its \
                 aria-labelledby points to an element that is hidden or does not exist.",
                "Das <object> hat keinen Ersatzinhalt, kein aria-label und keinen title; sein \
                 aria-labelledby zeigt auf ein verstecktes oder nicht vorhandenes Element.",
            ),
            n.id(),
        ));
    }
}

// --- media/audio-autoplay ---------------------------------------------------

/// Eine Zeitangabe eines Medienfragments (Media Fragments URI 1.0, 4.2.1,
/// NPT): Sekunden, `mm:ss` oder `hh:mm:ss`, mit Nachkommastellen.
fn npt_seconds(v: &str) -> Option<f64> {
    v.split(':').try_fold(0.0, |acc, part| {
        Some(acc * 60.0 + part.parse::<f64>().ok()?)
    })
}

/// Die Länge, die ein Medienfragment `#t=start,end` abspielt; `None` ohne
/// Ende (dann läuft das Medium bis zum Schluss).
fn fragment_length(src: &str) -> Option<f64> {
    let (_, fragment) = src.split_once('#')?;
    let t = fragment.split('&').find_map(|p| p.strip_prefix("t="))?;
    let t = t.strip_prefix("npt:").unwrap_or(t);
    let (start, end) = t.split_once(',')?;
    let start = if start.is_empty() {
        0.0
    } else {
        npt_seconds(start)?
    };
    Some(npt_seconds(end)? - start)
}

/// Ob jede Quelle des Mediums ein Fragment von höchstens drei Sekunden
/// abspielt — dann endet der Ton rechtzeitig (1.4.2, ACT 80f0bf, Passed
/// Example 2).
fn plays_at_most_three_seconds<'a, N: Node<'a>>(n: N) -> bool {
    let sources: Vec<&str> = n
        .attr("src")
        .into_iter()
        .chain(
            n.children()
                .filter(|k| k.is_element("source"))
                .filter_map(|k| k.attr("src")),
        )
        .collect();
    !sources.is_empty()
        && sources
            .iter()
            .all(|s| fragment_length(s).is_some_and(|l| l <= 3.0))
}

/// Ob das Medium eigene Bedienelemente zeigt, die jeder erreicht: `controls`,
/// ohne `hidden` und ohne `aria-hidden="true"` am Element oder einem
/// Vorfahren. Versteckte Bedienelemente sind kein Mechanismus (ACT 4c31df:
/// sichtbar und zugänglich).
fn usable_controls<'a, N: Node<'a>>(n: N) -> bool {
    n.has_attr("controls")
        && !std::iter::once(n)
            .chain(ancestors(n))
            .any(|a| a.has_attr("hidden") || a.attr("aria-hidden") == Some("true"))
}

/// 1.4.2: Ein `<audio autoplay>` oder `<video autoplay>` ohne `muted` spielt
/// beim Laden los.
///
/// `REVIEW` statt auditmysites Verstoß: Ob der Ton länger als drei Sekunden
/// läuft, ob ein Video überhaupt eine Tonspur hat und ob die Seite ihn
/// anhalten lässt, steht nicht im Markup. Als Kriterium 1.4.2 (Audio Control)
/// statt auditmysites 1.4.7 (Hintergrundton unter Sprache, AAA):
/// Selbststartender Ton ist der Gegenstand von 1.4.2.
///
/// Kein Befund, wo das Markup die Frage beantwortet: `controls` gibt den
/// Mechanismus zum Anhalten und Stummschalten (ACT 80f0bf, Passed Example 1;
/// 4c31df), und ein Medienfragment von höchstens drei Sekunden endet
/// rechtzeitig.
///
/// Beleg: auditmysite-Korpus `media_and_visual`.
pub(crate) fn audio_autoplay<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let media = n.is_element("audio") || n.is_element("video");
        if media
            && n.has_attr("autoplay")
            && !n.has_attr("muted")
            && !usable_controls(n)
            && !plays_at_most_three_seconds(n)
        {
            out.push(
                Finding::review(
                    "media/audio-autoplay",
                    pick!(
                        locale,
                        "This audio or video starts playing on its own. Check that any sound \
                         stops within three seconds or that it can be paused or turned down \
                         right away.",
                        "Ton oder Video startet von selbst. Prüfen, ob der Ton nach spätestens \
                         drei Sekunden endet oder sich sofort anhalten oder leiser stellen lässt.",
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
fn invisible_frame<'a, N: Node<'a>>(n: N) -> bool {
    let tiny = |a: &str| n.attr(a).is_some_and(|v| matches!(v.trim(), "0" | "1"));
    let hidden = n.attr("style").is_some_and(|s| {
        s.split(';').any(|d| {
            let mut parts = d.splitn(2, ':');
            let name = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            name.eq_ignore_ascii_case("display")
                && value
                    .trim_end_matches("!important")
                    .trim()
                    .eq_ignore_ascii_case("none")
        })
    });
    (tiny("width") && tiny("height")) || hidden
}

/// 4.1.2 (ACT cae760): Ein `<iframe>` ohne Namen. Für 2.4.1 ist der Titel
/// eine von mehreren Techniken (H64), sein Fehlen also kein Verstoß dort. Screenreader nennen beim Betreten
/// des Rahmens dessen Namen; ohne ihn bleibt offen, was darin steckt.
///
/// Ausgenommen sind Rahmen außerhalb der Tab-Reihenfolge (negativer
/// `tabindex`; ACT cae760 gilt nur für per Tab erreichbare Rahmen) und solche,
/// die nie zu sehen sind ([`invisible_frame`]). `role="none"` nimmt einen
/// Rahmen nicht aus: Er ist fokussierbar, also verwirft der Browser die Rolle
/// (WAI-ARIA 1.2, § 9.3; ACT cae760). Der Teil von auditmysites
/// `media_rules`, der fremde Rahmen als ungeprüft meldet (`frame-tested`),
/// bleibt im Host — ob ein Rahmen fremd ist, weiß nur der Browser.
///
/// Belege: auditmysite-Korpus `frame_missing_title`; die Sportdaten-Rahmen
/// auf spiegel.de (ohne `title`) und n-tv.de (`title=""`).
pub(crate) fn frame_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("iframe")
            || doc.is_ignored(n)
            || tabindex(n).is_some_and(|t| t < 0)
            || invisible_frame(n)
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
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}
