//! Formularregeln nach WCAG 2.2 (1.3.1, 1.3.5, 3.2.1, 3.2.2, 3.3.1, 3.3.2,
//! 3.3.7, 3.3.8), dem HTML-Standard („Autofill", Formulare) und WAI-ARIA 1.2
//! (`aria-invalid`, `aria-errormessage`, `aria-required`).
//!
//! Portiert aus auditmysite (`form_rules`, `error_identification`,
//! `input_purpose`, `identify_purpose`, `redundant_entry`, `label_title_only`,
//! `on_focus`, `instructions`, der statische Teil von `on_input` und
//! `accessible_authentication`; casoon/barrierlab#16). auditmysite ist der
//! Vergleichspunkt, nicht die Norm — Abweichungen stehen an der jeweiligen
//! Stelle und im CHANGELOG.
//!
//! Kennungen: `forms/*` für Formularfelder und Formulare, `context/*` für den
//! Kontextwechsel bei Fokus und Eingabe (3.2.1, 3.2.2 — nicht auf Formulare
//! beschränkt), `auth/*` für die Anmeldung (3.3.8).
//!
//! Wo nur geraten werden kann — Zweck eines Felds aus seiner Beschriftung,
//! Formatvorgaben, wiederholte Eingaben, Captchas, Handler —, meldet die Regel
//! `REVIEW`, nicht `FAIL`.

use std::collections::HashMap;

use a11y_dom::{
    Document, NameSource, Node, NodeId, ScopedId, Semantics, ancestors, closest, descendants,
    elements, scoped_id, subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::explicit_role;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn input_type<'a, N: Node<'a>>(n: N) -> String {
    n.attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "text".into())
}

fn is_input<'a, N: Node<'a>>(n: N, types: &[&str]) -> bool {
    n.is_element("input") && types.contains(&input_type(n).as_str())
}

/// Die IDs der Sicht. Was in einem versteckten Teilbaum liegt, fehlt darin.
fn id_index<'a, D: Document>(doc: &'a D) -> HashMap<ScopedId<'a>, D::N<'a>> {
    let mut ids = HashMap::new();
    for n in elements(doc) {
        if let Some(id) = n.attr("id") {
            ids.entry(scoped_id(n, id)).or_insert(n);
        }
    }
    ids
}

/// Der Formulareigentümer nach HTML: das `form`-Attribut, sonst das nächste
/// `<form>` darüber.
fn form<'a, N: Node<'a>>(n: N, ids: &HashMap<ScopedId<'a>, N>) -> Option<N> {
    match n.attr("form") {
        Some(f) => ids
            .get(&scoped_id(n, f.trim()))
            .copied()
            .filter(|f| f.is_element("form")),
        None => closest(n, "form"),
    }
}

/// Der Text, auf den `aria-describedby` zeigt, und ob ein Verweis ins Leere
/// der Sicht ging.
///
/// Ein Ziel außerhalb der Sicht ist entweder versteckt — dann trägt es nach
/// accname trotzdem zur Beschreibung bei — oder fehlt ganz, und das meldet
/// `aria/reference-missing`. Beide Fälle zählen hier als beschrieben.
fn description<'a, N: Node<'a>>(
    n: N,
    attr: &str,
    ids: &HashMap<ScopedId<'a>, N>,
) -> (String, bool) {
    let mut text = String::new();
    let mut open = false;
    for t in n.attr(attr).unwrap_or("").split_whitespace() {
        match ids.get(&scoped_id(n, t)) {
            Some(z) => {
                text.push_str(z.attr("aria-label").unwrap_or(""));
                text.push(' ');
                text.push_str(&subtree_text(*z));
                text.push(' ');
            }
            None => open = true,
        }
    }
    (text, open)
}

fn described<'a, N: Node<'a>>(n: N, attr: &str, ids: &HashMap<ScopedId<'a>, N>) -> bool {
    let (text, open) = description(n, attr, ids);
    open || !text.trim().is_empty()
}

/// Wörter in Kleinschreibung, getrennt an allem, was kein Buchstabe und keine
/// Ziffer ist, und an camelCase-Grenzen (`viewName` → `view`, `name`).
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut small = false;
    for c in text.chars() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            small = false;
            continue;
        }
        if c.is_uppercase() && small {
            out.push(std::mem::take(&mut current));
        }
        small = c.is_lowercase();
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

// --- forms/autocomplete-invalid -------------------------------------------

/// Feldnamen ohne Kontaktangabe (HTML, „Autofill", Tabelle der
/// „autofill field names").
const AUTOFILL_FIELD: &[&str] = &[
    "name",
    "honorific-prefix",
    "given-name",
    "additional-name",
    "family-name",
    "honorific-suffix",
    "nickname",
    "username",
    "new-password",
    "current-password",
    "one-time-code",
    "organization-title",
    "organization",
    "street-address",
    "address-line1",
    "address-line2",
    "address-line3",
    "address-level4",
    "address-level3",
    "address-level2",
    "address-level1",
    "country",
    "country-name",
    "postal-code",
    "cc-name",
    "cc-given-name",
    "cc-additional-name",
    "cc-family-name",
    "cc-number",
    "cc-exp",
    "cc-exp-month",
    "cc-exp-year",
    "cc-csc",
    "cc-type",
    "transaction-currency",
    "transaction-amount",
    "language",
    "bday",
    "bday-day",
    "bday-month",
    "bday-year",
    "sex",
    "url",
    "photo",
];

/// Feldnamen, vor denen eine Kontaktangabe stehen darf.
const AUTOFILL_CONTACT_FIELD: &[&str] = &[
    "tel",
    "tel-country-code",
    "tel-national",
    "tel-area-code",
    "tel-local",
    "tel-local-prefix",
    "tel-local-suffix",
    "tel-extension",
    "email",
    "impp",
];

const AUTOFILL_CONTACT: &[&str] = &["home", "work", "mobile", "fax", "pager"];

