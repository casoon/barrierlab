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
# Vendored subset: the full `tests/html/` and `tests/html-aria/` trees
# (~4655 .html fixtures), plus `tests/html-math/`, `tests/html-rdfa/` and
# `tests/html-rdfalite/` in full and the vnu-authored part of
# `tests/html-svg/` (issue #86). Not vendored: `css/`, `xhtml/`, `svg/`,
# `html-its/`, `langdetect/`, `normalization/`, `schema-validation/`, and the
# W3C-copyrighted SVG 1.1 test cases in `html-svg/` — the reasons are in
# docs/html-conform/guides/vnu-comparison.md.
#
# html-svg: 516 of its 522 files embed W3C SVG 1.1 test cases whose header
# says "Copyright 2009 World Wide Web Consortium ... All Rights Reserved";
# W3C distributes that suite under its document license (no modification, a
# subset counts as a derivative work), which is not the MIT license of this
# repository. Only the files WITHOUT that header (written by the vnu authors,
# MIT) are copied.
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
# the vendored trees' keys (tests/differential.rs's actual manifest); the
# untrimmed original stays upstream, not vendored.
#
# Uses a full shallow clone of the tagged commit (not per-file curl, which
# doesn't scale to ~4655 files) — only the vendored trees,
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

echo "==> copying tests/html, html-aria, html-math, html-rdfa, html-rdfalite"
rm -rf "${DEST_DIR}/html" "${DEST_DIR}/html-aria" "${DEST_DIR}/html-math" \
       "${DEST_DIR}/html-rdfa" "${DEST_DIR}/html-rdfalite" "${DEST_DIR}/html-svg"
for tree in html html-aria html-math html-rdfa html-rdfalite; do
  cp -R "${CLONE_DIR}/tests/${tree}" "${DEST_DIR}/${tree}"
done

echo "==> copying the vnu-authored files of tests/html-svg (no W3C copyright header)"
mkdir -p "${DEST_DIR}/html-svg"
SVG_TOTAL="$(find "${CLONE_DIR}/tests/html-svg" -name '*.html' | wc -l | tr -d ' ')"
while IFS= read -r file; do
  cp "${file}" "${DEST_DIR}/html-svg/"
done < <(grep -L "World Wide Web Consortium" "${CLONE_DIR}"/tests/html-svg/*.html)

HTML_COUNT="$(find "${DEST_DIR}/html" -name '*.html' | wc -l | tr -d ' ')"
ARIA_COUNT="$(find "${DEST_DIR}/html-aria" -name '*.html' | wc -l | tr -d ' ')"
MATH_COUNT="$(find "${DEST_DIR}/html-math" -name '*.html' | wc -l | tr -d ' ')"
RDFA_COUNT="$(find "${DEST_DIR}/html-rdfa" -name '*.html' | wc -l | tr -d ' ')"
RDFALITE_COUNT="$(find "${DEST_DIR}/html-rdfalite" -name '*.html' | wc -l | tr -d ' ')"
SVG_COUNT="$(find "${DEST_DIR}/html-svg" -name '*.html' | wc -l | tr -d ' ')"

echo "==> trimming tests/messages.json to the vendored trees' keys"
# html-svg is only partly vendored, so its keys are limited to files that exist.
SVG_KEYS="$(cd "${DEST_DIR}" && find html-svg -name '*.html' | jq -R . | jq -s .)"
jq --argjson svg "${SVG_KEYS}" \
  'with_entries(select((.key | test("^(html|html-aria|html-math|html-rdfa|html-rdfalite)/")) or (.key as $k | $svg | index($k) != null)))' \
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
  "vendored_trees": [
    "tests/html", "tests/html-aria", "tests/html-math", "tests/html-rdfa",
    "tests/html-rdfalite", "tests/html-svg (vnu-authored files only)"
  ],
  "excluded_trees": [
    "tests/css", "tests/xhtml", "tests/svg", "tests/html-its",
    "tests/langdetect", "tests/normalization", "tests/schema-validation",
    "tests/html-svg (W3C-copyrighted SVG 1.1 test cases)"
  ],
  "html_fixture_count": ${HTML_COUNT},
  "html_aria_fixture_count": ${ARIA_COUNT},
  "html_math_fixture_count": ${MATH_COUNT},
  "html_rdfa_fixture_count": ${RDFA_COUNT},
  "html_rdfalite_fixture_count": ${RDFALITE_COUNT},
  "html_svg_fixture_count": ${SVG_COUNT},
  "html_svg_upstream_fixture_count": ${SVG_TOTAL},
  "messages_json_entry_count": ${MESSAGE_COUNT},
  "expectation_model": "a fixture path present as a messages.json key is expected to produce at least one finding; a path absent from messages.json is expected to produce none"
}
EOF

echo "==> done: ${HTML_COUNT} html/ + ${ARIA_COUNT} html-aria/ + ${MATH_COUNT} html-math/ + ${RDFA_COUNT} html-rdfa/ + ${RDFALITE_COUNT} html-rdfalite/ + ${SVG_COUNT}/${SVG_TOTAL} html-svg/ fixtures, ${MESSAGE_COUNT} messages.json entries in ${DEST_DIR}"
