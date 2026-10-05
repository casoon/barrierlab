//! Heuristische Tier-3-Regeln: Barrieren, die sich aus dem Layout vermuten,
//! aber nicht belegen lassen.
//!
//! Jede dieser Regeln meldet `REVIEW`, nie `FAIL` — eine umgekehrte
//! Flex-Richtung kann gewollt und harmlos sein, eine Endlos-Animation einen
//! Pause-Knopf haben, den keine Heuristik erkennt. Der Befund sagt, wo ein
//! Mensch hinsehen soll.
//!
//! Anlass: LiveAudit meldete am 29.09.2026 auf barrierlab.eu zu absichtlich
//! eingebauten Barrieren dieser Art nichts (liveaudit#6) — Reihenfolge per
//! `flex-direction`, Endlos-Animation, `min-width` über 320 px, anklickbare
//! `div`s ohne Rolle, von fixierten Leisten verdeckter Fokus, zu kleine Ziele.
//!
//! Die Daten liefert der Host über [`Rendering::layout`] und
//! [`Rendering::bounds`]. Ohne sie melden diese Regeln nichts — bis auf die
//! Fokus-Sichtbarkeit: Die wird nur in einem eigenen Durchgang gemessen, und
//! ohne ihn steht sie als `UNTESTED` auf der Seite.
//!
//! [`Rendering::layout`]: a11y_dom::Rendering::layout
//! [`Rendering::bounds`]: a11y_dom::Rendering::bounds

use std::collections::{HashMap, HashSet};

use a11y_dom::{
    Layout, Node, NodeId, NodeKind, Rect, Rendering, Tier, ancestors, closest, descendants,
    elements, subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::registry::Meta;
use crate::sicht::Scope;
use crate::structure::per_tab_erreichbar;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Ab dieser Breite erzwingt `min-width` waagerechtes Scrollen bei 320 CSS-px
/// — der Breite, auf die WCAG 1.4.10 eine Seite umbrechen lässt.
const REFLOW_PX: f32 = 320.0;

/// Mindestgröße eines Ziels nach WCAG 2.5.8.
const ZIEL_PX: f32 = 24.0;

/// Elemente, die von sich aus bedienbar sind oder deren Klick der Browser
/// weiterreicht. Ein `cursor: pointer` darin ist kein Hinweis auf einen Fake.
fn ist_bedienelement<'a, N: Node<'a>>(n: N) -> bool {
    matches!(
        n.local_name(),
        "a" | "button"
            | "input"
            | "select"
            | "textarea"
            | "summary"
            | "label"
            | "option"
            | "details"
            | "area"
            | "video"
            | "audio"
    ) || n.has_attr("role")
        || n.has_attr("tabindex")
        || n.has_attr("contenteditable")
}

/// Elemente, die nach WCAG 1.4.10 zweidimensional scrollen dürfen.
fn darf_breit_sein<'a, N: Node<'a>>(n: N) -> bool {
    matches!(
        n.local_name(),
        "table" | "pre" | "code" | "video" | "canvas" | "iframe" | "img" | "svg" | "math"
    )
}

fn hat_tab_ziel<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(descendants(n))
        .any(|d| d.kind() == NodeKind::Element && per_tab_erreichbar(d))
}

/// Ob ein Kind etwas trägt, dessen Reihenfolge zählt: ein Bedienelement oder
/// Text. 1.3.2 betrifft die Lesereihenfolge, nicht nur die Tabreihenfolge
/// (auditmysite-Korpus `text_and_layout`, `div.flex-reorder`).
fn hat_inhalt<'a, N: Node<'a>>(n: N) -> bool {
    hat_tab_ziel(n) || !subtree_text(n).trim().is_empty()
}

