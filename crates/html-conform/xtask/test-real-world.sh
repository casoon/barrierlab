#!/usr/bin/env bash
# Tests the baseline/diff logic of compare-real-world.sh without Java, vnu or
# network: synthetic snapshot, a stub checker and canned vnu JSON, all in a
# temp dir. This does NOT exercise vnu or html-conform themselves.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
compare="${script_dir}/compare-real-world.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

export RW_SNAPSHOT_DIR="${tmp}/snap" RW_BASELINE="${tmp}/baseline.tsv"
mkdir -p "${RW_SNAPSHOT_DIR}" "${tmp}/vnu"
printf '<p>a\n' >"${RW_SNAPSHOT_DIR}/one.html"
printf '<p>b\n<p>b\n' >"${RW_SNAPSHOT_DIR}/two.html"

# Stub checker: one finding per line of the input file.
cat >"${tmp}/check" <<'STUB'
#!/usr/bin/env bash
while IFS= read -r _; do echo '{"rule_id":"x"}'; done <"$1"
STUB
chmod +x "${tmp}/check"
export RW_CHECK_FILE="${tmp}/check"

echo '{"messages":[{},{}]}' >"${tmp}/vnu/one.json"
echo '{"messages":[{}]}' >"${tmp}/vnu/two.json"
export VNU_JSON_DIR="${tmp}/vnu"

fail() { echo "FAIL: $*" >&2; exit 1; }

"${compare}" --update-baseline >/dev/null
grep -q $'^one\t.*\t1\t2$' "${RW_BASELINE}" || fail "baseline row for one"

"${compare}" >"${tmp}/run1"
"${compare}" >"${tmp}/run2"
cmp "${tmp}/run1" "${tmp}/run2" || fail "two runs differ"
grep -q "no changes against baseline" "${tmp}/run1" || fail "expected no changes"
"${compare}" --check >/dev/null || fail "--check should pass when unchanged"

# html-conform count change on an unchanged page is reported.
sed -i.bak $'s/^one\t\\(.*\\)\t1\t2$/one\t\\1\t5\t2/' "${RW_BASELINE}"
"${compare}" >"${tmp}/run3"
grep -q "changed       one  html-conform 5 -> 1" "${tmp}/run3" || fail "count change not reported"
"${compare}" --check >/dev/null && fail "--check should fail on change"

# Without vnu: not-run '-' is not a change and does not erase the baseline.
"${compare}" --update-baseline >/dev/null
unset VNU_JSON_DIR
"${compare}" >"${tmp}/run4"
grep -q "no changes against baseline" "${tmp}/run4" || fail "'-' treated as change"
"${compare}" --update-baseline >/dev/null
grep -q $'^one\t.*\t1\t2$' "${RW_BASELINE}" || fail "recorded vnu count lost"

# A changed page is reported as snapshot change, not as a regression.
printf '<p>a\n<p>a\n' >"${RW_SNAPSHOT_DIR}/one.html"
"${compare}" >"${tmp}/run5"
grep -q "snapshot changed one" "${tmp}/run5" || fail "snapshot change not reported"

echo "ok: baseline/diff logic (no java, no vnu, no network)"
