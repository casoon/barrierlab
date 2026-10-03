//! Formularregeln (casoon/barrierlab#16).
//!
//! Die Fälle stammen aus auditmysite: dem Detection-Korpus
//! (`tests/fixtures/detection_corpus/`), den Tests der portierten Regeln und
//! den Issues #643, #656, #658 und #728; der Name jedes Tests nennt die
//! Quelle. Wo ein Korpusfall ein Element ohne `id` prüft, ist eine `id`
//! ergänzt, damit der Test den Befund verorten kann. Jeder Fall läuft gegen
//! zwei Hosts: Rolle und Name aus `accname` und einen, der sich wie Chrome
//! verhält (Rolle für Passwort- und Datumsfelder, Quelle des Namens).

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, NameSource, Node, Semantics, elements};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run_with_semantics;
use accname::IdIndex;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

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

struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
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

fn typ(n: ArenaNode<'_>) -> String {
    n.attr("type").unwrap_or("text").to_ascii_lowercase()
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        if self.chrome && node.is_element("input") {
            match typ(node).as_str() {
                "password" => return Some("textbox".into()),
                "date" | "datetime-local" | "month" | "week" => return Some("Date".into()),
                "time" => return Some("InputTime".into()),
                _ => {}
            }
        }
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    /// Wie Chrome: die Quelle, aus der der Name tatsächlich stammt.
    fn name_source<'n>(&'n self, node: Self::N<'n>) -> Option<NameSource> {
        if !self.chrome {
            return None;
        }
        let gesetzt = |a: &str| node.attr(a).is_some_and(|v| !v.trim().is_empty());
        let beschriftet = node.attr("id").is_some_and(|id| {
            elements(self.doc).any(|l| l.is_element("label") && l.attr("for") == Some(id))
        }) || a11y_dom::closest(node, "label").is_some();
        Some(if gesetzt("aria-labelledby") {
            NameSource::AriaLabelledBy
        } else if gesetzt("aria-label") {
            NameSource::AriaLabel
        } else if beschriftet {
            NameSource::Label
        } else if gesetzt("title") {
            NameSource::Title
        } else if gesetzt("placeholder") {
            NameSource::Placeholder
        } else {
            NameSource::Contents
        })
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

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

fn urteile(r: &Report, id: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(|f| (f.outcome, f.severity))
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

// --- Korpus forms_and_misc, forms_extended ----------------------------------

/// Korpus `forms_and_misc` (Formularteil): `autocomplete-valid`,
/// `identify-purpose`, `label-title-only`, `form-field-group`.
const FORMS_AND_MISC: &str = r#"
    <form>
        <label for="misc1">Misc field</label>
        <input type="text" id="misc1" autocomplete="not-a-real-token">
        <label for="user-email">Email</label>
        <input type="email" id="user-email" name="user_email">
        <input type="text" id="titel" title="Search field only">
        <input type="radio" name="color" id="c1"><label for="c1">Red</label>
        <input type="radio" name="color" id="c2"><label for="c2">Blue</label>
        <button aria-label="Submit the entire form now">Send</button>
    </form>"#;

#[test]
fn korpus_forms_and_misc() {
    let src = FORMS_AND_MISC;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/autocomplete-invalid"), ["misc1"]);
        assert_eq!(
            urteile(&r, "forms/autocomplete-invalid"),
            [(Outcome::Fail, Severity::Low)]
        );
        assert_eq!(an(src, &r, "forms/purpose-missing"), ["user-email"]);
        assert_eq!(
            urteile(&r, "forms/purpose-missing"),
            [(Outcome::Review, Severity::Medium)]
        );
        assert_eq!(an(src, &r, "forms/title-only-label"), ["titel"]);
        assert_eq!(
            urteile(&r, "forms/title-only-label"),
            [(Outcome::Review, Severity::Medium)]
        );
        assert_eq!(an(src, &r, "forms/group-missing"), ["c1", "c2"]);
        assert_eq!(
            urteile(&r, "forms/group-missing")[0],
            (Outcome::Fail, Severity::Medium)
        );
        assert_eq!(anzahl(&r, "forms/no-submit"), 0);
    }
}

/// Korpus `forms_extended`: `input-error-message` und
/// `aria-invalid-without-describedby` (ein Befund), `input-no-context-change`,
/// `focus-no-context-change`, `label-title-only`, `redundant-entry`.
const FORMS_EXTENDED: &str = r#"
    <input type="text" id="ungueltig" aria-invalid="true" aria-label="Email">
    <select id="springen" aria-label="Navigate to page" onchange="location.href=this.value">
        <option value="/a">Page A</option>
        <option value="/b">Page B</option>
    </select>
    <div id="fokus" onfocus="this.style.color='red'" tabindex="0">Non-interactive element with an onfocus handler</div>
    <input type="text" id="suche" title="Search the site">
    <form id="kasse">
        <label for="email1">Email</label>
        <input id="email1" name="email" type="email">
        <label for="billing-email">Billing Email</label>
        <input id="billing-email" name="billing_email" type="email">
        <button type="submit">Submit</button>
    </form>"#;

#[test]
fn korpus_forms_extended() {
    let src = FORMS_EXTENDED;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/error-unidentified"), ["ungueltig"]);
        assert_eq!(
            urteile(&r, "forms/error-unidentified"),
            [(Outcome::Fail, Severity::Medium)]
        );
        assert_eq!(an(src, &r, "context/on-input"), ["springen"]);
        assert_eq!(
            urteile(&r, "context/on-input"),
            [(Outcome::Fail, Severity::Medium)]
        );
        assert_eq!(an(src, &r, "context/on-focus"), ["fokus"]);
        assert_eq!(
            urteile(&r, "context/on-focus"),
            [(Outcome::Review, Severity::High)]
        );
        assert_eq!(an(src, &r, "forms/title-only-label"), ["suche"]);
        assert_eq!(an(src, &r, "forms/redundant-entry"), ["kasse"]);
        assert_eq!(
            urteile(&r, "forms/redundant-entry"),
            [(Outcome::Review, Severity::Medium)]
        );
    }
}

