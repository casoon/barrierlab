//! Die WASM-Grenze von `html-conform`.
//!
//! Dieses Crate enthält **keine eigene Prüfung**. Es ruft `html_conform::check`
//! auf und gibt das Ergebnis als Bericht im gemeinsamen Befundmodell
//! (`a11y-report`) an JavaScript zurück. Eine Regel gehört nach `html-conform`.

#![forbid(unsafe_code)]

use a11y_report::{NotRun, Report, RuleRun};
use html_conform::CheckError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const _REPORT_TS: &str = include_str!("report.d.ts");

/// Id des Laufvermerks für ein Dokument, das nicht geprüft werden konnte.
const RULE_RUN_ID: &str = "html-conform";

/// Der Bericht zu einem Dokument.
///
/// Ein Dokument, das nicht geprüft werden konnte (zu tief verschachtelt), ergibt
/// keinen leeren Bericht, sondern einen Laufvermerk `html-conform` mit
/// `not_run: "errored"` und Grund — nie einen stillen Erfolg.
pub fn report(html: &str) -> Result<Report, String> {
    match html_conform::check(html) {
        Ok(checked) => Ok(checked.to_report()),
        Err(error @ CheckError::DocumentTooDeep { .. }) => {
            let mut report = Report::new();
            report.record(
                RuleRun::not_run(RULE_RUN_ID, NotRun::Errored).with_reason(error.to_string()),
            );
            Ok(report.finish())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Prüft ein vollständiges HTML-Dokument und liefert den `Report`.
///
/// Wirft einen `Error`, wenn die Prüfung selbst nicht aufgebaut werden konnte.
#[wasm_bindgen(skip_typescript)]
pub fn check(html: &str) -> Result<JsValue, JsValue> {
    let report = report(html).map_err(|message| JsError::new(&message))?;
    serde_wasm_bindgen::to_value(&report).map_err(JsValue::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const REPORT_TS: &str = include_str!("report.d.ts");

    fn ts_felder(name: &str) -> BTreeSet<String> {
        let kopf = format!("export interface {name} {{");
        let rest = REPORT_TS.split(&kopf).nth(1).expect("Interface fehlt");
        rest.split("\n}")
            .next()
            .unwrap()
            .lines()
            .filter_map(|z| z.trim().split(':').next())
            .filter(|f| !f.is_empty() && !f.starts_with("/**"))
            .map(|f| f.trim_end_matches('?').to_string())
            .collect()
    }

    fn json_felder(v: &serde_json::Value) -> BTreeSet<String> {
        v.as_object().unwrap().keys().cloned().collect()
    }

    fn zu_tief() -> String {
        format!("<!doctype html><title>t</title>{}", "<div>".repeat(600))
    }

    #[test]
    fn ein_gueltiges_dokument_hat_keine_befunde_und_keinen_vermerk() {
        let report =
            report(r#"<!doctype html><html lang="en"><title>t</title><p>Hello</p>"#).unwrap();
        assert!(report.findings.is_empty());
        assert!(report.rule_runs.is_empty());
        assert!(report.inputs.is_empty());
    }

    #[test]
    fn ein_zu_tiefes_dokument_ist_nicht_gelaufen_und_kein_erfolg() {
        let report = report(&zu_tief()).unwrap();
        assert!(report.findings.is_empty());
        assert_eq!(report.rule_runs.len(), 1);
        assert_eq!(report.rule_runs[0].rule_id, RULE_RUN_ID);
        assert_eq!(report.rule_runs[0].not_run, Some(NotRun::Errored));
        assert!(report.rule_runs[0].reason.is_some());
    }

    /// Die Typen driften nicht: Befund, Position und Laufvermerk tragen die
    /// Felder, die das Interface nennt.
    #[test]
    fn typescript_interface_deckt_befund_position_und_vermerk() {
        let invalid =
            report(r#"<!doctype html><html lang="en"><title>t</title><body><p bogus="1">x</p>"#)
                .unwrap();
        let json = serde_json::to_value(&invalid).unwrap();
        let ort = &json["findings"][0]["location"];
        assert_eq!(json_felder(&json["summary"]), ts_felder("Summary"));
        assert!(json_felder(ort).is_subset(&ts_felder("Location")));
        assert_eq!(json_felder(&ort["position"]), ts_felder("SourcePosition"));

        let json = serde_json::to_value(report(&zu_tief()).unwrap()).unwrap();
        assert!(json_felder(&json["rule_runs"][0]).is_subset(&ts_felder("RuleRun")));
    }
}
