# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-08-29

### Added
- **Core Engine:** HTML5 conformance checking library combining 5 validation layers (Parser, RELAX NG Schema, Datatype Microsyntaxes, Schematron Co-constraints, JSON/CSP validators).
- **Schema Layer:** Full vendored W3C RELAX NG schemas for HTML5, SVG 1.1, and MathML 3 via `relax-ng`.
- **Datatype Library:** 50 custom W3C attribute datatype checkers (`w:image-candidate-strings` for `srcset`, `w:content-security-policy`, `w:media-query`, `w:datetime`, `w:iri-ref`, BCP 47 language tags).
- **Assertions:** Schematron rule engine with 100+ co-constraint rules across ARIA 1.2, HTML structural restrictions, element nesting, heading hierarchy, and link/script attribute combinations.
- **Dedicated Script/CSP Validators:** Spec-compliant JSON checkers for `<script type="importmap">` and `<script type="speculationrules">`, plus Content Security Policy enforcement for `<meta http-equiv="Content-Security-Policy">`.
- **Differential Test Suite:** 4,655 W3C/vnu corpus test fixtures with 98.8% accuracy and 0 False Positives.
- **Compliance & Tooling:** REUSE compliance, `deny.toml` for `cargo-deny`, and `THIRD-PARTY-NOTICES.md`.
