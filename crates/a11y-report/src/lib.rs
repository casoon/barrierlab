//! Gemeinsames Befund- und Berichtsmodell für Accessibility-Werkzeuge.
//!
//! Ein Regelbestand bedient drei Oberflächen — Build-Zeit, CI/Crawl und die
//! laufende Seite — mit identischen Regelkennungen und identischem JSON.
//! Dieses Crate definiert, was dabei herauskommt.
//!
//! # Zwei Achsen, nicht drei
//!
//! [`Outcome`] sagt, *wie sicher* die Aussage ist; [`Severity`] sagt, *wie
//! schwer* das Problem wiegt. Eine dritte Achse „certainty" gibt es bewusst
//! nicht — eine nur heuristisch belegbare Regel liefert [`Outcome::Review`],
//! nicht [`Outcome::Fail`] mit niedriger Gewissheit.
//!
//! ```
//! use a11y_report::{Finding, Location, Report, Severity};
//!
//! let mut report = Report::new();
//!
//! // Eindeutig: das Attribut fehlt.
//! report.push(
//!     Finding::fail("images/alt-missing", "Das Bild besitzt kein alt-Attribut.")
//!         .with_severity(Severity::High)
//!         .with_wcag(["1.1.1"])
//!         .at(Location::file("about/index.html").with_selector("main > img:nth-child(2)")),
//! );
//!
//! // Heuristisch: der Alt-Text ist da, aber womöglich nichtssagend.
//! report.push(
//!     Finding::review("images/alt-suspicious", "Der Alt-Text sieht nach einem Dateinamen aus.")
//!         .with_severity(Severity::Medium)
//!         .with_wcag(["1.1.1"]),
//! );
//!
//! let report = report.finish();
//! assert_eq!(report.summary.fail, 1);
//! assert_eq!(report.summary.review, 1);
//! assert_eq!(report.summary.problems(), 2);
//! ```
//!
//! # Regeln, die nicht laufen konnten
//!
//! Der Unterschied zwischen „lief und fand nichts" und „konnte nicht laufen"
//! wird über [`RuleRun`] festgehalten. Ohne ihn liest sich eine Kontrastprüfung
//! ohne Rendering-Zugriff wie eine bestandene Prüfung.
//!
//! ```
//! use a11y_report::{NotRun, Report, RuleRun};
//!
//! let mut report = Report::new();
//! report.record(
//!     RuleRun::not_run("contrast/text", NotRun::CapabilityMissing)
//!         .with_reason("statische Analyse liefert keine Rendering-Werte"),
//! );
//! let report = report.finish();
//! assert_eq!(report.summary.rules_not_run, 1);
//! ```

#![forbid(unsafe_code)]

mod extra;
mod finding;
mod outcome;
mod report;

