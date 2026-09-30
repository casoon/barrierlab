//! ARIA-Attribute und -Beziehungen nach WAI-ARIA 1.2 und ARIA in HTML.
//!
//! Portiert aus auditmysite (`aria_allowed_attr`, `aria_prohibited_attr`,
//! `aria_valid_attr_value`, `aria_required_parent`, `aria_roles`, `parsing`,
//! `widget_rules`, `modern_attributes`; casoon/barrierlab#14). Norm ist die
//! Spezifikation, auditmysite der Vergleichspunkt — Abweichungen stehen an der
//! jeweiligen Stelle und im CHANGELOG.
//!
//! Die Tabellen sind aus den Rollen- und Attributdefinitionen von WAI-ARIA 1.2
//! (<https://www.w3.org/TR/wai-aria-1.2/>) erzeugt: je Rolle eine Bitmaske über
//! [`ATTRIBUTE`], kein Vec je Rolle — das Crate läuft auch als WASM, und dort
//! zählt jedes Kilobyte.

use a11y_dom::{Document, Node, NodeId, Semantics, ancestors, descendants, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::{VALID_ROLES, explizite_rolle};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- Tabellen -------------------------------------------------------------

/// Alle Zustände und Eigenschaften aus WAI-ARIA 1.2, alphabetisch. Der Index
/// ist die Bitposition in [`GLOBAL`] und [`JE_ROLLE`].
const ATTRIBUTE: [&str; 48] = [
    "aria-activedescendant",
    "aria-atomic",
    "aria-autocomplete",
    "aria-busy",
    "aria-checked",
    "aria-colcount",
    "aria-colindex",
    "aria-colspan",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-details",
    "aria-disabled",
    "aria-dropeffect",
    "aria-errormessage",
    "aria-expanded",
    "aria-flowto",
    "aria-grabbed",
    "aria-haspopup",
    "aria-hidden",
    "aria-invalid",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-level",
    "aria-live",
    "aria-modal",
    "aria-multiline",
    "aria-multiselectable",
    "aria-orientation",
    "aria-owns",
    "aria-placeholder",
    "aria-posinset",
    "aria-pressed",
    "aria-readonly",
    "aria-relevant",
    "aria-required",
    "aria-roledescription",
    "aria-rowcount",
    "aria-rowindex",
    "aria-rowspan",
    "aria-selected",
    "aria-setsize",
    "aria-sort",
    "aria-valuemax",
    "aria-valuemin",
    "aria-valuenow",
    "aria-valuetext",
];

/// Was jede Rolle unterstützt (die Schnittmenge aller konkreten Rollen).
const GLOBAL: u64 = 0x0008423f7f0a;

/// Für Rollen, die WAI-ARIA 1.2 nicht kennt: kein Urteil.
const UNBEKANNT: u64 = u64::MAX;

/// Was eine Rolle über [`GLOBAL`] hinaus unterstützt — eigene, geerbte und
/// erforderliche Attribute, in derselben Reihenfolge wie [`VALID_ROLES`].
/// In ARIA 1.2 auf einer Rolle als veraltet markierte Attribute zählen als
/// unterstützt: veraltet ist nicht verboten.
const JE_ROLLE: [u64; 83] = [
    0x002000c00000, // alert
    0x002004c00000, // alertdialog
    0x002000c08001, // application
    0x042100c00000, // article
    0x002000c00000, // banner
    0x002000c00000, // blockquote
    0x002200c08000, // button
    0x002000000000, // caption
    0x01a000c000c0, // cell
    0x003400c08010, // checkbox
    0x002000000000, // code
    0x0bb400c080c0, // columnheader
    0x003400c08005, // combobox
    0x002000c00000, // complementary
    0x002000c00000, // contentinfo
    0x002000c00000, // definition
    0x002000000000, // deletion
    0x002004c00000, // dialog
    0x002000c00000, // directory
    0x002000c00000, // document
    0x002000000000, // emphasis
    0x002000c00000, // feed
    0x002000c00000, // figure
    0x002000c00000, // form
    0x000000000000, // generic
    0x006410c00021, // grid
    0x03b400c080c0, // gridcell
    0x002000c00001, // group
    0x002001c00000, // heading
    0x002000c00000, // img
    0x002000000000, // insertion
    0x002000c08000, // link
    0x002000c00000, // list
    0x003430c08001, // listbox
    0x042101c00000, // listitem
    0x002000c00000, // log
    0x002000c00000, // main
    UNBEKANNT,      // mark (ARIA 1.3, nicht in 1.2)
    0x002000c00000, // marquee
    0x002000c00000, // math
    0x002020c00001, // menu
    0x002020c00001, // menubar
    0x042100c08000, // menuitem
    0x042100c08010, // menuitemcheckbox
    0x042100c08010, // menuitemradio
    0xf02000c00000, // meter
    0x002000c00000, // navigation
    0x002000000000, // none
    0x002000c00000, // note
    0x062100c00010, // option
    0x002000000000, // paragraph
    0x002000000000, // presentation
    0xf02000c00000, // progressbar
    0x042100c00010, // radio
    0x003420c00001, // radiogroup
    0x002000c00000, // region
    0x06a101c08041, // row
    0x002000c00000, // rowgroup
    0x0bb400c080c0, // rowheader
    0xf02020c00000, // scrollbar
    0x002000c00000, // search
    0x003488c00005, // searchbox
    0xf02020c00000, // separator
    0xf02420c00000, // slider
    0xf03400c00001, // spinbutton
    0x002000c00000, // status
    0x002000000000, // strong
    0x002000000000, // subscript
    0x002000000000, // superscript
    0x003400c08010, // switch
    0x062100c08000, // tab
    0x006000c00020, // table
    0x002030c00001, // tablist
    0x002000c00000, // tabpanel
    0x002000c00000, // term
    0x003488c00005, // textbox
    0x002000c00000, // time
    0x002000c00000, // timer
    0x002020c00001, // toolbar
    0x002000c00000, // tooltip
    0x003030c00001, // tree
    0x007430c00021, // treegrid
    0x062101c08010, // treeitem
];

const _: () = assert!(JE_ROLLE.len() == VALID_ROLES.len());

/// Attribute aus dem Entwurf von WAI-ARIA 1.3, die Browser schon umsetzen.
/// Sie sind kein Tippfehler; über ihre Zulässigkeit urteilt diese Fassung
/// nicht.
const ARIA_13: &[&str] = &[
    "aria-braillelabel",
    "aria-brailleroledescription",
    "aria-colindextext",
    "aria-description",
    "aria-rowindextext",
];

fn ist_aria(name: &str) -> bool {
    name.get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("aria-"))
}

