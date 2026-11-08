//! Regeln über Messungen, die ein Host an der laufenden Seite vornimmt
//! (casoon/barrierlab#130): Rahmen, Drehung der Wurzel, Einfügen in
//! Passwortfelder, Fokus unter fixierten Elementen, Textabstand.
//!
//! Jede dieser Regeln liest ein Feld, das ein Host liefern *kann* und das die
//! Vorgabe von [`Rendering`] mit `None` („nicht gemessen") beantwortet. Fehlt
//! die Messung an allen Elementen, auf die die Regel zielt, entsteht ein
//! Vermerk `NotRun::CapabilityMissing` mit Grund (`crate::not_run`), nie ein
//! stilles Bestehen. Gibt es nichts, worauf die Regel zielt, lief sie und
//! meldet nichts.

use a11y_dom::{Node, NodeId, Rendering, Tier, elements};
use a11y_report::{Evidence, Finding, Location, Severity};

use crate::locale::{Locale, de_text, pick, tr};
use crate::not_run;
use crate::registry::Meta;
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- frames/cross-origin ------------------------------------------------------

/// Ob ein Rahmen im Sinne der Abdeckungsnotiz „dargestellt" ist: nicht
/// präsentational, nicht versteckt, größer als 1 px in beiden Richtungen
/// (auditmysite-Umfang).
fn frame_in_scope<'a, D: Rendering>(doc: &'a D, n: D::N<'a>) -> bool {
    let role = n
        .attr("role")
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_ascii_lowercase);
    if matches!(role.as_deref(), Some("none" | "presentation"))
        || n.has_attr("hidden")
        || n.attr("aria-hidden")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    {
        return false;
    }
    if let Some(style) = doc.computed_style(n)
        && (style.display.as_deref() == Some("none")
            || matches!(style.visibility.as_deref(), Some("hidden" | "collapse")))
    {
        return false;
    }
    !doc.bounds(n)
        .is_some_and(|b| b.width <= 1.0 || b.height <= 1.0)
}

/// Kein Kriterium: ein Dokument, das das Werkzeug nicht lesen kann, lässt
/// sich an keinem messen. Das ist eine Abdeckungsnotiz, keine Konformitätsprüfung.
fn frames<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut measured = false;
    let mut candidates = false;
    for n in elements(doc).filter(|n| matches!(n.local_name(), "iframe" | "frame")) {
        if !frame_in_scope(doc, n) {
            continue;
        }
        candidates = true;
        match doc.frame_reachable(n) {
            None => {}
            Some(true) => measured = true,
            Some(false) => {
                measured = true;
                let src = n.attr("src").unwrap_or("");
                out.push(
                    Finding::untested(
                        "frames/cross-origin",
                        tr!(
                            locale,
                            "The document in this frame (src: {src}) cannot be read, so it was \
                             not tested. It is cross-origin, or its navigation had not finished. \
                             Check it by hand or test its address on its own.",
                            "Das Dokument in diesem Rahmen (src: {src}) ist nicht lesbar und \
                             blieb deshalb ungeprüft. Es hat einen fremden Ursprung, oder seine \
                             Navigation war nicht abgeschlossen. Von Hand prüfen oder die \
                             Adresse für sich prüfen."
                        ),
                    )
                    .with_severity(Severity::Low)
                    .with_evidence([Evidence::dom_attribute(
                        "src",
                        n.attr("src").map(str::to_string),
                    )])
                    .at(at(n.id())),
                );
            }
        }
    }
    if candidates && !measured {
        out.push(not_run::note(
            "frames/cross-origin",
            pick!(
                locale,
                "host does not report frame access",
                "Host meldet den Zugriff auf Rahmen nicht",
            ),
        ));
    }
}

pub(crate) const METAS: &[Meta] = &[Meta {
    ids: &["frames/cross-origin"],
    tier: Tier::Rendering,
    scope: Scope::Rendered,
    wcag: &[],
    severity: Severity::Low,
    help: "A frame whose document the tool cannot read (cross-origin, or still navigating) \
           was not tested. Not a conformance check and tied to no success criterion: a \
           coverage note. Frames that are presentational, hidden or 1 px or smaller are \
           skipped.",
    help_de: de_text!(
        "Ein Rahmen, dessen Dokument das Werkzeug nicht lesen kann (fremder Ursprung oder \
              Navigation läuft noch), blieb ungeprüft. Keine Konformitätsprüfung und an kein \
              Erfolgskriterium gebunden: eine Abdeckungsnotiz. Präsentationale, versteckte und \
              höchstens 1 px große Rahmen bleiben außen vor."
    ),
}];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn functions<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 1] {
    [frames]
}

const _: () = assert!(METAS.len() == 1);
