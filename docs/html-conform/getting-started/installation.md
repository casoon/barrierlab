---
title: "Installation"
description: "html-conform is published on crates.io. It is a library crate; there is no binary to install."
order: 1
---

## Add the crate

```sh
cargo add html-conform
```

or in `Cargo.toml`:

```toml
[dependencies]
html-conform = "0.2.1"
```

## Requirements

- Rust **1.88** or newer, as declared in `Cargo.toml` (`rust-version = "1.88.0"`, edition
  2024).
- Nothing at runtime. The HTML schema and the Schematron rules are compiled into the crate,
  so there are no files to ship next to your binary, and no Java, subprocess or network
  access is involved.

## Command line

html-conform does not ship a command-line tool. The repository contains a small example
that checks files and prints how many findings each one has:

```sh
git clone https://github.com/casoon/html-conform
cd html-conform
cargo run --example findings
```

It checks every `examples/showcase/*.html` file and writes the report next to it as JSON. To
check your own documents, call the library from your code, as the
[Quickstart](../quickstart/) shows.