// --- forms/error-unidentified -----------------------------------------------

#[test]
fn fehlerbeschreibung_zaehlt_auditmysite_error_identification() {
    let src = r#"
        <label>E-Mail <input id="beschrieben" aria-invalid="true" aria-describedby="e1"></label>
        <p id="e1">Bitte eine gültige Adresse eingeben.</p>
        <label>PLZ <input id="fehlermeldung" aria-invalid="true" aria-errormessage="e2"></label>
        <p id="e2">Fünf Ziffern.</p>
        <label>Ort <input id="leer" aria-invalid="true" aria-describedby="e3"></label>
        <p id="e3"></p>
        <label>Land <input id="versteckt" aria-invalid="grammar" aria-describedby="e4"></label>
        <p id="e4" hidden>Unbekanntes Land.</p>
        <label>Name <input id="gueltig" aria-invalid="false"></label>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/error-unidentified"), ["leer"]);
    }
}

// --- forms/group-missing (auditmysite#643) ----------------------------------

/// Korpus `form_heuristics_flagged`: zwei Kontrollkästchen mit demselben
/// `name` ohne Gruppe; das Datumsfeld ohne Anleitung ist `REVIEW`.
const HEURISTICS_FLAGGED: &str = r#"
    <form>
        <label for="dob">Date of birth</label>
        <input type="text" id="dob" name="dob">
        <p>Interests</p>
        <label><input type="checkbox" id="news" name="interests" value="news"> News</label>
        <label><input type="checkbox" id="events" name="interests" value="events"> Events</label>
        <button type="submit">Register</button>
    </form>"#;

#[test]
fn korpus_form_heuristics_flagged() {
    let src = HEURISTICS_FLAGGED;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/group-missing"), ["events", "news"]);
        assert_eq!(an(src, &r, "forms/instructions-missing"), ["dob"]);
        assert_eq!(
            urteile(&r, "forms/instructions-missing"),
            [(Outcome::Review, Severity::Low)]
        );
    }
}

