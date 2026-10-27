# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases up to 0.2.1 are described
in the git history only.

## [Unreleased]

## [0.2.2] - 2026-10-27

### Fixed

- **Validator:** `validate` no longer recurses once per element nesting level; open elements are
  kept on a heap stack, so arbitrarily deep documents cannot overflow the call stack
  (auditmysite#783). Results and their order are unchanged.
