<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 06 skeleton rule set — see rules/aria.sch's header comment.
  Small, representative subset (not the exhaustive obsolete-element
  list) — exhaustive coverage is Phase 08.

  Uses named element tests (`self::font` etc.) — needs the `h:`
  namespace prefix. See rules/tables.sch's header comment /
  rules/README.md for why.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="obsolete-elements">
    <rule context="*[self::h:font or self::h:center or self::h:marquee or self::h:blink]">
      <report id="obsolete-elements.deprecated" role="warning" test="true()">
        This element is obsolete and must not be used.
      </report>
    </rule>
  </pattern>
</schema>