/// Alle Bedienelemente darin führen zum selben Ziel — die Teaserkarte mit
/// Bild- und Titellink (spiegel.de: 20 Karten mit `flex-row-reverse`). Die
/// Reihenfolge ändert dann nicht, wohin man kommt.
fn ein_ziel<'a, N: Node<'a>>(n: N) -> bool {
    let mut ziele =
        descendants(n).filter(|d| d.kind() == NodeKind::Element && per_tab_erreichbar(*d));
    let Some(erstes) = ziele.next() else {
        return false;
    };
    let href = |e: N| e.is_element("a").then(|| e.attr("href")).flatten();
    let Some(ziel) = href(erstes) else {
        return false;
    };
    let rest: Vec<N> = ziele.collect();
    !rest.is_empty() && rest.into_iter().all(|e| href(e) == Some(ziel))
}

/// Ob irgendein Element dieses Layout-Feld gemessen hat.
fn gemessen<D: Rendering>(doc: &D, feld: impl Fn(&Layout) -> bool) -> bool {
    elements(doc).any(|n| doc.layout(n).is_some_and(|l| feld(&l)))
}

/// Nicht gemessen ist nicht bestanden: ein `UNTESTED` für die Seite, wenn
/// kein Element das Feld trägt, das die Heuristik braucht.
fn ungemessen<D: Rendering>(
    doc: &D,
    id: &str,
    wcag: &[&str],
    feld: &str,
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    out.push(
        Finding::untested(
            id,
            tr!(
                locale,
                "The host does not measure {feld}; this heuristic did not run.",
                "Der Host misst {feld} nicht; diese Heuristik lief nicht."
            ),
        )
        .with_severity(Severity::Low)
        .with_wcag(wcag.iter().copied())
        .at(at(doc.root().id())),
    );
}

/// 1.3.2 / 2.4.3: umgekehrte Flex-Richtung oder `order` über bedienbarem
/// Inhalt — Tabreihenfolge und sichtbare Reihenfolge laufen auseinander.
fn reihenfolge<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    if !gemessen(doc, |l| l.flex_reversed.is_some() || l.order.is_some()) {
        let feld = "flex-direction / order";
        return ungemessen(
            doc,
            "order/visual-mismatch",
            &["1.3.2", "2.4.3"],
            feld,
            locale,
            out,
        );
    }
    for n in elements(doc) {
        let Some(layout) = doc.layout(n) else {
            continue;
        };
        let umgekehrt = layout.flex_reversed == Some(true)
            && n.children()
                .filter(|k| k.kind() == NodeKind::Element && hat_inhalt(*k))
                .nth(1)
                .is_some()
            && !ein_ziel(n);
        let umsortiert = n.children().any(|k| {
            k.kind() == NodeKind::Element
                && doc.layout(k).and_then(|l| l.order).is_some_and(|o| o != 0)
                && hat_inhalt(k)
        }) && !ein_ziel(n);
        if umgekehrt || umsortiert {
            out.push(
                Finding::review(
                    "order/visual-mismatch",
                    pick!(
                        locale,
                        "CSS reorders focusable content (flex-direction: *-reverse or order). \
                         Check that the tab order still follows the visual order.",
                        "CSS ordnet bedienbaren Inhalt um (flex-direction: *-reverse oder \
                         order). Prüfen, ob die Tabreihenfolge der sichtbaren noch folgt.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.2", "2.4.3"])
                .at(at(n.id())),
            );
        }
    }
}

/// Das äußerste Element mit dieser Eigenschaft: Nachfahren eines gemeldeten
/// Elements werden nicht noch einmal gemeldet.
fn aeusserste<'a, N: Node<'a>>(
    kandidaten: impl Iterator<Item = N>,
    mut trifft: impl FnMut(N) -> bool,
) -> Vec<N> {
    let mut gemeldet: HashSet<NodeId> = HashSet::new();
    let mut out = Vec::new();
    for n in kandidaten {
        if !trifft(n) || ancestors(n).any(|a| gemeldet.contains(&a.id())) {
            continue;
        }
        gemeldet.insert(n.id());
        out.push(n);
    }
    out
}

