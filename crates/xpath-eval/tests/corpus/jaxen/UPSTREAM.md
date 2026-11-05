# Jaxen XPath test cases

Unmodified `tests.xml` (upstream `core/xml/test/tests.xml`) and the sample
documents it refers to (`xml/`, upstream `core/xml/`) from
[`jaxen-xpath/jaxen`](https://github.com/jaxen-xpath/jaxen).

- Upstream revision: `e675b18efa486234ed364483184ae8ce493c48d1`
- License: BSD-3-Clause-style (Jaxen Project); see the unmodified upstream
  `LICENSE.txt` (SPDX header in `REUSE.toml`). Upstream states one licence for
  the whole repository; the sample documents carry no notice of their own.
- Refresh procedure: remove this directory deliberately, then run
  `xtask/vendor-jaxen.sh` from `crates/xpath-eval`; changing the pinned
  revision requires re-blessing `tests/jaxen_known_failures.txt`
  (`JAXEN_BLESS=1 cargo test -p xpath-eval --test jaxen`).
- Used by `tests/jaxen.rs`.