/// Korpus `form_heuristics_clean` (auditmysite#643): gruppierte Optionen,
/// Anleitung per `aria-describedby` gleich welchen Wortlauts, „passiert" ist
/// kein Reisepass, ein einzelnes Kontrollkästchen ist keine Gruppe.
const HEURISTICS_CLEAN: &str = r#"
    <form>
        <fieldset>
            <legend>Mode</legend>
            <label><input type="radio" name="mode" value="broken" checked> Broken</label>
            <label><input type="radio" name="mode" value="fixed"> Fixed</label>
        </fieldset>
        <label for="booking-date">Date</label>
        <input type="text" id="booking-date" name="date" inputmode="numeric" aria-describedby="booking-date-hint">
        <p id="booking-date-hint">Day of travel</p>
        <label for="report-what">Was ist passiert?</label>
        <textarea id="report-what" name="what" rows="4" aria-describedby="report-what-hint"></textarea>
        <span id="report-what-hint">Was Sie erwartet haben und was stattdessen passiert ist</span>
        <label><input type="checkbox" class="bill-sr"> Screen reader view</label>
        <label><input type="checkbox" name="newsletter"> Send me the newsletter</label>
        <button type="submit">Search</button>
    </form>"#;

#[test]
fn korpus_form_heuristics_clean() {
    let src = HEURISTICS_CLEAN;
    for r in beide(src) {
        for id in [
            "forms/group-missing",
            "forms/instructions-missing",
            "forms/group-name-missing",
            "forms/required-unmarked",
        ] {
            assert_eq!(anzahl(&r, id), 0, "{id}: {:?}", r.findings);
        }
    }
}

#[test]
fn checkbox_mengen_je_formular_auditmysite_643() {
    // Gleicher `name` in zwei Formularen ist keine Menge; das `form`-Attribut
    // ordnet ein Kästchen außerhalb seinem Formular zu. Ein einzelnes
    // Optionsfeld braucht trotzdem eine Gruppe.
    let src = r#"
        <form id="a"><label><input type="checkbox" id="a1" name="x"> A</label></form>
        <form id="b"><label><input type="checkbox" id="b1" name="x"> B</label></form>
        <label><input type="checkbox" id="b2" name="x" form="b"> B2</label>
        <div role="group" aria-label="Farben">
            <label><input type="checkbox" name="f"> Rot</label>
            <label><input type="checkbox" name="f"> Blau</label>
        </div>
        <label><input type="radio" id="allein" name="r"> Allein</label>
        <div role="radiogroup" aria-label="Größe"><div role="radio" aria-checked="false">S</div></div>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/group-missing"), ["allein", "b1", "b2"]);
    }
}

// --- forms/group-name-missing -------------------------------------------------

#[test]
fn gruppe_ohne_namen_auditmysite_instructions() {
    let src = r#"
        <fieldset id="ohne"><label>A <input type="text"></label></fieldset>
        <fieldset><legend>Adresse</legend><label>B <input type="text"></label></fieldset>
        <div role="group" id="rolle"><label>C <input type="checkbox"></label></div>
        <div role="group" id="leer"><p>Kein Feld</p></div>
        <details open><summary>Mehr</summary><label>D <input type="text"></label></details>
        <div role="radiogroup" id="radios"><div role="radio" aria-checked="false">S</div></div>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/group-name-missing"), ["ohne", "rolle"]);
        assert_eq!(an(src, &r, "names/required-missing"), ["radios"]);
    }
}

// --- forms/no-submit (auditmysite#728) ----------------------------------------

/// Korpus `form_toggles_without_submit`: Schalter ohne `action`, Textfeld und
/// Absende-Element sind ein Prüfhinweis; ein Formular mit `action` bleibt ein
/// Verstoß.
const TOGGLES: &str = r#"
    <form id="toggles">
      <fieldset>
        <legend>Remove accessibility</legend>
        <input type="checkbox" id="t-labels"><label for="t-labels">Remove labels</label>
        <input type="checkbox" id="t-contrast"><label for="t-contrast">Remove contrast</label>
        <input type="radio" name="size" id="t-small"><label for="t-small">Small</label>
        <input type="radio" name="size" id="t-large"><label for="t-large">Large</label>
      </fieldset>
      <button type="reset">Reset</button>
    </form>
    <p class="demo">Sample text.</p>
    <form id="filter" action="/results">
      <fieldset>
        <legend>Filter</legend>
        <input type="checkbox" id="f-open" name="open"><label for="f-open">Open only</label>
      </fieldset>
    </form>"#;

