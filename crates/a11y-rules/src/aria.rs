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

use a11y_dom::{
    Document, Node, NodeId, ScopedId, Semantics, ancestors, descendants, elements, scoped_id,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::{VALID_ROLES, explicit_role};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- Tabellen -------------------------------------------------------------

/// Alle Zustände und Eigenschaften aus WAI-ARIA 1.2, alphabetisch. Der Index
/// ist die Bitposition in [`GLOBAL`] und [`PER_ROLE`].
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
const UNKNOWN: u64 = u64::MAX;

/// Was eine Rolle über [`GLOBAL`] hinaus unterstützt — eigene, geerbte und
/// erforderliche Attribute, in derselben Reihenfolge wie [`VALID_ROLES`].
/// In ARIA 1.2 auf einer Rolle als veraltet markierte Attribute zählen als
/// unterstützt: veraltet ist nicht verboten.
const PER_ROLE: [u64; 83] = [
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
    UNKNOWN,        // mark (ARIA 1.3, nicht in 1.2)
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

const _: () = assert!(PER_ROLE.len() == VALID_ROLES.len());

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

fn is_aria(name: &str) -> bool {
    name.get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("aria-"))
}

fn attribute_index(name: &str) -> Option<usize> {
    ATTRIBUTE.iter().position(|a| a.eq_ignore_ascii_case(name))
}

fn roles_index(role: &str) -> Option<usize> {
    VALID_ROLES.iter().position(|r| *r == role)
}

// --- aria/attribute-unknown -----------------------------------------------

/// Ein `aria-*`-Attribut, das es nicht gibt — meist ein Tippfehler
/// (`aria-labeledby`). Der Browser ignoriert es, die gemeinte Beziehung oder
/// der gemeinte Zustand fehlt. Beleg: auditmysite-Korpus
/// `aria_attribute_validation` (`aria-nonexistentattr`).
pub(crate) fn attribute_names<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        for (name, _) in n.attributes() {
            if !is_aria(name)
                || attribute_index(name).is_some()
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

enum ValueType {
    /// Genau einer der Werte.
    One(&'static [&'static str]),
    /// Eine Liste aus diesen Werten, durch Leerraum getrennt.
    List(&'static [&'static str]),
    Integer,
    Number,
}

const TRUE_WRONG: &[&str] = &["true", "false"];
const TRUE_WRONG_UNDEF: &[&str] = &["true", "false", "undefined"];
const TRISTATE: &[&str] = &["true", "false", "mixed", "undefined"];

/// Der Werttyp nach WAI-ARIA 1.2, Abschnitt 6.6/6.7.
///
/// Nicht geprüft: IDREFs (das ist `aria/reference-missing`), freie Zeichenketten
/// und `aria-current`/`aria-invalid` — für diese beiden legt die Spezifikation
/// fest, dass ein unbekannter Wert als `true` gilt; der Wert tut damit, was
/// gemeint ist. auditmysite meldet sie; hier bewusst nicht.
fn value_type(attr: &str) -> Option<ValueType> {
    use ValueType::*;
    Some(match attr {
        "aria-atomic"
        | "aria-busy"
        | "aria-disabled"
        | "aria-modal"
        | "aria-multiline"
        | "aria-multiselectable"
        | "aria-readonly"
        | "aria-required" => One(TRUE_WRONG),
        "aria-expanded" | "aria-grabbed" | "aria-hidden" | "aria-selected" => One(TRUE_WRONG_UNDEF),
        "aria-checked" | "aria-pressed" => One(TRISTATE),
        "aria-autocomplete" => One(&["inline", "list", "both", "none"]),
        "aria-haspopup" => One(&["false", "true", "menu", "listbox", "tree", "grid", "dialog"]),
        "aria-live" => One(&["assertive", "off", "polite"]),
        "aria-orientation" => One(&["horizontal", "vertical", "undefined"]),
        "aria-sort" => One(&["ascending", "descending", "none", "other"]),
        "aria-dropeffect" => List(&["copy", "execute", "link", "move", "none", "popup"]),
        "aria-relevant" => List(&["additions", "removals", "text", "all"]),
        "aria-colcount" | "aria-colindex" | "aria-colspan" | "aria-level" | "aria-posinset"
        | "aria-rowcount" | "aria-rowindex" | "aria-rowspan" | "aria-setsize" => Integer,
        "aria-valuemax" | "aria-valuemin" | "aria-valuenow" => Number,
        _ => return None,
    })
}

fn a_of(list: &[&str], v: &str) -> bool {
    list.iter().any(|t| t.eq_ignore_ascii_case(v))
}

