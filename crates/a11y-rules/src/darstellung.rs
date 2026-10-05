//! Darstellungsregeln mit gemessenen Werten (casoon/barrierlab#21, zweiter
//! Teil): überflüssiges `role="list"` (4.1.2), Links, die sich nur durch
//! Farbe abheben (1.4.1), und scrollbare Bereiche ohne Tastaturzugang
//! (2.1.1).
//!
//! Portiert aus auditmysite (`redundant_role` für `<ul>`/`<ol>`,
//! `use_of_color`, `scrollable_region`). Anders als die Heuristiken in
//! `heuristik.rs` urteilen diese Regeln mit `FAIL`: Was sie brauchen — das
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

use crate::locale::{Locale, pick, tr};
use crate::registry::Meta;
use crate::sicht::Scope;
use crate::structure::{explizite_rolle, per_tab_erreichbar};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn seite<D: Rendering>(doc: &D) -> Location {
    at(doc.root().id())
}

// --- lists/role-redundant -----------------------------------------------------

/// 4.1.2: `role="list"` an `<ul>`/`<ol>`, deren Listenzeichen sichtbar sind.
///
/// Mit `list-style-type: none` ist die Rolle nicht überflüssig: WebKit und
/// VoiceOver nehmen einer solchen Liste ihre Semantik, und die ausdrückliche
/// Rolle stellt sie wieder her (auditmysite#644). Deshalb braucht die Regel
/// den berechneten Wert; `aria/role-redundant` (Tier 1) lässt das Paar aus.
pub(crate) fn listenrolle<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut ungemessen = false;
    for n in elements(doc)
        .filter(|n| matches!(n.local_name(), "ul" | "ol") && explizite_rolle(*n) == Some("list"))
    {
        match doc.computed_style(n).and_then(|s| s.list_style_type) {
            None => ungemessen = true,
            Some(t) if t.trim().eq_ignore_ascii_case("none") => {}
            Some(_) => out.push(
                Finding::fail(
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
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            ),
        }
    }
    if ungemessen {
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
            .at(seite(doc)),
        );
    }
}

// --- color/link-indistinct ----------------------------------------------------

fn anzeige(stil: Option<&ComputedStyle>) -> &str {
    stil.and_then(|s| s.display.as_deref()).unwrap_or("")
}

