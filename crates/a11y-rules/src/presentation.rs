//! Darstellungsregeln mit gemessenen Werten (casoon/barrierlab#21, zweiter
//! Teil): überflüssiges `role="list"` (4.1.2), Links, die sich nur durch
//! Farbe abheben (1.4.1), und scrollbare Bereiche ohne Tastaturzugang
//! (2.1.1).
//!
//! Portiert aus auditmysite (`redundant_role` für `<ul>`/`<ol>`,
//! `use_of_color`, `scrollable_region`). Anders als die Heuristiken in
//! `heuristics.rs` urteilen diese Regeln mit `FAIL`: Was sie brauchen — das
//! berechnete `list-style-type`, die Textauszeichnung eines Links, der
//! Überhang eines scrollbaren Kastens —, ist gemessen, nicht vermutet.
//!
//! Fehlt dem Host ein Feld, steht das im Bericht: je Regel ein `UNTESTED`
//! für die Seite, sobald es etwas zu prüfen gäbe. Ein stiller Nicht-Befund
//! wäre ein stilles Bestanden.

use a11y_dom::{
    ComputedStyle, Node, NodeId, NodeKind, Rendering, Tier, ancestors, descendants, elements,
    subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, de_text, pick, tr};
use crate::registry::Meta;
use crate::structure::{explicit_role, per_tab_reachable};
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn page<D: Rendering>(doc: &D) -> Location {
    at(doc.root().id())
}

// --- lists/role-redundant -----------------------------------------------------

/// `role="list"` an `<ul>`/`<ol>`, deren Listenzeichen sichtbar sind. Best
/// Practice ohne Kriterium: Die Rolle stimmt, nur überflüssig (auditmysite#779).
///
/// Mit `list-style-type: none` ist die Rolle nicht überflüssig: WebKit und
/// VoiceOver nehmen einer solchen Liste ihre Semantik, und die ausdrückliche
/// Rolle stellt sie wieder her (auditmysite#644). Deshalb braucht die Regel
/// den berechneten Wert; `aria/role-redundant` (Tier 1) lässt das Paar aus.
pub(crate) fn list_role<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut unmeasured = false;
    for n in elements(doc)
        .filter(|n| matches!(n.local_name(), "ul" | "ol") && explicit_role(*n) == Some("list"))
    {
        match doc.computed_style(n).and_then(|s| s.list_style_type) {
            None => unmeasured = true,
            Some(t) if t.trim().eq_ignore_ascii_case("none") => {}
            Some(_) => out.push(
                Finding::best_practice(
                    "lists/role-redundant",
                    pick!(
                        locale,
                        "role=\"list\" repeats the implicit role of this list; its markers are \
                         visible, so nothing restores the semantics here.",
                        "role=\"list\" wiederholt die implizite Rolle dieser Liste; ihre Zeichen \
                         sind sichtbar, es gibt hier nichts wiederherzustellen.",
                    ),
                )
                .with_severity(Severity::Low)
                .at(at(n.id())),
            ),
        }
    }
    if unmeasured {
        out.push(
            Finding::untested(
                "lists/role-redundant",
                pick!(
                    locale,
                    "The host provides no computed list-style-type; whether role=\"list\" on a \
                     <ul> or <ol> is redundant was not checked.",
                    "Der Host liefert kein berechnetes list-style-type; ob role=\"list\" an einem \
                     <ul> oder <ol> überflüssig ist, wurde nicht geprüft.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["4.1.2"])
            .at(page(doc)),
        );
    }
}

// --- color/link-indistinct ----------------------------------------------------

fn display(style: Option<&ComputedStyle>) -> &str {
    style.and_then(|s| s.display.as_deref()).unwrap_or("")
}

/// Ein Block im Sinne von axe `isInTextBlock`: alles, was nicht `inline`
/// oder `contents` ist. Ohne berechneten Stil entscheidet der Tag.
fn is_block<'a, N: Node<'a>>(n: N, style: Option<&ComputedStyle>) -> bool {
    match style.and_then(|s| s.display.as_deref()) {
        Some(d) => d != "inline" && d != "contents",
        None => !matches!(
            n.local_name(),
            "a" | "abbr"
                | "b"
                | "bdi"
                | "bdo"
                | "cite"
                | "code"
                | "data"
                | "dfn"
                | "em"
                | "i"
                | "kbd"
                | "mark"
                | "q"
                | "s"
                | "samp"
                | "small"
                | "span"
                | "strong"
                | "sub"
                | "sup"
                | "time"
                | "u"
                | "var"
                | "label"
                | "button"
                | "input"
                | "select"
                | "textarea"
                | "img"
                | "svg"
        ),
    }
}

fn is_widget<'a, N: Node<'a>>(n: N) -> bool {
    (n.is_element("a") && n.has_attr("href"))
        || matches!(n.local_name(), "button" | "input" | "select" | "textarea")
        || explicit_role(n).is_some_and(|r| {
            matches!(
                r,
                "link" | "button" | "menuitem" | "tab" | "checkbox" | "radio" | "switch" | "option"
            )
        })
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text des Blockabschnitts um den Link, getrennt in Widget-Text (der Link
/// selbst, weitere Links und Bedienelemente) und übrigen Text. Ein `<br>` oder
/// `<hr>` vor dem Link beginnt den Abschnitt neu, eines danach beendet ihn;
/// verschachtelte Blöcke sind eigene Zeilen.
fn block_texts<'d, D: Rendering>(doc: &'d D, link: D::N<'d>) -> Option<(String, String)> {
    let block = ancestors(link).find(|b| {
        let style = doc.computed_style(*b);
        b.is_element("body") || is_block(*b, style.as_ref())
    })?;
    let (mut other, mut widget) = (String::new(), String::new());
    // 0 vor dem Link, 1 danach, 2 Abschnitt zu Ende.
    let mut state = 0u8;
    fn walk<'d, D: Rendering>(
        doc: &'d D,
        n: D::N<'d>,
        link_id: NodeId,
        other: &mut String,
        widget: &mut String,
        state: &mut u8,
    ) {
        for kind in n.children() {
            if *state == 2 {
                return;
            }
            if kind.kind() == NodeKind::Text {
                other.push_str(kind.text());
                continue;
            }
            match kind.local_name() {
                "br" | "hr" => {
                    if *state == 0 {
                        other.clear();
                        widget.clear();
                    } else {
                        *state = 2;
                    }
                    continue;
                }
                "script" | "style" | "template" | "noscript" => continue,
                _ => {}
            }
            if kind.id() == link_id {
                *state = 1;
                widget.push(' ');
                widget.push_str(&subtree_text(kind));
                continue;
            }
            let style = doc.computed_style(kind);
            if style.as_ref().is_some_and(|s| {
                s.display.as_deref() == Some("none")
                    || matches!(s.visibility.as_deref(), Some("hidden" | "collapse"))
            }) {
                continue;
            }
            if is_widget(kind) {
                widget.push(' ');
                widget.push_str(&subtree_text(kind));
                continue;
            }
            if is_block(kind, style.as_ref()) {
                continue;
            }
            walk(doc, kind, link_id, other, widget, state);
        }
    }
    walk(doc, block, link.id(), &mut other, &mut widget, &mut state);
    Some((clean(&other), clean(&widget)))
}

/// Fließtext nach auditmysite#710: mindestens zwei Wörter neben den Links,
/// und mehr übriger Text als Widget-Text. Trenner wie `|` oder `·` zwischen
/// Fußzeilen-Links sind kein Fließtext.
fn in_running_text(other: &str, widget: &str) -> bool {
    let words = other
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().any(char::is_alphabetic))
        .count();
    words >= 2 && other.chars().count() > widget.chars().count()
}

