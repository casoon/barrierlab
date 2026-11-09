---
title: "Command line"
description: "The html-conform binary: files, directories and stdin in, text, JSON, SARIF or JUnit out."
order: 4
---

The library is the product; the command line is a thin front end to `check()` for tools
outside the Rust ecosystem (static site generators, plain CI). It is built only with the `cli`
feature, which is off by default and adds no dependency, so people who use the crate as a
library pay nothing for it.

```sh
cargo install html-conform --features cli
```

It reads files and standard input. It never fetches a URL: fetching belongs to the host that
calls it (`curl … | html-conform -` works).

## Usage

```sh
html-conform dist/                       # every *.html and *.htm below dist/, recursively
html-conform index.html about.html       # single files, any extension
cat page.html | html-conform -           # standard input, reported as <stdin>
html-conform dist --format sarif > html-conform.sarif
```

| Option | Meaning |
|---|---|
| `--format text\|json\|sarif\|junit` | Output format, `text` by default |
| `--fail-on fail\|review` | Lowest outcome that gives exit code 1. `fail` (default): errors. `review`: errors and warnings |
| `--color auto\|always\|never` | Colour of the text format. `auto` needs a terminal, an unset or empty `NO_COLOR`, and `TERM` other than `dumb`; `always` and `never` override the environment |
| `-q`, `--quiet` | Text format: no summary and no line for clean files. Findings and notes on unchecked content stay |

Files are checked in parallel; output is sorted by path, then by line and column, so it is the
same on every run.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Nothing at or above the `--fail-on` level |
| 1 | At least one finding at or above it |
| 2 | Usage error, a path that cannot be read, a file that is not valid UTF-8, a document nested deeper than the checker validates, or a directory without HTML files |

Code 2 wins over 1: a run that could not look at everything does not get to say "findings" or
"clean". Every unchecked file is named on standard error and recorded in the chosen format.

Info-level findings (`Untested`) and classes of content the checker does not validate
(`<style>` CSS, CSP hash sources) never change the exit code, but they appear in every format.
Silence about them is not a pass.

## Formats

**text**: `path:line:column: error[rule.id] message`, then one line per class that was not
fully checked, then a summary.

**json**: the serialised [`a11y_report::Report`](https://docs.rs/a11y-report) of all files.
Every finding carries `location.file` and `location.position`. A class that was not checked,
and a file that could not be checked, are entries of `inputs` with kind `document_incomplete`
and the file as `subject`.

**sarif**: SARIF 2.1.0, valid against the official schema and meant for GitHub Code Scanning:

```yaml
- run: html-conform dist --format sarif > html-conform.sarif || true
- uses: github/codeql-action/upload-sarif@v3
  with: { sarif_file: html-conform.sarif }
```

- `error` results are findings with outcome `Fail`, `warning` results are `Review`.
- `artifactLocation.uri` is the path relative to the working directory (run it from the
  repository root), `region` holds `startLine` and `startColumn`.
- `tool.driver.rules` lists the rules that occur in the results, with a description for the
  layer; html-conform has no registry of rules that did not fire.
- `Untested` findings, unchecked content classes and unchecked files are **not** results. They
  are `toolExecutionNotifications` of the invocation, and `executionSuccessful` is `false` if a
  file could not be checked.
- `partialFingerprints["htmlConform/v1"]` is `<hash>:<n>`. `<hash>` is the 64-bit FNV-1a of
  rule id, NUL, the path, NUL and the message, in 16 hex digits. `<n>` numbers findings of one
  file with the same rule and message in position order. Line numbers are left out so an edit
  above a finding does not turn it into a new alert. Changing this derivation would reopen
  every alert, so it is versioned in the key.

**junit**: one `testsuite` per file. A `Fail` finding is a `<failure>`. `Review` and
`Untested` findings and each unchecked content class are `<skipped message="…">` testcases. A
file that could not be checked is an `<error>`. A file with no failure gets a passing
`conformance` testcase. Text is XML-escaped; characters XML 1.0 cannot carry become U+FFFD.

## Not covered

The files are read as UTF-8 only; other encodings are reported as unchecked rather than
guessed. There is no configuration file, no rule selection and no `--no-parse-errors`. Pre-built
binaries and their checksums are not published yet.
