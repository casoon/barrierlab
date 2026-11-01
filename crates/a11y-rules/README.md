# a11y-rules

Accessibility rules, generic over the document model from
[`a11y-dom`](https://crates.io/crates/a11y-dom). Part of
[a11y-core](https://github.com/casoon/a11y-core).

One rule set, several surfaces: build time, CI/crawl and the live page — with
identical rule IDs and identical JSON.

## "Did not run" is not "passed"

`run` runs with what the host provides and records for every rule whether it
could run. A tier 2 rule on a host without `Semantics` does not produce a silent
non-finding but a note with `NotRun::CapabilityMissing`.

```rust
use a11y_dom::Arena;
use a11y_rules::run;

let doc = Arena::builder()
    .open("html").open("body")
        .open("img").attr("src", "logo.png").close()
    .close().close()
    .build();

let report = run(&doc);
assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));

// Not judged: the tier 2 and tier 3 rules, because this host provides neither
// semantics nor rendering.
assert_eq!(report.summary.rules_not_run, 7);
```

With a host that implements `Semantics`, those rules run via `run_with_semantics`;
a host with `Semantics` **and** `Rendering` uses `run_full`. The separation is not
a formality: rules that need a real accessible name — links, buttons, SVG — must
not guess without one, and contrast cannot be determined from static markup at
all.

| Host provides | Entry point |
|---|---|
| structure only | `run` |
| + semantics | `run_with_semantics` |
| + rendering | `run_with_rendering` |
| both | `run_full` |

Every entry point has a variant with `_in` and a `Locale` (`En`, `De`) for the
finding texts, e.g. `run_full_in(&doc, Locale::De)`. Without `_in` the texts are
English. Rule IDs, outcomes and severities are the same in every language. The
German texts sit behind the `de` feature (default: on); with
`default-features = false` they are not compiled into the binary at all, and
`Locale::De` does not exist.

## One set of names for findings and notes

`Meta::ids` declares **all** finding IDs a rule can produce, and `RuleRun` is kept
per ID. `rule_runs` and `findings` therefore use the same set of names and can be
joined via `rule_id`.

In 0.1.0 these were separate: notes carried a parent rule ID (`images/alt`),
findings the specific one (`images/alt-missing`). A join silently returned
nothing. Two tests now guard the guarantee — every produced ID must be declared,
and every finding must have a matching note.

## Rules

The full list of rule IDs, with tier, WCAG criterion and default severity, is on
the [rules reference page](https://casoon.github.io/barrierlab/docs/a11y/reference/regeln/);
`examples/a11y/rules.json` is its machine-readable source. At the time of writing
there are 152: 95 in tier 1 (structure), 35 in tier 2 (semantics), 17 in tier 3
(rendering) and 5 for stylesheets.

Not every missing landmark is a violation: `main` must be present, but a page may
do without navigation. `landmarks/navigation-missing`, `landmarks/banner-missing`,
`landmarks/contentinfo-missing`, `headings/h1-multiple` and
`keyboard/skip-link-missing` therefore return `REVIEW`, not `FAIL` — the skip link
can only be recognised heuristically anyway, via link text and class names.

Tier 2 includes `links/ambiguous-name` and `links/generic-name`, which are two
different rules: one says "two links have the same name but go elsewhere", the
other "this text says nothing about the target on its own". Both return `REVIEW`.

Tier 3 comes with a requirement on the host: `ComputedStyle::background_color` is
the **effective** background colour, resolved through the ancestors. If the host
cannot determine it — background image, gradient, `background-blend-mode` — it
returns `None`, and the rule reports `contrast/text-undetermined` with
`UNTESTED`. A check against a guessed white would be worse than no statement: it
produces a `PASS` that someone relies on.

## License

MIT.
