---
title: "Findings and Reports"
description: "The finding model from a11y-report: two axes, four states, one record per rule ID and a lean JSON contract."
order: 2
---

## Two axes, not three

`Outcome` says *how certain* the statement is, `Severity` says *how severe* the problem is.

| `Outcome` | Meaning |
|---|---|
| `fail` | automatically detected problem, unambiguously evidenced |
| `review` | potential problem, only heuristically suspected, needs human confirmation |
| `pass` | automated check passed, recorded but not shown by default |
| `untested` | cannot be judged automatically, either in principle or because the host lacks the data |

`Severity` ranges from `low` through `medium` and `high` to `critical`. For tools that expect the
axe vocabulary, `Severity::as_axe_impact` returns `minor`, `moderate`, `serious` and
`critical`.

There is deliberately no third "certainty" axis. It would largely duplicate the same axis,
because `fail` already means automatically detected, `review` heuristic and `untested` only
manually assessable. A rule that can only suspect its statement therefore returns `review`,
not `fail` with low certainty.

## No score

`Summary` counts states and severities, nothing more. Severities are counted only across problems,
that is `fail` and `review`; `Summary::problems` is their sum. `untested` is not counted, because it
makes no statement. An aggregated percentage would flatten exactly the distinction the four
states exist for.

## One record per ID

`Report` has three fields: `findings`, `rule_runs` and `summary`. `RuleRun` answers "did it run
at all?", `Finding` answers "what came out of it?". The records are kept per
**finding** ID, not per parent rule. This way `rule_runs` and
`findings` use the same set of names and can be joined via `rule_id`.

Anyone checking the same page under several conditions keeps one record per run with
`viewport`; the key is then `(rule_id, viewport)`. A record for a rule that did not run can name
via `wcag` which criterion remained unchecked.

## The JSON contract

A finding carries four required fields: `rule_id`, `outcome`, `severity` and `message`. Everything
else is omitted when not set:

```json
{
  "rule_id": "images/alt-missing",
  "outcome": "fail",
  "severity": "high",
  "message": "The image has no alt attribute.",
  "location": {
    "node": "15"
  },
  "wcag": [
    "1.1.1"
  ]
}
```

The finding comes from `examples/teaser.semantik.json`. The optional fields:

| Field | Content |
|---|---|
| `rule_name` | human-readable name of the rule |
| `location` | `file`, `url`, `selector`, `node`, `source_hint` |
| `role`, `name` | computed role and accessible name of the element |
| `wcag`, `wcag_level` | success criteria and level `A`, `AA` or `AAA` |
| `tags` | free-form keywords |
| `help`, `help_url`, `suggestion`, `suggested_code` | what to do |
| `snippet` | the affected markup, as far as the host can provide it |
| `evidence` | evidence with `source`, `field`, `value` |

## Language

`message`, the reason of a record for a rule that did not run, and the declaration's hint are available in
English and German. The host chooses the language at run time (`run_with_semantics_in(doc,
Locale::De)`); the entry points without `_in` return English. Everything by which a tool
recognizes a finding — ID, outcome, severity, WCAG reference — is the same in every language. The
contract itself does not change: `message` remains a text, only in the chosen language.

The German texts hang on the `de` feature, which is on by default. Anyone shipping only English
and having to watch size — like `a11y-wasm` in the running page — builds with
`default-features = false` and saves the German templates entirely.

## Room for what does not belong in the contract

`Finding::extra` is an opaque slot for tool-specific extras, such as the
element screenshot that auditmysite crops for its PDF report. It travels with the finding,
is **never serialized** and does **not count toward identity**: two findings that are equal in content
remain equal even if one has a screenshot attached.

```rust
use a11y_report::Finding;

struct Screenshot(Vec<u8>);

let f = Finding::fail("images/alt-missing", "no alt")
    .with_extra(Screenshot(vec![0x89, 0x50]));

assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(2));
```
