#!/usr/bin/env bash
# Downloads a fixed list of real-world pages (xtask/real-world-sites.txt)
# into xtask/.cache/real-world/, for supplementary, non-CI conformance
# checking against actually-running websites.
#
# This is a manual/periodic dev step, not run in CI and not part of
# tests/differential.rs's pinned vnu-corpus baseline — real-world pages
# drift over time (redesigns, unrelated markup errors), so this is a
# separate, best-effort signal, not a regression gate.
#
# Requires: curl.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sites_file="${script_dir}/real-world-sites.txt"
out_dir="${script_dir}/.cache/real-world"

for tool in curl; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: required tool '${tool}' not found on PATH" >&2
        exit 1
    fi
done

if [[ ! -f "${sites_file}" ]]; then
    echo "error: sites file not found: ${sites_file}" >&2
    exit 1
fi

mkdir -p "${out_dir}"

slugify() {
    local url="$1"
    local slug="${url#*://}"
    slug="${slug//\//-}"
    slug="${slug//./-}"
    slug="${slug//:/-}"
    printf '%s' "${slug}"
}

while IFS= read -r url; do
    [[ -z "${url}" ]] && continue
    [[ "${url}" == \#* ]] && continue

    slug="$(slugify "${url}")"
    dest="${out_dir}/${slug}.html"

    echo "fetching ${url} -> ${dest}"
    if ! curl --fail --silent --show-error --location --max-time 30 \
        --output "${dest}" "${url}"; then
        echo "warning: failed to fetch ${url}, skipping" >&2
        rm -f "${dest}"
        continue
    fi
done <"${sites_file}"

echo "done: fetched pages into ${out_dir}"