fn in_navigation<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| {
        a.is_element("nav")
            || explicit_role(a).is_some_and(|r| matches!(r, "navigation" | "menubar" | "menu"))
    })
}

fn is_logo<'a, N: Node<'a>>(n: N) -> bool {
    descendants(n).any(|d| matches!(d.local_name(), "img" | "svg"))
        || ancestors(n).chain(std::iter::once(n)).any(|a| {
            ["class", "id"].iter().any(|k| {
                a.attr(k)
                    .is_some_and(|v| v.to_ascii_lowercase().contains("logo"))
            })
        })
}

/// Die Stilwerte, die ein Link und sein Elternelement zum Vergleich brauchen.
/// `None`, sobald der Host eines nicht liefert.
fn comparison_values(s: &ComputedStyle) -> Option<[String; 5]> {
    Some([
        s.font_weight?.to_string(),
        s.font_style.clone()?,
        s.font_family.clone()?,
        s.border_bottom_style.clone()?,
        s.background_color
            .map(|c| format!("{},{},{},{}", c.r, c.g, c.b, c.a))
            .unwrap_or_default(),
    ])
}

/// 1.4.1: Ein Link im Fließtext, der sich vom umgebenden Text nur durch
/// die Farbe unterscheidet — ohne Unterstreichung, ohne anderes Gewicht,
/// anderen Schnitt, andere Schrift, Unterkante oder Hintergrund.
///
/// Wie axe `link-in-text-block` und auditmysite#710 zählen nur Links im
/// Fließtext; Navigation, Menüs, Logo-Links und Links, die als Block, Flex-
/// oder Grid-Element stehen, sind durch ihre Lage unterscheidbar. Ein Link
/// ist ein `a[href]` oder ein Element mit der Rolle `link` (F73 gilt für
/// jeden Link).
///
/// Erreicht die Linkfarbe 3:1 zum umgebenden Text, kann G183 erfüllt sein
/// (dazu ein Hinweis bei Hover und Fokus, den nur die Stylesheets zeigen):
/// dann `REVIEW` statt `FAIL`.
pub(crate) fn link_color_only<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut unmeasured = false;
    // Ob der Host überhaupt Stile liefert, zeigt der `<body>` (ohne ihn die
    // Wurzel). Liefert er welche, hat ein Link ohne Stil keinen Kasten.
    let with_styles = elements(doc).find(|n| n.is_element("body")).map_or_else(
        || doc.computed_style(doc.root()).is_some(),
        |b| doc.computed_style(b).is_some(),
    );
    let is_link = |n: &D::N<'_>| match explicit_role(*n) {
        Some(r) => r == "link",
        None => n.is_element("a") && n.has_attr("href"),
    };
    for link in elements(doc).filter(is_link) {
        let Some(parent) = link.parent().filter(|p| p.kind() == NodeKind::Element) else {
            continue;
        };
        let children: Vec<_> = link
            .children()
            .filter(|k| k.kind() == NodeKind::Element)
            .collect();
        if children.len() == 1 && matches!(children[0].local_name(), "img" | "svg" | "i") {
            continue;
        }
        if subtree_text(link).trim().is_empty() {
            continue;
        }
        let (style, parent_style) = (doc.computed_style(link), doc.computed_style(parent));
        if matches!(
            display(style.as_ref()),
            "block" | "flex" | "grid" | "inline-flex" | "inline-grid"
        ) || matches!(
            display(parent_style.as_ref()),
            "flex" | "grid" | "inline-flex" | "inline-grid"
        ) {
            continue;
        }
        if in_navigation(link) || is_logo(link) {
            continue;
        }
        // Ohne Kasten (`display: none`, so meldet ein Host aus einem
        // Layout-Schnappschuss etwa den Ersatzinhalt eines geladenen
        // `<object>`) unterscheidet sich der Link von nichts.
        if display(style.as_ref()) == "none" {
            continue;
        }
        let (style, parent_style) = match (style, parent_style) {
            (Some(s), Some(e)) => (s, e),
            // Ohne eigenen Stil bei einem Host, der Stile liefert, wird der
            // Link nicht dargestellt (etwa der Ersatzinhalt eines geladenen
            // `<object>`, dessen Element selbst auch keinen Stil hat): Er
            // unterscheidet sich von nichts, ungemessen ist er nicht.
            (None, _) if with_styles => continue,
            _ => {
                unmeasured = true;
                continue;
            }
        };
        let (Some(underline), Some(own), Some(foreign)) = (
            style.text_decoration_line.as_deref(),
            comparison_values(&style),
            comparison_values(&parent_style),
        ) else {
            unmeasured = true;
            continue;
        };
        if underline.to_ascii_lowercase().contains("underline") || own != foreign {
            continue;
        }
        let Some((other, widget)) = block_texts(doc, link) else {
            continue;
        };
        if !in_running_text(&other, &widget) {
            continue;
        }
        let color_distance = match (
            style.color,
            parent_style.color,
            parent_style.background_color,
        ) {
            (Some(a), Some(b), Some(h)) => Some(crate::rendering::ratio(
                crate::rendering::over(a, h),
                crate::rendering::over(b, h),
            )),
            _ => None,
        };
        let finding = if color_distance.is_some_and(|v| v >= 3.0) {
            Finding::review(
                "color/link-indistinct",
                pick!(
                    locale,
                    "This link in running text differs from the surrounding text only by \
                     colour, with at least 3:1 contrast to it. Check that hover and focus add \
                     a cue other than colour (G183).",
                    "Dieser Link im Fließtext unterscheidet sich vom umgebenden Text nur durch \
                     die Farbe, mit mindestens 3:1 dazu. Prüfen, ob Hover und Fokus einen \
                     Hinweis außer der Farbe ergänzen (G183).",
                ),
            )
        } else {
            Finding::fail(
                "color/link-indistinct",
                pick!(
                    locale,
                    "This link in running text differs from the surrounding text only by \
                     colour: no underline, same weight, style, font, border and background.",
                    "Dieser Link im Fließtext unterscheidet sich vom umgebenden Text nur durch \
                     die Farbe: keine Unterstreichung, gleiches Gewicht, gleicher Schnitt, \
                     gleiche Schrift, Unterkante und Hintergrund.",
                ),
            )
        };
        out.push(
            finding
                .with_severity(Severity::Medium)
                .with_wcag(["1.4.1"])
                .at(at(link.id())),
        );
    }
    if unmeasured {
        out.push(
            Finding::untested(
                "color/link-indistinct",
                pick!(
                    locale,
                    "The host provides no text decoration, font style, font family or border \
                     style; whether links in running text differ only by colour was not checked.",
                    "Der Host liefert keine Textauszeichnung, keinen Schriftschnitt, keine \
                     Schriftfamilie oder Unterkante; ob Links im Fließtext sich nur durch die \
                     Farbe abheben, wurde nicht geprüft.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.4.1"])
            .at(page(doc)),
        );
    }
}

