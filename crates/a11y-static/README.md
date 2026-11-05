# a11y-static

Browser-free accessibility audit of static HTML. You hand over the HTML text and
the stylesheet texts you fetched; you get an [`a11y-report`](https://docs.rs/a11y-report)
`Report` back. Fetching and printing stay in your tool.

```rust
use a11y_static::{Environment, Page, Sheet, StylesheetRef, audit};

let html = std::fs::read_to_string("page.html")?;

// One call, when there is nothing to fetch:
let report = audit(&html, &[], Environment::default());

// Or: discover what to fetch, fetch it, audit.
let page = Page::parse(&html);
let mut sheets = Vec::new();
for r in page.stylesheet_refs() {
    if let StylesheetRef::Link { href, .. } = r {
        let css = fetch(&href)?; // your code
        sheets.push(Sheet { source: href, css, media: None });
    }
}
let report = page.audit(&sheets, Environment::default(), a11y_static::Locale::En);
```

## What it does

`html5-parser` tree (read in place through `a11y-dom`, no copy) -> roles and
names from `accname` (`aria-hidden` and `inert` respected) -> computed styles
from `style-cascade` (`display: none`, the `hidden` attribute and
`visibility` respected) -> every rule tier of `a11y-rules` plus its
stylesheet rules.

Stylesheet discovery follows the HTML rules: `<link rel~=stylesheet>` without
`alternate`, without `disabled`, with a non-empty `href`; every `<style>`. A
`media` attribute on either is evaluated by the cascade against the
`Environment`.

Incomplete input is reported, not hidden: a `<link>` without a supplied `Sheet`
gives an `InputKind::StylesheetUnavailable` note, an unresolved `@import` an
`InputKind::StylesheetUnresolved` note. With either note the host claims no
colours, so the contrast rules report `untested` instead of passing or failing
on an unstyled page; `display` and `visibility` still apply.

## Features

- `de`: German finding texts (`Locale::De`), passed through to `a11y-rules`.

## Status

0.1.0, not yet published. `barrierlab-cli` is to adopt it once it is on crates.io.

MIT licensed.
