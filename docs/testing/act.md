---
title: "W3C ACT test cases"
description: "How the BarrierLab rules fare on the public test cases of the W3C ACT Rules: per rule, with every wrong result counted."
order: 1
---

The [ACT Rules](https://www.w3.org/WAI/standards-guidelines/act/) of the W3C describe
accessibility checks precisely enough that two tools can be held to the same answer. Each rule
comes with test cases: small pages that must pass, must fail, or to which the rule does not
apply. The W3C publishes, per tool, how many rules it implements consistently with those cases.
It is the most neutral yardstick there is for an automated checker: the cases are written by
the ACT Rules Community Group, not by the tool's maker.

## Result

Run of 14 November 2026 against all 1,134 test cases, with `a11y-static` (the BarrierLab rules
on the page source, no browser):

| | BarrierLab (own run) | axe-core 4.14.0 (W3C report of 31 October 2026) |
|---|---|---|
| ACT rules implemented consistently | 8 | 35 (30 WCAG rules, 5 proposed) |
| implemented partially consistently | 8 | 9 |

Of the 512 test cases of the rules BarrierLab maps, 408 come out right, 44 undecided and 60
wrong: 40 failed examples reported as passing (missed barriers) and 20 passing or
inapplicable examples reported as failing (false alarms).
Each of them is listed, with its test case, in
[barrierlab#225](https://github.com/casoon/barrierlab/issues/225).

The comparison is lopsided, and the reason is plain. axe-core runs in a browser and has had its
rules fitted to these cases for years. BarrierLab has not been fitted to them at all: this is
the first run, and nothing was changed to improve it. Rules that need a running browser
(focus, scripts, shadow DOM, media) cannot come out consistent in a run without one.

The axe-core figures are the W3C's, from its
[implementation report](https://www.w3.org/WAI/standards-guidelines/act/implementations/axe-core/).
The BarrierLab figures are this repository's own run and have not been submitted to the W3C.

## Categories

The categories are those of the W3C implementation reports:

- **consistent**: no passing or inapplicable example is reported as failing, every failing
  example is reported as failing, and not every case is undecided;
- **partial**: no false alarm, and at least one failing example is found;
- **not consistent**: a false alarm, or no failing example found.

A test case counts as failing for BarrierLab when one of the mapped rules reports `FAIL`; as
undecided when they only report `REVIEW` or `UNTESTED`, or did not run; as passing otherwise.
ACT rules that need human judgement (descriptive names, media alternatives) or a browser are
not mapped and count as untested.

## Per rule

| ACT rule | Result | BarrierLab rules | Wrong cases |
|---|---|---|---|
| [Text has enhanced contrast](https://act-rules.github.io/rules/09o5cg) | partial | `contrast/text-enhanced` | 4 of 29 |
| [Scrollable element is keyboard accessible](https://act-rules.github.io/rules/0ssw9k) | not consistent | `keyboard/scrollable-region-not-focusable` | 0 of 10 |
| [Image has non-empty accessible name](https://act-rules.github.io/rules/23a2a8) | partial | `images/alt-missing`, `names/required-missing` | 3 of 18 |
| [HTML page has non-empty title](https://act-rules.github.io/rules/2779a5) | consistent | `document/title-missing`, `document/title-empty` | 0 of 11 |
| [Visible label is part of accessible name](https://act-rules.github.io/rules/2ee8b8) | not consistent | `label-in-name/mismatch` | 2 of 13 |
| [Id attribute value is unique](https://act-rules.github.io/rules/3ea0c8) | not consistent | `ids/duplicate` | 0 of 10 |
| [Element with role attribute has required states and properties](https://act-rules.github.io/rules/4e8ab6) | not consistent | `aria/required-attribute-missing` | 2 of 8 |
| [Image button has non-empty accessible name](https://act-rules.github.io/rules/59796f) | not consistent | `images/input-alt-missing` | 2 of 12 |
| [HTML page lang and xml:lang attributes have matching values](https://act-rules.github.io/rules/5b7ae0) | consistent | `document/lang-mismatch` | 0 of 12 |
| [ARIA state or property is permitted](https://act-rules.github.io/rules/5c01ea) | not consistent | `aria/attribute-not-allowed`, `aria/attribute-prohibited` | 3 of 15 |
| [ARIA attribute is defined in WAI-ARIA](https://act-rules.github.io/rules/5f99a7) | consistent | `aria/attribute-unknown` | 0 of 7 |
| [Role attribute has valid value](https://act-rules.github.io/rules/674b10) | consistent | `aria/role-invalid` | 0 of 10 |
| [ARIA state or property has valid value](https://act-rules.github.io/rules/6a7281) | partial | `aria/attribute-value-invalid` | 2 of 24 |
| [Element with aria-hidden has no content in sequential focus navigation](https://act-rules.github.io/rules/6cfa84) | partial | `keyboard/hidden-focusable` | 0 of 14 |
| [Autocomplete attribute has valid value](https://act-rules.github.io/rules/73f2c2) | consistent | `forms/autocomplete-invalid` | 0 of 20 |
| [SVG element with explicit role has non-empty accessible name](https://act-rules.github.io/rules/7d6734) | partial | `svg/name-missing` | 0 of 10 |
| [Audio or video element avoids automatically playing audio](https://act-rules.github.io/rules/80f0bf) | not consistent | `media/audio-autoplay` | 1 of 8 |
| [Object element rendering non-text content has non-empty accessible name](https://act-rules.github.io/rules/8fc3b6) | not consistent | `objects/alt-missing` | 3 of 14 |
| [Button has non-empty accessible name](https://act-rules.github.io/rules/97a4e1) | not consistent | `buttons/name-missing` | 1 of 17 |
| [Headers attribute specified on a cell refers to cells in the same table element](https://act-rules.github.io/rules/a25f45) | not consistent | `tables/headers-attr-invalid` | 1 of 17 |
| [Text has minimum contrast](https://act-rules.github.io/rules/afw4f7) | not consistent | `contrast/text-insufficient` | 10 of 28 |
| [Orientation of the page is not restricted using CSS transform property](https://act-rules.github.io/rules/b33eff) | not consistent | `orientation/root-rotated` | 0 of 10 |
| [Meta viewport allows for zoom](https://act-rules.github.io/rules/b4f0c3) | not consistent | `zoom/viewport-locked`, `zoom/viewport-scale-limited` | 2 of 11 |
| [HTML page has lang attribute](https://act-rules.github.io/rules/b5c3f8) | consistent | `document/lang-missing` | 0 of 7 |
| [ARIA required owned elements](https://act-rules.github.io/rules/bc4a75) | partial | `aria/required-children-missing` | 4 of 19 |
| [Meta element has no refresh delay](https://act-rules.github.io/rules/bc659a) | not consistent | `timing/meta-refresh` | 1 of 15 |
| [HTML page lang attribute has valid language tag](https://act-rules.github.io/rules/bf051a) | partial | `document/lang-invalid` | 2 of 7 |
| [Meta element has no refresh delay (no exception)](https://act-rules.github.io/rules/bisz58) | not consistent | `timing/meta-refresh` | 2 of 14 |
| [Link has non-empty accessible name](https://act-rules.github.io/rules/c487ae) | consistent | `links/name-missing`, `images/area-alt-missing` | 0 of 28 |
| [Iframe element has non-empty accessible name](https://act-rules.github.io/rules/cae760) | not consistent | `frames/name-missing` | 2 of 10 |
| [Bypass Blocks of Repeated Content](https://act-rules.github.io/rules/cf77f2) | not consistent | `keyboard/skip-link-missing` | 0 of 14 |
| [Table header cell has assigned cells](https://act-rules.github.io/rules/d0f69e) | not consistent | `tables/header-without-data` | 3 of 16 |
| [Form field has non-empty accessible name](https://act-rules.github.io/rules/e086e5) | not consistent | `forms/label-missing`, `names/required-missing` | 1 of 17 |
| [ARIA required context role](https://act-rules.github.io/rules/ff89c9) | not consistent | `aria/required-parent-missing` | 4 of 15 |
| [Heading has non-empty accessible name](https://act-rules.github.io/rules/ffd0e9) | partial | `headings/empty` | 3 of 15 |
| [Menuitem has non-empty accessible name](https://act-rules.github.io/rules/m6b1q3) | consistent | `names/required-missing` | 0 of 8 |
| [Element in sequential focus order has visible focus](https://act-rules.github.io/rules/oj04fd) | not consistent | `focus/outline-removed`, `focus/indicator-missing` | 2 of 9 |

## Run it yourself

```bash
python3 xtask/act/fetch.py target/act
cargo run --release --manifest-path xtask/act/Cargo.toml -- target/act \
  --summary xtask/act/results.json --markdown target/act/table.md
```

The first command downloads the test cases into `target/act` (they are not part of this
repository). The second runs them, prints every wrong case with its URL and writes
`target/act/earl.json`, an EARL report in the form the W3C takes implementation reports in. The
mapping from ACT rules to BarrierLab rule IDs is the table `MAPPING` in
[`xtask/act/src/main.rs`](https://github.com/casoon/barrierlab/blob/main/xtask/act/src/main.rs).