#[test]
fn korpus_form_toggles_without_submit() {
    let src = TOGGLES;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/no-submit"), ["filter", "toggles"]);
        let mut u = urteile(&r, "forms/no-submit");
        u.sort_by_key(|(o, _)| *o == Outcome::Review);
        assert_eq!(
            u,
            [
                (Outcome::Fail, Severity::Medium),
                (Outcome::Review, Severity::Low)
            ]
        );
    }
}

#[test]
fn absende_element_auch_ueber_form_attribut() {
    let src = r#"
        <form id="suche" action="/s"><label>Suche <input type="search" name="q"></label></form>
        <button form="suche">Suchen</button>
        <form id="text"><label>Notiz <input type="text" name="n"></label></form>
        <form id="leer"><input type="hidden" name="t"></form>
        <form id="bild"><label>Q <input name="q"></label><input type="image" alt="Los" src="go.png"></form>
        <form id="aus"><label>Q <input name="q"></label><button disabled>Los</button></form>"#;
    for r in beide(src) {
        // Mit Textfeld kann die Eingabetaste absenden: Verstoß, kein Hinweis.
        assert_eq!(an(src, &r, "forms/no-submit"), ["aus", "text"]);
        assert!(
            urteile(&r, "forms/no-submit")
                .iter()
                .all(|u| *u == (Outcome::Fail, Severity::Medium))
        );
    }
}

// --- forms/redundant-entry ----------------------------------------------------

#[test]
fn wiederholte_angabe_auditmysite_redundant_entry() {
    let src = r#"
        <form id="doppelt">
            <label>Shipping street address <input name="s1"></label>
            <label>Billing street address <input name="s2"></label>
        </form>
        <form id="bestaetigung">
            <label>E-Mail <input name="m1"></label>
            <label>E-Mail bestätigen <input name="m2"></label>
        </form>
        <form id="autofill">
            <label>Shipping street address <input name="a1" autocomplete="shipping street-address"></label>
            <label>Billing street address <input name="a2" autocomplete="billing street-address"></label>
        </form>
        <form id="uebernahme">
            <label>Shipping street address <input name="u1"></label>
            <label><input type="checkbox" name="same"> Same as shipping</label>
            <label>Billing street address <input name="u2"></label>
        </form>
        <form id="vorbelegt">
            <label>Shipping email <input name="v1"></label>
            <label>Billing email <input name="v2" value="a@b.de"></label>
        </form>
        <form id="verschieden">
            <label>Email <input name="e"></label>
            <label>Phone <input name="p"></label>
        </form>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/redundant-entry"), ["doppelt"]);
    }
}

// --- forms/purpose-missing (auditmysite#658) ----------------------------------

/// Korpus `input_purpose_name`: Der Name einer Ansicht, eines Projekts oder
/// einer Datei ist keine Angabe über die Person; „Hotel" ist kein Telefon.
const INPUT_PURPOSE_NAME: &str = r#"
    <form>
        <label for="view-name">Name</label>
        <input id="view-name" type="text" placeholder="My view">
        <label for="project-title">Project name</label>
        <input id="project-title" type="text">
        <label for="upload-target">Dateiname</label>
        <input id="upload-target" type="text">
        <label for="stay">Hotel</label>
        <input id="stay" type="text">
        <button type="submit">Save the view</button>
    </form>
    <form>
        <label for="your-name">Your name</label>
        <input id="your-name" type="text">
        <label for="given">Vorname</label>
        <input id="given" type="text">
        <label for="phone-de">Telefonnummer</label>
        <input id="phone-de" type="text">
        <label for="zip-de">Postleitzahl</label>
        <input id="zip-de" type="text">
        <button type="submit">Send</button>
    </form>"#;

#[test]
fn korpus_input_purpose_name() {
    let src = INPUT_PURPOSE_NAME;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "forms/purpose-missing"),
            ["given", "phone-de", "your-name", "zip-de"]
        );
    }
}

