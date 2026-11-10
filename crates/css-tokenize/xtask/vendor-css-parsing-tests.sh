#!/usr/bin/env bash
set -euo pipefail

# Vendors the tokenizer-relevant file of the css-parsing-tests suite without
# modifying it.
#
# Source: https://github.com/SimonSapin/css-parsing-tests
# Revision: 203ce36bffd617db7f118c551e32794561fb273d
# License: CC0-1.0 (upstream LICENSE)
#
# Usage: xtask/vendor-css-parsing-tests.sh [--check]
#   no argument  replaces the corpus with the pinned upstream revision
#   --check      vendors into a temp dir and diffs it against the committed
#                corpus (exit 1 on any difference); needs network

readonly upstream_repository='https://github.com/SimonSapin/css-parsing-tests.git'
readonly upstream_revision='203ce36bffd617db7f118c551e32794561fb273d'

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
readonly destination="$script_directory/../tests/corpus/css-parsing-tests"

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

git clone --quiet "$upstream_repository" "$temporary_directory/css-parsing-tests"
git -C "$temporary_directory/css-parsing-tests" checkout --quiet --detach "$upstream_revision"

if [[ $(git -C "$temporary_directory/css-parsing-tests" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

readonly staging="$temporary_directory/staging"
mkdir -p "$staging"
# UPSTREAM.md is written by hand, not taken from upstream.
cp "$destination/UPSTREAM.md" "$staging/"
cp "$temporary_directory/css-parsing-tests/component_value_list.json" "$staging/"
cp "$temporary_directory/css-parsing-tests/LICENSE" "$staging/LICENSE"
cp "$temporary_directory/css-parsing-tests/README.rst" "$staging/README.rst"
(cd "$staging" && shasum -a 256 component_value_list.json > component_value_list.json.sha256)

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
