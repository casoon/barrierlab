//! Namensregeln (casoon/barrierlab#15).
//!
//! Die Fälle stammen aus auditmysite: dem Detection-Korpus
//! (`tests/fixtures/detection_corpus/`), den übrigen Fixtures
//! (`tests/fixtures/`) und den Tests der portierten Regeln; der Name jedes
//! Tests nennt die Quelle. Jeder Fall läuft gegen zwei Hosts: Rolle und Name
//! aus `accname` und einen, der sich wie Chrome verhält.

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, NameSource, Node, Semantics, elements};
use a11y_report::{Outcome, Report};
use a11y_rules::run_with_semantics;
use accname::IdIndex;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

/// Ein HTML-Fragment als Arena, über den echten Parser.
fn html(src: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body><main>{src}</main></body></html>"
    ))
    .document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    uebernehmen(&doc, wurzel, Arena::builder()).build()
}

fn uebernehmen(
    doc: &html5_parser::Document,
    id: html5_parser::NodeId,
    b: ArenaBuilder,
) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.close()
        }
        H::Text { content } => b.text(content),
        H::DocumentFragment => doc.children(id).fold(b, |b, k| uebernehmen(doc, k, b)),
        _ => b,
    }
}

/// Tier-2-Host: die Arena mit Rolle und Name aus `accname`.
struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    /// Verhält sich wie Chrome: `<summary>` ist `DisclosureTriangle`, ein
    /// geschlossenes `<dialog>` fehlt im Baum, und der Host kennt die Quelle
    /// des Namens.
    chrome: bool,
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        if self.chrome && node.is_element("summary") {
            return Some("DisclosureTriangle".into());
        }
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn name_source<'n>(&'n self, node: Self::N<'n>) -> Option<NameSource> {
        if !self.chrome {
            return None;
        }
        if node
            .attr("aria-labelledby")
            .is_some_and(|v| !v.trim().is_empty())
        {
            Some(NameSource::AriaLabelledBy)
        } else if node
            .attr("aria-label")
            .is_some_and(|v| !v.trim().is_empty())
        {
            Some(NameSource::AriaLabel)
        } else {
            Some(NameSource::Contents)
        }
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
            || (self.chrome && node.is_element("dialog") && !node.has_attr("open"))
    }
}

/// Beide Hosts über dasselbe Fragment.
fn beide(src: &str) -> [Report; 2] {
    let a = html(src);
    [false, true].map(|chrome| {
        run_with_semantics(&MitSemantik {
            doc: &a,
            ids: IdIndex::build(a.root()),
            chrome,
        })
    })
}

fn anzahl(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

fn outcomes(r: &Report, id: &str) -> Vec<Outcome> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(|f| f.outcome)
        .collect()
}

/// Die Befunde einer Kennung, verortet an der `id` des Elements.
fn an(src: &str, r: &Report, rule: &str) -> Vec<String> {
    let a = html(src);
    let mut v: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| {
            let knoten = f.location.node.as_deref()?;
            elements(&a)
                .find(|n| n.id().to_string() == knoten)
                .map(|n| n.attr("id").unwrap_or(n.local_name()).to_string())
        })
        .collect();
    v.sort();
    v
}

// --- names/required-missing -------------------------------------------------

/// Korpus `aria_naming_roles`: je eine unbenannte Rolle aus
/// `aria-command-name`, `-input-field-name`, `-meter-name`,
/// `-progressbar-name`, `-toggle-field-name`, `-treeitem-name`.
const ARIA_NAMING_ROLES: &str = r#"
    <div role="menuitem" tabindex="0" id="menuitem"></div>
    <div role="slider" tabindex="0" aria-valuenow="5" aria-valuemin="0" aria-valuemax="10" id="slider"></div>
    <div role="meter" aria-valuenow="5" aria-valuemin="0" aria-valuemax="10" id="meter"></div>
    <div role="progressbar" aria-valuenow="50" aria-valuemin="0" aria-valuemax="100" id="progressbar"></div>
    <div role="switch" tabindex="0" aria-checked="false" id="switch"></div>
    <div role="tree"><div role="treeitem" tabindex="0" id="treeitem"></div></div>
    <div role="dialog" id="dialog"></div>"#;

#[test]
fn unbenannte_rollen_korpus_aria_naming_roles() {
    for r in beide(ARIA_NAMING_ROLES) {
        assert_eq!(
            an(ARIA_NAMING_ROLES, &r, "names/required-missing"),
            [
                "menuitem",
                "meter",
                "progressbar",
                "slider",
                "switch",
                "treeitem"
            ]
        );
        assert_eq!(an(ARIA_NAMING_ROLES, &r, "dialog/name-missing"), ["dialog"]);
        let meter = r
            .findings
            .iter()
            .find(|f| f.rule_id == "names/required-missing" && f.wcag == ["1.1.1"]);
        assert!(meter.is_some(), "meter/progressbar unter 1.1.1");
    }
}