#[test]
fn zweck_aus_id_name_und_typ_auditmysite_identify_purpose() {
    let src = r#"
        <label>Kontakt <input id="k" name="email_address"></label>
        <label>Rufnummer <input id="t" type="tel"></label>
        <label>IP-Adresse <input id="ip" name="ip_address"></label>
        <label>Suche <input id="q" type="search" name="q"></label>
        <label>E-Mail <input id="gesetzt" type="email" autocomplete="email"></label>
        <label>E-Mail <input id="aus" type="email" autocomplete="off"></label>
        <label>Passwort <input id="pw" type="password"></label>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "forms/purpose-missing"),
            ["aus", "k", "pw", "t"]
        );
    }
}

// --- forms/autocomplete-invalid -----------------------------------------------

#[test]
fn autocomplete_grammatik_auditmysite_input_purpose() {
    let src = r#"
        <label>Straße <input id="a" autocomplete="shipping street-address"></label>
        <label>Telefon <input id="b" autocomplete="work tel"></label>
        <label>Name <input id="c" autocomplete="work name"></label>
        <label>Name <input id="d" autocomplete="foobar"></label>
        <label>Land <select id="e" autocomplete="country"><option>DE</option></select></label>
        <input type="hidden" id="f" autocomplete="nonsense">
        <label>Code <input id="g" autocomplete="one-time-code webauthn"></label>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/autocomplete-invalid"), ["c", "d"]);
    }
}

// --- forms/required-unmarked, forms/instructions-missing ----------------------

#[test]
fn pflichtfeld_ohne_hinweis_auditmysite_instructions() {
    let src = r#"
        <label>Email <input id="ohne" required></label>
        <label>Email * <input id="stern" required></label>
        <label>Name (required) <input id="wort" required></label>
        <label>Telefon <input id="beschrieben" aria-required="true" aria-describedby="h"></label>
        <span id="h">Pflichtfeld</span>
        <label>Notiz <input id="optional"></label>
        <label><input type="checkbox" id="agb" required> Ich akzeptiere die AGB</label>
        <label>Passwort <input type="password" id="pw" required></label>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "forms/required-unmarked"),
            ["agb", "ohne", "pw"]
        );
        assert!(
            urteile(&r, "forms/required-unmarked")
                .iter()
                .all(|u| *u == (Outcome::Review, Severity::Medium))
        );
    }
}

#[test]
fn formatanleitung_auditmysite_656() {
    // Native Datumsfelder bringen ihr Format mit; ein Zahlenfeld braucht nur
    // über seine Beschriftung eine Anleitung.
    let src = r#"
        <label>Order date <input type="date" id="nativ"></label>
        <label>Quantity <input type="number" id="menge"></label>
        <label>Postal code <input type="number" id="plz"></label>
        <label>Date (DD.MM.YYYY) <input id="mit-format"></label>
        <label>Telefon <input id="platzhalter" placeholder="+49 30 1234567"></label>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/instructions-missing"), ["plz"]);
    }
}

// --- forms/title-only-label ---------------------------------------------------

#[test]
fn nur_titel_auditmysite_label_title_only() {
    let src = r#"
        <input type="text" id="nur" title="Suche">
        <input type="text" id="label" title="Suche"><label for="label">Suche</label>
        <input type="text" aria-label="Suche" title="Suche">
        <input type="text" id="platzhalter" title="Suche" placeholder="Suche">
        <input type="range" id="regler" title="Lautstärke">
        <button title="Senden">Senden</button>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/title-only-label"), ["nur", "regler"]);
        // Mit placeholder meldet die Strukturregel den Mangel.
        assert_eq!(an(src, &r, "forms/placeholder-as-label"), ["platzhalter"]);
    }
}

// --- context/on-input -----------------------------------------------------------

