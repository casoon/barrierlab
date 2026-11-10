#!/usr/bin/env bash
set -euo pipefail

# Vendors the web-platform-tests accname name-computation cases without
# modifying them.
#
# Source: https://github.com/web-platform-tests/wpt, path accname/name/
# License: BSD-3-Clause (upstream LICENSE.md)
#
# Usage: xtask/vendor-wpt-accname.sh [--check]
#   no argument  replaces the corpus with the pinned upstream revision
#   --check      vendors into a temp dir and diffs it against the committed
#                corpus (exit 1 on any difference); needs network

readonly upstream_repository='https://github.com/web-platform-tests/wpt.git'
readonly upstream_revision='8a829e6ed64a2c6c995776c620bc34fa78a4032b'

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
git -C "$temporary_directory/wpt" sparse-checkout set --no-cone /accname/name /LICENSE.md
git -C "$temporary_directory/wpt" fetch --quiet --depth 1 --filter=blob:none origin "$upstream_revision"
git -C "$temporary_directory/wpt" checkout --quiet --detach FETCH_HEAD

if [[ $(git -C "$temporary_directory/wpt" rev-parse HEAD) != "$upstream_revision" ]]; then
  echo 'checked out revision does not match the pinned upstream revision' >&2
  exit 1
fi

readonly staging="$temporary_directory/staging"
mkdir -p "$staging"
cp -R "$temporary_directory/wpt/accname/name/." "$staging/"
rm -f "$staging/ReadMe.md" "$staging/WEB_FEATURES.yml"
cp "$temporary_directory/wpt/LICENSE.md" "$staging/LICENSE.md"

cat > "$staging/UPSTREAM.md" <<EOF
# web-platform-tests accname name-computation cases

Unmodified HTML test files from
[\`web-platform-tests/wpt\`](https://github.com/web-platform-tests/wpt), path
\`accname/name/\`.

- Upstream revision: \`$upstream_revision\`
- License: BSD-3-Clause; see the unmodified upstream \`LICENSE.md\`
  (SPDX header in \`REUSE.toml\`)
- Refresh procedure: run \`xtask/vendor-wpt-accname.sh\` (\`--check\` compares
  without writing); changing the
  pinned revision requires re-blessing \`tests/wpt_known_failures.txt\`
  (\`WPT_BLESS=1 cargo test -p accname --test wpt\`).
- Used by \`tests/wpt.rs\`: every element with \`data-expectedlabel\` is one case.
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
