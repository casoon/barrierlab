#!/usr/bin/env bash
set -euo pipefail

# Vendors the upstream RELAX NG validation test suite without modifying the
# original test data.
#
# Source: https://github.com/relaxng/jing-trang
# Revision: a6bc0041035988325dfbfe7823ef2c098fc56597
# License: BSD-3-Clause (upstream copying.txt)
#
# Usage: xtask/vendor-testsuite.sh [--check]
#   no argument  replaces the corpus with the pinned upstream revision
#   --check      vendors into a temp dir and diffs it against the committed
#                corpus (exit 1 on any difference); needs network

readonly upstream_repository='https://github.com/relaxng/jing-trang.git'
readonly upstream_revision='a6bc0041035988325dfbfe7823ef2c098fc56597'
readonly upstream_test_path='mod/rng-validate/test/spectest.xml'

mode=update
case "${1:-}" in
  '') ;;
  --check) mode=check ;;
  *)
    echo "usage: $0 [--check]" >&2
    exit 2
    ;;
esac

script_directory=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
readonly destination="$script_directory/../tests/corpus/relaxng"

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

git clone --quiet "$upstream_repository" "$temporary_directory/jing-trang"
git -C "$temporary_directory/jing-trang" checkout --quiet --detach "$upstream_revision"

if [[ $(git -C "$temporary_directory/jing-trang" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

readonly staging="$temporary_directory/staging"
mkdir -p "$staging"
# UPSTREAM.md is written by hand, not taken from upstream.
cp "$destination/UPSTREAM.md" "$staging/"
cp "$temporary_directory/jing-trang/$upstream_test_path" "$staging/spectest.xml"
cp "$temporary_directory/jing-trang/copying.txt" "$staging/copying.txt"
printf '%s  %s\n' \
  '3812289f941d4a1aa8ad0ab0f5e16f85f59d912eec710feb86b6b14a6e942a96' \
  'spectest.xml' > "$staging/spectest.xml.sha256"
cat > "$staging/spectest.xml.license" <<'EOF'
SPDX-FileCopyrightText: 2001-2003 Thai Open Source Software Center Ltd
SPDX-License-Identifier: BSD-3-Clause
EOF
cat > "$staging/copying.txt.license" <<'EOF'
SPDX-FileCopyrightText: 2001-2003 Thai Open Source Software Center Ltd
SPDX-License-Identifier: BSD-3-Clause
EOF

(
  cd "$staging"
  shasum -a 256 --check spectest.xml.sha256
)

if [[ $mode == check ]]; then
  if ! diff -r "$staging" "$destination"; then
    echo "error: $destination differs from upstream $upstream_revision" >&2
    exit 1
  fi
  echo "ok: $destination equals upstream $upstream_revision"
else
  rm -rf "$destination"
  mkdir -p "$destination"
  cp -R "$staging/." "$destination/"
  echo "vendored upstream $upstream_revision into $destination"
fi
