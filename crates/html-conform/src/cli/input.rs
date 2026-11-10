//! Turns command-line inputs into the list of documents to check, and checks
//! them. Reads files and standard input only; it never opens a socket.

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use a11y_report::{Report, SourcePosition};

use crate::args::Input;
use crate::config::Selection;

pub const STDIN_NAME: &str = "<stdin>";

/// The outcome for one document: a report, or the reason it was not checked.
pub struct FileResult {
    pub path: String,
    pub state: State,
}

pub enum State {
    Checked(Report),
    NotChecked(String),
}

impl FileResult {
    fn not_checked(path: String, reason: impl Into<String>) -> Self {
        Self {
            path,
            state: State::NotChecked(reason.into()),
        }
    }

    pub fn report(&self) -> Option<&Report> {
        match &self.state {
            State::Checked(report) => Some(report),
            State::NotChecked(_) => None,
        }
    }
}

enum Source {
    Stdin,
    File { path: PathBuf, display: String },
}

/// Collects, checks and returns the results sorted by displayed path.
pub fn check_all(inputs: &[Input]) -> Vec<FileResult> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut results = Vec::new();
    let mut sources = Vec::new();
    for input in inputs {
        match input {
            Input::Stdin => sources.push(Source::Stdin),
            Input::Path(path) => collect(path, &cwd, &mut sources, &mut results),
        }
    }
    sources.sort_by_key(display_of);
    sources.dedup_by(|a, b| display_of(a) == display_of(b));

    let slots: Vec<Mutex<Option<FileResult>>> = sources.iter().map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(sources.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(source) = sources.get(index) else {
                        break;
                    };
                    let result = check_source(source);
                    *slots[index].lock().expect("no panicking holder") = Some(result);
                }
            });
        }
    });
    results.extend(
        slots
            .into_iter()
            .filter_map(|slot| slot.into_inner().expect("no panicking holder")),
    );
    results.sort_by(|a, b| a.path.cmp(&b.path));
    results
}

/// Removes the findings the selection does not keep; returns how many.
pub fn apply_selection(results: &mut [FileResult], selection: &Selection) -> usize {
    if selection.is_empty() {
        return 0;
    }
    let mut hidden = 0;
    for result in results {
        if let State::Checked(report) = &mut result.state {
            let before = report.findings.len();
            report.findings.retain(|f| selection.keeps(&f.rule_id));
            hidden += before - report.findings.len();
        }
    }
    hidden
}

fn display_of(source: &Source) -> String {
    match source {
        Source::Stdin => STDIN_NAME.to_owned(),
        Source::File { display, .. } => display.clone(),
    }
}

fn collect(path: &Path, cwd: &Path, sources: &mut Vec<Source>, errors: &mut Vec<FileResult>) {
    let display = display_path(path, cwd);
    match fs::metadata(path) {
        Err(err) => errors.push(FileResult::not_checked(display, err.to_string())),
        Ok(meta) if meta.is_file() => sources.push(Source::File {
            path: path.to_owned(),
            display,
        }),
        Ok(_) => {
            let before = sources.len();
            walk(path, cwd, sources, errors);
            if sources.len() == before {
                errors.push(FileResult::not_checked(
                    display,
                    "no .html or .htm files found",
                ));
            }
        }
    }
}

fn walk(dir: &Path, cwd: &Path, sources: &mut Vec<Source>, errors: &mut Vec<FileResult>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(FileResult::not_checked(
                display_path(dir, cwd),
                err.to_string(),
            ));
            return;
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        // Symlinked directories are not followed, which rules out loops.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            walk(&path, cwd, sources, errors);
        } else if is_html(&path) && fs::metadata(&path).is_ok_and(|m| m.is_file()) {
            let display = display_path(&path, cwd);
            sources.push(Source::File { path, display });
        }
    }
}

fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}

/// The path relative to the working directory when it lies below it, with
/// `/` separators and no `.` components.
pub fn display_path(path: &Path, cwd: &Path) -> String {
    let relative = path.strip_prefix(cwd).unwrap_or(path);
    let parts: Vec<String> = relative
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let joined = parts.join("/");
    if joined.starts_with("//") {
        joined[1..].to_owned()
    } else {
        joined
    }
}

fn check_source(source: &Source) -> FileResult {
    let display = display_of(source);
    let bytes = match source {
        Source::Stdin => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .lock()
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        }
        Source::File { path, .. } => fs::read(path),
    };
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(err) => return FileResult::not_checked(display, err.to_string()),
    };
    let html = match crate::encoding::decode(&bytes) {
        Ok(html) => html,
        Err(reason) => return FileResult::not_checked(display, reason),
    };
    match html_conform::check(&html) {
        Ok(checked) => {
            let mut report = checked.to_report();
            for finding in &mut report.findings {
                finding.location.file = Some(display.clone());
            }
            report.findings.sort_by(|a, b| {
                let key = |f: &a11y_report::Finding| {
                    f.location
                        .position
                        .map_or((0, 0), |SourcePosition { line, column, .. }| (line, column))
                };
                key(a)
                    .cmp(&key(b))
                    .then_with(|| a.rule_id.cmp(&b.rule_id))
                    .then_with(|| a.message.cmp(&b.message))
            });
            FileResult {
                path: display,
                state: State::Checked(report),
            }
        }
        Err(err) => FileResult::not_checked(display, err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_is_relative_and_slash_separated() {
        let cwd = Path::new("/work/site");
        assert_eq!(display_path(Path::new("./a/b.html"), cwd), "a/b.html");
        assert_eq!(
            display_path(Path::new("/work/site/a/b.html"), cwd),
            "a/b.html"
        );
        assert_eq!(
            display_path(Path::new("/elsewhere/x.html"), cwd),
            "/elsewhere/x.html"
        );
    }

    #[test]
    fn html_extensions() {
        assert!(is_html(Path::new("a.HTML")));
        assert!(is_html(Path::new("a.htm")));
        assert!(!is_html(Path::new("a.txt")));
        assert!(!is_html(Path::new("html")));
    }
}
