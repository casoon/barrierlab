# css-parse

A pure-Rust stylesheet parser following [CSS Syntax Level 3 §5
"Parsing"](https://www.w3.org/TR/css-syntax-3/#parsing) (the current
Editor's Draft algorithms, which include nesting) and
[CSS Nesting](https://www.w3.org/TR/css-nesting-1/). It turns raw CSS text
into rules, at-rules and declarations, and flattens them — through
`@media`, group rules and nesting — into style rules with resolved
selectors and their context.

It takes text, it fetches nothing, and it does not evaluate anything: no
cascade, no selector matching, no media query evaluation. Tokenizing is
done by the CSS Syntax Level 3 tokenizer of
[`media-query-parse`](https://crates.io/crates/media-query-parse).

## Usage

```rust
use css_parse::parse_stylesheet;

let sheet = parse_stylesheet(
    "a:focus { outline: none }
     .card { @media (prefers-reduced-motion: reduce) { transition: none } }
     @keyframes spin { to { transform: rotate(360deg) } }",
);

for rule in sheet.style_rules() {
    println!("{:?} {:?} {:?}", rule.selectors, rule.context.media, rule.declarations);
}
for (keyframes, context) in sheet.keyframes() {
    println!("{} in {:?}", keyframes.name, context.media);
}
```

## What comes out

- `Rule::Style`, `Rule::Media`, `Rule::Keyframes` (including
  `@-webkit-keyframes` etc., prefix in `vendor`), `Rule::Group`
  (`@supports`, `@layer {}`, `@container`, `@scope`, `@document`,
  `@-moz-document`, `@starting-style`) and `Rule::Other` for everything
  else (`@import`, `@charset`, `@font-face`, `@layer a, b;`, `@page`, …).
- At-rule names and property names are ASCII-lowercased; custom property
  names (`--x`) and keyframes names keep their case. Keyframe selectors
  (`from`, `50%`) are lowercased.
- Preludes, selectors and values are re-serialized from tokens:
  whitespace collapsed to one space, trimmed, comments gone. `!important`
  is removed from the value and reported as `important`.
- `Stylesheet::style_rules()` resolves nesting: `&` at the start of a
  selector becomes the parent selector (or `:is(a, b)` for a parent list),
  `&` anywhere else becomes `:is(parent)` (plain text substitution would
  change what `.c &` matches), a selector without `&` is treated as
  `& selector`, and a selector that is just `&` stands for the parent list
  itself. Declarations directly inside a nested `@media`/group rule apply
  to the parent selector.

## Error recovery

As specified: an invalid declaration is dropped up to the next `;` at the
same level, unclosed blocks are closed by the end of input, a stray `}`
at top level becomes part of the next rule's prelude (and so invalidates
that rule), `<!--`/`-->` at top
level are ignored. No input panics.

Beyond the syntax algorithms, a few things are dropped because no grammar
can ever accept them (browsers drop them too):

- declarations whose value contains a `<bad-string-token>`,
  `<bad-url-token>` or an unmatched `)`/`]`/`}`;
- style rules and keyframes whose selector list has an empty entry
  (`a,,b`) or contains such tokens;
- declarations directly inside a top-level `@media`/group rule, and
  at-rules other than `@media`/group rules inside a style rule;
- child rules of `Rule::Other` at-rules (e.g. `@page` margin boxes, or an
  unknown `@screen { … }`).

Full selector validation is out of scope.

Limits against hostile input: blocks nested deeper than 128 levels are
skipped, and a resolved nested selector (or selector list) longer than
64 KiB is dropped together with its descendants.

## Lossy serialization

Re-tokenizing a serialized value yields the same tokens, except:

- Numbers are written compactly: `.5` → `0.5`, `1.0` → `1`, `1e3` → `1000`,
  `+2` → `2` (the "integer" type flag of `1.0` is lost). A non-negative
  number directly after a token it would merge with gets a `+`, so
  `2n+1` stays `2n+1`. Literals beyond the `f64` range are clamped.
- Strings are always written with `"`.
- Where two tokens would merge without the comment that separated them,
  an empty comment `/**/` is inserted (CSS Syntax §9).
- `<bad-string-token>` and `<bad-url-token>` in at-rule preludes become
  `""` and `url()`.
- Declarations after nested rules are merged into the rule's
  `declarations` (CSS Nesting would wrap them in a nested-declarations
  rule; only the cascade order between them and the nested rules differs).

## Performance

Measured in release mode on an Apple Silicon laptop (2026-10-03): the
3.6 MB Tailwind 2.2.19 build in about 0.3 s, Bootstrap 5.3.3 (280 KB) in
20 ms, 337 real stylesheets of 48 sites (24 MB) in 3.7 s including
`style_rules()` — roughly 0.1 s per MB.

## License

MIT — see [LICENSE](LICENSE).