fn attribut_index(name: &str) -> Option<usize> {
    ATTRIBUTE.iter().position(|a| a.eq_ignore_ascii_case(name))
}

fn rollen_index(rolle: &str) -> Option<usize> {
    VALID_ROLES.iter().position(|r| *r == rolle)
}

// --- aria/attribute-unknown -----------------------------------------------

/// Ein `aria-*`-Attribut, das es nicht gibt — meist ein Tippfehler
/// (`aria-labeledby`). Der Browser ignoriert es, die gemeinte Beziehung oder
/// der gemeinte Zustand fehlt. Beleg: auditmysite-Korpus
/// `aria_attribute_validation` (`aria-nonexistentattr`).
pub(crate) fn attribute_names<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        for (name, _) in n.attributes() {
            if !ist_aria(name)
                || attribut_index(name).is_some()
                || ARIA_13.iter().any(|a| a.eq_ignore_ascii_case(name))
            {
                continue;
            }
            out.push(
                Finding::fail(
                    "aria/attribute-unknown",
                    tr!(
                        locale,
                        "\"{name}\" is not an ARIA attribute.",
                        "\"{name}\" ist kein ARIA-Attribut."
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- aria/attribute-value-invalid -----------------------------------------

enum Werttyp {
    /// Genau einer der Werte.
    Eins(&'static [&'static str]),
    /// Eine Liste aus diesen Werten, durch Leerraum getrennt.
    Liste(&'static [&'static str]),
    Ganzzahl,
    Zahl,
}

const WAHR_FALSCH: &[&str] = &["true", "false"];
const WAHR_FALSCH_UNDEF: &[&str] = &["true", "false", "undefined"];
const TRISTATE: &[&str] = &["true", "false", "mixed", "undefined"];

/// Der Werttyp nach WAI-ARIA 1.2, Abschnitt 6.6/6.7.
///
/// Nicht geprüft: IDREFs (das ist `aria/reference-missing`), freie Zeichenketten
/// und `aria-current`/`aria-invalid` — für diese beiden legt die Spezifikation
/// fest, dass ein unbekannter Wert als `true` gilt; der Wert tut damit, was
/// gemeint ist. auditmysite meldet sie; hier bewusst nicht.
fn werttyp(attr: &str) -> Option<Werttyp> {
    use Werttyp::*;
    Some(match attr {
        "aria-atomic"
        | "aria-busy"
        | "aria-disabled"
        | "aria-modal"
        | "aria-multiline"
        | "aria-multiselectable"
        | "aria-readonly"
        | "aria-required" => Eins(WAHR_FALSCH),
        "aria-expanded" | "aria-grabbed" | "aria-hidden" | "aria-selected" => {
            Eins(WAHR_FALSCH_UNDEF)
        }
        "aria-checked" | "aria-pressed" => Eins(TRISTATE),
        "aria-autocomplete" => Eins(&["inline", "list", "both", "none"]),
        "aria-haspopup" => Eins(&["false", "true", "menu", "listbox", "tree", "grid", "dialog"]),
        "aria-live" => Eins(&["assertive", "off", "polite"]),
        "aria-orientation" => Eins(&["horizontal", "vertical", "undefined"]),
        "aria-sort" => Eins(&["ascending", "descending", "none", "other"]),
        "aria-dropeffect" => Liste(&["copy", "execute", "link", "move", "none", "popup"]),
        "aria-relevant" => Liste(&["additions", "removals", "text", "all"]),
        "aria-colcount" | "aria-colindex" | "aria-colspan" | "aria-level" | "aria-posinset"
        | "aria-rowcount" | "aria-rowindex" | "aria-rowspan" | "aria-setsize" => Ganzzahl,
        "aria-valuemax" | "aria-valuemin" | "aria-valuenow" => Zahl,
        _ => return None,
    })
}

fn einer_von(liste: &[&str], v: &str) -> bool {
    liste.iter().any(|t| t.eq_ignore_ascii_case(v))
}

/// Eine Dezimalzahl nach dem Muster `-1`, `2.5`, `.5`, `1e3` — ohne
/// Gleitkomma-Parser, der im WASM-Build mehrere Kilobyte kostet.
fn ist_zahl(v: &str) -> bool {
    let v = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (mantisse, exponent) = match v.find(['e', 'E']) {
        Some(i) => (&v[..i], Some(&v[i + 1..])),
        None => (v, None),
    };
    let (ganz, bruch) = mantisse.split_once('.').unwrap_or((mantisse, ""));
    let ziffern = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    (!ganz.is_empty() || !bruch.is_empty())
        && ziffern(ganz)
        && ziffern(bruch)
        && exponent.is_none_or(|e| {
            let e = e.strip_prefix(['+', '-']).unwrap_or(e);
            !e.is_empty() && ziffern(e)
        })
}

fn wert_gueltig(typ: &Werttyp, v: &str) -> bool {
    match typ {
        Werttyp::Eins(l) => einer_von(l, v),
        Werttyp::Liste(l) => v.split_whitespace().all(|t| einer_von(l, t)),
        Werttyp::Ganzzahl => v.parse::<i64>().is_ok(),
        Werttyp::Zahl => ist_zahl(v),
    }
}

/// Ein Wert außerhalb des Wertebereichs. Browser fallen dann auf die Vorgabe
/// zurück — `aria-expanded="yes"` wird zu „nicht aufklappbar", der Zustand ist
/// verloren. Ein leerer Wert gilt nach ARIA wie ein fehlendes Attribut und
/// wird nicht gemeldet. Beleg: auditmysite-Korpus `aria_invalid_attr_value`
/// (`aria-expanded="maybe"`).
pub(crate) fn attribute_values<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        for (name, wert) in n.attributes() {
            let Some(typ) = attribut_index(name).and_then(|i| werttyp(ATTRIBUTE[i])) else {
                continue;
            };
            let v = wert.trim();
            if v.is_empty() || wert_gueltig(&typ, v) {
                continue;
            }
            let erwartet = match typ {
                Werttyp::Eins(l) | Werttyp::Liste(l) => l.join(", "),
                Werttyp::Ganzzahl => pick!(locale, "an integer", "eine ganze Zahl").to_string(),
                Werttyp::Zahl => pick!(locale, "a number", "eine Zahl").to_string(),
            };
            out.push(
                Finding::fail(
                    "aria/attribute-value-invalid",
                    tr!(
                        locale,
                        "{name}=\"{v}\" is not a valid value; expected: {erwartet}.",
                        "{name}=\"{v}\" ist kein gültiger Wert; erwartet: {erwartet}."
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- aria/owns-conflict ---------------------------------------------------

/// Dieselbe ID in mehreren `aria-owns`. WAI-ARIA 1.2 (`aria-owns`): „An
/// element's ID MUST NOT be specified in more than one other element's
/// aria-owns attribute at any time." Welches Element der Besitzer ist, bleibt
/// dann offen. Der Befund zeigt auf den zweiten Besitzer. Beleg: auditmysite
/// `parsing` (`duplicate-id-aria`), der Combobox-Fall mit zwei Besitzern
/// derselben Liste.
pub(crate) fn owns_conflict<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    // Vec statt HashMap: aria-owns ist selten, und jede weitere
    // HashMap-Ausprägung kostet im WASM-Build Platz.
    let mut erster: Vec<(&str, NodeId)> = Vec::new();
    let mut gemeldet: Vec<&str> = Vec::new();
    for n in elements(doc) {
        let Some(v) = n.attr("aria-owns") else {
            continue;
        };
        for id in v.split_whitespace() {
            match erster.iter().find(|(i, _)| *i == id) {
                None => erster.push((id, n.id())),
                Some((_, e)) if *e != n.id() && !gemeldet.contains(&id) => {
                    gemeldet.push(id);
                    out.push(
                    Finding::fail(
                        "aria/owns-conflict",
                        tr!(
                            locale,
                            "The element \"{id}\" is already owned by another element's aria-owns.",
                            "Das Element \"{id}\" gehört bereits über aria-owns zu einem anderen Element."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                    );
                }
                Some(_) => {}
            }
        }
    }
}

// --- Widgets --------------------------------------------------------------

fn ist_wahr(v: Option<&str>) -> bool {
    v.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

fn nicht_leer(v: Option<&str>) -> bool {
    v.is_some_and(|v| !v.trim().is_empty())
}

/// Tabs und Combobox, soweit WAI-ARIA 1.2 sie festlegt.
///
/// - `aria/tab-selected-missing` (`REVIEW`): Keiner der Tabs einer Tabliste
///   trägt `aria-selected="true"`. ARIA 1.2 (`tab`) sagt SHOULD, und der
///   fehlende Wert ist `false` — welcher Tab gewählt ist, erfährt niemand.
///   Anders als auditmysite nicht je Tab ohne `aria-selected`: Die übrigen
///   Tabs sind mit der Vorgabe `false` richtig ausgezeichnet.
/// - `aria/tabpanel-missing` (`REVIEW`): eine Tabliste ohne ein einziges
///   `tabpanel` im Dokument. ARIA 1.2 beschreibt das nur als üblich
///   („typically placed near"), deshalb kein `FAIL` wie in auditmysite.
///   Beleg: auditmysite#715 (magyarorszag.hu, Navigation als Tabliste).
/// - `aria/combobox-popup-missing`: eine aufgeklappte Combobox ohne
///   `aria-controls`. ARIA 1.2 (`combobox`): „Authors MUST set aria-controls on
///   a combobox element to a value that refers to the combobox popup element."
///   Ein Popup im eigenen Teilbaum oder per `aria-owns` (das Muster aus ARIA
///   1.1) wird noch anerkannt. Beleg: auditmysite-Korpus `widget_patterns`.
pub(crate) fn widgets<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut tablisten = Vec::new();
    let mut panel = false;
    for n in elements(doc) {
        match explizite_rolle(n) {
            Some("tablist") => tablisten.push(n),
            Some("tabpanel") => panel = true,
            Some("combobox") => combobox(n, locale, out),
            _ => {}
        }
    }
    for liste in tablisten {
        if !panel {
            out.push(
                Finding::review(
                    "aria/tabpanel-missing",
                    pick!(
                        locale,
                        "The tab list has no tab panel in the document.",
                        "Zur Tabliste gibt es im Dokument kein Tab-Panel.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["1.3.1", "4.1.2"])
                .at(at(liste.id())),
            );
        }
        if ist_wahr(liste.attr("aria-multiselectable")) {
            continue;
        }
        let mut tabs = descendants(liste).filter(|d| {
            explizite_rolle(*d) == Some("tab")
                && ancestors(*d).find(|a| explizite_rolle(*a) == Some("tablist")) == Some(liste)
        });
        let Some(erster) = tabs.next() else {
            continue;
        };
        if !std::iter::once(erster)
            .chain(tabs)
            .any(|t| ist_wahr(t.attr("aria-selected")))
        {
            out.push(
                Finding::review(
                    "aria/tab-selected-missing",
                    pick!(
                        locale,
                        "No tab in the tab list is marked aria-selected=\"true\".",
                        "Kein Tab der Tabliste ist mit aria-selected=\"true\" ausgezeichnet.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["4.1.2"])
                .at(at(liste.id())),
            );
        }
    }
}

fn combobox<'a, N: Node<'a>>(n: N, locale: Locale, out: &mut Vec<Finding>) {
    if !ist_wahr(n.attr("aria-expanded"))
        || nicht_leer(n.attr("aria-controls"))
        || nicht_leer(n.attr("aria-owns"))
    {
        return;
    }
    let popup = descendants(n).any(|d| {
        matches!(
            explizite_rolle(d),
            Some("listbox" | "tree" | "grid" | "dialog")
        )
    });
    if !popup {
        out.push(
            Finding::fail(
                "aria/combobox-popup-missing",
                pick!(
                    locale,
                    "The expanded combobox does not reference its popup with aria-controls.",
                    "Die aufgeklappte Combobox verweist nicht per aria-controls auf ihr Popup.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- popover ---------------------------------------------------------------

/// `popovertarget` muss auf ein Element mit `popover` zeigen (HTML, „The
/// popover target attributes"). Sonst öffnet der Button nichts, und sein
/// Zustand — aufgeklappt oder nicht — wird nie übermittelt. Beleg:
/// auditmysite `modern_attributes`.
pub(crate) fn popover<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut ausloeser = elements(doc)
        .filter(|n| n.has_attr("popovertarget"))
        .peekable();
    if ausloeser.peek().is_none() {
        return;
    }
    for n in ausloeser {
        let ziel = n.attr("popovertarget").unwrap_or("");
        let (kennung, text) = match mit_id(doc, ziel) {
            None => (
                "popover/target-missing",
                tr!(
                    locale,
                    "popovertarget references \"{ziel}\", which does not exist.",
                    "popovertarget verweist auf \"{ziel}\", das es nicht gibt."
                ),
            ),
            Some(z) if !z.has_attr("popover") => (
                "popover/target-invalid",
                tr!(
                    locale,
                    "popovertarget references \"{ziel}\", which has no popover attribute.",
                    "popovertarget verweist auf \"{ziel}\", das kein popover-Attribut trägt."
                ),
            ),
            Some(_) => continue,
        };
        out.push(
            Finding::fail(kennung, text)
                .with_severity(Severity::Medium)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
        );
    }
}

// --- inert ----------------------------------------------------------------

/// Ein offener Dialog, der selbst oder über einen Vorfahren `inert` ist: Er
/// ist zu sehen, lässt sich aber weder fokussieren noch bedienen, und die
/// Assistenztechnik bekommt ihn nicht (HTML, „The inert attribute").
///
/// `FAIL` nur für `<dialog open>` — dessen Offenheit steht im Markup. Ein
/// `role="dialog"` kann auch ein geschlossener Dialog sein, den ein Stylesheet
/// ausblendet; ohne berechnete Stile ist das nicht zu unterscheiden, deshalb
/// `REVIEW`. auditmysite prüft auch sichtbare `role="menu"`; das ist hier
/// ausgelassen, weil ein per `transform` aus dem Bild geschobenes Menü mit
/// `inert` genau richtig gebaut ist. Beleg: auditmysite `modern_attributes`.
pub(crate) fn inert_dialog<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let offen = n.is_element("dialog") && n.has_attr("open");
        if !offen && !matches!(explizite_rolle(n), Some("dialog" | "alertdialog")) {
            continue;
        }
        if !n.has_attr("inert") && !ancestors(n).any(|a| a.has_attr("inert")) {
            continue;
        }
        let text = pick!(
            locale,
            "The dialog is inert: it cannot be focused or operated.",
            "Der Dialog ist inert: Er lässt sich weder fokussieren noch bedienen.",
        );
        let f = if offen {
            Finding::fail("inert/dialog-inert", text)
        } else {
            Finding::review("inert/dialog-inert", text)
        };
        out.push(
            f.with_severity(Severity::High)
                .with_wcag(["2.1.1", "4.1.2"])
                .at(at(n.id())),
        );
    }
}

// --- Rollenabhängig: erlaubte und verbotene Attribute ---------------------

/// Rollen, an denen ARIA 1.2 einen Namen verbietet (`aria-label`,
/// `aria-labelledby`), dazu `aria-roledescription` an `generic`.
fn verboten(rolle: &str, attr: &str) -> bool {
    let name = matches!(attr, "aria-label" | "aria-labelledby");
    match rolle {
        "generic" => name || attr == "aria-roledescription",
        "caption" | "code" | "deletion" | "emphasis" | "insertion" | "none" | "paragraph"
        | "presentation" | "strong" | "subscript" | "superscript" => name,
        _ => false,
    }
}

/// Was ARIA in HTML an einem nativen Element ausdrücklich verbietet
/// („Authors MUST NOT …", Abschnitt 3.2): ARIA, das dem nativen Zustand
/// widersprechen kann. `aria-hidden="true"` an `<body>` gehört auch dazu, ist
/// in der Sicht dieser Regel aber nie zu sehen (der Teilbaum fehlt darin).
fn nativ_verboten<'a, N: Node<'a>>(n: N, attr: &str, v: &str) -> bool {
    let falsch = v.eq_ignore_ascii_case("false");
    match attr {
        "aria-checked" => {
            n.is_element("input")
                && n.attr("type").is_some_and(|t| {
                    let t = t.trim();
                    t.eq_ignore_ascii_case("checkbox") || t.eq_ignore_ascii_case("radio")
                })
        }
        "aria-disabled" => falsch && n.has_attr("disabled"),
        "aria-readonly" => falsch && n.has_attr("readonly"),
        "aria-required" => falsch && n.has_attr("required"),
        "aria-placeholder" => n.has_attr("placeholder"),
        "aria-valuemax" => n.has_attr("max"),
        "aria-valuemin" => n.has_attr("min"),
        "aria-colspan" => n.attr("colspan").is_some_and(|c| c.trim() != v),
        "aria-rowspan" => n.attr("rowspan").is_some_and(|c| c.trim() != v),
        _ => false,
    }
}

/// Attribute, die die Rolle des Elements nicht unterstützt oder verbietet.
///
/// Die Rolle ist die des Hosts — explizit oder implizit. Rollen, die WAI-ARIA
/// 1.2 nicht kennt (etwa browserinterne), bleiben ohne Urteil. Leere Werte
/// zählen wie ein fehlendes Attribut. Anders als auditmysite nicht nur für
/// explizite `role`-Angaben: `<div aria-expanded>` ist genauso ein Befund wie
/// `<div role="img" aria-checked>`. Beleg: auditmysite-Korpus
/// `aria_attribute_validation` (`aria-checked` an `role="img"`,
/// `aria-label` an `role="presentation"`).
pub(crate) fn attributes_allowed<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || !n.attributes().any(|(k, _)| ist_aria(k)) {
            continue;
        }
        let rolle = doc.role(n).and_then(|r| rollen_index(&r));
        for (name, wert) in n.attributes() {
            let v = wert.trim();
            let Some(ai) = attribut_index(name) else {
                continue;
            };
            let attr = ATTRIBUTE[ai];
            if v.is_empty() {
                continue;
            }
            if nativ_verboten(n, attr, v) {
                out.push(
                    Finding::fail(
                        "aria/attribute-not-allowed",
                        tr!(
                            locale,
                            "{attr} must not be used on <{}>: the native attribute or state already carries it (ARIA in HTML).",
                            "{attr} darf an <{}> nicht stehen: Das native Attribut oder der native Zustand trägt es bereits (ARIA in HTML).",
                            n.local_name()
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
                continue;
            }
            let Some(ri) = rolle else {
                continue;
            };
            let r = VALID_ROLES[ri];
            if verboten(r, attr) {
                out.push(
                    Finding::fail(
                        "aria/attribute-prohibited",
                        tr!(
                            locale,
                            "{attr} is prohibited on role \"{r}\".",
                            "{attr} ist an der Rolle \"{r}\" verboten."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            } else if (GLOBAL | JE_ROLLE[ri]) & (1 << ai) == 0 {
                out.push(
                    Finding::fail(
                        "aria/attribute-not-allowed",
                        tr!(
                            locale,
                            "{attr} is not supported on role \"{r}\".",
                            "{attr} wird von der Rolle \"{r}\" nicht unterstützt."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

// --- Rollenabhängig: Kontext und Bestandteile -----------------------------

/// Erforderlicher Kontext (WAI-ARIA 1.2, „Required Context Role").
///
/// Über die Spezifikation hinaus zugelassen, weil ältere Fassungen oder
/// native Muster sie tragen und ein Befund dort nicht sicher wäre:
/// `group` für `listitem` (ARIA 1.1), `combobox` für `option` (`<select>`),
/// `radiogroup` für `menuitemradio` (eine Unterrolle von `group`).
const KONTEXT: &[(&str, &[&str])] = &[
    ("caption", &["figure", "grid", "table", "treegrid"]),
    ("cell", &["row"]),
    ("columnheader", &["row"]),
    ("gridcell", &["row"]),
    ("listitem", &["directory", "list", "group"]),
    ("menuitem", &["group", "menu", "menubar"]),
    ("menuitemcheckbox", &["group", "menu", "menubar"]),
    ("menuitemradio", &["group", "menu", "menubar", "radiogroup"]),
    ("option", &["group", "listbox", "combobox"]),
    ("row", &["grid", "rowgroup", "table", "treegrid"]),
    ("rowgroup", &["grid", "table", "treegrid"]),
    ("rowheader", &["row"]),
    ("tab", &["tablist"]),
    ("treeitem", &["group", "tree"]),
];

/// Erforderliche Bestandteile (WAI-ARIA 1.2, „Required Owned Elements").
/// `rowgroup → row` und `group → …` sind vereinfacht: die Gruppe selbst zählt.
const BESTANDTEILE: &[(&str, &[&str])] = &[
    ("feed", &["article"]),
    ("grid", &["row", "rowgroup"]),
    ("list", &["listitem"]),
    ("listbox", &["group", "option"]),
    (
        "menu",
        &["group", "menuitem", "menuitemcheckbox", "menuitemradio"],
    ),
    (
        "menubar",
        &["group", "menuitem", "menuitemcheckbox", "menuitemradio"],
    ),
    ("radiogroup", &["radio"]),
    ("row", &["cell", "columnheader", "gridcell", "rowheader"]),
    ("rowgroup", &["row"]),
    ("table", &["row", "rowgroup"]),
    ("tablist", &["tab"]),
    ("tree", &["group", "treeitem"]),
    ("treegrid", &["row", "rowgroup"]),
];

fn nachschlagen(
    tabelle: &'static [(&str, &'static [&'static str])],
    rolle: &str,
) -> Option<&'static [&'static str]> {
    tabelle.iter().find(|(r, _)| *r == rolle).map(|(_, l)| *l)
}

/// Wie ein Knoten auf dem Weg zwischen Behälter und Bestandteil zählt.
#[derive(Clone, Copy)]
enum Wirksam {
    /// `generic`, `none`/`presentation`, keine Rolle oder vom Host
    /// ausgeblendet: ARIA 1.2 überspringt solche Zwischenknoten.
    Durchlaessig,
    /// Eine Rolle aus [`VALID_ROLES`].
    Rolle(usize),
    /// Eine Rolle, die WAI-ARIA 1.2 nicht kennt: kein Urteil möglich.
    Unbekannt,
}

fn einordnen(rolle: &str) -> Wirksam {
    match rolle {
        "generic" | "none" | "presentation" => Wirksam::Durchlaessig,
        _ => rollen_index(rolle).map_or(Wirksam::Unbekannt, Wirksam::Rolle),
    }
}

/// Ob `n` ein `<li>` direkt in `<ul>`, `<ol>` oder `<menu>` ist.
fn li_in_liste<'a, N: Node<'a>>(n: N) -> Option<N> {
    if !n.is_element("li") || explizite_rolle(n).is_some() {
        return None;
    }
    n.parent()
        .filter(|p| matches!(p.local_name(), "ul" | "ol" | "menu"))
}

/// Die Rolle, mit der ein Knoten in Kontext- und Bestandteilprüfungen zählt.
///
/// Die explizite Rolle aus dem Markup geht vor, dann die des Hosts. Eine
/// Ausnahme: Ein `<li>` in `<ul>`/`<ol>`/`<menu>` ist nach ARIA in HTML ein
/// `listitem`, auch wenn die Liste per `role` etwas anderes geworden ist.
/// Chrome glättet es dann zu `generic` — der Accessibility-Tree zeigt
/// `tablist → tab`, obwohl das `<li>` dazwischen steht (auditmysite#715, axe
/// meldet es). Nur wenn die Liste `none`/`presentation` ist, erbt der Eintrag
/// das (ARIA 1.2, `presentation`: „required owned elements").
fn wirksame_rolle<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> Wirksam {
    if let Some(r) = explizite_rolle(n) {
        return einordnen(r);
    }
    if let Some(liste) = li_in_liste(n) {
        return match explizite_rolle(liste) {
            Some("none" | "presentation") => Wirksam::Durchlaessig,
            _ => Wirksam::Rolle(rollen_index("listitem").unwrap_or_default()),
        };
    }
    if doc.is_ignored(n) {
        return Wirksam::Durchlaessig;
    }
    match doc.role(n) {
        None => Wirksam::Durchlaessig,
        Some(r) => einordnen(&r),
    }
}

/// Das erste Element mit dieser ID, wie `getElementById`.
fn mit_id<'n, D: Document>(doc: &'n D, id: &str) -> Option<D::N<'n>> {
    elements(doc).find(|n| n.attr("id") == Some(id))
}

/// Wem ein Element per `aria-owns` gehört, nach ID. Eine Liste statt einer
/// HashMap: aria-owns ist selten, und jede HashMap-Ausprägung kostet im
/// WASM-Build Platz.
type Besitzer<'n, N> = Vec<(&'n str, N)>;

fn besitzer<'n, D: Document>(doc: &'n D) -> Besitzer<'n, D::N<'n>> {
    let mut m: Besitzer<'n, D::N<'n>> = Vec::new();
    for n in elements(doc) {
        if let Some(v) = n.attr("aria-owns") {
            for id in v.split_whitespace() {
                if !m.iter().any(|(i, _)| *i == id) {
                    m.push((id, n));
                }
            }
        }
    }
    m
}

fn besitzer_von<'n, N: Node<'n>>(besitzer: &Besitzer<'n, N>, n: N) -> Option<N> {
    let id = n.attr("id")?;
    besitzer.iter().find(|(i, _)| *i == id).map(|(_, b)| *b)
}

/// Ob über `n` — durch durchlässige Knoten hindurch — eine der Rollen steht.
/// `None`: nicht entscheidbar.
fn im_kontext<'n, D: Semantics>(
    doc: &'n D,
    n: D::N<'n>,
    erlaubt: &[&str],
    besitzer: &Besitzer<'n, D::N<'n>>,
) -> Option<bool> {
    let mut cur = n;
    // Schranke gegen aria-owns-Zyklen.
    for _ in 0..256 {
        let eltern = besitzer_von(besitzer, cur)
            .filter(|b| *b != cur)
            .or_else(|| cur.parent());
        let Some(p) = eltern else {
            return Some(false);
        };
        if p.is_element("body") || p.is_element("html") {
            return Some(false);
        }
        match wirksame_rolle(doc, p) {
            Wirksam::Durchlaessig => cur = p,
            Wirksam::Unbekannt => return None,
            Wirksam::Rolle(i) => return Some(erlaubt.contains(&VALID_ROLES[i])),
        }
    }
    None
}

/// Ob `n` eines der geforderten Elemente besitzt: seine Kinder, durch
/// durchlässige Knoten hindurch, und was es per `aria-owns` an sich zieht.
/// Was einem anderen per `aria-owns` gehört, zählt dort, nicht hier.
/// `None`: nichts besessen oder nicht entscheidbar.
fn besitzt<'n, D: Semantics>(
    doc: &'n D,
    n: D::N<'n>,
    noetig: &[&str],
    besitzer: &Besitzer<'n, D::N<'n>>,
) -> Option<bool> {
    let fremd = |k: D::N<'n>| besitzer_von(besitzer, k).is_some_and(|b| b != n);
    let mut stapel: Vec<D::N<'n>> = n
        .attr("aria-owns")
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|id| mit_id(doc, id))
        .collect();
    stapel.extend(n.children());
    let mut irgendwas = false;
    let mut unbekannt = false;
    while let Some(k) = stapel.pop() {
        if k.kind() != a11y_dom::NodeKind::Element
            || matches!(k.local_name(), "script" | "style" | "template")
            || (k.parent() == Some(n) && fremd(k))
        {
            continue;
        }
        match wirksame_rolle(doc, k) {
            Wirksam::Durchlaessig => stapel.extend(k.children().filter(|c| !fremd(*c))),
            Wirksam::Unbekannt => unbekannt = true,
            Wirksam::Rolle(i) => {
                if noetig.contains(&VALID_ROLES[i]) {
                    return Some(true);
                }
                irgendwas = true;
            }
        }
    }
    (irgendwas && !unbekannt).then_some(false)
}

/// Kontext und Bestandteile nach WAI-ARIA 1.2, geprüft am DOM.
///
/// - `aria/required-parent-missing`: Eine Rolle steht nicht in ihrem
///   erforderlichen Kontext („Authors MUST ensure elements with role tab are
///   contained in, or owned by, an element with the role tablist"). Geprüft
///   werden explizite Rollen und das `<li>` einer Liste, die per `role` keine
///   mehr ist. Beleg: auditmysite-Korpus `aria_attribute_validation`
///   (verwaister Tab) und auditmysite#715 (`ul[role=tablist] > li > a[role=tab]`).
/// - `aria/required-children-missing`: Ein Behälter mit expliziter Rolle
///   besitzt keines seiner erforderlichen Elemente. Native Tabellen und Listen
///   ohne `role` prüft das nicht — ihre Bestandteile sind durch HTML gegeben
///   (auditmysite#659, #674: `<table><tbody><tr>`), die `lists/*`-Regeln
///   decken sie ab. Ohne jeden besessenen Knoten, mit `aria-busy="true"` oder
///   `aria-expanded="false"` kein Befund (anders als axe, wie auditmysite).
///   Beleg: auditmysite#715 (Tabliste aus `<li>`) und auditmysite-Korpus
///   `table_required_rows_tbody` (Tabelle nur mit Beschriftung).
///
/// Durchlässig sind `generic`, `none`/`presentation` und was der Host
/// ausblendet. Chrome blendet ein schlichtes `<tbody>` aus; die Zeilen
/// darunter zählen trotzdem (auditmysite#659).
pub(crate) fn required_context<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let besitzer = besitzer(doc);
    for n in elements(doc) {
        let rolle = match explizite_rolle(n) {
            Some(r) => Some(r),
            // Das <li> einer Liste, die per role etwas anderes ist.
            None => li_in_liste(n).and_then(|l| {
                explizite_rolle(l)
                    .filter(|r| !matches!(*r, "list" | "directory" | "none" | "presentation"))
                    .map(|_| "listitem")
            }),
        };
        let Some(rolle) = rolle else {
            continue;
        };

        if let Some(erlaubt) = nachschlagen(KONTEXT, rolle)
            && im_kontext(doc, n, erlaubt, &besitzer) == Some(false)
        {
            let kontext = erlaubt.join(", ");
            out.push(
                Finding::fail(
                    "aria/required-parent-missing",
                    tr!(
                        locale,
                        "role=\"{rolle}\" must be contained in or owned by an element with role {kontext}.",
                        "role=\"{rolle}\" muss in einem Element mit der Rolle {kontext} stehen oder ihm gehören."
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["1.3.1", "4.1.2"])
                .at(at(n.id())),
            );
        }

        if let Some(noetig) = nachschlagen(BESTANDTEILE, rolle) {
            if n.attr("aria-expanded")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"))
            {
                continue;
            }
            if std::iter::once(n)
                .chain(ancestors(n))
                .any(|a| ist_wahr(a.attr("aria-busy")))
            {
                continue;
            }
            if besitzt(doc, n, noetig, &besitzer) == Some(false) {
                let teile = noetig.join(", ");
                out.push(
                    Finding::fail(
                        "aria/required-children-missing",
                        tr!(
                            locale,
                            "role=\"{rolle}\" must contain or own an element with role {teile}.",
                            "role=\"{rolle}\" muss ein Element mit der Rolle {teile} enthalten oder besitzen."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["1.3.1", "4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}
