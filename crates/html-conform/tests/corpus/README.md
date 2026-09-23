# tests/corpus

Vendored HTML test corpus derived from vnu (`validator/validator`,
MIT-licensed, `LICENSE` in this directory), used by `tests/differential.rs`
for differential testing against vnu's reference behavior.

Not hand-edited — updated only via `xtask/vendor-corpus.sh` (which must be
run with the same vnu commit/tag as `xtask/vendor-schema.sh` — see that
script's header and `plan/DECISIONS.md`; schema and corpus drift silently
out of sync otherwise). See `plan/07-corpus-differential.md`.

## Contents

- `html/`, `html-aria/` — the vendored fixture trees (`*.html`), copied
  byte-identical from upstream `tests/html`/`tests/html-aria`. Scope
  decision (why these two trees and not the rest of vnu's `tests/`
  directory) and exclusion list: `manifest.json`, `plan/DECISIONS.md`.
- `messages.json` — vnu's own expected-message map, trimmed to only the
  `html/`/`html-aria/` keys. A fixture path present here is expected to
  produce at least one finding; a path absent is expected to produce none
  — verified (see `xtask/vendor-corpus.sh`'s header comment) to be a
  complete, self-consistent signal on its own, without needing to parse
  the fixture filenames' `-novalid`/`-haswarn`/`-isvalid`/etc. suffixes.
- `manifest.json` — provenance (source commit/tag, vendored/excluded
  trees, fixture counts) written by `xtask/vendor-corpus.sh` on each run.
