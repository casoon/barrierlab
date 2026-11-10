#!/usr/bin/env bash
# Fetch the two vnu test trees that are NOT vendored because their files are
# under the W3C Test Suite License, not MIT, into a gitignored cache:
#
#   tests/html-svg  (522 files, 516 of them W3C SVG 1.1 test cases)
#   tests/html-its  (320 files, 90 .html; W3C ITS 2.0 test suite)
#
# plus tests/messages.json (vnu's expected-result map, needed for html-svg).
# Nothing foreign-licensed is committed (issues #174, #175); the opt-in
# measurement is `tests/w3c_trees.rs` (see xtask/README.md).
#
# Source:  https://github.com/validator/validator
# Commit:  388cb365257d1410d4d32af960dba1cbd1b9af91 (tag 26.8.20)
#          Same value as VNU_COMMIT in vendor-schema.sh / vendor-corpus.sh
#          (each script carries its own copy; keep them equal).
# Cache:   xtask/.cache/w3c-trees/{html-svg,html-its,messages.json,SHA256SUMS}
#          (xtask/.cache/ is gitignored). SHA256SUMS lists every fetched file,
#          sorted by path, so two runs of the pinned commit are byte-identical.
#
# Uses a shallow, blob-less, sparse clone of the tag (3 paths), not a per-file
# curl loop. Requires: git, shasum or sha256sum.
#
# Usage: xtask/fetch-w3c-trees.sh

set -euo pipefail

VNU_COMMIT="388cb365257d1410d4d32af960dba1cbd1b9af91"
VNU_TAG="26.8.20"
VNU_REPO="https://github.com/validator/validator"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CACHE_DIR="${SCRIPT_DIR}/.cache/w3c-trees"

if [[ $# -ne 0 ]]; then
  echo "usage: $0" >&2
  exit 2
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$@"; }
else
  sha256() { shasum -a 256 "$@"; }
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT
CLONE_DIR="${WORK_DIR}/clone"

echo "==> cloning validator/validator@${VNU_TAG} (shallow, sparse)"
git clone --quiet --depth 1 --filter=blob:none --sparse --branch "${VNU_TAG}" \
  "${VNU_REPO}" "${CLONE_DIR}"
git -C "${CLONE_DIR}" sparse-checkout set --no-cone \
  /tests/html-svg/ /tests/html-its/ /tests/messages.json

ACTUAL_COMMIT="$(git -C "${CLONE_DIR}" rev-parse HEAD)"
if [[ "${ACTUAL_COMMIT}" != "${VNU_COMMIT}" ]]; then
  echo "error: tag ${VNU_TAG} resolved to ${ACTUAL_COMMIT}, expected ${VNU_COMMIT}" \
       "-- upstream tag moved, update VNU_COMMIT after checking what changed" >&2
  exit 1
fi

OUT_DIR="${WORK_DIR}/out"
mkdir -p "${OUT_DIR}"
cp -R "${CLONE_DIR}/tests/html-svg" "${OUT_DIR}/html-svg"
cp -R "${CLONE_DIR}/tests/html-its" "${OUT_DIR}/html-its"
cp "${CLONE_DIR}/tests/messages.json" "${OUT_DIR}/messages.json"

(cd "${OUT_DIR}" && find html-svg html-its messages.json -type f | LC_ALL=C sort \
  | while IFS= read -r f; do sha256 "$f"; done > SHA256SUMS)

rm -rf "${CACHE_DIR}"
mkdir -p "${CACHE_DIR}"
cp -R "${OUT_DIR}/." "${CACHE_DIR}/"

count() { find "${CACHE_DIR}/$1" -type f -name "$2" | wc -l | tr -d ' '; }
echo "==> html-svg: $(count html-svg '*') files ($(count html-svg '*.html') .html)"
echo "==> html-its: $(count html-its '*') files ($(count html-its '*.html') .html)"
echo "==> $(wc -l < "${CACHE_DIR}/SHA256SUMS" | tr -d ' ') files in manifest, sha256 of SHA256SUMS: $(sha256 "${CACHE_DIR}/SHA256SUMS" | cut -d' ' -f1)"
echo "==> done: ${CACHE_DIR} (gitignored, not for commit: W3C Test Suite License)"
