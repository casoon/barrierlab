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
//! // Nicht beurteilt: die Tier-2- und Tier-3-Regeln und die über
//! // Stylesheets, weil dieser Host weder Semantik noch Darstellung noch
//! // Stylesheets liefert. Sie fehlen nicht im Bericht, sie stehen mit
//! // `NotRun::CapabilityMissing` darin.
//! assert_eq!(report.summary.rules_not_run, 59);
//! ```
//!
//! Mit einem Host, der [`a11y_dom::Semantics`] erfüllt, laufen die
//! über [`run_with_semantics`] mit.

#![forbid(unsafe_code)]

mod aria;
mod best_practice;
mod catalogue;
mod checklist;
mod document;
mod forms;
mod heuristics;
mod landmarks;
mod links;
mod locale;
mod media;
mod names;
mod not_run;
mod presentation;
mod probes;
mod registry;
mod rendering;
mod roles;
mod semantics;
mod structure;
mod stylesheet_rules;
mod tables;
mod visibility;
mod viz;

pub use best_practice::{BEST_PRACTICE_RULES, BestPracticeRule, best_practice_rule};
pub use catalogue::{catalogue, meta_for};
pub use locale::Locale;
use locale::pick;
pub use registry::{Meta, RenderingRule, SemanticsRule, StructureRule, StylesheetRule};
pub use visibility::Scope;
use visibility::{Hidden, Visibility};

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

/// Siehe [`structure_metas`]. Die Regeln über Stylesheets laufen über
/// [`run_stylesheets`].
pub fn stylesheet_metas() -> &'static [Meta] {
    stylesheet_rules::METAS
}

/// Alle Regeln über Stylesheets.
pub fn stylesheet_rules<D: Document>() -> Vec<StylesheetRule<D>> {
    stylesheet_rules::rules()
}

/// Vermerkt je deklarierter Kennung, wie viele Befunde darauf entfallen.
///
/// Der Vermerk läuft über die **Befund**-Kennungen, nicht über eine
/// übergeordnete Regelkennung. Nur so benutzen `rule_runs` und `findings`
/// dieselbe Namensmenge und lassen sich verbinden.
fn record_runs(meta: &Meta, found: Vec<Finding>, report: &mut Report) {
    // Eine Regel, der die Messung des Hosts fehlt, meldet das als Vermerk
    // (`not_run::note`): kein Befund, sondern `NotRun::CapabilityMissing`.
    let (notes, found): (Vec<Finding>, Vec<Finding>) =
        found.into_iter().partition(not_run::is_note);
    for id in meta.ids {
        let run = match notes.iter().find(|f| f.rule_id == *id) {
            Some(note) => {
                RuleRun::not_run(*id, NotRun::CapabilityMissing).with_reason(note.message.clone())
            }
            None => RuleRun::ran(*id, found.iter().filter(|f| f.rule_id == *id).count()),
        };
        report.record(run.with_wcag(meta.wcag.iter().copied()));
    }
    report.extend(found);
}

/// Vermerkt jede Kennung der übergebenen Deklarationen als nicht gelaufen.
///
/// Das ist die Umsetzung von „nicht geprüft ist nicht bestanden": Eine Regel,
/// deren Tier dieser Host nicht bedient, verschwindet nicht aus dem Bericht,
/// sondern steht mit `NotRun::CapabilityMissing` darin.
fn record_not_run(metas: &'static [Meta], reason: &str, report: &mut Report) {
    for meta in metas {
        for id in meta.ids {
            report.record(
                RuleRun::not_run(*id, NotRun::CapabilityMissing)
                    .with_wcag(meta.wcag.iter().copied())
                    .with_reason(reason),
            );
        }
    }
}

fn without_semantics_reason(locale: Locale) -> &'static str {
    pick!(
        locale,
        "host provides no role and no accessible name",
        "Host liefert keine Rolle und keinen Accessible Name",
    )
}

fn without_presentation_reason(locale: Locale) -> &'static str {
    pick!(
        locale,
        "host provides no computed styles and no geometry",
        "Host liefert keine berechneten Stile und keine Geometrie",
    )
}

fn without_stylesheets_reason(locale: Locale) -> &'static str {
    pick!(
        locale,
        "host provides no stylesheets",
        "Host liefert keine Stylesheets",
    )
}

