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

// ---- Peppol BIS Billing 3.0 and XRechnung (casoon/barrierlab#62) --------------------------------
//
// Both rule sets call helper functions declared with `xsl:function`. The tests run them against
// example invoices (fetched by the same script, pinned by SHA-256) and against a copy that is
// broken on purpose, so that a rule which needs a function result is seen to fire.

const PEPPOL: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xpath-eval/tests/corpus/peppol"
);
const XRECHNUNG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xpath-eval/tests/corpus/xrechnung"
);

/// Resolves `<include href>` against the directory of the including file.
struct Files;

impl schematron_engine::SchemaResolver for Files {
    fn resolve(
        &self,
        href: &str,
        base_uri: &str,
    ) -> Result<schematron_engine::SchemaSource, schematron_engine::ResolveError> {
        let directory = std::path::Path::new(base_uri)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let path = fs::canonicalize(directory.join(href))
            .map_err(|e| schematron_engine::ResolveError::new(format!("{href}: {e}")))?;
        let text = fs::read_to_string(&path)
            .map_err(|e| schematron_engine::ResolveError::new(format!("{href}: {e}")))?;
        Ok(schematron_engine::SchemaSource::new(
            text,
            path.to_string_lossy(),
        ))
    }
}

/// Parses a rule set and checks that every `xsl:function` of it is one the engine runs.
fn load(path: &str, functions: usize) -> schematron_engine::Schema {
    let mut text = fs::read_to_string(path).unwrap();
    // The Peppol CII rule set declares the assert id PEPPOL-EN16931-R043 twice (document level
    // and line level allowance/charge), and an id is unique in a Schematron document, so the
    // parser refuses it. The second one is renamed here; the file itself stays as fetched.
    if path.ends_with("peppol/cii.sch") {
        let needle = "id=\"PEPPOL-EN16931-R043\"";
        let first = text.find(needle).unwrap();
        let second = first + 1 + text[first + 1..].find(needle).unwrap();
        text.replace_range(second..second + needle.len(), "id=\"PEPPOL-EN16931-R043b\"");
    }
    let schema = schematron_engine::parse_with_resolver(&text, path, &Files).unwrap();
    assert_eq!(schema.functions.len(), functions, "{path}");
    let unsupported: Vec<_> = schema
        .functions
        .iter()
        .filter(|f| f.unsupported.is_some())
        .map(|f| (&f.name, &f.unsupported))
        .collect();
    assert!(unsupported.is_empty(), "{path}: {unsupported:?}");
    schema
}

/// The ids of the checks that fired on `xml`, in order; panics if the run did not complete.
fn fired(schema: &schematron_engine::Schema, xml: &str, what: &str) -> Vec<String> {
    let doc = RoxmlDoc::parse(xml);
    match evaluate(schema, &doc) {
        Ok(reports) => reports
            .into_iter()
            .map(|r| r.check_id.unwrap_or_default())
            .collect(),
        Err(error) => panic!("{what}: {error}"),
    }
}

fn example(directory: &str, name: &str) -> String {
    fs::read_to_string(format!("{directory}/examples/{name}")).unwrap()
}

/// `xml` with `from` replaced by `to`; it must occur, so a changed example cannot make the
/// "broken copy" silently identical to the original.
fn broken(xml: &str, from: &str, to: &str) -> String {
    assert!(xml.contains(from), "{from}");
    xml.replacen(from, to, 1)
}

const GLN_FROM: &str = "<cbc:EndpointID schemeID=\"0088\">9482348239847239874<";
const GLN_TO: &str = "<cbc:EndpointID schemeID=\"0088\">9482348239847239875<";

#[test]
#[ignore = "needs the externally fetched Peppol corpus (crates/xpath-eval/xtask/vendor-en16931.sh)"]
fn peppol_ubl_examples_pass_and_a_wrong_gln_fails() {
    let schema = load(&format!("{PEPPOL}/ubl.sch"), 13);
    for name in ["base-example.xml", "Allowance-example.xml"] {
        let xml = example(PEPPOL, name);
        // The rules must match nodes, otherwise "nothing fired" would prove nothing.
        let doc = RoxmlDoc::parse(&xml);
        let firings = evaluate_trace(&schema, &doc)
            .unwrap()
            .iter()
            .filter(|event| matches!(event, Event::FiredRule { .. }))
            .count();
        assert!(firings > 20, "{name}: {firings}");
        assert_eq!(fired(&schema, &xml, name), Vec::<String>::new(), "{name}");
    }
    // PEPPOL-COMMON-R040 is `matches(...) and u:gln(...)`: only the check digit is wrong.
    let xml = broken(&example(PEPPOL, "base-example.xml"), GLN_FROM, GLN_TO);
    assert_eq!(fired(&schema, &xml, "wrong GLN"), ["PEPPOL-COMMON-R040"]);
}

