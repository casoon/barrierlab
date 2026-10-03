//! Dokument-, Sprach- und Zeitregeln nach WCAG 2.2 (2.2.1, 3.1.1, 3.1.2,
//! 3.1.4) und dem HTML-Standard (`lang`/`xml:lang`, „shared declarative
//! refresh steps").
//!
//! Portiert aus auditmysite (`language_extended`, `language_of_parts`,
//! `abbreviations`, `timing_adjustable`; casoon/barrierlab#20). auditmysite
//! ist der Vergleichspunkt, nicht die Norm — Abweichungen stehen an der
//! jeweiligen Stelle und im CHANGELOG.
//!
//! Alles hier liest Tags, Attribute und Text: Tier 1.

use a11y_dom::{Document, Node, NodeId, NodeKind, ancestors, descendants, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Die Primärkennung eines Sprach-Tags, klein geschrieben: `de` aus `de-AT`.
fn primaer(tag: &str) -> String {
    tag.trim()
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

// --- document/lang-mismatch -------------------------------------------------

/// 3.1.1: `lang` und `xml:lang` am `<html>`-Element nennen verschiedene
/// Sprachen. Welche gilt, hängt dann vom Werkzeug ab.
///
/// Verglichen wird wie in auditmysite die Primärkennung; HTML verlangt sogar
/// denselben Wert. Ob `lang` selbst gültig ist, prüft `document/lang-invalid`.
/// Beleg: auditmysite-Korpus `misc_content_checks`
/// (`<html lang="en" xml:lang="de">`); bing.com trägt beide gleich.
pub(crate) fn lang_mismatch<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let root = doc.root();
    let (Some(lang), Some(xml)) = (root.attr("lang"), root.attr("xml:lang")) else {
        return;
    };
    if lang.trim().is_empty() || xml.trim().is_empty() || primaer(lang) == primaer(xml) {
        return;
    }
    out.push(
        Finding::fail(
            "document/lang-mismatch",
            tr!(
                locale,
                "lang=\"{lang}\" and xml:lang=\"{xml}\" name different languages.",
                "lang=\"{lang}\" und xml:lang=\"{xml}\" nennen verschiedene Sprachen."
            ),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["3.1.1"])
        .at(at(root.id())),
    );
}

// --- language/part-unmarked -------------------------------------------------

/// Häufige Funktionswörter je Sprache, aus auditmysite übernommen. Nur
/// Deutsch und Englisch: Für andere Sprachen gibt es keine Liste.
const MARKER_DE: &[&str] = &[
    "der", "die", "das", "und", "ist", "sind", "mit", "für", "von", "auf", "eine", "einer",
    "nicht", "werden", "wird", "auch", "dass",
];
const MARKER_EN: &[&str] = &[
    "the", "and", "is", "are", "with", "for", "from", "this", "that", "not", "will", "also",
    "have", "has", "your", "our",
];

const TEXTBLOECKE: &[&str] = &["p", "li", "dd", "td", "figcaption"];

/// Inhalt, dessen Sprache nicht zählt: Code, Adressen, Zitate und was
/// ausdrücklich nicht übersetzt werden soll.
fn ausgenommen<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n).chain(ancestors(n)).any(|a| {
        matches!(a.local_name(), "code" | "pre" | "address" | "blockquote")
            || a.attr("translate") == Some("no")
    })
}

/// Der Text eines Teilbaums ohne `<script>`, `<style>` und `<template>`. Ein
/// eingebettetes `<style>` ist kein Lesetext — sparkasse.de setzt es in
/// Listeneinträge, und sein CSS las sich sonst als Englisch.
fn lesetext<'a, N: Node<'a>>(n: N, out: &mut String) {
    for k in n.children() {
        match k.kind() {
            NodeKind::Text => {
                out.push_str(k.text());
                out.push(' ');
            }
            NodeKind::Element
                if !matches!(k.local_name(), "script" | "style" | "template")
                    && !k.attr("lang").is_some_and(|lang| !lang.trim().is_empty()) =>
            {
                lesetext(k, out);
            }
            _ => {}
        }
    }
}

/// Die erkannte Sprache eines Textes: mindestens fünf Funktionswörter der
/// einen und höchstens eines der anderen.
fn erkannt(woerter: &[String]) -> Option<&'static str> {
    let de = woerter
        .iter()
        .filter(|w| MARKER_DE.contains(&w.as_str()))
        .count();
    let en = woerter
        .iter()
        .filter(|w| MARKER_EN.contains(&w.as_str()))
        .count();
    if de >= 5 && en <= 1 {
        Some("de")
    } else if en >= 5 && de <= 1 {
        Some("en")
    } else {
        None
    }
}

