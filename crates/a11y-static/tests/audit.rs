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
    Sheet::new(source, css)
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

fn untested_with(r: &Report, kind: InputKind, detail: &str) {
    let note = r.inputs.iter().find(|n| n.kind == kind).expect("note");
    assert_eq!(note.detail.as_deref(), Some(detail), "{:?}", r.inputs);
    assert!(!contrast(r, Outcome::Fail) && !contrast(r, Outcome::Pass));
    assert!(contrast(r, Outcome::Untested), "{:?}", r.findings);
}

#[test]
fn failure_reason_reaches_the_note() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let r = audit(
        &html,
        &[Sheet::failed("a.css", "HTTP 404")],
        Environment::default(),
    );
    let note = &r.inputs[0];
    assert_eq!(note.subject.as_deref(), Some("a.css"));
    untested_with(&r, InputKind::StylesheetUnavailable, "HTTP 404");
}

#[test]
fn failed_sheet_text_is_not_used() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let mut s = Sheet::failed("a.css", "too large");
    s.css = "body{background:#fff;color:#000}".into();
    let r = audit(&html, &[s], Environment::default());
    untested_with(&r, InputKind::StylesheetUnavailable, "too large");
}

#[test]
fn imported_sheet_clears_the_note() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let a = Sheet::new("a.css", "@import url(b.css); p{color:#999}")
        .with_imports(vec![Sheet::new("b.css", "p{background:#fff}")]);
    let r = audit(&html, &[a], Environment::default());
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn nested_and_quoted_imports_resolve() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let b = Sheet::new("b.css", "@import 'c.css'; p{color:#999}")
        .with_imports(vec![Sheet::new("c.css", "p{background:#fff}")]);
    let a = Sheet::new("a.css", "@import \"b.css\" screen;").with_imports(vec![b]);
    let r = audit(&html, &[a], Environment::default());
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn import_in_inline_style_uses_top_level_sheets() {
    let html = doc("<style>@import url(b.css); p{color:#999}</style>", LOW);
    let r = audit(
        &html,
        &[sheet("b.css", "p{background:#fff}")],
        Environment::default(),
    );
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn failed_import_keeps_reason_and_untested() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let a = Sheet::new("a.css", "@import url(b.css); p{color:#999}")
        .with_imports(vec![Sheet::failed("b.css", "HTTP 500")]);
    let r = audit(&html, &[a], Environment::default());
    untested_with(&r, InputKind::StylesheetUnresolved, "HTTP 500");
}

#[test]
fn import_cycle_stays_unresolved() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let a = Sheet::new("a.css", "@import url(b.css);").with_imports(vec![
        Sheet::new("b.css", "@import url(b.css);")
            .with_imports(vec![Sheet::new("b.css", "p{background:#fff}")]),
    ]);
    let r = audit(&html, &[a], Environment::default());
    assert!(
        r.inputs
            .iter()
            .any(|n| n.kind == InputKind::StylesheetUnresolved),
        "{:?}",
        r.inputs
    );
    assert!(!contrast(&r, Outcome::Pass) && !contrast(&r, Outcome::Fail));
}

// CSS Cascade 5 §6.4.1: the rules of `@import … layer(x)` sit in layer x, which
// loses to every unlayered rule (§6.4.2).
#[test]
fn layered_import_is_applied_in_its_layer() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let imported = || vec![Sheet::new("b.css", "body p{color:#999}")];
    for import in [
        "@import url(b.css) layer(x);",
        "@import url(b.css) layer;",
        "@import url(b.css) layer(x) supports(display: grid) screen;",
    ] {
        let a = Sheet::new("a.css", format!("{import} p{{color:#000;background:#fff}}"))
            .with_imports(imported());
        let r = audit(&html, &[a], Environment::default());
        assert!(r.inputs.is_empty(), "{import}: {:?}", r.inputs);
        assert!(
            !contrast(&r, Outcome::Fail) && !contrast(&r, Outcome::Untested),
            "{import}: {:?}",
            r.findings
        );
    }
    // Control: without a layer the imported rule applies.
    let a = Sheet::new("a.css", "@import url(b.css); p{background:#fff}")
        .with_imports(vec![Sheet::new("b.css", "body p{color:#999}")]);
    let r = audit(&html, &[a], Environment::default());
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn unsupplied_layered_import_stays_unresolved() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let a = Sheet::new(
        "a.css",
        "@import url(b.css) layer(x); p{background:#fff;color:#000}",
    );
    let r = audit(&html, &[a], Environment::default());
    let note = r
        .inputs
        .iter()
        .find(|n| n.kind == InputKind::StylesheetUnresolved)
        .expect("note");
    assert_eq!(note.subject.as_deref(), Some("url(b.css) layer(x)"));
    assert!(!contrast(&r, Outcome::Pass) && !contrast(&r, Outcome::Fail));
}