/// Korpus `on_input_context_change`, statischer Teil: Ein Handler, der den
/// Kontext sichtbar wechselt, ist ein Verstoß; einer, der eine Funktion
/// aufruft, `REVIEW` — deren Quelltext liest erst der Host. Ohne Handler kein
/// Befund (auditmysite#657); die Namensvermutung („Language") bleibt im Host.
const ON_INPUT: &str = r##"
    <label for="filter-preset">Filter preset</label>
    <select id="filter-preset"><option>All</option><option>Active</option></select>
    <label for="sort-preset">Sort preset</label>
    <select id="sort-preset" onchange="applySort(this.value)"><option>Name</option></select>
    <label for="grid-language">Language</label>
    <select id="grid-language"><option>English</option></select>
    <label for="jump">Jump to page</label>
    <select id="jump" onchange="goTo(this.value)"><option value="#a">Page A</option></select>
    <form action="#submitted">
        <label for="per-page">Rows per page</label>
        <select id="per-page" name="per_page" onchange="this.form.submit()"><option>10</option></select>
    </form>
    <label><input type="radio" id="fenster" name="v" onchange="window.open(this.value)"> Neu</label>"##;

#[test]
fn korpus_on_input_context_change() {
    let src = ON_INPUT;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "context/on-input"),
            ["fenster", "jump", "per-page", "sort-preset"]
        );
        let review = r
            .findings
            .iter()
            .filter(|f| f.rule_id == "context/on-input" && f.outcome == Outcome::Review)
            .count();
        assert_eq!(review, 2);
    }
}

// --- context/autofocus ------------------------------------------------------------

#[test]
fn autofocus_auditmysite_on_focus() {
    let src = r#"
        <button id="knopf" autofocus>Los</button>
        <label>Suche <input autofocus></label>
        <div role="searchbox" contenteditable="true" autofocus aria-label="S"></div>
        <button onfocus="track()">Fokus</button>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "context/autofocus"), ["knopf"]);
        assert_eq!(
            urteile(&r, "context/autofocus"),
            [(Outcome::Review, Severity::Medium)]
        );
        assert_eq!(anzahl(&r, "context/on-focus"), 0);
    }
}

// --- auth/captcha -------------------------------------------------------------------

/// Korpus `accessible_authentication`, statischer Teil: das Bild-Captcha im
/// Anmeldeformular ist `REVIEW`, das im Kontaktformular gehört nicht zu
/// 3.3.8. Der Einfüge-Test bleibt im Host.
const AUTH: &str = r#"
  <form id="login-captcha" action="/login" method="post">
    <label for="pw-captcha">Password</label>
    <input id="pw-captcha" name="password4" type="password" autocomplete="current-password">
    <img id="captcha-image" src="c.gif" alt="captcha" width="120" height="40">
    <label for="captcha-answer">Type the characters shown</label>
    <input id="captcha-answer" name="captcha" type="text">
    <button type="submit">Sign in</button>
  </form>
  <form id="contact" action="/contact" method="post">
    <label for="msg">Message</label>
    <textarea id="msg" name="message"></textarea>
    <img id="contact-captcha" src="c.gif" alt="captcha" width="120" height="40">
    <button type="submit">Send</button>
  </form>
  <form id="otp" action="/verify">
    <label>Code <input autocomplete="one-time-code"></label>
    <div class="g-recaptcha" data-size="invisible"></div>
    <button>Verify</button>
  </form>
  <form id="recaptcha" action="/login">
    <label>Passwort <input type="password"></label>
    <div class="g-recaptcha"></div>
    <button>Anmelden</button>
  </form>"#;

#[test]
fn korpus_accessible_authentication() {
    let src = AUTH;
    for r in beide(src) {
        assert_eq!(an(src, &r, "auth/captcha"), ["login-captcha", "recaptcha"]);
        assert!(
            urteile(&r, "auth/captcha")
                .iter()
                .all(|u| *u == (Outcome::Review, Severity::Medium))
        );
    }
}

// --- Beschriftung: labels.rs check_form_control (auditmysite#690) -------------------