/// Die Grammatik der Autofill-Angabe (HTML, „autofill detail tokens"):
/// `[section-*] [shipping|billing] [Kontakt] Feldname [webauthn]`, oder
/// allein `on` bzw. `off`. Groß-/Kleinschreibung zählt nicht.
fn autocomplete_valid(value: &str) -> bool {
    let tokens: Vec<String> = value
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    let mut rest: Vec<&str> = tokens.iter().map(String::as_str).collect();
    if matches!(rest.as_slice(), [] | ["on"] | ["off"]) {
        return true;
    }
    if rest.last() == Some(&"webauthn") {
        rest.pop();
    }
    let Some(field) = rest.pop() else {
        return false;
    };
    let contact_field = AUTOFILL_CONTACT_FIELD.contains(&field);
    if !contact_field && !AUTOFILL_FIELD.contains(&field) {
        return false;
    }
    if contact_field && rest.last().is_some_and(|k| AUTOFILL_CONTACT.contains(k)) {
        rest.pop();
    }
    if rest
        .last()
        .is_some_and(|k| matches!(*k, "shipping" | "billing"))
    {
        rest.pop();
    }
    if rest.last().is_some_and(|k| k.starts_with("section-")) {
        rest.pop();
    }
    rest.is_empty()
}

/// Ein `autocomplete`, das der HTML-Standard nicht kennt, sagt dem Browser
/// und der Assistenztechnik nichts über den Zweck (1.3.5).
///
/// Geprüft wird die ganze Grammatik, nicht nur das letzte Token wie in
/// auditmysite: `autocomplete="foo email"` ist ungültig, `"work email"`
/// gültig, `"work name"` nicht (Kontaktangaben nur vor Kontaktfeldern).
///
/// Anwendbar wie in ACT 73f2c2: nicht an `input` vom Typ `hidden`, `button`,
/// `submit` oder `reset` und nicht an deaktivierten Feldern (`disabled`,
/// `aria-disabled="true"`) — dort nimmt niemand eine Eingabe vor.
pub(crate) fn autocomplete_invalid<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let field = match n.local_name() {
            "select" | "textarea" => true,
            "input" => !matches!(
                input_type(n).as_str(),
                "hidden" | "button" | "submit" | "reset"
            ),
            _ => false,
        } && !disabled(n)
            && !n
                .attr("aria-disabled")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
        let Some(value) = n.attr("autocomplete").filter(|_| field) else {
            continue;
        };
        if autocomplete_valid(value) {
            continue;
        }
        let value = value.trim();
        out.push(
            Finding::fail(
                "forms/autocomplete-invalid",
                tr!(
                    locale,
                    "autocomplete=\"{value}\" is not a valid autofill token.",
                    "autocomplete=\"{value}\" ist keine gültige Autofill-Angabe.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.3.5"])
            .at(at(n.id())),
        );
    }
}

// --- forms/error-unidentified ---------------------------------------------

/// `aria-invalid` mit einem Wert außer `false` (WAI-ARIA 1.2: unbekannte Werte
/// gelten als `true`).
fn invalid<'a, N: Node<'a>>(n: N) -> bool {
    n.attr("aria-invalid")
        .map(str::trim)
        .is_some_and(|v| !v.is_empty() && !v.eq_ignore_ascii_case("false"))
}

/// Ein als ungültig ausgezeichnetes Feld ohne Fehlerbeschreibung: Der
/// Screenreader sagt „ungültig", aber nicht, was falsch ist (3.3.1).
///
/// Als Beschreibung zählen `aria-describedby` und `aria-errormessage` mit
/// Text. `REVIEW`: Eine sichtbare Fehlermeldung neben dem Feld erfüllt 3.3.1
/// auch ohne Verknüpfung (G83, auditmysite#779). Ersetzt zwei auditmysite-Regeln, die denselben Fall doppelt meldeten.
pub(crate) fn error_unidentified<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    for n in elements(doc) {
        if !invalid(n)
            || described(n, "aria-describedby", &ids)
            || described(n, "aria-errormessage", &ids)
        {
            continue;
        }
        out.push(
            Finding::review(
                "forms/error-unidentified",
                pick!(
                    locale,
                    "The field is marked aria-invalid but has no error description (aria-describedby or aria-errormessage).",
                    "Das Feld ist als aria-invalid ausgezeichnet, hat aber keine Fehlerbeschreibung (aria-describedby oder aria-errormessage).",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.3.1"])
            .at(at(n.id())),
        );
    }
}

// --- forms/group-missing --------------------------------------------------

/// Ob ein Vorfahre die Felder gruppiert: `<fieldset>`, `<details>` (HTML-AAM:
/// `group`) oder `role="group"`/`"radiogroup"`.
fn in_group<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| match explicit_role(a) {
        Some(r) => r == "group" || r == "radiogroup",
        None => a.is_element("fieldset") || a.is_element("details"),
    })
}

fn is_radio<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => r == "radio",
        None => is_input(n, &["radio"]),
    }
}

/// Ein Kontrollkästchen, das zu einer Menge gehören kann: mit `name`.
fn checkbox_name<'a, N: Node<'a>>(n: N) -> Option<&'a str> {
    if !is_input(n, &["checkbox"]) || explicit_role(n).is_some_and(|r| r != "checkbox") {
        return None;
    }
    n.attr("name").map(str::trim).filter(|v| !v.is_empty())
}

