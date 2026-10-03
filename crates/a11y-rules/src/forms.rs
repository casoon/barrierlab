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
    Document, NameSource, Node, NodeId, Semantics, ancestors, closest, descendants, elements,
    subtree_text,
};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn input_typ<'a, N: Node<'a>>(n: N) -> String {
    n.attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "text".into())
}

fn ist_input<'a, N: Node<'a>>(n: N, typen: &[&str]) -> bool {
    n.is_element("input") && typen.contains(&input_typ(n).as_str())
}

/// Die IDs der Sicht. Was in einem versteckten Teilbaum liegt, fehlt darin.
fn id_index<'a, D: Document>(doc: &'a D) -> HashMap<&'a str, D::N<'a>> {
    let mut ids = HashMap::new();
    for n in elements(doc) {
        if let Some(id) = n.attr("id") {
            ids.entry(id).or_insert(n);
        }
    }
    ids
}

/// Der Formulareigentümer nach HTML: das `form`-Attribut, sonst das nächste
/// `<form>` darüber.
fn formular<'a, N: Node<'a>>(n: N, ids: &HashMap<&str, N>) -> Option<N> {
    match n.attr("form") {
        Some(f) => ids.get(f.trim()).copied().filter(|f| f.is_element("form")),
        None => closest(n, "form"),
    }
}

/// Der Text, auf den `aria-describedby` zeigt, und ob ein Verweis ins Leere
/// der Sicht ging.
///
/// Ein Ziel außerhalb der Sicht ist entweder versteckt — dann trägt es nach
/// accname trotzdem zur Beschreibung bei — oder fehlt ganz, und das meldet
/// `aria/reference-missing`. Beide Fälle zählen hier als beschrieben.
fn beschreibung<'a, N: Node<'a>>(n: N, attr: &str, ids: &HashMap<&str, N>) -> (String, bool) {
    let mut text = String::new();
    let mut offen = false;
    for t in n.attr(attr).unwrap_or("").split_whitespace() {
        match ids.get(t) {
            Some(z) => {
                text.push_str(z.attr("aria-label").unwrap_or(""));
                text.push(' ');
                text.push_str(&subtree_text(*z));
                text.push(' ');
            }
            None => offen = true,
        }
    }
    (text, offen)
}

fn beschrieben<'a, N: Node<'a>>(n: N, attr: &str, ids: &HashMap<&str, N>) -> bool {
    let (text, offen) = beschreibung(n, attr, ids);
    offen || !text.trim().is_empty()
}

/// Wörter in Kleinschreibung, getrennt an allem, was kein Buchstabe und keine
/// Ziffer ist, und an camelCase-Grenzen (`viewName` → `view`, `name`).
fn woerter(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut aktuell = String::new();
    let mut klein = false;
    for c in text.chars() {
        if !c.is_alphanumeric() {
            if !aktuell.is_empty() {
                out.push(std::mem::take(&mut aktuell));
            }
            klein = false;
            continue;
        }
        if c.is_uppercase() && klein {
            out.push(std::mem::take(&mut aktuell));
        }
        klein = c.is_lowercase();
        aktuell.extend(c.to_lowercase());
    }
    if !aktuell.is_empty() {
        out.push(aktuell);
    }
    out
}

// --- forms/autocomplete-invalid -------------------------------------------

