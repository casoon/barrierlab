<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: ID uniqueness (html/assertions/duplicate-id-*.html).

  No `<ns>` binding needed: context/test only use the `*` wildcard
  element test and attribute tests — see rules/README.md.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <pattern id="ids">
    <rule context="*[@id]">
      <let name="id" value="@id"/>
      <assert id="ids.duplicate" role="error" test="count(//*[@id = $id]) = 1">
        Duplicate ID.
      </assert>
    </rule>
  </pattern>
</schema>
