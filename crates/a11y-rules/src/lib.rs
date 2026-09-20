//! Accessibility-Regeln, generisch über das Dokumentmodell.
//!
//! Ein Regelbestand, drei Oberflächen: Build-Zeit, CI/Crawl und die laufende
//! Seite. Welche Regeln laufen können, hängt davon ab, welche
//! [`Tier`](a11y_dom::Tier)s der Host bedient.
//!
//! # Nicht gelaufen ist nicht bestanden
//!
//! [`run`] läuft mit dem, was da ist, und hält für jede Regel fest, ob sie
//! laufen konnte. Eine Tier-2-Regel auf einem Host ohne
//! [`Semantics`](a11y_dom::Semantics) erzeugt keinen stillen Nicht-Befund,
//! sondern einen Vermerk mit `NotRun::CapabilityMissing`.
//!
//! ```
//! use a11y_dom::Arena;
//! use a11y_rules::run;
//!
//! let doc = Arena::builder()
//!     .open("html")
//!         .open("body")
//!             .open("img").attr("src", "logo.png").close()
//!         .close()
//!     .close()
//!     .build();
//!
//! let report = run(&doc);
//!
//! // Gefunden: kein lang, kein title, kein alt.
//! assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));
//! assert!(report.findings.iter().any(|f| f.rule_id == "document/lang-missing"));
//!
//! // Nicht beurteilt: die Tier-2- und Tier-3-Regeln, weil dieser Host weder
//! // Semantik noch Darstellung liefert. Sie fehlen nicht im Bericht, sie
//! // stehen mit `NotRun::CapabilityMissing` darin.
//! assert_eq!(report.summary.rules_not_run, 6);
//! ```
//!
//! Mit einem Host, der [`Semantics`](a11y_dom::Semantics) erfüllt, laufen die
//! über [`run_with_semantics`] mit.

#![forbid(unsafe_code)]

mod registry;
mod rendering;
mod semantics;
mod structure;

pub use registry::{Meta, RenderingRule, SemanticsRule, StructureRule};

use a11y_dom::{Document, Rendering, Semantics};
use a11y_report::{Finding, NotRun, Report, RuleRun};

/// Alle Tier-1-Regeln.
pub fn structure_rules<D: Document>() -> Vec<StructureRule<D>> {
    structure::rules()
}

/// Alle Tier-2-Regeln.
pub fn semantics_rules<D: Semantics>() -> Vec<SemanticsRule<D>> {
    semantics::rules()
}

/// Alle Tier-3-Regeln.
pub fn rendering_rules<D: Rendering>() -> Vec<RenderingRule<D>> {
    rendering::rules()
}

/// Die Deklarationen ohne Bindung an einen Host — für Werkzeuge, die den
/// Regelbestand auflisten oder Kennungen benennen müssen, ohne ihn auszuführen.
pub fn structure_metas() -> &'static [Meta] {
    structure::METAS
}

/// Siehe [`structure_metas`].
pub fn semantics_metas() -> &'static [Meta] {
    semantics::METAS
}

/// Siehe [`structure_metas`].
pub fn rendering_metas() -> &'static [Meta] {
    rendering::METAS
}

/// Vermerkt je deklarierter Kennung, wie viele Befunde darauf entfallen.
///
/// Der Vermerk läuft über die **Befund**-Kennungen, nicht über eine
/// übergeordnete Regelkennung. Nur so benutzen `rule_runs` und `findings`
/// dieselbe Namensmenge und lassen sich verbinden.
fn vermerke(meta: &Meta, gefunden: &[Finding], report: &mut Report) {
    for id in meta.ids {
        let anzahl = gefunden.iter().filter(|f| f.rule_id == *id).count();
        report.record(RuleRun::ran(*id, anzahl));
    }
}

/// Vermerkt jede Kennung der übergebenen Deklarationen als nicht gelaufen.
///
/// Das ist die Umsetzung von „nicht geprüft ist nicht bestanden": Eine Regel,
/// deren Tier dieser Host nicht bedient, verschwindet nicht aus dem Bericht,
/// sondern steht mit `NotRun::CapabilityMissing` darin.
fn nicht_gelaufen(metas: &'static [Meta], grund: &'static str, report: &mut Report) {
    for meta in metas {
        for id in meta.ids {
            report.record(RuleRun::not_run(*id, NotRun::CapabilityMissing).with_reason(grund));
        }
    }
}

const OHNE_SEMANTIK: &str = "Host liefert keine Rolle und keinen Accessible Name";
const OHNE_DARSTELLUNG: &str = "Host liefert keine berechneten Stile und keine Geometrie";

fn run_structure<D: Document>(doc: &D, report: &mut Report) {
    for rule in structure_rules::<D>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(doc, &mut out);
        vermerke(&rule.meta, &out, report);
        report.extend(out);
    }
}

/// Prüft ein Dokument, das nur Struktur liefert.
///
/// Tier-2-Regeln werden mit `NotRun::CapabilityMissing` vermerkt, nicht
/// übergangen — der Bericht sagt damit aus, was er *nicht* geprüft hat.
pub fn run<D: Document>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    nicht_gelaufen(semantics_metas(), OHNE_SEMANTIK, &mut report);
    nicht_gelaufen(rendering_metas(), OHNE_DARSTELLUNG, &mut report);
    report.finish()
}

/// Prüft ein Dokument, das zusätzlich Rolle und Accessible Name liefert.
pub fn run_with_semantics<D: Semantics>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    run_semantics(doc, &mut report);
    nicht_gelaufen(rendering_metas(), OHNE_DARSTELLUNG, &mut report);
    report.finish()
}

fn run_semantics<D: Semantics>(doc: &D, report: &mut Report) {
    for rule in semantics_rules::<D>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(doc, &mut out);
        vermerke(&rule.meta, &out, report);
        report.extend(out);
    }
}

fn run_rendering<D: Rendering>(doc: &D, report: &mut Report) {
    for rule in rendering_rules::<D>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(doc, &mut out);
        vermerke(&rule.meta, &out, report);
        report.extend(out);
    }
}

/// Prüft ein Dokument, das Struktur, Semantik **und** Darstellung liefert.
///
/// Das ist der Fall der laufenden Seite und der von Chrome getriebenen
/// Prüfung. Alle Regeln laufen; der Bericht enthält keinen
/// `CapabilityMissing`-Vermerk mehr.
pub fn run_full<D: Semantics + Rendering>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    run_semantics(doc, &mut report);
    run_rendering(doc, &mut report);
    report.finish()
}

/// Prüft ein Dokument, das Struktur und Darstellung liefert, aber keine
/// Semantik. Selten — aufgeführt, damit die Tier-Kombination nicht durch das
/// Raster fällt.
pub fn run_with_rendering<D: Rendering>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    nicht_gelaufen(semantics_metas(), OHNE_SEMANTIK, &mut report);
    run_rendering(doc, &mut report);
    report.finish()
}
