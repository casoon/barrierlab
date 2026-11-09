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
//! [`Rendering::bounds`]. Ohne sie melden diese Regeln keinen Befund, sondern
//! stehen in `rule_runs` mit `NotRun::CapabilityMissing` und Grund. Das gilt
//! auch für die Fokus-Sichtbarkeit, die nur ein eigener Durchgang misst.
//!
//! [`Rendering::layout`]: a11y_dom::Rendering::layout
//! [`Rendering::bounds`]: a11y_dom::Rendering::bounds

use std::collections::{HashMap, HashSet};

use a11y_dom::{
    Layout, Node, NodeId, NodeKind, Rect, Rendering, ScopedId, Tier, ancestors, closest,
    descendants, elements, scoped_id, subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, de_text, pick, tr};
use crate::not_run;
use crate::registry::Meta;
use crate::structure::per_tab_reachable;
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Ab dieser Breite erzwingt `min-width` waagerechtes Scrollen bei 320 CSS-px
/// — der Breite, auf die WCAG 1.4.10 eine Seite umbrechen lässt.
const REFLOW_PX: f32 = 320.0;

/// Mindestgröße eines Ziels nach WCAG 2.5.8.
const TARGET_PX: f32 = 24.0;

/// Elemente, die von sich aus bedienbar sind oder deren Klick der Browser
/// weiterreicht. Ein `cursor: pointer` darin ist kein Hinweis auf einen Fake.
fn is_control<'a, N: Node<'a>>(n: N) -> bool {
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
fn may_be_wide<'a, N: Node<'a>>(n: N) -> bool {
    matches!(
        n.local_name(),
        "table" | "pre" | "code" | "video" | "canvas" | "iframe" | "img" | "svg" | "math"
    )
}

fn has_tab_target<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(descendants(n))
        .any(|d| d.kind() == NodeKind::Element && per_tab_reachable(d))
}

/// Ob ein Kind etwas trägt, dessen Reihenfolge zählt: ein Bedienelement oder
/// Text. 1.3.2 betrifft die Lesereihenfolge, nicht nur die Tabreihenfolge
/// (auditmysite-Korpus `text_and_layout`, `div.flex-reorder`).
fn has_content<'a, N: Node<'a>>(n: N) -> bool {
    has_tab_target(n) || !subtree_text(n).trim().is_empty()
}

/// Alle Bedienelemente darin führen zum selben Ziel — die Teaserkarte mit
/// Bild- und Titellink (spiegel.de: 20 Karten mit `flex-row-reverse`). Die
/// Reihenfolge ändert dann nicht, wohin man kommt.
fn a_target<'a, N: Node<'a>>(n: N) -> bool {
    let mut targets =
        descendants(n).filter(|d| d.kind() == NodeKind::Element && per_tab_reachable(*d));
    let Some(first) = targets.next() else {
        return false;
    };
    let href = |e: N| e.is_element("a").then(|| e.attr("href")).flatten();
    let Some(target) = href(first) else {
        return false;
    };
    let rest: Vec<N> = targets.collect();
    !rest.is_empty() && rest.into_iter().all(|e| href(e) == Some(target))
}

/// Ob irgendein Element dieses Layout-Feld gemessen hat.
fn measured<D: Rendering>(doc: &D, field: impl Fn(&Layout) -> bool) -> bool {
    elements(doc).any(|n| doc.layout(n).is_some_and(|l| field(&l)))
}

/// Nicht gemessen ist nicht bestanden: ein Vermerk `NotRun::CapabilityMissing`
/// (kein Befund), wenn kein Element das Feld trägt, das die Heuristik braucht.
fn unmeasured(id: &str, field: &str, locale: Locale, out: &mut Vec<Finding>) {
    out.push(not_run::note(
        id,
        tr!(
            locale,
            "The host does not measure {field}; this heuristic did not run.",
            "Der Host misst {field} nicht; diese Heuristik lief nicht."
        ),
    ));
}

