#!/usr/bin/env bash
# Compares html-conform's finding counts against a locally running vnu (Nu
# Html Checker) for every page of the pinned snapshot made by
# fetch-real-world.sh, and diffs the result against the committed baseline
# xtask/real-world-baseline.tsv.
#
# Manual / on-tag dev signal. Never run in per-push CI. Real pages carry
# their own markup errors, so the counts are not expected to reach 0 false
# positives; the point is that a change between two runs on the same
# snapshot is visible and reviewable.
#
# Usage:
#   compare-real-world.sh                  print table, diff against baseline
#   compare-real-world.sh --check          same, exit 1 if anything changed
#   compare-real-world.sh --update-baseline  rewrite the baseline from this run
#
# vnu: set VNU_JAR to a local vnu.jar (needs java), or VNU_JSON_DIR to a
# directory of pre-recorded `<slug>.json` outputs (vnu --format json). With
# neither, xtask/.cache/vnu.jar is used if present (gitignored) and java works. The jar is
# verified against the pinned SHA-256 below (vnu 26.8.20, the version the
# vendored schema is pinned to; npm package vnu-jar@26.8.20, build/dist/vnu.jar);
# a mismatch aborts. With none of these, the vnu column is "-" (NotRun, not 0) and only html-conform is
# compared. A "-" never overwrites a recorded vnu count in the baseline.
#
# Output is deterministic for a given snapshot: no timings, no paths, sorted
# by page name. Each page carries the first 12 hex digits of its sha256; if
# that differs from the baseline the page itself changed and its counts are
# reported as "snapshot changed", not as a regression.
#
# Env overrides (used by test-real-world.sh): RW_SNAPSHOT_DIR, RW_BASELINE,
# RW_CHECK_FILE (command printing one JSON object per finding).
#
# Requires: jq, shasum or sha256sum; java + VNU_JAR when running vnu.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cache_dir="${RW_SNAPSHOT_DIR:-${script_dir}/.cache/real-world}"
baseline="${RW_BASELINE:-${script_dir}/real-world-baseline.tsv}"
check_file_manifest="${script_dir}/check-file/Cargo.toml"

mode=report
case "${1:-}" in
    "") ;;
    --check) mode=check ;;
    --update-baseline) mode=update ;;
    *) echo "usage: $0 [--check|--update-baseline]" >&2; exit 2 ;;
esac

command -v jq >/dev/null 2>&1 || { echo "error: 'jq' not found on PATH" >&2; exit 1; }

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -c1-12
    else shasum -a 256 "$1" | cut -c1-12; fi
}

if [[ -z "$(find "${cache_dir}" -maxdepth 1 -name '*.html' -print -quit 2>/dev/null)" ]]; then
    echo "error: no snapshot in ${cache_dir} — run xtask/fetch-real-world.sh first" >&2
    exit 1
fi

VNU_JAR_SHA256="582b3506ab5bb76a7cb7c78930c427720856e79497084f69847a4ebf3f9d217f"
if [[ -z "${VNU_JAR:-}" && -z "${VNU_JSON_DIR:-}" && -f "${script_dir}/.cache/vnu.jar" ]] \
    && java -version >/dev/null 2>&1; then
    VNU_JAR="${script_dir}/.cache/vnu.jar"
fi

vnu_mode=none
if [[ -n "${VNU_JSON_DIR:-}" ]]; then
    vnu_mode=json
elif [[ -n "${VNU_JAR:-}" ]]; then
    if ! java -version >/dev/null 2>&1; then
        echo "error: VNU_JAR set but no working java" >&2
        exit 1
    fi
    [[ -f "${VNU_JAR}" ]] || { echo "error: VNU_JAR (${VNU_JAR}) does not exist" >&2; exit 1; }
    jar_sha="$( (sha256sum "${VNU_JAR}" 2>/dev/null || shasum -a 256 "${VNU_JAR}") | cut -d' ' -f1)"
    if [[ "${jar_sha}" != "${VNU_JAR_SHA256}" ]]; then
        echo "error: ${VNU_JAR} has sha256 ${jar_sha}, expected ${VNU_JAR_SHA256} (vnu 26.8.20)" >&2
        exit 1
    fi
    vnu_mode=jar
fi

if [[ -n "${RW_CHECK_FILE:-}" ]]; then
    check_cmd=("${RW_CHECK_FILE}")
