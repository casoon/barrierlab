//! Runs the `html-conform` binary on the fixtures in `tests/cli-fixtures`
//! (the working directory, so paths in the output are stable) and compares
//! each format with its golden file in `tests/cli-golden`. Regenerate with
//! `UPDATE_GOLDEN=1 cargo test -p html-conform --features cli --test cli`.
//!
//! `sarif_validates_against_official_schema` is `#[ignore]`d: it needs the
//! OASIS SARIF 2.1.0 schema (not vendored; licence) and a Python with the
//! `jsonschema` package, see the test.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_with(args: &[&str], stdin: Option<&str>, envs: &[(&str, &str)]) -> Run {
    run_bytes(args, stdin.map(str::as_bytes), envs)
}

fn run_bytes(args: &[&str], stdin: Option<&[u8]>, envs: &[(&str, &str)]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_html-conform"));
    command
        .args(args)
        .current_dir(manifest_dir().join("tests/cli-fixtures"))
        .env_remove("NO_COLOR")
        .env_remove("TERM")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("binary starts");
    let mut pipe = child.stdin.take().expect("stdin");
    if let Some(text) = stdin {
        pipe.write_all(text).expect("write stdin");
    }
    drop(pipe);
    let output = child.wait_with_output().expect("binary finishes");
    Run {
        code: output.status.code().expect("exit code"),
        stdout: String::from_utf8(output.stdout).expect("utf-8"),
        stderr: String::from_utf8(output.stderr).expect("utf-8"),
    }
}

fn run(args: &[&str]) -> Run {
    run_with(args, None, &[])
}

fn golden(name: &str, actual: &str) {
    // The tool version changes with every release; keep it out of the golden files.
    let actual = actual.replace(
        &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
        "\"version\": \"<VERSION>\"",
    );
    let path = manifest_dir().join("tests/cli-golden").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(actual, expected, "golden {name} differs");
}

const SET: [&str; 3] = ["invalid.html", "style.html", "valid/ok.html"];

fn with(format: &str) -> Vec<&str> {
    let mut args = vec!["--format", format];
    args.extend(SET);
    args
}

#[test]
fn golden_text() {
    let result = run(&with("text"));
    assert_eq!(result.code, 1);
    golden("set.txt", &result.stdout);
}

#[test]
fn golden_json() {
    let result = run(&with("json"));
    assert_eq!(result.code, 1);
    golden("set.json", &result.stdout);
    let value: serde_json::Value = serde_json::from_str(&result.stdout).expect("valid JSON");
    assert_eq!(value["schema"], 1);
    assert!(value["summary"]["fail"].as_u64().unwrap() > 0);
}

#[test]
fn golden_sarif() {
    let result = run(&with("sarif"));
    assert_eq!(result.code, 1);
    golden("set.sarif", &result.stdout);
}

