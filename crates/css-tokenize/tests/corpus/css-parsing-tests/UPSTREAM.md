# css-parsing-tests

Unmodified files from [`SimonSapin/css-parsing-tests`](https://github.com/SimonSapin/css-parsing-tests).

- Upstream revision: `203ce36bffd617db7f118c551e32794561fb273d`
- Files: `component_value_list.json` (the tokenizer-relevant suite), `README.rst` (describes the
  JSON representation), `LICENSE`
- License: CC0-1.0 (annotation in the root `REUSE.toml`)
- Integrity: SHA-256 of the JSON in `component_value_list.json.sha256`
- Refresh procedure: remove this directory deliberately, then run
  `xtask/vendor-css-parsing-tests.sh` from `crates/css-tokenize`; changing the pinned revision
  requires re-checking `tests/css_parsing_tests_known_failures.txt`.
- Used by `tests/css_parsing_tests.rs`.
