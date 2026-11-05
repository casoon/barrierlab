#!/usr/bin/env bash
set -euo pipefail

# Vendors the Jaxen XPath test cases (tests.xml and the documents it uses)
# without modifying them. Run from crates/xpath-eval.
#
# Source: https://github.com/jaxen-xpath/jaxen, path core/xml/
# License: BSD-3-Clause-style (upstream LICENSE.txt)

readonly upstream_repository='https://github.com/jaxen-xpath/jaxen.git'
readonly upstream_revision='e675b18efa486234ed364483184ae8ce493c48d1'
readonly destination='tests/corpus/jaxen'

if [[ -e "$destination/UPSTREAM.md" ]]; then
  echo "refusing to overwrite $destination" >&2
  exit 1
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
mkdir -p "$destination/xml"
cp "$source_directory/xml/test/tests.xml" "$destination/tests.xml"
# Only the documents tests.xml refers to.
grep -o 'document url="xml/[^"]*"' "$source_directory/xml/test/tests.xml" |
  sed 's/^document url="//; s/"$//' | sort -u |
  while read -r document; do
    cp "$source_directory/$document" "$destination/$document"
  done
cp "$temporary_directory/jaxen/LICENSE.txt" "$destination/LICENSE.txt"

cat > "$destination/UPSTREAM.md" <<EOF
# Jaxen XPath test cases

Unmodified \`tests.xml\` (upstream \`core/xml/test/tests.xml\`) and the sample
documents it refers to (\`xml/\`, upstream \`core/xml/\`) from
[\`jaxen-xpath/jaxen\`](https://github.com/jaxen-xpath/jaxen).

- Upstream revision: \`$upstream_revision\`
- License: BSD-3-Clause-style (Jaxen Project); see the unmodified upstream
  \`LICENSE.txt\` (SPDX header in \`REUSE.toml\`). Upstream states one licence for
  the whole repository; the sample documents carry no notice of their own.
- Refresh procedure: remove this directory deliberately, then run
  \`xtask/vendor-jaxen.sh\` from \`crates/xpath-eval\`; changing the pinned
  revision requires re-blessing \`tests/jaxen_known_failures.txt\`
  (\`JAXEN_BLESS=1 cargo test -p xpath-eval --test jaxen\`).
- Used by \`tests/jaxen.rs\`.
EOF
