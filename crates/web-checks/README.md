# web-checks

Shared evaluation for web audits. This crate is the middle ground between two
auditors that check the same things on the same site:
[auditmysite](https://crates.io/crates/auditmysite) against a live page in
Chrome, `astro-post-audit` against a built `dist/`. What differs is the
**collection**; what coincides is the **evaluation** — and that lives here.

Two rules keep it small:

1. **Nothing is fetched.** No HTTP, no filesystem, no browser. A host passes in
   text or data, this crate computes.
2. **No wording.** Rule identifiers, severities and report text stay with the
   host — they differ there and are localized. Here you get data and predicates
   a host turns into findings.

## Scope

- `robots` — robots.txt grammar, bot classification, longest-match path rules.
- `meta` — title and meta description length, counted in characters (not bytes),
  with the shared recommended ranges 30–60 and 120–160.
- `social` — OpenGraph and Twitter Card tag lists, presence (empty content counts
  as missing), completeness, valid `twitter:card` values, absolute image URLs.
- `structured_data` — JSON-LD: parse `ld+json` script text into normalized nodes
  (top-level arrays, `@graph`, every `@type` entry, strict schema.org `@context`),
  structural issues as data, per-type property assessment against Google Search
  Central's documented required/recommended properties (with source URL and
  ruleset version), and `@type` values duplicated across blocks.
- `hreflang` — language codes (Google's subset of BCP 47, case-insensitive),
  `x-default`, and whether a page lists itself (`x-default` does not count).

More families follow one at a time, each checked first for whether it really
asks the same question. "Security" looked like duplication by name; in fact one
tool checks HTTP headers and the other checks HTML.

## License

MIT
