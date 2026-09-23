//! Regeln als Funktionszeiger, nach Tier getrennt registriert.
//!
//! Keine Trait-Objekte: Eine Regel ist ein `fn`, die Registry ist ein Slice.
//! Das monomorphisiert pro Host, allokiert nichts pro Regel und hält die
//! Tier-Grenze im Typsystem — eine Tier-2-Regel kann gar nicht erst mit einem
//! Host aufgerufen werden, der [`Semantics`] nicht erfüllt.
//!
//! [`Semantics`]: a11y_dom::Semantics

use a11y_dom::{Document, Rendering, Semantics, Tier};
use a11y_report::{Finding, Severity};

/// Was über eine Regel unabhängig vom Host feststeht.
///
/// `ids` listet **alle** Befund-Kennungen, die diese Regel erzeugen kann.
/// Das ist kein Beiwerk: [`RuleRun`] wird je Kennung geführt, damit
/// `rule_runs` und `findings` dieselbe Namensmenge benutzen und sich
/// verbinden lassen. Eine Regel, die `images/alt` hieße, aber
/// `images/alt-missing` meldete, wäre für einen Auswerter nicht zuordenbar.
///
/// [`RuleRun`]: a11y_report::RuleRun
#[derive(Debug, Clone, Copy)]
pub struct Meta {
    /// Alle Befund-Kennungen dieser Regel, stabil über alle Oberflächen.
    pub ids: &'static [&'static str],
    /// Welche Datenschicht die Regel braucht.
    pub tier: Tier,
    /// WCAG-Erfolgskriterien, z. B. `["1.1.1"]`.
    pub wcag: &'static [&'static str],
    /// Vorgabeschwere. Einzelne Befunde dürfen davon abweichen.
    pub severity: Severity,
    pub help: &'static str,
}

/// Eine Regel auf [`Tier::Structure`] — Tags, Attribute, Text, Hierarchie.
pub struct StructureRule<D: Document> {
    pub meta: Meta,
    pub run: fn(&D, &mut Vec<Finding>),
}

impl<D: Document> Clone for StructureRule<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Document> Copy for StructureRule<D> {}

/// Eine Regel auf [`Tier::Semantics`] — braucht Rolle und Accessible Name.
pub struct SemanticsRule<D: Semantics> {
    pub meta: Meta,
    pub run: fn(&D, &mut Vec<Finding>),
}

impl<D: Semantics> Clone for SemanticsRule<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Semantics> Copy for SemanticsRule<D> {}

/// Eine Regel auf [`Tier::Rendering`] — braucht berechnete Stile und Geometrie.
pub struct RenderingRule<D: Rendering> {
    pub meta: Meta,
    pub run: fn(&D, &mut Vec<Finding>),
}

impl<D: Rendering> Clone for RenderingRule<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Rendering> Copy for RenderingRule<D> {}
