#!/usr/bin/env bash
# Vendor the SVG 1.1 and MathML 3 RELAX NG schema modules from
# validator/validator (vnu) into schema/svg11/ and schema/mml3/, as-is in
# RELAX NG compact syntax (.rnc) — no conversion. These are the modules
# vnu's real default schema entry point (`http://s.validator.nu/
# html5-all.rnc`, resolved via `schema/.drivers/html5-all.rnc` ->
# `html5-svg-mathml.rnc` -> `include "svg11/svg11-inc.rnc"`/`include
# "mml3/mathml3-inc.rnc"`) actually validates embedded <svg>/<math>
# content against — confirmed by reading vnu's own driver files and
# TestRunner.java's DEFAULT_SCHEMA constant, not assumed. schema/html5/
# (Phase 03) only vendors what schema/html5/html5.rnc's own include graph
# reaches, which never mentions SVG/MathML at all — that's not a
# vendoring gap, vnu's *modular* html5.rnc genuinely doesn't include them
# either; the integration only happens in the separate top-level driver.
#
# Source:  https://github.com/validator/validator
# Commit:  388cb365257d1410d4d32af960dba1cbd1b9af91 (tag 26.8.20) — same
#          commit as xtask/vendor-schema.sh and xtask/vendor-corpus.sh.
# License: SVG 1.1 and MathML 3 schema modules are each licensed under
#          their own embedded W3C Software Notice (with additional
#          Mozilla Foundation copyright on the MathML side) — see each
#          file's own header comment. Neither directory has a LICENSE
#          file of its own in the source repo (unlike schema/html5/,
#          which does — that one explicitly scopes its MIT grant to "all
#          files in this directory", i.e. schema/html5/ only).
#
# Requires: curl only.
#
# Usage: xtask/vendor-svg-mathml.sh

set -euo pipefail

VNU_COMMIT="388cb365257d1410d4d32af960dba1cbd1b9af91"
RAW_BASE="https://raw.githubusercontent.com/validator/validator/${VNU_COMMIT}/schema"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl not found on PATH" >&2
  exit 1
fi

# The integration driver: adds <svg>/<math> as phrasing content and wires
# up the ARIA-in-foreign-content patterns this crate's schema layer
# already remaps custom elements/data-*/etc. into
# (src/infoset.rs::merge_text_and_comment_runs used to unconditionally
# skip these subtrees — see plan/DECISIONS.md, this vendoring's entry).
curl -sSf -o "${REPO_ROOT}/schema/html5-svg-mathml.rnc" \
  "${RAW_BASE}/.drivers/html5-svg-mathml.rnc"
# REUSE-IgnoreStart
printf '# SPDX-License-Identifier: MIT\n# (schema/html5/LICENSE governs this file — it patches patterns\n# schema/html5/*.rnc defines, see this file'"'"'s own vendoring header.)\n' \
  | cat - "${REPO_ROOT}/schema/html5-svg-mathml.rnc" > "${REPO_ROOT}/schema/html5-svg-mathml.rnc.tmp"
# REUSE-IgnoreEnd
mv "${REPO_ROOT}/schema/html5-svg-mathml.rnc.tmp" "${REPO_ROOT}/schema/html5-svg-mathml.rnc"

# ---------------------------------------------------------------------
# SVG 1.1 (full profile — NOT the Basic/Tiny mobile profiles, which
# svg11-inc.rnc's own include graph never reaches): exactly the modules
# reachable from svg11-inc.rnc, computed by walking every `include
# "...rnc"` statement transitively (not hand-picked) — matches
# xtask/vendor-schema.sh's own "only what's reachable" precedent.
# ---------------------------------------------------------------------
SVG_MODULES=(
  svg-animation.rnc
  svg-animevents-attrib.rnc
  svg-basic-clip.rnc
  svg-basic-filter.rnc
  svg-basic-font.rnc
  svg-basic-graphics-attrib.rnc
  svg-basic-structure.rnc
  svg-basic-text.rnc
  svg-clip.rnc
  svg-conditional.rnc
  svg-container-attrib.rnc
  svg-core-attrib.rnc
  svg-cursor.rnc
  svg-datatypes.rnc
  svg-docevents-attrib.rnc
  svg-extensibility.rnc
  svg-extresources-attrib.rnc
  svg-filter.rnc
  svg-font.rnc
  svg-gradient.rnc
  svg-graphevents-attrib.rnc
  svg-graphics-attrib.rnc
  svg-hyperlink.rnc
  svg-image.rnc
  svg-marker.rnc
  svg-mask.rnc
  svg-opacity-attrib.rnc
  svg-paint-attrib.rnc
  svg-pattern.rnc
  svg-profile.rnc
  svg-script.rnc
  svg-shape.rnc
  svg-structure.rnc
  svg-style.rnc
  svg-text.rnc
  svg-view.rnc
  svg-viewport-attrib.rnc
  svg-xlink-attrib.rnc
  svg11-inc.rnc
)

SVG_DEST="${REPO_ROOT}/schema/svg11"
echo "==> downloading ${#SVG_MODULES[@]} SVG 1.1 .rnc modules from validator/validator@${VNU_COMMIT}"
mkdir -p "${SVG_DEST}"
rm -f "${SVG_DEST}"/*.rnc
for module in "${SVG_MODULES[@]}"; do
  curl -sSf -o "${SVG_DEST}/${module}" "${RAW_BASE}/svg11/${module}"
  printf '# License: W3C Software Notice (see this file'"'"'s own header comment)\n' \
    | cat - "${SVG_DEST}/${module}" > "${SVG_DEST}/${module}.tmp"
  mv "${SVG_DEST}/${module}.tmp" "${SVG_DEST}/${module}"
done

# ---------------------------------------------------------------------
# MathML 3: exactly the modules reachable from mathml3-inc.rnc (same
# transitive-walk method as SVG above).
# ---------------------------------------------------------------------
MATHML_MODULES=(
  mathml3-common.rnc
  mathml3-content.rnc
  mathml3-inc.rnc
  mathml3-presentation.rnc
  mathml3-strict-content.rnc
)

MATHML_DEST="${REPO_ROOT}/schema/mml3"
echo "==> downloading ${#MATHML_MODULES[@]} MathML 3 .rnc modules from validator/validator@${VNU_COMMIT}"
mkdir -p "${MATHML_DEST}"
rm -f "${MATHML_DEST}"/*.rnc
for module in "${MATHML_MODULES[@]}"; do
  curl -sSf -o "${MATHML_DEST}/${module}" "${RAW_BASE}/mml3/${module}"
  printf '# License: W3C Software Notice, additional Mozilla Foundation\n# copyright (see this file'"'"'s own header comment)\n' \
    | cat - "${MATHML_DEST}/${module}" > "${MATHML_DEST}/${module}.tmp"
  mv "${MATHML_DEST}/${module}.tmp" "${MATHML_DEST}/${module}"
done

echo "==> done: 1 driver + ${#SVG_MODULES[@]} SVG modules + ${#MATHML_MODULES[@]} MathML modules"
