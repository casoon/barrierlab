---
title: "Template"
description: "The structure every package page follows."
order: 99
---

Every package gets `docs/packages/<name>.md` with this structure. Keep it short;
the API reference lives on docs.rs, not here.

```markdown
# <name>

One sentence: what the package computes and what it does not.

## Status

Version, registry, who uses it. Honest about anything half-built.

## Structure

One Mermaid diagram: data flow or modules. Do not repeat the graph from
architecture.md; show the inside of this package.

## Public surface

The few types and functions a consumer needs, with one sentence each. Link to
docs.rs for the rest.

## Dependencies

Downward: which packages this one needs. Upward: who uses it.

## Limits

What the package does not decide, does not prove, does not see. This section
is mandatory — it prevents claims the data cannot support.
```