/// Wörter, an denen sich ein Bedienelement zum Anhalten von Bewegung erkennen
/// lässt. Bewusst nur an Bedienelementen gesucht, nicht an Links.
const ANHALTEN: &[&str] = &[
    // Stämme: „paus" trifft pause, pausar, pausieren, mettre en pause.
    "paus",
    "stop",
    "anhalten",
    "arrêt",
    "deten",
    "motion",
    "animation",
    "bewegung",
    "movimiento",
    "mouvement",
];

/// Der sichtbare oder beschriftende Text eines Bedienelements — eine Näherung
/// ohne Namensberechnung (Tier 3 hat keine Semantik), aber mit den Wegen, auf
/// denen eine Checkbox „Pause motion" üblicherweise beschriftet ist.
fn beschriftung<'a, N: Node<'a>>(n: N, labels: &HashMap<&str, Vec<N>>) -> String {
    let mut s = String::new();
    for a in ["aria-label", "title", "value"] {
        if let Some(v) = n.attr(a) {
            s.push_str(v);
            s.push(' ');
        }
    }
    s.push_str(&subtree_text(n));
    if let Some(l) = closest(n, "label") {
        s.push_str(&subtree_text(l));
    }
    if let Some(id) = n.attr("id") {
        for l in labels.get(id).into_iter().flatten() {
            s.push_str(&subtree_text(*l));
        }
    }
    s.to_lowercase()
}

fn ist_schalter<'a, N: Node<'a>>(n: N) -> bool {
    matches!(n.local_name(), "button" | "input" | "select")
        || matches!(n.attr("role"), Some("button" | "switch" | "checkbox"))
}

/// Die `<label for>` des Dokuments, nach Ziel-ID.
fn labels_nach_ziel<'a, D: Rendering>(doc: &'a D) -> HashMap<&'a str, Vec<D::N<'a>>> {
    let mut m: HashMap<&str, Vec<D::N<'a>>> = HashMap::new();
    for l in elements(doc).filter(|n| n.is_element("label")) {
        if let Some(ziel) = l.attr("for") {
            m.entry(ziel).or_default().push(l);
        }
    }
    m
}

/// 2.2.2: eine Animation, die nie endet.
///
/// Gemeldet wird je Gruppe, nicht je Element: Ein animiertes SVG aus zwanzig
/// Kreisen ist eine Bewegung. Und gar nicht, wenn die Seite ein Bedienelement
/// zum Anhalten hat — ob es wirkt, sagt diese Heuristik nicht.
fn endlos_animation<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // `<marquee>` läuft endlos, ohne eigene Pause und unabhängig von
    // `prefers-reduced-motion` — dafür braucht es kein Layout
    // (auditmysite `pause_stop_hide`, Korpus `no_focus_targets`).
    let mit_layout = gemessen(doc, |l| l.infinite_animation.is_some());
    if !mit_layout {
        let feld = "animation-iteration-count";
        ungemessen(
            doc,
            "motion/infinite-animation",
            &["2.2.2"],
            feld,
            locale,
            out,
        );
    }
    let treffer = aeusserste(elements(doc), |n| {
        n.is_element("marquee")
            || (mit_layout && doc.layout(n).and_then(|l| l.infinite_animation) == Some(true))
    });
    if treffer.is_empty() {
        return;
    }
    let labels = labels_nach_ziel(doc);
    let hat_pause = elements(doc).any(|n| {
        ist_schalter(n) && {
            let text = beschriftung(n, &labels);
            ANHALTEN.iter().any(|w| text.contains(w))
        }
    });
    if hat_pause {
        return;
    }
    let mut gruppen: HashSet<NodeId> = HashSet::new();
    for n in treffer {
        let gruppe = n.parent().unwrap_or(n);
        if !gruppen.insert(gruppe.id()) {
            continue;
        }
        let n = if n.parent().is_some_and(|p| {
            p.children()
                .filter(|k| doc.layout(*k).and_then(|l| l.infinite_animation) == Some(true))
                .nth(1)
                .is_some()
        }) {
            gruppe
        } else {
            n
        };
        out.push(
            Finding::review(
                "motion/infinite-animation",
                pick!(
                    locale,
                    "An animation repeats endlessly. Check that it can be paused, stopped or \
                     hidden, or that it ends within five seconds.",
                    "Eine Animation wiederholt sich endlos. Prüfen, ob sie sich anhalten, \
                     beenden oder ausblenden lässt oder nach fünf Sekunden endet.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.2.2"])
            .at(at(n.id())),
        );
    }
}

