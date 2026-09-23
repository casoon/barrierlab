# csp-parse

A pure-Rust implementation of the [Content Security Policy Level 3](https://www.w3.org/TR/CSP3/)
directive grammar — parses a serialized CSP (or a comma-separated list of
policies, as sent in the `Content-Security-Policy` HTTP header) and
reports whether it is syntactically valid, with structured access to its
directives and their values.

Generic and standalone — not tied to HTML, a specific host language, or
any particular consumer. Nothing in the API assumes a particular caller:
it's equally usable for validating/linting a CSP header or `<meta>` tag,
building a CSP config editor or security scanner, checking a policy in
CI, or any other tool that needs to know whether a CSP string is
well-formed and what it actually says.

## Usage

```rust
use csp_parse::{parse_policy_list, DirectiveValue};

let list = parse_policy_list("default-src 'self'; script-src 'self' 'nonce-abc123'");
for directive in &list.policies[0].directives {
    match directive.value() {
        DirectiveValue::SourceList(source_list) => println!("{}: {source_list:?}", directive.name),
        other => println!("{}: {other:?}", directive.name),
    }
}
```

See [`examples/parse.rs`](examples/parse.rs) (`cargo run --example
parse`) for a fuller, runnable example.

Parsing is deliberately lenient/infallible, matching CSP's own
forward-compatible design: an unrecognized directive name, an
unrecognized `source-expression` token, or a byte outside a
production's allowed character class does not fail the whole parse —
it's still reported structurally, with per-item validity queryable
separately (`Directive::name_is_valid`/`value_is_valid`,
`DirectiveValue::Unknown`, `SourceListEntry::expression: None`, …). See
`plan/DECISIONS.md` for the reasoning.

### Directive registry scope

The directive → value-grammar registry (`registry_lookup`) is a
snapshot of CSP3's directive set at the time this crate was written,
not a permanent guarantee of completeness — new directives get added to
the spec over time faster than the rest of the grammar changes. An
unregistered directive name is not an error (CSP3 itself treats unknown
directives as forward-compatible, not invalid); it just doesn't get a
structured `DirectiveValue` (falls back to `DirectiveValue::Unknown`,
with its raw name/value still available via `Directive`).

### Reuse for Subresource Integrity hash expressions

`parse_hash_expression` parses exactly the `hash-algorithm "-"
base64-value` grammar shared between CSP's `hash-source` (used inside
a `source-list`, quoted: `'sha256-...'`) and Subresource Integrity's
unquoted `integrity=""` hash tokens (`sha256-...`, no quotes). It's
exported as its own building block so callers that only need to
validate/parse SRI hash expressions can use it directly, without
depending on the rest of this crate's CSP-specific parsing (one such
caller: [`html-conform`](https://github.com/casoon/html-conform)'s
`w:integrity-metadata` datatype).

## Status

Phases 02–07 implemented (generic directive splitting, source-list
grammar, directive registry, spec-example/edge-case sweep, public API
docs, licensing/compliance) — see `plan/00-STATUS.md` (not tracked in
git, local working document; see `plan/README.md` for why). Not yet
published to crates.io (Phase 08). No official CSP grammar-only
testsuite exists to vendor (web-platform-tests' CSP tests are
browser-enforcement tests, not syntax tests); conformance confidence
instead comes from the per-phase test matrices plus
`tests/spec_examples.rs`.

## Package name

Intended crates.io name: `csp-parse`. Not yet published.

## Semver

`0.1.0` is a pre-release working version with no semver stability
guarantee — breaking changes are expected before the first crates.io
publish (Phase 08). Normal semver applies afterwards.

## License

MIT — see [LICENSE](LICENSE).