/// Optionsfelder und zusammengehörige Kontrollkästchen ohne Gruppe (1.3.1).
///
/// Ein Optionsfeld ergibt nur als Teil einer Auswahl Sinn; die Frage, die die
/// Auswahl beantwortet, steht im Namen der Gruppe. Kontrollkästchen gehören
/// zusammen, wenn sie im selben Formular denselben `name` tragen — ein
/// einzelnes („AGB akzeptieren") ist keine Gruppe (auditmysite#643).
/// `REVIEW`: `fieldset` ist eine Technik (H71), keine Pflicht — sagen die
/// Beschriftungen schon alles, ist 1.3.1 erfüllt (auditmysite#779).
pub(crate) fn group_missing<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    let key = |n| checkbox_name(n).map(|name| (form(n, &ids).map(|f| f.id()), name));
    let mut sets: HashMap<(Option<NodeId>, &str), usize> = HashMap::new();
    for n in elements(doc) {
        if let Some(k) = key(n) {
            *sets.entry(k).or_default() += 1;
        }
    }
    for n in elements(doc) {
        let grouped = is_radio(n) || key(n).is_some_and(|k| sets.get(&k).is_some_and(|&z| z > 1));
        if !grouped || in_group(n) {
            continue;
        }
        out.push(
            Finding::review(
                "forms/group-missing",
                pick!(
                    locale,
                    "Related options are not grouped (fieldset with legend, or role=\"group\" with a name).",
                    "Zusammengehörige Optionen sind nicht gruppiert (fieldset mit legend oder role=\"group\" mit Namen).",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1"])
            .at(at(n.id())),
        );
    }
}

// --- forms/no-submit --------------------------------------------------------

fn disabled<'a, N: Node<'a>>(n: N) -> bool {
    n.has_attr("disabled")
}

fn is_control<'a, N: Node<'a>>(n: N) -> bool {
    match n.local_name() {
        "input" => input_type(n) != "hidden" && !disabled(n),
        "select" | "textarea" | "button" => !disabled(n),
        _ => false,
    }
}

fn is_submit<'a, N: Node<'a>>(n: N) -> bool {
    if disabled(n) {
        return false;
    }
    if n.is_element("button") {
        // HTML: Fehlt `type` oder ist der Wert ungültig, ist der Button ein
        // Submit Button (missing und invalid value default).
        return !n.attr("type").is_some_and(|t| {
            let t = t.trim();
            t.eq_ignore_ascii_case("button") || t.eq_ignore_ascii_case("reset")
        });
    }
    if is_input(n, &["submit", "image"]) {
        return true;
    }
    explicit_role(n) == Some("button")
        && (n.has_attr("data-submit")
            || n.attr("aria-label").is_some_and(|l| {
                let l = l.to_lowercase();
                ["submit", "send", "search"].iter().any(|w| l.contains(w))
            }))
}

/// Ein Feld, in dem die Eingabetaste das Formular implizit absendet.
fn is_text_input<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("input")
        && !disabled(n)
        && !matches!(
            input_type(n).as_str(),
            "hidden"
                | "checkbox"
                | "radio"
                | "button"
                | "reset"
                | "file"
                | "range"
                | "color"
                | "submit"
                | "image"
        )
}

#[derive(Default)]
struct Inventory {
    fields: usize,
    submit: bool,
    text_input: bool,
}

/// Ein Formular mit Eingaben, aber ohne Absende-Element (H32, 3.2.2). Immer
/// `REVIEW`: 3.2.2 verletzt erst ein Kontextwechsel bei der Eingabe, nicht
/// der fehlende Knopf (auditmysite#779).
///
/// Ohne `action` und ohne Textfeld kann nur ein Skript das Formular absenden —
/// meist sind das Schalter, die Inhalte auf der Seite ändern. Dann `REVIEW`
/// mit niedriger Schwere statt Verstoß (auditmysite#728). Anders als
/// auditmysite zählen auch Felder und Buttons, die über das `form`-Attribut
/// außerhalb des Formulars stehen (HTML, „form owner").
pub(crate) fn no_submit<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    let mut inventory: HashMap<NodeId, Inventory> = HashMap::new();
    for n in elements(doc) {
        let field = is_control(n);
        let submit = is_submit(n);
        if !field && !submit {
            continue;
        }
        let Some(f) = form(n, &ids) else { continue };
        let b = inventory.entry(f.id()).or_default();
        b.fields += usize::from(field);
        b.submit |= submit;
        b.text_input |= is_text_input(n);
    }
    for f in elements(doc).filter(|n| n.is_element("form")) {
        let Some(b) = inventory.get(&f.id()) else {
            continue;
        };
        if b.fields == 0 || b.submit {
            continue;
        }
        let count = b.fields;
        let finding = if !f.has_attr("action") && !b.text_input {
            Finding::review(
                "forms/no-submit",
                tr!(
                    locale,
                    "The form has {count} control(s), no submit control and nothing submits it without script; check that changing a control does not submit, navigate or open a window.",
                    "Das Formular hat {count} Bedienelement(e), kein Absende-Element, und ohne Skript sendet es nichts ab; prüfen, dass eine Änderung nicht absendet, navigiert oder ein Fenster öffnet.",
                ),
            )
            .with_severity(Severity::Low)
        } else {
            Finding::review(
                "forms/no-submit",
                tr!(
                    locale,
                    "The form has {count} control(s) but no submit control.",
                    "Das Formular hat {count} Bedienelement(e), aber kein Absende-Element.",
                ),
            )
            .with_severity(Severity::Medium)
        };
        out.push(finding.with_wcag(["3.2.2"]).at(at(f.id())));
    }
}

// --- forms/redundant-entry --------------------------------------------------

/// Angaben, die ein Formular nicht zweimal verlangen sollte, mit den Wörtern,
/// an denen sie zu erkennen sind (auditmysite `redundant_entry`).
struct Info {
    en: &'static str,
    #[cfg(feature = "de")]
    de: &'static str,
    words: &'static [&'static str],
}

const INFOS: &[Info] = &[
    Info {
        en: "email address",
        #[cfg(feature = "de")]
        de: "E-Mail-Adresse",
        words: &["email", "e-mail"],
    },
    Info {
        en: "phone number",
        #[cfg(feature = "de")]
        de: "Telefonnummer",
        words: &["telefon", "phone", "mobil", "handynummer"],
    },
    Info {
        en: "name",
        #[cfg(feature = "de")]
        de: "Name",
        words: &[
            "vorname",
            "nachname",
            "first name",
            "last name",
            "firstname",
            "lastname",
            "full name",
            "surname",
            "family name",
        ],
    },
    Info {
        en: "street address",
        #[cfg(feature = "de")]
        de: "Straße",
        words: &["straße", "strasse", "address line", "street address"],
    },
    Info {
        en: "postal code",
        #[cfg(feature = "de")]
        de: "Postleitzahl",
        words: &["postleitzahl", "plz", "zip code", "zipcode", "postal code"],
    },
];

/// Bewusste Wiederholung: Bestätigungsfelder, Einmalcodes, Captchas.
const REPETITION: &[&str] = &[
    "confirm",
    "repeat",
    "again",
    "verify",
    "wiederhol",
    "bestät",
    "bestaet",
    "erneut",
    "captcha",
    "otp",
    "one-time code",
    "one time code",
    "verification code",
];

/// Formulierungen, mit denen ein Formular die frühere Angabe anbietet.
const ADOPTION: &[&str] = &[
    "same as",
    "identical to",
    "use shipping",
    "use the shipping",
    "use billing",
    "use the billing",
    "wie oben",
    "wie bei",
    "entspricht",
    "gleich wie",
];

