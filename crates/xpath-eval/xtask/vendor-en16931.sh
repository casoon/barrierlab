#!/usr/bin/env bash
set -euo pipefail

# Fetches the preprocessed EN 16931 validation Schematron files (UBL and CII,
# queryBinding="xslt2") into tests/corpus/en16931 on demand. The directory is
# gitignored: the files are EUPL-1.2 licensed and are intentionally NOT part of
# this repository. They are only the source of XPath expressions for the
# XPath 2.0 coverage measurement (see CONFORMANCE.md). Then:
#   cargo test -p xpath-eval --test en16931 -- --ignored --nocapture
# Also fetches two UBL example invoices of the same tag (tests/corpus/en16931/examples), the
# input of the end-to-end measurement of schematron-engine (tests/en16931.rs, --ignored).
#
# Source: https://github.com/ConnectingEurope/eInvoicing-EN16931, tag
# validation-1.3.17 (commit d0dea7300c2ad60d50af4a4f31e122fdbf7c0ed3)
# License: EUPL-1.2

readonly tag='validation-1.3.17'
readonly base="https://raw.githubusercontent.com/ConnectingEurope/eInvoicing-EN16931/$tag"
readonly ubl_sha256='64b93f26acadd19de9792cec9e085628271c6fe3870ecacbf4e7271272f2ef08'
readonly cii_sha256='74fa94df0513881b9c8e369650bb2282f9c8a30bb52ce4ce43d1219e88ccedc2'
script_directory=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
readonly destination="$script_directory/../tests/corpus/en16931"

readonly examples_sha256_1='507a03e3c45761c435cf81e4a32097bedb3cb9b724572a9989028a4dfc2c7b51'
readonly examples_sha256_2='1137eccac470c19b67706d6d9c568450ebb9e5596487e73f50f5c8164fc13506'

mkdir -p "$destination"
if [[ -e "$destination/ubl.sch" ]]; then
  echo "$destination/ubl.sch already present; remove it to fetch again" >&2
else
  trap 'rm -f "$destination"/ubl.sch "$destination"/cii.sch' ERR
  curl -sfL "$base/ubl/schematron/preprocessed/EN16931-UBL-validation-preprocessed.sch" -o "$destination/ubl.sch"
  curl -sfL "$base/cii/schematron/preprocessed/EN16931-CII-validation-preprocessed.sch" -o "$destination/cii.sch"
  curl -sfL "$base/LICENSE.txt" -o "$destination/LICENSE.txt"

  if [[ $(shasum -a 256 "$destination/ubl.sch" | cut -d' ' -f1) != "$ubl_sha256" ]]; then
    echo 'ubl.sch does not match the pinned checksum' >&2
    exit 1
  fi
  if [[ $(shasum -a 256 "$destination/cii.sch" | cut -d' ' -f1) != "$cii_sha256" ]]; then
    echo 'cii.sch does not match the pinned checksum' >&2
    exit 1
  fi
  echo "fetched EN 16931 $tag into $destination"
fi

if [[ ! -e "$destination/examples/ubl-tc434-example1.xml" ]]; then
  mkdir -p "$destination/examples"
  trap 'rm -rf "$destination"/examples' ERR
  curl -sfL "$base/ubl/examples/ubl-tc434-example1.xml" -o "$destination/examples/ubl-tc434-example1.xml"
  curl -sfL "$base/ubl/examples/ubl-tc434-example2.xml" -o "$destination/examples/ubl-tc434-example2.xml"
  if [[ $(shasum -a 256 "$destination/examples/ubl-tc434-example1.xml" | cut -d' ' -f1) != "$examples_sha256_1" ||
        $(shasum -a 256 "$destination/examples/ubl-tc434-example2.xml" | cut -d' ' -f1) != "$examples_sha256_2" ]]; then
    echo 'an example invoice does not match the pinned checksum' >&2
    exit 1
  fi
  echo "fetched the UBL example invoices into $destination/examples"
fi

# --- Peppol BIS Billing 3.0 and XRechnung (issue #62, the xsl:function bridge) -------------------
# The rule sets of both call helper functions declared with xsl:function (check sums, IBAN), so
# they run end to end only with the function bridge of schematron-engine
# (cargo test -p schematron-engine --test en16931 -- --ignored). Every file is pinned by
# SHA-256 and fetched on demand, never committed. Licences: Peppol BIS Billing 3.0 rules and
# examples (OpenPEPPOL, Creative Commons Attribution 4.0), XRechnung Schematron and test suite
# (KoSIT, Apache-2.0), the CII example (EN 16931 validation artefacts, EUPL-1.2).
#
# Sources:
#   OpenPEPPOL/peppol-bis-invoice-3, commit 38d693d71d28d813781323a6ca97324edcc4d239
#   itplr-kosit/xrechnung-schematron, commit 7e3ddbe224674427fd174a8406635708b0d50ed1
#   itplr-kosit/xrechnung-testsuite, commit 1ce1dafffd035e5ce59a7ab0d3bf44ac14af16c9

