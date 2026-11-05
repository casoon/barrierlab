#!/usr/bin/env bash
set -euo pipefail

# Vendors the tokenizer-relevant file of the css-parsing-tests suite without
# modifying it. Run from crates/css-tokenize.
#
# Source: https://github.com/SimonSapin/css-parsing-tests
# Revision: 203ce36bffd617db7f118c551e32794561fb273d
# License: CC0-1.0 (upstream LICENSE)

readonly upstream_repository='https://github.com/SimonSapin/css-parsing-tests.git'
readonly upstream_revision='203ce36bffd617db7f118c551e32794561fb273d'
readonly destination='tests/corpus/css-parsing-tests'

if [[ -e "$destination/component_value_list.json" ]]; then
  echo "refusing to overwrite $destination/component_value_list.json" >&2
  exit 1
fi

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

git clone --quiet "$upstream_repository" "$temporary_directory/css-parsing-tests"
git -C "$temporary_directory/css-parsing-tests" checkout --quiet --detach "$upstream_revision"

if [[ $(git -C "$temporary_directory/css-parsing-tests" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

mkdir -p "$destination"
cp "$temporary_directory/css-parsing-tests/component_value_list.json" "$destination/"
cp "$temporary_directory/css-parsing-tests/LICENSE" "$destination/LICENSE"
cp "$temporary_directory/css-parsing-tests/README.rst" "$destination/README.rst"
(cd "$destination" && shasum -a 256 component_value_list.json > component_value_list.json.sha256)