pub use extra::Extra;
pub use finding::{Evidence, Finding, Location};
pub use outcome::{Outcome, Severity, WcagLevel};
pub use report::{NotRun, Report, RuleRun, Summary};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaehlt_zustaende_und_schweregrade_getrennt() {
        let mut r = Report::new();
        r.push(Finding::fail("a", "x").with_severity(Severity::Critical));
        r.push(Finding::review("b", "x").with_severity(Severity::Low));
        r.push(Finding::pass("c", "x").with_severity(Severity::Critical));
        r.push(Finding::untested("d", "x").with_severity(Severity::High));
        let r = r.finish();

        assert_eq!(
            (
                r.summary.fail,
                r.summary.review,
                r.summary.pass,
                r.summary.untested
            ),
            (1, 1, 1, 1)
        );
        // Schweregrade zaehlen nur ueber Probleme: das Critical des Pass und das
        // High des Untested bleiben aussen vor.
        assert_eq!(
            (r.summary.critical, r.summary.high, r.summary.low),
            (1, 0, 1)
        );
        assert_eq!(r.summary.problems(), 2);
    }

    #[test]
    fn pass_wird_standardmaessig_nicht_angezeigt() {
        let mut r = Report::new();
        r.push(Finding::fail("a", "x"));
        r.push(Finding::pass("b", "x"));
        r.push(Finding::untested("c", "x"));
        assert_eq!(r.visible().count(), 2);
        assert_eq!(r.by_outcome(Outcome::Pass).count(), 1);
    }

    #[test]
    fn untested_ist_kein_problem() {
        assert!(Outcome::Fail.is_problem());
        assert!(Outcome::Review.is_problem());
        assert!(!Outcome::Pass.is_problem());
        assert!(!Outcome::Untested.is_problem());
    }

    #[test]
    fn json_bleibt_schlank_wenn_nichts_gesetzt_ist() {
        let f = Finding::fail("images/alt-missing", "kein alt");
        let j: serde_json::Value = serde_json::to_value(&f).unwrap();
        let obj = j.as_object().unwrap();
        // Nur die vier Pflichtfelder landen im JSON.
        assert_eq!(
            obj.len(),
            4,
            "unerwartete Felder: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
        assert_eq!(obj["outcome"], "fail");
        assert_eq!(obj["severity"], "medium");
    }

    #[test]
    fn json_rundlauf_erhaelt_alles() {
        let f = Finding::review("aria/role-invalid", "unbekannte Rolle")
            .with_severity(Severity::High)
            .with_wcag(["4.1.2"])
            .with_wcag_level(WcagLevel::A)
            .with_tags(["best-practice"])
            .with_help("Nur Rollen aus der ARIA-Spezifikation verwenden.")
            .with_help_url("https://www.w3.org/TR/wai-aria-1.2/#roles")
            .with_suggestion("role=\"buton\" ist vermutlich role=\"button\".")
            .with_suggested_code("<div role=\"button\">")
            .with_snippet("<div role=\"buton\">")
            .at(Location::node("42").with_selector("#menu > div"))
            .add_evidence(Evidence::dom_attribute("role", Some("buton".into())));

        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn wcag_stufen_schliessen_ein() {
        assert!(WcagLevel::A.included_in(WcagLevel::AA));
        assert!(WcagLevel::AA.included_in(WcagLevel::AA));
        assert!(!WcagLevel::AAA.included_in(WcagLevel::AA));
        assert!(WcagLevel::AAA.included_in(WcagLevel::AAA));
    }

    #[test]
    fn severity_ordnet_sich_aufsteigend() {
        let mut v = vec![
            Severity::High,
            Severity::Low,
            Severity::Critical,
            Severity::Medium,
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                Severity::Low,
                Severity::Medium,
                Severity::High,
                Severity::Critical
            ]
        );
    }

    #[test]
    fn axe_impact_bleibt_kompatibel() {
        assert_eq!(Severity::Low.as_axe_impact(), "minor");
        assert_eq!(Severity::Medium.as_axe_impact(), "moderate");
        assert_eq!(Severity::High.as_axe_impact(), "serious");
        assert_eq!(Severity::Critical.as_axe_impact(), "critical");
    }

    #[test]
    fn nicht_gelaufene_regeln_werden_gezaehlt() {
        let mut r = Report::new();
        r.record(RuleRun::ran("images/alt-missing", 3));
        r.record(RuleRun::not_run("contrast/text", NotRun::CapabilityMissing));
        r.record(RuleRun::not_run("forms/label", NotRun::NotApplicable));
        let r = r.finish();
        assert_eq!(r.summary.rules_not_run, 2);
        assert!(r.rule_runs[0].did_run());
        assert!(!r.rule_runs[1].did_run());
    }

    // --- Erweiterungsslot ------------------------------------------------

    #[derive(Debug)]
    struct Screenshot(Vec<u8>);

    #[test]
    fn beigabe_kommt_typisiert_zurueck() {
        let f =
            Finding::fail("images/alt-missing", "kein alt").with_extra(Screenshot(vec![1, 2, 3]));
        assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(3));
        // Eine andere Sorte liegt nicht darin -- kein falscher Treffer.
        assert!(f.extra.get::<String>().is_none());
    }

    #[test]
    fn beigabe_landet_nicht_im_json() {
        let f = Finding::fail("a", "x").with_extra(Screenshot(vec![0; 4096]));
        let j = serde_json::to_string(&f).unwrap();
        assert!(
            !j.contains("extra"),
            "Beigabe darf nicht serialisiert werden: {j}"
        );
        // Und sie blaeht den Vertrag nicht auf.
        assert!(j.len() < 200, "{} Bytes", j.len());
    }

    #[test]
    fn beigabe_zaehlt_nicht_zur_identitaet() {
        // Sonst waeren zwei inhaltlich gleiche Befunde ungleich, nur weil an
        // einem ein Screenshot haengt.
        let ohne = Finding::fail("a", "x");
        let mit = Finding::fail("a", "x").with_extra(Screenshot(vec![9]));
        assert_eq!(ohne, mit);
    }

    #[test]
    fn beigabe_ueberlebt_das_klonen() {
        let f = Finding::fail("a", "x").with_extra(Screenshot(vec![7, 7]));
        let k = f.clone();
        assert_eq!(
            k.extra.get::<Screenshot>().map(|s| s.0.clone()),
            Some(vec![7, 7])
        );
    }

    // --- Rolle, Name, Regelname ------------------------------------------

    #[test]
    fn element_und_regelname_gehen_durch_das_json() {
        let f = Finding::fail("buttons/name-missing", "kein Name")
            .with_rule_name("Button braucht einen zugänglichen Namen")
            .with_element(Some("button".into()), None);

        let j: serde_json::Value = serde_json::to_value(&f).unwrap();
        assert_eq!(j["role"], "button");
        assert_eq!(j["rule_name"], "Button braucht einen zugänglichen Namen");
        // Ein fehlender Name ist kein leerer Name -- das Feld faellt weg.
        assert!(j.get("name").is_none());

        let zurueck: Finding = serde_json::from_value(j).unwrap();
        assert_eq!(zurueck, f);
    }

    #[test]
    fn der_schlanke_fall_bleibt_schlank() {
        // Die neuen Felder duerfen den Vertrag nicht aufblaehen, wenn sie
        // ungesetzt sind.
        let f = Finding::fail("images/alt-missing", "kein alt");
        let j: serde_json::Value = serde_json::to_value(&f).unwrap();
        assert_eq!(
            j.as_object().unwrap().len(),
            4,
            "unerwartete Felder: {:?}",
            j.as_object().unwrap().keys().collect::<Vec<_>>()
        );
    }

    // --- Vermerke je Durchgang -------------------------------------------

    #[test]
    fn derselbe_regelvermerk_kann_je_durchgang_vorkommen() {
        // Wer dieselbe Seite unter mehreren Bedingungen prueft, fuehrt je
        // Durchgang einen Vermerk. Der Schluessel ist dann (rule_id, viewport).
        let mut r = Report::new();
        r.record(RuleRun::ran("target-size/minimum", 0).in_viewport("desktop"));
        r.record(RuleRun::ran("target-size/minimum", 3).in_viewport("mobile"));
        let r = r.finish();

        assert_eq!(r.rule_runs.len(), 2);
        let mobil = r
            .rule_runs
            .iter()
            .find(|x| x.viewport.as_deref() == Some("mobile"))
            .unwrap();
        assert_eq!(mobil.findings, 3);
    }

    #[test]
    fn ein_nicht_gelaufener_vermerk_nennt_das_ungeprueffte_kriterium() {
        let mut r = Report::new();
        r.record(
            RuleRun::not_run("contrast/text", NotRun::CapabilityMissing)
                .with_wcag(["1.4.3"])
                .with_reason("statische Analyse liefert keine Rendering-Werte"),
        );
        let r = r.finish();
        assert_eq!(r.summary.rules_not_run, 1);
        assert_eq!(r.rule_runs[0].wcag, vec!["1.4.3"]);
    }
}
