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
//! `gate_jede_kennung_hat_alle_drei_arten` lässt den Lauf scheitern, sobald
//! eine Kennung eine der drei Arten nicht hat — auch eine neu hinzugekommene.
//!
//! `KORPUS_ID=<kennung>[,<kennung>…]` beschränkt `korpus_faelle` auf die
//! Fälle dieser Kennungen; der Mutationslauf (`xtask/mutate.py`) benutzt das.

mod korpus {
    pub mod faelle;
    pub mod host;
    pub mod katalog;
}

use std::collections::{BTreeMap, BTreeSet};

use a11y_report::{Outcome, Report};

use korpus::faelle::{Art, Fall, Modus, alle};
use korpus::host::Host;

/// Alle deklarierten Kennungen, aus den Registraturen — nicht aus dem Text.
fn kennungen() -> BTreeSet<&'static str> {
    a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas())
        .chain(a11y_rules::stylesheet_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect()
}

/// Führt einen Fall so aus, wie ein Host es täte, der alle Tiers liefert.
fn bericht(fall: &Fall) -> Report {
    let doc = korpus::host::dokument(fall.html);
    let host = Host::new(&doc);
    let sheets = host.stylesheets();
    match fall.modus {
        Modus::Voll => a11y_rules::run_stylesheets(a11y_rules::run_full(&host), &host, &sheets),
        Modus::Semantik => {
            a11y_rules::run_stylesheets(a11y_rules::run_with_semantics(&host), &host, &sheets)
        }
    }
}

/// Prüft einen Fall; `Err` mit einer lesbaren Begründung, wenn er scheitert.
fn pruefe(fall: &Fall) -> Result<(), String> {
    let r = bericht(fall);
    let lauf = r.rule_runs.iter().find(|l| l.rule_id == fall.id);
    let Some(lauf) = lauf else {
        return Err("kein RuleRun für die Kennung".into());
    };
    let urteile: Vec<Outcome> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == fall.id)
        .map(|f| f.outcome)
        .collect();
    match fall.art {
        Art::Positiv(erwartet) => {
            if urteile.is_empty() {
                return Err(format!("erwartet {erwartet:?}, nichts gemeldet"));
            }
            if urteile.iter().any(|u| *u != erwartet) {
                return Err(format!("erwartet nur {erwartet:?}, gemeldet {urteile:?}"));
            }
            if let Some(n) = fall.anzahl
                && urteile.len() != n
            {
                return Err(format!("erwartet {n} Befunde, gemeldet {}", urteile.len()));
            }
            Ok(())
        }
        Art::Negativ | Art::NichtAnwendbar => {
            if lauf.not_run.is_some() {
                return Err(format!("Regel lief nicht: {:?}", lauf.not_run));
            }
            if !urteile.is_empty() {
                return Err(format!("nichts erwartet, gemeldet {urteile:?}"));
            }
            Ok(())
        }
    }
}

#[test]
fn korpus_faelle() {
    let nur = std::env::var("KORPUS_ID").ok();
    let auswahl: Option<Vec<&str>> = nur.as_deref().map(|s| s.split(',').collect());
    let mut fehler = Vec::new();
    let mut gelaufen = 0;
    for fall in alle() {
        if auswahl.as_ref().is_some_and(|ids| !ids.contains(&fall.id)) {
            continue;
        }
        gelaufen += 1;
        if let Err(grund) = pruefe(&fall) {
            fehler.push(format!(
                "{} [{}{}]: {grund}\n    Quelle: {}",
                fall.id,
                fall.art.name(),
                fall.variante.map(|v| format!(", {v}")).unwrap_or_default(),
                fall.quelle,
            ));
        }
    }
    assert!(gelaufen > 0, "keine Fälle gelaufen (KORPUS_ID = {nur:?})");
    assert!(
        fehler.is_empty(),
        "{} von {gelaufen} Fällen scheitern:\n{}",
        fehler.len(),
        fehler.join("\n")
    );
}

#[test]
fn gate_jede_kennung_hat_alle_drei_arten() {
    let ids = kennungen();
    let mut arten: BTreeMap<&str, [usize; 3]> = ids.iter().map(|id| (*id, [0; 3])).collect();
    let mut unbekannt = BTreeSet::new();
    for fall in alle() {
        assert!(
            !fall.quelle.trim().is_empty(),
            "{}: Fall ohne Quelle",
            fall.id
        );
        match arten.get_mut(fall.id) {
            Some(z) => z[fall.art.index()] += 1,
            None => {
                unbekannt.insert(fall.id);
            }
        }
    }
    assert!(
        unbekannt.is_empty(),
        "Fälle für nicht deklarierte Kennungen: {unbekannt:?}"
    );
    let luecken: Vec<String> = arten
        .iter()
        .filter(|(_, z)| z.contains(&0))
        .map(|(id, z)| format!("{id}: positiv {}, negativ {}, n/a {}", z[0], z[1], z[2]))
        .collect();
    assert!(
        luecken.is_empty(),
        "{} von {} Kennungen ohne alle drei Fallarten:\n{}",
        luecken.len(),
        ids.len(),
        luecken.join("\n")
    );
}

#[test]
fn gate_mutationskatalog_deckt_jede_kennung() {
    let ids = kennungen();
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let eintraege = korpus::katalog::lade(&wurzel.join("tests/mutations"));
    let mut gedeckt = BTreeSet::new();
    let mut fehler = Vec::new();
    for e in &eintraege {
        if !ids.contains(e.id.as_str()) {
            fehler.push(format!("{}: Kennung nicht deklariert", e.id));
        }
        gedeckt.insert(e.id.clone());
        let pfad = wurzel.join(&e.file);
        let Ok(text) = std::fs::read_to_string(&pfad) else {
            fehler.push(format!("{}: Datei {} fehlt", e.id, e.file));
            continue;
        };
        let n = text.matches(e.find.as_str()).count();
        if n != 1 {
            fehler.push(format!(
                "{}: `find` kommt {n}-mal in {} vor, erwartet genau einmal",
                e.id, e.file
            ));
        }
        if e.find == e.replace {
            fehler.push(format!("{}: `find` und `replace` sind gleich", e.id));
        }
        if !matches!(e.expect.as_str(), "killed" | "equivalent") {
            fehler.push(format!("{}: expect = {:?}", e.id, e.expect));
        }
        if e.expect == "equivalent" && e.note.trim().is_empty() {
            fehler.push(format!("{}: äquivalente Mutation ohne Begründung", e.id));
        }
    }
    for id in &ids {
        if !gedeckt.contains(*id) {
            fehler.push(format!("{id}: keine Mutation im Katalog"));
        } else if !eintraege
            .iter()
            .any(|e| e.id == *id && e.expect == "killed")
        {
            fehler.push(format!("{id}: keine Mutation, die ein Test töten muss"));
        }
    }
    assert!(
        fehler.is_empty(),
        "Mutationskatalog unvollständig:\n{}",
        fehler.join("\n")
    );
}
