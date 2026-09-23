<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: confirmed ARIA co-constraints beyond simple role-redundancy
  (rules/roles.sch) — each rule below was verified against concrete
  corpus fixtures (html-aria/author-requirements, presentation-role,
  presentational-children, roles-properties-global), not extrapolated
  from the wider WAI-ARIA spec, to avoid over-fitting.

  Uses named element tests (`h:img`, `h:tr`, `h:body`) — needs the `h:`
  namespace prefix. See rules/tables.sch's header comment /
  rules/README.md for why.

  Each concern gets its own `<pattern>` — see rules/elements.sch's
  header comment for why (first-matching-rule-per-pattern gotcha).
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>

  <pattern id="author-requirements-group-in-list">
    <rule context="*[@role = 'group']">
      <assert id="author-requirements.group-in-list" role="error" test="not(parent::*[@role = 'list'])">
        An element with "role=group" must not be a child of an element with "role=list".
      </assert>
    </rule>
  </pattern>

  <pattern id="author-requirements-group-in-menu">
    <rule context="*[@role = 'group' and ancestor::*[@role = 'menu' or @role = 'menubar']]">
      <assert id="author-requirements.group-in-menu" role="error"
        test="not(*[@role and not(@role = 'menuitem' or @role = 'menuitemcheckbox' or @role = 'menuitemradio')])">
        An element with "role=group" that is a descendant of an element with "role=menu" or "role=menubar" must contain only elements with "role=menuitem", "role=menuitemcheckbox", or "role=menuitemradio".
      </assert>
    </rule>
  </pattern>

  <pattern id="author-requirements-group-in-tree">
    <rule context="*[@role = 'group' and ancestor::*[@role = 'tree']]">
      <assert id="author-requirements.group-in-tree" role="error"
        test="not(*[@role and not(@role = 'treeitem')])">
        An element with "role=group" that is a descendant of an element with "role=tree" must contain only elements with "role=treeitem".
      </assert>
    </rule>
  </pattern>

  <pattern id="author-requirements-rowgroup-child">
    <rule context="*[@role = 'rowgroup']/*">
      <assert id="author-requirements.rowgroup-child-must-be-row" role="error" test="self::h:tr or @role = 'row'">
        An element that is a child of an element with "role=rowgroup" must have "role=row".
      </assert>
    </rule>
  </pattern>

  <pattern id="presentation-role-img-empty-alt">
    <rule context="h:img[(@role = 'presentation' or @role = 'none') and @alt = '']">
      <assert id="presentation-role.img-empty-alt-with-aria" role="error" test="not(@*[starts-with(name(), 'aria-')])">
        An "img" element with a "role" attribute must not have an "alt" attribute whose value is the empty string.
      </assert>
    </rule>
  </pattern>

  <!--
    An `img` with a `role` attribute (any value, including
    "none"/"presentation" — confirmed by
    html-aria/misc/img-role-no-alt-novalid.html, `<img role='none'
    src='1234.png'>`) must have an accessible name. Distinct from
    `presentation-role-img-empty-alt` above: this fires when there's no
    name source at all (missing/empty `alt` and no aria-label/
    aria-labelledby), not specifically an *empty* `alt` combined with a
    global aria-* attribute.
  -->
  <pattern id="img-role-needs-accessible-name">
    <rule context="h:img[@role]">
      <assert id="img-role-needs-accessible-name.has-name" role="error"
        test="normalize-space(@alt) != '' or @aria-label or @aria-labelledby">
        An "img" element with a "role" attribute must also have an accessible name (e.g., an "alt" attribute).
      </assert>
    </rule>
  </pattern>

  <!--
    An `img` with any `aria-*` attribute other than `aria-hidden` must
    have an accessible name — `aria-hidden` alone signals "decorative,
    excluded from the accessibility tree", any other aria-* attribute
    implies the image is meant to be exposed and therefore needs a
    name. Confirmed by
    html-aria/misc/img-aria-relevant-no-alt-novalid.html (`<img
    aria-relevant="all" src='1234.png'>`).
  -->
  <pattern id="img-aria-needs-accessible-name">
    <rule context="h:img[@*[starts-with(name(), 'aria-') and name() != 'aria-hidden']]">
      <assert id="img-aria-needs-accessible-name.has-name" role="error"
        test="normalize-space(@alt) != '' or @aria-label or @aria-labelledby">
        An "img" element with any "aria-*" attributes other than "aria-hidden" must also have an accessible name. (e.g., an "alt" attribute).
      </assert>
    </rule>
  </pattern>

  <pattern id="presentational-children-label">
    <rule context="h:label">
      <assert id="presentational-children.label-descendant" role="warning"
        test="not(ancestor::*[@role = 'separator' or @role = 'progressbar' or @role = 'img' or @role = 'slider' or @role = 'math'])">
        The element "label" should not appear as a descendant of an element with a role that only allows presentational children.
      </assert>
    </rule>
  </pattern>

  <pattern id="roles-properties-global-main">
    <rule context="*[@role = 'main']">
      <report id="roles-properties-global.disabled-on-main" role="warning" test="@aria-disabled">
        The "aria-disabled" attribute should not be used on any element which has "role=main".
      </report>
      <report id="roles-properties-global.haspopup-on-main" role="warning" test="@aria-haspopup">
        The "aria-haspopup" attribute should not be used on any element which has "role=main".
      </report>
      <report id="roles-properties-global.invalid-on-main" role="warning" test="@aria-invalid">
        The "aria-invalid" attribute should not be used on any element which has "role=main".
      </report>
    </rule>
  </pattern>

  <pattern id="roles-properties-global-body">
    <rule context="h:body">
      <assert id="roles-properties-global.aria-hidden-on-body" role="error" test="not(@aria-hidden = 'true')">
        "aria-hidden=true" must not be used on the "body" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-placeholder-with-placeholder">
    <rule context="*[@aria-placeholder and @placeholder]">
      <assert id="aria-placeholder.not-with-placeholder" role="error" test="not(@placeholder)">
        The "aria-placeholder" attribute must not be specified on elements that have a "placeholder" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-valuemin-with-min">
    <rule context="*[@aria-valuemin and @min]">
      <assert id="aria-valuemin.not-with-min" role="error" test="not(@min)">
        The "aria-valuemin" attribute must not be used on an element which has a "min" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-valuemax-with-max">
    <rule context="*[@aria-valuemax and @max]">
      <assert id="aria-valuemax.not-with-max" role="error" test="not(@max)">
        The "aria-valuemax" attribute must not be used on an element which has a "max" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-valuemin-valuemax-meter-progress-input">
    <rule context="h:meter">
      <report id="aria-valuemax.on-meter" role="warning" test="@aria-valuemax">
        The "aria-valuemax" attribute should not be used on a "meter" element.
      </report>
      <report id="aria-valuemin.on-meter" role="warning" test="@aria-valuemin">
        The "aria-valuemin" attribute should not be used on a "meter" element.
      </report>
    </rule>
    <rule context="h:progress">
      <report id="aria-valuemax.on-progress" role="warning" test="@aria-valuemax">
        The "aria-valuemax" attribute should not be used on a "progress" element.
      </report>
      <report id="aria-valuemin.on-progress" role="warning" test="@aria-valuemin">
        The "aria-valuemin" attribute should not be used on a "progress" element.
      </report>
    </rule>
    <rule context="h:input[@type = 'number']">
      <report id="aria-valuemax.on-input-number" role="warning" test="@aria-valuemax and not(@max)">
        The "aria-valuemax" attribute should not be used on an "input" element which has a "type" attribute whose value is "number".
      </report>
      <report id="aria-valuemin.on-input-number" role="warning" test="@aria-valuemin and not(@min)">
        The "aria-valuemin" attribute should not be used on an "input" element which has a "type" attribute whose value is "number".
      </report>
    </rule>
  </pattern>

  <pattern id="aria-checked-input-types">
    <rule context="h:input[@aria-checked and (@type = 'checkbox' or @type = 'radio') and (not(@role) or @role = @type)]">
      <assert id="aria-checked.on-implicit-checkbox-radio" role="error" test="false()">
        The "aria-checked" attribute must not be used on an "input" element which has a "type" attribute whose value is "checkbox".
      </assert>
    </rule>
    <rule context="h:input[@aria-checked and not(@type = 'checkbox' or @type = 'radio' or not(@type)) and not(@role = 'checkbox' or @role = 'switch' or @role = 'menuitemcheckbox' or @role = 'menuitemradio' or @role = 'option' or @role = 'radio' or @role = 'button')]">
      <assert id="aria-checked.on-input-type" role="error" test="false()">
        The "aria-checked" attribute must not be used on an "input" element which has a "type" attribute whose value is not "checkbox" or "radio".
      </assert>
    </rule>
  </pattern>

  <pattern id="checkbox-role-button-aria-pressed">
    <rule context="h:input[@type = 'checkbox' and @role = 'button']">
      <assert id="checkbox.role-button-needs-aria-pressed" role="error" test="@aria-pressed">
        An "input" element with a "type" attribute whose value is "checkbox" and with a "role" attribute whose value is "button" must have an "aria-pressed" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="presentational-children-button-img">
    <rule context="*[@role = 'button']//h:h1 | *[@role = 'button']//h:h2 | *[@role = 'button']//h:h3">
      <assert id="presentational.no-heading-in-button" role="error" test="false()">
        The element "h1" must not appear as a descendant of an element with the attribute "role=button".
      </assert>
    </rule>
    <rule context="*[@role = 'img']//h:button">
      <assert id="presentational.no-button-in-img" role="warning" test="false()">
        The element "button" should not appear as a descendant of an element with the attribute "role=img".
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-input-hidden-no-aria">
    <rule context="h:input[@type = 'hidden' and (@aria-hidden or @aria-expanded or @aria-label or @aria-labelledby or @aria-describedby or @aria-disabled or @aria-invalid or @aria-required or @aria-checked or @aria-pressed or @aria-selected or @aria-autocomplete or @aria-controls or @aria-owns or @aria-haspopup or @aria-live or @aria-atomic or @aria-busy or @aria-current or @aria-details or @aria-flowto or @aria-keyshortcuts or @aria-modal or @aria-multiline or @aria-multiselectable or @aria-orientation or @aria-placeholder or @aria-readonly or @aria-relevant or @aria-roledescription or @aria-sort or @aria-valuemax or @aria-valuemin or @aria-valuenow or @aria-valuetext)]">
      <assert id="aria.input-hidden-no-aria-attrs" role="error" test="false()">
        An "input" element with a "type" attribute whose value is "hidden" must not have any "aria-*" attributes.
      </assert>
    </rule>
  </pattern>
  <pattern id="aria-none-role-override">
    <rule context="*[@role = 'none' or @role = 'presentation'][@aria-label or @aria-labelledby or @aria-describedby or @aria-details or @aria-roledescription]">
      <report id="aria.none-role-global-aria-ignored" role="warning" test="true()">
        The "none" role does not affect elements that have global ARIA attributes.
      </report>
    </rule>
    <rule context="*[@role = 'none' or @role = 'presentation'][@tabindex]">
      <report id="aria.none-role-tabindex-ignored" role="warning" test="true()">
        The "none" role does not affect elements that have a "tabindex" attribute.
      </report>
    </rule>
  </pattern>

  <pattern id="aria-option-no-aria-selected">
    <rule context="h:option[@aria-selected]">
      <report id="aria.option-no-aria-selected" role="warning" test="true()">
        The "aria-selected" attribute should not be used on the "option" element.
      </report>
    </rule>
  </pattern>

  <pattern id="aria-label-hidden-labelable-descendant">
    <rule context="h:label[@aria-hidden][descendant::*[self::h:button or self::h:input[not(@type = 'hidden')] or self::h:meter or self::h:output or self::h:progress or self::h:select or self::h:textarea]]">
      <assert id="aria.label-hidden-labelable-descendant" role="error" test="false()">
        The "aria-hidden" attribute must not be used on any "label" element that is an ancestor of a labelable element.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-img-alt-role-none-presentation">
    <rule context="h:img[@alt and normalize-space(@alt) != ''][@role = 'none' or @role = 'presentation']">
      <assert id="aria.img-alt-role-none-presentation" role="error" test="false()">
        An "img" element with a non-empty "alt" attribute must not have a "role" attribute whose value is "none" or "presentation".
      </assert>
    </rule>
  </pattern>

  <!--
    An *active* tab needs a tabpanel it is actually wired to.

    A first attempt at this (removed again, see `plan/DECISIONS.md`'s
    "roles-tab-needs-tabpanel" entry) read the message text as "every
    `role=tab` needs a `role=tabpanel` somewhere in the document" and
    regressed 12 fixtures in `html-aria/roles-properties-supported*/`
    that use an isolated `role="tab"` to test unrelated ARIA property
    support. The real condition comes from vnu's `Assertions.java` (its
    `tabElementsActive`/`tabpanelElements` maps and the `tabElements:`
    labelled loop that reconciles them at end-of-document, read from
    `validator/validator`'s source rather than inferred), and is
    considerably narrower on both ends:

    - "active" means literally `aria-selected="true"` — that is the only
      condition under which a `role=tab` element is recorded at all, so
      `aria-selected="false"`/`"undefined"`/absent is never flagged. This
      alone is what the earlier attempt was missing.
    - "corresponding" means *wired up*, not merely co-present: either the
      tab's `aria-controls` is the `id` of a `role=tabpanel` element, or
      some `role=tabpanel` element's `aria-labelledby` is the tab's own
      `id`. A tabpanel that neither references nor is referenced by the
      tab does not satisfy it.

    Both `id`-keyed lookups are skipped when the id in question is absent
    (vnu keys its maps by the `id` attribute, and a `null` key can never
    equal a real `aria-controls`/`aria-labelledby` value), hence the
    `!= ''` guards. `role` is matched as a space-separated token list
    (vnu splits it), so `role="tablist"` correctly does not count as a
    tab.
  -->
  <pattern id="aria-active-tab-needs-tabpanel">
    <rule context="*[@aria-selected = 'true'][contains(concat(' ', normalize-space(@role), ' '), ' tab ')]">
      <let name="tab-id" value="string(@id)"/>
      <let name="controls" value="string(@aria-controls)"/>
      <assert id="aria.active-tab-needs-tabpanel" role="error"
        test="($controls != '' and //*[contains(concat(' ', normalize-space(@role), ' '), ' tabpanel ')][@id = $controls])
              or ($tab-id != '' and //*[contains(concat(' ', normalize-space(@role), ' '), ' tabpanel ')][@aria-labelledby = $tab-id])">
        Every active "role=tab" element must have a corresponding "role=tabpanel" element.
      </assert>
    </rule>
  </pattern>
</schema>
