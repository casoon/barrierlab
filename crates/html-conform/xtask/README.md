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

`fetch-real-world.sh` (requires `curl`) and `compare-real-world.sh`
(requires `java`, `jq`, and `VNU_JAR` pointing at a local vnu.jar) are a
separate, manual/periodic dev signal: they fetch real websites from
`real-world-sites.txt` into `xtask/.cache/real-world/` and compare
html-conform's findings (via the `check-file` binary in
`xtask/check-file/`) against a locally running vnu. Not run in CI, and not
part of `tests/corpus/`'s pinned differential baseline — real pages carry
unrelated, pre-existing markup errors, so this is a coarse sanity check,
not a 0-false-positive gate.