// --- keyboard/scrollable-region-not-focusable ---------------------------------

/// Überhang, ab dem ein Kasten als scrollbar gilt. Subpixel-Layout und kleine
/// Innenabstände lassen `scrollHeight` um ein paar Pixel über `clientHeight`
/// hinausgehen, ohne dass es etwas zu scrollen gäbe; axe nimmt denselben
/// Puffer.
const SCROLL_BUFFER_PX: f32 = 13.0;

fn inert<'a, N: Node<'a>>(n: N) -> bool {
    std::iter::once(n)
        .chain(ancestors(n))
        .any(|a| a.has_attr("inert"))
}

/// 2.1.1: Ein Bereich, dessen Inhalt scrollt, muss per Tastatur erreichbar
/// sein — selbst (`tabindex="0"`) oder über ein fokussierbares Element darin.
/// Sonst bleibt der verdeckte Teil für Tastaturnutzer unerreichbar.
///
/// Wie auditmysite#717 (gov.si, `div.menus`). Der Seiten-Scroller selbst
/// (`html`, `body`) scrollt immer per Tastatur. Ein Bereich ohne Text zählt
/// nicht: Scrollen würde dort nichts zeigen.
pub(crate) fn scroll_area<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Ob der Host misst, zeigt auch das Dokument selbst: Eine leere Seite hat
    // keinen Kandidaten, ist deshalb aber nicht ungemessen.
    let mut measured = elements(doc)
        .filter(|n| matches!(n.local_name(), "html" | "body"))
        .any(|n| doc.scroll_overflow_px(n).is_some());
    for n in elements(doc).filter(|n| !matches!(n.local_name(), "html" | "body")) {
        let Some(overhang) = doc.scroll_overflow_px(n) else {
            continue;
        };
        measured = true;
        if overhang <= SCROLL_BUFFER_PX
            || subtree_text(n).trim().is_empty()
            || inert(n)
            || per_tab_reachable(n)
            || descendants(n).any(|d| per_tab_reachable(d) && !inert(d))
        {
            continue;
        }
        out.push(
            Finding::fail(
                "keyboard/scrollable-region-not-focusable",
                tr!(
                    locale,
                    "This region scrolls ({overhang:.0}px of content beyond its box) but neither \
                     it nor anything inside it can take keyboard focus.",
                    "Dieser Bereich scrollt ({overhang:.0}px Inhalt über den Kasten hinaus), \
                     aber weder er noch etwas darin kann den Tastaturfokus bekommen."
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.1.1"])
            .at(at(n.id())),
        );
    }
    if !measured {
        out.push(
            Finding::untested(
                "keyboard/scrollable-region-not-focusable",
                pick!(
                    locale,
                    "The host measures no scroll extents; scrollable regions without keyboard \
                     access were not checked.",
                    "Der Host misst keinen Scroll-Überhang; scrollbare Bereiche ohne \
                     Tastaturzugang wurden nicht geprüft.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.1.1"])
            .at(page(doc)),
        );
    }
}

// --- Registrierung ------------------------------------------------------------

pub(crate) const METAS: [Meta; 3] = [
    Meta {
        ids: &["lists/role-redundant"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &[],
        severity: Severity::Low,
        help: "role=\"list\" on a <ul> or <ol> is only needed when list-style-type is none.",
        help_de: de_text!(
            "role=\"list\" an <ul> oder <ol> braucht es nur bei list-style-type: none."
        ),
    },
    Meta {
        ids: &["color/link-indistinct"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &["1.4.1"],
        severity: Severity::Medium,
        help: "A link in running text needs more than colour to stand out: an underline, or \
               a different weight, style, border or background.",
        help_de: de_text!(
            "Ein Link im Fließtext braucht mehr als Farbe, um sich abzuheben: eine \
                  Unterstreichung oder ein anderes Gewicht, einen anderen Schnitt, eine \
                  Unterkante oder einen Hintergrund."
        ),
    },
    Meta {
        ids: &["keyboard/scrollable-region-not-focusable"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &["2.1.1"],
        severity: Severity::High,
        help: "A scrollable region needs keyboard access: tabindex=\"0\" on the region or a \
               focusable element inside it.",
        help_de: de_text!(
            "Ein scrollbarer Bereich braucht Tastaturzugang: tabindex=\"0\" am Bereich oder \
                  ein fokussierbares Element darin."
        ),
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn functions<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 3] {
    [list_role, link_color_only, scroll_area]
}
