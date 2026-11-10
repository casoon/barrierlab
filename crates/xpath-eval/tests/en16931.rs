//! Coverage measurement against the XPath expressions of the EN 16931 validation
//! Schematron (UBL and CII, `queryBinding="xslt2"`). The corpus is NOT in the
//! repository (EUPL-1.2): fetch it with `xtask/vendor-en16931.sh`, then run
//! `cargo test -p xpath-eval --test en16931 -- --ignored --nocapture`.
//!
//! Every `@test` and `@context` of the preprocessed schemas is parsed and then
//! evaluated against a small sample invoice (context node: the document
//! element for `@test`, the document root for `@context`). "Evaluates" means
//! "returns a value instead of an error"; the result itself is not judged, the
//! sample is not a conforming invoice. The numbers go into `CONFORMANCE.md`.

use std::collections::BTreeSet;
use std::fs;

mod common;

use common::{Doc, N, parse_xml};
use xpath_eval::{
    Document, EvaluationContext, Node, XPathVersion, evaluate_versioned, parse_versioned,
};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/en16931");

const SAMPLE_UBL: &str = r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
  xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
  xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>
  <cac:InvoicePeriod><cbc:StartDate>2026-01-01</cbc:StartDate><cbc:EndDate>2026-01-31</cbc:EndDate></cac:InvoicePeriod>
  <cac:TaxTotal><cbc:TaxAmount currencyID="EUR">19.00</cbc:TaxAmount></cac:TaxTotal>
  <cac:LegalMonetaryTotal>
    <cbc:LineExtensionAmount currencyID="EUR">100.00</cbc:LineExtensionAmount>
    <cbc:TaxExclusiveAmount currencyID="EUR">100.00</cbc:TaxExclusiveAmount>
    <cbc:TaxInclusiveAmount currencyID="EUR">119.00</cbc:TaxInclusiveAmount>
    <cbc:PayableAmount currencyID="EUR">119.00</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
  <cac:InvoiceLine><cbc:ID>1</cbc:ID><cbc:LineExtensionAmount currencyID="EUR">100.00</cbc:LineExtensionAmount>
    <cac:Item><cac:ClassifiedTaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19</cbc:Percent>
      <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:ClassifiedTaxCategory></cac:Item>
  </cac:InvoiceLine>
</Invoice>"#;

const SAMPLE_CII: &str = r#"<rsm:CrossIndustryInvoice
  xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
  xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100"
  xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100">
  <rsm:SupplyChainTradeTransaction>
    <ram:IncludedSupplyChainTradeLineItem>
      <ram:SpecifiedLineTradeSettlement>
        <ram:ApplicableTradeTax><ram:TypeCode>VAT</ram:TypeCode><ram:CategoryCode>S</ram:CategoryCode><ram:RateApplicablePercent>19</ram:RateApplicablePercent></ram:ApplicableTradeTax>
        <ram:SpecifiedTradeSettlementLineMonetarySummation><ram:LineTotalAmount>100.00</ram:LineTotalAmount></ram:SpecifiedTradeSettlementLineMonetarySummation>
      </ram:SpecifiedLineTradeSettlement>
    </ram:IncludedSupplyChainTradeLineItem>
    <ram:ApplicableHeaderTradeSettlement>
      <ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>
      <ram:ApplicableTradeTax><ram:CalculatedAmount>19.00</ram:CalculatedAmount><ram:TypeCode>VAT</ram:TypeCode><ram:BasisAmount>100.00</ram:BasisAmount><ram:CategoryCode>S</ram:CategoryCode><ram:RateApplicablePercent>19</ram:RateApplicablePercent></ram:ApplicableTradeTax>
      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>
        <ram:LineTotalAmount>100.00</ram:LineTotalAmount><ram:TaxBasisTotalAmount>100.00</ram:TaxBasisTotalAmount>
        <ram:TaxTotalAmount currencyID="EUR">19.00</ram:TaxTotalAmount><ram:GrandTotalAmount>119.00</ram:GrandTotalAmount><ram:DuePayableAmount>119.00</ram:DuePayableAmount>
      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>
    </ram:ApplicableHeaderTradeSettlement>
  </rsm:SupplyChainTradeTransaction>