#[test]
fn import_with_unmet_condition_does_not_apply() {
    // Media Queries 4 / Conditional 3 §7.1: a failing condition drops the sheet.
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    for import in [
        "@import url(b.css) print;",
        "@import url(b.css) supports(not (display: grid));",
    ] {
        let a = Sheet::new("a.css", format!("{import} p{{color:#000;background:#fff}}"))
            .with_imports(vec![Sheet::new("b.css", "body p{color:#999}")]);
        let r = audit(&html, &[a], Environment::default());
        assert!(r.inputs.is_empty(), "{import}: {:?}", r.inputs);
        assert!(
            !contrast(&r, Outcome::Fail) && !contrast(&r, Outcome::Untested),
            "{import}: {:?}",
            r.findings
        );
    }
}

#[test]
fn links_with_same_href_match_by_media() {
    let html = doc(
        "<link rel=stylesheet href=a.css media=screen><link rel=stylesheet href=a.css media=print>",
        LOW,
    );
    let screen = Sheet::new("a.css", "p{background:#fff;color:#999}").with_media("Screen ");
    let print = Sheet::new("a.css", "p{background:#fff;color:#000}").with_media("print");
    let r = audit(&html, &[screen.clone(), print], Environment::default());
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));

    // only the screen sheet supplied: the print link is unavailable
    let r = audit(&html, &[screen], Environment::default());
    assert_eq!(r.inputs.len(), 1, "{:?}", r.inputs);
    assert_eq!(r.inputs[0].subject.as_deref(), Some("a.css"));
    assert!(!contrast(&r, Outcome::Fail) && !contrast(&r, Outcome::Pass));
}

#[test]
fn exact_media_wins_over_wildcard_sheet() {
    let html = doc("<link rel=stylesheet href=a.css media=screen>", LOW);
    let any = sheet("a.css", "p{background:#fff;color:#000}");
    let exact = Sheet::new("a.css", "p{background:#fff;color:#999}").with_media("screen");
    let r = audit(&html, &[any, exact], Environment::default());
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn sheet_with_other_media_does_not_answer_the_link() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let s = Sheet::new("a.css", "p{background:#fff;color:#000}").with_media("print");
    let r = audit(&html, &[s], Environment::default());
    assert_eq!(r.inputs[0].kind, InputKind::StylesheetUnavailable);
}

#[test]
fn empty_href_is_noted() {
    for link in [
        "<link rel=stylesheet href=\"\">",
        "<link rel=stylesheet href=\"  \">",
        "<link rel=stylesheet>",
    ] {
        let r = audit(&doc(link, LOW), &[], Environment::default());
        assert_eq!(r.inputs.len(), 1, "{link}");
        assert_eq!(r.inputs[0].subject, None);
        untested_with(
            &r,
            InputKind::StylesheetUnavailable,
            "<link rel=stylesheet> has an empty or missing href",
        );
    }
    // still not listed for fetching
    let page = Page::parse(&doc("<link rel=stylesheet href=\"\">", ""));
    assert!(page.stylesheet_refs().is_empty());
}

const FONT_IMPORT: &str = "@import url(https://fonts.example/css?family=X);";

#[test]
fn inert_import_keeps_contrast_decided() {
    let html = doc(
        &format!("<style>{FONT_IMPORT} p{{color:#999;background:#fff}}</style>"),
        LOW,
    );
    // Unanswered: the host cannot know what the import sets.
    let r = audit(&html, &[], Environment::default());
    assert!(
        r.inputs
            .iter()
            .any(|i| i.kind == InputKind::StylesheetUnresolved)
    );
    assert!(!failed(&r, "contrast/text-insufficient"));
    // Declared by the host as without effect on styles.
    let sheets = [Sheet::inert("https://fonts.example/css?family=X")];
    let r = audit(&html, &sheets, Environment::default());
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));
}

#[test]
fn inert_import_inside_a_linked_sheet() {
    let html = doc("<link rel=stylesheet href=a.css>", LOW);
    let a = Sheet::new(
        "a.css",
        format!("{FONT_IMPORT} p{{color:#999;background:#fff}}"),
    )
    .with_imports(vec![Sheet::inert("https://fonts.example/css?family=X")]);
    let r = audit(&html, &[a], Environment::default());
    assert!(r.inputs.is_empty(), "{:?}", r.inputs);
    assert!(failed(&r, "contrast/text-insufficient"));
}
