---
title: "Display Modes"
description: "The convention for visualizations and display modes (draft v0): three layers per graphic, one page-wide mode, and the rules that check them."
order: 4
---

A page with charts, maps and 3D scenes should exclude no one, without giving up the
graphics. The convention solves this with **one setting for the whole page** —
as natural as light and dark — and with **three layers per visualization**, from
which the mode selects. It comes from the reference implementation
[Geographia](https://geographia.eu) and has applied BarrierLab-wide since 28.09.2026. The status here
is **draft v0**: names and required fields may still change.

The convention is not a WCAG requirement. It is a way to reliably satisfy several criteria at
once — 1.1.1 (text alternative), 1.3.1 (values as a table), 2.2.2 (turning off motion) —
and makes that way machine-checkable.

## Three modes

| Mode | shows | intended for |
|---|---|---|
| `visual` | everything, including 3D, animation, interaction | the default |
| `calm` | still images instead of motion | motion-sensitive users, no WebGL, `Save-Data` |
| `text` | no graphics; key message, values, source | screen reader users who read the page linearly; read-aloud; print |

The mode is set on the document as `<html data-display="visual|calm|text">`. A small,
blocking script in `<head>` sets it **before first paint**: from `localStorage`
(key `display`), falling back to `calm` under `prefers-reduced-motion`. A toggle, marked
with `[data-display-toggle]`, changes it. There is **one URL per page** — no separate
text portal that has to be maintained twice and goes stale.

## Three layers per visualization

Every visualization is a `<figure data-viz="…">`. The value says what it is: `chart`,
`diagram`, `image`, `3d` or `interactive`.

| Layer | Content | Required |
|---|---|---|
| `[data-viz-text]` | the key message in one sentence, the values (for `chart` as a `<table>`), source | always, and always in the HTML |
| `[data-viz-static]` | still image of the same message: SVG or image | for `3d` and `interactive` |
| `[data-viz-live]` | 3D, animation, interaction; loaded only in `visual` | no |
| `<figcaption>` | names the visualization | always |

The text layer is **never** hidden via `hidden`, `aria-hidden` or `inert`. Outside text
mode it is hidden only visually (a `visually-hidden` class); screen readers then read
it in the page flow, with its structure. A reference via `aria-describedby` is not necessary
and would otherwise read the message out twice.

```html
<html lang="en" data-display="visual">
<head>
  <script src="/display-init.js"></script>  <!-- classic, without defer -->
</head>
<body>
  <header>
    <div data-display-toggle>
      <button type="button" aria-pressed="true">Visual</button>
      <button type="button" aria-pressed="false">Calm</button>
      <button type="button" aria-pressed="false">Text</button>
    </div>
  </header>
  <main>
    <figure data-viz="interactive">
      <div data-viz-live><!-- three.js, only in "visual" --></div>
      <svg data-viz-static role="img" aria-label="Warming by region" viewBox="0 0 640 360">…</svg>
      <div class="viz-text" data-viz-text>
        <p>The key message in one sentence.</p>
        <table>…</table>
        <p>Source: …</p>
      </div>
      <figcaption>Warming by region</figcaption>
    </figure>
  </main>
</body>
</html>
```

A sketch of the matching styles:

```css
html:not([data-display="text"]) [data-viz-text] { /* visually hidden */ }
html[data-display="text"] [data-viz-live],
html[data-display="text"] [data-viz-static] { display: none; }
html[data-display="calm"] [data-viz-live] { display: none; }
```

## Why this way

- **Page-wide instead of per graphic.** The common approach is a "show table" toggle on every chart.
  Anyone who cannot or does not want to see graphics then has to find and operate it everywhere. A mode
  for the whole page makes the decision once.
- **In the content, not in an overlay.** The author writes the layers; no tool loaded later
  guesses them. What is missing is missing in the HTML — and is therefore checkable.
- **Text is designed, not stripped down.** Text mode is a reading view of its own and a good one, with
  key message and tables, not a leftover page.
- **Before first paint.** If a late script sets the mode, the animation starts
  before it disappears — exactly what `calm` is meant to prevent.

## Checks

`a11y-rules` checks the static half; each rule reports only on pages that use the convention,
carries the `best-practice` tag and attaches to the nearest WCAG criterion. What only
the running page shows — computed visibility, the timing of `data-display`, whether graphics are
still visible in text mode — is measured by [auditmysite](https://github.com/casoon/auditmysite)
(`display/text-not-visible`, `display/text-media-visible` and the rendered parts of the others).

| ID | checks | Verdict, severity | WCAG |
|---|---|---|---|
| `viz/text-missing` | `figure[data-viz]` without a non-empty `[data-viz-text]` | `FAIL`, high | 1.1.1 |
| `display/text-hidden` | `[data-viz-text]` with `hidden`, `aria-hidden="true"` or `inert` (on it or an ancestor) | `FAIL`, high; `REVIEW`, low, if an element of the graphic references it via `aria-describedby`/`aria-details` | 1.1.1 |
| `viz/caption-missing` | `figure[data-viz]` without a `<figcaption>` child | `FAIL`, low | 1.1.1 |
| `viz/static-missing` | `3d` or `interactive` without `[data-viz-static]` | `FAIL`, medium | 2.2.2 |
| `viz/table-missing` | `chart` without `<table>` | `REVIEW`, low | 1.3.1 |
| `display/toggle-missing` | visualizations, but no `[data-display-toggle]` | `FAIL`, medium | 2.2.2 |
| `display/init-missing` | neither `html[data-display]` in the markup nor a blocking script in `<head>` | `REVIEW`, low | 2.2.2 |

`display/text-hidden` was called `viz/text-hidden` in the first draft; the ID follows auditmysite
so that static and measured findings coincide.

### Limits

- **The key message is not evaluated.** `viz/text-missing` checks that text is present, not whether
  it matches the message of the graphic. No machine can decide that.
- **Media outside the convention** (`canvas`, `video`, `svg[role=img]` without `figure[data-viz]`)
  are checked by no rule. The draft provided for `viz/orphan-media`; so far there is no real
  case for it.
- **Whether a script sets the mode** is shown only by the running page. Statically, all that is certain is that
  without a blocking script in `<head>` nobody can set it in time.
- **Only `figure` carries the checks.** A `data-viz` on another element is not checked for
  text, caption and still image.