// Peppol publishes no CII example. The EN 16931 example is no Peppol invoice, so Peppol's own
// rules on the business process, the electronic addresses and so on fire, each for a reason that
// is stated here; a line of that example (a returned item with a positive quantity and a negative
// amount) also fails PEPPOL-EN16931-R120, which is `u:slack(...)`. Adding the missing electronic
// addresses removes R010 and R020; a GLN with a wrong check digit in them then fires R040.
#[test]
#[ignore = "needs the externally fetched Peppol corpus (crates/xpath-eval/xtask/vendor-en16931.sh)"]
fn peppol_cii_example_runs_and_a_wrong_gln_fails() {
    let schema = load(&format!("{PEPPOL}/cii.sch"), 12);
    let xml = example(PEPPOL, "CII_example1.xml");
    assert_eq!(
        fired(&schema, &xml, "CII_example1"),
        [
            "PEPPOL-EN16931-R001",  // no business process
            "PEPPOL-EN16931-R007",  // so no approved profile
            "PEPPOL-EN16931-R004",  // the guideline id is not the Peppol one
            "PEPPOL-EN16931-R003",  // no buyer reference or order reference
            "PEPPOL-EN16931-R002",  // two notes
            "PEPPOL-EN16931-R010",  // no buyer electronic address
            "PEPPOL-EN16931-R020",  // no seller electronic address
            "PEPPOL-EN16931-R120",  // line 20: quantity 6, price 18.33, line amount -109.98
            "PEPPOL-COMMON-R056-2", // Dutch VAT number NL8200.98.395.B.01
            "NL-R-003",             // Dutch supplier without KVK or OIN
        ]
    );

    let with_addresses = |gln: &str| {
        let address = format!(
            "<ram:URIUniversalCommunication><ram:URIID schemeID=\"0088\">{gln}</ram:URIID></ram:URIUniversalCommunication>"
        );
        let xml = broken(
            &xml,
            "<ram:SellerTradeParty>",
            &format!("<ram:SellerTradeParty>{address}"),
        );
        broken(
            &xml,
            "<ram:BuyerTradeParty>",
            &format!("<ram:BuyerTradeParty>{address}"),
        )
    };
    let good = fired(&schema, &with_addresses("9482348239847239874"), "good GLN");
    assert!(
        !good
            .iter()
            .any(|id| id.contains("R010") || id.contains("R020"))
    );
    assert!(
        !good.iter().any(|id| id == "PEPPOL-COMMON-R040"),
        "{good:?}"
    );
    let bad = fired(&schema, &with_addresses("9482348239847239875"), "wrong GLN");
    assert_eq!(
        bad.iter().filter(|id| *id == "PEPPOL-COMMON-R040").count(),
        2,
        "{bad:?}"
    );
}

// The XRechnung rule sets (UBL and CII) are fragments: `common.sch` is included, the version
// placeholders (`@xr-schematron.version.full@`) are left in the title. Of the test suite cases,
// 02.01a and 03.01a pass; 01.01a has no delivery date or period, which only the
// information-level BR-DE-TMP-32 asks for. The one function (`xr:checkIBAN`, a regular expression
// and a mod 97 over a number of 24 digits) decides BR-DE-19.
#[test]
#[ignore = "needs the externally fetched XRechnung corpus (crates/xpath-eval/xtask/vendor-en16931.sh)"]
fn xrechnung_examples_pass_and_a_wrong_iban_fails() {
    for (schema_path, functions, suffix, iban_from) in [
        (
            format!("{XRECHNUNG}/ubl/XRechnung-UBL-validation.sch"),
            1,
            "ubl",
            "<cbc:ID>DE79000000001234567890</cbc:ID>",
        ),
        (
            format!("{XRECHNUNG}/cii/XRechnung-CII-validation.sch"),
            2,
            "uncefact",
            "<ram:IBANID>DE79000000001234567890</ram:IBANID>",
        ),
    ] {
        let schema = load(&schema_path, functions);
        for case in ["01.01a", "02.01a", "03.01a"] {
            let name = format!("{case}-INVOICE_{suffix}.xml");
            let xml = example(XRECHNUNG, &name);
            let doc = RoxmlDoc::parse(&xml);
            let firings = evaluate_trace(&schema, &doc)
                .unwrap()
                .iter()
                .filter(|event| matches!(event, Event::FiredRule { .. }))
                .count();
            assert!(firings > 5, "{name}: {firings}");
            let expected: &[&str] = if case == "01.01a" {
                &["BR-DE-TMP-32"]
            } else {
                &[]
            };
            assert_eq!(fired(&schema, &xml, &name), expected, "{name}");
        }
        // A wrong check digit: the shape of the IBAN is fine, the mod 97 is not.
        let xml = broken(
            &example(XRECHNUNG, &format!("01.01a-INVOICE_{suffix}.xml")),
            iban_from,
            &iban_from.replace("DE79000000001234567890", "DE79000000001234567891"),
        );
        // The order of the two follows the order of the patterns, which differs per format.
        let mut ids = fired(&schema, &xml, "wrong IBAN");
        ids.sort();
        assert_eq!(ids, ["BR-DE-19", "BR-DE-TMP-32"]);
    }
}
