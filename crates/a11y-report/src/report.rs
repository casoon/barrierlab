//! Der Bericht: Befunde plus die Frage, ob jede Regel überhaupt laufen konnte.

use serde::{Deserialize, Serialize};

use crate::finding::Finding;
use crate::outcome::{Outcome, Severity};

/// Warum eine Regel nicht gelaufen ist.
///
/// Der Unterschied zwischen „lief und fand nichts" und „konnte nicht laufen"
/// ist der Kern des Vier-Zustands-Modells. Ohne ihn liest sich eine
/// Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotRun {
    /// Der Host liefert die nötigen Daten nicht — z. B. Rendering-Werte bei
    /// statischer HTML-Analyse. Die Regel ist nicht anwendbar, nicht bestanden.
    CapabilityMissing,
    /// Die Regel wurde per Konfiguration abgeschaltet.
    Disabled,
    /// Auf dieser Seite gibt es nichts zu prüfen — kein Formular, kein Video.
    NotApplicable,
    /// Die Regel ist gelaufen und hat einen Fehler geworfen.
    Errored,
}

/// Ein Ausführungsvermerk je Regel. Beantwortet „lief das überhaupt?", während
/// [`Finding`] beantwortet „was kam dabei heraus?".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRun {
    pub rule_id: String,
    /// Der Durchgang, in dem diese Regel lief — etwa `desktop` oder `mobile`.
    ///
    /// Werkzeuge, die dieselbe Seite mehrfach unter verschiedenen Bedingungen
    /// prüfen, führen je Durchgang einen eigenen Vermerk. Der Schlüssel eines
    /// Vermerks ist dann `(rule_id, viewport)`, nicht `rule_id` allein.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    /// Die Erfolgskriterien, die mit dieser Regel stehen und fallen. Nötig,
    /// damit ein Bericht auch für eine **nicht** gelaufene Regel sagen kann,
    /// welches Kriterium ungeprüft blieb.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wcag: Vec<String>,
    /// `None` = gelaufen. `Some(_)` = nicht gelaufen, mit Grund.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_run: Option<NotRun>,
    /// Wie viele Befunde diese Regel beigetragen hat.
    #[serde(default)]
    pub findings: usize,
    /// Freitext zur Einordnung, etwa welche Fähigkeit gefehlt hat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl RuleRun {
    pub fn ran(rule_id: impl Into<String>, findings: usize) -> Self {
        RuleRun {
            rule_id: rule_id.into(),
            viewport: None,
            wcag: Vec::new(),
            not_run: None,
            findings,
            reason: None,
        }
    }

    pub fn not_run(rule_id: impl Into<String>, why: NotRun) -> Self {
        RuleRun {
            rule_id: rule_id.into(),
            viewport: None,
            wcag: Vec::new(),
            not_run: Some(why),
            findings: 0,
            reason: None,
        }
    }

    /// Hält fest, in welchem Durchgang die Regel lief.
    pub fn in_viewport(mut self, v: impl Into<String>) -> Self {
        self.viewport = Some(v.into());
        self
    }

    pub fn with_wcag(mut self, criteria: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.wcag = criteria.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_reason(mut self, r: impl Into<String>) -> Self {
        self.reason = Some(r.into());
        self
    }

    pub fn did_run(&self) -> bool {
        self.not_run.is_none()
    }
}

/// Zählwerk über einen Befundsatz. Bewusst keine Gesamtnote: Ein aggregierter
/// Score würde genau die Unterscheidung einebnen, für die es die vier Zustände
/// gibt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub fail: usize,
    pub review: usize,
    pub pass: usize,
    pub untested: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    /// Regeln, die nicht laufen konnten.
    pub rules_not_run: usize,
}

impl Summary {
    /// Zählt Probleme: `Fail` und `Review`. `Untested` zählt nicht mit, weil es
    /// gar keine Aussage trifft.
    pub fn problems(&self) -> usize {
        self.fail + self.review
    }
}

/// Das Gesamtergebnis eines Laufs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_runs: Vec<RuleRun>,
    pub summary: Summary,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, f: Finding) {
        self.findings.push(f);
    }

    pub fn extend(&mut self, fs: impl IntoIterator<Item = Finding>) {
        self.findings.extend(fs);
    }

    pub fn record(&mut self, r: RuleRun) {
        self.rule_runs.push(r);
    }

    /// Nur die Befunde eines Zustands.
    pub fn by_outcome(&self, o: Outcome) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(move |f| f.outcome == o)
    }

    /// Was standardmäßig angezeigt wird — alles außer `Pass`.
    pub fn visible(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| f.outcome.is_visible_by_default())
    }

    /// Rechnet [`Report::summary`] aus den aktuellen Befunden neu.
    /// Muss nach dem Befüllen einmal aufgerufen werden.
    pub fn finish(mut self) -> Self {
        let mut s = Summary::default();
        for f in &self.findings {
            match f.outcome {
                Outcome::Fail => s.fail += 1,
                Outcome::Review => s.review += 1,
                Outcome::Pass => s.pass += 1,
                Outcome::Untested => s.untested += 1,
            }
            // Severity wird nur über Probleme gezählt; ein bestandener Test hat
            // keine Schwere.
            if f.outcome.is_problem() {
                match f.severity {
                    Severity::Critical => s.critical += 1,
                    Severity::High => s.high += 1,
                    Severity::Medium => s.medium += 1,
                    Severity::Low => s.low += 1,
                }
            }
        }
        s.rules_not_run = self.rule_runs.iter().filter(|r| !r.did_run()).count();
        self.summary = s;
        self
    }
}
