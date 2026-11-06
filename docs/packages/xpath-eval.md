---
title: "xpath-eval"
description: "XPath 1.0 parser and evaluator, generic over the caller's document model."
order: 13
---

Parses and evaluates XPath 1.0 expressions against a document the caller
provides through the `Document`/`Node` traits. It builds no tree and parses no
XML or HTML itself. Used by `schematron-engine`.

## Conformance

Measured against the Jaxen XPath test cases (281 case runs: 263 pass, the
18 failures are functions outside XPath 1.0 and the unbound-prefix policy) and
locked by own tests in `tests/xpath10_regression.rs`. Categories, known
failures and how to rerun the externally fetched corpus:
[`CONFORMANCE.md`](../../crates/xpath-eval/CONFORMANCE.md).
