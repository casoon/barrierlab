# web-platform-tests accname name-computation cases

Unmodified HTML test files from
[`web-platform-tests/wpt`](https://github.com/web-platform-tests/wpt), path
`accname/name/`.

- Upstream revision: `8a829e6ed64a2c6c995776c620bc34fa78a4032b`
- License: BSD-3-Clause; see the unmodified upstream `LICENSE.md`
  (SPDX header in `REUSE.toml`)
- Refresh procedure: remove this directory deliberately, then run
  `xtask/vendor-wpt-accname.sh` from `crates/accname`; changing the
  pinned revision requires re-blessing `tests/wpt_known_failures.txt`
  (`WPT_BLESS=1 cargo test -p accname --test wpt`).
- Used by `tests/wpt.rs`: every element with `data-expectedlabel` is one case.