/// 3.1.2: Ein langer Absatz, der deutlich nach der anderen Sprache klingt,
/// erbt die Seitensprache. Ein Screenreader liest ihn dann mit der falschen
/// Aussprache.
///
/// Heuristik wie in auditmysite, nur für deutsche und englische Seiten: ab
/// 100 Zeichen und 16 Wörtern, mit eindeutigem Überhang an Funktionswörtern
/// der anderen Sprache; Code, Adressen, Zitate und `translate="no"`
/// ausgenommen. Gemeldet wird der innerste Textblock, nicht zusätzlich der
/// umgebende (`<li><p>`). Deshalb und weil Namen und Fachbegriffe täuschen
/// `REVIEW`. Auf Seiten in einer anderen Sprache kann die Regel nichts sagen
/// und meldet `language/part-undetermined` — nicht geprüft ist nicht bestanden.
/// Beleg: auditmysite-Korpus `text_and_layout` (deutscher Absatz auf einer
/// englischen Seite).
pub(crate) fn part_unmarked<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let root = doc.root();
    let Some(seite) = root.attr("lang").map(primaer).filter(|l| !l.is_empty()) else {
        // Ohne Seitensprache meldet `document/lang-missing`.
        return;
    };
    if !matches!(seite.as_str(), "de" | "en") {
        out.push(
            Finding::untested(
                "language/part-undetermined",
                tr!(
                    locale,
                    "Passages in another language are only recognised on German and English \
                     pages; this page is \"{seite}\". Check by hand that foreign-language \
                     passages carry a lang attribute.",
                    "Anderssprachige Passagen werden nur auf deutschen und englischen Seiten \
                     erkannt; diese Seite ist \"{seite}\". Von Hand prüfen, ob anderssprachige \
                     Passagen ein lang-Attribut tragen."
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.1.2"])
            .at(at(root.id())),
        );
        return;
    }
    for n in elements(doc) {
        if !TEXTBLOECKE.contains(&n.local_name())
            || descendants(n).any(|d| TEXTBLOECKE.contains(&d.local_name()))
            || ausgenommen(n)
        {
            continue;
        }
        let mut text = String::new();
        lesetext(n, &mut text);
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let woerter: Vec<String> = text
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect();
        if text.chars().count() < 100 || woerter.len() < 16 {
            continue;
        }
        let erklaert = std::iter::once(n)
            .chain(ancestors(n))
            .find_map(|a| a.attr("lang"))
            .map(primaer);
        if erklaert.is_some_and(|l| l != seite) {
            continue;
        }
        let Some(sprache) = erkannt(&woerter).filter(|s| *s != seite) else {
            continue;
        };
        out.push(
            Finding::review(
                "language/part-unmarked",
                tr!(
                    locale,
                    "This passage reads like \"{sprache}\" but inherits the page language \
                     \"{seite}\". If it is in another language, mark it with lang=\"{sprache}\".",
                    "Die Passage liest sich wie \"{sprache}\", erbt aber die Seitensprache \
                     \"{seite}\". Ist sie anderssprachig, mit lang=\"{sprache}\" auszeichnen."
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- language/abbreviation-unexpanded --------------------------------------

/// 3.1.4 (AAA): `<abbr>` oder `<acronym>` ohne `title`.
///
/// `REVIEW` statt auditmysites Verstoß: 3.1.4 lässt sich auch mit der
/// Ausschreibung im Text oder einem Glossar erfüllen (G97, G55), und
/// `<abbr>` ohne `title` ist gültiges HTML. Beleg: auditmysite-Korpus
/// `misc_content_checks` (`<abbr>W3C</abbr>`); w3.org/WAI und gov.ie tragen
/// `title`.
pub(crate) fn abbreviations<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !matches!(n.local_name(), "abbr" | "acronym")
            || n.attr("title").is_some_and(|t| !t.trim().is_empty())
        {
            continue;
        }
        out.push(
            Finding::review(
                "language/abbreviation-unexpanded",
                pick!(
                    locale,
                    "The abbreviation has no title. Check that its expansion is available, in \
                     the title, the text or a glossary.",
                    "Die Abkürzung hat kein title. Prüfen, ob die Langform verfügbar ist — im \
                     title, im Text oder in einem Glossar.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["3.1.4"])
            .at(at(n.id())),
        );
    }
}

// --- timing/meta-refresh ----------------------------------------------------

/// Die Wartezeit einer Refresh-Anweisung in Sekunden, nach HTML („shared
/// declarative refresh steps"): die führenden Ziffern. Ohne Ziffern ignoriert
/// der Browser die Anweisung.
fn wartezeit(content: &str) -> Option<u64> {
    let s = content.trim_start();
    let ziffern: String = s.chars().take_while(char::is_ascii_digit).collect();
    if ziffern.is_empty() {
        return None;
    }
    Some(ziffern.parse().unwrap_or(u64::MAX))
}

/// Ab 20 Stunden gilt eine Frist als ausreichend (2.2.1, „20 Hour
/// Exception").
const ZWANZIG_STUNDEN: u64 = 20 * 60 * 60;

/// 2.2.1: `<meta http-equiv="refresh">` lädt die Seite nach einer Frist neu
/// oder leitet weiter; anhalten oder verlängern lässt sich das nicht.
///
/// Anders als auditmysite nicht bei `0`: Eine sofortige Weiterleitung ist
/// keine Frist (H76, axe `meta-refresh`). web.de und gmx.net leiten so ohne
/// JavaScript weiter (`content="0;url=/?status=no-script"`). Eine Anweisung
/// ohne Ziffern führt der Browser nicht aus. Beleg: auditmysite-Korpus
/// `meta_refresh_present` (`content="120"`).
pub(crate) fn meta_refresh<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("meta")
            || !n
                .attr("http-equiv")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("refresh"))
        {
            continue;
        }
        let Some(s) = n.attr("content").and_then(wartezeit) else {
            continue;
        };
        if s == 0 || s >= ZWANZIG_STUNDEN {
            continue;
        }
        out.push(
            Finding::fail(
                "timing/meta-refresh",
                tr!(
                    locale,
                    "The page reloads or redirects itself after {s} s via <meta \
                     http-equiv=\"refresh\">; users cannot stop or extend that.",
                    "Die Seite lädt sich per <meta http-equiv=\"refresh\"> nach {s} s neu oder \
                     leitet weiter; anhalten oder verlängern lässt sich das nicht."
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.2.1"])
            .at(at(n.id())),
        );
    }
}

// --- headings/section-without-heading -------------------------------------

fn ist_ueberschrift<'a, N: Node<'a>>(n: N) -> bool {
    matches!(n.local_name(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        || crate::structure::explizite_rolle(n) == Some("heading")
}

/// Ein Abschnitt, der eine Überschrift erwarten lässt: `<article>` und ein
/// benanntes `<section>` (eine `region`). Navigationen nicht — sie werden über
/// ihren Namen gefunden, eine Überschrift erwartet dort niemand.
fn ist_abschnitt<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("article")
        || (n.is_element("section")
            && (n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
                || n.has_attr("aria-labelledby")))
}

/// 2.4.10 (AAA): Artikel oder benannte Abschnitte ohne eigene Überschrift.
///
/// Ein Hinweis je Seite wie in auditmysite, aber genauer gezählt: auditmysite
/// stellt alle Abschnitte samt Navigationen allen Überschriften der Seite
/// gegenüber, hier zählt jeder Abschnitt ohne Überschrift darin. `REVIEW`,
/// weil der Inhalt entscheidet, ob ein Abschnitt eine Überschrift braucht —
/// Teaser-Karten als `<article>` sind auf Nachrichtenseiten die Regel
/// (t-online.de 61, faz.net 39, bild.de 25). Die Lücken in der Gliederung
/// meldet `headings/skip-level`.
pub(crate) fn section_without_heading<D: Document>(
    doc: &D,
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let ohne = elements(doc)
        .filter(|n| ist_abschnitt(*n) && !descendants(*n).any(ist_ueberschrift))
        .count();
    if ohne == 0 {
        return;
    }
    out.push(
        Finding::review(
            "headings/section-without-heading",
            tr!(
                locale,
                "Articles or named sections without a heading: {ohne}. Check whether headings \
                 would help users find their content.",
                "Artikel oder benannte Abschnitte ohne Überschrift: {ohne}. Prüfen, ob \
                 Überschriften beim Auffinden des Inhalts helfen würden."
            ),
        )
        .with_severity(Severity::Low)
        .with_wcag(["2.4.10"])
        .at(at(doc.root().id())),
    );
}
