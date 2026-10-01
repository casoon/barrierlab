//! Namensregeln nach WAI-ARIA 1.2, accname 1.2, HTML-AAM und WCAG 2.5.3.
//!
//! Portiert aus auditmysite (`aria_naming_rules`, `dialog_rules`,
//! `summary_name`, `status_messages`, `label_in_name`, `accessible_name`;
//! casoon/barrierlab#15). Norm ist die Spezifikation, auditmysite der
//! Vergleichspunkt — Abweichungen stehen an der jeweiligen Stelle und im
//! CHANGELOG.
//!
//! Kennungen: Elementfamilien mit eigener Namensregel heißen
//! `<familie>/name-missing` wie schon `links/`, `buttons/`, `svg/` und
//! `tables/`; hier kommen `dialog/` und `summary/` dazu. Alle übrigen Rollen,
//! für die ARIA einen Namen verlangt, laufen unter `names/required-missing` —
//! die Rolle steht in der Meldung, nicht in der Kennung.

use a11y_dom::{NameSource, Node, NodeId, NodeKind, Semantics, ancestors, descendants, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::semantics::named;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- names/required-missing -----------------------------------------------

/// Rollen mit „Accessible Name Required: True" in WAI-ARIA 1.2, soweit
/// auditmysite sie prüfte. Ohne `button` (`buttons/name-missing`) und
/// `dialog`/`alertdialog` (`dialog/name-missing`).
///
/// Dazu `tab`: ARIA 1.2 führt für ihn kein „Name Required", aber ein Tab ist
/// ein Bedienelement, und WCAG 4.1.2 verlangt für jedes einen bestimmbaren
/// Namen. auditmysite meldete den leeren Tab (`aria-label`); ohne ihn ginge
/// der Fall verloren.
///
/// Nicht übernommen: `menu` — ein Container, kein Bedienelement; ARIA 1.2
/// verlangt keinen Namen. Die weiteren Pflichtrollen der Spezifikation
/// (`region`, `img`, `table`, `grid`, `tree`, …) haben eigene Regeln oder
/// keinen Beleg im Korpus.
const NAME_PFLICHT: &[&str] = &[
    "checkbox",
    "combobox",
    "link",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "option",
    "progressbar",
    "radio",
    "radiogroup",
    "searchbox",
    "slider",
    "spinbutton",
    "switch",
    "tab",
    "textbox",
    "treeitem",
];

/// Ob ein natives Formularfeld überhaupt eine Beschriftung behauptet — dieselbe
/// Prüfung wie `forms/label-missing`. Fehlt sie, meldet jene Regel das Feld;
/// hier bleibt der Fall, dass die Beschriftung da ist, aber leer ausgeht.
fn beschriftung_behauptet<'a, N: Node<'a>>(n: N, label_for: &[&str]) -> bool {
    n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
        || n.has_attr("aria-labelledby")
        || n.attr("id").is_some_and(|id| label_for.contains(&id))
        || ancestors(n).any(|a| a.is_element("label"))
        || n.attr("title").is_some_and(|v| !v.trim().is_empty())
}

pub(crate) fn required_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let label_for: Vec<&str> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for"))
        .collect();

    for n in elements(doc) {
        if doc.is_ignored(n) {
            continue;
        }
        let Some(rolle) = doc.role(n) else { continue };
        if !NAME_PFLICHT.contains(&rolle.as_str()) {
            continue;
        }
        // Schon von einer eigenen Regel erfasst: `a[href]` von
        // `links/name-missing`.
        if n.is_element("a") && n.has_attr("href") {
            continue;
        }
        // Die Optionen eines `<select>` stellt Chrome nicht als `option` aus,
        // und die leere Platzhalteroption ist gängig und harmlos.
        if n.is_element("option") {
            continue;
        }
        if matches!(n.local_name(), "input" | "select" | "textarea")
            && !beschriftung_behauptet(n, &label_for)
        {
            continue;
        }
        if named(doc, n) {
            continue;
        }
        // auditmysite ordnet `meter` und `progressbar` 1.1.1 zu (axe), die
        // übrigen 4.1.2.
        let wcag: &[&str] = if matches!(rolle.as_str(), "meter" | "progressbar") {
            &["1.1.1"]
        } else {
            &["4.1.2"]
        };
        out.push(
            Finding::fail(
                "names/required-missing",
                tr!(
                    locale,
                    "The element with role \"{rolle}\" has no accessible name.",
                    "Das Element mit der Rolle \"{rolle}\" hat keinen zugänglichen Namen.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(wcag.iter().copied())
            .at(at(n.id())),
        );
    }
}

// --- names/symbol-only ----------------------------------------------------

/// Die interaktiven Rollen aus auditmysite `accessible_name`.
const INTERAKTIV: &[&str] = &[
    "button",
    "link",
    "textbox",
    "checkbox",
    "radio",
    "combobox",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "tab",
    "treeitem",
    "slider",
    "spinbutton",
    "searchbox",
    "switch",
];

/// Ob der Name aus einem Autorenattribut stammt (`aria-label`,
/// `aria-labelledby`, `title`). Mit `name_source` vom Host sicher, ohne es aus
/// den Attributen erschlossen: `aria-labelledby` und `aria-label` schlagen den
/// Inhalt, `title` zählt nur, wenn er den Namen wörtlich stellt.
fn name_aus_attribut<'n, D: Semantics>(doc: &'n D, n: D::N<'n>, name: &str) -> bool {
    match doc.name_source(n) {
        Some(q) => matches!(
            q,
            NameSource::AriaLabel | NameSource::AriaLabelledBy | NameSource::Title
        ),
        None => {
            n.attr("aria-labelledby")
                .is_some_and(|v| !v.trim().is_empty())
                || n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
                || n.attr("title").is_some_and(|v| v.trim() == name.trim())
        }
    }
}

/// Ein Name aus einem einzigen Zeichen, das weder Buchstabe noch Ziffer ist —
/// „×", „☰", „→". Ein Screenreader liest daraus „mal" oder nichts.
///
/// `REVIEW` statt auditmysites Verstoß: 4.1.2 verlangt einen Namen, über
/// seine Güte sagt es nichts. Buchstaben zählen nach Unicode, nicht nur ASCII
/// („Ä" ist kein Symbol).
pub(crate) fn symbol_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) {
            continue;
        }
        let Some(rolle) = doc.role(n) else { continue };
        if !INTERAKTIV.contains(&rolle.as_str()) {
            continue;
        }
        let Some(name) = doc.accessible_name(n) else {
            continue;
        };
        let mut zeichen = name.trim().chars();
        let (Some(z), None) = (zeichen.next(), zeichen.next()) else {
            continue;
        };
        if z.is_alphanumeric() || !name_aus_attribut(doc, n, &name) {
            continue;
        }
        out.push(
            Finding::review(
                "names/symbol-only",
                tr!(
                    locale,
                    "The accessible name \"{z}\" is only a symbol.",
                    "Der zugängliche Name \"{z}\" ist nur ein Symbol.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- dialog/* ---------------------------------------------------------------

/// Ein `<dialog>` ohne `open` ist geschlossen und nicht dargestellt (HTML,
/// „The dialog element"); er steht nicht im Accessibility-Tree.
fn geschlossen<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("dialog") && !n.has_attr("open")
}

/// Dialoge brauchen einen Namen (WAI-ARIA 1.2, `dialog` und `alertdialog`:
/// „Accessible Name Required"). Ersetzt auditmysite `aria-dialog-name` und
/// `dialog-name`, die denselben Fall doppelt meldeten.
pub(crate) fn dialog_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || geschlossen(n) {
            continue;
        }
        if !matches!(doc.role(n).as_deref(), Some("dialog" | "alertdialog")) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "dialog/name-missing",
                    pick!(
                        locale,
                        "The dialog has no accessible name.",
                        "Der Dialog hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

/// Ein `role="dialog"` ohne `aria-modal="true"`.
///
/// `REVIEW` statt auditmysites Verstoß: ARIA verlangt `aria-modal` nicht, und
/// ein nicht modaler Dialog trägt es zu Recht nicht. Ob der Dialog modal
/// gemeint ist, zeigt erst das Verhalten. Ein natives `<dialog>` bleibt außen
/// vor: Ob es per `showModal()` geöffnet wurde, liefert kein Tier.
pub(crate) fn dialog_modal<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) || n.is_element("dialog") {
            continue;
        }
        if doc.role(n).as_deref() != Some("dialog") {
            continue;
        }
        if n.attr("aria-modal").is_some_and(|v| v.trim() == "true") {
            continue;
        }
        out.push(
            Finding::review(
                "dialog/modal-unmarked",
                pick!(
                    locale,
                    "The dialog is not marked as modal (aria-modal=\"true\").",
                    "Der Dialog ist nicht als modal ausgezeichnet (aria-modal=\"true\").",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["4.1.2"])
            .at(at(n.id())),
        );
    }
}

// --- summary/name-missing ---------------------------------------------------

/// Die `<summary>`, die ihr `<details>` bedient: das erste `summary`-Kind
/// (HTML, „The details element"). Nur sie ist ein Bedienelement.
fn bedient_details<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("summary")
        && n.parent().is_some_and(|p| {
            p.is_element("details")
                && p.children()
                    .find(|k| k.kind() == NodeKind::Element && k.is_element("summary"))
                    .is_some_and(|erste| erste == n)
        })
}

