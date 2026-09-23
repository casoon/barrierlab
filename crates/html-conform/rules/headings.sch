<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: empty-heading warning (html/warnings/h2-empty-*.html and
  siblings for h3..h6).

  Uses named element tests — needs the `h:` namespace prefix. See
  rules/tables.sch's header comment / rules/README.md for why.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="headings">
    <rule context="*[self::h:h1 or self::h:h2 or self::h:h3 or self::h:h4 or self::h:h5 or self::h:h6]">
      <report id="headings.empty" role="warning" test="normalize-space(.) = '' and not(*)">
        Empty heading.
      </report>
    </rule>
  </pattern>

  <pattern id="headings-article">
    <rule context="h:article">
      <report id="headings.article-lacks-heading" role="warning"
        test="not(descendant::*[self::h:h1 or self::h:h2 or self::h:h3 or self::h:h4 or self::h:h5 or self::h:h6])">
        Article lacks heading. Consider using "h2"-"h6" elements to add identifying headings to all articles, or else use a "div" element instead for any cases where no heading is needed.
      </report>
    </rule>
  </pattern>

  <pattern id="headings-section">
    <rule context="h:section">
      <report id="headings.section-lacks-heading" role="warning"
        test="not(descendant::*[self::h:h1 or self::h:h2 or self::h:h3 or self::h:h4 or self::h:h5 or self::h:h6])">
        Section lacks heading. Consider using "h2"-"h6" elements to add identifying headings to all sections, or else use a "div" element instead for any cases where no heading is needed.
      </report>
    </rule>
  </pattern>
  <pattern id="headings-no-top-level">
    <rule context="h:body[descendant::*[self::h:h2 or self::h:h3 or self::h:h4 or self::h:h5 or self::h:h6] and not(descendant::h:h1)]">
      <report id="headings.no-top-level" role="warning" test="true()">
        This document has heading elements but none of them has a computed heading level of 1.
      </report>
    </rule>
  </pattern>

  <pattern id="headings-skip-level">
    <rule context="h:h3[preceding::h:h1 and not(preceding::h:h2)]">
      <report id="headings.skip-level-h1-to-h3" role="error" test="true()">
        The heading "h3" (with computed level 3) follows the heading "h1" (with computed level 1), skipping 1 heading level.
      </report>
    </rule>
  </pattern>
</schema>
