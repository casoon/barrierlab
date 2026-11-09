//! Command-line parsing by hand: five flags do not justify a dependency.

use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: html-conform [OPTIONS] <PATH|->...

Checks HTML files for conformance. A directory is searched recursively for
*.html and *.htm files; `-` reads one document from standard input. Nothing
is fetched from the network.

Options:
  --format <text|json|sarif|junit>  Output format (default: text)
  --fail-on <fail|review>           Lowest outcome that gives exit code 1
                                    (default: fail). `fail` = errors,
                                    `review` = errors and warnings
  --color <auto|always|never>       Colour for text output (default: auto;
                                    honours NO_COLOR and TERM=dumb)
  -q, --quiet                       Text output: only findings and notes on
                                    unchecked content, no summary
  -h, --help                        Show this help
  -V, --version                     Show the version

Exit codes: 0 no findings at the --fail-on level, 1 findings at or above it,
2 usage or I/O error, or a document that could not be checked.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
    Sarif,
    Junit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailOn {
    Fail,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub inputs: Vec<Input>,
    pub format: Format,
    pub fail_on: FailOn,
    pub color: ColorMode,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Stdin,
    Path(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Run(Options),
    Help,
    Version,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut inputs = Vec::new();
    let mut format = Format::Text;
    let mut fail_on = FailOn::Fail;
    let mut color = ColorMode::Auto;
    let mut quiet = false;
    let mut only_paths = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let text = arg.to_str();
        if only_paths || text.is_none_or(|t| !t.starts_with('-') || t == "-") {
            inputs.push(if text == Some("-") {
                Input::Stdin
            } else {
                Input::Path(PathBuf::from(arg))
            });
            continue;
        }
        let text = text.unwrap_or_default();
        let (name, inline) = match text.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (text, None),
        };
        let mut value = |name: &str| -> Result<String, String> {
            match inline.clone() {
                Some(value) => Ok(value),
                None => args
                    .next()
                    .and_then(|v| v.into_string().ok())
                    .ok_or_else(|| format!("{name} needs a value")),
            }
        };
        match name {
            "--" => only_paths = true,
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-q" | "--quiet" => quiet = true,
            "--format" => {
                format = match value(name)?.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    "sarif" => Format::Sarif,
                    "junit" => Format::Junit,
                    other => return Err(format!("unknown --format `{other}`")),
                }
            }
            "--fail-on" => {
                fail_on = match value(name)?.as_str() {
                    "fail" => FailOn::Fail,
                    "review" => FailOn::Review,
                    other => return Err(format!("unknown --fail-on `{other}`")),
                }
            }
            "--color" => {
                color = match value(name)?.as_str() {
                    "auto" => ColorMode::Auto,
                    "always" => ColorMode::Always,
                    "never" => ColorMode::Never,
                    other => return Err(format!("unknown --color `{other}`")),
                }
            }
            _ => return Err(format!("unknown option `{text}`")),
        }
    }
    if inputs.is_empty() {
        return Err("no input: give files, directories or `-`".to_owned());
    }
    Ok(Command::Run(Options {
        inputs,
        format,
        fail_on,
        color,
        quiet,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn defaults_and_inputs() {
        let Ok(Command::Run(options)) = run(&["a.html", "-", "dir"]) else {
            panic!("expected Run");
        };
        assert_eq!(options.format, Format::Text);
        assert_eq!(options.fail_on, FailOn::Fail);
        assert_eq!(options.color, ColorMode::Auto);
        assert_eq!(
            options.inputs,
            [
                Input::Path("a.html".into()),
                Input::Stdin,
                Input::Path("dir".into())
            ]
        );
    }

    #[test]
    fn flags_with_space_and_equals() {
        let Ok(Command::Run(options)) = run(&[
            "--format=sarif",
            "--fail-on",
            "review",
            "-q",
            "--color",
            "never",
            "x",
        ]) else {
            panic!("expected Run");
        };
        assert_eq!(options.format, Format::Sarif);
        assert_eq!(options.fail_on, FailOn::Review);
        assert_eq!(options.color, ColorMode::Never);
        assert!(options.quiet);
    }

    #[test]
    fn double_dash_ends_options() {
        let Ok(Command::Run(options)) = run(&["--", "-weird.html"]) else {
            panic!("expected Run");
        };
        assert_eq!(options.inputs, [Input::Path("-weird.html".into())]);
    }

    #[test]
    fn usage_errors() {
        assert!(run(&[]).is_err());
        assert!(run(&["--format", "xml", "a"]).is_err());
        assert!(run(&["--bogus", "a"]).is_err());
        assert!(run(&["a", "--format"]).is_err());
        assert_eq!(run(&["--help"]), Ok(Command::Help));
    }
}