/// Ein Block im Sinne von axe `isInTextBlock`: alles, was nicht `inline`
/// oder `contents` ist. Ohne berechneten Stil entscheidet der Tag.
fn ist_block<'a, N: Node<'a>>(n: N, stil: Option<&ComputedStyle>) -> bool {
    match stil.and_then(|s| s.display.as_deref()) {
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

fn ist_widget<'a, N: Node<'a>>(n: N) -> bool {
    (n.is_element("a") && n.has_attr("href"))
        || matches!(n.local_name(), "button" | "input" | "select" | "textarea")
        || explizite_rolle(n).is_some_and(|r| {
            matches!(
                r,
                "link" | "button" | "menuitem" | "tab" | "checkbox" | "radio" | "switch" | "option"
            )
        })
}

fn saeubern(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text des Blockabschnitts um den Link, getrennt in Widget-Text (der Link
/// selbst, weitere Links und Bedienelemente) und übrigen Text. Ein `<br>` oder
/// `<hr>` vor dem Link beginnt den Abschnitt neu, eines danach beendet ihn;
/// verschachtelte Blöcke sind eigene Zeilen.
fn blocktexte<'d, D: Rendering>(doc: &'d D, link: D::N<'d>) -> Option<(String, String)> {
    let block = ancestors(link).find(|b| {
        let stil = doc.computed_style(*b);
        b.is_element("body") || ist_block(*b, stil.as_ref())
    })?;
    let (mut andere, mut widget) = (String::new(), String::new());
    // 0 vor dem Link, 1 danach, 2 Abschnitt zu Ende.
    let mut zustand = 0u8;
    fn gehe<'d, D: Rendering>(
        doc: &'d D,
        n: D::N<'d>,
        link_id: NodeId,
        andere: &mut String,
        widget: &mut String,
        zustand: &mut u8,
    ) {
        for kind in n.children() {
            if *zustand == 2 {
                return;
            }
            if kind.kind() == NodeKind::Text {
                andere.push_str(kind.text());
                continue;
            }
            match kind.local_name() {
                "br" | "hr" => {
                    if *zustand == 0 {
                        andere.clear();
                        widget.clear();
                    } else {
                        *zustand = 2;
                    }
                    continue;
                }
                "script" | "style" | "template" | "noscript" => continue,
                _ => {}
            }
            if kind.id() == link_id {
                *zustand = 1;
                widget.push(' ');
                widget.push_str(&subtree_text(kind));
                continue;
            }
            let stil = doc.computed_style(kind);
            if stil.as_ref().is_some_and(|s| {
                s.display.as_deref() == Some("none")
                    || matches!(s.visibility.as_deref(), Some("hidden" | "collapse"))
            }) {
                continue;
            }
            if ist_widget(kind) {
                widget.push(' ');
                widget.push_str(&subtree_text(kind));
                continue;
            }
            if ist_block(kind, stil.as_ref()) {
                continue;
            }
            gehe(doc, kind, link_id, andere, widget, zustand);
        }
    }
    gehe(
        doc,
        block,
        link.id(),
        &mut andere,
        &mut widget,
        &mut zustand,
    );
    Some((saeubern(&andere), saeubern(&widget)))
}

/// Fließtext nach auditmysite#710: mindestens zwei Wörter neben den Links,
/// und mehr übriger Text als Widget-Text. Trenner wie `|` oder `·` zwischen
/// Fußzeilen-Links sind kein Fließtext.
fn im_fliesstext(andere: &str, widget: &str) -> bool {
    let woerter = andere
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().any(char::is_alphabetic))
        .count();
    woerter >= 2 && andere.chars().count() > widget.chars().count()
}

fn in_navigation<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| {
        a.is_element("nav")
            || explizite_rolle(a).is_some_and(|r| matches!(r, "navigation" | "menubar" | "menu"))
    })
}

fn ist_logo<'a, N: Node<'a>>(n: N) -> bool {
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
fn vergleichswerte(s: &ComputedStyle) -> Option<[String; 5]> {
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
/// oder Grid-Element stehen, sind durch ihre Lage unterscheidbar. Ob die
/// Farbe selbst genug Kontrast zum Text hat (3:1), prüft diese Regel nicht.
pub(crate) fn link_nur_farbe<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut ungemessen = false;
    for link in elements(doc).filter(|n| n.is_element("a") && n.has_attr("href")) {
        let Some(eltern) = link.parent().filter(|p| p.kind() == NodeKind::Element) else {
            continue;
        };
        let kinder: Vec<_> = link
            .children()
            .filter(|k| k.kind() == NodeKind::Element)
            .collect();
        if kinder.len() == 1 && matches!(kinder[0].local_name(), "img" | "svg" | "i") {
            continue;
        }
        if subtree_text(link).trim().is_empty() {
            continue;
        }
        let (stil, eltern_stil) = (doc.computed_style(link), doc.computed_style(eltern));
        if matches!(
            anzeige(stil.as_ref()),
            "block" | "flex" | "grid" | "inline-flex" | "inline-grid"
        ) || matches!(
            anzeige(eltern_stil.as_ref()),
            "flex" | "grid" | "inline-flex" | "inline-grid"
        ) {
            continue;
        }
        if in_navigation(link) || ist_logo(link) {
            continue;
        }
        let (Some(stil), Some(eltern_stil)) = (stil, eltern_stil) else {
            ungemessen = true;
            continue;
        };
        let (Some(unterstreichung), Some(eigene), Some(fremde)) = (
            stil.text_decoration_line.as_deref(),
            vergleichswerte(&stil),
            vergleichswerte(&eltern_stil),
        ) else {
            ungemessen = true;
            continue;
        };
        if unterstreichung.to_ascii_lowercase().contains("underline") || eigene != fremde {
            continue;
        }
        let Some((andere, widget)) = blocktexte(doc, link) else {
            continue;
        };
        if !im_fliesstext(&andere, &widget) {
            continue;
        }
        out.push(
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
            .with_severity(Severity::Medium)
            .with_wcag(["1.4.1"])
            .at(at(link.id())),
        );
    }
    if ungemessen {
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
            .at(seite(doc)),
        );
    }
}

