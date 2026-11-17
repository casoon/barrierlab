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

Run of 16 November 2026 against all 1,134 test cases, with `a11y-static` 0.5.1 and
`a11y-rules` 0.30.3 (the BarrierLab rules on the page source, no browser), counted the way the
W3C counts:

| | BarrierLab (own run) | axe-core 4.14.0 (W3C report of 31 October 2026) |
|---|---|---|
| ACT rules implemented consistently | 36 | 35 (30 WCAG rules, 5 proposed) |
| implemented partially consistently | 5 | 9 |

Of the 611 test cases of the rules BarrierLab maps, 558 come out right, 50 undecided and 3
wrong. All three need a script to run before the content exists (a shadow root built by
script), which a check of the page source never sees.

The first run, before any rule was changed, gave 8 consistent and 8 partial rules, with 60
wrong cases. Each of those 60 was looked at and the rule corrected against the specification
where a static decision is possible; the list is in
[barrierlab#225](https://github.com/casoon/barrierlab/issues/225). Undecided is the honest
answer where the page source does not hold the answer: background images, the length of a
media file, layout, which block is the main content.

Of the 35 rules axe-core implements consistently, BarrierLab implements 30 consistently too,
and it is consistent on 5 that axe-core is not. The 5 others are contrast, scrolling and
orientation, which need a browser to decide, and one missed context role. The axe-core figures are the W3C's, from
its [implementation report](https://www.w3.org/WAI/standards-guidelines/act/implementations/axe-core/).
The BarrierLab figures are this repository's own run and have not been submitted to the W3C.

## In a browser

The same rules run inside a browser when a host hands them the rendered page.
[auditmysite](https://github.com/casoon/auditmysite) does that in Chrome: computed styles,
layout, scroll extents, the colours text is drawn with, the playback of autoplaying media. Run
of 17 November 2026 with auditmysite on `a11y-rules` 0.30.10 and `style-cascade` 0.3.2
([casoon/auditmysite#1099](https://github.com/casoon/auditmysite/pull/1099)), all 601 mapped
test cases, `--level aaa` so that the AAA rules run too:

| | BarrierLab in auditmysite | page source only | axe-core 4.14.0 (W3C report) |
|---|---|---|---|
| ACT rules implemented consistently | 41 | 36 | 35 |
| implemented partially consistently | 2 | 5 | 9 |
| test cases right / undecided / wrong | 580 / 31 / 0 | 558 / 50 / 3 | — |

The browser decides what the page source cannot: contrast over gradients, under `opacity` and
behind a text shadow, scrollable regions, the AAA contrast cases, whether an autoplaying medium
is heard. No case comes out wrong. Two rules stay partial on purpose: duplicate ids (3ea0c8)
are a review, since WCAG 2.2 dropped 4.1.1 Parsing, and whether a page has a way to bypass
repeated blocks (cf77f2) needs a person to judge what is main content.

To repeat the run:

```bash
cargo run --manifest-path xtask/act/Cargo.toml -- target/act --urls target/act/urls.txt
auditmysite --url-file target/act/urls.txt --modules a11y --level aaa -f json --per-page-reports -o target/act/auditmysite
cargo run --manifest-path xtask/act/Cargo.toml -- target/act --auditmysite target/act/auditmysite --summary xtask/act/results-auditmysite.json
```

A finding auditmysite files as a review hint is undecided, never passed; a rule auditmysite has
not taken up yet is undecided too.

## Categories

The categories follow `getConsistency` in
[act-rules/act-tools](https://github.com/act-rules/act-tools), the code behind the W3C
implementation reports:

- **consistent**: no passing or inapplicable example is reported as failing, no failing example
  is reported as passing or inapplicable, every case has a result, and at least one failing
  example is found. Undecided is allowed on some examples;
- **partial**: no false alarm, and at least one failing example is found — or some case is
  undecided and every inapplicable example is reported as passing or inapplicable;
- **not consistent**: anything else.

The W3C report also checks that the success criteria a tool names match the rule's; the run
here does not repeat that check.

A test case counts as failing for BarrierLab when one of the mapped rules reports `FAIL`; as
undecided when they only report `REVIEW` or `UNTESTED`, or did not run; as passing otherwise.
A test case that is not an HTML page (SVG, XHTML, XML) is inapplicable for rules that apply
to HTML pages only, and otherwise goes through the same check as an HTML page. ACT rules that
need human judgement (descriptive names, media alternatives) or a browser are not mapped and
count as untested.

## Per rule

| ACT rule | Result | BarrierLab rules | Wrong cases |
|---|---|---|---|
| [Text has enhanced contrast](https://act-rules.github.io/rules/09o5cg) | partial | `contrast/text-enhanced`, `contrast/text-undetermined` | 1 of 29 |
| [Scrollable element is keyboard accessible](https://act-rules.github.io/rules/0ssw9k) | not consistent | `keyboard/scrollable-region-not-focusable` | 0 of 10 |
| [Image has non-empty accessible name](https://act-rules.github.io/rules/23a2a8) | consistent | `images/alt-missing`, `names/required-missing` | 0 of 18 |
| [Letter spacing in style attributes is not !important](https://act-rules.github.io/rules/24afc2) | consistent | `text-spacing/letter-important` | 0 of 16 |
| [HTML page has non-empty title](https://act-rules.github.io/rules/2779a5) | consistent | `document/title-missing`, `document/title-empty` | 0 of 11 |
| [Visible label is part of accessible name](https://act-rules.github.io/rules/2ee8b8) | consistent | `label-in-name/mismatch` | 0 of 13 |
| [Element with presentational children has no focusable content](https://act-rules.github.io/rules/307n5z) | consistent | `aria/presentational-children-focusable` | 0 of 7 |
| [Id attribute value is unique](https://act-rules.github.io/rules/3ea0c8) | partial | `ids/duplicate` | 0 of 10 |
| [Element marked as decorative is not exposed](https://act-rules.github.io/rules/46ca7f) | consistent | `aria/presentation-conflict` | 0 of 10 |
| [Element with role attribute has required states and properties](https://act-rules.github.io/rules/4e8ab6) | consistent | `aria/required-attribute-missing`, `aria/combobox-popup-missing` | 0 of 8 |
| [Image button has non-empty accessible name](https://act-rules.github.io/rules/59796f) | consistent | `images/input-alt-missing` | 0 of 12 |
| [HTML page lang and xml:lang attributes have matching values](https://act-rules.github.io/rules/5b7ae0) | consistent | `document/lang-mismatch` | 0 of 12 |
| [ARIA state or property is permitted](https://act-rules.github.io/rules/5c01ea) | consistent | `aria/attribute-not-allowed`, `aria/attribute-prohibited` | 0 of 15 |
| [ARIA attribute is defined in WAI-ARIA](https://act-rules.github.io/rules/5f99a7) | consistent | `aria/attribute-unknown` | 0 of 7 |
| [Role attribute has valid value](https://act-rules.github.io/rules/674b10) | consistent | `aria/role-invalid` | 0 of 10 |
| [ARIA state or property has valid value](https://act-rules.github.io/rules/6a7281) | consistent | `aria/attribute-value-invalid` | 0 of 24 |
| [Element with aria-hidden has no content in sequential focus navigation](https://act-rules.github.io/rules/6cfa84) | consistent | `keyboard/hidden-focusable` | 0 of 14 |
| [Autocomplete attribute has valid value](https://act-rules.github.io/rules/73f2c2) | consistent | `forms/autocomplete-invalid` | 0 of 20 |
| [Line height in style attributes is not !important](https://act-rules.github.io/rules/78fd32) | consistent | `text-spacing/line-height-important` | 0 of 21 |
| [SVG element with explicit role has non-empty accessible name](https://act-rules.github.io/rules/7d6734) | consistent | `svg/name-missing` | 0 of 10 |
| [Audio or video element avoids automatically playing audio](https://act-rules.github.io/rules/80f0bf) | not consistent | `media/audio-autoplay`, `media/audio-autoplay-measured` | 0 of 8 |
| [Object element rendering non-text content has non-empty accessible name](https://act-rules.github.io/rules/8fc3b6) | consistent | `objects/alt-missing` | 0 of 14 |
| [Button has non-empty accessible name](https://act-rules.github.io/rules/97a4e1) | consistent | `buttons/name-missing` | 0 of 17 |
| [Word spacing in style attributes is not !important](https://act-rules.github.io/rules/9e45ec) | consistent | `text-spacing/word-important` | 0 of 16 |
| [Headers attribute specified on a cell refers to cells in the same table element](https://act-rules.github.io/rules/a25f45) | consistent | `tables/headers-attr-invalid` | 0 of 17 |
| [Text has minimum contrast](https://act-rules.github.io/rules/afw4f7) | partial | `contrast/text-insufficient`, `contrast/text-undetermined` | 1 of 28 |
| [Orientation of the page is not restricted using CSS transform property](https://act-rules.github.io/rules/b33eff) | consistent | `orientation/locked` | 0 of 10 |
| [Meta viewport allows for zoom](https://act-rules.github.io/rules/b4f0c3) | consistent | `zoom/viewport-locked` | 0 of 11 |
| [HTML page has lang attribute](https://act-rules.github.io/rules/b5c3f8) | consistent | `document/lang-missing` | 0 of 7 |
| [ARIA required owned elements](https://act-rules.github.io/rules/bc4a75) | consistent | `aria/required-children-missing` | 0 of 19 |
| [Meta element has no refresh delay](https://act-rules.github.io/rules/bc659a) | consistent | `timing/meta-refresh` | 0 of 15 |
| [HTML page lang attribute has valid language tag](https://act-rules.github.io/rules/bf051a) | consistent | `document/lang-invalid` | 0 of 7 |
| [Meta element has no refresh delay (no exception)](https://act-rules.github.io/rules/bisz58) | consistent | `timing/meta-refresh-enhanced` | 0 of 14 |
| [Link has non-empty accessible name](https://act-rules.github.io/rules/c487ae) | consistent | `links/name-missing`, `images/area-alt-missing` | 0 of 28 |
| [Iframe element has non-empty accessible name](https://act-rules.github.io/rules/cae760) | consistent | `frames/name-missing` | 0 of 10 |
| [Bypass Blocks of Repeated Content](https://act-rules.github.io/rules/cf77f2) | partial | `headings/none` | 0 of 14 |
| [Table header cell has assigned cells](https://act-rules.github.io/rules/d0f69e) | consistent | `tables/header-without-data` | 0 of 16 |
| [Element with lang attribute has valid language tag](https://act-rules.github.io/rules/de46e4) | consistent | `language/part-invalid` | 0 of 19 |
| [Form field has non-empty accessible name](https://act-rules.github.io/rules/e086e5) | consistent | `forms/label-missing`, `names/required-missing` | 0 of 17 |
| [ARIA required context role](https://act-rules.github.io/rules/ff89c9) | partial | `aria/required-parent-missing` | 1 of 15 |
| [Heading has non-empty accessible name](https://act-rules.github.io/rules/ffd0e9) | consistent | `headings/empty` | 0 of 15 |
| [Menuitem has non-empty accessible name](https://act-rules.github.io/rules/m6b1q3) | consistent | `names/required-missing` | 0 of 8 |
| [Element in sequential focus order has visible focus](https://act-rules.github.io/rules/oj04fd) | consistent | `focus/outline-removed`, `focus/indicator-missing` | 0 of 9 |

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