/// 1.4.10: ein `min-width`, das bei 320 CSS-px waagerechtes Scrollen erzwingt.
fn reflow<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    if !gemessen(doc, |l| l.min_width_px.is_some()) {
        return ungemessen(
            doc,
            "reflow/min-width",
            &["1.4.10"],
            "min-width",
            locale,
            out,
        );
    }
    let treffer = aeusserste(elements(doc), |n| {
        !darf_breit_sein(n)
            && !ancestors(n).any(darf_breit_sein)
            && doc
                .layout(n)
                .and_then(|l| l.min_width_px)
                .is_some_and(|px| px > REFLOW_PX)
    });
    for n in treffer {
        let px = doc.layout(n).and_then(|l| l.min_width_px).unwrap_or(0.0);
        out.push(
            Finding::review(
                "reflow/min-width",
                tr!(
                    locale,
                    "min-width is {px:.0}px — wider than 320px. Check at 400% zoom that the \
                     content reflows without horizontal scrolling.",
                    "min-width ist {px:.0}px — breiter als 320px. Bei 400 % Zoom prüfen, ob \
                     der Inhalt ohne waagerechtes Scrollen umbricht."
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.4.10"])
            .at(at(n.id())),
        );
    }
}

/// 2.1.1 / 4.1.2: sieht anklickbar aus, ist aber weder Bedienelement noch per
/// Tastatur erreichbar — das „Suchen" aus einem `<div>`.
///
/// Ein Inline-`onclick`, das schon `keyboard/click-handler-not-focusable`
/// meldet, zählt hier nicht ein zweites Mal.
fn nur_zeiger<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Ohne gemessenes `cursor` läuft nur der `onclick`-Teil.
    if !gemessen(doc, |l| l.cursor_pointer.is_some()) {
        let wcag = &["2.1.1", "4.1.2"];
        ungemessen(doc, "keyboard/pointer-only", wcag, "cursor", locale, out);
    }
    for n in elements(doc) {
        // Kartenmuster: `cursor: pointer` auf einem Kasten, der einen Link oder
        // Knopf enthält — der Klick landet dort (bundesregierung.de: 27
        // Teaserkarten `div.bpa-teaser`).
        if ist_bedienelement(n)
            || ancestors(n).any(ist_bedienelement)
            || descendants(n).any(|d| d.kind() == NodeKind::Element && ist_bedienelement(d))
            || crate::links::klick_ohne_tastatur(n)
        {
            continue;
        }
        let zeiger = |k| doc.layout(k).and_then(|l| l.cursor_pointer) == Some(true);
        let klickbar = n.has_attr("onclick") || zeiger(n);
        // `cursor` vererbt sich: gemeldet wird nur, wo er beginnt.
        let geerbt = n
            .parent()
            .is_some_and(|p| zeiger(p) && !p.has_attr("onclick"));
        let mit_inhalt = n.has_attr("onclick")
            || descendants(n).any(|d| d.kind() == NodeKind::Text && !d.text().trim().is_empty());
        if klickbar && !geerbt && mit_inhalt {
            out.push(
                Finding::review(
                    "keyboard/pointer-only",
                    pick!(
                        locale,
                        "The element looks clickable (cursor: pointer or onclick) but has no role \
                         and no tabindex. If it does something, keyboard users cannot reach it.",
                        "Das Element sieht anklickbar aus (cursor: pointer oder onclick), hat aber \
                         weder Rolle noch tabindex. Wenn es etwas auslöst, erreicht die Tastatur \
                         es nicht.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["2.1.1", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

/// 2.4.11: ein Bedienelement, das eine fixierte Leiste überdeckt — oder eine
/// Leiste, unter der ein fokussiertes Element verschwinden kann, weil
/// `scroll-padding-top` sie nicht freihält.
fn verdeckt<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    if !gemessen(doc, |l| l.obscured.is_some() || l.hides_focus.is_some()) {
        let feld = "overlap with fixed or sticky elements";
        return ungemessen(doc, "focus/obscured", &["2.4.11"], feld, locale, out);
    }
    for n in elements(doc) {
        let Some(layout) = doc.layout(n) else {
            continue;
        };
        let meldung = if layout.hides_focus == Some(true) {
            pick!(
                locale,
                "This fixed or sticky bar reaches lower than scroll-padding-top. A control \
                 reached by tabbing backwards can end up entirely hidden under it.",
                "Diese fixierte oder klebende Leiste reicht tiefer als scroll-padding-top. \
                 Ein Bedienelement, das rückwärts per Tab erreicht wird, kann ganz darunter \
                 verschwinden.",
            )
        } else if layout.obscured == Some(true)
            && per_tab_erreichbar(n)
            && doc.visually_hidden(n) != Some(true)
        {
            pick!(
                locale,
                "A fixed or sticky element covers this control. Check that it stays \
                 visible when it receives focus.",
                "Ein fixiertes oder klebendes Element überdeckt dieses Bedienelement. \
                 Prüfen, ob es beim Fokussieren sichtbar bleibt.",
            )
        } else {
            continue;
        };
        out.push(
            Finding::review("focus/obscured", meldung)
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.11"])
                .at(at(n.id())),
        );
    }
}

/// Ein Link mitten im Fließtext: für ihn gilt die Ausnahme „inline" aus 2.5.8.
fn im_fliesstext<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("a")
        && n.parent().is_some_and(|p| {
            p.children()
                .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
        })
}

/// Eine Checkbox oder ein Radio mit Label: Das Label ist Teil des Ziels, ein
/// Klick darauf trifft das Feld.
fn ziel_durch_label<'a, N: Node<'a>>(n: N, labels: &HashMap<&str, Vec<N>>) -> bool {
    n.is_element("input")
        && n.attr("type")
            .is_some_and(|t| matches!(t.trim().to_ascii_lowercase().as_str(), "checkbox" | "radio"))
        && (closest(n, "label").is_some() || n.attr("id").is_some_and(|id| labels.contains_key(id)))
}

fn mitte(r: &Rect) -> (f32, f32) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Abstand eines Punkts zu einem Rechteck, 0 innerhalb.
fn abstand(p: (f32, f32), r: &Rect) -> f32 {
    let dx = (r.x - p.0).max(p.0 - (r.x + r.width)).max(0.0);
    let dy = (r.y - p.1).max(p.1 - (r.y + r.height)).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// Mindestgröße eines Ziels nach WCAG 2.5.5 (AAA).
const ZIEL_ERHOEHT_PX: f32 = 44.0;

fn inert<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(ancestors(n))
        .any(|a| a.has_attr("inert"))
}

fn aria_versteckt<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(ancestors(n))
        .any(|a| a.attr("aria-hidden").is_some_and(|v| v.trim() == "true"))
}

/// Ein Kasten ganz links oder über dem Dokument — das übliche Verschieben
/// aus dem Bild (`left: -9999px`).
fn ausserhalb(r: &Rect) -> bool {
    r.x + r.width <= 0.0 || r.y + r.height <= 0.0
}

/// Ob ein Element ein Ziel ist, das man sehen und anklicken kann: nicht
/// `inert`, kein `pointer-events: none`, nicht optisch verborgen, nicht aus
/// dem Bild geschoben.
fn klickbar<'d, D: Rendering>(doc: &'d D, n: D::N<'d>, r: &Rect) -> bool {
    !inert(n)
        && doc.layout(n).and_then(|l| l.pointer_events_none) != Some(true)
        && doc.visually_hidden(n) != Some(true)
        && !ausserhalb(r)
}

/// Das Ziel eines Links ohne Fragment — `None` für Verweise innerhalb der
/// Seite, bei denen das Fragment die ganze Funktion ist (`#setup`, `#`).
fn dokument_ziel<'a, N: Node<'a>>(n: N) -> Option<&'a str> {
    if !n.is_element("a") {
        return None;
    }
    let href = n.attr("href")?.trim();
    let dokument = href.split('#').next().unwrap_or("");
    (!dokument.is_empty()).then_some(dokument)
}

/// 2.5.8 / 2.5.5, Ausnahme „Equivalent": Ein anderer, sichtbarer Link zum
/// selben Dokument hat die geforderte Größe. Nach auditmysite#706
/// (`target_size_equivalent`): Ein Gegenstück unter `aria-hidden`, `inert`,
/// optisch verborgen oder außerhalb des Bilds zählt nicht.
fn gleichwertig<'d, D: Rendering>(
    doc: &'d D,
    n: D::N<'d>,
    ziele: &[(D::N<'d>, Rect)],
    mindest: f32,
) -> bool {
    let Some(ziel) = dokument_ziel(n) else {
        return false;
    };
    ziele.iter().any(|(k, kr)| {
        k.id() != n.id()
            && dokument_ziel(*k) == Some(ziel)
            && kr.width >= mindest
            && kr.height >= mindest
            && !aria_versteckt(*k)
            && klickbar(doc, *k, kr)
    })
}

/// Ob die Größe des Ziels noch nicht die endgültige ist.
fn wachsend<'d, D: Rendering>(doc: &'d D, n: D::N<'d>) -> bool {
    doc.layout(n).and_then(|l| l.animating) == Some(true)
}

fn ungemessen_ziel(locale: Locale, id: &str, wcag: &str, n: NodeId) -> Finding {
    Finding::untested(
        id,
        pick!(
            locale,
            "The target was still animating when measured; its size was not judged.",
            "Das Ziel war beim Messen noch in Bewegung; seine Größe wurde nicht beurteilt.",
        ),
    )
    .with_severity(Severity::Medium)
    .with_wcag([wcag])
    .at(at(n))
}

/// 2.5.8: ein Ziel unter 24 × 24 CSS-px, das auch die Abstandsausnahme nicht
/// erfüllt — nach dem Normtext: Ein Kreis von 24 px Durchmesser um die Mitte
/// des Ziels schneidet ein anderes Ziel oder den Kreis eines anderen zu kleinen
/// Ziels. Dazu die Ausnahmen „inline" und „Equivalent".
///
/// Gemessen werden nur Ziele, die man sehen und anklicken kann; Nachbarn unter
/// `inert` oder mit `pointer-events: none` engen nicht ein (auditmysite#705).
/// Ein Ziel, das beim Messen noch wächst, bleibt ungemessen (#706).
fn zielgroesse<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let labels = labels_nach_ziel(doc);
    let ziele: Vec<(D::N<'_>, Rect)> = elements(doc)
        .filter(|n| per_tab_erreichbar(*n))
        .filter_map(|n| doc.bounds(n).filter(|r| !r.is_empty()).map(|r| (n, r)))
        .filter(|(n, r)| klickbar(doc, *n, r))
        .collect();
    let zu_klein = |r: &Rect| r.width < ZIEL_PX || r.height < ZIEL_PX;
    let radius = ZIEL_PX / 2.0;

    for (n, r) in &ziele {
        if !zu_klein(r) || im_fliesstext(*n) || ziel_durch_label(*n, &labels) {
            continue;
        }
        let m = mitte(r);
        let bedraengt = ziele.iter().any(|(k, kr)| {
            k != n
                && if zu_klein(kr) {
                    let km = mitte(kr);
                    ((m.0 - km.0).powi(2) + (m.1 - km.1).powi(2)).sqrt() < 2.0 * radius
                } else {
                    abstand(m, kr) < radius
                }
        });
        if !bedraengt || gleichwertig(doc, *n, &ziele, ZIEL_PX) {
            continue;
        }
        if wachsend(doc, *n) {
            out.push(ungemessen_ziel(locale, "targets/size", "2.5.8", n.id()));
            continue;
        }
        let r = *r;
        let n = *n;
        out.push(
            Finding::review(
                "targets/size",
                tr!(
                    locale,
                    "The target is {:.0} × {:.0}px, under 24 × 24px, and too close to another \
                     target for the spacing exception.",
                    "Das Ziel misst {:.0} × {:.0}px, unter 24 × 24px, und liegt für die \
                     Abstandsausnahme zu nah an einem anderen Ziel.",
                    r.width,
                    r.height
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.5.8"])
            .at(at(n.id())),
        );
    }
}

/// 2.5.5 (AAA): ein Ziel unter 44 × 44 CSS-px. Ohne Abstandsausnahme — die
/// kennt 2.5.5 nicht —, mit den Ausnahmen „inline" und „Equivalent".
/// Gemeldet wird nur, was 2.5.8 nicht schon meldet.
fn zielgroesse_erhoeht<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let labels = labels_nach_ziel(doc);
    let ziele: Vec<(D::N<'_>, Rect)> = elements(doc)
        .filter(|n| per_tab_erreichbar(*n))
        .filter_map(|n| doc.bounds(n).filter(|r| !r.is_empty()).map(|r| (n, r)))
        .filter(|(n, r)| klickbar(doc, *n, r))
        .collect();
    let mut schon = Vec::new();
    zielgroesse(doc, locale, &mut schon);
    let gemeldet: HashSet<String> = schon
        .iter()
        .filter_map(|f| f.location.node.clone())
        .collect();
    for (n, r) in &ziele {
        if (r.width >= ZIEL_ERHOEHT_PX && r.height >= ZIEL_ERHOEHT_PX)
            || im_fliesstext(*n)
            || ziel_durch_label(*n, &labels)
            || gemeldet.contains(&n.id().to_string())
            || gleichwertig(doc, *n, &ziele, ZIEL_ERHOEHT_PX)
        {
            continue;
        }
        if wachsend(doc, *n) {
            out.push(ungemessen_ziel(
                locale,
                "targets/size-enhanced",
                "2.5.5",
                n.id(),
            ));
            continue;
        }
        out.push(
            Finding::review(
                "targets/size-enhanced",
                tr!(
                    locale,
                    "The target is {:.0} × {:.0}px; enhanced target size (AAA) asks for \
                     44 × 44px.",
                    "Das Ziel misst {:.0} × {:.0}px; die erhöhte Zielgröße (AAA) verlangt \
                     44 × 44px.",
                    r.width,
                    r.height
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["2.5.5"])
            .at(at(n.id())),
        );
    }
}

/// 2.4.7: beim Fokussieren ändert sich kein Stil, der als Indikator taugt.
fn fokus_sichtbar<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut ziele = 0usize;
    let mut gemessen = 0usize;
    for n in elements(doc) {
        if !per_tab_erreichbar(n) {
            continue;
        }
        ziele += 1;
        let Some(sichtbar) = doc.layout(n).and_then(|l| l.focus_visible) else {
            continue;
        };
        gemessen += 1;
        if !sichtbar {
            out.push(
                Finding::review(
                    "focus/indicator-missing",
                    pick!(
                        locale,
                        "On focus, neither outline, box-shadow, border, background nor text \
                         changes. Check that keyboard focus is visible.",
                        "Beim Fokussieren ändern sich weder outline, box-shadow, Rahmen, \
                         Hintergrund noch Text. Prüfen, ob der Tastaturfokus sichtbar ist.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["2.4.7"])
                .at(at(n.id())),
            );
        }
    }
    // Nicht gemessen ist nicht bestanden.
    if ziele > 0 && gemessen == 0 {
        out.push(
            Finding::untested(
                "focus/indicator-unmeasured",
                pick!(
                    locale,
                    "Focus visibility was not measured — that needs a pass that moves focus. \
                     Check by hand that every control shows keyboard focus.",
                    "Die Fokus-Sichtbarkeit wurde nicht gemessen — das braucht einen Durchgang, \
                     der den Fokus bewegt. Von Hand prüfen, ob jedes Bedienelement den \
                     Tastaturfokus zeigt.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.7"])
            .at(at(doc.root().id())),
        );
    }
}

pub(crate) const METAS: [Meta; 8] = [
    Meta {
        ids: &["order/visual-mismatch"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["1.3.2", "2.4.3"],
        severity: Severity::Medium,
        help: "Visual order and tab order should match; CSS reordering (flex-direction: \
               *-reverse, order) over focusable content is a hint they do not.",
        #[cfg(feature = "de")]
        help_de: "Sichtbare Reihenfolge und Tabreihenfolge sollen übereinstimmen; CSS-\
                  Umordnung über bedienbarem Inhalt ist ein Hinweis, dass sie es nicht tun.",
    },
    Meta {
        ids: &["motion/infinite-animation"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.2.2"],
        severity: Severity::Medium,
        help: "Movement that lasts longer than five seconds needs a way to pause, stop or \
               hide it.",
        #[cfg(feature = "de")]
        help_de: "Bewegung, die länger als fünf Sekunden läuft, braucht eine Möglichkeit, sie \
                  anzuhalten, zu beenden oder auszublenden.",
    },
    Meta {
        ids: &["reflow/min-width"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["1.4.10"],
        severity: Severity::Medium,
        help: "Content must reflow at 320 CSS pixels wide; a min-width above that forces \
               horizontal scrolling.",
        #[cfg(feature = "de")]
        help_de: "Inhalt muss bei 320 CSS-Pixeln Breite umbrechen; ein min-width darüber \
                  erzwingt waagerechtes Scrollen.",
    },
    Meta {
        ids: &["keyboard/pointer-only"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.1.1", "4.1.2"],
        severity: Severity::High,
        help: "Whatever reacts to a click must be a control: a <button> or <a>, or a role and \
               tabindex with keyboard handling.",
        #[cfg(feature = "de")]
        help_de: "Was auf einen Klick reagiert, muss ein Bedienelement sein: <button> oder <a>, \
                  oder Rolle und tabindex mit Tastaturbedienung.",
    },
    Meta {
        ids: &["focus/obscured"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.4.11"],
        severity: Severity::Medium,
        help: "A focused control must not be entirely hidden by fixed or sticky content.",
        #[cfg(feature = "de")]
        help_de: "Ein fokussiertes Bedienelement darf nicht ganz von fixiertem oder klebendem \
                  Inhalt verdeckt sein.",
    },
    Meta {
        ids: &["targets/size"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.5.8"],
        severity: Severity::Medium,
        help: "Targets need at least 24 × 24 CSS pixels, or enough spacing to their \
               neighbours.",
        #[cfg(feature = "de")]
        help_de: "Ziele brauchen mindestens 24 × 24 CSS-Pixel oder genug Abstand zu ihren \
                  Nachbarn.",
    },
    Meta {
        ids: &["targets/size-enhanced"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.5.5"],
        severity: Severity::Low,
        help: "Enhanced target size (AAA): targets of at least 44 × 44 CSS pixels.",
        #[cfg(feature = "de")]
        help_de: "Erhöhte Zielgröße (AAA): Ziele von mindestens 44 × 44 CSS-Pixeln.",
    },
    Meta {
        ids: &["focus/indicator-missing", "focus/indicator-unmeasured"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.4.7"],
        severity: Severity::High,
        help: "Keyboard focus must be visible on every control.",
        #[cfg(feature = "de")]
        help_de: "Der Tastaturfokus muss an jedem Bedienelement sichtbar sein.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn funktionen<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 8] {
    [
        reihenfolge,
        endlos_animation,
        reflow,
        nur_zeiger,
        verdeckt,
        zielgroesse,
        zielgroesse_erhoeht,
        fokus_sichtbar,
    ]
}
