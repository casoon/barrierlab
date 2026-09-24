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

`robots` — robots.txt grammar, bot classification, longest-match path rules.

More families follow one at a time, each checked first for whether it really
asks the same question. "Security" looked like duplication by name; in fact one
tool checks HTTP headers and the other checks HTML.

## License

MIT