fn info(text: &str) -> Option<usize> {
    let text = text.to_lowercase();
    if REPETITION.iter().any(|w| text.contains(w)) {
        return None;
    }
    INFOS
        .iter()
        .position(|a| a.words.iter().any(|w| text.contains(w)))
}

/// Ein Formular verlangt dieselbe Angabe mehrmals, ohne sie vorzubelegen,
/// per `autocomplete` füllbar zu machen oder „wie oben" anzubieten (3.3.7).
///
/// `REVIEW`: Ob beide Felder zum selben Vorgang gehören und dieselbe Angabe
/// meinen, entscheidet ein Mensch. Geprüft wird je `<form>`, wie in
/// auditmysite.
pub(crate) fn redundant_entry<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut labels: HashMap<ScopedId, Vec<_>> = HashMap::new();
    for l in elements(doc).filter(|n| n.is_element("label")) {
        if let Some(f) = l.attr("for") {
            labels.entry(scoped_id(l, f)).or_default().push(l);
        }
    }
    for form in elements(doc).filter(|n| n.is_element("form")) {
        let form_text = subtree_text(form).to_lowercase();
        if ADOPTION.iter().any(|u| form_text.contains(u)) {
            continue;
        }
        let mut counter = [0usize; INFOS.len()];
        for n in descendants(form) {
            let field = match n.local_name() {
                "select" | "textarea" => true,
                "input" => !matches!(
                    input_type(n).as_str(),
                    "hidden" | "password" | "submit" | "button" | "reset" | "checkbox" | "radio"
                ),
                _ => false,
            };
            let autocomplete = n.attr("autocomplete").is_some_and(|v| {
                let v = v.trim();
                !v.is_empty() && !v.eq_ignore_ascii_case("off") && !v.eq_ignore_ascii_case("on")
            });
            let prefilled = n.has_attr("readonly")
                || disabled(n)
                || n.attr("value").is_some_and(|v| !v.trim().is_empty());
            if !field || autocomplete || prefilled {
                continue;
            }
            let mut text = String::new();
            for a in ["name", "id", "placeholder", "aria-label"] {
                text.push_str(n.attr(a).unwrap_or(""));
                text.push(' ');
            }
            if let Some(l) = n.attr("id").and_then(|id| labels.get(&scoped_id(n, id))) {
                text.push_str(&subtree_text(l[0]));
                text.push(' ');
            }
            if let Some(l) = closest(n, "label") {
                text.push_str(&subtree_text(l));
            }
            if let Some(i) = info(&text) {
                counter[i] += 1;
            }
        }
        for (a, &count) in INFOS.iter().zip(&counter) {
            if count < 2 {
                continue;
            }
            let was = pick!(locale, a.en, a.de);
            out.push(
                Finding::review(
                    "forms/redundant-entry",
                    tr!(
                        locale,
                        "The form asks for the {was} {count} times without autocomplete, prefill or a \"same as\" option.",
                        "Das Formular fragt {count}-mal nach {was}, ohne autocomplete, Vorbelegung oder „wie oben\".",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.3.7"])
                .at(at(form.id())),
            );
        }
    }
}

// --- context/on-input -------------------------------------------------------

/// Code, mit dem ein Handler den Kontext wechselt: Navigation, Absenden, neues
/// Fenster, Fokus (ohne Leerraum verglichen).
const CONTEXT_CHANGE: &[&str] = &[
    "location=",
    "location.href=",
    "location.assign(",
    "location.replace(",
    "location.reload(",
    "location.search=",
    "location.pathname=",
    ".submit(",
    ".requestSubmit(",
    "window.open(",
    ".focus(",
    "navigate(",
];

/// Ein `onchange` an Auswahlliste oder Optionsfeld, das den Kontext wechselt
/// (3.2.2, F36/F37).
///
/// Nur der statische Teil von auditmysite `on_input`: der Handlertext im
/// Markup. Ruft er eine Funktion auf, deren Quelltext erst die laufende Seite
/// kennt, bleibt es `REVIEW`; das Nachschlagen über `window` bleibt im Host.
/// Anders als auditmysite zählen auch Optionsfelder und Kontrollkästchen mit
/// `onchange` (F37 nennt Optionsfeld, Kontrollkästchen und Auswahlliste).
pub(crate) fn on_input<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(handler) = n.attr("onchange") else {
            continue;
        };
        let selection = match explicit_role(n) {
            Some(r) => matches!(r, "combobox" | "listbox" | "radio" | "checkbox"),
            None => n.is_element("select") || is_input(n, &["radio", "checkbox"]),
        };
        if !selection {
            continue;
        }
        let code: String = handler.chars().filter(|c| !c.is_whitespace()).collect();
        // 3.2.2 lässt den Kontextwechsel zu, wenn vorher darauf hingewiesen
        // wird (G13). Eine Beschreibung am Feld kann dieser Hinweis sein; ob
        // sie es ist, entscheidet ein Prüfer.
        let pointed_out = n
            .attr("aria-describedby")
            .is_some_and(|v| !v.trim().is_empty());
        let change = CONTEXT_CHANGE.iter().any(|k| code.contains(k));
        let finding = if change && pointed_out {
            Finding::review(
                "context/on-input",
                pick!(
                    locale,
                    "The onchange handler changes context; the field has a description. Check that it advises of the change before use (G13).",
                    "Der onchange-Handler wechselt den Kontext; das Feld hat eine Beschreibung. Prüfen, ob sie vor der Bedienung auf den Wechsel hinweist (G13).",
                ),
            )
            .with_severity(Severity::Medium)
        } else if change {
            Finding::fail(
                "context/on-input",
                pick!(
                    locale,
                    "The onchange handler navigates, submits, opens a window or moves focus.",
                    "Der onchange-Handler navigiert, sendet ab, öffnet ein Fenster oder verschiebt den Fokus.",
                ),
            )
            .with_severity(Severity::Medium)
        } else {
            Finding::review(
                "context/on-input",
                pick!(
                    locale,
                    "The control has an onchange handler; check that it only updates content and does not change context.",
                    "Das Bedienelement hat einen onchange-Handler; prüfen, dass er nur Inhalt ändert und den Kontext nicht wechselt.",
                ),
            )
            .with_severity(Severity::Low)
        };
        out.push(finding.with_wcag(["3.2.2"]).at(at(n.id())));
    }
}

// --- context/on-focus, context/autofocus ------------------------------------