/// Korpus `missing_label` und die Lücke aus auditmysite `labels.rs`: ein
/// Feld ganz ohne Beschriftung meldet `forms/label-missing`; ARIA-Widgets ohne
/// Namen, ein leeres oder ins Leere zeigendes `aria-labelledby` und ein leeres
/// `<label for>` meldet `names/required-missing`. Keine neue Kennung nötig.
#[test]
fn beschriftungsluecke_ist_abgedeckt_korpus_missing_label() {
    let src = r#"
        <form>
            <input type="text" name="email" id="ohne">
            <label for="leer"></label><input type="text" id="leer">
            <input type="text" id="labelledby-leer" aria-labelledby="">
            <input type="text" id="labelledby-tot" aria-labelledby="gibts-nicht">
            <div role="textbox" id="widget-text" tabindex="0"></div>
            <div role="combobox" id="widget-combo" aria-expanded="false" tabindex="0"></div>
            <div role="slider" id="widget-slider" aria-valuenow="1" tabindex="0"></div>
            <div role="spinbutton" id="widget-spin" aria-valuenow="1" tabindex="0"></div>
            <div role="searchbox" id="widget-suche" tabindex="0"></div>
            <div role="switch" id="widget-schalter" aria-checked="false" tabindex="0"></div>
            <button type="submit">Send</button>
        </form>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/label-missing"), ["ohne"]);
        assert_eq!(
            an(src, &r, "names/required-missing"),
            [
                "labelledby-leer",
                "labelledby-tot",
                "leer",
                "widget-combo",
                "widget-schalter",
                "widget-slider",
                "widget-spin",
                "widget-suche",
                "widget-text",
            ]
        );
    }
}

// --- Geltungsbereich ------------------------------------------------------------------

#[test]
fn versteckte_formulare_werden_nicht_geprueft() {
    let src = r#"
        <div hidden>
            <form><input type="radio" name="x"><input aria-invalid="true" autocomplete="quatsch"></form>
        </div>
        <div aria-hidden="true">
            <form><label>Email <input type="email" required></label></form>
        </div>"#;
    for r in beide(src) {
        let formulare: Vec<&str> = r
            .findings
            .iter()
            .map(|f| f.rule_id.as_str())
            .filter(|id| {
                id.starts_with("forms/") || id.starts_with("context/") || id.starts_with("auth/")
            })
            .collect();
        assert!(formulare.is_empty(), "{formulare:?}");
    }
}

#[cfg(feature = "de")]
#[test]
fn formularbefunde_folgen_der_sprache() {
    let src = format!("{FORMS_AND_MISC}{FORMS_EXTENDED}{TOGGLES}{AUTH}{ON_INPUT}");
    let a = html(&src);
    let host = MitSemantik {
        doc: &a,
        ids: IdIndex::build(a.root()),
        chrome: false,
    };
    let en = run_with_semantics(&host);
    let de = a11y_rules::run_with_semantics_in(&host, a11y_rules::Locale::De);
    let eigene = |r: &Report| -> Vec<(String, String)> {
        r.findings
            .iter()
            .filter(|f| {
                f.rule_id.starts_with("forms/")
                    || f.rule_id.starts_with("context/")
                    || f.rule_id.starts_with("auth/")
            })
            .map(|f| (f.rule_id.clone(), f.message.clone()))
            .collect()
    };
    let (en, de) = (eigene(&en), eigene(&de));
    assert_eq!(en.len(), de.len());
    assert!(en.len() > 10);
    for (e, d) in en.iter().zip(&de) {
        assert_eq!(e.0, d.0);
        assert_ne!(e.1, d.1, "ohne deutsche Fassung: {}", e.0);
    }
}

/// Geographia quiz options (astro-post-audit#75) have no input format.
#[test]
fn geographia_quiz_options_need_no_format_instructions() {
    let src = r#"<label><input id="radio" type="radio">Das Datum einer wichtigen Erfindung</label>
        <label><input id="checkbox" type="checkbox">The date of an important invention</label>
        <label><input id="text" type="text">Date of birth</label>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/instructions-missing"), ["text"]);
    }
}

/// Geographia station/country lookups are not personal data (astro-post-audit#76).
#[test]
fn geographia_search_fields_need_no_personal_autocomplete() {
    let src = r#"<input type="search" aria-label="Land oder Region">
        <div role="search"><input type="text" aria-label="Country"></div>
        <input type="text" role="combobox" aria-label="Country">
        <input id="personal" type="text" aria-label="Country">"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "forms/purpose-missing"), ["personal"]);
    }
}