/// Eine Dezimalzahl nach dem Muster `-1`, `2.5`, `.5`, `1e3` — ohne
/// Gleitkomma-Parser, der im WASM-Build mehrere Kilobyte kostet.
fn is_number(v: &str) -> bool {
    let v = v.strip_prefix(['+', '-']).unwrap_or(v);
    let (mantissa, exponent) = match v.find(['e', 'E']) {
        Some(i) => (&v[..i], Some(&v[i + 1..])),
        None => (v, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    (!whole.is_empty() || !fraction.is_empty())
        && digits(whole)
        && digits(fraction)
        && exponent.is_none_or(|e| {
            let e = e.strip_prefix(['+', '-']).unwrap_or(e);
            !e.is_empty() && digits(e)
        })
}

fn value_valid(ty: &ValueType, v: &str) -> bool {
    match ty {
        ValueType::One(l) => a_of(l, v),
        ValueType::List(l) => v.split_whitespace().all(|t| a_of(l, t)),
        ValueType::Integer => v.parse::<i64>().is_ok(),
        ValueType::Number => is_number(v),
    }
}

/// Ein Wert außerhalb des Wertebereichs. Browser fallen dann auf die Vorgabe
/// zurück — `aria-expanded="yes"` wird zu „nicht aufklappbar", der Zustand ist
/// verloren. Ein leerer Wert gilt nach ARIA wie ein fehlendes Attribut und
/// wird nicht gemeldet. Beleg: auditmysite-Korpus `aria_invalid_attr_value`
/// (`aria-expanded="maybe"`).
pub(crate) fn attribute_values<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        for (name, value) in n.attributes() {
            let Some(ty) = attribute_index(name).and_then(|i| value_type(ATTRIBUTE[i])) else {
                continue;
            };
            let v = value.trim();
            if v.is_empty() || value_valid(&ty, v) {
                continue;
            }
            let expects = match ty {
                ValueType::One(l) | ValueType::List(l) => l.join(", "),
                ValueType::Integer => pick!(locale, "an integer", "eine ganze Zahl").to_string(),
                ValueType::Number => pick!(locale, "a number", "eine Zahl").to_string(),
            };
            out.push(
                Finding::fail(
                    "aria/attribute-value-invalid",
                    tr!(
                        locale,
                        "{name}=\"{v}\" is not a valid value; expected: {expects}.",
                        "{name}=\"{v}\" ist kein gültiger Wert; erwartet: {expects}."
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
    let mut first: Vec<(ScopedId, NodeId)> = Vec::new();
    let mut reported: Vec<ScopedId> = Vec::new();
    for n in elements(doc) {
        let Some(v) = n.attr("aria-owns") else {
            continue;
        };
        for id in v.split_whitespace() {
            let key = scoped_id(n, id);
            match first.iter().find(|(i, _)| *i == key) {
                None => first.push((key, n.id())),
                Some((_, e)) if *e != n.id() && !reported.contains(&key) => {
                    reported.push(key);
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

fn is_true(v: Option<&str>) -> bool {
    v.is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

fn not_empty(v: Option<&str>) -> bool {
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
    let mut tab_lists = Vec::new();
    let mut panel = false;
    for n in elements(doc) {
        match explicit_role(n) {
            Some("tablist") => tab_lists.push(n),
            Some("tabpanel") => panel = true,
            Some("combobox") => combobox(n, locale, out),
            _ => {}
        }
    }
    for list in tab_lists {
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
                .at(at(list.id())),
            );
        }
        if is_true(list.attr("aria-multiselectable")) {
            continue;
        }
        let mut tabs = descendants(list).filter(|d| {
            explicit_role(*d) == Some("tab")
                && ancestors(*d).find(|a| explicit_role(*a) == Some("tablist")) == Some(list)
        });
        let Some(first) = tabs.next() else {
            continue;
        };
        if !std::iter::once(first)
            .chain(tabs)
            .any(|t| is_true(t.attr("aria-selected")))
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
                .at(at(list.id())),
            );
        }
    }
}

fn combobox<'a, N: Node<'a>>(n: N, locale: Locale, out: &mut Vec<Finding>) {
    if !is_true(n.attr("aria-expanded"))
        || not_empty(n.attr("aria-controls"))
        || not_empty(n.attr("aria-owns"))
    {
        return;
    }
    let popup = descendants(n).any(|d| {
        matches!(
            explicit_role(d),
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
    let mut triggers = elements(doc)
        .filter(|n| n.has_attr("popovertarget"))
        .peekable();
    if triggers.peek().is_none() {
        return;
    }
    for n in triggers {
        let target = n.attr("popovertarget").unwrap_or("");
        let (id, text) = match with_id(doc, n, target) {
            None => (
                "popover/target-missing",
                tr!(
                    locale,
                    "popovertarget references \"{target}\", which does not exist.",
                    "popovertarget verweist auf \"{target}\", das es nicht gibt."
                ),
            ),
            Some(z) if !z.has_attr("popover") => (
                "popover/target-invalid",
                tr!(
                    locale,
                    "popovertarget references \"{target}\", which has no popover attribute.",
                    "popovertarget verweist auf \"{target}\", das kein popover-Attribut trägt."
                ),
            ),
            Some(_) => continue,
        };
        out.push(
            Finding::fail(id, text)
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
        let open = n.is_element("dialog") && n.has_attr("open");
        if !open && !matches!(explicit_role(n), Some("dialog" | "alertdialog")) {
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
        let f = if open {
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
fn forbidden(role: &str, attr: &str) -> bool {
    let name = matches!(attr, "aria-label" | "aria-labelledby");
    match role {
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
fn native_forbidden<'a, N: Node<'a>>(n: N, attr: &str, v: &str) -> bool {
    let wrong = v.eq_ignore_ascii_case("false");
    match attr {
        "aria-checked" => {
            n.is_element("input")
                && n.attr("type").is_some_and(|t| {
                    let t = t.trim();
                    t.eq_ignore_ascii_case("checkbox") || t.eq_ignore_ascii_case("radio")
                })
        }
        "aria-disabled" => wrong && n.has_attr("disabled"),
        "aria-readonly" => wrong && n.has_attr("readonly"),
        "aria-required" => wrong && n.has_attr("required"),
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
        if doc.is_ignored(n) || !n.attributes().any(|(k, _)| is_aria(k)) {
            continue;
        }
        let role = doc.role(n).and_then(|r| roles_index(&r));
        for (name, value) in n.attributes() {
            let v = value.trim();
            let Some(ai) = attribute_index(name) else {
                continue;
            };
            let attr = ATTRIBUTE[ai];
            if v.is_empty() {
                continue;
            }
            if native_forbidden(n, attr, v) {
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
            let Some(ri) = role else {
                continue;
            };
            let r = VALID_ROLES[ri];
            if forbidden(r, attr) {
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
            } else if (GLOBAL | PER_ROLE[ri]) & (1 << ai) == 0 {
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
const CONTEXT: &[(&str, &[&str])] = &[
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
const PARTS: &[(&str, &[&str])] = &[
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

fn look_up(
    table: &'static [(&str, &'static [&'static str])],
    role: &str,
) -> Option<&'static [&'static str]> {
    table.iter().find(|(r, _)| *r == role).map(|(_, l)| *l)
}

/// Wie ein Knoten auf dem Weg zwischen Behälter und Bestandteil zählt.
#[derive(Clone, Copy)]
enum Effective {
    /// `generic`, `none`/`presentation`, keine Rolle oder vom Host
    /// ausgeblendet: ARIA 1.2 überspringt solche Zwischenknoten.
    Translucent,
    /// Eine Rolle aus [`VALID_ROLES`].
    Role(usize),
    /// Eine Rolle, die WAI-ARIA 1.2 nicht kennt: kein Urteil möglich.
    Unknown,
}

fn classify(role: &str) -> Effective {
    match role {
        "generic" | "none" | "presentation" => Effective::Translucent,
        _ => roles_index(role).map_or(Effective::Unknown, Effective::Role),
    }
}

/// Ob `n` ein `<li>` direkt in `<ul>`, `<ol>` oder `<menu>` ist.
fn li_in_list<'a, N: Node<'a>>(n: N) -> Option<N> {
    if !n.is_element("li") || explicit_role(n).is_some() {
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
fn effective_role<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> Effective {
    if let Some(r) = explicit_role(n) {
        return classify(r);
    }
    if let Some(list) = li_in_list(n) {
        return match explicit_role(list) {
            Some("none" | "presentation") => Effective::Translucent,
            _ => Effective::Role(roles_index("listitem").unwrap_or_default()),
        };
    }
    if doc.is_ignored(n) {
        return Effective::Translucent;
    }
    match doc.role(n) {
        None => Effective::Translucent,
        Some(r) => classify(&r),
    }
}

/// Das erste Element mit dieser ID im Bereich von `of`, wie
/// `getElementById` auf dessen Wurzel.
fn with_id<'n, D: Document>(doc: &'n D, of: D::N<'n>, id: &str) -> Option<D::N<'n>> {
    elements(doc).find(|n| n.attr("id") == Some(id) && n.tree_scope() == of.tree_scope())
}

/// Wem ein Element per `aria-owns` gehört, nach ID. Eine Liste statt einer
/// HashMap: aria-owns ist selten, und jede HashMap-Ausprägung kostet im
/// WASM-Build Platz.
type Owner<'n, N> = Vec<(ScopedId<'n>, N)>;

fn owner<'n, D: Document>(doc: &'n D) -> Owner<'n, D::N<'n>> {
    let mut m: Owner<'n, D::N<'n>> = Vec::new();
    for n in elements(doc) {
        if let Some(v) = n.attr("aria-owns") {
            for id in v.split_whitespace().map(|id| scoped_id(n, id)) {
                if !m.iter().any(|(i, _)| *i == id) {
                    m.push((id, n));
                }
            }
        }
    }
    m
}

fn owner_of<'n, N: Node<'n>>(owner: &Owner<'n, N>, n: N) -> Option<N> {
    let id = scoped_id(n, n.attr("id")?);
    owner.iter().find(|(i, _)| *i == id).map(|(_, b)| *b)
}

/// Ob über `n` — durch durchlässige Knoten hindurch — eine der Rollen steht.
/// `None`: nicht entscheidbar.
fn in_context<'n, D: Semantics>(
    doc: &'n D,
    n: D::N<'n>,
    allowed: &[&str],
    owner: &Owner<'n, D::N<'n>>,
) -> Option<bool> {
    let mut cur = n;
    // Schranke gegen aria-owns-Zyklen.
    for _ in 0..256 {
        let parent = owner_of(owner, cur)
            .filter(|b| *b != cur)
            .or_else(|| cur.parent());
        let Some(p) = parent else {
            return Some(false);
        };
        if p.is_element("body") || p.is_element("html") {
            return Some(false);
        }
        match effective_role(doc, p) {
            Effective::Translucent => cur = p,
            Effective::Unknown => return None,
            Effective::Role(i) => return Some(allowed.contains(&VALID_ROLES[i])),
        }
    }
    None
}

/// Ob `n` eines der geforderten Elemente besitzt: seine Kinder, durch
/// durchlässige Knoten hindurch, und was es per `aria-owns` an sich zieht.
/// Was einem anderen per `aria-owns` gehört, zählt dort, nicht hier.
/// `None`: nichts besessen oder nicht entscheidbar.
fn owns<'n, D: Semantics>(
    doc: &'n D,
    n: D::N<'n>,
    needed: &[&str],
    owner: &Owner<'n, D::N<'n>>,
) -> Option<bool> {
    let foreign = |k: D::N<'n>| owner_of(owner, k).is_some_and(|b| b != n);
    let mut stack: Vec<D::N<'n>> = n
        .attr("aria-owns")
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|id| with_id(doc, n, id))
        .collect();
    stack.extend(n.children());
    let mut something = false;
    let mut unknown = false;
    while let Some(k) = stack.pop() {
        if k.kind() != a11y_dom::NodeKind::Element
            || matches!(k.local_name(), "script" | "style" | "template")
            || (k.parent() == Some(n) && foreign(k))
        {
            continue;
        }
        match effective_role(doc, k) {
            Effective::Translucent => stack.extend(k.children().filter(|c| !foreign(*c))),
            Effective::Unknown => unknown = true,
            Effective::Role(i) => {
                if needed.contains(&VALID_ROLES[i]) {
                    return Some(true);
                }
                something = true;
            }
        }
    }
    (something && !unknown).then_some(false)
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
    let owner = owner(doc);
    for n in elements(doc) {
        let role = match explicit_role(n) {
            Some(r) => Some(r),
            // Das <li> einer Liste, die per role etwas anderes ist.
            None => li_in_list(n).and_then(|l| {
                explicit_role(l)
                    .filter(|r| !matches!(*r, "list" | "directory" | "none" | "presentation"))
                    .map(|_| "listitem")
            }),
        };
        let Some(role) = role else {
            continue;
        };

        // Der Kontext eines `listitem` ist Sache der Listen-Familie
        // (`lists/item-outside-list`) — sonst meldeten zwei Regeln denselben
        // verwaisten Eintrag (auditmysite-Kalibrierung `invalid_aria.html`).
        if role != "listitem"
            && let Some(allowed) = look_up(CONTEXT, role)
            && in_context(doc, n, allowed, &owner) == Some(false)
        {
            let context = allowed.join(", ");
            out.push(
                Finding::fail(
                    "aria/required-parent-missing",
                    tr!(
                        locale,
                        "role=\"{role}\" must be contained in or owned by an element with role {context}.",
                        "role=\"{role}\" muss in einem Element mit der Rolle {context} stehen oder ihm gehören."
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["1.3.1", "4.1.2"])
                .at(at(n.id())),
            );
        }

        if let Some(needed) = look_up(PARTS, role) {
            if n.attr("aria-expanded")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"))
            {
                continue;
            }
            if std::iter::once(n)
                .chain(ancestors(n))
                .any(|a| is_true(a.attr("aria-busy")))
            {
                continue;
            }
            if owns(doc, n, needed, &owner) == Some(false) {
                let parts = needed.join(", ");
                out.push(
                    Finding::fail(
                        "aria/required-children-missing",
                        tr!(
                            locale,
                            "role=\"{role}\" must contain or own an element with role {parts}.",
                            "role=\"{role}\" muss ein Element mit der Rolle {parts} enthalten oder besitzen."
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