#[test]
fn benannte_rollen_bleiben_still_auditmysite_all_named_elements_pass() {
    let src = r#"
        <div role="menuitem" tabindex="0">Öffnen</div>
        <div role="slider" aria-label="Lautstärke" tabindex="0" aria-valuenow="5"></div>
        <span id="l">Speicher</span><div role="meter" aria-labelledby="l" aria-valuenow="5"></div>
        <div role="progressbar" title="Laden" aria-valuenow="5"></div>
        <div role="radiogroup" aria-label="Optionen"><div role="radio" aria-checked="false">A</div></div>
        <div role="dialog" aria-label="Einstellungen" aria-modal="true"></div>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "names/required-missing"), 0, "{:?}", r.findings);
        assert_eq!(anzahl(&r, "dialog/name-missing"), 0);
    }
}

#[test]
fn eigene_familien_melden_nicht_doppelt_korpus_aria_and_widgets() {
    // `<button aria-label="">` meldet `buttons/name-missing`, ein leerer Link
    // `links/name-missing` — nicht zusätzlich `names/required-missing`.
    let src = r#"<button aria-label=""><svg aria-hidden="true" width="10" height="10"></svg></button>
                 <a href="/x"></a>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "buttons/name-missing"), 1);
        assert_eq!(anzahl(&r, "links/name-missing"), 1);
        assert_eq!(anzahl(&r, "names/required-missing"), 0, "{:?}", r.findings);
    }
}

#[test]
fn natives_feld_ohne_label_meldet_forms_korpus_missing_label() {
    // Ohne jede Beschriftung: `forms/label-missing`. Mit einem leeren Label:
    // nur die Namensregel sieht, dass es leer ausgeht.
    let src = r#"<form><input type="text" name="email" id="ohne">
                 <label for="leer"></label><input type="text" id="leer">
                 <label for="gut">E-Mail</label><input type="text" id="gut"></form>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/label-missing"), ["ohne"]);
        assert_eq!(an(src, &r, "names/required-missing"), ["leer"]);
    }
}

#[test]
fn native_optionen_und_versteckte_rollen_bleiben_still() {
    let src = r#"<label>Land <select><option value=""></option><option>DE</option></select></label>
                 <div hidden><div role="slider" tabindex="0"></div></div>
                 <div role="menuitem" aria-hidden="true"></div>
                 <div role="tab">ohne Pflicht</div><div role="menu"></div>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "names/required-missing"), 0, "{:?}", r.findings);
    }
}

// --- names/symbol-only ------------------------------------------------------

#[test]
fn symbol_als_name_auditmysite_test_icon_only_name_flagged() {
    let src = r#"<button aria-label="×" id="x"><svg aria-hidden="true"></svg></button>
                 <button id="inhalt">×</button>
                 <button aria-label="Ä">Ä</button>
                 <button aria-label="Schließen">×</button>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "names/symbol-only"), ["x"]);
        assert_eq!(outcomes(&r, "names/symbol-only"), [Outcome::Review]);
    }
}

// --- dialog/* ----------------------------------------------------------------

#[test]
fn dialog_ohne_namen_korpus_iframe_widget_rules() {
    // `consent-dialog` ist sichtbar und namenlos; `hidden-dialog` steckt in
    // einem versteckten Rahmen und wird nicht gemeldet.
    let src = r#"<div role="dialog" id="consent-dialog"><p>Cookies?</p><button>OK</button></div>
                 <div hidden><div role="dialog" id="hidden-dialog"></div></div>
                 <div role="alertdialog" id="alarm"></div>
                 <dialog id="zu"></dialog>
                 <dialog open id="offen"></dialog>
                 <dialog open aria-labelledby="h"><h2 id="h">Titel</h2></dialog>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "dialog/name-missing"),
            ["alarm", "consent-dialog", "offen"]
        );
        assert_eq!(anzahl(&r, "names/required-missing"), 0);
    }
}

#[test]
fn dialog_ohne_aria_modal_ist_review_auditmysite_dialog_rules() {
    let src = r#"<div role="dialog" aria-label="A" id="ohne"></div>
                 <div role="dialog" aria-label="B" aria-modal="true"></div>
                 <div role="alertdialog" aria-label="C"></div>
                 <dialog open aria-label="D"></dialog>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "dialog/modal-unmarked"), ["ohne"]);
        assert_eq!(outcomes(&r, "dialog/modal-unmarked"), [Outcome::Review]);
    }
}

// --- summary/name-missing ---------------------------------------------------

#[test]
fn leere_summary_korpus_summary_empty() {
    let src =
        r#"<details id="d"><summary id="s"></summary><p>Hidden details content.</p></details>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "summary/name-missing"), ["s"]);
        // `accname` gibt der summary die Rolle `button`: kein zweiter Befund.
        assert_eq!(anzahl(&r, "buttons/name-missing"), 0, "{:?}", r.findings);
    }
}

