---
title: "Testing a website"
description: "How to test a page with the tools built on BarrierLab: from the terminal, after a build, in the running page and in a pipeline. What they do well, and where they fall short."
order: 0
---

BarrierLab itself checks nothing. It is the rule set and the report model; a tool supplies the
page. To test a website you pick the tool that fits the place where you want to look at it.
All of them report the same rule IDs in the same JSON, so their results can be laid side by
side.

## The first test, from the terminal

Two command-line tools read a page from its address and print a report.

**auditmysite** drives a real Chrome, so it sees the page as a visitor does: after scripts,
with computed styles and the browser's own accessibility tree. It needs a Chrome or Chromium
on the machine.

```bash
curl -fsSL https://raw.githubusercontent.com/casoon/auditmysite/main/install.sh | bash
auditmysite https://example.com
```

**barrierlab-cli** needs no browser. It fetches the HTML and the stylesheets, resolves the
cascade itself and runs the rules on that. It is quicker to set up on a server and sees less.
There is no release yet; it is built from the repository.

```bash
cargo install --git https://github.com/casoon/barrierlab-cli
barrierlab https://example.com
```

Both end with a summary and, below it, three lists: what failed, what needs a person to
decide, and what was not tested. The third list is part of the result, not a footnote.

## Which tool for which place

| Where you look | Tool | What it sees |
|---|---|---|
| A live page, from the terminal or CI | [auditmysite](https://github.com/casoon/auditmysite) | the rendered page in Chrome: DOM after scripts, styles, accessibility tree |
| A live page or a file, without a browser | [barrierlab-cli](https://github.com/casoon/barrierlab-cli) | the markup the server sends, plus its stylesheets |
| An Astro site after `astro build` | [astro-post-audit](https://github.com/casoon/astro-post-audit) | every page in `dist/`, offline, together with links and SEO |
| The running page, on your own site | [liveaudit](https://github.com/casoon/liveaudit) | the page in its current state, findings drawn on the element |
| Your own checker | the crates and `@casoon/a11y-wasm` | whatever your host hands over |

A sensible order: astro-post-audit or barrierlab-cli on every build, because they are fast and
need nothing else; auditmysite for the pages that depend on scripts and for a report you hand
to someone; liveaudit while you fix things in the page.

## What is different

**One finding, everywhere the same.** `contrast/text-insufficient` means the same check with
the same threshold in a build log, a terminal report and a marker in the browser. When two
tools disagree about a page, the difference is in what each could see, not in two copies of a
rule that drifted apart.

**"Not run" is not "passed".** A rule that lacks its input, such as contrast without computed
styles or focus visibility without a moving focus, appears in the report as not run, with the
reason. A clean report from a tool that could only see the source says so in its summary
instead of looking like a full pass.

**What a machine cannot decide becomes a checklist.** Whether an alternative text describes
the image, whether errors are explained, whether a time limit can be extended: these appear
as items to check by hand, once per page where they apply.

**Built for pipelines.** Exit codes carry the verdict, the JSON follows one schema, and every
finding carries its rule ID, outcome, severity and WCAG criteria. auditmysite and
barrierlab-cli are single compiled binaries; neither needs a Node toolchain on the machine
that runs it.

**HTML conformance in the same run.** [html-conform](../html-conform/index.md) checks markup
against the HTML standard in-process, measured against the Nu Html Checker's own test files,
so a validity check does not need a Java service next to the audit.

## In a pipeline

barrierlab-cli ends with exit code 1 when a rule failed, 0 when none did and 2 when it could
not read the page. That is enough for a CI step:

```bash
barrierlab https://staging.example.com --json > report.json
```

`--fail-on review` makes findings that need review fail the step too. auditmysite writes
JSON or a PDF report (`-f json -o report.json`); astro-post-audit fails the build itself.
Because the rule IDs are stable, a pipeline can compare today's report with yesterday's and
show only what changed.

## Next to other engines

The most widely used engine for this work is axe-core, inside Lighthouse, browser extensions
and many test setups. BarrierLab does the same job, so the two are compared whether anyone
intends it or not. On the test bench of [barrierlab.eu](https://barrierlab.eu/tools/coverage/),
a set of practice pages with 22 known barriers, barrierlab-cli reported 8 of them as failures,
as axe-core did, and neither flagged anything on the corrected versions of those pages. No
tool on that bench found more than 10.

That is the more useful figure: an automated check, whichever engine runs it, finds about a
third of what is wrong with a page. Running two independently written engines on the same page
catches mistakes in either one; barrierlab.eu runs axe-core beside auditmysite for that reason.

## Where it falls short

- **Young and small.** The rule set is checked against real pages and its own corpus, but it
  has seen far fewer sites than engines that have been in use for years. Expect false positives
  that others fixed long ago, and report them.
- **barrierlab-cli is not released.** It is built from source with a Rust toolchain; there is
  no binary to download yet.
- **Without a browser, the page is the source.** barrierlab-cli and astro-post-audit do not see
  what scripts build, where things sit on screen, focus, target sizes or reflow. Those rules
  are reported as not run. For a single-page application the result says little.
- **Its own cascade is not a browser.** barrierlab-cli computes styles itself. Selectors such as
  `:has()` or container queries leave values undetermined, and the computation has not been
  compared with a browser on a large corpus.
- **No browser extension.** liveaudit runs inside your own page, behind a script tag; it is not
  an extension you point at someone else's site.
- **Documentation is partly German.** The accessibility section of this site still is.
- **In maintenance.** The libraries take fixes, not new features, until the tools above have
  taken up what is there. One maintainer, a small community.

Every point on this list is an open question, and the good ones become
[issues](https://github.com/casoon/barrierlab/issues). A false positive or a missed barrier on
a real page is the most useful report there is: it comes with a case, and a rule only changes
with a case.
