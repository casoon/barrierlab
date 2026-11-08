//! „Nicht gemessen" als Ausführungsvermerk statt als Befund.
//!
//! Eine Regel hat `fn(&D, Locale, &mut Vec<Finding>)` als Gestalt und kann
//! deshalb nur Befunde liefern. Fehlt ihr eine Messung des Hosts, legt sie
//! mit [`note`] einen markierten `Untested`-Befund ab; die Läufer
//! (`record_runs`) nehmen ihn aus den Befunden und führen die Kennung als
//! `RuleRun::not_run(…, NotRun::CapabilityMissing)` mit dem Grund. Der Bericht
//! bekommt so keinen Wurzelbefund, nach dem der Host filtern müsste, und die
//! Kennung gilt trotzdem nicht als bestanden.

use a11y_report::Finding;

const MARKER: &str = "a11y-rules:not-run";

/// Die Regel `id` konnte nicht laufen, weil dem Host die Messung fehlt.
pub(crate) fn note(id: &str, reason: impl Into<String>) -> Finding {
    Finding::untested(id, reason).with_tags([MARKER])
}

pub(crate) fn is_note(f: &Finding) -> bool {
    f.tags.iter().any(|t| t == MARKER)
}
