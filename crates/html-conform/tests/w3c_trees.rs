//! Opt-in measurement against the two vnu test trees that are not vendored
//! because their files are under the W3C Test Suite License (issues #174,
//! #175): `tests/html-svg` (522 files) and `tests/html-its` (90 `.html`).
//!
//! The trees are fetched on demand by `xtask/fetch-w3c-trees.sh` into the
//! gitignored `xtask/.cache/w3c-trees/`. This is a measurement, not a gate:
//! it prints numbers and asserts nothing about them.
//!
//! - **Cache missing:** prints "not run: cache missing" and returns. That is
//!   "did not run", not "passed". With `W3C_TREES_REQUIRED=1` it fails instead.
//! - **html-svg:** expectation as in `differential.rs`: a file listed in
//!   vnu's `messages.json` expects findings, any other file expects none.
//! - **html-its:** vnu has no expected-result file for it. The per-file
//!   verdict (clean vs. findings) comes from the vnu jar at
//!   `xtask/.cache/vnu.jar` (SHA-256 pinned, as in `compare-real-world.sh`);
//!   without a jar or Java the vnu side is reported as NotRun.
//!
//! Run: `cargo test --release -p html-conform --test w3c_trees -- --ignored --nocapture`

// Reads the deprecated `Finding::rule_id`, like `differential.rs`.
#![allow(deprecated)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use html_conform::check;

/// vnu 26.8.20, same pin as `xtask/compare-real-world.sh`.
const VNU_JAR_SHA256: &str = "582b3506ab5bb76a7cb7c78930c427720856e79497084f69847a4ebf3f9d217f";

/// The harness-only missing-`lang` exception, see `differential.rs`.
const MISSING_LANG_RULE_ID: &str = "assertion.elements.html-missing-lang";

fn xtask_cache() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("xtask/.cache")
}

fn html_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in
        fs::read_dir(dir).unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            out.extend(html_files(&path));
        } else if path.extension().is_some_and(|x| x == "html") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Full path as text; vnu reports `file:/<this path>` as the message url.
