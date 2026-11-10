//! `html-conform`: the command-line front end of the library.
//!
//! Built only with the `cli` feature. It reads files and standard input and
//! never fetches a URL; everything else is `html_conform::check`.

mod args;
mod config;
mod encoding;
mod input;
mod junit;
mod output;
mod sarif;

use std::io::Write;
use std::process::ExitCode;

use a11y_report::Outcome;
use args::{Command, FailOn, Format};
use config::Settings;
use input::FileResult;

fn main() -> ExitCode {
    let options = match args::parse(std::env::args_os().skip(1)) {
        Ok(Command::Run(options)) => options,
        Ok(Command::Help) => {
            println!("{}", args::USAGE);
            return ExitCode::SUCCESS;
        }
        Ok(Command::ListRules) => {
            return match html_conform::rules() {
                Ok(rules) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&rules).expect("rules serialise")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("html-conform: {error}");
                    ExitCode::from(2)
                }
            };
        }
        Ok(Command::Version) => {
            println!("html-conform {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("html-conform: {message}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    match config::resolve(options) {
        Ok(settings) => ExitCode::from(run(&settings)),
        Err(message) => {
            eprintln!("html-conform: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(options: &Settings) -> u8 {
    let mut results = input::check_all(&options.inputs);
    let hidden = input::apply_selection(&mut results, &options.selection);
    if hidden > 0 {
        eprintln!("html-conform: note: {hidden} finding(s) hidden by --only/--disable");
    }
    let rendered = match options.format {
        Format::Text => output::text(
            &results,
            options.quiet,
            output::color_enabled(options.color),
        ),
        Format::Json => output::json(&results),
        Format::Sarif => sarif::render(&results),
        Format::Junit => junit::render(&results),
    };
    // Files that were not checked are named on stderr in every format.
    for result in &results {
        if let Some(reason) = output::not_checked_reason(result) {
            eprintln!("html-conform: error: {}: {reason}", result.path);
        }
    }
    if std::io::stdout().write_all(rendered.as_bytes()).is_err() {
        return 2;
    }
    exit_code(&results, options.fail_on)
}

fn exit_code(results: &[FileResult], fail_on: FailOn) -> u8 {
    if results
        .iter()
        .any(|r| output::not_checked_reason(r).is_some())
    {
        return 2;
    }
    let reaches = |outcome: Outcome| match fail_on {
        FailOn::Fail => outcome == Outcome::Fail,
        FailOn::Review => matches!(outcome, Outcome::Fail | Outcome::Review),
    };
    u8::from(
        results
            .iter()
            .flat_map(output::findings)
            .any(|f| reaches(f.outcome)),
    )
}
