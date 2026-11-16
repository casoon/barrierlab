//! Runs the W3C ACT Rules test cases through `a11y-static` and reports, per
//! ACT rule, how consistent the BarrierLab rules are with the expected results.
//!
//! ```text
//! python3 xtask/act/fetch.py target/act
//! cargo run --release --manifest-path xtask/act/Cargo.toml -- target/act
//! ```
//!
//! Writes `<cache>/earl.json` (an EARL report in the form the ACT Rules
//! Community Group takes implementation reports in) and prints a summary;
//! `--summary <file>` also writes the summary as JSON, `--markdown <file>` the
//! per-rule table for the documentation.
//!
//! With a browser as host, auditmysite renders the test cases in Chrome and
//! the runner reads its per-page reports instead of running `a11y-static`:
//!
//! ```text
//! cargo run --release --manifest-path xtask/act/Cargo.toml -- target/act \
//!   --urls target/act/urls.txt
//! auditmysite --url-file target/act/urls.txt --modules a11y -f json \
//!   --per-page-reports -o target/act/auditmysite
//! cargo run --release --manifest-path xtask/act/Cargo.toml -- target/act \
//!   --auditmysite target/act/auditmysite
//! ```
//!
//! auditmysite reports the BarrierLab rules it has adopted under the same IDs;
//! a rule it has not adopted is listed as not run, and its cases come out
//! `cantTell`. The EARL report is then `<cache>/earl-auditmysite.json`.
//!
//! The outcome of one test case for one ACT rule, from the findings of the
//! mapped BarrierLab rules (see [`MAPPING`]):
//!
//! - `failed`: a mapped rule reported `FAIL`;
//! - `cantTell`: no `FAIL`, but a mapped rule reported `REVIEW` or `UNTESTED`,
//!   or did not run (its input was missing);
//! - `passed`: all mapped rules ran and reported nothing.
//!
//! - `inapplicable`: the test case is not an HTML page (SVG, XHTML, XML) and
//!   the ACT rule applies to HTML pages only (see [`HTML_PAGE_ONLY`]). For the
//!   other rules such a file goes through `a11y-static` like an HTML page: the
//!   HTML parser puts its elements in the SVG or MathML namespace or leaves
//!   them unknown, and rules about HTML elements do not apply to them.
//!
//! An ACT rule without a mapping is `untested`. The categories per rule follow
//! `getConsistency` in `act-rules/act-tools`, the code behind the W3C ACT
//! implementation reports:
//!
//! - **consistent** (`complete` there): no passed or inapplicable example comes
//!   out `failed`, no failed example comes out `passed`, `inapplicable` or
//!   `untested`, no case is `untested`, and at least one failed example comes
//!   out `failed`. `cantTell` on some examples is allowed;
//! - **partial**: no such false positive, and at least one failed example comes
//!   out `failed` — or some case is `cantTell` and every inapplicable example
//!   comes out `passed` or `inapplicable`;
//! - otherwise **not consistent**.
//!
//! The W3C report also checks that the reported success criteria match the
//! rule's; that check is not repeated here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use a11y_report::{Outcome, Report};
use a11y_static::{Environment, Locale, Page, Sheet, StylesheetRef};
use serde_json::{Value, json};

/// ACT rules whose applicability is the `html` element of an HTML page (a
/// document served as `text/html`). A test case in another format has no such
/// element, so the rule is inapplicable there whatever the page contains.
const HTML_PAGE_ONLY: &[&str] = &["2779a5", "5b7ae0", "b5c3f8", "bf051a", "cf77f2"];

