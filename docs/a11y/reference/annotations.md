---
title: "Annotation file"
description: "A file of external assessments of findings, keyed by finding id. Defined once; read by the audit tools, written by reviewers and agents."
order: 3
---

An annotation file carries **assessments of findings** that a person or an agent made
after the rules ran: whether an alt text really says nothing, whether a link purpose is
clear in context, what a reading-flow pass noticed. A host reads the file and shows the
assessments next to the findings they belong to.

The file is an input. The tools that read it stay stateless: nothing is stored between runs,
and the rules neither read nor produce annotations.

## Rules for every reader

1. **An annotation never changes a result.** Outcome, severity, scores, risk levels, exit
   codes and any fail threshold behave exactly as without the file. A reader can be tested
   by running the same audit with and without the file and comparing everything except the
   annotation section.
2. **Annotations are shown apart and labelled.** An assessment appears next to its finding,
   marked with who made it (`agent` or `human`). It is never merged into the finding's own
   text, and never counts as a finding.
3. **No new outcome.** `assessment` has three values and none of them is `fail` or `pass`.
   Turning a `review` finding into a failure is a change to the rule or to the page, not to
   the file.
4. **Nothing is dropped silently.** An annotation whose id matches no finding of this run
   (the finding may have been fixed, or the element changed) is listed as unmatched.
   Findings without annotation stay as they are.

## Format

A JSON object with `schema` (currently `1`) and `annotations`. The JSON Schema is
[`annotations.schema.json`](annotations.schema.json).

```json
{
  "schema": 1,
  "annotations": [
    {
      "finding_id": "F-1a2b3c4d",
      "pattern_id": "RC-5e6f7a8b",
      "author": { "kind": "agent", "name": "reviewer-agent 0.1" },
      "assessment": "likely_issue",
      "reasoning": "The alt text repeats the file name ('IMG_2041.jpg') and says nothing about the image.",
      "evidence": [{ "kind": "screenshot", "path": "shots/F-1a2b3c4d.png" }],
      "created": "2026-11-06"
    },
    {
      "scope": "reading_flow",
      "url": "https://example.com/",
      "author": { "kind": "human", "name": "reviewer" },
      "assessment": "undetermined",
      "reasoning": "Navigation is announced twice before the main heading; unclear whether the second is a skip-link target."
    }
  ]
}
```

| Field | Required | Meaning |
|---|---|---|
| `scope` | no | `finding` (default) or `reading_flow`. A `reading_flow` annotation belongs to a page, not to one finding. |
| `finding_id` | with scope `finding` | The `F-…` id of the finding (`FindingId` in `a11y-report`): rule plus the set of element patterns, the same on every page and run. |
| `pattern_id` | no | The `RC-…` id (`RootCauseId`) of one element pattern. Narrows the annotation to that pattern of the finding. |
| `url` | no | The page the annotation belongs to. A reader that audits several pages matches per page; without it the annotation applies to every page that has the finding. Required for scope `reading_flow` in multi-page runs. |
| `author` | yes | `kind` (`agent` or `human`) and a free `name` (tool and version, or the reviewer). |
| `assessment` | yes | `likely_issue`, `likely_fine` or `undetermined`. |
| `reasoning` | yes | Why, in a sentence or two, quoting what was looked at. Not empty. |
| `evidence` | no | A list of `{ "kind": "screenshot" \| "text" \| "link", … }`. A `screenshot` has `path` (relative to the annotation file), a `link` has `url`, a `text` has `text`. A missing file is reported by the reader, not fatal. |
| `created` | no | ISO date. |

Unknown fields are ignored by readers and kept by writers, so the format can grow without a
new `schema` number. A change that alters the meaning of an existing field raises `schema`.

## What the id guarantees

`F-…` and `RC-…` depend on the rule and the normalised selector pattern, not on node ids or
the order of occurrences. They stay the same while the rule and the element pattern stay the
same. When the element changes, the id changes, and an old annotation becomes unmatched
instead of silently attaching to something else.

## Producing annotations

The format does not say who writes the file. Typical sources are a reviewer going through the
`review` findings of a report, and an agent doing the same with its reasoning recorded.
Whoever writes it should record what was actually looked at in `reasoning` and `evidence`; a
reader cannot verify it.