fn name(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A working `java`: `$JAVA`, `java`, or Homebrew's openjdk.
fn find_java() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(java) = std::env::var("JAVA") {
        candidates.push(java);
    }
    candidates.push("java".to_string());
    if let Ok(out) = Command::new("brew").args(["--prefix", "openjdk"]).output()
        && out.status.success()
    {
        let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
        candidates.push(format!("{prefix}/bin/java"));
    }
    candidates.into_iter().find(|java| {
        Command::new(java)
            .arg("-version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

fn sha256_of(path: &Path) -> Option<String> {
    for (tool, args) in [("sha256sum", vec![]), ("shasum", vec!["-a", "256"])] {
        if let Ok(out) = Command::new(tool).args(&args).arg(path).output()
            && out.status.success()
        {
            return String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .next()
                .map(str::to_string);
        }
    }
    None
}

/// Per file: does vnu report any message? `Err` carries the NotRun reason.
fn vnu_verdicts(files: &[PathBuf]) -> Result<BTreeMap<String, bool>, String> {
    let jar = xtask_cache().join("vnu.jar");
    if !jar.is_file() {
        return Err("xtask/.cache/vnu.jar missing".into());
    }
    match sha256_of(&jar) {
        Some(sum) if sum == VNU_JAR_SHA256 => {}
        Some(sum) => {
            return Err(format!(
                "vnu.jar has sha256 {sum}, expected {VNU_JAR_SHA256}"
            ));
        }
        None => return Err("no sha256sum/shasum to verify vnu.jar".into()),
    }
    let java = find_java().ok_or("no working java")?;
    let output = Command::new(java)
        .arg("-jar")
        .arg(&jar)
        .args(["--format", "json", "--stdout"])
        .args(files)
        .output()
        .map_err(|e| format!("could not start java: {e}"))?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("vnu output is not JSON: {e}"))?;
    let mut with_messages = BTreeSet::new();
    for message in json["messages"]
        .as_array()
        .ok_or("vnu JSON has no messages")?
    {
        if let Some(url) = message["url"].as_str() {
            with_messages.insert(url.strip_prefix("file:").unwrap_or(url).to_string());
        }
    }
    Ok(files
        .iter()
        .map(|f| (name(f), with_messages.contains(&name(f))))
        .collect())
}

#[test]
#[ignore = "measurement, needs xtask/fetch-w3c-trees.sh; run with `cargo test --release -p html-conform --test w3c_trees -- --ignored --nocapture`"]
fn measure_w3c_trees_from_cache() {
    let root = xtask_cache().join("w3c-trees");
    if !root.join("html-svg").is_dir() || !root.join("html-its").is_dir() {
        let reason =
            "not run: cache missing (xtask/.cache/w3c-trees/; run xtask/fetch-w3c-trees.sh)";
        if std::env::var("W3C_TREES_REQUIRED").is_ok_and(|v| v == "1") {
            panic!("{reason}");
        }
        println!("{reason}");
        return;
    }

    measure_svg(&root);
    measure_its(&root);
}

fn measure_svg(root: &Path) {
    let raw = fs::read_to_string(root.join("messages.json")).expect("messages.json in cache");
    let expected: BTreeMap<String, String> =
        serde_json::from_str(&raw).expect("messages.json should be valid JSON");
    let files = html_files(&root.join("html-svg"));
    let (mut tp, mut tn, mut fp, mut fneg, mut err) = (0, 0, 0, 0, 0);
    let mut fp_list = Vec::new();
    let mut fn_list = Vec::new();
    for file in &files {
        let key = format!("html-svg/{}", file.file_name().unwrap().to_string_lossy());
        let html = fs::read_to_string(file).expect("readable fixture");
        let Ok(report) = check(&html) else {
            err += 1;
            continue;
        };
        let findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule_id != MISSING_LANG_RULE_ID)
            .collect();
        match (expected.get(&key), findings.is_empty()) {
            (Some(_), false) => tp += 1,
            (None, true) => tn += 1,
            (None, false) => {
                fp += 1;
                let ids: BTreeSet<_> = findings.iter().map(|f| f.rule_id.as_str()).collect();
                fp_list.push(format!(
                    "{key}: {ids:?} e.g. {}",
                    findings[0].message.chars().take(100).collect::<String>()
                ));
            }
            (Some(message), true) => {
                fneg += 1;
                fn_list.push(format!("{key}: vnu says {message}"));
            }
        }
    }
    println!(
        "html-svg ({} files): true_positive={tp} true_negative={tn} false_positive={fp} \
         false_negative={fneg} not_comparable={err}",
        files.len()
    );
    for line in &fp_list {
        println!("html-svg FP {line}");
    }
    for line in &fn_list {
        println!("html-svg FN {line}");
    }
}

fn measure_its(root: &Path) {
    let files = html_files(&root.join("html-its"));
    let vnu = vnu_verdicts(&files);
    let mut agree = 0;
    let mut disagree = Vec::new();
    let (mut ours_with, mut err) = (0, 0);
    for file in &files {
        let html = fs::read_to_string(file).expect("readable fixture");
        let Ok(report) = check(&html) else {
            err += 1;
            continue;
        };
        let ours = !report.findings.is_empty();
        ours_with += usize::from(ours);
        if let Ok(verdicts) = &vnu {
            if verdicts[&name(file)] == ours {
                agree += 1;
            } else {
                disagree.push(format!(
                    "{} (vnu={}, ours={ours})",
                    name(file),
                    verdicts[&name(file)]
                ));
            }
        }
    }
    match &vnu {
        Ok(verdicts) => {
            let vnu_with = verdicts.values().filter(|v| **v).count();
            println!(
                "html-its ({} .html): html-conform findings in {ours_with} files, vnu findings in \
                 {vnu_with} files, per-file agreement (clean vs. findings) {agree} of {} \
                 not_comparable={err}",
                files.len(),
                files.len() - err
            );
            for line in &disagree {
                println!("html-its DISAGREE {line}");
            }
        }
        Err(reason) => println!(
            "html-its ({} .html): html-conform findings in {ours_with} files; vnu side NotRun: {reason}",
            files.len()
        ),
    }
}