const INTERACTIVE_ROLES: &[&str] = &[
    "button",
    "link",
    "checkbox",
    "radio",
    "switch",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "tab",
    "textbox",
    "combobox",
    "searchbox",
    "slider",
    "spinbutton",
];

/// Ein `onfocus` an einem nicht interaktiven Element und `autofocus` an etwas
/// anderem als einem Eingabefeld (3.2.1).
///
/// `REVIEW` statt auditmysites Verstoß: Ob der Handler den Kontext wechselt,
/// steht nicht im Markup, und `autofocus` wechselt ihn für sich genommen
/// nicht. Die Schweregrade bleiben (hoch, mittel).
pub(crate) fn on_focus<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let role = explicit_role(n);
        if n.has_attr("onfocus")
            && !matches!(
                n.local_name(),
                "a" | "button" | "input" | "select" | "textarea" | "summary"
            )
            && !role.is_some_and(|r| INTERACTIVE_ROLES.contains(&r))
        {
            out.push(
                Finding::review(
                    "context/on-focus",
                    pick!(
                        locale,
                        "A non-interactive element has an onfocus handler; check that focus does not change the context.",
                        "Ein nicht interaktives Element hat einen onfocus-Handler; prüfen, dass der Fokus den Kontext nicht wechselt.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["3.2.1"])
                .at(at(n.id())),
            );
        }
        if n.has_attr("autofocus")
            && !matches!(n.local_name(), "input" | "select" | "textarea")
            && !role.is_some_and(|r| matches!(r, "textbox" | "searchbox" | "combobox"))
        {
            out.push(
                Finding::review(
                    "context/autofocus",
                    pick!(
                        locale,
                        "autofocus on an element that is not an input field can disorient users.",
                        "autofocus an einem Element, das kein Eingabefeld ist, kann desorientieren.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.2.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- auth/captcha -------------------------------------------------------------

fn is_login_field<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("input")
        && (input_type(n) == "password"
            || n.attr("autocomplete").is_some_and(|v| {
                v.split_ascii_whitespace().any(|t| {
                    t.eq_ignore_ascii_case("one-time-code")
                        || t.eq_ignore_ascii_case("current-password")
                })
            }))
}

/// Die Widgets der verbreiteten Captcha-Dienste und Captcha-Bilder, ohne die
/// unsichtbaren Varianten, die keinen Test stellen.
fn is_captcha<'a, N: Node<'a>>(n: N) -> bool {
    let invisible = n.attr("data-size") == Some("invisible");
    let class = |k: &str| {
        n.attr("class")
            .is_some_and(|c| c.split_ascii_whitespace().any(|x| x == k))
    };
    if (class("g-recaptcha") || class("h-captcha")) && !invisible {
        return true;
    }
    let src = n.attr("src").unwrap_or("").to_ascii_lowercase();
    if n.is_element("iframe") {
        return ((src.contains("recaptcha/api2/anchor")
            || src.contains("recaptcha/enterprise/anchor"))
            && !src.contains("size=invisible"))
            || (src.contains("hcaptcha.com") && src.contains("checkbox"));
    }
    n.is_element("img")
        && ["src", "alt", "id", "class"].iter().any(|a| {
            n.attr(a)
                .is_some_and(|v| v.to_ascii_lowercase().contains("captcha"))
        })
}

/// Ein Captcha in einem Anmeldeformular (3.3.8). `REVIEW`: Ob es ein
/// Objekterkennungstest ist oder eine Alternative ohne kognitiven Test
/// angeboten wird, steht nicht im Markup. Als Anmeldung zählt ein Formular
/// mit Passwort- oder Einmalcode-Feld; ein Captcha im Kontaktformular gehört
/// nicht zu 3.3.8. Der Einfüge-Test an Passwortfeldern bleibt im Host.
pub(crate) fn captcha<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for form in elements(doc).filter(|n| n.is_element("form")) {
        if !descendants(form).any(is_login_field) || !descendants(form).any(is_captcha) {
            continue;
        }
        out.push(
            Finding::review(
                "auth/captcha",
                pick!(
                    locale,
                    "The sign-in form contains a CAPTCHA; check that it is an object-recognition test or that an alternative without a cognitive function test exists.",
                    "Das Anmeldeformular enthält ein Captcha; prüfen, dass es ein Objekterkennungstest ist oder eine Alternative ohne kognitiven Test besteht.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.3.8"])
            .at(at(form.id())),
        );
    }
}

// --- Tier 2 -------------------------------------------------------------------

/// Die Rolle eines Felds. Ein Passwortfeld hat nach HTML-AAM keine Rolle,
/// Chrome stellt es als `textbox` aus — hier für jeden Host gleich.
fn field_role<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> Option<String> {
    doc.role(n)
        .or_else(|| is_input(n, &["password"]).then(|| "textbox".to_string()))
}

const INPUT_ROLES: &[&str] = &[
    "textbox",
    "searchbox",
    "combobox",
    "listbox",
    "spinbutton",
    "slider",
    "checkbox",
    "radio",
    "switch",
];

// --- forms/purpose-missing ----------------------------------------------------

/// Wörter, an denen eine Beschriftung eine Angabe über die Person erkennen
/// lässt — als ganze Wörter, damit „Hotel" kein „tel" ist. Englisch und
/// Deutsch gemischt, unabhängig von der Ausgabesprache. „name" fehlt: Es
/// benennt Dinge so oft wie Personen (auditmysite#658, [`name_reference`]).
const PERSON_WORDS: &[&str] = &[
    "email",
    "phone",
    "telephone",
    "tel",
    "fax",
    "mobile",
    "address",
    "street",
    "city",
    "town",
    "zip",
    "zipcode",
    "postcode",
    "postal",
    "country",
    "password",
    "username",
    "first",
    "last",
    "birthday",
    "birth",
    "dob",
    "telefon",
    "handy",
    "mobil",
    "handynummer",
    "mobilnummer",
    "faxnummer",
    "plz",
    "postleitzahl",
    "adresse",
    "anschrift",
    "strasse",
    "straße",
    "hausnummer",
    "ort",
    "wohnort",
    "stadt",
    "land",
    "passwort",
    "kennwort",
    "geburtstag",
    "geburtsdatum",
];

/// Anfänge zusammengesetzter Wörter („Telefonnummer", „Geburtsort").
const PEOPLE_START: &[&str] = &[
    "telefon", "phone", "email", "postal", "postleit", "password", "passwort", "birth", "geburts",
];

/// Enden zusammengesetzter Wörter („Lieferadresse", „Hauptstraße").
const PEOPLE_END: &[&str] = &[
    "adresse", "address", "telefon", "strasse", "straße", "passwort", "password", "kennwort",
];

/// Vor „address" machen diese Wörter eine technische Adresse daraus
/// (`ip_address`, auditmysite `identify_purpose`).
const TECHNICAL_ADDRESS: &[&str] = &["ip", "mac", "wallet", "contract"];

fn asks_for_person(text: &str) -> bool {
    let w = words(text);
    w.iter().enumerate().any(|(i, word)| {
        let word = word.as_str();
        let hit = PERSON_WORDS.contains(&word)
            || PEOPLE_START.iter().any(|p| word.starts_with(p))
            || PEOPLE_END.iter().any(|s| word.ends_with(s))
            // „E-Mail" zerfällt in „e" und „mail".
            || (word == "mail" && i > 0 && w[i - 1] == "e");
        let technical = i > 0
            && TECHNICAL_ADDRESS.contains(&w[i - 1].as_str())
            && (word.ends_with("address") || word.ends_with("adresse"));
        hit && !technical
    })
}

/// Wörter, die immer den Namen einer Person meinen.
const PERSON_NAME: &[&str] = &[
    "firstname",
    "lastname",
    "fullname",
    "surname",
    "givenname",
    "familyname",
    "middlename",
    "username",
    "nickname",
    "vorname",
    "nachname",
    "familienname",
    "benutzername",
    "geburtsname",
    "rufname",
];

/// Wörter vor einem bloßen „name", die ihn nicht zum Namen eines Dings machen:
/// die Person selbst („Ihr Name") oder technische Teile einer ID.
const NAME_NEUTRAL: &[&str] = &[
    "your",
    "my",
    "full",
    "first",
    "last",
    "given",
    "family",
    "middle",
    "legal",
    "maiden",
    "user",
    "contact",
    "customer",
    "person",
    "member",
    "guest",
    "applicant",
    "author",
    "cardholder",
    "ihr",
    "ihren",
    "dein",
    "deinen",
    "vollständiger",
    "voller",
    "input",
    "field",
    "form",
    "fld",
    "txt",
    "inp",
    "text",
    "billing",
    "shipping",
    "account",
    "profile",
    "signup",
    "register",
    "registration",
    "checkout",
];

/// Wörter nach „name", die das benannte Ding einleiten („Name der Ansicht").
const NAME_OF: &[&str] = &["of", "for", "der", "des", "für", "von"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum NameReference {
    Person,
    Thing,
}

/// Was ein Text über „Name" sagt; `None` ohne Namenswort. Eine Lesart als
/// Person irgendwo im Text schlägt die als Ding (auditmysite#658).
fn name_reference(text: &str) -> Option<NameReference> {
    let w = words(text);
    let mut result = None;
    for (i, word) in w.iter().enumerate() {
        if PERSON_NAME.contains(&word.as_str()) {
            return Some(NameReference::Person);
        }
        if word != "name" {
            continue;
        }
        let before = w[..i]
            .iter()
            .rev()
            .find(|x| !x.chars().all(|c| c.is_ascii_digit()));
        let thing = before.is_some_and(|d| !NAME_NEUTRAL.contains(&d.as_str()))
            || w.get(i + 1).is_some_and(|n| NAME_OF.contains(&n.as_str()));
        if !thing {
            return Some(NameReference::Person);
        }
        result = Some(NameReference::Thing);
    }
    result
}

/// Ob das Feld nach dem Namen der Person fragt. Die Beschriftung entscheidet;
/// ein Ding in `id` oder `name` (`#view-name` mit Beschriftung „Name") nimmt
/// es zurück, außer das andere Attribut nennt die Person. Ohne Namenswort in
/// der Beschriftung zählt ein Personenname in `id` oder `name`.
fn asks_for_names(label: &str, attribute: [Option<NameReference>; 2]) -> bool {
    match name_reference(label) {
        Some(NameReference::Person) => {
            attribute.contains(&Some(NameReference::Person))
                || !attribute.contains(&Some(NameReference::Thing))
        }
        Some(NameReference::Thing) => false,
        None => attribute.contains(&Some(NameReference::Person)),
    }
}

/// Ein Feld, das nach einer Angabe über die Person fragt, ohne
/// `autocomplete` (1.3.5).
///
/// Erkannt an der Beschriftung (dem Accessible Name), an `id` und `name` und
/// an den Typen `email` und `tel`. `REVIEW` statt auditmysites Verstoß: Ob
/// das Feld wirklich die Person betrifft, ist geraten. Ersetzt zugleich
/// auditmysite `identify_purpose` (1.3.6), das denselben Fall an `id` und
/// `name` erkannte.
pub(crate) fn purpose_missing<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !is_input(n, &["text", "email", "tel", "url", "password"])
            || doc.is_ignored(n)
            || std::iter::once(n).chain(ancestors(n)).any(|a| {
                a.attr("role").is_some_and(|role| {
                    role.split_ascii_whitespace().any(|r| {
                        r.eq_ignore_ascii_case("search") || r.eq_ignore_ascii_case("combobox")
                    })
                })
            })
        {
            continue;
        }
        let without = n.attr("autocomplete").is_none_or(|v| {
            let v = v.trim();
            v.is_empty() || v.eq_ignore_ascii_case("on") || v.eq_ignore_ascii_case("off")
        });
        if !without {
            continue;
        }
        let label = doc.accessible_name(n).unwrap_or_default();
        let attribute = [n.attr("id"), n.attr("name")].map(|a| a.unwrap_or(""));
        let person = matches!(input_type(n).as_str(), "email" | "tel")
            || asks_for_person(&label)
            || attribute.iter().any(|a| asks_for_person(a))
            || asks_for_names(&label, attribute.map(name_reference));
        if !person {
            continue;
        }
        let field = label.trim();
        out.push(
            Finding::review(
                "forms/purpose-missing",
                tr!(
                    locale,
                    "The field \"{field}\" seems to ask for information about the user but has no autocomplete token.",
                    "Das Feld \"{field}\" fragt offenbar nach Angaben zur Person, hat aber keine autocomplete-Angabe.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.5"])
            .at(at(n.id())),
        );
    }
}

// --- forms/required-unmarked, forms/instructions-missing ----------------------

/// Wörter für „Pflichtfeld" in den Sprachen aus auditmysite.
const REQUIRED: &[&str] = &[
    "*",
    "required",
    "pflicht",
    "erforderlich",
    "obligatoire",
    "requis",
    "obligatorio",
    "requerido",
    "obbligatorio",
    "richiesto",
    "obrigatório",
    "verplicht",
    "vereist",
    "obligatoriskt",
    "krävs",
    "wymagane",
    "zorunlu",
];

/// Beschriftungen, hinter denen ein Format steckt, das man kennen muss.
const FORMAT_NEED: &[&str] = &[
    "date",
    "phone",
    "tel",
    "zip",
    "postal",
    "credit card",
    "ssn",
    "social security",
    "passport",
    "account",
    "routing",
    "datum",
    "telefon",
    "postleitzahl",
    "plz",
    "kreditkarte",
    "reisepass",
    "kontonummer",
    "téléphone",
    "code postal",
    "carte de crédit",
    "passeport",
    "numéro de compte",
    "teléfono",
    "código postal",
    "tarjeta de crédito",
    "pasaporte",
    "número de cuenta",
    "telefono",
    "codice postale",
    "carta di credito",
    "passaporto",
    "numero di conto",
    "telefone",
    "cartão de crédito",
    "passaporte",
    "telefoon",
    "postcode",
    "creditcard",
    "paspoort",
    "rekeningnummer",
    "postnummer",
    "kreditkort",
    "pass",
    "kod pocztowy",
    "karta kredytowa",
    "paszport",
    "posta kodu",
    "kredi kartı",
    "pasaport",
];

/// Formatangaben in der Beschriftung selbst.
const FORMAT_HINT: &[&str] = &[
    "format:",
    "example:",
    "e.g.",
    "(",
    "mm/dd",
    "yyyy",
    "z.b.",
    "bsp.",
    "beispiel:",
    "ex.",
    "exemple:",
    "ej.",
    "ejemplo:",
    "es.",
    "esempio:",
    "exemplo:",
    "bijv.",
    "voorbeeld:",
    "t.ex.",
    "np.",
    "przykład:",
];

/// Kurze Begriffe („pass", „tel", „date") zählen nur als ganzes Wort — „Was
/// ist passiert?" ist kein Reisepass (auditmysite#643); längere auch in
/// Zusammensetzungen („Geburtsdatum").
fn format_term(name: &str, term: &str) -> bool {
    if term.chars().count() >= 5 {
        return name.contains(term);
    }
    name.match_indices(term).any(|(start, _)| {
        let before = name[..start].chars().next_back();
        let after = name[start + term.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn is_required<'a, N: Node<'a>>(n: N) -> bool {
    (n.has_attr("required") && matches!(n.local_name(), "input" | "select" | "textarea"))
        || n.attr("aria-required")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Pflichtfelder ohne erkennbaren Hinweis und Felder mit Formatvorgabe ohne
/// Anleitung (3.3.2).
///
/// Beides `REVIEW`. Ein Pflichtfeld sagt der Screenreader als solches an; ob
/// es auch sichtbar gekennzeichnet ist — vielleicht mit einem Satz über dem
/// Formular —, sieht nur ein Mensch. auditmysite meldete den Fall zweimal
/// (`form_rules`, niedrig; `instructions`, mittel) als Verstoß; hier einmal,
/// mittel. Die Formatvorgabe ist aus der Beschriftung geraten; jede
/// Beschreibung per `aria-describedby` zählt als Anleitung, gleich wie sie
/// formuliert ist (auditmysite#643). Native Datums- und Zeitfelder bringen
/// ihr Format selbst mit (auditmysite#656).
pub(crate) fn instructions<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    for n in elements(doc) {
        if doc.is_ignored(n) {
            continue;
        }
        if !field_role(doc, n).is_some_and(|r| INPUT_ROLES.contains(&r.as_str())) {
            continue;
        }
        let Some(name) = doc.accessible_name(n).filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let name = name.to_lowercase();
        let (description, open) = description(n, "aria-describedby", &ids);
        let description = description.to_lowercase();

        if is_required(n)
            && !REQUIRED
                .iter()
                .any(|p| name.contains(p) || description.contains(p))
        {
            out.push(
                Finding::review(
                    "forms/required-unmarked",
                    pick!(
                        locale,
                        "The required field does not say in its label or description that it is required.",
                        "Das Pflichtfeld sagt weder in der Beschriftung noch in der Beschreibung, dass es Pflicht ist.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.3.2"])
                .at(at(n.id())),
            );
        }

        let native_date = is_input(n, &["date", "time", "datetime-local", "month", "week"]);
        let guided = open
            || !description.trim().is_empty()
            || FORMAT_HINT.iter().any(|h| name.contains(h))
            || n.attr("placeholder").is_some_and(|p| !p.trim().is_empty());
        if !is_input(n, &["radio", "checkbox"])
            && !native_date
            && !guided
            && FORMAT_NEED.iter().any(|b| format_term(&name, b))
        {
            out.push(
                Finding::review(
                    "forms/instructions-missing",
                    pick!(
                        locale,
                        "The field probably expects a particular format but gives no instructions.",
                        "Das Feld erwartet vermutlich ein bestimmtes Format, gibt aber keine Anleitung.",
                    ),
                )
                .with_severity(Severity::Low)
                .with_wcag(["3.3.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- forms/title-only-label ---------------------------------------------------

const TITLE_ROLES: &[&str] = &[
    "textbox",
    "combobox",
    "listbox",
    "slider",
    "spinbutton",
    "searchbox",
];

/// Ein Eingabefeld, dessen Name allein aus `title` stammt: sichtbar nur beim
/// Überfahren mit der Maus, nicht für Tastatur und Touch.
///
/// `REVIEW` statt auditmysites Verstoß: `title` ist eine zulässige Technik
/// (H65), wenn der Zweck aus dem sichtbaren Umfeld hervorgeht. Ohne
/// `name_source` vom Host gilt der Name als aus `title`, wenn er ihm gleicht
/// und keine andere Beschriftung da ist. Mit `placeholder` meldet
/// `forms/placeholder-as-label` denselben Mangel.
pub(crate) fn title_only<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let label_for: Vec<ScopedId> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for").map(|f| scoped_id(n, f)))
        .collect();
    for n in elements(doc) {
        if doc.is_ignored(n) || n.has_attr("placeholder") {
            continue;
        }
        if !field_role(doc, n).is_some_and(|r| TITLE_ROLES.contains(&r.as_str())) {
            continue;
        }
        let Some(name) = doc.accessible_name(n).filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let has_only_title = match doc.name_source(n) {
            Some(q) => q == NameSource::Title,
            None => {
                n.attr("title").is_some_and(|t| t.trim() == name.trim())
                    && !n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
                    && !n
                        .attr("aria-labelledby")
                        .is_some_and(|v| !v.trim().is_empty())
                    && !n
                        .attr("id")
                        .is_some_and(|id| label_for.contains(&scoped_id(n, id)))
                    && closest(n, "label").is_none()
            }
        };
        if has_only_title {
            out.push(
                Finding::review(
                    "forms/title-only-label",
                    pick!(
                        locale,
                        "The field is labelled only by its title attribute.",
                        "Das Feld ist nur über sein title-Attribut beschriftet.",
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- forms/group-name-missing -------------------------------------------------

/// Elemente, die Chrome als `group` ausstellt, die aber keine
/// Formulargruppen sind.
const NO_FORM_GROUP: &[&str] = &[
    "details", "address", "hgroup", "figure", "article", "section", "aside", "optgroup",
];

/// Eine Formulargruppe ohne Namen: `<fieldset>` ohne `<legend>` oder ein
/// `role="group"` mit Eingabefeldern darin (1.3.1, 3.3.2). Die Gruppe sagt
/// dann nicht, welche Frage ihre Felder beantworten — sofern sie eine stellt:
/// Ein `fieldset` kann auch nur gestalten, deshalb `REVIEW`
/// (auditmysite#779). `radiogroup` meldet
/// `names/required-missing`.
pub(crate) fn group_name_missing<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || doc.role(n).as_deref() != Some("group") {
            continue;
        }
        let form_group = n.is_element("fieldset")
            || (!NO_FORM_GROUP.contains(&n.local_name())
                && descendants(n).any(|d| {
                    field_role(doc, d).is_some_and(|r| INPUT_ROLES.contains(&r.as_str()))
                }));
        if !form_group || crate::semantics::named(doc, n) {
            continue;
        }
        out.push(
            Finding::review(
                "forms/group-name-missing",
                pick!(
                    locale,
                    "The form group has no name (legend or aria-labelledby).",
                    "Die Formulargruppe hat keinen Namen (legend oder aria-labelledby).",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1", "3.3.2"])
            .at(at(n.id())),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autocomplete_grammar_follows_html() {
        for good in [
            "",
            "on",
            "OFF",
            "email",
            "work email",
            "shipping street-address",
            "section-a billing mobile tel",
            "section-x shipping home email webauthn",
            "current-password webauthn",
            "transaction-amount",
            "photo",
        ] {
            assert!(autocomplete_valid(good), "{good}");
        }
        for bad in [
            "not-a-real-token",
            "foo email",
            "work name",
            "email work",
            "billing section-a email",
            "on email",
            "webauthn",
            "shipping",
        ] {
            assert!(!autocomplete_valid(bad), "{bad}");
        }
    }

    /// auditmysite#658: Namen von Dingen sind keine Personendaten.
    #[test]
    fn name_of_a_thing_is_not_a_person_name() {
        for thing in [
            "View name",
            "File name",
            "Filename",
            "Project name",
            "Company name",
            "Name of the view",
            "Dateiname",
            "Projektname",
            "Name der Ansicht",
            "Rename",
        ] {
            assert!(!asks_for_names(thing, [None, None]), "{thing}");
            assert!(!asks_for_person(thing), "{thing}");
        }
        for person in [
            "Name",
            "Your name",
            "Full name",
            "First name",
            "Last Name *",
            "Surname",
            "Vorname",
            "Nachname",
            "Ihr Name",
            "Benutzername",
        ] {
            assert!(
                asks_for_names(person, [None, None]) || asks_for_person(person),
                "{person}"
            );
        }
        let attr = |a: &str| [name_reference(a), None];
        assert!(!asks_for_names("Name", attr("view-name")));
        assert!(!asks_for_names("Name", attr("projectName")));
        assert!(asks_for_names("Name", attr("input-name")));
        assert!(asks_for_names("Name", attr("billing_first_name")));
        assert!(asks_for_names("Name", attr("field-2-name")));
        assert!(asks_for_names(
            "Name",
            [name_reference("view-name"), name_reference("fullName")]
        ));
        // Die Beschriftung nennt ein Ding: `name="name"` macht keine Person
        // daraus.
        assert!(!asks_for_names(
            "Project name",
            [None, name_reference("name")]
        ));
    }

    #[test]
    fn person_words_only_as_whole_words() {
        for yes in [
            "Email",
            "E-Mail",
            "Tel.-Nr.",
            "ZIP code",
            "PLZ",
            "Ort",
            "Straße",
            "Telefonnummer",
            "EmailAddress",
            "phoneNumber",
            "Lieferadresse",
            "Geburtsdatum",
            "Neues Kennwort",
            "billing_address",
        ] {
            assert!(asks_for_person(yes), "{yes}");
        }
        for no in [
            "Hotel",
            "Title",
            "Hostel",
            "Cityscape",
            "Lastly",
            "Countryside",
            "Portal",
            "Sorting",
            "Landing page",
            "ip_address",
            "mac-address",
            "walletAddress",
        ] {
            assert!(!asks_for_person(no), "{no}");
        }
    }

    /// auditmysite#643: kurze Begriffe nur als ganzes Wort.
    #[test]
    fn short_format_terms_only_as_a_word() {
        let needs = |name: &str| {
            let name = name.to_lowercase();
            FORMAT_NEED.iter().any(|b| format_term(&name, b))
        };
        assert!(!needs("Was ist passiert?"));
        assert!(!needs("Hotel"));
        assert!(!needs("Last update"));
        assert!(!needs("Quantity"));
        assert!(needs("Date of birth"));
        assert!(needs("Tel."));
        assert!(needs("Reisepass"));
        assert!(needs("Geburtsdatum"));
        assert!(needs("Postal code"));
    }
}