/// 1.3.2 / 2.4.3: umgekehrte Flex-Richtung oder `order` über bedienbarem
/// Inhalt — Tabreihenfolge und sichtbare Reihenfolge laufen auseinander.
fn order<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    if !measured(doc, |l| l.flex_reversed.is_some() || l.order.is_some()) {
        let field = "flex-direction / order";
        return unmeasured("order/visual-mismatch", field, locale, out);
    }
    for n in elements(doc) {
        let Some(layout) = doc.layout(n) else {
            continue;
        };
        let reversed = layout.flex_reversed == Some(true)
            && n.children()
                .filter(|k| k.kind() == NodeKind::Element && has_content(*k))
                .nth(1)
                .is_some()
            && !a_target(n);
        let reordered = n.children().any(|k| {
            k.kind() == NodeKind::Element
                && doc.layout(k).and_then(|l| l.order).is_some_and(|o| o != 0)
                && has_content(k)
        }) && !a_target(n);
        if reversed || reordered {
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
fn outermost<'a, N: Node<'a>>(
    candidates: impl Iterator<Item = N>,
    mut hits: impl FnMut(N) -> bool,
) -> Vec<N> {
    let mut reported: HashSet<NodeId> = HashSet::new();
    let mut out = Vec::new();
    for n in candidates {
        if !hits(n) || ancestors(n).any(|a| reported.contains(&a.id())) {
            continue;
        }
        reported.insert(n.id());
        out.push(n);
    }
    out
}

/// Wörter, an denen sich ein Bedienelement zum Anhalten von Bewegung erkennen
/// lässt. Bewusst nur an Bedienelementen gesucht, nicht an Links.
const PAUSE: &[&str] = &[
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
fn label<'a, N: Node<'a>>(n: N, labels: &HashMap<ScopedId, Vec<N>>) -> String {
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
        for l in labels.get(&scoped_id(n, id)).into_iter().flatten() {
            s.push_str(&subtree_text(*l));
        }
    }
    s.to_lowercase()
}

fn is_switch<'a, N: Node<'a>>(n: N) -> bool {
    matches!(n.local_name(), "button" | "input" | "select")
        || matches!(n.attr("role"), Some("button" | "switch" | "checkbox"))
}

/// Die `<label for>` des Dokuments, nach Ziel-ID.
fn labels_by_target<'a, D: Rendering>(doc: &'a D) -> HashMap<ScopedId<'a>, Vec<D::N<'a>>> {
    let mut m: HashMap<ScopedId<'a>, Vec<D::N<'a>>> = HashMap::new();
    for l in elements(doc).filter(|n| n.is_element("label")) {
        if let Some(target) = l.attr("for") {
            m.entry(scoped_id(l, target)).or_default().push(l);
        }
    }
    m
}

/// 2.2.2: eine Animation, die nie endet.
///
/// Gemeldet wird je Gruppe, nicht je Element: Ein animiertes SVG aus zwanzig
/// Kreisen ist eine Bewegung. Und gar nicht, wenn die Seite ein Bedienelement
/// zum Anhalten hat — ob es wirkt, sagt diese Heuristik nicht.
fn endless_animation<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // `<marquee>` läuft endlos, ohne eigene Pause und unabhängig von
    // `prefers-reduced-motion` — dafür braucht es kein Layout
    // (auditmysite `pause_stop_hide`, Korpus `no_focus_targets`).
    let with_layout = measured(doc, |l| l.infinite_animation.is_some());
    if !with_layout {
        let field = "animation-iteration-count";
        unmeasured("motion/infinite-animation", field, locale, out);
    }
    let hit = outermost(elements(doc), |n| {
        n.is_element("marquee")
            || (with_layout && doc.layout(n).and_then(|l| l.infinite_animation) == Some(true))
    });
    if hit.is_empty() {
        return;
    }
    let labels = labels_by_target(doc);
    let has_pause = elements(doc).any(|n| {
        is_switch(n) && {
            let text = label(n, &labels);
            PAUSE.iter().any(|w| text.contains(w))
        }
    });
    if has_pause {
        return;
    }
    let mut groups: HashSet<NodeId> = HashSet::new();
    for n in hit {
        let group = n.parent().unwrap_or(n);
        if !groups.insert(group.id()) {
            continue;
        }
        let n = if n.parent().is_some_and(|p| {
            p.children()
                .filter(|k| doc.layout(*k).and_then(|l| l.infinite_animation) == Some(true))
                .nth(1)
                .is_some()
        }) {
            group
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
    if !measured(doc, |l| l.min_width_px.is_some()) {
        return unmeasured("reflow/min-width", "min-width", locale, out);
    }
    let hit = outermost(elements(doc), |n| {
        !may_be_wide(n)
            && !ancestors(n).any(may_be_wide)
            && doc
                .layout(n)
                .and_then(|l| l.min_width_px)
                .is_some_and(|px| px > REFLOW_PX)
    });
    for n in hit {
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
fn only_pointer<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Ohne gemessenes `cursor` läuft nur der `onclick`-Teil.
    if !measured(doc, |l| l.cursor_pointer.is_some()) {
        unmeasured("keyboard/pointer-only", "cursor", locale, out);
    }
    for n in elements(doc) {
        // Kartenmuster: `cursor: pointer` auf einem Kasten, der einen Link oder
        // Knopf enthält — der Klick landet dort (bundesregierung.de: 27
        // Teaserkarten `div.bpa-teaser`).
        if is_control(n)
            || ancestors(n).any(is_control)
            || descendants(n).any(|d| d.kind() == NodeKind::Element && is_control(d))
            || crate::links::click_without_keyboard(n)
        {
            continue;
        }
        let pointer = |k| doc.layout(k).and_then(|l| l.cursor_pointer) == Some(true);
        let clickable = n.has_attr("onclick") || pointer(n);
        // `cursor` vererbt sich: gemeldet wird nur, wo er beginnt.
        let inherited = n
            .parent()
            .is_some_and(|p| pointer(p) && !p.has_attr("onclick"));
        let with_content = n.has_attr("onclick")
            || descendants(n).any(|d| d.kind() == NodeKind::Text && !d.text().trim().is_empty());
        if clickable && !inherited && with_content {
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
///
/// Hat der Host nach dem Fokussieren gemessen (`Rendering::focus_cover`),
/// entscheidet die Messung über das Element: ganz überdeckt heißt `REVIEW`,
/// und das Bit `obscured` (statische Geometrie) zählt für dieses Element
/// nicht mehr. Ohne Fokuslauf bleibt es bei den Bits.
fn covered<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let probed = elements(doc).any(|n| doc.focus_cover(n).is_some());
    if !probed && !measured(doc, |l| l.obscured.is_some() || l.hides_focus.is_some()) {
        let field = "overlap with fixed or sticky elements";
        return unmeasured("focus/obscured", field, locale, out);
    }
    for n in elements(doc) {
        let cover = doc.focus_cover(n);
        if let Some(c) = cover.filter(|c| c.points > 0 && c.covered >= c.points) {
            out.push(
                Finding::review(
                    "focus/obscured",
                    pick!(
                        locale,
                        "After receiving focus, every sample point of this control lies under a \
                         fixed or sticky element. Check that the control is not entirely hidden \
                         when it is focused.",
                        "Nach dem Fokussieren liegt jeder Abtastpunkt dieses Bedienelements unter \
                         einem fixierten oder klebenden Element. Prüfen, ob das Bedienelement \
                         beim Fokussieren nicht ganz verdeckt ist.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.11"])
                .with_evidence(crate::probes::cover_evidence(c))
                .at(at(n.id())),
            );
            continue;
        }
        let Some(layout) = doc.layout(n) else {
            continue;
        };
        let message = if layout.hides_focus == Some(true) {
            pick!(
                locale,
                "This fixed or sticky bar reaches lower than scroll-padding-top. A control \
                 reached by tabbing backwards can end up entirely hidden under it.",
                "Diese fixierte oder klebende Leiste reicht tiefer als scroll-padding-top. \
                 Ein Bedienelement, das rückwärts per Tab erreicht wird, kann ganz darunter \
                 verschwinden.",
            )
        } else if layout.obscured == Some(true)
            && cover.is_none()
            && per_tab_reachable(n)
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
            Finding::review("focus/obscured", message)
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.11"])
                .at(at(n.id())),
        );
    }
}

/// Ein Link mitten im Fließtext: für ihn gilt die Ausnahme „inline" aus 2.5.8.
fn in_running_text<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("a")
        && n.parent().is_some_and(|p| {
            p.children()
                .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
        })
}

/// Eine Checkbox oder ein Radio mit Label: Das Label ist Teil des Ziels, ein
/// Klick darauf trifft das Feld.
fn target_by_label<'a, N: Node<'a>>(n: N, labels: &HashMap<ScopedId, Vec<N>>) -> bool {
    n.is_element("input")
        && n.attr("type")
            .is_some_and(|t| matches!(t.trim().to_ascii_lowercase().as_str(), "checkbox" | "radio"))
        && (closest(n, "label").is_some()
            || n.attr("id")
                .is_some_and(|id| labels.contains_key(&scoped_id(n, id))))
}

fn centre(r: &Rect) -> (f32, f32) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Abstand eines Punkts zu einem Rechteck, 0 innerhalb.
fn gap(p: (f32, f32), r: &Rect) -> f32 {
    let dx = (r.x - p.0).max(p.0 - (r.x + r.width)).max(0.0);
    let dy = (r.y - p.1).max(p.1 - (r.y + r.height)).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// Mindestgröße eines Ziels nach WCAG 2.5.5 (AAA).
const TARGET_ENHANCED_PX: f32 = 44.0;

fn inert<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(ancestors(n))
        .any(|a| a.has_attr("inert"))
}

fn aria_hidden<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(ancestors(n))
        .any(|a| a.attr("aria-hidden").is_some_and(|v| v.trim() == "true"))
}

/// Ein Kasten ganz links oder über dem Dokument — das übliche Verschieben
/// aus dem Bild (`left: -9999px`).
fn outside_of(r: &Rect) -> bool {
    r.x + r.width <= 0.0 || r.y + r.height <= 0.0
}

/// Ein Zeigerziel im Sinne von 2.5.8: ein Bedienelement, unabhängig vom
/// `tabindex`. Mit „roving tabindex" (Karussellpunkte, Tabs, Menüs) hat nur
/// ein Element der Gruppe `tabindex="0"`, die übrigen `-1` — angeklickt
/// werden sie trotzdem (bundesregierung.de, `#slick-slide-control00`,
/// 10 × 10 px).
fn is_pointer_target<'a, N: Node<'a>>(n: N) -> bool {
    // Ein eingebettetes Dokument ist ein Behälter, kein Ziel: Zeigeraktionen
    // nehmen die Ziele darin an, und die beurteilt der Durchgang durch den
    // Rahmen (Understanding 2.5.8, „target").
    if matches!(n.local_name(), "iframe" | "frame" | "object" | "embed") {
        return false;
    }
    let native = match n.local_name() {
        "a" | "area" => n.has_attr("href"),
        "button" | "select" | "textarea" | "summary" => !n.has_attr("disabled"),
        "input" => {
            !n.has_attr("disabled")
                && !n
                    .attr("type")
                    .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden"))
        }
        _ => false,
    };
    native
        || crate::structure::explicit_role(n).is_some_and(|r| {
            matches!(
                r,
                "button"
                    | "link"
                    | "checkbox"
                    | "radio"
                    | "switch"
                    | "tab"
                    | "menuitem"
                    | "menuitemcheckbox"
                    | "menuitemradio"
                    | "option"
                    | "slider"
                    | "spinbutton"
                    | "treeitem"
            )
        })
        || per_tab_reachable(n)
}

/// Der Bereich, der Zeigeraktionen für das Ziel annimmt. Ein Inline-Kasten
/// (`<a>` um ein Bild) umfasst seine ersetzten Kinder nicht; ein Klick auf
/// das Bild aktiviert aber den Link (Understanding 2.5.8, „target: region of
/// the display that will accept a pointer action"). Deshalb die Vereinigung
/// mit den dargestellten ersetzten Nachfahren.
fn target_box<'d, D: Rendering>(doc: &'d D, n: D::N<'d>) -> Option<Rect> {
    let mut r = doc.bounds(n).filter(|r| !r.is_empty())?;
    let inline = doc
        .computed_style(n)
        .is_some_and(|s| s.display.as_deref() == Some("inline"));
    if !inline {
        return Some(r);
    }
    for d in a11y_dom::descendants(n).filter(|d| {
        matches!(
            d.local_name(),
            "img" | "svg" | "video" | "canvas" | "picture" | "object" | "embed"
        )
    }) {
        let visible = doc
            .computed_style(d)
            .is_none_or(|s| s.display.as_deref() != Some("none"));
        if let Some(k) = doc.bounds(d).filter(|k| !k.is_empty() && visible) {
            let (x0, y0) = (r.x.min(k.x), r.y.min(k.y));
            let x1 = (r.x + r.width).max(k.x + k.width);
            let y1 = (r.y + r.height).max(k.y + k.height);
            r = Rect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            };
        }
    }
    Some(r)
}

