<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: confirmed ARIA-in-HTML restriction rules from
  `html-aria/misc` — the `lang`/`xml:lang` schema-layer bugfix
  (`plan/DECISIONS.md`) unmasked a large batch of these, previously
  hidden behind a spurious `schema.html5` error firing on nearly every
  fixture. Every condition below was verified against the actual corpus
  fixture and its expected `messages.json` text before being encoded,
  not extrapolated from the wider ARIA spec.

  Uses named element tests — needs the `h:` namespace prefix. See
  rules/tables.sch's header comment / rules/README.md for why.

  Each independent concern gets its own `<pattern>` — see
  rules/elements.sch's header comment for why (first-matching-rule-
  per-pattern gotcha).
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>

  <!--
    31 elements whose implicit ARIA role (when no explicit @role
    overrides it) is one of the "naming prohibited" roles (caption,
    code, deletion, emphasis, generic, insertion, paragraph,
    presentation, strong, subscript, superscript) — confirmed against
    all 31 `html-aria/misc/aria-label-on-<element>-novalid.html`-style
    fixtures (and the aria-labelledby/aria-braillelabel siblings).

    `a`/`area` are excluded when they carry `@href`: an `a`/`area`
    *with* `href` has implicit role "link" (naming allowed), only an
    `a`/`area` *without* `href` falls back to the naming-prohibited
    "generic" role — confirmed by
    `html-aria/name-computation-general/603.html`/`604.html`
    (`<a href="..." aria-labelledby="...">`/`aria-label="..."`, both
    expected clean).
  -->
  <pattern id="naming-prohibited">
    <!--
      Custom elements (`contains(local-name(), '-')` — an approximation
      of the WHATWG custom-element-name grammar's "must contain a
      hyphen" requirement, not the full grammar with its reserved-name
      exclusions; see `src/datatypes/structural.rs::check_custom_element_name`
      for the exact Rust-side check used elsewhere in this crate, not
      replicated here in XPath) also have an implicit "generic" role —
      confirmed by html-aria/misc/aria-braillelabel-autonomous-custom-element-novalid.html
      and its aria-label/aria-labelledby siblings (`<custom-el
      aria-braillelabel="...">`, expected an error). None of the 31
      standard elements below contain a hyphen, so there is no overlap
      to worry about.
    -->
    <rule context="*[self::h:a[not(@href)] or self::h:abbr or self::h:area[not(@href)] or self::h:b or self::h:bdi or self::h:bdo or self::h:caption or self::h:cite or self::h:code or self::h:data or self::h:del or self::h:div or self::h:em or self::h:figcaption or self::h:i or self::h:ins or self::h:kbd or self::h:mark or self::h:p or self::h:pre or self::h:q or self::h:s or self::h:samp or self::h:small or self::h:span or self::h:strong or self::h:sub or self::h:sup or self::h:time or self::h:u or self::h:var or contains(local-name(), '-')]
                [not(@role) or @role = 'caption' or @role = 'code' or @role = 'deletion' or @role = 'emphasis' or @role = 'generic' or @role = 'insertion' or @role = 'paragraph' or @role = 'presentation' or @role = 'strong' or @role = 'subscript' or @role = 'superscript']">
      <assert id="naming-prohibited.no-aria-label" role="error" test="not(@aria-label)">
        The "aria-label" attribute must not be specified on this element unless it has a "role" value other than the naming-prohibited roles.
      </assert>
      <assert id="naming-prohibited.no-aria-labelledby" role="error" test="not(@aria-labelledby)">
        The "aria-labelledby" attribute must not be specified on this element unless it has a "role" value other than the naming-prohibited roles.
      </assert>
      <assert id="naming-prohibited.no-aria-braillelabel" role="error" test="not(@aria-braillelabel)">
        The "aria-braillelabel" attribute must not be specified on this element unless it has a "role" value other than the naming-prohibited roles.
      </assert>
    </rule>
  </pattern>

  <!--
    15 confirmed roles that must not appear on a descendant of a
    heading element (html-aria/misc/role-*-inside-h1-novalid.html).
  -->
  <pattern id="role-forbidden-inside-heading">
    <rule context="*[@role = 'alert' or @role = 'alertdialog' or @role = 'application' or @role = 'dialog' or @role = 'document' or @role = 'feed' or @role = 'listbox' or @role = 'log' or @role = 'marquee' or @role = 'math' or @role = 'note' or @role = 'status' or @role = 'tabpanel' or @role = 'timer' or @role = 'toolbar']">
      <assert id="role-forbidden-inside-heading.not-in-heading" role="error"
        test="not(ancestor::*[self::h:h1 or self::h:h2 or self::h:h3 or self::h:h4 or self::h:h5 or self::h:h6])">
        An element with this "role" must not appear as a descendant of an "h1", "h2", "h3", "h4", "h5", or "h6" element.
      </assert>
    </rule>
  </pattern>

  <!--
    8 confirmed "widget" roles whose descendants must not include an
    interactive `a[href]` (html-aria/misc/a-href-inside-role-*-haswarn.html).
  -->
  <pattern id="a-href-forbidden-inside-widget-role">
    <rule context="h:a[@href]">
      <report id="a-href-forbidden-inside-widget-role.descendant" role="warning"
        test="ancestor::*[@role = 'checkbox' or @role = 'menuitem' or @role = 'menuitemcheckbox' or @role = 'menuitemradio' or @role = 'option' or @role = 'radio' or @role = 'switch' or @role = 'tab']">
        The element "a" with the attribute "href" should not appear as a descendant of an element with a widget role that only allows non-interactive content.
      </report>
    </rule>
  </pattern>

  <!--
    2 confirmed roles whose descendants must not include a `tabindex`
    attribute (html-aria/misc/tabindex-inside-role-*-haswarn.html).
  -->
  <pattern id="tabindex-forbidden-inside-role">
    <rule context="*[@tabindex]">
      <report id="tabindex-forbidden-inside-role.descendant" role="warning"
        test="ancestor::*[@role = 'option' or @role = 'tab']">
        An element with the attribute "tabindex" should not appear as a descendant of an element with "role=option" or "role=tab".
      </report>
    </rule>
  </pattern>

  <!--
    Extends rules/roles.sch's "unnecessary explicit role" family with
    pairs confirmed in html-aria/misc: dialog/dialog, figure/figure,
    s/deletion, button/button, a[href]/link, tbody/rowgroup, li/listitem.
  -->
  <pattern id="unnecessary-role-extra">
    <rule context="h:dialog[@role = 'dialog']">
      <report id="unnecessary-role-extra.dialog" role="warning" test="true()">
        The "dialog" role is unnecessary for element "dialog".
      </report>
    </rule>
    <rule context="h:figure[@role = 'figure']">
      <report id="unnecessary-role-extra.figure" role="warning" test="true()">
        The "figure" role is unnecessary for element "figure".
      </report>
    </rule>
    <rule context="h:s[@role = 'deletion']">
      <report id="unnecessary-role-extra.deletion" role="warning" test="true()">
        The "deletion" role is unnecessary for element "s".
      </report>
    </rule>
    <rule context="h:button[@role = 'button']">
      <report id="unnecessary-role-extra.button" role="warning" test="true()">
        The "button" role is unnecessary for element "button".
      </report>
    </rule>
    <rule context="h:a[@href and @role = 'link']">
      <report id="unnecessary-role-extra.link" role="warning" test="true()">
        The "link" role is unnecessary for element "a" with attribute "href".
      </report>
    </rule>
    <rule context="h:tbody[@role = 'rowgroup']">
      <report id="unnecessary-role-extra.rowgroup" role="warning" test="true()">
        The "rowgroup" role is unnecessary for element "tbody".
      </report>
    </rule>
    <rule context="h:li[@role = 'listitem']">
      <report id="unnecessary-role-extra.listitem" role="warning" test="true()">
        The "listitem" role is unnecessary for element "li".
      </report>
    </rule>
  </pattern>

  <!--
    select[role=listbox]: unnecessary (warning) when @multiple or
    @size>1 already imply it; not allowed (error) otherwise —
    confirmed by both select-without-multiple-bad-role-novalid.html and
    unnecessary-listbox-role-on-select-haswarn.html.
  -->
  <pattern id="select-listbox-role">
    <rule context="h:select[@role = 'listbox']">
      <assert id="select-listbox-role.requires-multiple-or-size" role="error"
        test="@multiple or number(@size) &gt; 1">
        The "listbox" role is not allowed for element "select" without a "multiple" attribute and without a "size" attribute whose value is greater than 1.
      </assert>
      <report id="select-listbox-role.redundant" role="warning"
        test="@multiple or number(@size) &gt; 1">
        The "listbox" role is unnecessary for element "select" with a "multiple" attribute or with a "size" attribute whose value is greater than 1.
      </report>
    </rule>
  </pattern>

  <!-- Extends elements.sch's multiple-main check to also count role=main. -->
  <pattern id="multiple-main-with-role">
    <rule context="h:html">
      <assert id="multiple-main-with-role.at-most-one-visible" role="error"
        test="count((descendant::h:main | descendant::*[@role = 'main'])[not(@hidden)]) &lt;= 1">
        A document should not include more than one visible element with "role=main".
      </assert>
    </rule>
  </pattern>

  <pattern id="a-href-aria-disabled">
    <rule context="h:a[@href]">
      <report id="a-href-aria-disabled.true" role="warning" test="@aria-disabled = 'true'">
        An "aria-disabled" attribute whose value is "true" should not be specified on an "a" element that has an "href" attribute.
      </report>
    </rule>
  </pattern>

  <pattern id="aria-disabled-redundant">
    <rule context="*[@disabled]">
      <report id="aria-disabled-redundant.on-disabled" role="warning" test="@aria-disabled">
        Attribute "aria-disabled" is unnecessary for elements that have attribute "disabled".
      </report>
    </rule>
  </pattern>

  <pattern id="aria-expanded-restrictions">
    <rule context="*[@aria-expanded]">
      <assert id="aria-expanded-restrictions.not-with-command" role="error" test="not(@command)">
        The "aria-expanded" attribute must not be used on any element which has a "command" attribute.
      </assert>
      <assert id="aria-expanded-restrictions.not-with-popovertarget" role="error" test="not(@popovertarget)">
        The "aria-expanded" attribute must not be used on any element which has a "popovertarget" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-hidden-not-on-hidden-input">
    <rule context="h:input[@type = 'hidden']">
      <assert id="aria-hidden-restrictions.not-on-hidden-input" role="error" test="not(@aria-hidden)">
        The "aria-hidden" attribute must not be specified on an "input" element whose "type" attribute has the value "hidden".
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-hidden-not-true-with-hidden-until-found">
    <rule context="*[@hidden = 'until-found']">
      <assert id="aria-hidden-restrictions.not-true-with-hidden-until-found" role="error" test="not(@aria-hidden = 'true')">
        Attribute "aria-hidden" with value "true" must not be specified on elements with "hidden" attribute value "until-found".
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-valuemax-on-max">
    <rule context="*[@max]">
      <assert id="aria-valuemax-on-max.forbidden" role="error" test="not(@aria-valuemax)">
        The "aria-valuemax" attribute must not be used on an element which has a "max" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-valuemin-on-meter">
    <rule context="h:meter">
      <report id="aria-valuemin-on-meter.forbidden" role="warning" test="@aria-valuemin">
        The "aria-valuemin" attribute should not be used on a "meter" element.
      </report>
    </rule>
  </pattern>

  <pattern id="role-listitem-aria-level">
    <rule context="*[@role = 'listitem']">
      <report id="role-listitem-aria-level.forbidden" role="warning" test="@aria-level">
        The "aria-level" attribute should not be used on any element which has "role=listitem".
      </report>
    </rule>
  </pattern>

  <pattern id="dl-div-child-role">
    <rule context="h:dl/h:div[@role]">
      <assert id="dl-div-child-role.presentation-or-none" role="error" test="@role = 'presentation' or @role = 'none'">
        A "div" child of a "dl" element must not have any "role" value other than "presentation" or "none".
      </assert>
    </rule>
  </pattern>

  <!--
    "summary that is the summary for its parent details" = the first
    `summary` child of a `details` element (HTML5 tree-construction
    rule) — confirmed unconditionally forbidding @role regardless of
    its value (summary-with-role-novalid.html: role=button;
    summary-for-its-details-with-role-paragraph-novalid.html:
    role=paragraph — both flagged the same way).
  -->
  <pattern id="summary-role">
    <rule context="h:details/h:summary[not(preceding-sibling::h:summary)]">
      <assert id="summary-role.forbidden" role="error" test="not(@role)">
        The "role" attribute must not be used on any "summary" element that is a summary for its parent "details" element.
      </assert>
    </rule>
  </pattern>

  <!--
    HTML5 labelable elements: button, input (except type=hidden), meter,
    output, progress, select, textarea.
  -->
  <pattern id="label-role-restrictions">
    <rule context="h:label[@role or @for]">
      <let name="for-id" value="@for"/>
      <let name="wraps-labelable"
        value="boolean(descendant::*[self::h:button or (self::h:input and not(@type = 'hidden')) or self::h:meter or self::h:output or self::h:progress or self::h:select or self::h:textarea])"/>
      <let name="for-labelable"
        value="boolean($for-id != '' and //*[@id = $for-id][self::h:button or (self::h:input and not(@type = 'hidden')) or self::h:meter or self::h:output or self::h:progress or self::h:select or self::h:textarea])"/>
      <assert id="label-role-restrictions.role-forbidden" role="error"
        test="not(@role) or not($wraps-labelable or $for-labelable)">
        The "role" attribute must not be used on any "label" element that is an ancestor of, or associated with, a labelable element.
      </assert>
      <assert id="label-role-restrictions.aria-label-forbidden-when-for" role="error"
        test="not(@aria-label) or not($for-labelable)">
        The "aria-label" attribute must not be used on any "label" element that is associated with a labelable element.
      </assert>
    </rule>
  </pattern>

  <!--
    Containment constraints confirmed for three "requires a specific
    role ancestor" cases (html-aria/misc/role-*-without-*-ancestor-novalid.html).
    The message text itself says "contained in, *or owned by*" — ARIA's
    `aria-owns` lets an element outside the normal DOM-ancestor chain
    still count as contained, by having the actual container reference
    this element's `@id` in its own `aria-owns` (space-separated token
    list). `html-aria/misc/aria-owns-broken-idref-isvalid.html` confirms
    this for the option/listbox case (an ancestor-only check false-
    positives there) — applied consistently to all three rather than
    only where a counter-example happened to exist in the corpus.
  -->
  <pattern id="role-cell-containment">
    <rule context="*[@role = 'cell']">
      <let name="my-id" value="@id"/>
      <assert id="role-cell-containment.needs-row-ancestor" role="error"
        test="ancestor::*[@role = 'row'] or ($my-id != '' and //*[@role = 'row'][contains(concat(' ', @aria-owns, ' '), concat(' ', $my-id, ' '))])">
        An element with "role=cell" must be contained in, or owned by, an element with the "role" value "row".
      </assert>
    </rule>
  </pattern>

  <pattern id="role-option-containment">
    <rule context="*[@role = 'option']">
      <let name="my-id" value="@id"/>
      <assert id="role-option-containment.needs-listbox-ancestor" role="error"
        test="ancestor::*[@role = 'listbox'] or ($my-id != '' and //*[@role = 'listbox'][contains(concat(' ', @aria-owns, ' '), concat(' ', $my-id, ' '))])">
        An element with "role=option" must be contained in, or owned by, an element with the "role" value "listbox".
      </assert>
    </rule>
  </pattern>

  <pattern id="role-row-containment">
    <rule context="*[@role = 'row']">
      <let name="my-id" value="@id"/>
      <assert id="role-row-containment.needs-table-family-ancestor" role="error"
        test="ancestor::*[@role = 'treegrid' or @role = 'grid' or @role = 'rowgroup' or @role = 'table']
              or ($my-id != '' and //*[@role = 'treegrid' or @role = 'grid' or @role = 'rowgroup' or @role = 'table'][contains(concat(' ', @aria-owns, ' '), concat(' ', $my-id, ' '))])">
        An element with "role=row" must be contained in, or owned by, an element with the "role" value "treegrid", "grid", "rowgroup", or "table".
      </assert>
    </rule>
  </pattern>

  <!--
    li role restricted by the role of its nearest structural ancestor —
    confirmed for four ancestor-role families
    (html-aria/misc/li-role-button-with-role-*-ancestor-novalid.html).

    `role="none"`/`"presentation"` is always allowed in addition to the
    enumerated roles below, on every one of the four families: it's the
    general ARIA "opt out of implicit semantics" escape hatch, and
    `html-aria/misc/li-role-none-with-role-menu-ancestor-isvalid.html`
    confirms vnu accepts it for at least the menu/menubar family — kept
    consistent across all four rather than only adding it where a
    counter-example happened to exist in the corpus.
  -->
  <pattern id="li-role-by-ancestor-listbox-or-list">
    <rule context="h:li[@role and ancestor::*[@role = 'listbox' or @role = 'list']]">
      <assert id="li-role-by-ancestor.listbox-or-list" role="error"
        test="@role = 'group' or @role = 'option' or @role = 'none' or @role = 'presentation'">
        An "li" element that is a descendant of a "role=listbox" element or "role=list" element must not have any "role" value other than "group" or "option".
      </assert>
    </rule>
  </pattern>

  <pattern id="li-role-by-ancestor-menu-or-menubar">
    <rule context="h:li[@role and ancestor::*[@role = 'menu' or @role = 'menubar']]">
      <assert id="li-role-by-ancestor.menu-or-menubar" role="error"
        test="@role = 'group' or @role = 'menuitem' or @role = 'menuitemcheckbox' or @role = 'menuitemradio' or @role = 'separator' or @role = 'none' or @role = 'presentation'">
        An "li" element that is a descendant of a "role=menu" element or "role=menubar" element must not have any "role" value other than "group", "menuitem", "menuitemcheckbox", "menuitemradio", or "separator".
      </assert>
    </rule>
  </pattern>

  <pattern id="li-role-by-ancestor-tablist">
    <rule context="h:li[@role and ancestor::*[@role = 'tablist']]">
      <assert id="li-role-by-ancestor.tablist" role="error" test="@role = 'tab' or @role = 'none' or @role = 'presentation'">
        An "li" element that is a descendant of a "role=tablist" element must not have any "role" value other than "tab".
      </assert>
    </rule>
  </pattern>

  <pattern id="li-role-by-ancestor-tree">
    <rule context="h:li[@role and ancestor::*[@role = 'tree']]">
      <assert id="li-role-by-ancestor.tree" role="error" test="@role = 'treeitem' or @role = 'none' or @role = 'presentation'">
        An "li" element that is a descendant of a "role=tree" element must not have any "role" value other than "treeitem".
      </assert>
    </rule>
  </pattern>
</schema>
