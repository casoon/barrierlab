# xtask

Vendor-/build-time scripts (`vendor-*.sh`) that pull the RELAX NG schema and
test corpus from `validator/validator` into `schema/` and `tests/corpus/`.
Build/vendor-time only — no JVM/Trang/git dependency ships in the built
crate. `vendor-corpus.sh` requires `git` and `jq`. Both scripts must
be run against the **same** vnu commit/tag (documented in each script's
header) — schema and corpus expectations drift silently out of sync
otherwise.

## Local deviations from vnu (`schema-patches/`)

`schema/html5/` is upstream plus the patches in `schema-patches/NNNN-<what>.patch`
(header comment in each: what, why, file). `vendor-schema.sh` applies them after
the download and aborts if one does not apply. Never edit `schema/html5/*.rnc`
by hand: change or add a patch, then re-run the script.

- **Reproducibility check:** `xtask/vendor-schema.sh --check` vendors the pinned
  commit into a temp dir, applies the patches and diffs against the committed
  tree (exit 1 on any difference). Needs network, not run in CI; run it after
  every change to the script, a patch or `schema/html5/`.
  `xtask/vendor-svg-mathml.sh --check` does the same for the unpatched
  `schema/html5-svg-mathml.rnc`, `schema/svg11/` and `schema/mml3/`.
  `xtask/vendor-corpus.sh --check` does it for `tests/corpus/` (shallow clone of the tag).
- **Offline part:** `cargo test -p html-conform --test schema_patches` checks
  that each patch targets an existing module and that its added lines are in
  the committed file.
- **New patch:** vendor without it, edit the module in a copy, then
  `diff -u -L a/<f> -L b/<f> upstream-with-SPDX-header patched` (paths relative
  to `schema/html5/`, `patch -p1`), and put a comment header on top.

After every `vendor-schema.sh`/`vendor-svg-mathml.sh` run, regenerate the compiled schema that
ships in the crate (`prebuilt/html5.rngc`, loaded instead of compiling the `.rnc` files at
start-up): `UPDATE_PREBUILT=1 cargo test -p html-conform prebuilt_schema_is_current`. The same
test, without the variable, fails when the file is stale.

## Real-world comparison (html-conform vs vnu)

Manual / on-tag dev signal, never in per-push CI. Real pages carry their own
markup errors, so this is not a 0-false-positive gate; it makes changes
between runs on the *same* pages visible.

- **Page list:** `real-world-sites.txt` (fixed, in git).
- **Snapshot:** `fetch-real-world.sh` (needs `curl`) downloads the pages into
  `xtask/.cache/real-world/` (gitignored; third-party pages are not
  committed). The snapshot is pinned: existing files are kept, `--refresh`
  re-downloads. After a refresh the baseline must be regenerated.
- **Baseline:** `real-world-baseline.tsv` (in git): per page the first 12 hex
  digits of the snapshot's sha256, the html-conform finding count and the
  vnu message count (`-` = vnu not run, never 0). Finding-class agreement is
  not included; it waits for the finding-level metric.
- **Run:** `compare-real-world.sh` prints a table and a diff against the
  baseline (`--check` exits 1 on any change, `--update-baseline` rewrites the
  baseline; a recorded vnu count is kept when a run had none and the page is
  unchanged). A page whose sha256 differs from the baseline is reported as
  "snapshot changed", not as a regression. Output has no timings or paths:
  two runs on one snapshot are byte-identical.
- **vnu:** `VNU_JAR=/path/vnu.jar` (needs `java`, `jq`); without it `xtask/.cache/vnu.jar`
  (gitignored) is used if present. The jar is checked against a pinned SHA-256 (vnu 26.8.20,
  the schema's version; the npm package `vnu-jar@26.8.20` ships it, GitHub only has a rolling
  `latest` release) and a mismatch aborts; or `VNU_JSON_DIR` with
  pre-recorded `<slug>.json` (`vnu --format json`). Neither is vendored.
- **Self-test:** `test-real-world.sh` checks the baseline/diff logic with a
  synthetic snapshot, no Java, vnu or network.

Typical flow: `fetch-real-world.sh`, then `VNU_JAR=... compare-real-world.sh`;
review the diff; `--update-baseline` and commit the changed baseline.

## W3C-licensed vnu trees (fetched on demand)

`tests/html-svg` (516 W3C SVG 1.1 test cases) and `tests/html-its` (W3C ITS 2.0 test suite) of
vnu are under the W3C Test Suite License, not MIT, so they are not vendored (issues #174, #175).

- `fetch-w3c-trees.sh` (needs `git`, `shasum` or `sha256sum`) makes a shallow sparse clone of the
  pinned vnu commit (the same `VNU_COMMIT` as `vendor-*.sh`; each script has its own copy) and
  copies both trees plus `tests/messages.json` into `xtask/.cache/w3c-trees/` (gitignored), with a
  sorted `SHA256SUMS` manifest. It prints the file counts and replaces the cache on every run.
- Measurement: `cargo test --release -p html-conform --test w3c_trees -- --ignored --nocapture`.
  `html-svg` is compared with `messages.json` as in `differential.rs`; `html-its` has no
  expected-result file, so per-file agreement (clean vs. findings) is taken from
  `xtask/.cache/vnu.jar` (same pinned SHA-256 as `compare-real-world.sh`; Java from `$JAVA`,
  `PATH` or `$(brew --prefix openjdk)/bin/java`), else the vnu side is reported as NotRun. It
  asserts nothing: without the cache it prints "not run: cache missing" (with
  `W3C_TREES_REQUIRED=1` it fails). Not in CI. Results: `docs/html-conform/guides/vnu-comparison.md`.