/// Der Umschalter eines `<details>` braucht einen Namen. Über das Element
/// erkannt, nicht über die Rolle: `accname` gibt `button`, Chrome
/// `DisclosureTriangle`. auditmysite meldete zusätzlich das `<details>`
/// selbst; dessen Name stammt aber aus der `summary`, das war derselbe Fall
/// zweimal.
pub(crate) fn summary_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !bedient_details(n) || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "summary/name-missing",
                    pick!(
                        locale,
                        "The summary of the disclosure widget has no accessible name.",
                        "Die Zusammenfassung des Aufklappelements hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- status/live-overridden -------------------------------------------------

/// Eine Live-Region, deren `aria-live` die Vorgabe ihrer Rolle umstellt
/// (WAI-ARIA 1.2: `alert` assertive, `status` und `log` polite).
///
/// `off` stellt die Ansage ab — „updates to the region should not be
/// presented" —, das ist ein Verstoß gegen 4.1.3. Eine andere Dringlichkeit
/// dagegen wird weiter angesagt, nur früher oder später; ARIA erlaubt das
/// Überschreiben. Deshalb dort `REVIEW` statt auditmysites Verstoß.
/// Schweregrade wie in auditmysite: `alert` hoch, `status` und `log` mittel.
pub(crate) fn live_regions<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if doc.is_ignored(n) {
            continue;
        }
        let Some(live) = n.attr("aria-live") else {
            continue;
        };
        let live = live.trim().to_ascii_lowercase();
        let Some(rolle) = doc.role(n) else { continue };
        let (vorgabe, severity) = match rolle.as_str() {
            "alert" => ("assertive", Severity::High),
            "status" => ("polite", Severity::Medium),
            "log" => ("polite", Severity::Medium),
            _ => continue,
        };
        let finding = match live.as_str() {
            "off" => Finding::fail(
                "status/live-overridden",
                tr!(
                    locale,
                    "role=\"{rolle}\" has aria-live=\"off\": its updates are not announced.",
                    "role=\"{rolle}\" hat aria-live=\"off\": Änderungen werden nicht angesagt.",
                ),
            ),
            // Die Dringlichkeit von `log` umzustellen bemängelte auditmysite
            // nicht.
            "polite" | "assertive" if live != vorgabe && rolle != "log" => Finding::review(
                "status/live-overridden",
                tr!(
                    locale,
                    "role=\"{rolle}\" has aria-live=\"{live}\" instead of its default \"{vorgabe}\".",
                    "role=\"{rolle}\" hat aria-live=\"{live}\" statt der Vorgabe \"{vorgabe}\".",
                ),
            ),
            _ => continue,
        };
        out.push(
            finding
                .with_severity(severity)
                .with_wcag(["4.1.3"])
                .at(at(n.id())),
        );
    }
}