/// Ob ein Element ein Ziel ist, das man sehen und anklicken kann: nicht
/// `inert`, kein `pointer-events: none`, nicht optisch verborgen, nicht aus
/// dem Bild geschoben.
fn clickable<'d, D: Rendering>(doc: &'d D, n: D::N<'d>, r: &Rect) -> bool {
    !inert(n)
        && doc.layout(n).and_then(|l| l.pointer_events_none) != Some(true)
        && doc.visually_hidden(n) != Some(true)
        && !outside_of(r)
}

/// Das Ziel eines Links ohne Fragment — `None` für Verweise innerhalb der
/// Seite, bei denen das Fragment die ganze Funktion ist (`#setup`, `#`).
fn document_target<'a, N: Node<'a>>(n: N) -> Option<&'a str> {
    if !n.is_element("a") {
        return None;
    }
    let href = n.attr("href")?.trim();
    let document = href.split('#').next().unwrap_or("");
    (!document.is_empty()).then_some(document)
}

/// 2.5.8 / 2.5.5, Ausnahme „Equivalent": Ein anderer, sichtbarer Link zum
/// selben Dokument hat die geforderte Größe. Nach auditmysite#706
/// (`target_size_equivalent`): Ein Gegenstück unter `aria-hidden`, `inert`,
/// optisch verborgen oder außerhalb des Bilds zählt nicht.
fn equivalent<'d, D: Rendering>(
    doc: &'d D,
    n: D::N<'d>,
    targets: &[(D::N<'d>, Rect)],
    minimum: f32,
) -> bool {
    let Some(target) = document_target(n) else {
        return false;
    };
    targets.iter().any(|(k, kr)| {
        k.id() != n.id()
            && document_target(*k) == Some(target)
            && kr.width >= minimum
            && kr.height >= minimum
            && !aria_hidden(*k)
            && clickable(doc, *k, kr)
    })
}

/// Ob die Größe des Ziels noch nicht die endgültige ist.
fn growing<'d, D: Rendering>(doc: &'d D, n: D::N<'d>) -> bool {
    doc.layout(n).and_then(|l| l.animating) == Some(true)
}

