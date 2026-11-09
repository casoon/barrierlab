//! The config file and the settings the run uses. The file is a flat subset
//! of TOML written by hand (`key = value`, `key = ["a", "b"]`, `#` comments):
//! six keys do not justify a TOML dependency. Command-line options win.

use std::path::Path;

use crate::args::{ColorMode, FailOn, Format, Input, Options};

pub const CONFIG_NAME: &str = "html-conform.toml";

/// The first segment of every rule id the checker emits (docs/html-conform/
/// guides/rule-ids.md). A selector with another first segment is a typo and
/// would otherwise hide or keep nothing without a word.
const RULE_FAMILIES: [&str; 6] = ["parser", "schema", "assertion", "scripts", "csp", "tables"];

pub struct Settings {
    pub inputs: Vec<Input>,
    pub format: Format,
    pub fail_on: FailOn,
    pub color: ColorMode,
    pub quiet: bool,
    pub selection: Selection,
}

/// Which findings are shown. A selector is a rule id or a prefix of one that
/// ends at a `.`: `assertion.aria` covers `assertion.aria.hidden-not-focusable`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub only: Vec<String>,
    pub disable: Vec<String>,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.only.is_empty() && self.disable.is_empty()
    }

    pub fn keeps(&self, rule_id: &str) -> bool {
        let covers = |selector: &String| {
            rule_id == selector
                || rule_id
                    .strip_prefix(selector.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
        };
        (self.only.is_empty() || self.only.iter().any(covers)) && !self.disable.iter().any(covers)
    }

    fn validate(&self) -> Result<(), String> {
        for selector in self.only.iter().chain(&self.disable) {
            let family = selector.split('.').next().unwrap_or_default();
            if !RULE_FAMILIES.contains(&family) || selector.ends_with('.') {
                return Err(format!(
                    "unknown rule `{selector}`: ids start with one of {}",
                    RULE_FAMILIES.join(", ")
                ));
            }
        }
        Ok(())
    }
}

/// Values from the config file; `None` means the key is absent.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct File {
    format: Option<Format>,
    fail_on: Option<FailOn>,
    color: Option<ColorMode>,
    quiet: Option<bool>,
    disable: Option<Vec<String>>,
    only: Option<Vec<String>>,
}

/// Merges the command line over the config file over the defaults. The file
/// is `--config`, else `html-conform.toml` in the working directory if there
/// is one, unless `--no-config`.
pub fn resolve(options: Options) -> Result<Settings, String> {
    let file = match (&options.config, options.no_config) {
        (_, true) => File::default(),
        (Some(path), false) => load(path)?,
        (None, false) => {
            let path = Path::new(CONFIG_NAME);
            if path.is_file() {
                load(path)?
            } else {
                File::default()
            }
        }
    };
    let selection = Selection {
        only: options.only.or(file.only).unwrap_or_default(),
        disable: options.disable.or(file.disable).unwrap_or_default(),
    };
    selection.validate()?;
    Ok(Settings {
        inputs: options.inputs,
        format: options.format.or(file.format).unwrap_or(Format::Text),
        fail_on: options.fail_on.or(file.fail_on).unwrap_or(FailOn::Fail),
        color: options.color.or(file.color).unwrap_or(ColorMode::Auto),
        quiet: options.quiet || file.quiet.unwrap_or(false),
        selection,
    })
}

fn load(path: &Path) -> Result<File, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read config {}: {err}", path.display()))?;
    parse(&text).map_err(|err| format!("{}: {err}", path.display()))
}

