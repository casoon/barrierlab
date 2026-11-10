#!/usr/bin/env bash
set -euo pipefail

# Vendors two web-platform-tests media-query files without modifying them.
#
#
# Source: https://github.com/web-platform-tests/wpt, path css/mediaqueries/
# License: BSD-3-Clause (upstream LICENSE.md)
#
# Usage: xtask/vendor-wpt-mediaqueries.sh [--check]
#   no argument  replaces the corpus with the pinned upstream revision
#   --check      vendors into a temp dir and diffs it against the committed
#                corpus (exit 1 on any difference); needs network

readonly upstream_repository='https://github.com/web-platform-tests/wpt.git'
readonly upstream_revision='fc7c47fcb9172a216914879460e98018348ee3c5'

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
readonly destination="$script_directory/../tests/corpus/wpt"

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

readonly staging="$temporary_directory/staging"
mkdir -p "$staging"
cp "$temporary_directory/wpt/css/mediaqueries/test_media_queries.html" "$staging/"
cp "$temporary_directory/wpt/css/mediaqueries/match-media-parsing.html" "$staging/"
cp "$temporary_directory/wpt/LICENSE.md" "$staging/LICENSE.md"

cat > "$staging/UPSTREAM.md" <<EOF
# web-platform-tests media-query parsing cases

Unmodified HTML test files from
[\`web-platform-tests/wpt\`](https://github.com/web-platform-tests/wpt), path
\`css/mediaqueries/\`: \`test_media_queries.html\` and \`match-media-parsing.html\`.

- Upstream revision: \`$upstream_revision\`
- License: BSD-3-Clause; see the unmodified upstream \`LICENSE.md\`
  (SPDX header in \`REUSE.toml\`)
- Refresh procedure: run \`xtask/vendor-wpt-mediaqueries.sh\` (\`--check\` compares
  without writing); changing
  the pinned revision requires re-blessing \`tests/wpt_known_failures.txt\`
  (\`WPT_BLESS=1 cargo test -p media-query-parse --test wpt\`).
- Used by \`tests/wpt.rs\`: every call with a single string literal argument is
  one case; calls built from loops or template strings need a JavaScript engine
  and are not run.
EOF

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
