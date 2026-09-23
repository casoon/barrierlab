//! Die zwei Klassifikationsachsen: *wie sicher* ist die Aussage (`Outcome`)
//! und *wie schwer* wiegt das Problem (`Severity`).
//!
//! Eine dritte Achse „certainty" gibt es bewusst nicht — die Gewissheit steckt
//! bereits im Zustand. Eine Regel, die ihre Aussage nur heuristisch treffen
//! kann, liefert [`Outcome::Review`] statt [`Outcome::Fail`] mit niedriger
//! Gewissheit.

use serde::{Deserialize, Serialize};

/// Wie sicher ist die Aussage über dieses Ergebnis?
///
/// Bewusst kein aggregierter Score: Ein einzelner Prozentwert verschleiert den
/// Unterschied zwischen sicher automatisch erkannt, heuristisch vermutet und
/// grundsätzlich nicht automatisierbar.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Automatisch festgestelltes Problem. Die Regel konnte es eindeutig belegen.
    #[default]
    Fail,
    /// Potenzielles Problem. Die Regel kann es nur heuristisch vermuten und
    /// braucht menschliche Bestätigung — etwa ein vorhandener, aber womöglich
    /// nichtssagender Alt-Text.
    Review,
    /// Automatische Prüfung bestanden. Wird intern geführt, aber standardmäßig
    /// nicht visualisiert.
    Pass,
    /// Automatisiert nicht beurteilbar. Entweder grundsätzlich (Screenreader-
    /// Erfahrung, inhaltliche Verständlichkeit) oder weil die aufrufende
    /// Oberfläche die nötigen Daten nicht liefern kann — etwa eine
    /// Kontrastprüfung ohne Rendering-Zugriff.
    ///
    /// Erzeugt eine manuelle Prüfliste, kein automatisches Urteil.
    Untested,
}

impl Outcome {
    /// Wird dieses Ergebnis standardmäßig angezeigt? `Pass` nicht.
    pub fn is_visible_by_default(self) -> bool {
        !matches!(self, Outcome::Pass)
    }

    /// Zählt dieses Ergebnis als Problem? `Review` zählt mit, weil es eine
    /// offene Frage ist; `Untested` nicht, weil es gar keine Aussage trifft.
    pub fn is_problem(self) -> bool {
        matches!(self, Outcome::Fail | Outcome::Review)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Fail => "fail",
            Outcome::Review => "review",
            Outcome::Pass => "pass",
            Outcome::Untested => "untested",
        }
    }
}

/// Wie schwer wiegt das Problem — unabhängig davon, wie sicher es ist.
///
/// `Critical` + [`Outcome::Untested`] ist fachlich etwas anderes als `Medium` +
/// [`Outcome::Fail`]; genau deshalb sind es zwei Achsen.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Verbesserungswürdig, aber ohne spürbare Auswirkung.
    Low,
    /// Relevantes, aber nicht existenzielles Problem.
    #[default]
    Medium,
    /// Klares Problem mit spürbarer Auswirkung.
    High,
    /// Schwerwiegend: hohe Auswirkung oder hohes rechtliches Risiko.
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }

    /// axe-core-kompatible Bezeichnung, für Werkzeuge, die diese Vokabel erwarten.
    pub fn as_axe_impact(self) -> &'static str {
        match self {
            Severity::Low => "minor",
            Severity::Medium => "moderate",
            Severity::High => "serious",
            Severity::Critical => "critical",
        }
    }
}

/// WCAG-Konformitätsstufe des zugrundeliegenden Erfolgskriteriums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WcagLevel {
    A,
    AA,
    AAA,
}

impl WcagLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            WcagLevel::A => "A",
            WcagLevel::AA => "AA",
            WcagLevel::AAA => "AAA",
        }
    }

    /// Ist diese Stufe in einer Prüfung auf `target` enthalten?
    /// AA schließt A ein, AAA schließt beide ein.
    pub fn included_in(self, target: WcagLevel) -> bool {
        self <= target
    }
}
