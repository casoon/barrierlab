# relax-ng

A pure-Rust implementation of [RELAX NG](https://relaxng.org/) — reads a
RELAX NG schema (compact `.rnc` or XML `.rng` syntax) and validates a
document's structure against it: which elements/attributes are allowed
where, in what order, how many times.

Generic and standalone — not tied to HTML, XML parsing, or any specific
document type. Callers provide their own parsed document via a trait; this
crate only implements the RELAX NG side (schema parsing, composition,
pattern matching, datatypes).

## Usage

```rust
use relax_ng::{
    Content, DatatypeRegistry, Element, ExpandedName, ResolveError, Schema, SchemaResolver,
    SchemaSource, SchemaSyntax,
};

// This crate never reads files or the network itself — it always asks a
// caller-supplied resolver for any `include`/`externalRef`. A schema with
// none of those never actually calls it.
struct NoResolver;
impl SchemaResolver for NoResolver {
    fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("no such resource: {href}")))
    }
}

let source = SchemaSource::new(
    "element greeting { attribute lang { text }, text }",
    "memory:/greeting.rnc",
    SchemaSyntax::Compact,
);
let schema = Schema::compile(&source, &NoResolver).expect("schema compiles");

// Implement `Element` over your own document tree — an XML DOM, an HTML
// parser's tree, whatever you already have. This crate parses none of it.
struct Doc { /* ... */ }
impl Element for Doc {
    fn name(&self) -> ExpandedName { /* ... */ unimplemented!() }
    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> { /* ... */ std::iter::empty() }
    fn children(&self) -> impl Iterator<Item = Content<Self>> { /* ... */ std::iter::empty() }
}

let errors = schema.validate(&DatatypeRegistry::new(), &Doc { /* ... */ })
    .expect("only built-in/XSD datatypes are used");
assert!(errors.is_empty(), "document is valid");
```

See [`examples/validate.rs`](examples/validate.rs) for a complete,
runnable version (`cargo run --example validate`), and the `Schema`/
`Element` rustdoc for the full contract.

## Datatype libraries

- The RELAX NG built-in library (empty `datatypeLibrary`, `string`/
  `token`) and XSD (`http://www.w3.org/2001/XMLSchema-datatypes` — all 19
  primitive types, the built-in derived types, and all facets) are
  implemented and registered by default via `DatatypeRegistry::new()`.
  `decimal`/date-time ordering and `duration` comparison have documented,
  deliberate precision limits — see `plan/DECISIONS.md`'s Phase 06 entry.
- Any other `datatypeLibrary` (e.g. vnu's own
  `http://whattf.org/datatype-draft`) is **not** implemented by this
  crate — register your own [`DatatypeLibrary`] implementation on the
  registry instead; an unregistered library fails schema compilation
  fast, with a clear error, rather than silently mismatching.

## Resource loading

This crate never opens a file or makes a network request on its own.
`SchemaSource` carries schema text plus a base URI; a caller-supplied
`SchemaResolver` resolves any `include`/`externalRef` an `href` — reading
from disk, an in-memory map, HTTP, or anything else, with whatever
security boundary (allowed schemes, path restrictions) fits the caller.

## Performance

Validation time per page for the HTML schema that
`html-conform` validates against, measured on real pages:
the schema stage of `html_conform::check` (release build, warm, median of
7 runs), Apple M4 Pro, 48 GB, macOS 27.2, rustc 1.99.1. Both columns are
this crate, on the same machine with the same harness; nothing else is
compared.

| Page | Tags | 0.2.2 | 0.2.3 |
|---|---:|---:|---:|
| example.com | 10 | 3.3 ms | 2.2 ms |
| www.w3.org | 369 | 99 ms | 4.9 ms |
| www.gnu.org | 399 | 108 ms | 6.7 ms |
| www.usa.gov | 476 | 127 ms | 6.5 ms |
| insights.casoon.de | 492 | 291 ms | 7.0 ms |
| news.ycombinator.com | 814 | 4.0 ms | 2.3 ms |
| www.python.org | 840 | 248 ms | 7.5 ms |
| developer.mozilla.org | 851 | 285 ms | 7.5 ms |
| Insights article (Claude Code) | 854 | 406 ms | 7.8 ms |
| Insights article (Caching) | 1,390 | 734 ms | 10.4 ms |
| Insights article (Typst) | 1,659 | 897 ms | 8.6 ms |
| en.wikipedia.org/wiki/HTML | 8,069 | 2,658 ms | 74 ms |
| Insights category page | 10,281 | 8,136 ms | 32 ms |

Compiling the schema is a one-time cost per process and not included.
The pages come from `crates/html-conform/xtask/real-world-sites.txt`
(fetched with `fetch-real-world.sh`); `cargo bench -p html-conform --bench
check -- real_world` measures the whole check over the same set.

## Status

Parsing (both `.rnc` and `.rng`), composition/simplification, the
Brzozowski-derivative validation engine, and the built-in+XSD datatype
libraries are implemented. The vendored official RELAX NG test suite
passes at 100% for both schema classification and instance validation
(the subset not requiring a multi-file resolver — see
`plan/07-conformance-loop.md`). Not yet published to crates.io; the public
API may still change before `1.0`. See `plan/00-STATUS.md` for the
authoritative, up-to-date phase-by-phase status (not tracked in git,
local working document; see `plan/README.md` for why).

## Why this exists

Built for [`html-conform`](https://github.com/casoon/html-conform)'s
RELAX NG validation layer (schema.rs), after evaluating two existing
options and rejecting both — see that project's `plan/DECISIONS.md` for
the full writeup:

- `xmloxide`'s RelaxNG engine has several confirmed bugs (upstream issues
  [#52](https://github.com/jonwiggins/xmloxide/issues/52)–[#56](https://github.com/jonwiggins/xmloxide/issues/56)),
  the worst of which rejects almost any attribute in a realistically
  modular schema.
- [`dholroyd/relaxng-rust`](https://github.com/dholroyd/relaxng-rust) is
  technically strong (passes the official 384-test RELAX NG test suite,
  reads `.rnc` natively, handles multi-file composition) but carried
  **no license at all** when this crate was started — unusable as a
  dependency without the author's explicit permission. It has been
  licensed MIT OR Apache-2.0 since October 2026.

This crate is architecturally *informed by* `relaxng-rust`'s public
design (the syntax → model → validator split, targeting the official
RELAX NG test suite) but is an independent, clean-room implementation —
no code taken from it — published under MIT from the start.

## Package name and versioning

Published on crates.io as `relax-ng`. The current `0.1.1` in `Cargo.toml`
is a pre-release working version, not a semver commitment — breaking
changes to the public API are still expected before a `1.0` release.
After that, this crate follows normal semver.

## License

MIT — see [LICENSE](LICENSE).