// --- label-in-name/mismatch -------------------------------------------------

/// Rollen, deren sichtbare Beschriftung ihr Inhalt ist (WAI-ARIA 1.2, „Name
/// From: contents"). auditmysite prüfte nur `button`.
const NAME_AUS_INHALT: &[&str] = &[
    "button",
    "checkbox",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "radio",
    "switch",
    "tab",
    "treeitem",
];

/// Kleinbuchstaben, Buchstaben und Ziffern nach Unicode, alles andere fällt
/// weg — auch die Leerzeichen. So stört weder Satzzeichen noch die fehlende
/// Trennung zwischen Blockelementen („A.B" statt „A. B", auditmysite#513).
fn normiert(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Ob der Name vom Autor statt aus dem Inhalt kommt.
fn name_vom_autor<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> bool {
    match doc.name_source(n) {
        Some(q) => matches!(q, NameSource::AriaLabel | NameSource::AriaLabelledBy),
        None => {
            n.attr("aria-labelledby")
                .is_some_and(|v| !v.trim().is_empty())
                || n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
        }
    }
}

/// Der sichtbare Text eines Elements: aller dargestellte Text darunter, auch
/// unter `aria-hidden` — verborgen vor der Assistenztechnik ist nicht
/// unsichtbar. Visuell versteckter Text (`.sr-only`) ist ohne Stile nicht zu
/// erkennen und zählt mit.
fn sichtbarer_text<'a, N: Node<'a>>(n: N) -> String {
    descendants(n)
        .filter(|k| k.kind() == NodeKind::Text)
        .map(|k| k.text())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ab diesem Verhältnis von sichtbarem Text zu Name ist das Element
/// vermutlich ein zusammengesetztes Widget (Karte mit Vorder- und Rückseite),
/// dessen knapper Name bewusst gewählt ist (auditmysite#513).
const ZUSAMMENGESETZT: usize = 2;

/// WCAG 2.5.3: Der Name enthält den sichtbaren Text. Wer per Sprache „Senden"
/// sagt, trifft den Button mit `aria-label="Formular abschicken"` nicht.
///
/// Verglichen wird ohne Groß-/Kleinschreibung, Satzzeichen und Leerraum.
/// Kein Befund, wenn der sichtbare Text nur aus Symbolen besteht (Icons nimmt
/// das Understanding-Dokument aus) oder wenn umgekehrt der Name im sichtbaren
/// Text steht (wie auditmysite). Ist der sichtbare Text mehr als doppelt so
/// lang wie der Name, `REVIEW` statt Verstoß.
pub(crate) fn label_in_name<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        // Die Regel sieht Dargestelltes, auch unter `aria-hidden`; ein Element
        // dort hat aber keinen Namen, den jemand spräche.
        if doc.is_ignored(n)
            || n.attr("aria-hidden") == Some("true")
            || ancestors(n).any(|a| a.attr("aria-hidden") == Some("true"))
        {
            continue;
        }
        let Some(rolle) = doc.role(n) else { continue };
        if !NAME_AUS_INHALT.contains(&rolle.as_str()) || !name_vom_autor(doc, n) {
            continue;
        }
        let Some(name) = doc.accessible_name(n) else {
            continue;
        };
        let sichtbar = sichtbarer_text(n);
        let (s, m) = (normiert(&sichtbar), normiert(&name));
        if s.is_empty() || m.is_empty() || m.contains(&s) || s.contains(&m) {
            continue;
        }
        let (name, sichtbar) = (
            name.trim(),
            sichtbar.split_whitespace().collect::<Vec<_>>().join(" "),
        );
        let text = tr!(
            locale,
            "The accessible name \"{name}\" does not contain the visible label \"{sichtbar}\".",
            "Der zugängliche Name \"{name}\" enthält die sichtbare Beschriftung \"{sichtbar}\" nicht.",
        );
        let f = if s.chars().count() > m.chars().count() * ZUSAMMENGESETZT {
            Finding::review("label-in-name/mismatch", text)
        } else {
            Finding::fail("label-in-name/mismatch", text)
        };
        out.push(
            f.with_severity(Severity::Medium)
                .with_wcag(["2.5.3"])
                .at(at(n.id())),
        );
    }
}