#[test]
fn summary_mit_text_und_zweite_summary_bleiben_still() {
    let src = r#"<details><summary>Mehr</summary><summary></summary></details>
                 <details open><summary aria-label="Optionen"></summary></details>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "summary/name-missing"), 0, "{:?}", r.findings);
    }
}

// --- status/live-overridden -------------------------------------------------

#[test]
fn alert_mit_polite_korpus_aria_and_widgets() {
    let src = r#"<div role="alert" aria-live="polite">A critical error occurred</div>"#;
    for r in beide(src) {
        assert_eq!(outcomes(&r, "status/live-overridden"), [Outcome::Review]);
    }
}

#[test]
fn live_off_ist_verstoss_auditmysite_status_messages() {
    let src = r#"<div role="alert" aria-live="off" id="alert"></div>
                 <div role="status" aria-live="assertive" id="status"></div>
                 <div role="log" aria-live="off" id="log"></div>
                 <output aria-live="OFF" id="output"></output>
                 <div role="log" aria-live="assertive"></div>
                 <div role="status" aria-live="polite"></div>
                 <div role="alert" aria-live="assertive"></div>
                 <div role="timer" aria-live="off"></div>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "status/live-overridden"),
            ["alert", "log", "output", "status"]
        );
        let fails = outcomes(&r, "status/live-overridden")
            .into_iter()
            .filter(|o| *o == Outcome::Fail)
            .count();
        assert_eq!(fails, 3);
    }
}

#[test]
fn leere_status_region_ist_kein_befund_gov_uk() {
    // auditmysite `dialog_rules`: zwei leere `role="status"` auf www.gov.uk
    // waren kein Mangel — ARIA verlangt für Live-Regionen keinen Namen.
    let src = r#"<div role="status"></div><div role="status" aria-live="polite"></div>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "status/live-overridden"), 0);
        assert_eq!(anzahl(&r, "names/required-missing"), 0);
    }
}

// --- label-in-name/mismatch -------------------------------------------------

/// auditmysite `tests/fixtures/label_in_name.html` (#513).
const LABEL_IN_NAME: &str = r#"
    <button type="button" aria-label="Loads slowly. Google penalizes slow pages." id="bloecke">
        <h3>Loads slowly.</h3><p>Google penalizes slow pages.</p>
    </button>
    <button type="button" aria-label="Site is slow. Tap for the fix." id="karte">
        <h3>Site is slow.</h3>
        <p>Loading takes too long and visitors leave.</p>
        <h3>The fix</h3>
        <p>Move to edge hosting and ship a static build for sub-second loads.</p>
    </button>
    <button type="button" aria-label="Submit order" id="falsch">Cancel</button>"#;

#[test]
fn label_in_name_fixture_513() {
    for r in beide(LABEL_IN_NAME) {
        assert_eq!(
            an(LABEL_IN_NAME, &r, "label-in-name/mismatch"),
            ["falsch", "karte"]
        );
        let f = |id: &str| {
            let a = html(LABEL_IN_NAME);
            let k = elements(&a)
                .find(|n| n.attr("id") == Some(id))
                .unwrap()
                .id()
                .to_string();
            r.findings
                .iter()
                .find(|f| {
                    f.rule_id == "label-in-name/mismatch" && f.location.node.as_deref() == Some(&k)
                })
                .unwrap()
                .outcome
        };
        assert_eq!(f("falsch"), Outcome::Fail);
        assert_eq!(f("karte"), Outcome::Review);
    }
}

#[test]
fn label_in_name_korpus_forms_and_misc() {
    let src = r#"<form><button aria-label="Submit the entire form now">Send</button></form>"#;
    for r in beide(src) {
        assert_eq!(outcomes(&r, "label-in-name/mismatch"), [Outcome::Fail]);
    }
}

#[test]
fn label_in_name_ohne_befund() {
    let src = r#"<button aria-label="Close dialog"><span aria-hidden="true">×</span></button>
                 <button aria-label="Close"><span aria-hidden="true">✕</span> Close</button>
                 <a href="/p" aria-label="Read more about pricing">Read more</a>
                 <button aria-label="schließen">Schließen</button>
                 <a href="/q" aria-label="Pricing">Read more about pricing</a>
                 <button>Senden</button>
                 <div aria-hidden="true"><button aria-label="Weg">Anders</button></div>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "label-in-name/mismatch"), 0, "{:?}", r.findings);
    }
}

#[test]
fn label_in_name_gilt_auch_fuer_links_und_labelledby() {
    let src = r#"<span id="n">Startseite</span>
                 <a href="/" aria-labelledby="n" id="link">Home</a>
                 <div role="tab" aria-label="Erster" id="tab">Zweiter</div>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "label-in-name/mismatch"), ["link", "tab"]);
    }
}
