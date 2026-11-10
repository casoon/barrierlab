# web-platform-tests media-query parsing cases

Unmodified HTML test files from
[`web-platform-tests/wpt`](https://github.com/web-platform-tests/wpt), path
`css/mediaqueries/`: `test_media_queries.html` and `match-media-parsing.html`.

- Upstream revision: `fc7c47fcb9172a216914879460e98018348ee3c5`
- License: BSD-3-Clause; see the unmodified upstream `LICENSE.md`
  (SPDX header in `REUSE.toml`)
- Refresh procedure: run `xtask/vendor-wpt-mediaqueries.sh` (`--check` compares
  without writing); changing
  the pinned revision requires re-blessing `tests/wpt_known_failures.txt`
  (`WPT_BLESS=1 cargo test -p media-query-parse --test wpt`).
- Used by `tests/wpt.rs`: every call with a single string literal argument is
  one case; calls built from loops or template strings need a JavaScript engine
  and are not run.
