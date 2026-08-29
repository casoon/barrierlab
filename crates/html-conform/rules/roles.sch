<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: redundant explicit ARIA role warnings — an explicit role
  that matches the element's own implicit ARIA role adds nothing
  (html/warnings/unnecessary-role-*.html).

  "banner" (header) and "contentinfo" (footer) are only implicit when
  the element is not a sectioning-content descendant (article, aside,
  main, nav, section) per the HTML-AAM element/role mapping — outside
  that context header/footer have no implicit role, so an explicit
  role there is not redundant and must not be flagged.

  Each pairing gets its own `<rule>` with a non-overlapping,
  element-specific context, so there is no risk of Schematron's
  first-matching-rule-per-pattern behavior silently suppressing one
  check in favor of another (see rules/README.md's `h:` prefix note
  for the related namespace gotcha).
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="roles">
    <rule context="h:article[@role='article']">
      <report id="roles.unnecessary-article" role="warning" test="true()">
        The "article" role is unnecessary for element "article".
      </report>
    </rule>
    <rule context="h:header[@role='banner' and not(ancestor::h:article or ancestor::h:aside or ancestor::h:main or ancestor::h:nav or ancestor::h:section)]">
      <report id="roles.unnecessary-banner" role="warning" test="true()">
        The "banner" role is unnecessary for element "header".
      </report>
    </rule>
    <rule context="h:aside[@role='complementary']">
      <report id="roles.unnecessary-complementary" role="warning" test="true()">
        The "complementary" role is unnecessary for element "aside".
      </report>
    </rule>
    <rule context="h:footer[@role='contentinfo' and not(ancestor::h:article or ancestor::h:aside or ancestor::h:main or ancestor::h:nav or ancestor::h:section)]">
      <report id="roles.unnecessary-contentinfo" role="warning" test="true()">
        The "contentinfo" role is unnecessary for element "footer".
      </report>
    </rule>
    <rule context="h:dd[@role='definition']">
      <report id="roles.unnecessary-definition" role="warning" test="true()">
        The "definition" role is unnecessary for element "dd".
      </report>
    </rule>
    <rule context="h:details[@role='group']">
      <report id="roles.unnecessary-group" role="warning" test="true()">
        The "group" role is unnecessary for element "details".
      </report>
    </rule>
    <rule context="h:img[@role='img']">
      <report id="roles.unnecessary-img" role="warning" test="true()">
        The "img" role is unnecessary for element "img".
      </report>
    </rule>
    <rule context="*[(self::h:ul or self::h:ol) and @role='list']">
      <report id="roles.unnecessary-list" role="warning" test="true()">
        The "list" role is unnecessary for this element.
      </report>
    </rule>
    <rule context="h:main[@role='main']">
      <report id="roles.unnecessary-main" role="warning" test="true()">
        The "main" role is unnecessary for element "main".
      </report>
    </rule>
    <rule context="h:nav[@role='navigation']">
      <report id="roles.unnecessary-navigation" role="warning" test="true()">
        The "navigation" role is unnecessary for element "nav".
      </report>
    </rule>
    <rule context="h:progress[@role='progressbar']">
      <report id="roles.unnecessary-progressbar" role="warning" test="true()">
        The "progressbar" role is unnecessary for element "progress".
      </report>
    </rule>
    <rule context="h:section[@role='region']">
      <report id="roles.unnecessary-region" role="warning" test="true()">
        The "region" role is unnecessary for element "section".
      </report>
    </rule>
    <rule context="h:hr[@role='separator']">
      <report id="roles.unnecessary-separator" role="warning" test="true()">
        The "separator" role is unnecessary for element "hr".
      </report>
    </rule>
    <rule context="h:output[@role='status']">
      <report id="roles.unnecessary-status" role="warning" test="true()">
        The "status" role is unnecessary for element "output".
      </report>
    </rule>
    <rule context="h:table[@role='table']">
      <report id="roles.unnecessary-table" role="warning" test="true()">
        The "table" role is unnecessary for element "table".
      </report>
    </rule>
    <rule context="h:dt[@role='term']">
      <report id="roles.unnecessary-term" role="warning" test="true()">
        The "term" role is unnecessary for element "dt".
      </report>
    </rule>
    <rule context="h:form[@role='form']">
      <report id="roles.unnecessary-form" role="warning" test="true()">
        The "form" role is unnecessary for element "form".
      </report>
    </rule>
    <rule context="h:input[@role='spinbutton' and @type='number']">
      <report id="roles.unnecessary-spinbutton" role="warning" test="true()">
        The "spinbutton" role is unnecessary for element "input" whose type is "number".
      </report>
    </rule>
    <rule context="h:input[@role='textbox' and (not(@type) or @type='text') and not(@list)]">
      <report id="roles.unnecessary-textbox" role="warning" test="true()">
        The "textbox" role is unnecessary for an "input" element that has no "list" attribute and whose type is "text".
      </report>
    </rule>
    <rule context="h:input[@role='searchbox' and @type='search' and not(@list)]">
      <report id="roles.unnecessary-searchbox" role="warning" test="true()">
        The "searchbox" role is unnecessary for an "input" element that has no "list" attribute and whose type is "search".
      </report>
    </rule>
    <rule context="h:select[@role='combobox' and not(@multiple) and not(number(@size) &gt; 1)]">
      <report id="roles.unnecessary-combobox" role="warning" test="true()">
        The "combobox" role is unnecessary for element "select" without a "multiple" attribute and without a "size" attribute whose value is greater than 1.
      </report>
    </rule>
    <rule context="h:select[@role='listbox' and (@multiple or number(@size) &gt; 1)]">
      <report id="roles.unnecessary-listbox" role="warning" test="true()">
        The "listbox" role is unnecessary for element "select" with a "multiple" attribute or with a "size" attribute whose value is greater than 1.
      </report>
    </rule>
    <rule context="h:li[@role='listitem']">
      <report id="roles.unnecessary-listitem" role="warning" test="true()">
        The "listitem" role is unnecessary for element "li".
      </report>
    </rule>
    <rule context="h:tbody[@role='rowgroup']">
      <report id="roles.unnecessary-rowgroup" role="warning" test="true()">
        The "rowgroup" role is unnecessary for element "tbody".
      </report>
    </rule>
    <rule context="h:button[@role='button']">
      <report id="roles.unnecessary-button" role="warning" test="true()">
        The "button" role is unnecessary for element "button".
      </report>
    </rule>
    <rule context="h:dialog[@role='dialog']">
      <report id="roles.unnecessary-dialog" role="warning" test="true()">
        The "dialog" role is unnecessary for element "dialog".
      </report>
    </rule>
    <rule context="h:figure[@role='figure']">
      <report id="roles.unnecessary-figure" role="warning" test="true()">
        The "figure" role is unnecessary for element "figure".
      </report>
    </rule>
    <rule context="h:s[@role='deletion']">
      <report id="roles.unnecessary-deletion" role="warning" test="true()">
        The "deletion" role is unnecessary for element "s".
      </report>
    </rule>
    <rule context="h:a[@href and @role='link']">
      <report id="roles.unnecessary-link" role="warning" test="true()">
        The "link" role is unnecessary for element "a" with attribute "href".
      </report>
    </rule>
    <rule context="*[local-name()='math'][@role]">
      <report id="roles.unnecessary-math" role="warning" test="true()">
        Element "math" does not need a "role" attribute.
      </report>
    </rule>
  </pattern>

  <!--
    Deliberately NOT implemented as "any role=tab needs a role=tabpanel
    anywhere in the document" — that heuristic regressed 12 corpus
    fixtures (role-support/aria-property-support test cases that use a
    bare role="tab" in isolation, with no tabpanel in the document at
    all, to test something unrelated like aria-expanded/aria-selected
    support). vnu's real condition is narrower (likely: an *active*,
    i.e. aria-selected="true", tab must resolve its aria-controls
    reference to an existing role=tabpanel element) but no corpus
    fixture pins down aria-controls resolution specifically enough to
    implement that with confidence — left open, see
    html-aria/misc/role-tab-with-no-role-tabpanel-novalid.html in
    plan/00-STATUS.md's remaining backlog.
  -->
</schema>