/// ACT rule → the BarrierLab rule IDs that decide the same question. Only
/// rules whose check is the same as the ACT rule's are listed; an ACT rule
/// that needs judgement (descriptive names, media alternatives) or a running
/// browser that `a11y-static` does not have stays out and is `untested`.
const MAPPING: &[(&str, &[&str])] = &[
    ("5f99a7", &["aria/attribute-unknown"]),
    ("de46e4", &["language/part-invalid"]),
    ("24afc2", &["text-spacing/letter-important"]),
    ("9e45ec", &["text-spacing/word-important"]),
    ("78fd32", &["text-spacing/line-height-important"]),
    ("46ca7f", &["aria/presentation-conflict"]),
    ("307n5z", &["aria/presentational-children-focusable"]),
    ("ff89c9", &["aria/required-parent-missing"]),
    ("bc4a75", &["aria/required-children-missing"]),
    ("6a7281", &["aria/attribute-value-invalid"]),
    (
        "5c01ea",
        &["aria/attribute-not-allowed", "aria/attribute-prohibited"],
    ),
    (
        "4e8ab6",
        &[
            "aria/required-attribute-missing",
            "aria/combobox-popup-missing",
        ],
    ),
    ("674b10", &["aria/role-invalid"]),
    ("73f2c2", &["forms/autocomplete-invalid"]),
    ("97a4e1", &["buttons/name-missing"]),
    ("e086e5", &["forms/label-missing", "names/required-missing"]),
    ("m6b1q3", &["names/required-missing"]),
    ("c487ae", &["links/name-missing", "images/area-alt-missing"]),
    ("23a2a8", &["images/alt-missing", "names/required-missing"]),
    ("59796f", &["images/input-alt-missing"]),
    ("8fc3b6", &["objects/alt-missing"]),
    ("7d6734", &["svg/name-missing"]),
    ("cae760", &["frames/name-missing"]),
    ("ffd0e9", &["headings/empty"]),
    ("2ee8b8", &["label-in-name/mismatch"]),
    ("6cfa84", &["keyboard/hidden-focusable"]),
    ("0ssw9k", &["keyboard/scrollable-region-not-focusable"]),
    ("cf77f2", &["headings/none"]),
    ("b5c3f8", &["document/lang-missing"]),
    ("bf051a", &["document/lang-invalid"]),
    ("5b7ae0", &["document/lang-mismatch"]),
    (
        "2779a5",
        &["document/title-missing", "document/title-empty"],
    ),
    ("3ea0c8", &["ids/duplicate"]),
    ("a25f45", &["tables/headers-attr-invalid"]),
    ("d0f69e", &["tables/header-without-data"]),
    ("bc659a", &["timing/meta-refresh"]),
    ("bisz58", &["timing/meta-refresh"]),
    ("b4f0c3", &["zoom/viewport-locked"]),
    ("b33eff", &["orientation/root-rotated"]),
    ("80f0bf", &["media/audio-autoplay"]),
    (
        "afw4f7",
        &["contrast/text-insufficient", "contrast/text-undetermined"],
    ),
    (
        "09o5cg",
        &["contrast/text-enhanced", "contrast/text-undetermined"],
    ),
    (
        "oj04fd",
        &["focus/outline-removed", "focus/indicator-missing"],
    ),
];

fn local(cache: &Path, url: &str) -> PathBuf {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    cache.join("files").join(rest)
}

/// Resolves `href` against `base` (both URLs of the test case site).
fn join(base: &str, href: &str) -> String {
    if href.contains("://") {
        return href.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    if let Some(r) = href.strip_prefix("//") {
        return format!("{scheme}://{r}");
    }
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut segs: Vec<&str> = if href.starts_with('/') {
        Vec::new()
    } else {
        let mut s: Vec<&str> = path.split('/').collect();
        s.pop();
        s
    };
    for seg in href.trim_start_matches('/').split('/') {
        match seg {
            "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    format!("{scheme}://{host}/{}", segs.join("/"))
}

fn sheets(page: &Page, cache: &Path, url: &str) -> Vec<Sheet> {
    page.stylesheet_refs()
        .into_iter()
        .filter_map(|r| match r {
            StylesheetRef::Link { href, media } => {
                let path = local(cache, &join(url, &href));
                let sheet = match fs::read_to_string(&path) {
                    Ok(css) => Sheet::new(href, css),
                    Err(_) => Sheet::failed(href, "not in the ACT cache"),
                };
                Some(match media {
                    Some(m) => sheet.with_media(m),
                    None => sheet,
                })
            }
            _ => None,
        })
        .collect()
}

fn outcome(report: &Report, ids: &[&str]) -> &'static str {
    let mine = |id: &str| ids.contains(&id);
    let findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| mine(&f.rule_id))
        .collect();
    if findings.iter().any(|f| f.outcome == Outcome::Fail) {
        return "failed";
    }
    if !findings.is_empty() {
        return "cantTell";
    }
    let ran = ids.iter().all(|id| {
        report
            .rule_runs
            .iter()
            .any(|r| r.rule_id == *id && r.not_run.is_none())
    });
    if ran { "passed" } else { "cantTell" }
}

/// The outcome of one test case from an auditmysite page report: the same
/// decision as [`outcome`], over `findings[].contract.outcome` and
/// `rule_outcomes[]` (a rule counts as run when one viewport ran it).
fn outcome_auditmysite(page: &Value, ids: &[&str]) -> &'static str {
    let mine = |v: &Value| v.as_str().is_some_and(|id| ids.contains(&id));
    let verdicts: Vec<&str> = page["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| mine(&f["rule_id"]))
        .map(|f| f["contract"]["outcome"].as_str().unwrap_or(""))
        .collect();
    if verdicts.contains(&"fail") {
        return "failed";
    }
    if !verdicts.is_empty() {
        return "cantTell";
    }
    let runs = page["rule_outcomes"].as_array();
    // Findings with outcome review land in `accessibility_assessments`, not in
    // `findings[]`; `rule_outcomes` still counts them. A rule that reported
    // something did not pass.
    let reported = runs
        .into_iter()
        .flatten()
        .any(|r| mine(&r["rule_id"]) && r["findings"].as_u64().is_some_and(|n| n > 0));
    if reported {
        return "cantTell";
    }
    let ran = ids.iter().all(|id| {
        runs.into_iter()
            .flatten()
            .any(|r| r["rule_id"] == *id && r.get("not_run").is_none())
    });
    if ran { "passed" } else { "cantTell" }
}

/// The page reports auditmysite wrote with `--per-page-reports`, by URL.
fn auditmysite_pages(dir: &Path) -> BTreeMap<String, Value> {
    let mut pages = BTreeMap::new();
    for entry in fs::read_dir(dir).expect("auditmysite report directory") {
        let path = entry.expect("directory entry").path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Ok(report) = serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()) else {
            continue;
        };
        // `index.json` lists the same URLs without findings; only full page
        // reports carry `rule_outcomes`.
        for page in report["pages"].as_array().into_iter().flatten() {
            if let Some(url) = page["url"].as_str()
                && page.get("rule_outcomes").is_some()
            {
                pages.insert(url.to_string(), page.clone());
            }
        }
    }
    pages
}

