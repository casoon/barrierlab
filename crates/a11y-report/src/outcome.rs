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

/// Worauf stützt sich die Aussage: ein Erfolgskriterium oder eine Empfehlung?
///
/// Dritte, von [`Outcome`] und [`Severity`] unabhängige Angabe. Eine Empfehlung
/// („Best Practice") belegt nicht, dass ein Erfolgskriterium verfehlt ist: eine h2 ohne h1
/// ist unter 1.3.1 als Überschrift bestimmbar, WCAG verlangt keine Ebene. Ein
/// Host muss das deshalb nicht aus einem Schlagwort ableiten und nicht selbst
/// herabstufen — [`Finding::best_practice`] liefert zugleich [`Outcome::Review`].
///
/// [`Finding::best_practice`]: crate::Finding::best_practice
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// Die Aussage hängt an einem Erfolgskriterium (oder ist eine manuelle Frage dazu).
    #[default]
    Criterion,
    /// Empfehlung ohne Erfolgskriterium. Kein Verstoß, kein Punktabzug nötig.
    BestPractice,
}

impl Basis {
    pub fn is_criterion(&self) -> bool {
        *self == Basis::Criterion
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Basis::Criterion => "criterion",
            Basis::BestPractice => "best_practice",
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

    /// Die Stufe eines Erfolgskriteriums von WCAG 2.2, etwa `"1.4.3"` →
    /// `AA`. `None` für Unbekanntes und für das in 2.2 entfernte 4.1.1.
    pub fn of_criterion(criterion: &str) -> Option<WcagLevel> {
        const A: &[&str] = &[
            "1.1.1", "1.2.1", "1.2.2", "1.2.3", "1.3.1", "1.3.2", "1.3.3", "1.4.1", "1.4.2",
            "2.1.1", "2.1.2", "2.1.4", "2.2.1", "2.2.2", "2.3.1", "2.4.1", "2.4.2", "2.4.3",
            "2.4.4", "2.5.1", "2.5.2", "2.5.3", "2.5.4", "3.1.1", "3.2.1", "3.2.2", "3.2.6",
            "3.3.1", "3.3.2", "3.3.7", "4.1.2",
        ];
        const AA: &[&str] = &[
            "1.2.4", "1.2.5", "1.3.4", "1.3.5", "1.4.3", "1.4.4", "1.4.5", "1.4.10", "1.4.11",
            "1.4.12", "1.4.13", "2.4.5", "2.4.6", "2.4.7", "2.4.11", "2.5.7", "2.5.8", "3.1.2",
            "3.2.3", "3.2.4", "3.3.3", "3.3.4", "3.3.8", "4.1.3",
        ];
        const AAA: &[&str] = &[
            "1.2.6", "1.2.7", "1.2.8", "1.2.9", "1.3.6", "1.4.6", "1.4.7", "1.4.8", "1.4.9",
            "2.1.3", "2.2.3", "2.2.4", "2.2.5", "2.2.6", "2.3.2", "2.3.3", "2.4.8", "2.4.9",
            "2.4.10", "2.4.12", "2.4.13", "2.5.5", "2.5.6", "3.1.3", "3.1.4", "3.1.5", "3.1.6",
            "3.2.5", "3.3.5", "3.3.6", "3.3.9",
        ];
        let c = criterion.trim();
        if A.contains(&c) {
            Some(WcagLevel::A)
        } else if AA.contains(&c) {
            Some(WcagLevel::AA)
        } else if AAA.contains(&c) {
            Some(WcagLevel::AAA)
        } else {
            None
        }
    }

    /// Ist diese Stufe in einer Prüfung auf `target` enthalten?
    /// AA schließt A ein, AAA schließt beide ein.
    pub fn included_in(self, target: WcagLevel) -> bool {
        self <= target
    }
}