pub fn parse(text: &str) -> Result<File, String> {
    let mut file = File::default();
    let mut lines = text.lines().enumerate();
    while let Some((index, raw)) = lines.next() {
        let at = |message: String| format!("line {}: {message}", index + 1);
        let mut line = strip_comment(raw).trim().to_owned();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(at("expected `key = value`".to_owned()));
        };
        let key = key.trim().to_owned();
        let mut value = value.trim().to_owned();
        // A list may continue over several lines until its `]`.
        while value.starts_with('[') && !value.ends_with(']') {
            let Some((_, more)) = lines.next() else {
                return Err(at("list is not closed with `]`".to_owned()));
            };
            value.push(' ');
            value.push_str(strip_comment(more).trim());
        }
        line.clear();
        let slot_error = |key: &str| at(format!("`{key}` is given twice"));
        match key.as_str() {
            "format" => {
                if file
                    .format
                    .replace(Format::parse(&string(&value)?).map_err(at)?)
                    .is_some()
                {
                    return Err(slot_error(&key));
                }
            }
            "fail-on" => {
                if file
                    .fail_on
                    .replace(FailOn::parse(&string(&value)?).map_err(at)?)
                    .is_some()
                {
                    return Err(slot_error(&key));
                }
            }
            "color" => {
                if file
                    .color
                    .replace(ColorMode::parse(&string(&value)?).map_err(at)?)
                    .is_some()
                {
                    return Err(slot_error(&key));
                }
            }
            "quiet" => {
                let flag = match value.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(at("`quiet` is `true` or `false`".to_owned())),
                };
                if file.quiet.replace(flag).is_some() {
                    return Err(slot_error(&key));
                }
            }
            "disable" | "only" => {
                let list = list(&value).map_err(at)?;
                let slot = if key == "disable" {
                    &mut file.disable
                } else {
                    &mut file.only
                };
                if slot.replace(list).is_some() {
                    return Err(slot_error(&key));
                }
            }
            other => {
                return Err(at(format!(
                    "unknown key `{other}` (keys: format, fail-on, color, quiet, disable, only)"
                )));
            }
        }
    }
    Ok(file)
}

/// Drops a `#` comment that is not inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (at, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..at],
            _ => {}
        }
    }
    line
}

/// A bare word or a `"quoted"` string.
fn string(value: &str) -> Result<String, String> {
    let inner = match value.strip_prefix('"') {
        Some(rest) => rest
            .strip_suffix('"')
            .ok_or_else(|| "string is not closed".to_owned())?,
        None => value,
    };
    Ok(inner.to_owned())
}

fn list(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| "expected a list like [\"a\", \"b\"]".to_owned())?;
    let mut ids = Vec::new();
    for item in inner.split(',').map(str::trim).filter(|i| !i.is_empty()) {
        ids.push(string(item)?);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_lists_and_comments() {
        let file = parse(
            "# top\nformat = \"json\"\nfail-on = review  # trailing\nquiet = true\n\
             disable = [\"parser.html5\",\n  \"assertion.aria\", # two\n]\nonly = []\n",
        )
        .unwrap();
        assert_eq!(file.format, Some(Format::Json));
        assert_eq!(file.fail_on, Some(FailOn::Review));
        assert_eq!(file.quiet, Some(true));
        assert_eq!(
            file.disable,
            Some(vec!["parser.html5".to_owned(), "assertion.aria".to_owned()])
        );
        assert_eq!(file.only, Some(vec![]));
    }

    #[test]
    fn rejects_what_it_does_not_know() {
        assert!(parse("colour = never").unwrap_err().contains("line 1"));
        assert!(parse("format = xml").is_err());
        assert!(parse("quiet = yes").is_err());
        assert!(parse("disable = [\"a\"").is_err());
        assert!(parse("format = text\nformat = json").is_err());
        assert!(parse("just words").is_err());
    }

    #[test]
    fn selectors_match_at_dot_boundaries() {
        let only = |s: &[&str]| Selection {
            only: s.iter().map(|s| (*s).to_owned()).collect(),
            disable: vec![],
        };
        let sel = only(&["assertion.aria"]);
        assert!(sel.keeps("assertion.aria.hidden-not-focusable"));
        assert!(sel.keeps("assertion.aria"));
        assert!(!sel.keeps("assertion.ariax.y"));
        assert!(!sel.keeps("parser.html5"));
        let sel = Selection {
            only: vec![],
            disable: vec!["parser.html5".to_owned()],
        };
        assert!(!sel.keeps("parser.html5"));
        assert!(sel.keeps("schema.html5"));
    }

    #[test]
    fn unknown_rule_families_are_an_error() {
        let sel = Selection {
            only: vec!["parsr.html5".to_owned()],
            disable: vec![],
        };
        assert!(sel.validate().unwrap_err().contains("parsr.html5"));
    }
}
