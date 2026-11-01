---
title: "Installation"
description: "Which crates a tool needs and which Rust version they build with."
order: 1
---

The crates are on crates.io and require Rust 1.85 or newer. Which of them a project
includes depends on what it wants to do.

## Running rules

```sh
cargo add a11y-rules@0.26.1 a11y-dom@0.26.0 a11y-report@0.26.0
```

`a11y-rules` provides the rules and the entry functions `run`, `run_with_semantics`,
`run_with_rendering` and `run_full`. You need `a11y-dom` for the document model and `a11y-report`
for the report the rules return.

## Computing the accessible name

```sh
cargo add accname@0.26.1 a11y-dom@0.26.0
```

`accname` is generic over the document model from `a11y-dom` and can be used without the rules.
A host that does not provide the role and name itself plugs it into its
`Semantics` implementation, see [Quick start](../schnellstart/).

## Just the report model

```sh
cargo add a11y-report@0.26.1
```

For tools that have their own checks but want to speak the same JSON contract.
`a11y-report` depends only on `serde`.

## Versions

The four crates are versioned in lockstep. A breaking change in any one of them raises all four
to the same minor version. New rule IDs also count as a minor step, because they
make findings appear that a consumer has not expected so far.
