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