fn run_structure<D: Document>(doc: &D, v: &Hidden, locale: Locale, report: &mut Report) {
    for rule in structure_rules::<Visibility<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Visibility::new(doc, v, rule.meta.scope), locale, &mut out);
        record_runs(&rule.meta, out, report);
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
    let v = Hidden::by_attribute(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    record_not_run(
        semantics_metas(),
        without_semantics_reason(locale),
        &mut report,
    );
    record_not_run(
        rendering_metas(),
        without_presentation_reason(locale),
        &mut report,
    );
    record_not_run(
        stylesheet_metas(),
        without_stylesheets_reason(locale),
        &mut report,
    );
    report.finish()
}

/// Prüft ein Dokument, das zusätzlich Rolle und Accessible Name liefert.
pub fn run_with_semantics<D: Semantics>(doc: &D) -> Report {
    run_with_semantics_in(doc, Locale::En)
}

/// Wie [`run_with_semantics`], mit Befundtexten in der gewählten Sprache.
pub fn run_with_semantics_in<D: Semantics>(doc: &D, locale: Locale) -> Report {
    let v = Hidden::by_attribute(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    run_semantics(doc, &v, locale, &mut report);
    record_not_run(
        rendering_metas(),
        without_presentation_reason(locale),
        &mut report,
    );
    record_not_run(
        stylesheet_metas(),
        without_stylesheets_reason(locale),
        &mut report,
    );
    report.finish()
}

fn run_semantics<D: Semantics>(doc: &D, v: &Hidden, locale: Locale, report: &mut Report) {
    for rule in semantics_rules::<Visibility<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Visibility::new(doc, v, rule.meta.scope), locale, &mut out);
        record_runs(&rule.meta, out, report);
    }
}

fn run_rendering<D: Rendering>(doc: &D, v: &Hidden, locale: Locale, report: &mut Report) {
    for rule in rendering_rules::<Visibility<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(&Visibility::new(doc, v, rule.meta.scope), locale, &mut out);
        record_runs(&rule.meta, out, report);
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
    let v = Hidden::by_style(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    run_semantics(doc, &v, locale, &mut report);
    run_rendering(doc, &v, locale, &mut report);
    record_not_run(
        stylesheet_metas(),
        without_stylesheets_reason(locale),
        &mut report,
    );
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
    let v = Hidden::by_style(doc);
    let mut report = Report::new();
    run_structure(doc, &v, locale, &mut report);
    record_not_run(
        semantics_metas(),
        without_semantics_reason(locale),
        &mut report,
    );
    run_rendering(doc, &v, locale, &mut report);
    record_not_run(
        stylesheet_metas(),
        without_stylesheets_reason(locale),
        &mut report,
    );
    report.finish()
}

/// Ergänzt einen Bericht um die Regeln über Stylesheets.
///
/// Jedes `run_*` vermerkt diese Regeln zunächst als nicht gelaufen — ein
/// Host, der keine Stylesheets liefert, sagt damit, was er nicht geprüft hat.
/// Wer sie hat, reicht den Bericht hier durch: Die Vermerke werden durch die
/// Läufe ersetzt, die Befunde kommen hinzu. Die Stylesheets parst der Host
/// mit [`stylesheet_parse::parse_stylesheet`]; Reihenfolge wie im Dokument.
///
/// ```
/// use a11y_dom::Arena;
///
/// let doc = Arena::builder().open("html").open("body").close().close().build();
/// let sheets = [stylesheet_parse::parse_stylesheet("a:focus { outline: none }")];
/// let report = a11y_rules::run_stylesheets(a11y_rules::run(&doc), &doc, &sheets);
/// assert!(report.findings.iter().any(|f| f.rule_id == "focus/outline-removed"));
/// ```
pub fn run_stylesheets<D: Document>(
    report: Report,
    doc: &D,
    sheets: &[stylesheet_parse::Stylesheet],
) -> Report {
    run_stylesheets_in(report, doc, sheets, Locale::En)
}

/// Wie [`run_stylesheets`], mit Befundtexten in der gewählten Sprache.
pub fn run_stylesheets_in<D: Document>(
    mut report: Report,
    doc: &D,
    sheets: &[stylesheet_parse::Stylesheet],
    locale: Locale,
) -> Report {
    let ids: Vec<&str> = stylesheet_metas()
        .iter()
        .flat_map(|m| m.ids.iter().copied())
        .collect();
    report
        .rule_runs
        .retain(|r| !ids.contains(&r.rule_id.as_str()));
    let v = Hidden::by_attribute(doc);
    for rule in stylesheet_rules::<Visibility<'_, D>>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(
            &Visibility::new(doc, &v, rule.meta.scope),
            sheets,
            locale,
            &mut out,
        );
        record_runs(&rule.meta, out, &mut report);
    }
    report.finish()
}
