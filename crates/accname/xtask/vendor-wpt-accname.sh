#!/usr/bin/env bash
set -euo pipefail

# Vendors the web-platform-tests accname name-computation cases without
# modifying them. Run from crates/accname.
#
# Source: https://github.com/web-platform-tests/wpt, path accname/name/
# License: BSD-3-Clause (upstream LICENSE.md)

readonly upstream_repository='https://github.com/web-platform-tests/wpt.git'
readonly upstream_revision='8a829e6ed64a2c6c995776c620bc34fa78a4032b'
readonly destination='tests/corpus/wpt'

if [[ -e "$destination/UPSTREAM.md" ]]; then
  echo "refusing to overwrite $destination" >&2
  exit 1
fi

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

mkdir -p "$destination"
cp -R "$temporary_directory/wpt/accname/name/." "$destination/"
rm -f "$destination/ReadMe.md" "$destination/WEB_FEATURES.yml"
cp "$temporary_directory/wpt/LICENSE.md" "$destination/LICENSE.md"

cat > "$destination/UPSTREAM.md" <<EOF
# web-platform-tests accname name-computation cases

Unmodified HTML test files from
[\`web-platform-tests/wpt\`](https://github.com/web-platform-tests/wpt), path
\`accname/name/\`.

- Upstream revision: \`$upstream_revision\`
- License: BSD-3-Clause; see the unmodified upstream \`LICENSE.md\`
  (SPDX header in \`REUSE.toml\`)
- Refresh procedure: remove this directory deliberately, then run
  \`xtask/vendor-wpt-accname.sh\` from \`crates/accname\`; changing the
  pinned revision requires re-blessing \`tests/wpt_known_failures.txt\`
  (\`WPT_BLESS=1 cargo test -p accname --test wpt\`).
- Used by \`tests/wpt.rs\`: every element with \`data-expectedlabel\` is one case.
EOF
