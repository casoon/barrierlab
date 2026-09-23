#!/usr/bin/env bash
# Vendor the HTML5 RELAX NG schema from validator/validator (vnu) into
# schema/html5/, as-is in RELAX NG compact syntax (.rnc) — no conversion.
#
# Source:  https://github.com/validator/validator
# Commit:  388cb365257d1410d4d32af960dba1cbd1b9af91 (tag 26.8.20)
# License: MIT (schema/html5/LICENSE in the source repo)
#
# Kept in .rnc, unconverted: schema.rs (Phase 05) will be an in-house
# RELAX NG engine (architecturally informed by, but not derived from,
# dholroyd/relaxng-rust — that project is technically strong but
# unlicensed, see plan/DECISIONS.md), reading .rnc natively and handling
# <include>/<div>/`combine` per spec. An earlier version of this script
# ran the schema through Trang + rnginline + a hand-rolled flattener to
# work around limitations in xmloxide's RelaxNG parser (see the upstream
# issues filed against jonwiggins/xmloxide: #52, #53, #54, #55, #56) —
# that whole detour is moot now that xmloxide isn't the RelaxNG engine.
#
# Requires: curl only.
#
# Usage: xtask/vendor-schema.sh

set -euo pipefail

VNU_COMMIT="388cb365257d1410d4d32af960dba1cbd1b9af91"
RAW_BASE="https://raw.githubusercontent.com/validator/validator/${VNU_COMMIT}/schema/html5"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEST_DIR="${REPO_ROOT}/schema/html5"

if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl not found on PATH" >&2
  exit 1
fi

# RELAX NG compact-syntax modules that make up the vnu HTML5 schema.
# html5.rnc is the entry point; the others are pulled in via `include`.
# (xhtml5.rnc, rdfa.rnc, html5exclusions.rnc, web-forms-scripting.rnc and
# web-forms2-scripting.rnc exist in the source directory but are not
# reachable from html5.rnc's include graph -- they belong to the XHTML5 /
# RDFa / scripting-profile variants this crate does not target yet.)
MODULES=(
  applications.rnc
  aria.rnc
  block.rnc
  common.rnc
  core-scripting.rnc
  data.rnc
  embed.rnc
  form-datatypes.rnc
  html5.rnc
  media.rnc
  meta.rnc
  microdata.rnc
  phrase.rnc
  revision.rnc
  ruby.rnc
  sectional.rnc
  structural.rnc
  tables.rnc
  web-components.rnc
  web-forms.rnc
  web-forms2.rnc
)

echo "==> downloading ${#MODULES[@]} .rnc modules from validator/validator@${VNU_COMMIT}"
mkdir -p "${DEST_DIR}"
rm -f "${DEST_DIR}"/*.rnc
for module in "${MODULES[@]}"; do
  curl -sSf -o "${DEST_DIR}/${module}" "${RAW_BASE}/${module}"
  # REUSE-IgnoreStart
  printf '# SPDX-License-Identifier: MIT\n' | cat - "${DEST_DIR}/${module}" > "${DEST_DIR}/${module}.tmp"
  # REUSE-IgnoreEnd
  mv "${DEST_DIR}/${module}.tmp" "${DEST_DIR}/${module}"
done
curl -sSf -o "${DEST_DIR}/LICENSE" "${RAW_BASE}/LICENSE"

echo "==> done: ${#MODULES[@]} .rnc modules in ${DEST_DIR}"
