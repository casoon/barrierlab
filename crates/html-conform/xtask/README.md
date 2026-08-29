# xtask

Vendor-/build-time scripts (`vendor-*.sh`) that pull the RELAX NG schema and
test corpus from `validator/validator` into `schema/` and `tests/corpus/`.
Build/vendor-time only — no JVM/Trang/git dependency ships in the built
crate. `vendor-schema.sh`: `plan/03-schema-vendoring.md`. `vendor-corpus.sh`
(requires `git`, `jq`): `plan/07-corpus-differential.md`. Both scripts must
be run against the **same** vnu commit/tag (documented in each script's
header) — schema and corpus expectations drift silently out of sync
otherwise.