#[test]
fn sarif_is_well_formed_and_keeps_unchecked_out_of_results() {
    let result = run(&with("sarif"));
    let log: serde_json::Value = serde_json::from_str(&result.stdout).expect("valid JSON");
    assert_eq!(log["version"], "2.1.0");
    let run = &log["runs"][0];
    let results = run["results"].as_array().unwrap();
    assert!(!results.is_empty());
    let rule_ids: Vec<_> = run["tool"]["driver"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    let mut fingerprints = Vec::new();
    for result in results {
        let index = result["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(rule_ids[index], result["ruleId"]);
        assert!(["error", "warning"].contains(&result["level"].as_str().unwrap()));
        let physical = &result["locations"][0]["physicalLocation"];
        assert_eq!(physical["artifactLocation"]["uri"], "invalid.html");
        assert!(physical["region"]["startLine"].as_u64().unwrap() >= 1);
        fingerprints.push(
            result["partialFingerprints"]["htmlConform/v1"]
                .as_str()
                .unwrap(),
        );
    }
    let mut unique = fingerprints.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), fingerprints.len(), "fingerprints are unique");
    // style.html produced no result, but its unchecked CSS is a notification.
    let notes = run["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .unwrap();
    assert!(
        notes
            .iter()
            .any(|n| n["descriptor"]["id"] == "style-element-css")
    );
}

#[test]
fn golden_junit() {
    let result = run(&with("junit"));
    assert_eq!(result.code, 1);
    golden("set.xml", &result.stdout);
    assert_well_formed_xml(&result.stdout);
}

/// A minimal well-formedness check: tags nest and balance, and `&` only
/// starts one of the five predefined entities.
fn assert_well_formed_xml(xml: &str) {
    let mut stack: Vec<String> = Vec::new();
    let mut rest = xml
        .strip_prefix("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        .expect("declaration");
    while let Some(start) = rest.find('<') {
        check_entities(&rest[..start]);
        let end = rest[start..].find('>').expect("tag closes") + start;
        let tag = &rest[start + 1..end];
        assert!(!tag.contains('<'), "stray < in tag {tag}");
        check_entities(tag);
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(stack.pop().as_deref(), Some(name), "unbalanced </{name}>");
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().unwrap().to_owned());
        }
        rest = &rest[end + 1..];
    }
    assert!(stack.is_empty(), "unclosed {stack:?}");
}

fn check_entities(text: &str) {
    for (i, _) in text.match_indices('&') {
        assert!(
            ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"]
                .iter()
                .any(|e| text[i..].starts_with(e)),
            "bad entity in {text:?}"
        );
    }
}

#[test]
fn valid_file_exits_zero() {
    let result = run(&["valid/ok.html"]);
    assert_eq!(result.code, 0);
    assert!(result.stdout.contains("no findings"));
}

#[test]
fn quiet_hides_clean_files_and_summary() {
    let result = run(&["-q", "valid/ok.html"]);
    assert_eq!((result.code, result.stdout.as_str()), (0, ""));
}

#[test]
fn directory_is_searched_recursively_for_html_and_htm() {
    let result = run(&["--format", "junit", "mixed"]);
    assert_eq!(result.code, 1);
    assert!(result.stdout.contains("name=\"mixed/ok.html\""));
    assert!(result.stdout.contains("name=\"mixed/sub/bad.htm\""));
    assert!(!result.stdout.contains("notes.txt"));
}

#[test]
fn stdin_is_checked() {
    let bad = run_with(&["-"], Some("<p>x"), &[]);
    assert_eq!(bad.code, 1);
    assert!(bad.stdout.starts_with("<stdin>:"));
    let good = run_with(
        &["-"],
        Some("<!DOCTYPE html><html lang=en><title>t</title><p>x"),
        &[],
    );
    assert_eq!(good.code, 0);
}

#[test]
fn missing_file_is_exit_two_and_still_reported() {
    let result = run(&["--format", "json", "valid/ok.html", "nope.html"]);
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("nope.html"));
    let value: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(value["inputs"][0]["subject"], "nope.html");
}

#[test]
fn empty_directory_is_not_a_pass() {
    assert_eq!(run(&["empty"]).code, 2);
}

#[test]
fn usage_errors_exit_two() {
    assert_eq!(run(&[]).code, 2);
    assert_eq!(run(&["--format", "xml", "valid/ok.html"]).code, 2);
    assert_eq!(run(&["--help"]).code, 0);
}

#[test]
fn fail_on_review_counts_warnings() {
    // Only a missing-lang warning: passes by default, fails with --fail-on review.
    let html = "<!DOCTYPE html><title>t</title><p>x";
    assert_eq!(run_with(&["-"], Some(html), &[]).code, 0);
    assert_eq!(
        run_with(&["--fail-on", "review", "-"], Some(html), &[]).code,
        1
    );
}

const DOC_HEAD: &str = "<!DOCTYPE html><html lang=en><head>";

