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

Prebuilt binaries for Linux (x86_64, aarch64; musl), macOS (x86_64, aarch64) and Windows
(x86_64) are attached to the GitHub releases tagged `html-conform-vX.Y.Z`, with `SHA256SUMS` and
a build provenance attestation; see the README of the crate for how to verify them.

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
| `--disable <rule>` | Hide findings of a rule. Repeatable, or comma-separated. See [Selecting rules](#selecting-rules-and-the-config-file) |
| `--only <rule>` | Show only findings of these rules. Repeatable, or comma-separated |
| `--config <file>` | Read the options from this file instead of `html-conform.toml` in the working directory |
| `--list-rules` | Print the rules html-conform can report (id, layer, outcome, description, source) as JSON and exit |
| `--no-config` | Ignore `html-conform.toml` |

Files are checked in parallel; output is sorted by path, then by line and column, so it is the
same on every run.

## Encodings

A file is decoded the way a browser does it without a transport charset (WHATWG encoding
sniffing): a byte order mark (UTF-8, UTF-16LE, UTF-16BE) first, then a `<meta charset>` or
`<meta http-equiv="content-type">` in the first 1024 bytes (a declared UTF-16 means UTF-8, as in
the standard), else UTF-8. There is no HTTP header to consult, so nothing outranks the file.
Standard input is treated the same.

Bytes that are not valid in the encoding found are not replaced with U+FFFD and not checked
anyway: the file is reported as not checked, exit code 2, in every format. A file in
another encoding is decoded as the one it declares, so the other findings are about the real
text. HTML requires UTF-8, though, so the declaration itself is reported as
`assertion.elements.meta-charset-not-utf8`, as vnu does; the finding is not suppressed.

## Selecting rules and the config file

Findings carry stable rule IDs ([Findings and rule IDs](../rule-ids/)). A selector is an ID or a
prefix that ends at a dot: `parser.html5` is one rule, `assertion.elements` every
`assertion.elements.*`, `assertion` every Schematron rule. A selector whose first part is not a
known layer (`parser`, `schema`, `assertion`, `scripts`, `csp`, `tables`) is a usage error
rather than a rule that silently matches nothing.

`--disable` removes the findings it matches, `--only` keeps only those it matches; with both,
a finding must pass both. The check still runs in full; the findings are removed from the
result afterwards, and the number removed is written to standard error. The exit code is
computed from what remains. Hiding is a decision of the person who runs the tool, so it is not
a "not checked" state, but it is never silent.

`html-conform.toml` in the working directory (only there, no search upwards) or `--config`
holds the same options as flat `key = value` lines, in a small subset of TOML written by hand:

```toml
# html-conform.toml
format = "sarif"                       # text, json, sarif, junit
fail-on = "review"                     # fail, review
color = "never"                        # auto, always, never
quiet = false
disable = ["assertion.elements.html-missing-lang", "scripts"]
only = []                              # lists may span lines
```

Command-line options win over the file; `--disable` and `--only` on the command line replace
the file's list instead of adding to it. An unknown key, a malformed line, a file given with
`--config` that cannot be read, or a file given together with `--no-config` is exit code 2,
with the line number.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Nothing at or above the `--fail-on` level |
| 1 | At least one finding at or above it |
| 2 | Usage error, a path or config file that cannot be read or parsed, a file whose bytes are not valid in the encoding found, a document nested deeper than the checker validates, or a directory without HTML files |

Code 2 wins over 1: a run that could not look at everything does not get to say "findings" or
"clean". Every unchecked file is named on standard error and recorded in the chosen format.

Info-level findings (`Untested`) and classes of content the checker does not validate
(`<style>` CSS, CSP hash sources) never change the exit code, but they appear in every format.
Silence about them is not a pass.

## Formats

**text**: `path:line:column: error[rule.id] message`, then one line per class that was not
fully checked, then a summary.

**json**: the serialised [`a11y_report::Report`](https://docs.rs/a11y-report) of all files.
Every finding carries `location.file` and `location.position`; a finding whose layer knows where it
ends also carries `location.end` (`line`, `column`, `byte_offset`, exclusive). The range is always the element's start tag, also for a finding about the element's content. Units: see
[Positions and ranges](../../reference/api/#positions-and-ranges). A class that was not checked,
and a file that could not be checked, are entries of `inputs` with kind `document_incomplete`
and the file as `subject`.

**sarif**: SARIF 2.1.0, valid against the official schema and meant for GitHub Code Scanning:

```yaml
- run: html-conform dist --format sarif > html-conform.sarif || true
- uses: github/codeql-action/upload-sarif@v3
  with: { sarif_file: html-conform.sarif }
```

- `region` has `startLine`/`startColumn` and, where the end is known, `endLine`/`endColumn` (exclusive, as SARIF defines `endColumn`). The run sets `columnKind` to `unicodeCodePoints`, which is what the columns count. Byte offsets are not written: SARIF `byteOffset` belongs to binary regions, and the checked text may be decoded from another encoding than the file's bytes.
- `tool.driver.rules` takes id and description from the rule catalogue.
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

Encodings are sniffed from the file alone: no `--encoding` option and no legacy-encoding
heuristics beyond the prescan. Rule selection works on findings, so a rule cannot be turned off
to save its time, and there is no `--no-parse-errors`. The config file does not set input paths
and is not searched for in parent directories.
