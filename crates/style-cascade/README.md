# style-cascade

The CSS cascade without a browser: selector matching with specificity, inheritance, `var()` and
`@media` over a document (`a11y-dom`) and its stylesheets (`stylesheet-parse`). It produces the
computed styles that colour contrast rules need, and reports what it cannot know as unknown
rather than guessing.

```rust
use style_cascade::{Cascade, Environment};

let cascade = Cascade::new(&sheets, Environment::default());
let style = cascade.computed_style(node);
// style.color, style.background_color: `None` where they cannot be determined
```

Limits and structure: [package page](https://casoon.github.io/barrierlab/docs/packages/style-cascade/).
Licensed under MIT.
