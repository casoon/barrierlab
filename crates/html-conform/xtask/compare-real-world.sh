#!/usr/bin/env bash
# Compares html-conform's findings against a locally running vnu (Nu Html
# Checker) for every real-world page fetched by fetch-real-world.sh into
# xtask/.cache/real-world/.
#
# This is a supplementary, manual/periodic dev signal — separate from
# tests/differential.rs's pinned vnu-corpus baseline (tests/corpus/). Real
# pages carry unrelated, pre-existing markup errors, so unlike the corpus
# baseline this is NOT expected to reach 0 false positives; it's a coarse
# sanity check, not a regression gate. Never run in CI.
#
# vnu itself is the caller's responsibility: this script does not vendor,
# download, or install Java or vnu.jar. Point VNU_JAR at a local vnu.jar
# (https://github.com/validator/validator/releases) before running.
#
# Message text/wording isn't compared between html-conform and vnu (same
# reasoning as tests/differential.rs: different implementations, different
# vocabularies) — only finding counts and coarse category buckets.
#
# Requires: java (JRE/JDK), jq, a built VNU_JAR, and xtask/.cache/real-world
# populated via xtask/fetch-real-world.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cache_dir="${script_dir}/.cache/real-world"
check_file_manifest="${script_dir}/check-file/Cargo.toml"

for tool in java jq; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: required tool '${tool}' not found on PATH" >&2
        exit 1
    fi
done

if [[ -z "${VNU_JAR:-}" ]]; then
    echo "error: VNU_JAR is not set — point it at a local vnu.jar" \
        "(https://github.com/validator/validator/releases)" >&2
    exit 1
fi

if [[ ! -f "${VNU_JAR}" ]]; then
    echo "error: VNU_JAR (${VNU_JAR}) does not exist" >&2
    exit 1
fi

if [[ ! -d "${cache_dir}" ]] || [[ -z "$(find "${cache_dir}" -maxdepth 1 -name '*.html' -print -quit 2>/dev/null)" ]]; then
    echo "warning: ${cache_dir} is empty or missing — run" \
        "xtask/fetch-real-world.sh first" >&2
    exit 0
fi

echo "building check-file (release)..." >&2
cargo build --release --manifest-path "${check_file_manifest}" >&2

total_files=0
total_conform_findings=0
total_vnu_findings=0

for file in "${cache_dir}"/*.html; do
    [[ -e "${file}" ]] || continue
    total_files=$((total_files + 1))
    name="$(basename "${file}")"

    conform_json="$(cargo run --release --manifest-path "${check_file_manifest}" -- "${file}" 2>/dev/null || true)"
    conform_count=0
    conform_categories=""
    if [[ -n "${conform_json}" ]]; then
        conform_count="$(printf '%s\n' "${conform_json}" | grep -c '^{' || true)"
        conform_categories="$(printf '%s\n' "${conform_json}" | jq -r '.rule_id' 2>/dev/null | sort -u)"
    fi

    # ASSUMPTION: vnu's --format json message shape (top-level "messages"
    # array; each message has "type" ["error"|"info"|"non-document-error"],
    # optional "subtype", "message", "firstLine"/"lastLine",
    # "firstColumn"/"lastColumn") is taken from vnu's own documented wiki
    # page (github.com/validator/validator/wiki/Output-»-JSON), verified via
    # web search — not from a live `java -jar vnu.jar` run, since no JDK was
    # available when this script was written. --stdout is required because
    # vnu reports to stderr by default. Sanity-check this against a real
    # vnu.jar run before trusting the parsing below.
    vnu_json="$(java -jar "${VNU_JAR}" --format json --stdout "${file}" 2>/dev/null || true)"
    vnu_count=0
    vnu_categories=""
    if [[ -n "${vnu_json}" ]]; then
        vnu_count="$(printf '%s' "${vnu_json}" | jq '.messages | length' 2>/dev/null || echo 0)"
        vnu_categories="$(printf '%s' "${vnu_json}" | jq -r '.messages[] | (.type + (if .subtype then ":" + .subtype else "" end))' 2>/dev/null | sort -u)"
    fi

    only_conform="$(comm -23 <(printf '%s\n' "${conform_categories}") <(printf '%s\n' "${vnu_categories}") | grep -c . || true)"
    only_vnu="$(comm -13 <(printf '%s\n' "${conform_categories}") <(printf '%s\n' "${vnu_categories}") | grep -c . || true)"

    total_conform_findings=$((total_conform_findings + conform_count))
    total_vnu_findings=$((total_vnu_findings + vnu_count))

    echo "${name}: html-conform=${conform_count} vnu=${vnu_count}" \
        "categories-only-in-html-conform=${only_conform}" \
        "categories-only-in-vnu=${only_vnu}"
done

echo
echo "summary: files=${total_files}" \
    "total-html-conform-findings=${total_conform_findings}" \
    "total-vnu-findings=${total_vnu_findings}"