fn unmeasured_target(locale: Locale, id: &str, wcag: &str, n: NodeId) -> Finding {
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
fn target_size<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let labels = labels_by_target(doc);
    let targets: Vec<(D::N<'_>, Rect)> = elements(doc)
        .filter(|n| is_pointer_target(*n))
        .filter_map(|n| target_box(doc, n).map(|r| (n, r)))
        .filter(|(n, r)| clickable(doc, *n, r))
        .collect();
    let too_small = |r: &Rect| r.width < TARGET_PX || r.height < TARGET_PX;
    let radius = TARGET_PX / 2.0;

    for (n, r) in &targets {
        if !too_small(r) || in_running_text(*n) || target_by_label(*n, &labels) {
            continue;
        }
        let m = centre(r);
        let crowds = targets.iter().any(|(k, kr)| {
            k != n
                && if too_small(kr) {
                    let km = centre(kr);
                    ((m.0 - km.0).powi(2) + (m.1 - km.1).powi(2)).sqrt() < 2.0 * radius
                } else {
                    gap(m, kr) < radius
                }
        });
        if !crowds || equivalent(doc, *n, &targets, TARGET_PX) {
            continue;
        }
        if growing(doc, *n) {
            out.push(unmeasured_target(locale, "targets/size", "2.5.8", n.id()));
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
fn target_size_enhanced<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let labels = labels_by_target(doc);
    let targets: Vec<(D::N<'_>, Rect)> = elements(doc)
        .filter(|n| is_pointer_target(*n))
        .filter_map(|n| target_box(doc, n).map(|r| (n, r)))
        .filter(|(n, r)| clickable(doc, *n, r))
        .collect();
    let mut already = Vec::new();
    target_size(doc, locale, &mut already);
    let reported: HashSet<String> = already
        .iter()
        .filter_map(|f| f.location.node.clone())
        .collect();
    for (n, r) in &targets {
        if (r.width >= TARGET_ENHANCED_PX && r.height >= TARGET_ENHANCED_PX)
            || in_running_text(*n)
            || target_by_label(*n, &labels)
            || reported.contains(&n.id().to_string())
            || equivalent(doc, *n, &targets, TARGET_ENHANCED_PX)
        {
            continue;
        }
        if growing(doc, *n) {
            out.push(unmeasured_target(
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
fn focus_visible<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut targets = 0usize;
    let mut measured = 0usize;
    for n in elements(doc) {
        if !per_tab_reachable(n) {
            continue;
        }
        targets += 1;
        let Some(visible) = doc.layout(n).and_then(|l| l.focus_visible) else {
            continue;
        };
        measured += 1;
        if !visible {
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
    if targets > 0 && measured == 0 {
        out.push(not_run::note(
            "focus/indicator-missing",
            pick!(
                locale,
                "The host does not measure focus visibility — that needs a pass that moves \
                 focus.",
                "Der Host misst die Fokus-Sichtbarkeit nicht — das braucht einen Durchgang, \
                 der den Fokus bewegt.",
            ),
        ));
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
        help_de: de_text!(
            "Sichtbare Reihenfolge und Tabreihenfolge sollen übereinstimmen; CSS-\
                  Umordnung über bedienbarem Inhalt ist ein Hinweis, dass sie es nicht tun."
        ),
    },
    Meta {
        ids: &["motion/infinite-animation"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.2.2"],
        severity: Severity::Medium,
        help: "Movement that lasts longer than five seconds needs a way to pause, stop or \
               hide it.",
        help_de: de_text!(
            "Bewegung, die länger als fünf Sekunden läuft, braucht eine Möglichkeit, sie \
                  anzuhalten, zu beenden oder auszublenden."
        ),
    },
    Meta {
        ids: &["reflow/min-width"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["1.4.10"],
        severity: Severity::Medium,
        help: "Content must reflow at 320 CSS pixels wide; a min-width above that forces \
               horizontal scrolling.",
        help_de: de_text!(
            "Inhalt muss bei 320 CSS-Pixeln Breite umbrechen; ein min-width darüber \
                  erzwingt waagerechtes Scrollen."
        ),
    },
    Meta {
        ids: &["keyboard/pointer-only"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.1.1", "4.1.2"],
        severity: Severity::High,
        help: "Whatever reacts to a click must be a control: a <button> or <a>, or a role and \
               tabindex with keyboard handling.",
        help_de: de_text!(
            "Was auf einen Klick reagiert, muss ein Bedienelement sein: <button> oder <a>, \
                  oder Rolle und tabindex mit Tastaturbedienung."
        ),
    },
    Meta {
        ids: &["focus/obscured"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.4.11"],
        severity: Severity::Medium,
        help: "A focused control must not be entirely hidden by fixed or sticky content. \
               Seen: the bits obscured and hides_focus, and, when the host ran a focus pass, \
               Rendering::focus_cover (sample points under fixed or sticky elements after the \
               host focused and scrolled the control); the probe replaces the static bit for \
               that control. Sampling approximates \"entirely\", so the result stays a review. \
               Which scroll a real Tab produces is not settled; the host records how it \
               scrolled.",
        help_de: de_text!(
            "Ein fokussiertes Bedienelement darf nicht ganz von fixiertem oder klebendem \
                  Inhalt verdeckt sein. Gesehen werden die Bits obscured und hides_focus und, \
                  wenn der Host einen Fokuslauf gemacht hat, Rendering::focus_cover \
                  (Abtastpunkte unter fixierten oder klebenden Elementen, nachdem der Host das \
                  Bedienelement fokussiert und gescrollt hat); die Messung ersetzt für dieses \
                  Bedienelement das statische Bit. Die Abtastung nähert \"ganz\" nur an, das \
                  Ergebnis bleibt Review. Welchen Scroll ein echtes Tab erzeugt, ist offen; der \
                  Host hält fest, wie er gescrollt hat."
        ),
    },
    Meta {
        ids: &["targets/size"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.5.8"],
        severity: Severity::Medium,
        help: "Targets need at least 24 × 24 CSS pixels, or enough spacing to their \
               neighbours.",
        help_de: de_text!(
            "Ziele brauchen mindestens 24 × 24 CSS-Pixel oder genug Abstand zu ihren \
                  Nachbarn."
        ),
    },
    Meta {
        ids: &["targets/size-enhanced"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.5.5"],
        severity: Severity::Low,
        help: "Enhanced target size (AAA): targets of at least 44 × 44 CSS pixels.",
        help_de: de_text!("Erhöhte Zielgröße (AAA): Ziele von mindestens 44 × 44 CSS-Pixeln."),
    },
    Meta {
        ids: &["focus/indicator-missing"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.4.7"],
        severity: Severity::High,
        help: "Keyboard focus must be visible on every control.",
        help_de: de_text!("Der Tastaturfokus muss an jedem Bedienelement sichtbar sein."),
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn functions<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 8] {
    [
        order,
        endless_animation,
        reflow,
        only_pointer,
        covered,
        target_size,
        target_size_enhanced,
        focus_visible,
    ]
}
