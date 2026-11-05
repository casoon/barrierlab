# css-tokenize

A pure-Rust, dependency-free tokenizer for [CSS Syntax Module Level 3](https://www.w3.org/TR/css-syntax-3/)
(§3 preprocessing and §4 tokenization). It turns a `&str` into CSS tokens and nothing else:
grammars on top of the tokens (media queries, selectors, stylesheets) live in other crates.

```rust
use css_tokenize::{Token, tokenize};

let tokens = tokenize("a:hover");
assert!(matches!(tokens[0], Token::Ident(_)));
```

Extracted from [`media-query-parse`](https://crates.io/crates/media-query-parse), which still
re-exports it as `media_query_parse::tokenizer`. Used by `media-query-parse` and `stylesheet-parse`.

## License

MIT