// --- keyboard/scrollable-region-not-focusable ---------------------------------

/// Überhang, ab dem ein Kasten als scrollbar gilt. Subpixel-Layout und kleine
/// Innenabstände lassen `scrollHeight` um ein paar Pixel über `clientHeight`
/// hinausgehen, ohne dass es etwas zu scrollen gäbe; axe nimmt denselben
/// Puffer.
const SCROLL_PUFFER_PX: f32 = 13.0;

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
pub(crate) fn scrollbereich<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut gemessen = false;
    for n in elements(doc).filter(|n| !matches!(n.local_name(), "html" | "body")) {
        let Some(ueberhang) = doc.layout(n).and_then(|l| l.scroll_overflow_px) else {
            continue;
        };
        gemessen = true;
        if ueberhang <= SCROLL_PUFFER_PX
            || subtree_text(n).trim().is_empty()
            || inert(n)
            || per_tab_erreichbar(n)
            || descendants(n).any(|d| per_tab_erreichbar(d) && !inert(d))
        {
            continue;
        }
        out.push(
            Finding::fail(
                "keyboard/scrollable-region-not-focusable",
                tr!(
                    locale,
                    "This region scrolls ({ueberhang:.0}px of content beyond its box) but neither \
                     it nor anything inside it can take keyboard focus.",
                    "Dieser Bereich scrollt ({ueberhang:.0}px Inhalt über den Kasten hinaus), \
                     aber weder er noch etwas darin kann den Tastaturfokus bekommen."
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["2.1.1"])
            .at(at(n.id())),
        );
    }
    if !gemessen {
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
            .at(seite(doc)),
        );
    }
}

// --- Registrierung ------------------------------------------------------------

pub(crate) const METAS: [Meta; 3] = [
    Meta {
        ids: &["lists/role-redundant"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Low,
        help: "role=\"list\" on a <ul> or <ol> is only needed when list-style-type is none.",
        #[cfg(feature = "de")]
        help_de: "role=\"list\" an <ul> oder <ol> braucht es nur bei list-style-type: none.",
    },
    Meta {
        ids: &["color/link-indistinct"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &["1.4.1"],
        severity: Severity::Medium,
        help: "A link in running text needs more than colour to stand out: an underline, or \
               a different weight, style, border or background.",
        #[cfg(feature = "de")]
        help_de: "Ein Link im Fließtext braucht mehr als Farbe, um sich abzuheben: eine \
                  Unterstreichung oder ein anderes Gewicht, einen anderen Schnitt, eine \
                  Unterkante oder einen Hintergrund.",
    },
    Meta {
        ids: &["keyboard/scrollable-region-not-focusable"],
        tier: Tier::Rendering,
        scope: Scope::AccessibilityTree,
        wcag: &["2.1.1"],
        severity: Severity::High,
        help: "A scrollable region needs keyboard access: tabindex=\"0\" on the region or a \
               focusable element inside it.",
        #[cfg(feature = "de")]
        help_de: "Ein scrollbarer Bereich braucht Tastaturzugang: tabindex=\"0\" am Bereich oder \
                  ein fokussierbares Element darin.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn funktionen<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 3] {
    [listenrolle, link_nur_farbe, scrollbereich]
}