#[test]
fn encoding_is_sniffed_from_bom_then_meta_then_utf8() {
    // windows-1252 declared by <meta charset>: 0xE9 is "e acute", so the
    // document is clean. Read as UTF-8 it would be invalid.
    let mut latin1 =
        format!("{DOC_HEAD}<meta charset=iso-8859-1><title>t</title></head><body><p>caf")
            .into_bytes();
    latin1.push(0xE9);
    let result = run_bytes(&["-"], Some(&latin1), &[]);
    assert_eq!(
        (result.code, result.stderr.as_str()),
        (0, ""),
        "{}",
        result.stdout
    );

    // Shift_JIS: 0x82 0xA0 is one hiragana letter (the http-equiv form is covered by unit tests).
    let mut sjis =
        format!("{DOC_HEAD}<meta charset=shift_jis><title>t</title></head><body><p>").into_bytes();
    sjis.extend([0x82, 0xA0]);
    assert_eq!(run_bytes(&["-"], Some(&sjis), &[]).code, 0);

    // UTF-16LE with a BOM, no meta.
    let text = format!("{DOC_HEAD}<title>t</title></head><body><p>x");
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        utf16.extend(unit.to_le_bytes());
    }
    assert_eq!(run_bytes(&["-"], Some(&utf16), &[]).code, 0);

    // A BOM outranks the meta declaration, which then really disagrees: still a finding.
    let text = format!("{DOC_HEAD}<meta charset=iso-8859-1><title>t</title></head><body><p>x");
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        utf16.extend(unit.to_le_bytes());
    }
    let result = run_bytes(&["-"], Some(&utf16), &[]);
    assert_eq!(result.code, 1);
    assert!(result.stdout.contains("meta-charset-not-utf8"));
}

#[test]
fn undecodable_bytes_are_reported_in_every_format() {
    // 0xFF is never valid UTF-8; the second document is cut off inside a Shift_JIS pair.
    let utf8 = b"<!DOCTYPE html><title>t</title><p>\xFF".to_vec();
    let sjis = b"<meta charset=shift_jis><p>\x82".to_vec();
    for bytes in [&utf8, &sjis] {
        for format in ["text", "json", "sarif", "junit"] {
            let result = run_bytes(&["--format", format, "-"], Some(bytes), &[]);
            assert_eq!(result.code, 2, "{format}");
            assert!(result.stderr.contains("<stdin>: not valid"), "{format}");
            assert!(
                result.stdout.contains("not valid"),
                "{format}: {}",
                result.stdout
            );
            assert!(!result.stdout.contains("no findings"), "{format}");
        }
    }
}

fn rule_ids(run: &Run) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn disable_and_only_select_by_rule_id() {
    let all = run(&["--format", "json", "invalid.html"]);
    assert!(rule_ids(&all).iter().any(|id| id == "parser.html5"));

    let no_parser = run(&[
        "--format",
        "json",
        "--disable",
        "parser.html5",
        "invalid.html",
    ]);
    assert!(!rule_ids(&no_parser).is_empty());
    assert!(rule_ids(&no_parser).iter().all(|id| id != "parser.html5"));
    assert!(
        no_parser.stderr.contains("3 finding(s) hidden"),
        "{}",
        no_parser.stderr
    );

    // A group prefix and `--only` with a comma list.
    let only = run(&[
        "--format",
        "json",
        "--only",
        "assertion.elements,parser.html5",
        "invalid.html",
    ]);
    assert!(
        rule_ids(&only)
            .iter()
            .all(|id| id == "parser.html5" || id.starts_with("assertion.elements."))
    );

    // Hiding everything that failed leaves a clean exit code, but says so.
    let none = run(&["--only", "schema", "invalid.html"]);
    assert_eq!(none.code, 0);
    assert!(none.stderr.contains("hidden"));
}

#[test]
fn unknown_rule_selector_is_a_usage_error() {
    let result = run(&["--only", "parsr.html5", "valid/ok.html"]);
    assert_eq!(result.code, 2);
    assert!(result.stderr.contains("parsr.html5"));
}

