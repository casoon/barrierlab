//! Regelkorpus: direkte Funktionstests für jede Befund-Kennung
//! (casoon/auditmysite#783).
//!
//! Jede Kennung aus `Meta::ids` aller vier Registraturen braucht hier drei
//! Arten von Fällen:
//!
//! - **positiv** — die Kennung wird gemeldet, mit genau dem erwarteten
//!   Ergebnis (`Fail`, `Review` oder `Untested`). Ein `Fail` erfüllt keinen
//!   `Review`-Fall und umgekehrt.
//! - **negativ** — ein ähnlich aussehendes, konformes Dokument; die Kennung
//!   lief nachweislich und meldet nichts.
//! - **nicht anwendbar** — nichts im Dokument, worauf die Regel zielt; die
//!   Kennung lief nachweislich und meldet nichts.
//!
//! Dazu Grenzfälle auf beiden Seiten einer Schwelle und Varianten (versteckt,
//! nativ/ARIA, andere Beschriftungswege, Shadow DOM), wo die Regel sie kennt.
//! Die Erwartungen stammen aus der Norm (WCAG 2.2 mit Understanding und
//! Techniken, WAI-ARIA 1.2, HTML-AAM, accname 1.2, HTML), nicht aus der
//! aktuellen Ausgabe der Regeln; jeder Fall nennt seine Quelle.
//!
//! `gate_every_id_has_all_three_kinds` lässt den Lauf scheitern, sobald
//! eine Kennung eine der drei Arten nicht hat — auch eine neu hinzugekommene.
//!
//! `KORPUS_ID=<kennung>[,<kennung>…]` beschränkt `korpus_faelle` auf die
//! Fälle dieser Kennungen; der Mutationslauf (`xtask/mutate.py`) benutzt das.

mod korpus {
    pub mod catalog;
    pub mod faelle;
    pub mod host;
}

use std::collections::{BTreeMap, BTreeSet};

use a11y_report::{Outcome, Report};

use korpus::faelle::{Fall, Kind, Mode, all};
use korpus::host::Host;

/// Alle deklarierten Kennungen, aus den Registraturen — nicht aus dem Text.
fn all_ids() -> BTreeSet<&'static str> {
    a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas())
        .chain(a11y_rules::stylesheet_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect()
}

/// Führt einen Fall so aus, wie ein Host es täte, der alle Tiers liefert.
fn report(fall: &Fall) -> Report {
    let doc = korpus::host::document(fall.html);
    let host = Host::new(&doc);
    let sheets = host.stylesheets();
    match fall.mode {
        Mode::Full => a11y_rules::run_stylesheets(a11y_rules::run_full(&host), &host, &sheets),
        Mode::Semantics => {
            a11y_rules::run_stylesheets(a11y_rules::run_with_semantics(&host), &host, &sheets)
        }
    }
}

/// Prüft einen Fall; `Err` mit einer lesbaren Begründung, wenn er scheitert.
fn check(fall: &Fall) -> Result<(), String> {
    let r = report(fall);
    let rule_run = r.rule_runs.iter().find(|l| l.rule_id == fall.id);
    let Some(rule_run) = rule_run else {
        return Err("kein RuleRun für die Kennung".into());
    };
    let verdicts: Vec<Outcome> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == fall.id)
        .map(|f| f.outcome)
        .collect();
    match fall.kind {
        Kind::Positive(expects) => {
            if verdicts.is_empty() {
                return Err(format!("erwartet {expects:?}, nichts gemeldet"));
            }
            if verdicts.iter().any(|u| *u != expects) {
                return Err(format!("erwartet nur {expects:?}, gemeldet {verdicts:?}"));
            }
            if let Some(n) = fall.count
                && verdicts.len() != n
            {
                return Err(format!("erwartet {n} Befunde, gemeldet {}", verdicts.len()));
            }
            Ok(())
        }
        Kind::Negative | Kind::NotApplicable => {
            if rule_run.not_run.is_some() {
                return Err(format!("Regel lief nicht: {:?}", rule_run.not_run));
            }
            if !verdicts.is_empty() {
                return Err(format!("nichts erwartet, gemeldet {verdicts:?}"));
            }
            Ok(())
        }
    }
}

