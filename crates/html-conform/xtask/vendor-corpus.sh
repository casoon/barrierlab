#!/usr/bin/env bash
# Vendor a subset of the vnu (validator/validator) test corpus into
# tests/corpus/, for Phase 07's differential tests.
#
# Source:  https://github.com/validator/validator
# Commit:  388cb365257d1410d4d32af960dba1cbd1b9af91 (tag 26.8.20)
#          Same commit as xtask/vendor-schema.sh — schema and corpus must
#          come from the same vnu snapshot, or expectations silently drift
#          from what the vendored schema actually accepts.
# License: MIT (tests/corpus/LICENSE in this repo, vendored from the
#          source repo's root LICENSE — tests/html and tests/html-aria
#          have no LICENSE of their own, unlike schema/html5/).
#
# Vendored subset (see plan/07-corpus-differential.md and DECISIONS.md for
# the scope decision): the full `tests/html/` and `tests/html-aria/` trees
# (~4655 .html fixtures) — not the whole vnu repo, and not its `css/`,
# `xhtml/`, `svg/`, `html-rdfa(lite)/`, `html-math/`, `html-its/`,
# `langdetect/`, `normalization/`, `schema-validation/` trees, which cover
# profiles/languages this crate doesn't target (XHTML, RDFa, MathML,
# language-detection heuristics, ...).
#
# Expectation model: vnu's own tests/messages.json maps a fixture's
# relative path to its single recorded expected message — but only for
# fixtures that produce ANY message at all. A fixture path absent from
# messages.json is expected to be fully clean (verified directly: none of
# the 3746 html/+html-aria messages.json entries lack a definitive
# `-novalid`/`-haswarn`/`-hasinfo` suffix, and none of the `-isvalid`/
# `-valid`-suffixed or suffix-less fixtures appear in messages.json — the
# two signals agree completely, so messages.json presence alone is a
# reliable ground truth). This script copies messages.json trimmed to only
# the html/+html-aria keys (tests/differential.rs's actual manifest); the
# untrimmed original stays upstream, not vendored.
#
# Uses a full shallow clone of the tagged commit (not per-file curl, which
# doesn't scale to ~4655 files) — only tests/html, tests/html-aria,
# tests/messages.json, and LICENSE are copied out; the clone itself is
# discarded.
#
# Requires: git, jq.
#
# Usage: xtask/vendor-corpus.sh

set -euo pipefail

VNU_COMMIT="388cb365257d1410d4d32af960dba1cbd1b9af91"
VNU_TAG="26.8.20"
VNU_REPO="https://github.com/validator/validator"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEST_DIR="${REPO_ROOT}/tests/corpus"

for tool in git jq; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "error: ${tool} not found on PATH" >&2
    exit 1
  fi
done

CLONE_DIR="$(mktemp -d)"
trap 'rm -rf "${CLONE_DIR}"' EXIT

echo "==> cloning validator/validator@${VNU_TAG} (shallow)"
git clone --quiet --depth 1 --branch "${VNU_TAG}" "${VNU_REPO}" "${CLONE_DIR}"

ACTUAL_COMMIT="$(git -C "${CLONE_DIR}" rev-parse HEAD)"
if [[ "${ACTUAL_COMMIT}" != "${VNU_COMMIT}" ]]; then
  echo "error: tag ${VNU_TAG} resolved to ${ACTUAL_COMMIT}, expected ${VNU_COMMIT}" \
       "-- upstream tag moved, update VNU_COMMIT after checking what changed" >&2
  exit 1
fi

echo "==> copying tests/html and tests/html-aria"
rm -rf "${DEST_DIR}/html" "${DEST_DIR}/html-aria"
cp -R "${CLONE_DIR}/tests/html" "${DEST_DIR}/html"
cp -R "${CLONE_DIR}/tests/html-aria" "${DEST_DIR}/html-aria"

HTML_COUNT="$(find "${DEST_DIR}/html" -name '*.html' | wc -l | tr -d ' ')"
ARIA_COUNT="$(find "${DEST_DIR}/html-aria" -name '*.html' | wc -l | tr -d ' ')"

echo "==> trimming tests/messages.json to html/+html-aria keys"
jq 'with_entries(select(.key | startswith("html/") or startswith("html-aria/")))' \
  "${CLONE_DIR}/tests/messages.json" > "${DEST_DIR}/messages.json"
MESSAGE_COUNT="$(jq 'length' "${DEST_DIR}/messages.json")"

echo "==> copying LICENSE"
cp "${CLONE_DIR}/LICENSE" "${DEST_DIR}/LICENSE"

echo "==> writing manifest.json"
cat > "${DEST_DIR}/manifest.json" <<EOF
{
  "source_repo": "${VNU_REPO}",
  "source_commit": "${VNU_COMMIT}",
  "source_tag": "${VNU_TAG}",
  "vendored_trees": ["tests/html", "tests/html-aria"],
  "excluded_trees": [
    "tests/css", "tests/xhtml", "tests/svg", "tests/html-rdfa",
    "tests/html-rdfalite", "tests/html-math", "tests/html-its",
    "tests/langdetect", "tests/normalization", "tests/schema-validation"
  ],
  "html_fixture_count": ${HTML_COUNT},
  "html_aria_fixture_count": ${ARIA_COUNT},
  "messages_json_entry_count": ${MESSAGE_COUNT},
  "expectation_model": "a fixture path present as a messages.json key is expected to produce at least one finding; a path absent from messages.json is expected to produce none"
}
EOF

echo "==> done: ${HTML_COUNT} html/ + ${ARIA_COUNT} html-aria/ fixtures, ${MESSAGE_COUNT} messages.json entries in ${DEST_DIR}"
