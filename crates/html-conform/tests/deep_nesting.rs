//! A document nested deeper than the checker validates is a typed error,
//! never a process abort and never an empty (passing) report. Runs on a
//! 2 MiB thread, the default for tokio's worker threads (auditmysite#783).

use html_conform::{CheckError, CheckReport, check};

/// `html` and `body` are levels 1 and 2; `divs` nested `div`s follow.
fn nested_divs(divs: usize) -> String {
    format!(
        "<!doctype html><html lang=en><title>t</title><body>{}x{}",
        "<div>".repeat(divs),
        "</div>".repeat(divs)
    )
}

fn check_on_small_stack(html: String) -> Result<CheckReport, CheckError> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || check(&html))
        .expect("spawn test thread")
        .join()
        .expect("checking must not overflow the stack")
}

#[test]
fn document_at_the_depth_limit_is_checked() {
    let report = check_on_small_stack(nested_divs(510)).expect("512 levels are checked");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}

#[test]
fn document_one_level_over_the_limit_is_refused() {
    assert_eq!(
        check_on_small_stack(nested_divs(511)),
        Err(CheckError::DocumentTooDeep {
            depth: 513,
            limit: 512
        })
    );
}

#[test]
fn ten_thousand_levels_are_refused_without_overflow() {
    let error = check_on_small_stack(nested_divs(10_000)).expect_err("too deep to check");
    assert_eq!(
        error,
        CheckError::DocumentTooDeep {
            depth: 10_002,
            limit: 512
        }
    );
    assert_eq!(
        error.to_string(),
        "document not checked: element nesting depth 10002 exceeds the checker's limit of 512"
    );
}

/// `template` contents are walked like any other subtree, so their depth
/// counts too.
#[test]
fn depth_inside_template_contents_counts() {
    let html = format!(
        "<!doctype html><html lang=en><title>t</title><body><template>{}x{}</template>",
        "<div>".repeat(600),
        "</div>".repeat(600)
    );
    assert!(matches!(
        check_on_small_stack(html),
        Err(CheckError::DocumentTooDeep { limit: 512, .. })
    ));
}