#[test]
fn korpus_faelle() {
    let only = std::env::var("KORPUS_ID").ok();
    let selection: Option<Vec<&str>> = only.as_deref().map(|s| s.split(',').collect());
    let mut error = Vec::new();
    let mut run = 0;
    for fall in all() {
        if selection
            .as_ref()
            .is_some_and(|ids| !ids.contains(&fall.id))
        {
            continue;
        }
        run += 1;
        if let Err(reason) = check(&fall) {
            error.push(format!(
                "{} [{}{}]: {reason}\n    Quelle: {}",
                fall.id,
                fall.kind.name(),
                fall.variant.map(|v| format!(", {v}")).unwrap_or_default(),
                fall.source,
            ));
        }
    }
    assert!(run > 0, "keine Fälle gelaufen (KORPUS_ID = {only:?})");
    assert!(
        error.is_empty(),
        "{} von {run} Fällen scheitern:\n{}",
        error.len(),
        error.join("\n")
    );
}

#[test]
fn gate_every_id_has_all_three_kinds() {
    let ids = all_ids();
    let mut kinds: BTreeMap<&str, [usize; 3]> = ids.iter().map(|id| (*id, [0; 3])).collect();
    let mut unknown = BTreeSet::new();
    for fall in all() {
        assert!(
            !fall.source.trim().is_empty(),
            "{}: Fall ohne Quelle",
            fall.id
        );
        match kinds.get_mut(fall.id) {
            Some(z) => z[fall.kind.index()] += 1,
            None => {
                unknown.insert(fall.id);
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "Fälle für nicht deklarierte Kennungen: {unknown:?}"
    );
    let gaps: Vec<String> = kinds
        .iter()
        .filter(|(_, z)| z.contains(&0))
        .map(|(id, z)| format!("{id}: positiv {}, negativ {}, n/a {}", z[0], z[1], z[2]))
        .collect();
    assert!(
        gaps.is_empty(),
        "{} von {} Kennungen ohne alle drei Fallarten:\n{}",
        gaps.len(),
        ids.len(),
        gaps.join("\n")
    );
}

#[test]
fn gate_mutation_catalog_covers_every_id() {
    let ids = all_ids();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let entries = korpus::catalog::load(&root.join("tests/mutations"));
    let mut covered = BTreeSet::new();
    let mut error = Vec::new();
    for e in &entries {
        if !ids.contains(e.id.as_str()) {
            error.push(format!("{}: Kennung nicht deklariert", e.id));
        }
        covered.insert(e.id.clone());
        let path = root.join(&e.file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            error.push(format!("{}: Datei {} fehlt", e.id, e.file));
            continue;
        };
        let n = text.matches(e.find.as_str()).count();
        if n != 1 {
            error.push(format!(
                "{}: `find` kommt {n}-mal in {} vor, erwartet genau einmal",
                e.id, e.file
            ));
        }
        if e.find == e.replace {
            error.push(format!("{}: `find` und `replace` sind gleich", e.id));
        }
        if !matches!(e.expect.as_str(), "killed" | "equivalent") {
            error.push(format!("{}: expect = {:?}", e.id, e.expect));
        }
        if e.expect == "equivalent" && e.note.trim().is_empty() {
            error.push(format!("{}: äquivalente Mutation ohne Begründung", e.id));
        }
    }
    for id in &ids {
        if !covered.contains(*id) {
            error.push(format!("{id}: keine Mutation im Katalog"));
        } else if !entries.iter().any(|e| e.id == *id && e.expect == "killed") {
            error.push(format!("{id}: keine Mutation, die ein Test töten muss"));
        }
    }
    assert!(
        error.is_empty(),
        "Mutationskatalog unvollständig:\n{}",
        error.join("\n")
    );
}
