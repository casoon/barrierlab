#!/usr/bin/env bash
set -euo pipefail

# Vendors two web-platform-tests media-query files without modifying them.
# Run from crates/media-query-parse.
#
# Source: https://github.com/web-platform-tests/wpt, path css/mediaqueries/
# License: BSD-3-Clause (upstream LICENSE.md)

readonly upstream_repository='https://github.com/web-platform-tests/wpt.git'
readonly upstream_revision='fc7c47fcb9172a216914879460e98018348ee3c5'
readonly destination='tests/corpus/wpt'

if [[ -e "$destination/UPSTREAM.md" ]]; then
  echo "refusing to overwrite $destination" >&2
  exit 1
fi

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT

git -C "$temporary_directory" init --quiet wpt
git -C "$temporary_directory/wpt" remote add origin "$upstream_repository"
git -C "$temporary_directory/wpt" sparse-checkout set --no-cone \
  /css/mediaqueries/test_media_queries.html /css/mediaqueries/match-media-parsing.html /LICENSE.md
git -C "$temporary_directory/wpt" fetch --quiet --depth 1 --filter=blob:none origin "$upstream_revision"
git -C "$temporary_directory/wpt" checkout --quiet --detach FETCH_HEAD

if [[ $(git -C "$temporary_directory/wpt" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

mkdir -p "$destination"
cp "$temporary_directory/wpt/css/mediaqueries/test_media_queries.html" "$destination/"
cp "$temporary_directory/wpt/css/mediaqueries/match-media-parsing.html" "$destination/"
cp "$temporary_directory/wpt/LICENSE.md" "$destination/LICENSE.md"

cat > "$destination/UPSTREAM.md" <<EOF
# web-platform-tests media-query parsing cases

Unmodified HTML test files from
[\`web-platform-tests/wpt\`](https://github.com/web-platform-tests/wpt), path
\`css/mediaqueries/\`: \`test_media_queries.html\` and \`match-media-parsing.html\`.

- Upstream revision: \`$upstream_revision\`
- License: BSD-3-Clause; see the unmodified upstream \`LICENSE.md\`
  (SPDX header in \`REUSE.toml\`)
- Refresh procedure: remove this directory deliberately, then run
  \`xtask/vendor-wpt-mediaqueries.sh\` from \`crates/media-query-parse\`; changing
  the pinned revision requires re-blessing \`tests/wpt_known_failures.txt\`
  (\`WPT_BLESS=1 cargo test -p media-query-parse --test wpt\`).
- Used by \`tests/wpt.rs\`: every call with a single string literal argument is
  one case; calls built from loops or template strings need a JavaScript engine
  and are not run.
EOF