#[test]
fn config_file_sets_options_and_the_command_line_wins() {
    let html = "<!DOCTYPE html><title>t</title><p>x";
    let with_config = run_with(&["--config", "config/disable.toml", "-"], Some(html), &[]);
    // format = json and the hidden missing-lang warning: fail-on review still passes.
    assert_eq!(with_config.code, 0, "{}", with_config.stdout);
    assert!(with_config.stdout.trim_start().starts_with('{'));
    assert!(with_config.stderr.contains("1 finding(s) hidden"));

    let overridden = run_with(
        &[
            "--config",
            "config/disable.toml",
            "--format",
            "text",
            "--disable",
            "parser.html5",
            "-",
        ],
        Some(html),
        &[],
    );
    // The command-line --disable replaces the file's list, so the warning is back.
    assert_eq!(overridden.code, 1, "{}", overridden.stdout);
    assert!(overridden.stdout.contains("html-missing-lang"));

    let excluded = run_with(
        &["--config", "config/disable.toml", "--no-config", "-"],
        Some(html),
        &[],
    );
    assert_eq!(excluded.code, 2);
}

#[test]
fn bad_or_missing_config_is_exit_two() {
    let bad = run(&["--config", "config/bad.toml", "valid/ok.html"]);
    assert_eq!(bad.code, 2);
    assert!(bad.stderr.contains("bad.toml") && bad.stderr.contains("line 1"));
    assert_eq!(
        run(&["--config", "config/none.toml", "valid/ok.html"]).code,
        2
    );
}

#[test]
fn config_file_in_the_working_directory_is_found() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-config-cwd");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("html-conform.toml"), "format = \"json\"\n").unwrap();
    let bin = |extra: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_html-conform"))
            .args(extra)
            .arg("-")
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    };
    assert!(bin(&[]).trim_start().starts_with('{'));
    assert!(!bin(&["--no-config"]).trim_start().starts_with('{'));
}

#[test]
fn color_modes() {
    let colored =
        |args: &[&str], envs: &[(&str, &str)]| run_with(args, None, envs).stdout.contains('\u{1b}');
    assert!(colored(&["--color", "always", "invalid.html"], &[]));
    assert!(!colored(&["--color", "never", "invalid.html"], &[]));
    // Not a terminal here, so auto is plain; NO_COLOR/TERM only matter on a tty.
    assert!(!colored(&["invalid.html"], &[]));
    assert!(!colored(&["invalid.html"], &[("NO_COLOR", "1")]));
}

/// Validates the SARIF output against the official 2.1.0 JSON schema.
/// Run manually:
///
/// ```text
/// curl -o "$DIR/sarif-schema-2.1.0.json" https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json
/// pip install --target "$DIR/pylib" jsonschema
/// SARIF_SCHEMA="$DIR/sarif-schema-2.1.0.json" SARIF_PYTHONPATH="$DIR/pylib" \
///   cargo test -p html-conform --features cli --test cli -- --ignored
/// ```
#[test]
#[ignore = "needs SARIF_SCHEMA (path to sarif-schema-2.1.0.json) and python3 with jsonschema"]
fn sarif_validates_against_official_schema() {
    let schema = std::env::var("SARIF_SCHEMA").expect("SARIF_SCHEMA");
    for args in [
        with("sarif"),
        vec!["--format", "sarif", "nope.html", "valid/ok.html"],
    ] {
        let sarif = run(&args).stdout;
        let mut python = Command::new("python3");
        python.args([
            "-c",
            "import json,sys,jsonschema\n\
             schema=json.load(open(sys.argv[1]))\n\
             jsonschema.validate(json.load(sys.stdin), schema)\n\
             print('valid')",
            &schema,
        ]);
        if let Ok(path) = std::env::var("SARIF_PYTHONPATH") {
            python.env("PYTHONPATH", path);
        }
        let mut child = python
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("python3 starts");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sarif.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
