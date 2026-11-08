use a11y_report::{Outcome, Report};
use a11y_static::{Environment, audit};

const BODY: &str = "<h1>Gr\u{fc}\u{df}e \u{2014} \u{65e5}\u{672c}\u{8a9e} \u{1f600}</h1>\n\
<p>Zeile \u{e4}\u{f6}\u{fc}</p>\n\
  <img src=\"a.jpg\">\n\
<p>\u{65e5}\u{672c}\u{8a9e} <a href=\"/x\"></a> Text</p>\n\
<button></button>\n";

fn page(eol: &str) -> String {
    let html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>t</title>\n</head>\n<body>\n{BODY}</body>\n</html>\n"
    );
    html.replace('\n', eol)
}

/// Line and column (in characters) of `offset`, computed independently of the parser.
fn expected(html: &str, offset: usize) -> (u32, u32) {
    let before = &html[..offset];
    let line = before.matches('\n').count() as u32 + 1;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[start..].chars().count() as u32 + 1)
}

fn assert_points_at(html: &str, report: &Report, rule: &str, tag: &str) {
    let found: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == rule && f.outcome == Outcome::Fail)
        .collect();
    assert!(!found.is_empty(), "{rule}: no failing finding");
    let hit = found.iter().any(|f| {
        let Some(p) = f.location.position else {
            return false;
        };
        html[p.byte_offset..].starts_with(tag)
            && (p.line, p.column) == expected(html, p.byte_offset)
    });
    assert!(hit, "{rule}: no position at {tag}: {found:#?}");
}

fn check(eol: &str) {
    let html = page(eol);
    let r = audit(&html, &[], Environment::default());
    assert_points_at(&html, &r, "images/alt-missing", "<img src=\"a.jpg\">");
    assert_points_at(&html, &r, "links/name-missing", "<a href=\"/x\">");
    assert_points_at(&html, &r, "buttons/name-missing", "<button>");
}

#[test]
fn offsets_point_at_the_tag_with_multibyte_text() {
    check("\n");
}

#[test]
fn offsets_point_at_the_tag_with_crlf() {
    check("\r\n");
}

#[test]
fn implied_elements_have_no_position() {
    // No <html>, <head> or <body> in the source: a finding on them has no position.
    let html = "<title>t</title><p>x</p>";
    let r = audit(html, &[], Environment::default());
    let on_root = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "document/lang-missing")
        .collect::<Vec<_>>();
    assert!(!on_root.is_empty(), "{:?}", r.findings);
    assert!(on_root.iter().all(|f| f.location.position.is_none()));
}
