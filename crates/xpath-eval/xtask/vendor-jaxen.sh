#!/usr/bin/env bash
set -euo pipefail

# Fetches the Jaxen XPath test cases (tests.xml and the documents it uses) into
# tests/corpus/jaxen on demand. The directory is gitignored: the sample
# documents carry no licence notice of their own, so the corpus is
# intentionally NOT part of this repository (see CONFORMANCE.md).
# Run from crates/xpath-eval, then:
#   cargo test -p xpath-eval --test jaxen -- --ignored
#
# Source: https://github.com/jaxen-xpath/jaxen, path core/xml/
# License: BSD-3-Clause-style (upstream LICENSE.txt, copied along)

readonly upstream_repository='https://github.com/jaxen-xpath/jaxen.git'
readonly upstream_revision='e675b18efa486234ed364483184ae8ce493c48d1'
readonly tests_sha256='46c5c8f4acf69f1d3ce5e4773fc91ad4901d64e5379ef152a8d8f7d37eef204b'
readonly destination='tests/corpus/jaxen'

if [[ -e "$destination/tests.xml" ]]; then
  echo "$destination already present; remove it to fetch again" >&2
  exit 0
fi

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

git -C "$temporary_directory" init --quiet jaxen
git -C "$temporary_directory/jaxen" remote add origin "$upstream_repository"
git -C "$temporary_directory/jaxen" sparse-checkout set --no-cone /core/xml /LICENSE.txt
git -C "$temporary_directory/jaxen" fetch --quiet --depth 1 --filter=blob:none origin "$upstream_revision"
git -C "$temporary_directory/jaxen" checkout --quiet --detach FETCH_HEAD

if [[ $(git -C "$temporary_directory/jaxen" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

source_directory="$temporary_directory/jaxen/core"
if [[ $(shasum -a 256 "$source_directory/xml/test/tests.xml" | cut -d' ' -f1) != "$tests_sha256" ]]; then
  echo 'tests.xml does not match the pinned checksum' >&2
  exit 1
fi

mkdir -p "$destination/xml"
cp "$source_directory/xml/test/tests.xml" "$destination/tests.xml"
# Only the documents tests.xml refers to.
grep -o 'document url="xml/[^"]*"' "$source_directory/xml/test/tests.xml" |
  sed 's/^document url="//; s/"$//' | sort -u |
  while read -r document; do
    cp "$source_directory/$document" "$destination/$document"
  done
cp "$temporary_directory/jaxen/LICENSE.txt" "$destination/LICENSE.txt"
echo "fetched Jaxen corpus at $upstream_revision into $destination"
