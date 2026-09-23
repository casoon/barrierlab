<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: confirmed attribute-level co-constraints from
  `html/attributes` (and one from `html-aria/misc`, same "ID reference
  must exist" shape as `commandfor`).

  No `<ns>` binding needed for most rules (`*` wildcard element tests
  and attribute tests only) — see rules/README.md.

  Investigated but deliberately NOT implemented here:
  `html/attributes/lang/missing-lang-attribute-haswarn.html` ("Consider
  adding a lang attribute...") — the fixture's own comment warns this
  is tied to a hardcoded single-file exception in vnu's own TestRunner,
  and 799 other vendored corpus fixtures have no `lang=` attribute at
  all and are expected clean, confirming a general "html without lang"
  rule would be a large false-positive regression, not a fix.
  `rel-typo-*-hasinfo.html` ("Bad list of link-type keywords: Typo for
  X?") needs fuzzy/edit-distance string matching, not expressible in
  declarative XPath. `autofocus-multiple-novalid.html` needs the
  "nearest ancestor autofocus scoping root" concept (dialogs/popovers
  as scoping roots), which isn't safely expressible as a small,
  isolated XPath assertion without missing edge cases — deferred.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <!--
    The vendored schema's own comment on `common.attrs.lang`
    (rules/aria-html-restrictions.sch's sibling entry in
    plan/DECISIONS.md's `lang`/`xml:lang` bugfix has the full context)
    says this "sameness check" is deliberately left to Schematron, not
    the RELAX NG schema. Uses `@*[local-name()='xml:lang' and
    namespace-uri()='']` rather than `@xml:lang`: an unprefixed XPath
    name test never matches a colon-containing local name, and `xml:`
    as a *prefix* would resolve to the real XML namespace per XPath's
    always-bound `xml` prefix — but this crate's infoset deliberately
    keeps a literal `xml:lang` attribute on ordinary HTML elements as a
    single unsplit local name with no namespace (HTML5-parsing-spec
    behavior, see rules/aria-html-restrictions.sch and
    plan/DECISIONS.md), so it has to be matched that way here too.
  -->
  <pattern id="lang-xml-lang-sameness">
    <rule context="*[@*[local-name() = 'xml:lang' and namespace-uri() = '']]">
      <let name="xml-lang-value" value="@*[local-name() = 'xml:lang' and namespace-uri() = '']"/>
      <assert id="attributes.lang-xml-lang-sameness" role="error" test="@lang and @lang = $xml-lang-value">
        When the attribute "xml:lang" in no namespace is specified, the element must also have the attribute "lang" present with the same value.
      </assert>
    </rule>
  </pattern>

  <pattern id="headingoffset-range">
    <rule context="*[@headingoffset]">
      <assert id="headingoffset-range.zero-to-eight" role="error"
        test="number(@headingoffset) &gt;= 0 and number(@headingoffset) &lt;= 8">
        The value of the "headingoffset" attribute must be a number between "0" and "8".
      </assert>
    </rule>
  </pattern>

  <pattern id="commandfor-target-exists">
    <rule context="*[@commandfor]">
      <let name="target" value="@commandfor"/>
      <assert id="commandfor-target-exists.id-found" role="error" test="//*[@id = $target]">
        The value of the "commandfor" attribute must be the ID of an element in the same tree.
      </assert>
    </rule>
  </pattern>

  <pattern id="aria-activedescendant-target-exists">
    <rule context="*[@aria-activedescendant]">
      <let name="target" value="@aria-activedescendant"/>
      <assert id="aria-activedescendant-target-exists.id-found" role="error" test="//*[@id = $target]">
        The "aria-activedescendant" attribute must reference the ID of an element in this document.
      </assert>
    </rule>
  </pattern>

  <pattern id="form-target-exists">
    <rule context="*[@form]">
      <let name="form-id" value="@form"/>
      <assert id="attributes.form-ref-form" role="error" test="//h:form[@id = $form-id]">
        The "form" attribute must refer to a form element.
      </assert>
    </rule>
  </pattern>
</schema>
