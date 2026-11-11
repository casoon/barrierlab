use std::collections::HashSet;

use a11y_static::{Environment, audit, rule_anchor, rule_help_url};

const PAGE_URL: &str = "https://casoon.github.io/barrierlab/docs/a11y/reference/regeln/";

fn rule_page() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/a11y/reference/regeln.mdx"
    );
    std::fs::read_to_string(path).expect("read the generated rule reference")
}

/// Every declared rule has an anchor on the generated rule page, and the
/// anchors are unique.
#[test]
fn every_catalogue_rule_has_an_anchor_on_the_page() {
    let page = rule_page();
    let mut seen = HashSet::new();
    for meta in a11y_rules::catalogue() {
        for id in meta.ids {
            let anchor = rule_anchor(id);
            assert!(seen.insert(anchor.clone()), "{id}: duplicate anchor");
            assert!(
                page.contains(&format!("| `{id}`<a id=\"{anchor}\"></a> |")),
                "{id}: no anchor {anchor} on the rule page; regenerate it with examples/a11y"
            );
            assert_eq!(
                rule_help_url(id).as_deref(),
                Some(format!("{PAGE_URL}#{anchor}").as_str())
            );
        }
    }
}

#[test]
fn unknown_rule_has_no_url() {
    assert_eq!(rule_help_url("no/such-rule"), None);
}

#[test]
fn audit_findings_carry_the_url() {
    let html = r#"<!doctype html><html lang="en"><head><title>t</title></head>
      <body><img src="a.png"></body></html>"#;
    let report = audit(html, &[], Environment::default());
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .expect("images/alt-missing finding");
    assert_eq!(
        f.help_url.as_deref(),
        Some("https://casoon.github.io/barrierlab/docs/a11y/reference/regeln/#images-alt-missing")
    );
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.help_url.is_some() || a11y_rules::meta_for(&f.rule_id).is_none())
    );
}