else
    echo "building check-file (release)..." >&2
    cargo build --release --quiet --manifest-path "${check_file_manifest}" >&2
    target_dir="${CARGO_TARGET_DIR:-${script_dir}/check-file/target}"
    check_cmd=("${target_dir}/release/check-file")
fi

current="$(mktemp)"
trap 'rm -f "${current}" "${current}.none"' EXIT

for file in "${cache_dir}"/*.html; do
    name="$(basename "${file}" .html)"
    hash="$(sha256 "${file}")"

    conform_count="$("${check_cmd[@]}" "${file}" 2>/dev/null | grep -c '^{' || true)"

    vnu_count="-"
    case "${vnu_mode}" in
        jar)
            vnu_json="$(java -jar "${VNU_JAR}" --format json --stdout "${file}" 2>/dev/null || true)"
            vnu_count="$(printf '%s' "${vnu_json}" | jq '.messages | length' 2>/dev/null || echo "-")" ;;
        json)
            if [[ -f "${VNU_JSON_DIR}/${name}.json" ]]; then
                vnu_count="$(jq '.messages | length' "${VNU_JSON_DIR}/${name}.json" 2>/dev/null || echo "-")"
            fi ;;
    esac

    printf '%s\t%s\t%s\t%s\n' "${name}" "${hash}" "${conform_count}" "${vnu_count}" >>"${current}"
done
LC_ALL=C sort -o "${current}" "${current}"

echo "== current run (page, sha256[:12], html-conform, vnu; '-' = not run) =="
cat "${current}"
echo

baseline_or_empty="${baseline}"
if [[ ! -f "${baseline}" ]]; then
    baseline_or_empty="${current}.none"
    echo "# none" >"${baseline_or_empty}"
fi

# Diff against the baseline. For vnu, "-" on either side means "no value":
# not compared, not a change.
report="$(LC_ALL=C awk -F'\t' '
    function fmt(c, v) { return "html-conform=" c " vnu=" v }
    FNR == NR { if ($0 !~ /^#/ && NF >= 4 && $1 != "page") { bh[$1]=$2; bc[$1]=$3; bv[$1]=$4 } next }
    { seen[$1]=1
      if (!($1 in bh)) { print "new page      " $1 "  " fmt($3, $4); next }
      if (bh[$1] != $2) { print "snapshot changed " $1 "  (sha " bh[$1] " -> " $2 "), counts not comparable"; next }
      d = ""
      if (bc[$1] != $3) d = d " html-conform " bc[$1] " -> " $3
      if (bv[$1] != "-" && $4 != "-" && bv[$1] != $4) d = d " vnu " bv[$1] " -> " $4
      if (d != "") print "changed       " $1 " " d
    }
    END { for (p in bh) if (!(p in seen)) print "missing page  " p " (in baseline, not in snapshot)" }
' "${baseline_or_empty}" "${current}" | LC_ALL=C sort)"

if [[ ! -f "${baseline}" ]]; then
    report="no baseline at ${baseline} — run with --update-baseline"
    changed=1
elif [[ -z "${report}" ]]; then
    report="no changes against baseline"
    changed=0
else
    changed=1
fi

echo "== against baseline =="
printf '%s\n' "${report}"

if [[ "${mode}" == update ]]; then
    old_baseline="${baseline}"
    if [[ ! -f "${old_baseline}" ]]; then
        old_baseline="${current}.none"
        echo "# none" >"${old_baseline}"
    fi
    {
        echo "# html-conform vs vnu finding counts per real-world page."
        echo "# Snapshot is not in git (third-party pages); sha256[:12] pins its bytes."
        echo "# vnu '-' = not run. Regenerate: xtask/compare-real-world.sh --update-baseline"
        printf 'page\tsha256\thtml-conform\tvnu\n'
        # Keep a recorded vnu count where this run had none and the page is unchanged.
        LC_ALL=C awk -F'\t' '
            FNR == NR { if ($0 !~ /^#/ && $1 != "page" && NF >= 4) { bh[$1]=$2; bv[$1]=$4 } next }
            { v=$4; if (v == "-" && ($1 in bh) && bh[$1] == $2) v = bv[$1]
              printf "%s\t%s\t%s\t%s\n", $1, $2, $3, v }
        ' "${old_baseline}" "${current}"
    } >"${baseline}.new"
    mv "${baseline}.new" "${baseline}"
    echo "baseline written: ${baseline}"
elif [[ "${mode}" == check && "${changed}" -ne 0 ]]; then
    exit 1
fi
