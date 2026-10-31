---
title: "a11y-rules: Port from auditmysite"
description: "Evidence, deviations and rules not ported for each step of the rule migration from auditmysite."
order: 91
---

Starting with 0.14.0, the rules moved from auditmysite to `a11y-rules` step by
step (casoon/barrierlab#13, auditmysite#690). The
[changelog](https://github.com/casoon/barrierlab/blob/main/crates/a11y-rules/CHANGELOG.md)
lists the new identifiers for each version; this page records, for each step,
what they replace, what they rest on and where they deliberately deviate from
auditmysite.

Each section reflects the state of the release it names. Later changes to a
verdict or WCAG mapping, above all the
[normative mapping](a11y-rules.md#normative-mapping) in 0.26.0, are in the
changelog. Unless noted otherwise, `a11y-rules` sets no per-page cap;
auditmysite reported at most 5, 10 or 20 findings per rule.

**The 48 real pages** that the evidence from 0.18.0 onwards refers to are
auditmysite's reference pages plus 40 widely used German and international
sites, fetched on 2026-10-03.

## 0.14.0: ARIA (B1)

The ARIA rules from auditmysite (casoon/barrierlab#14, B1). The norm is WAI-ARIA
1.2 and ARIA in HTML; the role and attribute tables are generated from the
specification, as bitmasks per role.

### New identifiers

| New identifier | Tier | replaces in auditmysite |
|---|---|---|
| `aria/attribute-unknown` | 1 | `aria-attr-name-invalid` (`aria_roles`) |
| `aria/attribute-value-invalid` | 1 | `aria-valid-attr-value` (value ranges) |
| `aria/owns-conflict` | 1 | `duplicate-id-aria` (`parsing`) |
| `aria/tab-selected-missing` | 1 | `aria-tab-selected-state`, `tab-no-aria-selected` (`widget_rules`) |
| `aria/tabpanel-missing` | 1 | `aria-tablist-tabpanel` (`widget_rules`) |
| `aria/combobox-popup-missing` | 1 | `aria-combobox-options` (`widget_rules`) |
| `popover/target-missing`, `popover/target-invalid` | 1 | `modern-attribute-misuse` (`modern_attributes`) |
| `inert/dialog-inert` | 1 | `modern-attribute-misuse` (`active_surface_inert`) |
| `aria/attribute-not-allowed` | 2 | `aria-allowed-attr` |
| `aria/attribute-prohibited` | 2 | `aria-prohibited-attr` |
| `aria/required-parent-missing` | 2 | `aria-required-parent` |
| `aria/required-children-missing` | 2 | `aria-roles` (required owned elements) |

The IDREF check of `aria-valid-attr-value` is taken over by
`aria/reference-missing`. `aria/role-invalid` and `aria/role-abstract` already
existed; they replace the role check of `aria-roles`.

Context and owned elements are checked against the DOM, not the accessibility
tree: an `<li>` in `<ul role="tablist">` is a `listitem` according to ARIA in
HTML, even if Chrome flattens it to `generic`. So, like axe, the case from
auditmysite#715 (`ul[role=tablist] > li > a[role=tab]`) reports
`aria/required-children-missing` on the tablist and
`aria/required-parent-missing` on the tabs and items. `generic`,
`none`/`presentation` and anything hidden by the host are transparent
(`<tbody>` in Chrome, auditmysite#659); `aria-owns` counts as ownership.

### Deviations

- `aria/attribute-not-allowed` and `aria/attribute-prohibited` judge by the
  role of the host, explicit or implicit — auditmysite only by explicit `role`
  values (for `prohibited` additionally `div`/`span`). `<div aria-expanded>` is
  therefore a finding. Plus the "MUST NOT" cases from ARIA in HTML
  (`aria-checked` on a native checkbox/radio, `aria-valuemin`/`-max` next to
  `min`/`max`, `aria-placeholder` next to `placeholder`,
  `aria-disabled`/`-readonly`/`-required="false"` next to the native attribute,
  diverging `aria-colspan`/`-rowspan`). Empty values count as missing. Roles
  outside ARIA 1.2 (browser-internal ones, `mark`) get no verdict.
- `aria/attribute-value-invalid` does not report `aria-current` and
  `aria-invalid`: ARIA 1.2 specifies that an unknown value there counts as
  `true`. Comparison is case-insensitive; empty values produce no finding.
- `aria/required-children-missing` only checks containers with an explicit
  role — native tables and lists prescribe their parts through HTML
  (auditmysite#659, #674). The empty native table from the corpus
  `table_required_rows_tbody` (only a `<caption>`) is therefore no longer a
  finding. No finding without any owned node, under `aria-busy="true"` and with
  `aria-expanded="false"`.
- `aria/required-parent-missing` allows, beyond ARIA 1.2, `group` for
  `listitem` (ARIA 1.1), `combobox` for `option` and `radiogroup` for
  `menuitemradio`. If the walk upwards hits a role that ARIA 1.2 does not know,
  no finding is produced.
- `aria/tab-selected-missing` is `REVIEW` (ARIA says SHOULD) and sits on the
  tablist when *no* tab carries `aria-selected="true"` — not on every tab
  without the attribute; the others have the right state through the default
  `false`.
- `aria/tabpanel-missing` is `REVIEW`: ARIA describes the panel only as
  customary.
- `aria/combobox-popup-missing` relies on the MUST for `aria-controls`
  (ARIA 1.2); a popup in the subtree or via `aria-owns` (ARIA 1.1) still
  suffices.
- `inert/dialog-inert` is `FAIL` only for `<dialog open>`; `role="dialog"`
  becomes `REVIEW`, visible `role="menu"` is not checked at all (a menu pushed
  off-screen with `inert` is built correctly).

### Not ported

From `modern_attributes`: the missing name of open dialogs and popovers (belongs
to the name check for dialogs) and "focus inside an inert subtree" (needs the
active element, which no tier supplies).

## 0.15.0: Names (B2)

The naming rules from auditmysite (casoon/barrierlab#15, B2). The norm is
WAI-ARIA 1.2 ("Accessible Name Required"), accname 1.2, HTML-AAM and WCAG
2.5.3; all in Tier 2.

### New identifiers

| New identifier | Severity | replaces in auditmysite |
|---|---|---|
| `names/required-missing` | high | `aria-command-name` (without `button` and `a[href]`), `aria-input-field-name`, `aria-meter-name`, `aria-progressbar-name`, `aria-toggle-field-name`, `aria-treeitem-name` (`aria_naming_rules`); `aria-label` "no name" (`accessible_name`) |
| `names/symbol-only` | medium | `aria-label` "Icon Only" (`accessible_name`) |
| `dialog/name-missing` | high | `aria-dialog-name` (`aria_naming_rules`), `dialog-name` (`dialog_rules`) |
| `dialog/modal-unmarked` | medium | `dialog-name` "Dialog Modal" (`dialog_rules`) |
| `summary/name-missing` | high | `summary-name` (`summary_name`) |
| `status/live-overridden` | high (`alert`), medium (`status`, `log`) | `aria-live-region-role` (`status_messages`) |
| `label-in-name/mismatch` | medium | `label-content-name-mismatch` (`label_in_name`) |

Unnamed buttons, links (`a[href]`) and SVGs still report
`buttons/name-missing`, `links/name-missing` and `svg/name-missing`;
`names/required-missing` does not report them a second time. A native form
field with no label at all is reported by `forms/label-missing`;
`names/required-missing` only when a label is claimed but comes up empty
(`<label for>` without text).

### Deviations

- `names/required-missing` does not check `menu` and `tab` (ARIA 1.2 requires
  no name there) and no native `<option>` (the empty placeholder option of a
  `<select>` is common; Chrome does not expose it as an `option`). Unnamed,
  non-focusable elements are a violation as in `aria_naming_rules`, not the
  warning from `accessible_name`.
- `dialog/name-missing` reports one case for which auditmysite had two
  identifiers. A closed `<dialog>` (without `open`) is not checked.
- `dialog/modal-unmarked` is `REVIEW` rather than a violation (ARIA does not
  require `aria-modal`; a non-modal dialog rightly does not carry it) and only
  checks `role="dialog"`: whether a native `<dialog>` was opened with
  `showModal()` is not supplied by any tier.
- `summary/name-missing` reports only the `<summary>` that serves its
  `<details>`, not the `<details>` in addition.
- `status/live-overridden` is a violation only with `aria-live="off"` (the
  region is then not announced). A different urgency (`alert` with `polite`,
  `status` with `assertive`) is `REVIEW`: ARIA allows overriding, and the
  announcement still happens. Implicit roles count (`<output>` is `status`).
- `names/symbol-only` is `REVIEW` rather than a violation — 4.1.2 requires a
  name and says nothing about its quality. Letters count by Unicode.
- `label-in-name/mismatch` checks all roles with a name from content
  (`button`, `link`, `menuitem*`, `tab`, `checkbox`, `radio`, `switch`,
  `option`, `treeitem`) whose name comes from `aria-label` or
  `aria-labelledby` — auditmysite only `button` with `aria-label`. The computed
  name is compared, ignoring case, punctuation and whitespace, with letters by
  Unicode. As in auditmysite: no finding if the name appears in the visible
  text; `REVIEW` if the visible text is more than twice as long as the name
  (#513). Visually hidden text (`.sr-only`) cannot be recognized without styles
  and counts as visible.

### Not ported

From `accessible_name`: empty `aria-labelledby` and `aria-describedby` on an
element that has a name — the other name sources take effect, it does no harm
(as already with `aria/reference-missing`). Also not yet ported:
`description-duplicates-name` (auditmysite#713). That would require
`a11y_dom::Semantics` to provide the accessible description, which it does not
do so far.

## 0.16.0: Forms (B3)

The form rules from auditmysite (casoon/barrierlab#16, B3). The norm is
WCAG 2.2 (1.3.1, 1.3.5, 3.2.1, 3.2.2, 3.3.1, 3.3.2, 3.3.7, 3.3.8), the HTML
Standard ("Autofill", form owner) and WAI-ARIA 1.2 (`aria-invalid`,
`aria-errormessage`, `aria-required`). Identifiers: `forms/*` for fields and
forms, `context/*` for the change of context on focus and input, `auth/*` for
authentication.

### New identifiers

| New identifier | Tier | Verdict, severity | replaces in auditmysite |
|---|---|---|---|
| `forms/autocomplete-invalid` | 1 | `FAIL`, low | `autocomplete-valid` "Invalid autocomplete value" (`input_purpose`) |
| `forms/purpose-missing` | 2 | `REVIEW`, medium | `autocomplete-valid` "lacks autocomplete" (`input_purpose`), `identify-purpose` (`identify_purpose`) |
| `forms/error-unidentified` | 1 | `FAIL`, medium | `input-error-message` (`form_rules`), `aria-invalid-without-describedby` (`error_identification`) |
| `forms/group-missing` | 1 | `FAIL`, medium | `form-field-group` (`form_rules`, AX and DOM part) |
| `forms/group-name-missing` | 2 | `FAIL`, medium | `label` "Form group has no legend or label" (`instructions`) |
| `forms/required-unmarked` | 2 | `REVIEW`, medium | `label` "Required field not clearly indicated" (`instructions`), `label` "may not indicate required status" (`form_rules`) |
| `forms/instructions-missing` | 2 | `REVIEW`, low | `label` "may require format instructions" (`instructions`) |
| `forms/title-only-label` | 2 | `REVIEW`, medium | `label-title-only` (`label_title_only`) |
| `forms/no-submit` | 1 | `FAIL`, medium; `REVIEW`, low without `action` and text field | `form-no-submit` (`form_rules`) |
| `forms/redundant-entry` | 1 | `REVIEW`, medium | `redundant-entry` (`redundant_entry`) |
| `context/on-input` | 1 | `FAIL`, medium; `REVIEW`, low | `input-no-context-change`, inline `onchange` (`on_input`) |
| `context/on-focus` | 1 | `REVIEW`, high | `focus-no-context-change` "onfocus" (`on_focus`) |
| `context/autofocus` | 1 | `REVIEW`, medium | `focus-no-context-change` "autofocus" (`on_focus`) |
| `auth/captcha` | 1 | `REVIEW`, medium | `accessible-auth-captcha` (`accessible_authentication`) |

### Fixes carried over

As tests in `tests/forms.rs`: auditmysite#643 (instructions via
`aria-describedby` count regardless of wording; short format terms only as a
whole word; a single checkbox is not a group, same-named ones in the same form
are), #656 (native date fields, number fields only via the label), #658 (the
name of a thing is not a person's name, also via `id` and `name`), #728 (form
consisting only of buttons: `REVIEW`, low).

### Stays in the host

The paste test on password and one-time-code fields
(`accessible-auth-paste-blocked`), looking up called functions via `window`,
and the name guess ("Language") from `on_input`.

### Not ported

Because already covered: `label` "no accessible label" and "Placeholder used as
only label" (`instructions`) as well as `labels.rs` `check_form_control` —
`forms/label-missing`, `forms/placeholder-as-label` and
`names/required-missing` report the same cases, including ARIA widgets without
a name, empty or dangling `aria-labelledby` and empty `<label for>`. Likewise
the page-wide `UNTESTED` note from `identify_purpose` (1.3.6 for icons and
regions).

### Deviations

- `forms/autocomplete-invalid` checks the whole grammar of the HTML Standard
  (`[section-*] [shipping|billing] [contact] field-name [webauthn]`), not just
  the last token, with the complete list of field names. `input` (except
  `hidden`), `select` and `textarea` are checked.
- `forms/purpose-missing` is `REVIEW` rather than a violation: whether a field
  concerns the person is guessed from label, `id` and `name`. One case instead
  of two (`identify-purpose` reported the same fields by `id`/`name`, low). The
  label is the accessible name of the host; `email` and `tel` fields always
  count. Technical addresses (`ip_address`) do not count.
- `forms/error-unidentified` reports one case that auditmysite reported twice.
  `aria-describedby` and `aria-errormessage` with text count as a description;
  a target out of view (hidden or missing — `aria/reference-missing` reports
  that) counts as a description.
- `forms/group-missing` checks structure: `<fieldset>`, `<details>` and
  `role="group"`/`"radiogroup"` as an ancestor count as a group.
- `forms/required-unmarked` is `REVIEW` rather than a violation and reports the
  case once (auditmysite: twice, low and medium): the screen reader announces
  the required field; whether it is visibly marked, only a human can see.
- `forms/title-only-label` is `REVIEW` rather than a violation: `title` is a
  permitted technique (H65). Fields with `placeholder` are reported by
  `forms/placeholder-as-label`.
- `forms/group-name-missing` does not report `radiogroup` (`names/required-missing`
  does).
- `forms/no-submit` counts fields and buttons that sit outside the form via the
  `form` attribute.
- `context/on-input` reads only the handler in the markup; if it calls a
  function, it stays `REVIEW`. New: also radio fields with `onchange` (F37).
- `context/on-focus` and `context/autofocus` are `REVIEW` rather than a
  violation: whether an `onfocus` changes the context is not in the markup, and
  `autofocus` by itself does not change it. Severities unchanged.

## 0.17.0: Landmarks, keyboard, structure (B4)

The landmark, keyboard and structure rules from auditmysite
(casoon/barrierlab#17, B4). The norm is WAI-ARIA 1.2 (landmark roles,
`aria-activedescendant`), HTML-AAM (`header`, `footer`, `aside`, `section`,
`form`) and WCAG 2.2 (1.3.1, 2.1.1, 2.4.1, 2.4.3, 4.1.2). All new rules except
`headings/none` are Tier 2: whether `<form>` and `<section>` are landmarks
depends on their accessible name, whether an element is interactive on its
role. The rule determines the landmark role itself from the markup, not from
the role of the host — `accname` and older Chrome versions still give a
`<header>` in `<main>` the role `banner`.

### New identifiers

| New identifier | Tier | Verdict, severity | replaces in auditmysite |
|---|---|---|---|
| `landmarks/not-unique` | 2 | `REVIEW`, medium | `landmark-unique` (`landmark_granular`) |
| `landmarks/not-top-level` | 2 | `FAIL`, medium | `landmark-banner-is-top-level`, `landmark-contentinfo-is-top-level`, `landmark-main-is-top-level` (`landmark_granular`) |
| `landmarks/banner-duplicate` | 2 | `FAIL`, medium | `landmark-no-duplicate-banner` (`landmark_granular`) |
| `landmarks/contentinfo-duplicate` | 2 | `FAIL`, medium | `landmark-no-duplicate-contentinfo` (`landmark_granular`) |
| `landmarks/content-outside` | 2 | `FAIL`, medium | `region` (`region`) |
| `headings/none` | 1 | `FAIL`, medium; `REVIEW`, low behind an open dialog | `bypass` "No headings found" (`bypass_blocks`) |
| `keyboard/focusable-no-role` | 2 | `REVIEW`, low | `focusable-no-role` (`keyboard`) |
| `keyboard/interactive-not-focusable` | 2 | `REVIEW`, high | `keyboard` "appears not keyboard-focusable" (`keyboard`) |
| `dialog/focusable-missing` | 2 | `FAIL`, medium (2.4.3) | `dialog-no-focusable` (`patterns/modal_dialog`) |
| `patterns/accordion-controls-missing` | 2 | `REVIEW`, low | `accordion-no-controls` (`patterns/accordion`) |

### Fixes carried over

As tests in `tests/landmarks.rs`: auditmysite#639 (`<header>`/`<footer>` in
`main`, `article` or under `role="main"` are not banner/contentinfo), #642
(skip link recognized by its target, not by its text), #709 (no headings behind
an open dialog: note), #727 (an unnamed `<form>`/`<section>` is not a
landmark — neither for uniqueness nor as a surrounding landmark). #638 and #644
concern tables and `redundant-role`, not a rule of this step.

### Stays in the host

`keyboard-trap` (2.1.2) — the note per modal dialog and the page-wide
`UNTESTED` note need real keyboard operation. From `patterns/`, the pattern
detection and the journeys, plus `accordion-trigger-not-button` and
`aria-expanded-required` (see below).

### Not ported

- `accordion-trigger-not-button` — on roles that do not support
  `aria-expanded`, `aria/attribute-not-allowed` already reports it; on roles
  that do support it (`link`, `menuitem`, `tab`, `treeitem`, …), WAI-ARIA 1.2
  explicitly allows the state, and missing focus falls under
  `keyboard/interactive-not-focusable`.
- `aria-expanded-required` — guesses an expandable navigation from the word
  "menu"/"Menü" in the name; language-dependent like the error from #642, with
  no norm behind it.
- `tab-no-aria-selected` and `aria-dialog-name` already ran as
  `aria/tab-selected-missing` and `dialog/name-missing` (B1, B2); missing main
  and banner landmarks and duplicate main already as `landmarks/main-missing`,
  `landmarks/banner-missing` and `landmarks/main-duplicate` (B0).

### Deviations

- `landmarks/not-unique` is `REVIEW` rather than a violation: WAI-ARIA 1.2 and
  the APG require distinguishable names only as SHOULD, WCAG 1.3.1 presupposes
  none on landmarks. As in auditmysite, identically named and identically
  unnamed landmarks count; several `main`, `banner` or `contentinfo` therefore
  report in addition to their duplicate finding.
- `<aside>` in `article`, `aside`, `nav` or `section` is `complementary` only
  with a name (HTML-AAM).
- `landmarks/banner-duplicate` and `landmarks/contentinfo-duplicate` point,
  like `landmarks/main-duplicate`, at the second landmark; auditmysite at the
  first.
- `landmarks/content-outside` reports the outermost element without a landmark
  inside it, once per block (like axe `region`), not every text node and every
  named element individually. If text sits directly next to a landmark, the
  surrounding container carries the finding. The number of findings per page
  drops as a result.
- `keyboard/focusable-no-role` counts only the tab sequence (`tabindex` ≥ 0):
  `tabindex="-1"`, for instance on the target of a skip link, is not reached
  by Tab. A named region (`region`) with `tabindex="0"` is the recommended
  pattern for scrollable areas and does not count.
- `keyboard/interactive-not-focusable` excludes disabled fields, native
  `<option>` (Chrome does not list them as `option`), elements under
  `aria-activedescendant` and anything inert. Focusable means, as in Chrome,
  `focusable`: natively or with any `tabindex`, including `-1` (tabs with a
  roving `tabindex`).
- `dialog/focusable-missing` searches all descendants, auditmysite only the
  direct children. A closed `<dialog>` does not count.
- `patterns/accordion-controls-missing` is `REVIEW` rather than a violation:
  WAI-ARIA 1.2 does not require `aria-controls` on the button, and the APG
  lists it as optional for the disclosure pattern. Excluded as in auditmysite:
  collapsed buttons, `<summary>` and buttons in `navigation`/`banner`.
- `headings/none` also counts `role="heading"`; hidden headings do not count.

## 0.18.0: Links and pointer (B5)

The links and pointer rules from auditmysite (casoon/barrierlab#18, B5). The
norm is WCAG 2.2 (2.1.1, 2.4.8, 4.1.2), the HTML Standard (`<a>` without
`href` is a placeholder, not a link) and WAI-ARIA 1.2 (`aria-current`). All
three rules are Tier 1: they read only attributes.

### New identifiers

| New identifier | Tier | Verdict, severity | replaces in auditmysite |
|---|---|---|---|
| `keyboard/click-handler-not-focusable` | 1 | `FAIL`, high (2.1.1) | `click-events-have-key-events` (`click_handlers`) |
| `links/used-as-button` | 1 | `FAIL`, low (4.1.2) | `link-as-button` (`fake_navigation_link`) |
| `navigation/location-missing` | 1 | `REVIEW`, low (2.4.8, AAA) | `location` (`location`) |

### Evidence

The auditmysite corpus (`keyboard_and_targets`, `audit_exclude_cap`) and the 48
real pages. There, `links/used-as-button` found the call of the consent
settings (`<a href="#" onclick="UC_UI_recall();">`, wetter.com) and a reload
link (craigslist.org); `keyboard/click-handler-not-focusable` found nothing —
wetter.com attaches handlers via `data-onclick`, which is not a handler.
`navigation/location-missing` reports on 30 of the 48 pages, almost all of them
home pages; hence `REVIEW`.

### Not ported

`pointer_cancellation` (2.5.2): the static part looks for `onmousedown` and
`ontouchstart` on controls. That occurred on none of the 48 pages and in no
corpus case — without evidence the rule does not get in. The page-wide
`UNTESTED` note belongs to the manual checklist (casoon/barrierlab#39).

### Deviations

- `<a onclick>` **without** `href` reports
  `keyboard/click-handler-not-focusable`, not `links/used-as-button`: without
  `href` the element is not a link, is not announced as a link and is not
  focusable. auditmysite reports it as a fake link.
- `navigation/location-missing` is `REVIEW` rather than a violation: 2.4.8 can
  also be met with a title, headings or a sitemap. `aria-current="true"` does
  not count as a location indication, as in auditmysite — on t-online.de it
  marks carousel dots.

## 0.18.0: Images and media (B6)

The image and media rules from auditmysite (casoon/barrierlab#19, B6). The norm
is WCAG 2.2 (1.1.1, 1.4.2, 2.4.1, 4.1.2) and the HTML Standard (`<area>`,
`<input type="image">`, `<object>`, `ismap`, `<iframe>`). Five rules read only
attributes and text and are Tier 1; `frames/name-missing` needs the name
computation (`aria-labelledby`) and is Tier 2.

### New identifiers

| New identifier | Tier | Verdict, severity | replaces in auditmysite |
|---|---|---|---|
| `images/area-alt-missing` | 1 | `FAIL`, high (1.1.1) | `area-alt` (`image_input_rules`) |
| `images/input-alt-missing` | 1 | `FAIL`, high (1.1.1) | `input-image-alt` (`image_input_rules`) |
| `objects/alt-missing` | 1 | `FAIL`, high (1.1.1) | `object-alt` (`image_input_rules`) |
| `images/server-side-map` | 1 | `FAIL`, medium (1.1.1) | `server-side-image-map` (`server_side_image_map`) |
| `media/audio-autoplay` | 1 | `REVIEW`, medium (1.4.2) | `background-audio` (`background_audio`) |
| `frames/name-missing` | 2 | `FAIL`, high (2.4.1, 4.1.2) | `frame-title` (`media_rules`) |

### Evidence

The auditmysite corpus (`misc_content_checks`, `forms_and_misc`,
`object_no_alt`, `media_and_visual`, `frame_missing_title`) and the 48 real
pages. There, only `frames/name-missing` reports: the three sports-data frames
on n-tv.de with `title=""`. The sports-data frame on spiegel.de (without
`title`) is inserted by Alpine only in the browser from a `<template>`; it is
a case in the tests. `<area>`, `<input type="image">`, `<object>`, `ismap` and
`<audio autoplay>` occur on none of the 48 pages — the evidence for them is the
corpus cases.

The platform list for `manual/media-alternatives` (YouTube, Vimeo, Dailymotion,
Wistia) comes from auditmysite's `video-caption`. Whether such a video has
captions is controlled by the platform's player. Evidence: the YouTube video on
w3.org/WAI, which previously triggered no checklist item.

### Not ported

- `media-alt` (`media_alternative`, 1.2.8): reports an `UNTESTED` note on every
  page, even without media. Where media exist, `manual/media-alternatives` says
  the same (transcript, audio description); without media there is nothing to
  check. Hosts map `media-alt` to `manual/media-alternatives`.
- `video-caption` (`media_rules`, 1.2.2): without a checked caption file it
  stays at the `UNTESTED` note, and `manual/media-alternatives` gives that.
  Whether a `<track>` file actually loads is checked only by the host over the
  network; that part stays in auditmysite, as does `frame-tested` (whether a
  frame is foreign, only the browser knows).
- `role="application"` without a name (`media_rules`): no evidence — neither in
  the corpus nor on the 48 pages is there an unnamed one.
- `role="img"` without a name (`media_rules`): SVGs are covered by
  `svg/name-missing`, `<img>` by `images/alt-missing`; an unnamed other element
  with `role="img"` occurs neither in the corpus nor on the 48 pages.
- Named decorative element (`media_rules`): `aria-label` on
  `role="presentation"`/`"none"` is already reported by
  `aria/attribute-prohibited`. The only cases on the 48 pages (wikipedia.org,
  `title` on `role="presentation"`) carry `aria-hidden="true"` and are not in
  the accessibility tree.
- `<embed>` from `object-alt`: no evidence.

### Deviations

- `aria-labelledby` counts as an alternative on `<area>`, `<input type="image">`
  and `<object>`, `aria-label` also on `<area>`. auditmysite knows only `alt`
  on `<area>` and otherwise only `aria-label`.
- `media/audio-autoplay` is `REVIEW` and cites 1.4.2 instead of 1.4.7: whether
  the audio plays longer than three seconds and can be stopped is not in the
  markup; self-starting audio is the subject of 1.4.2 (Audio Control), while
  1.4.7 concerns background sounds under speech.
- `frames/name-missing` cites 2.4.1 and 4.1.2 (technique H64), auditmysite only
  2.4.1. Without computed styles, a frame with inline `display: none` counts as
  invisible (craigslist.org), plus, as in auditmysite, one with width and
  height 0 or 1 (duckduckgo.com). With styles (Tier 3) the view removes hidden
  frames anyway; the check for a rendered size of ≤ 1 px stays with the host.

## 0.18.0: Presentation convention (#22)

The static rules of the presentation convention (casoon/barrierlab#22, draft
v0): `figure[data-viz]` with text, still-image and live layers,
`html[data-display]` with a toggle. The convention is described on the page
[Presentation modes](../a11y/concepts/darstellungsmodi.md).

**Decision on the package:** `a11y-rules`, not `web-checks`. The rules need the
DOM — ancestors, descendants, references via `aria-describedby` — and in doing
so read only attributes and structure; all seven are Tier 1 (module `viz`).
They report only on pages that use the convention, carry the keyword
`best-practice` and, as in auditmysite, attach to the nearest WCAG criterion.
The `display/*` identifiers are the same as in auditmysite
(`src/wcag/rules/display_modes.rs`); the measured part there (computed
visibility, timing of `data-display`, text mode) stays there.

### New identifiers

| New identifier | Tier | Verdict, severity | in auditmysite |
|---|---|---|---|
| `viz/text-missing` | 1 | `FAIL`, high (1.1.1) | — (`display/text-not-visible` measures the text mode) |
| `display/text-hidden` | 1 | `FAIL`, high; `REVIEW`, low when referenced via `aria-describedby`/`aria-details` (1.1.1) | `display/text-hidden`, attribute part |
| `viz/caption-missing` | 1 | `FAIL`, low (1.1.1) | — |
| `viz/static-missing` | 1 | `FAIL`, medium (2.2.2) | — |
| `viz/table-missing` | 1 | `REVIEW`, low (1.3.1) | — |
| `display/toggle-missing` | 1 | `FAIL`, medium (2.2.2) | `display/toggle-missing`, static part |
| `display/init-missing` | 1 | `REVIEW`, low (2.2.2) | `display/init-missing`, static part |

### Evidence

The built pages of the Geographia reference implementation (371 pages,
`web-geographia/apps/*/dist`, state of 2026-10-03). `viz/text-missing` reports
the home page, which describes its diagrams in `.viz-desc` without
`[data-viz-text]`, and "Build Earth 2.0" (`space/solar-system/settlement/`),
which also triggers `viz/static-missing`; `viz/caption-missing` the climate
monitor and Germany maps without `<figcaption>`; `display/toggle-missing` the
lab pages without a header bar; `viz/table-missing` 196 diagrams, mostly
sparklines. `display/text-hidden` follows the case from auditmysite#704
(`div#layers-home-desc` on geographia.eu/atmosphere/) including the test cases
recorded there; in today's build Geographia hides only visually.
`display/init-missing` reports nothing there. On the 48 real pages without the
convention, none of the seven rules reports.

### Limits and not ported

- The core statement is checked only for "not empty", not for quality.
- `viz/orphan-media` from the draft (`canvas`, `video`, `svg[role=img]` outside
  the convention) is not included: in Geographia every such medium sits in
  `[data-viz]` or under `aria-hidden`; a real positive case is missing.
- `display/init-missing` can only establish absence; whether an existing script
  sets the mode shows only on the running page.

### Deviations

- `viz/*` does not exist in auditmysite; there, `display/text-not-visible`
  checks the text only in the rendered text mode.
- If the `figure` itself is hidden via `hidden`, `display/text-hidden` reports
  nothing (as in auditmysite#725); `aria-hidden` or `inert` on the `figure`,
  however, remain a finding because the graphic is then visible.

## 0.18.0: Tables, document, language, roles (B7)

The table, document, language and role rules from auditmysite
(casoon/barrierlab#20, B7). The norm is WCAG 2.2 (1.3.1, 1.4.13, 2.2.1,
2.4.10, 3.1.1, 3.1.2, 3.1.4, 4.1.2), the HTML Standard (`headers`,
`lang`/`xml:lang`, "shared declarative refresh steps"), ARIA in HTML
(redundant roles) and WAI-ARIA 1.2. All rules are Tier 1: they read tags,
attributes and text.

### New identifiers

| New identifier | Tier | Verdict, severity | replaces in auditmysite |
|---|---|---|---|
| `tables/header-without-data` | 1 | `FAIL`, high (1.3.1) | `th-has-data-cells` (`table_extended`) |
| `tables/data-undetermined` | 1 | `UNTESTED`, high (1.3.1) | `th-has-data-cells`, `incomplete` (`table_extended`) |
| `tables/headers-attr-invalid` | 1 | `FAIL`, high (1.3.1) | `td-headers-attr` (`table_extended`) |
| `document/lang-mismatch` | 1 | `FAIL`, medium (3.1.1) | `html-xml-lang-mismatch` (`language_extended`) |
| `language/part-unmarked` | 1 | `REVIEW`, medium (3.1.2) | `language-of-parts` (`language_of_parts`) |
| `language/part-undetermined` | 1 | `UNTESTED`, medium (3.1.2) | — (auditmysite is silent on other languages) |
| `language/abbreviation-unexpanded` | 1 | `REVIEW`, low (3.1.4, AAA) | `abbreviations` (`abbreviations`) |
| `timing/meta-refresh` | 1 | `FAIL`, high (2.2.1) | `meta-refresh` (`timing_adjustable`) |
| `headings/section-without-heading` | 1 | `REVIEW`, low (2.4.10, AAA) | `heading-order` (`section_headings`, section count) |
| `aria/role-redundant` | 1 | `FAIL`, low (4.1.2) | `redundant-role` (`redundant_role`) |
| `names/title-only` | 1 | `REVIEW`, medium (4.1.2) | `title-only-description` (`content_on_hover`) |
| `patterns/tooltip-unreferenced` | 1 | `FAIL`, low (1.4.13) | `content-on-hover-focus` (`content_on_hover`) |

### Evidence

The auditmysite corpus and the 48 real pages. Hits there:

- `aria/role-redundant`: 251 findings on 5 pages — wetter.com 188
  (`li[role=menuitem] > a[href][role=link]`), lidl.de 39 and sparkasse.de 16
  (`a[href][role=link]`), bahn.de and spiegel.de 4 each (`button[role=button]`
  as accordion trigger). Plus corpus `redundant_role_list_style`.
- `names/title-only`: 200 findings on 10 pages — spiegel.de 163 (teaser links
  around an image with empty `alt`), focus.de 16 (icon links), bahn.de 7
  (search buttons), plus heise.de, tagesschau.de, faz.net, sueddeutsche.de,
  check24.de, zeit.de and basf.com. Plus corpus
  `name_description_best_practice`.
- `headings/section-without-heading`: one note on 7 pages, almost only teaser
  cards as `<article>` (t-online.de 61, faz.net 39, bild.de 25, check24.de 20,
  basf.com 15, web.de 4, spiegel.de 1).
- On none of the 48 pages, evidenced in the corpus: `tables/header-without-data`
  (`table_headers_no_data`, `table_grid_rows_unrendered`),
  `tables/headers-attr-invalid` (`forms_extended`), `document/lang-mismatch`
  and `language/abbreviation-unexpanded` (`misc_content_checks`),
  `language/part-unmarked` (`text_and_layout`), `timing/meta-refresh`
  (`meta_refresh_present`), `patterns/tooltip-unreferenced`
  (`forms_and_misc`). The only hit of `language/part-unmarked` on real pages
  was a false alarm and has been fixed (embedded CSS, see Deviations).

### Fixes carried over

From auditmysite, each as a test:

- auditmysite#638: empty data cells, cells with an `aria-hidden` child and a
  column header above row headers are not a "header without data"
  (`korpus_table_headers_tbody_ignored_issue_638`,
  `spaltenkopf_ueber_zeilenkoepfen_barrierlab_eu_issue_638`).
- auditmysite#639: the table in the chapter with `<header>` in the article
  (`tabelle_im_kapitel_geographia_issue_639`; the landmark part has been in
  `tests/landmarks.rs` since B4).
- auditmysite#654: a row supply not yet rendered is
  `tables/data-undetermined`, not `FAIL`
  (`korpus_table_grid_rows_unrendered_issue_654`,
  `native_tabelle_mit_koerper_issue_654`).
- auditmysite#659: `tbody > tr` with row header and data cell
  (`korpus_table_required_rows_tbody_issue_659`; the role rule has been in
  `tests/aria.rs` since B1).
- auditmysite#644: `role="list"` on `<ul>`/`<ol>` is not reported
  (`korpus_redundant_role_list_style_issue_644`).

### Not ported

- `presentation-semantic-children` (`info_relationships`): according to
  WAI-ARIA 1.2, `role="presentation"`/`"none"` removes the semantics only from
  the element itself; descendants keep theirs, except the required owned
  elements of tables and lists. The table case is already reported by
  `tables/presentational-with-headers`, F92 concerns the element itself. On
  the 48 pages the rule would have reported 187 `<li role="none">` around menu
  links (t-online.de: the APG menubar pattern) — consistently false alarms.
- `section_headings`, gaps in the outline: already reported by
  `headings/skip-level`.
- `section_headings`, "more than 10 paragraphs, fewer than 3 headings": no
  evidence — on none of the 48 pages, in no corpus case.
- `timing_adjustable`: the page-wide `UNTESTED` note for script deadlines and
  `timeouts` (2.2.6) belongs to the checklist (`manual/timing`).
- `redundant_role` for `<ul>`/`<ol>` with `role="list"`: whether the role is
  redundant depends on the computed `list-style-type` (auditmysite#644), and
  `ComputedStyle` in `a11y-dom` did not have this field in 0.18.0. Until it
  did, the pair was not checked rather than guessed; `aria/role-redundant`
  says nothing about lists. auditmysite kept reporting `ol#numbered-list` in
  the corpus with style itself until `a11y-dom` supplied the field (since
  0.20.0, see [below](#0200-presentation-values-21-part-two)).

### Deviations

- `tables/data-undetermined` recognizes pending rows by a row group without a
  row in the view — also on the empty `<tbody>` in the markup that is waiting
  for its data. If the whole row group is hidden, it is missing from the view,
  and the header cells report `FAIL`.
- `tables/headers-attr-invalid` checks only cells (`td`, `th` and their roles);
  HTML defines the attribute only there.
- `language/part-unmarked` reports the innermost text block (`<li><p>` once),
  skips `<script>`, `<style>` and `<template>` — on sparkasse.de the embedded
  CSS otherwise read as English — and on pages that are neither German nor
  English reports `language/part-undetermined` instead of nothing.
- `language/abbreviation-unexpanded` is `REVIEW` rather than a violation: 3.1.4
  can also be met by spelling out in the text or a glossary. An empty `title`
  counts as missing.
- `timing/meta-refresh` does not report `0` s: an immediate redirect is not a
  deadline (H76, axe `meta-refresh`); web.de and gmx.net redirect this way
  without JavaScript. An instruction without digits is not executed by the
  browser.
- `headings/section-without-heading` counts articles and named sections
  without a heading of their own, excluding navigations, instead of all
  sections against all headings of the page; `REVIEW` rather than a violation.
- `aria/role-redundant` reports `<li role="listitem">` only in a list without
  a role of its own: in `ul[role=list]` it belongs to the same WebKit
  workaround (lidl.de).
- `names/title-only` is `REVIEW` rather than a violation, like
  `forms/title-only-label`: `title` is a valid name source (H65). Text fields
  are reported by `forms/title-only-label`; `submit` and `reset` have a default
  name.
- `patterns/tooltip-unreferenced` accepts `aria-labelledby` as a reference in
  addition to `aria-describedby`.

## 0.19.0: Stylesheets (#21, part one)

The rules about stylesheets from auditmysite (casoon/barrierlab#21, part one).
There, JavaScript read `document.styleSheets`; here the sheets come in as
text, parsed by the `stylesheet-parse` package. The norm is WCAG 2.2 (1.3.4,
1.4.8, 2.3.3, 2.4.7), CSS Syntax Level 3 and Selectors Level 4.

### New identifiers

| New identifier | Verdict, severity | replaces in auditmysite |
|---|---|---|
| `focus/outline-removed` | `FAIL`, high (2.4.7) | `focus-visible-outline-none` (`focus_visible_css`) |
| `motion/reduced-motion-ignored` | `REVIEW`, medium (2.3.3, AAA) | `prefers-reduced-motion` (`reduced_motion`) |
| `orientation/content-hidden` | `REVIEW`, medium; high for `html`/`body`/`main` (1.3.4) | `css-orientation-lock` (`orientation`), stylesheet part |
| `text/justified` | `REVIEW`, low (1.4.8, AAA) | `visual-presentation` (`visual_presentation`), justified text |
| `text/line-height-tight` | `REVIEW`, low (1.4.8, AAA) | `visual-presentation`, line spacing |

The selector matching against the document (a subset of Selectors Level 4,
three-valued: match, no match, unknown) ensures that only what occurs on the
page counts — like `querySelectorAll` in auditmysite (#712).

### Evidence

The auditmysite corpus (`media_and_motion`, `reduced_motion_transform`,
`reduced_motion_override`, `text_and_layout`) and the stylesheets of the 48
real pages (337 sheets, 24 MB, fetched 2026-10-03). On them,
`motion/reduced-motion-ignored` reports 30 pages, `text/line-height-tight` 21
(for instance gov.uk with 1.32 in body text), `focus/outline-removed`,
`orientation/content-hidden` and `text/justified` none.

### Deviations

- `focus/outline-removed`: an outline restored in a `:focus` rule counts as a
  replacement. sueddeutsche.de removes it for everyone and restores it for the
  keyboard (`[data-whatintent='keyboard'] *:focus`); auditmysite reported that
  as a violation.
- `orientation/content-hidden` is `REVIEW` rather than a violation and reports
  only selectors that hit an element of the page, without pseudo-elements. In
  auditmysite's form the rule triggered on four of the 48 pages: twice
  breakpoint markers (`body:before`, two pages of bundesregierung.de), a close
  button (n-tv.de) and welt.de. The real case, a dialog that shows "please
  rotate" in landscape (welt.de), is in the document only once the dialog is
  open. The check of `transform: rotate` on the computed style of `body`/`html`
  stays with the host.
- `text/*`: measured on the `<p>` elements of the page, not on selectors that
  begin with `body`, `p`, `div` … Otherwise `html { line-height: 1.15 }` from
  normalize.css would be reported on every page. Line height only without
  unit, in `em` or `%`. `REVIEW` rather than a violation; the `UNTESTED` note
  for color choice and column width belongs to the checklist.
- `motion/reduced-motion-ignored`: one report per page as in auditmysite, but
  `REVIEW` rather than a violation. A standalone `transition-duration` is not
  a transition (Tailwind `.duration-200`).

### Still open afterwards

`list-style-type` (for `redundant_role` on lists), link color against text
color (`use_of_color`), scroll extents (`scrollable_region`) and
`non_text_contrast_css`. They needed new fields in `ComputedStyle`/`Layout`
that a host run had to be seen filling first (auditmysite#698). The first
three came with 0.20.0, `non_text_contrast_css` with 0.23.0.

## 0.20.0: Presentation values (#21, part two)

The second part of casoon/barrierlab#21: three rules from auditmysite that need
measured presentation values. The norm is WCAG 2.2 (1.4.1, 2.1.1, 4.1.2) and
ARIA in HTML. All three are Tier 3 and judge with `FAIL` — what they read is
measured, not assumed. If the host does not supply a field, the report contains
one `UNTESTED` per rule and page, not a silent non-finding.

### New identifiers

| New identifier | Verdict, severity | needs | replaces in auditmysite |
|---|---|---|---|
| `lists/role-redundant` | `FAIL`, low (4.1.2) | `ComputedStyle::list_style_type` | `redundant-role` for `<ul>`/`<ol>` (`redundant_role`) |
| `color/link-indistinct` | `FAIL`, medium (1.4.1) | `text_decoration_line`, `font_style`, `font_family`, `border_bottom_style` | `link-in-text-block` (`use_of_color`) |
| `keyboard/scrollable-region-not-focusable` | `FAIL`, high (2.1.1) | `Rendering::scroll_overflow_px` | `scrollable-region-focusable` (`scrollable_region`) |

### Evidence

Evidence and fixes carried over from the auditmysite corpus
(`tests/darstellung.rs`): `redundant_role_list_style` (#644: `role="list"` with
`list-style: none` is not redundant), `link_in_text_block_context` (#710: only
links in running text; navigation, logo, lists of only one link and footers
with separators do not count), `scrollable_region_focusable` (#717: gov.si
`div.menus`; a region with `tabindex`, with a link inside, with
`overflow: hidden` or without overhang passes; buffer 13 px as in axe).

auditmysite (casoon/auditmysite#698) saw the new fields filled against the
unreleased state before the release: on gov.uk, bundesregierung.de,
wetter.com, n-tv.de and spiegel.de on all rendered elements in each case,
colors and background somewhat less often there (gradient, background image,
transparent text fill). The three corpus cases report as expected.

### Deviations

- `color/link-indistinct` compares, like auditmysite, underline, weight, style,
  font, bottom border and background with the parent element. The background
  here is the effective one, which the host resolves across the ancestors
  (`ComputedStyle::background_color`), not the element's own.
- `keyboard/scrollable-region-not-focusable` does not itself check whether
  content is visible — the rule's view takes care of that (hidden subtrees are
  missing from it).

## 0.21.0: Contrast (#47)

Contrast and `Layout` after the comparison with auditmysite's own contrast rule
(casoon/barrierlab#47, casoon/auditmysite#698).

### New identifiers

| New identifier | Verdict, severity | Norm |
|---|---|---|
| `contrast/text-enhanced` | `FAIL`, medium | WCAG 1.4.6 (AAA): 7:1, 4.5:1 for large text |

Only what passes 1.4.3 is reported; what already fails there is not listed
twice. Evidence: the link color of gov.uk, #1d70b8 on white, 5.17:1.

### Evidence

Before the release, auditmysite (casoon/auditmysite#698) filled the new inputs
in real runs: `visually_hidden` and `obscured` on gov.uk, bundesregierung.de,
wetter.com, n-tv.de and spiegel.de (the search button of gov.uk recognized as
hidden), `sampled_backdrop` on berlin.de and in the image fixtures, which
thereby keep their previous verdicts.

- Contrast over images and gradients is judged as in auditmysite before the
  switch. Evidence: auditmysite's `tests/fixtures/image_contrast.html` (dark
  gradient passes, light one fails, split one is a note).
- Optically hidden text: gov.uk, `button.gem-c-search__submit`, "Search GOV.UK"
  hidden via `text-indent: -5000px`, previously `FAIL` 3.91:1.

### Deviations

- Text under `aria-hidden` is still measured. WCAG 1.4.3 applies to visible
  text, independent of the accessibility tree. auditmysite excluded it (#395).

## 0.22.0: Target sizes and heuristics (#698)

The heuristics after the comparison with auditmysite's local rules
(casoon/auditmysite#698, corpus and six real pages, 2026-10-05).

### New identifiers

| New identifier | Verdict, severity | Norm |
|---|---|---|
| `targets/size-enhanced` | `REVIEW`, low | WCAG 2.5.5 (AAA): 44 × 44 CSS px, exceptions "inline" and "Equivalent", no spacing exception |

Only what `targets/size` does not already report is reported.

### Evidence

Before the release, auditmysite filled the new fields in real runs:
`pointer_events_none` (bundesregierung.de 114, spiegel.de 120, gov.uk 6,
berlin.de 2) and `animating` (bundesregierung.de 246, wetter.com 89,
spiegel.de 23, `target_size_animation` `a#slow`).

Corpus cases for `targets/size`: `target_size_equivalent`,
`target_size_hidden_neighbours` (#705), `target_size_animation` (#706). Real
pages: skip links at −99999 px and carousel dots of 10 × 10 px
(`#slick-slide-control00`) on bundesregierung.de, 27 teaser cards with
`cursor: pointer` there (`keyboard/pointer-only`), 20 teaser cards with image
and title link in `flex-row-reverse` (`order/visual-mismatch`) and a 1 × 1 px
skip link (`focus/obscured`) on spiegel.de.

### Deviations

- `motion/infinite-animation`: loading spinners with a short cycle remain
  reported, because 2.2.2 applies to motion that runs for more than five
  seconds in total. auditmysite counted only cycles over five seconds.

## 0.23.0: Review notes and contrast of controls

### Checklist

Seven new `manual/*` items for the review notes that auditmysite previously
generated itself (casoon/barrierlab#39, casoon/auditmysite#722). Like the
others, `UNTESTED`, once per page and only with a trigger. Previously,
auditmysite put each of these notes on every page, even without a trigger.

| Identifier | WCAG | Trigger |
|---|---|---|
| `manual/pointer-gestures` | 2.5.1 | `draggable`, touch/pointer handlers in the markup, carousel and map libraries (`swiper`, `slick`, `leaflet` …) |
| `manual/motion-actuation` | 2.5.4 | inline script with `devicemotion`/`deviceorientation` (own rule, `Scope::Markup`) |
| `manual/no-timing` | 2.2.3 | fields or `meta refresh` |
| `manual/interruptions` | 2.2.4 | live region, `role=alert/status/log/marquee/timer`, `<dialog>`, `meta refresh` |
| `manual/re-authenticating` | 2.2.5 | login (password field, captcha) |
| `manual/unusual-words` | 3.1.3 | text |
| `manual/help` | 3.3.5 | fields |

### Contrast of controls

`contrast/control-insufficient` and `contrast/control-undetermined` (1.4.11)
are a port of auditmysite's `non_text_contrast_css` (casoon/barrierlab#21,
casoon/auditmysite#698). Unlike there, no comparison against white is drawn
when the background is missing; `UNTESTED` is reported instead.

## 0.24.0: Link purpose

Link purpose from auditmysite (`link_purpose`, `link_purpose_link_only`;
casoon/barrierlab#13, #18). auditmysite#690 had deferred both rules to B5; B5
then did not take them along. All findings are Tier 2 and `REVIEW`: whether a
link text suffices is ultimately decided by the context.

### New identifiers

| New identifier | WCAG, severity | replaces in auditmysite |
|---|---|---|
| `links/url-name` | 2.4.4, low; only without context | "raw URL" from `link_purpose` |
| `links/single-character-name` | 2.4.4, medium; digits excluded | "single character" from `link_purpose` |
| `links/new-window-unannounced` | 3.2.5 (AAA), low | "opens in new window" from `link_purpose` |
| `links/purpose-link-only` | 2.4.9 (AAA), low | `link-name-only` (`link_purpose_link_only`) |

### Deviations

- `links/new-window-unannounced` checks `target="_blank"`. auditmysite checked
  `aria-haspopup` and never hit the intended case. G201 belongs to 3.2.5, not
  to 2.4.4 as previously in auditmysite.
- `links/generic-name` evaluates the context as auditmysite #569 does (plan
  71, technique H80).
