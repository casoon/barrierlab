use a11y_report::{InputKind, Outcome, Report};
use a11y_static::{Environment, Page, Sheet, StylesheetRef, audit};

const PAGE: &str = include_str!("page.html");

fn failed(r: &Report, id: &str) -> bool {
    r.findings
        .iter()
        .any(|f| f.rule_id == id && f.outcome == Outcome::Fail)
}

fn doc(head: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>t</title>{head}</head><body>{body}</body></html>"
    )
}

const LOW: &str = "<p>We answer within two days.</p>";

fn sheet(source: &str, css: &str) -> Sheet {
    Sheet {
        source: source.into(),
        css: css.into(),
        media: None,
    }
}

#[test]
fn example_page_matches_cli_readme() {
    let r = audit(PAGE, &[], Environment::default());
    for id in [
        "images/alt-missing",
        "links/name-missing",
        "contrast/text-insufficient",
    ] {
        assert!(failed(&r, id), "{id} missing");
    }
    assert!(r.summary.fail >= 5);
    assert!(r.inputs.is_empty());
}

#[test]
fn low_contrast_from_inline_style() {
    let html = doc("<style>body{background:#fff;color:#999}</style>", LOW);
    assert!(failed(
        &audit(&html, &[], Environment::default()),
        "contrast/text-insufficient"
    ));
    let ok = doc("<style>body{background:#fff;color:#000}</style>", LOW);
    assert!(!failed(
        &audit(&ok, &[], Environment::default()),
        "contrast/text-insufficient"
    ));
}

fn contrast(r: &Report, outcome: Outcome) -> bool {
    r.findings
        .iter()
        .any(|f| f.rule_id.starts_with("contrast/") && f.outcome == outcome)
}

#[test]
fn missing_link_sheet_makes_contrast_untested() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let r = audit(&html, &[], Environment::default());
    let note = r
        .inputs
        .iter()
        .find(|n| n.kind == InputKind::StylesheetUnavailable)
        .expect("note");
    assert_eq!(note.subject.as_deref(), Some("a.css"));
    assert!(!contrast(&r, Outcome::Fail));
    assert!(!contrast(&r, Outcome::Pass));
    assert!(
        r.findings
            .iter()
            .any(|f| f.rule_id == "contrast/text-undetermined" && f.outcome == Outcome::Untested),
        "{:?}",
        r.findings
    );
}

#[test]
fn complete_styles_still_decide_contrast() {
    let html = doc("<style>body{background:#fff;color:#000}</style>", LOW);
    let r = audit(&html, &[], Environment::default());
    assert!(!contrast(&r, Outcome::Untested));
}

#[test]
fn supplied_link_sheet_applies() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let s = sheet("a.css", "body{background:#fff;color:#999}");
    let r = audit(&html, &[s], Environment::default());
    assert!(r.inputs.is_empty());
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn alternate_disabled_and_print_do_not_apply() {
    let low = "background:#fff;color:#999";
    for head in [
        format!("<link rel=\"alternate stylesheet\" href=a.css><style>p{{{low}}}</style>"),
        format!("<link rel=stylesheet href=a.css disabled><style>p{{{low}}}</style>"),
    ] {
        // the link is not even a reference
        let page = Page::parse(&doc(&head, LOW));
        assert_eq!(page.stylesheet_refs().len(), 1);
    }
    let alt = doc("<link rel=\"alternate stylesheet\" href=a.css>", LOW);
    let r = audit(
        &alt,
        &[sheet("a.css", "p{background:#fff;color:#999}")],
        Environment::default(),
    );
    assert!(r.inputs.is_empty());
    assert!(!failed(&r, "contrast/text-insufficient"));

    let dis = doc("<link rel=stylesheet href=a.css disabled>", LOW);
    let r = audit(
        &dis,
        &[sheet("a.css", "p{background:#fff;color:#999}")],
        Environment::default(),
    );
    assert!(!failed(&r, "contrast/text-insufficient"));

    let print = doc(
        "<style media=print>body{background:#fff;color:#999}</style>",
        LOW,
    );
    assert!(!failed(
        &audit(&print, &[], Environment::default()),
        "contrast/text-insufficient"
    ));
    let print_link = doc("<link rel=stylesheet href=a.css media=print>", LOW);
    let r = audit(
        &print_link,
        &[sheet("a.css", "body{background:#fff;color:#999}")],
        Environment::default(),
    );
    assert!(!failed(&r, "contrast/text-insufficient"));
    let screen = doc(
        "<style media=screen>body{background:#fff;color:#999}</style>",
        LOW,
    );
    assert!(failed(
        &audit(&screen, &[], Environment::default()),
        "contrast/text-insufficient"
    ));
}

#[test]
fn import_is_noted() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let s = sheet("a.css", "@import url(b.css); p{color:#999}");
    let r = audit(&html, &[s], Environment::default());
    assert!(
        r.inputs
            .iter()
            .any(|n| n.kind == InputKind::StylesheetUnresolved),
        "{:?}",
        r.inputs
    );
}

#[test]
fn hidden_attribute_and_aria_hidden_are_not_audited() {
    let html = doc(
        "",
        "<img src=a.png hidden><div aria-hidden=true><img src=b.png></div>",
    );
    assert!(!failed(
        &audit(&html, &[], Environment::default()),
        "images/alt-missing"
    ));
}

#[test]
fn refs_in_document_order() {
    let html = doc(
        "<link rel=stylesheet href=a.css media=\"screen\"><style>p{}</style>\
         <link rel=\"preload stylesheet\" href=b.css><link rel=stylesheet href=\"\">\
         <link rel=icon href=i.png><style media=\" print \">q{}</style>",
        "",
    );
    assert_eq!(
        Page::parse(&html).stylesheet_refs(),
        vec![
            StylesheetRef::Link {
                href: "a.css".into(),
                media: Some("screen".into())
            },
            StylesheetRef::Inline {
                css: "p{}".into(),
                media: None
            },
            StylesheetRef::Link {
                href: "b.css".into(),
                media: None
            },
            StylesheetRef::Inline {
                css: "q{}".into(),
                media: Some("print".into())
            },
        ]
    );
}