</rsm:CrossIndustryInvoice>"#;

struct Corpus {
    namespaces: Vec<(String, String)>,
    /// `(kind, expression)`, duplicates kept.
    expressions: Vec<(&'static str, String)>,
}

fn load(file: &str) -> Corpus {
    let path = format!("{CORPUS}/{file}");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e} - run xtask/vendor-en16931.sh"));
    let xml = roxmltree::Document::parse(&text).unwrap();
    let mut namespaces = Vec::new();
    let mut expressions = Vec::new();
    for e in xml.descendants().filter(|n| n.is_element()) {
        match e.tag_name().name() {
            "ns" => namespaces.push((
                e.attribute("prefix").unwrap().to_owned(),
                e.attribute("uri").unwrap().to_owned(),
            )),
            "assert" | "report" => {
                expressions.push(("test", e.attribute("test").unwrap().to_owned()))
            }
            "rule" => expressions.push(("context", e.attribute("context").unwrap().to_owned())),
            _ => {}
        }
    }
    Corpus {
        namespaces,
        expressions,
    }
}

#[derive(Default)]
struct Tally {
    total: usize,
    parsed: usize,
    evaluated: usize,
    first_failures: Vec<String>,
}

fn measure(version: XPathVersion, corpus: &Corpus, doc: &Doc) -> (Tally, BTreeSet<String>) {
    let lookup_ns = |p: &str| {
        corpus
            .namespaces
            .iter()
            .find(|(k, _)| k == p)
            .map(|(_, v)| v.clone())
    };
    let root = doc.root();
    let element = root
        .children()
        .find(|c| c.expanded_name().is_some())
        .unwrap();
    let mut tally = Tally::default();
    let mut reasons = BTreeSet::new();
    for (kind, source) in &corpus.expressions {
        tally.total += 1;
        let expr = match parse_versioned(source, version) {
            Ok(e) => e,
            Err(e) => {
                reasons.insert(format!("parse: {e}"));
                continue;
            }
        };
        tally.parsed += 1;
        let node: N<'_> = if *kind == "context" { root } else { element };
        let mut ctx = EvaluationContext::new(node);
        ctx.namespaces = Some(&lookup_ns);
        match evaluate_versioned(&expr, &ctx) {
            Ok(_) => tally.evaluated += 1,
            Err(e) => {
                reasons.insert(format!("eval: {e}"));
                if tally.first_failures.len() < 5 {
                    tally.first_failures.push(format!("{e}: {source}"));
                }
            }
        }
    }
    (tally, reasons)
}

#[test]
#[ignore = "needs tests/corpus/en16931 (xtask/vendor-en16931.sh)"]
fn en16931_coverage() {
    for (file, sample) in [("ubl.sch", SAMPLE_UBL), ("cii.sch", SAMPLE_CII)] {
        let doc = parse_xml(sample);
        let corpus = load(file);
        for version in [XPathVersion::V1, XPathVersion::V2] {
            let (t, reasons) = measure(version, &corpus, &doc);
            println!(
                "{file} {version:?}: {} expressions, {} parse, {} evaluate",
                t.total, t.parsed, t.evaluated
            );
            for r in reasons.iter().take(8) {
                println!("    {}", r.lines().next().unwrap_or(""));
            }
            for f in &t.first_failures {
                let mut lines = f.lines();
                println!("    e.g. {} ...", lines.next().unwrap_or(""));
                if let Some(expr) = f.rsplit(": ").next() {
                    println!("      {}", &expr[..expr.len().min(160)]);
                }
            }
        }
    }
}
