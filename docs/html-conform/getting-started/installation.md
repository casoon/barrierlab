---
title: "Installation"
description: "html-conform is published on crates.io, as a library and, with the cli feature, as a command-line tool."
order: 1
---

## Add the crate

```sh
cargo add html-conform
```

or in `Cargo.toml`:

```toml
[dependencies]
html-conform = "0.6"
```

## Requirements

- Rust **1.88** or newer, as declared in `Cargo.toml` (`rust-version = "1.88.0"`, edition
  2024).
- Nothing at runtime. The HTML schema (as a serialized, already compiled pattern) and the Schematron rules are embedded in the crate,
  so there are no files to ship next to your binary, and no Java, subprocess or network
  access is involved.

## Command line

The crate also builds a command-line tool, behind the `cli` feature (off by default, no extra
dependency):

```sh
cargo install html-conform --features cli
html-conform dist/
```

See [Command line](../../guides/cli/) for options, exit codes and the SARIF and JUnit output.
To check documents from your own code, call the library as the [Quickstart](../quickstart/)
shows.
