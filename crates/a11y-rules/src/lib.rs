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
//! [`a11y_dom::Semantics`] erzeugt keinen stillen Nicht-Befund,
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
//! assert_eq!(report.summary.rules_not_run, 31);
//! ```
//!
//! Mit einem Host, der [`a11y_dom::Semantics`] erfüllt, laufen die
//! über [`run_with_semantics`] mit.

#![forbid(unsafe_code)]

mod aria;
mod checkliste;
mod forms;
mod heuristik;
mod locale;
mod names;
mod registry;
mod rendering;
mod semantics;
mod sicht;
mod structure;

pub use locale::Locale;
use locale::pick;
pub use registry::{Meta, RenderingRule, SemanticsRule, StructureRule};
pub use sicht::Scope;
use sicht::{Sicht, Verborgen};

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
fn nicht_gelaufen(metas: &'static [Meta], grund: &str, report: &mut Report) {
    for meta in metas {
        for id in meta.ids {
            report.record(RuleRun::not_run(*id, NotRun::CapabilityMissing).with_reason(grund));
        }
    }
}

fn ohne_semantik(locale: Locale) -> &'static str {
    pick!(
        locale,
        "host provides no role and no accessible name",
        "Host liefert keine Rolle und keinen Accessible Name",
    )
}

fn ohne_darstellung(locale: Locale) -> &'static str {
    pick!(
        locale,
        "host provides no computed styles and no geometry",
        "Host liefert keine berechneten Stile und keine Geometrie",
    )
}

fn run_structure<D: Document>(doc: &D, v: &Verborgen, locale: Locale, report: &mut Report) {
    for rule in structure_rules::<Sicht<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Sicht::new(doc, v, rule.meta.scope), locale, &mut out);
        vermerke(&rule.meta, &out, report);
        report.extend(out);
    }
}

/// Prüft ein Dokument, das nur Struktur liefert.
///
/// Tier-2-Regeln werden mit `NotRun::CapabilityMissing` vermerkt, nicht
/// übergangen — der Bericht sagt damit aus, was er *nicht* geprüft hat.
pub fn run<D: Document>(doc: &D) -> Report {
    run_in(doc, Locale::En)
}

/// Wie [`run`], mit Befundtexten in der gewählten Sprache.
pub fn run_in<D: Document>(doc: &D, locale: Locale) -> Report {
    let v = Verborgen::nach_attribut(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    nicht_gelaufen(semantics_metas(), ohne_semantik(locale), &mut report);
    nicht_gelaufen(rendering_metas(), ohne_darstellung(locale), &mut report);
    report.finish()
}

/// Prüft ein Dokument, das zusätzlich Rolle und Accessible Name liefert.
pub fn run_with_semantics<D: Semantics>(doc: &D) -> Report {
    run_with_semantics_in(doc, Locale::En)
}

/// Wie [`run_with_semantics`], mit Befundtexten in der gewählten Sprache.
pub fn run_with_semantics_in<D: Semantics>(doc: &D, locale: Locale) -> Report {
    let v = Verborgen::nach_attribut(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    run_semantics(doc, &v, locale, &mut report);
    nicht_gelaufen(rendering_metas(), ohne_darstellung(locale), &mut report);
    report.finish()
}

fn run_semantics<D: Semantics>(doc: &D, v: &Verborgen, locale: Locale, report: &mut Report) {
    for rule in semantics_rules::<Sicht<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Sicht::new(doc, v, rule.meta.scope), locale, &mut out);
        vermerke(&rule.meta, &out, report);
        report.extend(out);
    }
}

fn run_rendering<D: Rendering>(doc: &D, v: &Verborgen, locale: Locale, report: &mut Report) {
    for rule in rendering_rules::<Sicht<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Sicht::new(doc, v, rule.meta.scope), locale, &mut out);
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
    run_full_in(doc, Locale::En)
}

/// Wie [`run_full`], mit Befundtexten in der gewählten Sprache.
pub fn run_full_in<D: Semantics + Rendering>(doc: &D, locale: Locale) -> Report {
    let v = Verborgen::nach_stil(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    run_semantics(doc, &v, locale, &mut report);
    run_rendering(doc, &v, locale, &mut report);
    report.finish()
}

/// Prüft ein Dokument, das Struktur und Darstellung liefert, aber keine
/// Semantik. Selten — aufgeführt, damit die Tier-Kombination nicht durch das
/// Raster fällt.
pub fn run_with_rendering<D: Rendering>(doc: &D) -> Report {
    run_with_rendering_in(doc, Locale::En)
}

/// Wie [`run_with_rendering`], mit Befundtexten in der gewählten Sprache.
pub fn run_with_rendering_in<D: Rendering>(doc: &D, locale: Locale) -> Report {
    let v = Verborgen::nach_stil(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    nicht_gelaufen(semantics_metas(), ohne_semantik(locale), &mut report);
    run_rendering(doc, &v, locale, &mut report);
    report.finish()
}
