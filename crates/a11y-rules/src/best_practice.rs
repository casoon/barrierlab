//! Die Kennungen, die Empfehlungen sind und kein Erfolgskriterium belegen.
//!
//! Ein Host liest die Einordnung hier ab (oder am Befund selbst:
//! [`Finding::is_best_practice`]) und muss weder ein Schlagwort deuten noch
//! selbst herabstufen. Befunde dieser Kennungen tragen [`Outcome::Review`] und
//! [`Basis::BestPractice`]; sie zählen nicht als WCAG-Verstoß.
//!
//! Nicht hier: die Darstellungskonvention (`viz/*`, `display/*`). Sie trägt
//! ebenfalls das Schlagwort `best-practice`, hängt aber bewusst am nächsten
//! Erfolgskriterium und meldet nur auf Seiten, die die Konvention benutzen.
//!
//! [`Finding::is_best_practice`]: a11y_report::Finding::is_best_practice
//! [`Outcome::Review`]: a11y_report::Outcome::Review
//! [`Basis::BestPractice`]: a11y_report::Basis::BestPractice

use crate::locale::{Locale, pick};

/// Eine Kennung, die eine Empfehlung ist, mit der normativen Begründung.
#[derive(Debug, Clone, Copy)]
pub struct BestPracticeRule {
    pub id: &'static str,
    /// Warum kein Erfolgskriterium: englisch. Für andere Sprachen
    /// [`BestPracticeRule::reason_in`].
    pub reason: &'static str,
    /// Dieselbe Begründung auf Deutsch. Nur mit dem Feature `de`.
    #[cfg(feature = "de")]
    pub reason_de: &'static str,
}

impl BestPracticeRule {
    pub fn reason_in(&self, locale: Locale) -> &'static str {
        pick!(locale, self.reason, self.reason_de)
    }
}

macro_rules! rule {
    ($id:expr, $en:expr, $de:expr) => {
        BestPracticeRule {
            id: $id,
            reason: $en,
            #[cfg(feature = "de")]
            reason_de: $de,
        }
    };
}

/// Alle Empfehlungen des Bestands, nach Kennung geordnet.
pub const BEST_PRACTICE_RULES: &[BestPracticeRule] = &[
    rule!(
        "aria/role-redundant",
        "The role is correct, 4.1.2 is met; ARIA in HTML merely discourages repeating the implicit role.",
        "Die Rolle ist richtig, 4.1.2 ist erfüllt; ARIA in HTML rät nur davon ab, die implizite Rolle zu wiederholen."
    ),
    rule!(
        "headings/h1-missing",
        "A correctly marked-up h2 is programmatically determinable as a heading (1.3.1); WCAG does not require a particular heading level.",
        "Eine korrekt ausgezeichnete h2 ist als Überschrift bestimmbar (1.3.1); WCAG verlangt keine bestimmte Ebene."
    ),
    rule!(
        "headings/skip-level",
        "1.3.1 requires determinable levels, not gapless ones; G141 is sufficient, not mandatory.",
        "1.3.1 verlangt bestimmbare Ebenen, keine lückenlosen; G141 ist hinreichend, nicht zwingend."
    ),
    rule!(
        "landmarks/banner-duplicate",
        "Several banner landmarks are WAI-ARIA practice; no success criterion limits their number (axe landmark-no-duplicate-banner is best practice).",
        "Mehrere banner-Landmarks sind WAI-ARIA-Praxis; kein Erfolgskriterium begrenzt ihre Zahl."
    ),
    rule!(
        "landmarks/contentinfo-duplicate",
        "Several contentinfo landmarks are WAI-ARIA practice; no success criterion limits their number (axe landmark-no-duplicate-contentinfo is best practice).",
        "Mehrere contentinfo-Landmarks sind WAI-ARIA-Praxis; kein Erfolgskriterium begrenzt ihre Zahl."
    ),
    rule!(
        "landmarks/content-outside",
        "No criterion requires content to sit in landmarks (axe region is best practice).",
        "Kein Kriterium verlangt, dass Inhalt in Landmarks steht (axe region ist Best Practice)."
    ),
    rule!(
        "landmarks/main-duplicate",
        "HTML forbids multiple visible main elements, WCAG does not (4.1.1 was dropped).",
        "HTML verbietet mehrere sichtbare main-Elemente, WCAG nicht (4.1.1 entfiel)."
    ),
    rule!(
        "landmarks/main-missing",
        "1.3.1 does not require landmarks; 2.4.1 is also met via headings (H69) or a skip link (G1).",
        "1.3.1 verlangt keine Landmarks; 2.4.1 erfüllen auch Überschriften (H69) oder ein Sprunglink (G1)."
    ),
    rule!(
        "landmarks/not-top-level",
        "No criterion requires landmarks to be top-level (axe landmark-*-is-top-level is best practice).",
        "Kein Kriterium verlangt oberste Landmarks (axe landmark-*-is-top-level ist Best Practice)."
    ),
    rule!(
        "lists/empty",
        "An empty list withholds no information.",
        "Eine leere Liste enthält keine Information vor."
    ),
    rule!(
        "lists/role-redundant",
        "With visible markers role=\"list\" repeats the implicit role and restores nothing; 4.1.2 is met.",
        "Bei sichtbaren Zeichen wiederholt role=\"list\" die implizite Rolle und stellt nichts wieder her; 4.1.2 ist erfüllt."
    ),
    rule!(
        "patterns/tooltip-unreferenced",
        "1.4.13 governs appearing and disappearing on hover and focus, not the association.",
        "1.4.13 regelt Erscheinen und Verschwinden bei Hover und Fokus, nicht die Verknüpfung."
    ),
    rule!(
        "zoom/viewport-missing",
        "Without a viewport declaration zooming stays possible (1.4.4) and reflow is measured in the browser (1.4.10); the declaration does not touch either.",
        "Ohne Viewport-Angabe bleibt Zoomen möglich (1.4.4), und der Umbruch wird im Browser geprüft (1.4.10); die Angabe berührt beides nicht."
    ),
    rule!(
        "zoom/viewport-scale-limited",
        "A maximum-scale of at least 2 does not block zooming under 1.4.4; the 500 % limit is a recommendation.",
        "Ein maximum-scale ab 2 verhindert das Zoomen nach 1.4.4 nicht; die Grenze von 500 % ist eine Empfehlung."
    ),
];

/// Die Begründung zu einer Kennung, wenn sie eine Empfehlung ist.
pub fn best_practice_rule(id: &str) -> Option<&'static BestPracticeRule> {
    BEST_PRACTICE_RULES.iter().find(|r| r.id == id)
}