/// Feldnamen ohne Kontaktangabe (HTML, „Autofill", Tabelle der
/// „autofill field names").
const AUTOFILL_FELD: &[&str] = &[
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
const AUTOFILL_KONTAKTFELD: &[&str] = &[
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

const AUTOFILL_KONTAKT: &[&str] = &["home", "work", "mobile", "fax", "pager"];

/// Die Grammatik der Autofill-Angabe (HTML, „autofill detail tokens"):
/// `[section-*] [shipping|billing] [Kontakt] Feldname [webauthn]`, oder
/// allein `on` bzw. `off`. Groß-/Kleinschreibung zählt nicht.
fn autocomplete_gueltig(wert: &str) -> bool {
    let tokens: Vec<String> = wert
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
    let Some(feld) = rest.pop() else {
        return false;
    };
    let kontaktfeld = AUTOFILL_KONTAKTFELD.contains(&feld);
    if !kontaktfeld && !AUTOFILL_FELD.contains(&feld) {
        return false;
    }
    if kontaktfeld && rest.last().is_some_and(|k| AUTOFILL_KONTAKT.contains(k)) {
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
pub(crate) fn autocomplete_invalid<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let feld = match n.local_name() {
            "select" | "textarea" => true,
            "input" => input_typ(n) != "hidden",
            _ => false,
        };
        let Some(wert) = n.attr("autocomplete").filter(|_| feld) else {
            continue;
        };
        if autocomplete_gueltig(wert) {
            continue;
        }
        let wert = wert.trim();
        out.push(
            Finding::fail(
                "forms/autocomplete-invalid",
                tr!(
                    locale,
                    "autocomplete=\"{wert}\" is not a valid autofill token.",
                    "autocomplete=\"{wert}\" ist keine gültige Autofill-Angabe.",
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
fn ungueltig<'a, N: Node<'a>>(n: N) -> bool {
    n.attr("aria-invalid")
        .map(str::trim)
        .is_some_and(|v| !v.is_empty() && !v.eq_ignore_ascii_case("false"))
}

/// Ein als ungültig ausgezeichnetes Feld ohne Fehlerbeschreibung: Der
/// Screenreader sagt „ungültig", aber nicht, was falsch ist (3.3.1).
///
/// Als Beschreibung zählen `aria-describedby` und `aria-errormessage` mit
/// Text. Ersetzt zwei auditmysite-Regeln, die denselben Fall doppelt meldeten.
pub(crate) fn error_unidentified<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    for n in elements(doc) {
        if !ungueltig(n)
            || beschrieben(n, "aria-describedby", &ids)
            || beschrieben(n, "aria-errormessage", &ids)
        {
            continue;
        }
        out.push(
            Finding::fail(
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
fn in_gruppe<'a, N: Node<'a>>(n: N) -> bool {
    ancestors(n).any(|a| match explizite_rolle(a) {
        Some(r) => r == "group" || r == "radiogroup",
        None => a.is_element("fieldset") || a.is_element("details"),
    })
}

fn ist_radio<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => r == "radio",
        None => ist_input(n, &["radio"]),
    }
}

/// Ein Kontrollkästchen, das zu einer Menge gehören kann: mit `name`.
fn checkbox_name<'a, N: Node<'a>>(n: N) -> Option<&'a str> {
    if !ist_input(n, &["checkbox"]) || explizite_rolle(n).is_some_and(|r| r != "checkbox") {
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
pub(crate) fn group_missing<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    let schluessel = |n| checkbox_name(n).map(|name| (formular(n, &ids).map(|f| f.id()), name));
    let mut mengen: HashMap<(Option<NodeId>, &str), usize> = HashMap::new();
    for n in elements(doc) {
        if let Some(k) = schluessel(n) {
            *mengen.entry(k).or_default() += 1;
        }
    }
    for n in elements(doc) {
        let gruppiert =
            ist_radio(n) || schluessel(n).is_some_and(|k| mengen.get(&k).is_some_and(|&z| z > 1));
        if !gruppiert || in_gruppe(n) {
            continue;
        }
        out.push(
            Finding::fail(
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

fn deaktiviert<'a, N: Node<'a>>(n: N) -> bool {
    n.has_attr("disabled")
}

fn ist_bedienelement<'a, N: Node<'a>>(n: N) -> bool {
    match n.local_name() {
        "input" => input_typ(n) != "hidden" && !deaktiviert(n),
        "select" | "textarea" | "button" => !deaktiviert(n),
        _ => false,
    }
}

fn ist_absenden<'a, N: Node<'a>>(n: N) -> bool {
    if deaktiviert(n) {
        return false;
    }
    if n.is_element("button") {
        return n
            .attr("type")
            .is_none_or(|t| t.trim().eq_ignore_ascii_case("submit"));
    }
    if ist_input(n, &["submit", "image"]) {
        return true;
    }
    explizite_rolle(n) == Some("button")
        && (n.has_attr("data-submit")
            || n.attr("aria-label").is_some_and(|l| {
                let l = l.to_lowercase();
                ["submit", "send", "search"].iter().any(|w| l.contains(w))
            }))
}

/// Ein Feld, in dem die Eingabetaste das Formular implizit absendet.
fn ist_texteingabe<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("input")
        && !deaktiviert(n)
        && !matches!(
            input_typ(n).as_str(),
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
struct Bestand {
    felder: usize,
    absenden: bool,
    texteingabe: bool,
}

/// Ein Formular mit Eingaben, aber ohne Absende-Element (H32, 3.2.2).
///
/// Ohne `action` und ohne Textfeld kann nur ein Skript das Formular absenden —
/// meist sind das Schalter, die Inhalte auf der Seite ändern. Dann `REVIEW`
/// mit niedriger Schwere statt Verstoß (auditmysite#728). Anders als
/// auditmysite zählen auch Felder und Buttons, die über das `form`-Attribut
/// außerhalb des Formulars stehen (HTML, „form owner").
pub(crate) fn no_submit<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let ids = id_index(doc);
    let mut bestand: HashMap<NodeId, Bestand> = HashMap::new();
    for n in elements(doc) {
        let feld = ist_bedienelement(n);
        let absenden = ist_absenden(n);
        if !feld && !absenden {
            continue;
        }
        let Some(f) = formular(n, &ids) else { continue };
        let b = bestand.entry(f.id()).or_default();
        b.felder += usize::from(feld);
        b.absenden |= absenden;
        b.texteingabe |= ist_texteingabe(n);
    }
    for f in elements(doc).filter(|n| n.is_element("form")) {
        let Some(b) = bestand.get(&f.id()) else {
            continue;
        };
        if b.felder == 0 || b.absenden {
            continue;
        }
        let anzahl = b.felder;
        let finding = if !f.has_attr("action") && !b.texteingabe {
            Finding::review(
                "forms/no-submit",
                tr!(
                    locale,
                    "The form has {anzahl} control(s), no submit control and nothing submits it without script; check that changing a control does not submit, navigate or open a window.",
                    "Das Formular hat {anzahl} Bedienelement(e), kein Absende-Element, und ohne Skript sendet es nichts ab; prüfen, dass eine Änderung nicht absendet, navigiert oder ein Fenster öffnet.",
                ),
            )
            .with_severity(Severity::Low)
        } else {
            Finding::fail(
                "forms/no-submit",
                tr!(
                    locale,
                    "The form has {anzahl} control(s) but no submit control.",
                    "Das Formular hat {anzahl} Bedienelement(e), aber kein Absende-Element.",
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
struct Angabe {
    en: &'static str,
    #[cfg(feature = "de")]
    de: &'static str,
    woerter: &'static [&'static str],
}

const ANGABEN: &[Angabe] = &[
    Angabe {
        en: "email address",
        #[cfg(feature = "de")]
        de: "E-Mail-Adresse",
        woerter: &["email", "e-mail"],
    },
    Angabe {
        en: "phone number",
        #[cfg(feature = "de")]
        de: "Telefonnummer",
        woerter: &["telefon", "phone", "mobil", "handynummer"],
    },
    Angabe {
        en: "name",
        #[cfg(feature = "de")]
        de: "Name",
        woerter: &[
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
    Angabe {
        en: "street address",
        #[cfg(feature = "de")]
        de: "Straße",
        woerter: &["straße", "strasse", "address line", "street address"],
    },
    Angabe {
        en: "postal code",
        #[cfg(feature = "de")]
        de: "Postleitzahl",
        woerter: &["postleitzahl", "plz", "zip code", "zipcode", "postal code"],
    },
];

/// Bewusste Wiederholung: Bestätigungsfelder, Einmalcodes, Captchas.
const WIEDERHOLUNG: &[&str] = &[
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
const UEBERNAHME: &[&str] = &[
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

fn angabe(text: &str) -> Option<usize> {
    let text = text.to_lowercase();
    if WIEDERHOLUNG.iter().any(|w| text.contains(w)) {
        return None;
    }
    ANGABEN
        .iter()
        .position(|a| a.woerter.iter().any(|w| text.contains(w)))
}

/// Ein Formular verlangt dieselbe Angabe mehrmals, ohne sie vorzubelegen,
/// per `autocomplete` füllbar zu machen oder „wie oben" anzubieten (3.3.7).
///
/// `REVIEW`: Ob beide Felder zum selben Vorgang gehören und dieselbe Angabe
/// meinen, entscheidet ein Mensch. Geprüft wird je `<form>`, wie in
/// auditmysite.
pub(crate) fn redundant_entry<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut labels: HashMap<&str, Vec<_>> = HashMap::new();
    for l in elements(doc).filter(|n| n.is_element("label")) {
        if let Some(f) = l.attr("for") {
            labels.entry(f).or_default().push(l);
        }
    }
    for form in elements(doc).filter(|n| n.is_element("form")) {
        let formtext = subtree_text(form).to_lowercase();
        if UEBERNAHME.iter().any(|u| formtext.contains(u)) {
            continue;
        }
        let mut zaehler = [0usize; ANGABEN.len()];
        for n in descendants(form) {
            let feld = match n.local_name() {
                "select" | "textarea" => true,
                "input" => !matches!(
                    input_typ(n).as_str(),
                    "hidden" | "password" | "submit" | "button" | "reset" | "checkbox" | "radio"
                ),
                _ => false,
            };
            let autocomplete = n.attr("autocomplete").is_some_and(|v| {
                let v = v.trim();
                !v.is_empty() && !v.eq_ignore_ascii_case("off") && !v.eq_ignore_ascii_case("on")
            });
            let vorbelegt = n.has_attr("readonly")
                || deaktiviert(n)
                || n.attr("value").is_some_and(|v| !v.trim().is_empty());
            if !feld || autocomplete || vorbelegt {
                continue;
            }
            let mut text = String::new();
            for a in ["name", "id", "placeholder", "aria-label"] {
                text.push_str(n.attr(a).unwrap_or(""));
                text.push(' ');
            }
            if let Some(l) = n.attr("id").and_then(|id| labels.get(id)) {
                text.push_str(&subtree_text(l[0]));
                text.push(' ');
            }
            if let Some(l) = closest(n, "label") {
                text.push_str(&subtree_text(l));
            }
            if let Some(i) = angabe(&text) {
                zaehler[i] += 1;
            }
        }
        for (a, &anzahl) in ANGABEN.iter().zip(&zaehler) {
            if anzahl < 2 {
                continue;
            }
            let was = pick!(locale, a.en, a.de);
            out.push(
                Finding::review(
                    "forms/redundant-entry",
                    tr!(
                        locale,
                        "The form asks for the {was} {anzahl} times without autocomplete, prefill or a \"same as\" option.",
                        "Das Formular fragt {anzahl}-mal nach {was}, ohne autocomplete, Vorbelegung oder „wie oben\".",
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
const KONTEXTWECHSEL: &[&str] = &[
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
/// Anders als auditmysite zählen auch Optionsfelder mit `onchange` (F37).
pub(crate) fn on_input<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(handler) = n.attr("onchange") else {
            continue;
        };
        let auswahl = match explizite_rolle(n) {
            Some(r) => matches!(r, "combobox" | "listbox" | "radio"),
            None => n.is_element("select") || ist_input(n, &["radio"]),
        };
        if !auswahl {
            continue;
        }
        let code: String = handler.chars().filter(|c| !c.is_whitespace()).collect();
        let finding = if KONTEXTWECHSEL.iter().any(|k| code.contains(k)) {
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

const INTERAKTIVE_ROLLEN: &[&str] = &[
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
        let rolle = explizite_rolle(n);
        if n.has_attr("onfocus")
            && !matches!(
                n.local_name(),
                "a" | "button" | "input" | "select" | "textarea" | "summary"
            )
            && !rolle.is_some_and(|r| INTERAKTIVE_ROLLEN.contains(&r))
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
            && !rolle.is_some_and(|r| matches!(r, "textbox" | "searchbox" | "combobox"))
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

fn ist_anmeldefeld<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("input")
        && (input_typ(n) == "password"
            || n.attr("autocomplete").is_some_and(|v| {
                v.split_ascii_whitespace().any(|t| {
                    t.eq_ignore_ascii_case("one-time-code")
                        || t.eq_ignore_ascii_case("current-password")
                })
            }))
}

/// Die Widgets der verbreiteten Captcha-Dienste und Captcha-Bilder, ohne die
/// unsichtbaren Varianten, die keinen Test stellen.
fn ist_captcha<'a, N: Node<'a>>(n: N) -> bool {
    let unsichtbar = n.attr("data-size") == Some("invisible");
    let klasse = |k: &str| {
        n.attr("class")
            .is_some_and(|c| c.split_ascii_whitespace().any(|x| x == k))
    };
    if (klasse("g-recaptcha") || klasse("h-captcha")) && !unsichtbar {
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
        if !descendants(form).any(ist_anmeldefeld) || !descendants(form).any(ist_captcha) {
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
fn feldrolle<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> Option<String> {
    doc.role(n)
        .or_else(|| ist_input(n, &["password"]).then(|| "textbox".to_string()))
}

const EINGABE_ROLLEN: &[&str] = &[
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
/// benennt Dinge so oft wie Personen (auditmysite#658, [`namensbezug`]).
const PERSONENWOERTER: &[&str] = &[
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
const PERSONEN_ANFANG: &[&str] = &[
    "telefon", "phone", "email", "postal", "postleit", "password", "passwort", "birth", "geburts",
];

/// Enden zusammengesetzter Wörter („Lieferadresse", „Hauptstraße").
const PERSONEN_ENDE: &[&str] = &[
    "adresse", "address", "telefon", "strasse", "straße", "passwort", "password", "kennwort",
];

/// Vor „address" machen diese Wörter eine technische Adresse daraus
/// (`ip_address`, auditmysite `identify_purpose`).
const TECHNISCHE_ADRESSE: &[&str] = &["ip", "mac", "wallet", "contract"];

fn fragt_nach_person(text: &str) -> bool {
    let w = woerter(text);
    w.iter().enumerate().any(|(i, wort)| {
        let wort = wort.as_str();
        let treffer = PERSONENWOERTER.contains(&wort)
            || PERSONEN_ANFANG.iter().any(|p| wort.starts_with(p))
            || PERSONEN_ENDE.iter().any(|s| wort.ends_with(s))
            // „E-Mail" zerfällt in „e" und „mail".
            || (wort == "mail" && i > 0 && w[i - 1] == "e");
        let technisch = i > 0
            && TECHNISCHE_ADRESSE.contains(&w[i - 1].as_str())
            && (wort.ends_with("address") || wort.ends_with("adresse"));
        treffer && !technisch
    })
}

/// Wörter, die immer den Namen einer Person meinen.
const PERSONENNAME: &[&str] = &[
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
const NAME_VON: &[&str] = &["of", "for", "der", "des", "für", "von"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Namensbezug {
    Person,
    Ding,
}

/// Was ein Text über „Name" sagt; `None` ohne Namenswort. Eine Lesart als
/// Person irgendwo im Text schlägt die als Ding (auditmysite#658).
fn namensbezug(text: &str) -> Option<Namensbezug> {
    let w = woerter(text);
    let mut ergebnis = None;
    for (i, wort) in w.iter().enumerate() {
        if PERSONENNAME.contains(&wort.as_str()) {
            return Some(Namensbezug::Person);
        }
        if wort != "name" {
            continue;
        }
        let davor = w[..i]
            .iter()
            .rev()
            .find(|x| !x.chars().all(|c| c.is_ascii_digit()));
        let ding = davor.is_some_and(|d| !NAME_NEUTRAL.contains(&d.as_str()))
            || w.get(i + 1).is_some_and(|n| NAME_VON.contains(&n.as_str()));
        if !ding {
            return Some(Namensbezug::Person);
        }
        ergebnis = Some(Namensbezug::Ding);
    }
    ergebnis
}

/// Ob das Feld nach dem Namen der Person fragt. Die Beschriftung entscheidet;
/// ein Ding in `id` oder `name` (`#view-name` mit Beschriftung „Name") nimmt
/// es zurück, außer das andere Attribut nennt die Person. Ohne Namenswort in
/// der Beschriftung zählt ein Personenname in `id` oder `name`.
fn fragt_nach_namen(beschriftung: &str, attribute: [Option<Namensbezug>; 2]) -> bool {
    match namensbezug(beschriftung) {
        Some(Namensbezug::Person) => {
            attribute.contains(&Some(Namensbezug::Person))
                || !attribute.contains(&Some(Namensbezug::Ding))
        }
        Some(Namensbezug::Ding) => false,
        None => attribute.contains(&Some(Namensbezug::Person)),
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
        if !ist_input(n, &["text", "email", "tel", "url", "password"])
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
        let ohne = n.attr("autocomplete").is_none_or(|v| {
            let v = v.trim();
            v.is_empty() || v.eq_ignore_ascii_case("on") || v.eq_ignore_ascii_case("off")
        });
        if !ohne {
            continue;
        }
        let beschriftung = doc.accessible_name(n).unwrap_or_default();
        let attribute = [n.attr("id"), n.attr("name")].map(|a| a.unwrap_or(""));
        let person = matches!(input_typ(n).as_str(), "email" | "tel")
            || fragt_nach_person(&beschriftung)
            || attribute.iter().any(|a| fragt_nach_person(a))
            || fragt_nach_namen(&beschriftung, attribute.map(namensbezug));
        if !person {
            continue;
        }
        let feld = beschriftung.trim();
        out.push(
            Finding::review(
                "forms/purpose-missing",
                tr!(
                    locale,
                    "The field \"{feld}\" seems to ask for information about the user but has no autocomplete token.",
                    "Das Feld \"{feld}\" fragt offenbar nach Angaben zur Person, hat aber keine autocomplete-Angabe.",
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
const PFLICHT: &[&str] = &[
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
const FORMATBEDARF: &[&str] = &[
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
const FORMATHINWEIS: &[&str] = &[
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
fn formatbegriff(name: &str, begriff: &str) -> bool {
    if begriff.chars().count() >= 5 {
        return name.contains(begriff);
    }
    name.match_indices(begriff).any(|(start, _)| {
        let davor = name[..start].chars().next_back();
        let danach = name[start + begriff.len()..].chars().next();
        !davor.is_some_and(char::is_alphanumeric) && !danach.is_some_and(char::is_alphanumeric)
    })
}

fn ist_pflicht<'a, N: Node<'a>>(n: N) -> bool {
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
        if !feldrolle(doc, n).is_some_and(|r| EINGABE_ROLLEN.contains(&r.as_str())) {
            continue;
        }
        let Some(name) = doc.accessible_name(n).filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let name = name.to_lowercase();
        let (beschreibung, offen) = beschreibung(n, "aria-describedby", &ids);
        let beschreibung = beschreibung.to_lowercase();

        if ist_pflicht(n)
            && !PFLICHT
                .iter()
                .any(|p| name.contains(p) || beschreibung.contains(p))
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

        let nativ_datum = ist_input(n, &["date", "time", "datetime-local", "month", "week"]);
        let angeleitet = offen
            || !beschreibung.trim().is_empty()
            || FORMATHINWEIS.iter().any(|h| name.contains(h))
            || n.attr("placeholder").is_some_and(|p| !p.trim().is_empty());
        if !ist_input(n, &["radio", "checkbox"])
            && !nativ_datum
            && !angeleitet
            && FORMATBEDARF.iter().any(|b| formatbegriff(&name, b))
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

const TITEL_ROLLEN: &[&str] = &[
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
    let label_for: Vec<&str> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for"))
        .collect();
    for n in elements(doc) {
        if doc.is_ignored(n) || n.has_attr("placeholder") {
            continue;
        }
        if !feldrolle(doc, n).is_some_and(|r| TITEL_ROLLEN.contains(&r.as_str())) {
            continue;
        }
        let Some(name) = doc.accessible_name(n).filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let nur_titel = match doc.name_source(n) {
            Some(q) => q == NameSource::Title,
            None => {
                n.attr("title").is_some_and(|t| t.trim() == name.trim())
                    && !n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
                    && !n
                        .attr("aria-labelledby")
                        .is_some_and(|v| !v.trim().is_empty())
                    && !n.attr("id").is_some_and(|id| label_for.contains(&id))
                    && closest(n, "label").is_none()
            }
        };
        if nur_titel {
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
const KEINE_FORMULARGRUPPE: &[&str] = &[
    "details", "address", "hgroup", "figure", "article", "section", "aside", "optgroup",
];

/// Eine Formulargruppe ohne Namen: `<fieldset>` ohne `<legend>` oder ein
/// `role="group"` mit Eingabefeldern darin (1.3.1, 3.3.2). Die Gruppe sagt
/// dann nicht, welche Frage ihre Felder beantworten. `radiogroup` meldet
/// `names/required-missing`.
pub(crate) fn group_name_missing<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || doc.role(n).as_deref() != Some("group") {
            continue;
        }
        let formulargruppe = n.is_element("fieldset")
            || (!KEINE_FORMULARGRUPPE.contains(&n.local_name())
                && descendants(n).any(|d| {
                    feldrolle(doc, d).is_some_and(|r| EINGABE_ROLLEN.contains(&r.as_str()))
                }));
        if !formulargruppe || crate::semantics::named(doc, n) {
            continue;
        }
        out.push(
            Finding::fail(
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
    fn autocomplete_grammatik_nach_html() {
        for gut in [
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
            assert!(autocomplete_gueltig(gut), "{gut}");
        }
        for schlecht in [
            "not-a-real-token",
            "foo email",
            "work name",
            "email work",
            "billing section-a email",
            "on email",
            "webauthn",
            "shipping",
        ] {
            assert!(!autocomplete_gueltig(schlecht), "{schlecht}");
        }
    }

    /// auditmysite#658: Namen von Dingen sind keine Personendaten.
    #[test]
    fn name_eines_dings_ist_kein_personenname() {
        for ding in [
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
            assert!(!fragt_nach_namen(ding, [None, None]), "{ding}");
            assert!(!fragt_nach_person(ding), "{ding}");
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
                fragt_nach_namen(person, [None, None]) || fragt_nach_person(person),
                "{person}"
            );
        }
        let attr = |a: &str| [namensbezug(a), None];
        assert!(!fragt_nach_namen("Name", attr("view-name")));
        assert!(!fragt_nach_namen("Name", attr("projectName")));
        assert!(fragt_nach_namen("Name", attr("input-name")));
        assert!(fragt_nach_namen("Name", attr("billing_first_name")));
        assert!(fragt_nach_namen("Name", attr("field-2-name")));
        assert!(fragt_nach_namen(
            "Name",
            [namensbezug("view-name"), namensbezug("fullName")]
        ));
        // Die Beschriftung nennt ein Ding: `name="name"` macht keine Person
        // daraus.
        assert!(!fragt_nach_namen(
            "Project name",
            [None, namensbezug("name")]
        ));
    }

    #[test]
    fn personenwoerter_nur_als_ganze_woerter() {
        for ja in [
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
            assert!(fragt_nach_person(ja), "{ja}");
        }
        for nein in [
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
            assert!(!fragt_nach_person(nein), "{nein}");
        }
    }

    /// auditmysite#643: kurze Begriffe nur als ganzes Wort.
    #[test]
    fn formatbegriffe_kurz_nur_als_wort() {
        let braucht = |name: &str| {
            let name = name.to_lowercase();
            FORMATBEDARF.iter().any(|b| formatbegriff(&name, b))
        };
        assert!(!braucht("Was ist passiert?"));
        assert!(!braucht("Hotel"));
        assert!(!braucht("Last update"));
        assert!(!braucht("Quantity"));
        assert!(braucht("Date of birth"));
        assert!(braucht("Tel."));
        assert!(braucht("Reisepass"));
        assert!(braucht("Geburtsdatum"));
        assert!(braucht("Postal code"));
    }
}
