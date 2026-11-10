//! Source ranges and the rule catalogue, black-box over the vendored corpus.
//!
//! Definition of Done rule 4: a field counts as present only when a run has
//! seen it filled, so the corpus test counts, per layer, the findings that
//! carry an end. Run with `--nocapture` to see the counts.

#![allow(deprecated)]

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use html_conform::{CheckOptions, RuleLayer, check_with_ranges, rules};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("corpus dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "html") {
            out.push(path);
        }
    }
}

fn layer_of(rule_id: &str) -> &'static str {
    match rule_id.split('.').next().unwrap_or("") {
        "parser" => "parser",
        "schema" => "schema",
        "assertion" => "assertion",
        "scripts" => "scripts",
        "csp" => "csp",
        "tables" => "tables",
        _ => "unknown",
    }
}

#[test]
fn column_counts_chars_and_line_breaks_are_one_each() {
    // Tab, "é" (2 bytes) and an emoji (4 bytes, 2 UTF-16 units) before the
    // element, on a line that follows a CRLF.
    let html = "<!DOCTYPE html>\r\n<html lang=en><title>t</title>\t\u{e9}\u{1F600}<p bogus=1>x</p>";
    let ranged = check_with_ranges(html, CheckOptions::default()).expect("checks");
    let (finding, range) = ranged
        .findings()
        .find(|(finding, _)| finding.rule_id == "schema.html5")
        .expect("bogus attribute is reported");
    let start = finding.location.expect("located");
    assert_eq!(start.line, 2);
    let prefix = "<html lang=en><title>t</title>\t\u{e9}\u{1F600}";
    assert_eq!(start.column as usize, prefix.chars().count() + 1);
    assert_ne!(start.column as usize, prefix.len() + 1, "not bytes");
    assert_ne!(
        start.column as usize,
        prefix.encode_utf16().count() + 1,
        "not UTF-16 units"
    );
    assert_eq!(
        start.byte_offset,
        "<!DOCTYPE html>\r\n".len() + prefix.len()
    );
    let range = range.expect("element finding has a start-tag range");
    assert_eq!(range.start, start);
    assert_eq!(
        &html[range.start.byte_offset..range.end.byte_offset],
        "<p bogus=1>"
    );
    assert_eq!((range.end.line, range.end.column), (2, start.column + 11));
}

#[test]
fn parse_errors_and_synthesized_elements_have_no_range() {
    let ranged = check_with_ranges(
        "<!doctype html><html lang=en><p>&notAnEntity;</p>",
        CheckOptions::default(),
    )
    .expect("checks");
    assert!(!ranged.report().findings.is_empty());
    for (finding, range) in ranged.findings() {
        if finding.rule_id == "parser.html5" || finding.location.is_none() {
            assert_eq!(range, None, "{finding:?}");
        }
    }
}

#[test]
fn corpus_findings_carry_ends_per_layer_and_ids_are_in_the_catalogue() {
    let catalogue = rules().expect("rules load");
    let known: HashSet<&str> = catalogue.iter().map(|rule| rule.id.as_str()).collect();
    assert!(
        catalogue
            .iter()
            .any(|rule| rule.layer == RuleLayer::Assertion)
    );

    let mut files = Vec::new();
    collect(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus"),
        &mut files,
    );
    files.sort();
    assert!(files.len() > 1000, "the corpus is vendored");
    // A debug build is slow; a stride keeps the default run short. The
    // release run (`--release`) covers every file.
    let stride = if cfg!(debug_assertions) { 6 } else { 1 };

    // layer -> (findings, findings with located start, findings with end)
    let mut counts: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let mut unknown = BTreeMap::<String, String>::new();
    let mut checked = 0;
    for path in files.iter().step_by(stride) {
        let html = fs::read_to_string(path).expect("UTF-8 fixture");
        let Ok(ranged) = check_with_ranges(&html, CheckOptions::default()) else {
            continue;
        };
        checked += 1;
        assert_eq!(ranged.ranges().len(), ranged.report().findings.len());
        for (finding, range) in ranged.findings() {
            if !known.contains(finding.rule_id.as_str()) {
                unknown.insert(finding.rule_id.clone(), path.display().to_string());
            }
            let entry = counts.entry(layer_of(&finding.rule_id)).or_default();
            entry.0 += 1;
            entry.1 += usize::from(finding.location.is_some());
            if let Some(range) = range {
                entry.2 += 1;
                assert_eq!(Some(range.start), finding.location, "{}", path.display());
                let tag = &html[range.start.byte_offset..range.end.byte_offset];
                assert!(
                    tag.starts_with('<') && tag.ends_with('>'),
                    "{}: {tag:?}",
                    path.display()
                );
                assert!(range.end.byte_offset > range.start.byte_offset);
            }
        }
    }
    eprintln!("checked {checked} files; layer: (findings, located, with end)");
    for (layer, (all, located, ended)) in &counts {
        eprintln!("  {layer}: {all}, {located}, {ended}");
    }
    assert!(
        unknown.is_empty(),
        "rule ids missing from the catalogue: {unknown:?}"
    );
    let ended = |layer: &str| counts.get(layer).map_or(0, |c| c.2);
    assert!(ended("schema") > 0, "schema layer fills the end");
    assert!(ended("assertion") > 0, "assertion layer fills the end");
    assert_eq!(ended("parser"), 0, "parser layer has no end position");
}
