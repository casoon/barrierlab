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
  --disable <id>                    Hide findings of this rule (repeatable,
                                    or comma-separated). `parser.html5` hides
                                    that rule, `assertion.aria` the whole
                                    group `assertion.aria.*`
  --only <id>                       Show only findings of these rules
  --config <file>                   Read options from this file instead of
                                    ./html-conform.toml
  --no-config                       Ignore ./html-conform.toml
  --list-rules                      Print the rules html-conform can report as
                                    JSON and exit
  -h, --help                        Show this help
  -V, --version                     Show the version

The config file holds the keys format, fail-on, color, quiet, disable and
only as flat `key = value` lines (`disable = [\"a\", \"b\"]`); options on the
command line win. Rule selection hides findings after the check; the number
of hidden findings is reported on standard error.

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

/// What the command line said; `None` means "not given", so the config file
/// or the default decides.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub inputs: Vec<Input>,
    pub format: Option<Format>,
    pub fail_on: Option<FailOn>,
    pub color: Option<ColorMode>,
    pub quiet: bool,
    pub disable: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
    pub config: Option<PathBuf>,
    pub no_config: bool,
}

impl Format {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            "junit" => Ok(Self::Junit),
            other => Err(format!("unknown format `{other}`")),
        }
    }
}

impl FailOn {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fail" => Ok(Self::Fail),
            "review" => Ok(Self::Review),
            other => Err(format!("unknown fail-on `{other}`")),
        }
    }
}

impl ColorMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => Err(format!("unknown color `{other}`")),
        }
    }
}

/// Splits `a,b` and trims; used by `--disable` and `--only`.
pub fn split_ids(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect()
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
    ListRules,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut options = Options::default();
    let mut only_paths = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let text = arg.to_str();
        if only_paths || text.is_none_or(|t| !t.starts_with('-') || t == "-") {
            options.inputs.push(if text == Some("-") {
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
            "--list-rules" => return Ok(Command::ListRules),
            "-q" | "--quiet" => options.quiet = true,
            "--no-config" => options.no_config = true,
            "--format" => options.format = Some(Format::parse(&value(name)?)?),
            "--fail-on" => options.fail_on = Some(FailOn::parse(&value(name)?)?),
            "--color" => options.color = Some(ColorMode::parse(&value(name)?)?),
            "--disable" => options
                .disable
                .get_or_insert_with(Vec::new)
                .extend(split_ids(&value(name)?)),
            "--only" => options
                .only
                .get_or_insert_with(Vec::new)
                .extend(split_ids(&value(name)?)),
            "--config" => options.config = Some(PathBuf::from(value(name)?)),
            _ => return Err(format!("unknown option `{text}`")),
        }
    }
    if options.inputs.is_empty() {
        return Err("no input: give files, directories or `-`".to_owned());
    }
    if options.no_config && options.config.is_some() {
        return Err("--config and --no-config exclude each other".to_owned());
    }
    Ok(Command::Run(options))
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
        assert_eq!(options.format, None);
        assert_eq!(options.fail_on, None);
        assert_eq!(options.color, None);
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
            "--disable",
            "a.b,c",
            "--only=d",
            "--config",
            "c.toml",
            "x",
        ]) else {
            panic!("expected Run");
        };
        assert_eq!(options.format, Some(Format::Sarif));
        assert_eq!(options.fail_on, Some(FailOn::Review));
        assert_eq!(options.color, Some(ColorMode::Never));
        assert!(options.quiet);
        assert_eq!(
            options.disable,
            Some(vec!["a.b".to_owned(), "c".to_owned()])
        );
        assert_eq!(options.only, Some(vec!["d".to_owned()]));
        assert_eq!(options.config, Some(PathBuf::from("c.toml")));
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
        assert!(run(&["--config", "c", "--no-config", "a"]).is_err());
        assert_eq!(run(&["--help"]), Ok(Command::Help));
    }
}
