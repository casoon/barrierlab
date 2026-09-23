//! Reads one HTML file, runs `html_conform::check` on it, and prints one
//! JSON object per finding to stdout (`rule_id`, `severity`, `line`,
//! `column`, `message`). Used by `xtask/compare-real-world.sh` to feed
//! html-conform's findings into a comparison against a locally running vnu.

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: check-file <path-to-html-file>");
        return ExitCode::FAILURE;
    };

    let html = match fs::read_to_string(&path) {
        Ok(html) => html,
        Err(err) => {
            eprintln!("error: failed to read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    match html_conform::check(&html) {
        Ok(report) => {
            for finding in &report.findings {
                let (line, column) = match finding.location {
                    Some(location) => (Some(location.line), Some(location.column)),
                    None => (None, None),
                };
                let json = serde_json::json!({
                    "rule_id": finding.rule_id,
                    "severity": finding.severity,
                    "line": line,
                    "column": column,
                    "message": finding.message,
                });
                println!("{json}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
