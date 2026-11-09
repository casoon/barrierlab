# xtask

Vendor-/build-time scripts (`vendor-*.sh`) that pull the RELAX NG schema and
test corpus from `validator/validator` into `schema/` and `tests/corpus/`.
Build/vendor-time only — no JVM/Trang/git dependency ships in the built
crate. `vendor-corpus.sh` requires `git` and `jq`. Both scripts must
be run against the **same** vnu commit/tag (documented in each script's
header) — schema and corpus expectations drift silently out of sync
otherwise.

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
- **vnu:** `VNU_JAR=/path/vnu.jar` (needs `java`, `jq`), or `VNU_JSON_DIR` with
  pre-recorded `<slug>.json` (`vnu --format json`). Neither is vendored.
- **Self-test:** `test-real-world.sh` checks the baseline/diff logic with a
  synthetic snapshot, no Java, vnu or network.

Typical flow: `fetch-real-world.sh`, then `VNU_JAR=... compare-real-world.sh`;
review the diff; `--update-baseline` and commit the changed baseline.