/// One test case: expected outcome, actual outcome, URL.
type Case = (String, &'static str, String);

fn main() {
    let mut args = std::env::args().skip(1);
    let cache = PathBuf::from(args.next().unwrap_or_else(|| "target/act".into()));
    let mut summary_out = None;
    let mut markdown_out = None;
    let mut urls_out = None;
    let mut auditmysite = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--summary" => summary_out = args.next().map(PathBuf::from),
            "--markdown" => markdown_out = args.next().map(PathBuf::from),
            "--urls" => urls_out = args.next().map(PathBuf::from),
            "--auditmysite" => auditmysite = args.next().map(PathBuf::from),
            _ => {}
        }
    }
    let browser_pages = auditmysite.as_deref().map(auditmysite_pages);
    let mut urls = Vec::new();
    let index: Value = serde_json::from_str(
        &fs::read_to_string(cache.join("testcases.json")).expect("run xtask/act/fetch.py first"),
    )
    .expect("testcases.json");
    let mapping: BTreeMap<&str, &[&str]> = MAPPING.iter().copied().collect();

    // rule → (name, cases: (expected, actual, url))
    let mut rules: BTreeMap<String, (String, Vec<Case>)> = BTreeMap::new();
    let mut earl = Vec::new();
    for case in index["testcases"].as_array().expect("testcases") {
        let rule = case["ruleId"].as_str().unwrap().to_string();
        let name = case["ruleName"].as_str().unwrap().to_string();
        let expected = case["expected"].as_str().unwrap().to_string();
        let url = case["url"].as_str().unwrap().to_string();
        let actual = match mapping.get(rule.as_str()) {
            None => "untested",
            // SVG, XHTML and XML files: not an HTML page.
            Some(_) if !url.ends_with(".html") && HTML_PAGE_ONLY.contains(&rule.as_str()) => {
                "inapplicable"
            }
            Some(ids) if browser_pages.is_some() => {
                urls.push(url.clone());
                match browser_pages.as_ref().and_then(|p| p.get(&url)) {
                    Some(page) => outcome_auditmysite(page, ids),
                    // No report: the page did not load in the browser.
                    None => "cantTell",
                }
            }
            Some(ids) => match fs::read_to_string(local(&cache, &url)) {
                Ok(html) => {
                    urls.push(url.clone());
                    let page = Page::parse(&html);
                    let report = page.audit(
                        &sheets(&page, &cache, &url),
                        Environment::default(),
                        Locale::En,
                    );
                    outcome(&report, ids)
                }
                Err(_) => "cantTell",
            },
        };
        if actual != "untested" {
            earl.push(json!({
                "@type": "Assertion",
                "test": { "@type": "TestCase", "title": name, "isPartOf": [rule] },
                "subject": { "@type": ["TestSubject", "WebPage"], "source": url },
                "result": { "@type": "TestResult", "outcome": format!("earl:{actual}") },
            }));
        }
        rules
            .entry(rule)
            .or_insert_with(|| (name, Vec::new()))
            .1
            .push((expected, actual, url));
    }

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_rule = Vec::new();
    for (id, (name, cases)) in &rules {
        let deprecated = name.starts_with("DEPRECATED");
        // `getConsistency` of act-rules/act-tools (see the module comment).
        let false_positive = cases
            .iter()
            .any(|(e, a, _)| e != "failed" && *a == "failed");
        let false_negative = cases
            .iter()
            .any(|(e, a, _)| e == "failed" && matches!(*a, "passed" | "inapplicable" | "untested"));
        let true_positive = cases
            .iter()
            .any(|(e, a, _)| e == "failed" && *a == "failed");
        let untested = cases.iter().any(|c| c.1 == "untested");
        let cant_tell = cases.iter().any(|c| c.1 == "cantTell");
        let inapplicable_right = cases
            .iter()
            .all(|(e, a, _)| e != "inapplicable" || matches!(*a, "passed" | "inapplicable"));
        let consistency = if cases.iter().all(|c| c.1 == "untested") {
            "untested"
        } else if false_positive {
            "not consistent"
        } else if !untested && !false_negative && true_positive {
            "consistent"
        } else if true_positive || (cant_tell && inapplicable_right) {
            "partial"
        } else {
            "not consistent"
        };
        if !deprecated {
            *counts.entry(consistency).or_default() += 1;
        }
        let wrong: Vec<_> = cases
            .iter()
            .filter(|(e, a, _)| {
                (e == "failed" && matches!(*a, "passed" | "inapplicable"))
                    || (e != "failed" && *a == "failed")
            })
            .map(|(e, a, u)| json!({ "expected": e, "actual": a, "url": u }))
            .collect();
        println!("{id} {consistency:<12} {name}");
        for w in &wrong {
            println!("    {} → {}  {}", w["expected"], w["actual"], w["url"]);
        }
        per_rule.push(json!({
            "act": id, "name": name, "deprecated": deprecated,
            "consistency": consistency, "cases": cases.len(), "wrong": wrong,
            "barrierlab": mapping.get(id.as_str()).map(|ids| ids.to_vec()),
        }));
    }
    let total: usize = counts.values().sum();
    println!("\n{total} ACT rules (deprecated ones not counted): {counts:?}");

    // Per test case, over the rules that are implemented.
    let mut cases: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, list) in rules.values() {
        if name.starts_with("DEPRECATED") {
            continue;
        }
        for (e, a, _) in list.iter().filter(|c| c.1 != "untested") {
            let verdict = match (e.as_str(), *a) {
                (_, "cantTell") => "cantTell",
                ("failed", "failed") => "correct",
                ("failed", _) | (_, "failed") => "wrong",
                _ => "correct",
            };
            *cases.entry(verdict).or_default() += 1;
        }
    }
    println!("test cases of implemented rules: {cases:?}");

    if let Some(out) = urls_out {
        fs::write(out, urls.join("\n") + "\n").expect("write URL list");
    }
    let (assertor, earl_file) = match auditmysite {
        Some(_) => (
            "auditmysite (BarrierLab rules in Chrome)",
            "earl-auditmysite.json",
        ),
        None => ("BarrierLab a11y-static", "earl.json"),
    };
    let report = json!({
        "@context": "https://act-rules.github.io/earl-context.json",
        "@graph": [{
            "@type": "Assertor",
            "name": assertor,
            "release": { "revision": env!("CARGO_PKG_VERSION") },
            "assertions": earl,
        }],
    });
    fs::write(
        cache.join(earl_file),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("write EARL report");
    if let Some(out) = summary_out {
        let summary = json!({
            "source": "https://act-rules.github.io/testcases.json",
            "host": assertor,
            "testcases": index["count"],
            "rules": total,
            "consistency": counts,
            "cases": cases,
            "perRule": per_rule,
        });
        fs::write(out, serde_json::to_string_pretty(&summary).unwrap() + "\n")
            .expect("write summary");
    }
    if let Some(out) = markdown_out {
        let mut md = String::from(
            "| ACT rule | Result | BarrierLab rules | Wrong cases |\n|---|---|---|---|\n",
        );
        for r in per_rule
            .iter()
            .filter(|r| r["consistency"] != "untested" && r["deprecated"] == false)
        {
            let ids: Vec<String> = r["barrierlab"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|v| format!("`{}`", v.as_str().unwrap()))
                .collect();
            md.push_str(&format!(
                "| [{}](https://act-rules.github.io/rules/{}) | {} | {} | {} of {} |\n",
                r["name"].as_str().unwrap(),
                r["act"].as_str().unwrap(),
                r["consistency"].as_str().unwrap(),
                ids.join(", "),
                r["wrong"].as_array().map_or(0, Vec::len),
                r["cases"],
            ));
        }
        fs::write(out, md).expect("write markdown");
    }
}