trap - ERR
readonly peppol_base='https://raw.githubusercontent.com/OpenPEPPOL/peppol-bis-invoice-3/38d693d71d28d813781323a6ca97324edcc4d239'
readonly xr_base='https://raw.githubusercontent.com/itplr-kosit/xrechnung-schematron/7e3ddbe224674427fd174a8406635708b0d50ed1/src/validation/schematron'
readonly xr_tests='https://raw.githubusercontent.com/itplr-kosit/xrechnung-testsuite/1ce1dafffd035e5ce59a7ab0d3bf44ac14af16c9/src/test/business-cases/standard'

# fetch_pinned <url> <sha256> <destination file>
fetch_pinned() {
  local url=$1 sha=$2 target=$3
  if [[ -e "$target" ]]; then
    return
  fi
  mkdir -p "$(dirname "$target")"
  curl -sfL "$url" -o "$target.part"
  if [[ $(shasum -a 256 "$target.part" | cut -d' ' -f1) != "$sha" ]]; then
    rm -f "$target.part"
    echo "$target does not match the pinned checksum" >&2
    exit 1
  fi
  mv "$target.part" "$target"
  echo "fetched $target"
}

readonly peppol_destination="$destination/../peppol"
fetch_pinned "$peppol_base/rules/sch/PEPPOL-EN16931-UBL.sch" f637444fab75a6da7e5004f94a6de8e71383ef5658dafe6c68925103b17ea148 "$peppol_destination/ubl.sch"
fetch_pinned "$peppol_base/rules/sch/PEPPOL-EN16931-CII.sch" 7678c3cd4a219095ef5d4f2ece17e5d447eb92655e57a7d52290b35acfdc9dd8 "$peppol_destination/cii.sch"
fetch_pinned "$peppol_base/rules/examples/base-example.xml" 1b7cc3ff1834c8963f2c93f30f171b58002cbf0b2c52dc8765e7e83aebb9f7c9 "$peppol_destination/examples/base-example.xml"
fetch_pinned "$peppol_base/rules/examples/Allowance-example.xml" aa3df18eb8c634624637eb229891d989c5cfb7cd0d08894ff8e58c58f247ea5b "$peppol_destination/examples/Allowance-example.xml"
# Peppol publishes no CII example: an EN 16931 CII example, whose guideline id the test replaces.
fetch_pinned "$base/cii/examples/CII_example1.xml" 0c12e3ca9aab58299e6271b89d061274694c62159510ca2d848f13d287ee4f99 "$peppol_destination/examples/CII_example1.xml"

readonly xrechnung_destination="$destination/../xrechnung"
fetch_pinned "$xr_base/ubl/XRechnung-UBL-validation.sch" fdf6b930cfdf3dd44f8bf881d0043ffb2d494dc3a3ed18f729c1d4143a993dd2 "$xrechnung_destination/ubl/XRechnung-UBL-validation.sch"
fetch_pinned "$xr_base/cii/XRechnung-CII-validation.sch" efc58eaa9d215d8d7c94557e1ed37de068ddb1f97fcfd1c5da800c06223a97c3 "$xrechnung_destination/cii/XRechnung-CII-validation.sch"
fetch_pinned "$xr_base/common.sch" 7443d2f4c12d3e1ffa6b38f7f83cea89a9b03343f31f726083018d8985492a12 "$xrechnung_destination/common.sch"
fetch_pinned "$xr_tests/01.01a-INVOICE_ubl.xml" 74fb09c609d5fba15a8c543060998d3b92858f56a81fb5b0ed244d6794e498d1 "$xrechnung_destination/examples/01.01a-INVOICE_ubl.xml"
fetch_pinned "$xr_tests/01.01a-INVOICE_uncefact.xml" 727b51982a84c9b406599a7384783570910440ed405c9bb6d918b22442636886 "$xrechnung_destination/examples/01.01a-INVOICE_uncefact.xml"
fetch_pinned "$xr_tests/02.01a-INVOICE_ubl.xml" 942f3ac502fdebe48e3f1e130dd75174e9eb8c4b982b92f2664b14cf5dfc6149 "$xrechnung_destination/examples/02.01a-INVOICE_ubl.xml"
fetch_pinned "$xr_tests/02.01a-INVOICE_uncefact.xml" b0bc3f139d52d6717e35415fb4ddbd91fe656eb431da41ef41c7edcd911005e3 "$xrechnung_destination/examples/02.01a-INVOICE_uncefact.xml"
fetch_pinned "$xr_tests/03.01a-INVOICE_ubl.xml" f8a2b313d823e3d0319fa878f3b9da47d844e5f5b0c19f2583e621d93df2b81e "$xrechnung_destination/examples/03.01a-INVOICE_ubl.xml"
fetch_pinned "$xr_tests/03.01a-INVOICE_uncefact.xml" f42a6b02d0acada2bd9e9d1520b88385878f2377cb99444f0e87052eafb1e1de "$xrechnung_destination/examples/03.01a-INVOICE_uncefact.xml"
