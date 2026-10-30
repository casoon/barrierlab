# Changelog

All notable changes to this crate are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Release 0.1.0 is described in the
git history only.

## [Unreleased]

### Fixed

- **Source lists:** `'none'`, keyword sources, the `'nonce-` prefix and hash algorithm names
  match ASCII case-insensitively, so `'UNSAFE-INLINE'` or `'SHA256-…'` are recognised; nonce and
  hash values keep their case (#38). `parse_hash_expression` accepts upper-case algorithm names
  for the same reason.
