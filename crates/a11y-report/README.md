# a11y-report

Shared finding and report model for accessibility tools. Part of
[a11y-core](https://github.com/casoon/a11y-core).

## Two axes, not three

`Outcome` says *how certain* the statement is — `Fail`, `Review`, `Pass`,
`Untested`. `Severity` says *how serious* the problem is.

There is deliberately no third "certainty" axis: it would largely be the same
axis twice, because `Fail` already means established automatically, `Review`
means heuristic, and `Untested` means judgeable only by hand. A rule that can
only suspect its finding therefore returns `Review` — not `Fail` with low
certainty.

No aggregated score. A single percentage would flatten exactly the distinction
the four states exist for.

```rust
use a11y_report::{Finding, Location, Report, Severity};

let mut report = Report::new();
report.push(
    Finding::fail("images/alt-missing", "The image has no alt attribute.")
        .with_severity(Severity::High)
        .with_wcag(["1.1.1"])
        .at(Location::file("about/index.html").with_selector("main > img")),
);
let report = report.finish();
assert_eq!(report.summary.fail, 1);
```

## Room for what does not belong in the contract

The shared contract cannot hold every field of every tool — auditmysite, for
instance, attaches an element screenshot to a finding, which it crops for its PDF
report. A side table keyed by index does not hold up: findings are filtered,
sorted and merged from several runs, and indices shift.

`Extra` is therefore an opaque slot on the finding itself. It travels along, is
**never serialised** and does **not count towards identity** — two findings that
are equal in content stay equal even if one carries a screenshot.

```rust
use a11y_report::Finding;

struct Screenshot(Vec<u8>);

let f = Finding::fail("images/alt-missing", "no alt")
    .with_extra(Screenshot(vec![0x89, 0x50]));

assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(2));
```

The role and accessible name of the affected element are **not** an extra but
part of the contract: whoever reads a report wants to know what the element was
to assistive technology — a selector alone does not say that.

## "Did not run" is not "passed"

`RuleRun` records whether a rule could run at all. Without it, a contrast check
without access to rendering reads like a passed check — the most common way a
report promises more than it checked.

## License

MIT.
