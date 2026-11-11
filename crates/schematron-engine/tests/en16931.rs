//! End-to-end measurement: the preprocessed EN 16931 UBL validation Schematron
//! (`queryBinding="xslt2"`) against two UBL example invoices of the same release. Neither is in
//! the repository (EUPL-1.2): fetch them with `crates/xpath-eval/xtask/vendor-en16931.sh`, then
//! run `cargo test -p schematron-engine --test en16931 -- --ignored --nocapture`.
//!
//! The examples (`ubl-tc434-example1`, `-example2`) are valid invoices, so no `fatal` rule may
//! fire; whatever fires is printed.

#![cfg(feature = "xpath2")]

mod support;

use std::fs;

use schematron_engine::{Event, evaluate, evaluate_trace, parse};
use support::RoxmlDoc;

const CORPUS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xpath-eval/tests/corpus/en16931"
);

#[test]
#[ignore = "needs the externally fetched EN 16931 corpus (crates/xpath-eval/xtask/vendor-en16931.sh)"]
fn ubl_validation_schematron_runs_on_valid_examples() {
    let schema = parse(&fs::read_to_string(format!("{CORPUS}/ubl.sch")).unwrap()).unwrap();
    assert_eq!(schema.query_binding.as_deref(), Some("xslt2"));
    let rules: usize = schema.patterns.iter().map(|p| p.rules.len()).sum();
    let checks: usize = schema
        .patterns
        .iter()
        .flat_map(|p| &p.rules)
        .map(|r| r.checks.len())
        .sum();
    println!(
        "schema: {} patterns, {rules} rules, {checks} checks",
        schema.patterns.len()
    );

    for example in ["ubl-tc434-example1.xml", "ubl-tc434-example2.xml"] {
        let xml = fs::read_to_string(format!("{CORPUS}/examples/{example}")).unwrap();
        let doc = RoxmlDoc::parse(&xml);
        // The rules must match nodes, otherwise "nothing fired" would prove nothing.
        let fired_rules = evaluate_trace(&schema, &doc)
            .unwrap()
            .iter()
            .filter(|event| matches!(event, Event::FiredRule { .. }))
            .count();
        println!("{example}: {fired_rules} rule firings");
        assert!(fired_rules > 100, "{example}");
        let reports = match evaluate(&schema, &doc) {
            Ok(reports) => reports,
            Err(error) => panic!("{example}: {error}"),
        };
        println!("{example}: {} checks fired", reports.len());
        for report in &reports {
            println!(
                "  {:?} {:?} {}",
                report.role, report.check_id, report.message
            );
        }
        let fatal: Vec<_> = reports
            .iter()
            .filter(|r| r.role.as_deref() == Some("fatal"))
            .collect();
        assert!(fatal.is_empty(), "{example}: {fatal:#?}");
    }
}

// The same schema must notice a broken invoice: BR-03 (no issue date) and a wrong payable
// amount (BR-CO-16) both fire, so the passes above are not vacuous.
#[test]
#[ignore = "needs the externally fetched EN 16931 corpus (crates/xpath-eval/xtask/vendor-en16931.sh)"]
fn ubl_validation_schematron_notices_a_broken_invoice() {
    let schema = parse(&fs::read_to_string(format!("{CORPUS}/ubl.sch")).unwrap()).unwrap();
    let xml = fs::read_to_string(format!("{CORPUS}/examples/ubl-tc434-example1.xml")).unwrap();
    let broken = xml
        .replace("<cbc:IssueDate>2015-01-09</cbc:IssueDate>", "")
        .replace(
            "<cbc:PayableAmount currencyID=\"EUR\">250.33</cbc:PayableAmount>",
            "<cbc:PayableAmount currencyID=\"EUR\">250.34</cbc:PayableAmount>",
        );
    assert_ne!(broken, xml);
    let doc = RoxmlDoc::parse(&broken);
    let reports = evaluate(&schema, &doc).unwrap();
    let ids: Vec<_> = reports
        .iter()
        .filter_map(|r| r.check_id.as_deref())
        .collect();
    println!("{ids:?}");
    assert!(ids.contains(&"BR-03"), "{ids:?}");
    assert!(ids.contains(&"BR-CO-16"), "{ids:?}");
}
