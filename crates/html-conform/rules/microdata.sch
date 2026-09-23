<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: microdata attribute co-constraints
  (html/assertions/itemid-without-itemtype-*.html and siblings).

  No `<ns>` binding needed: microdata attributes apply to any element,
  so context uses the `*` wildcard — see rules/README.md.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <pattern id="microdata">
    <rule context="*[@itemid]">
      <assert id="microdata.itemid-without-itemtype" role="error" test="@itemscope and @itemtype">
        The itemid attribute must not be specified on elements that do not have both an itemscope attribute and an itemtype attribute.
      </assert>
    </rule>
    <rule context="*[@itemref]">
      <assert id="microdata.itemref-without-itemscope" role="error" test="@itemscope">
        The itemref attribute must not be specified on elements without an itemscope attribute.
      </assert>
    </rule>
    <rule context="*[@itemtype]">
      <assert id="microdata.itemtype-without-itemscope" role="error" test="@itemscope">
        The itemtype attribute must not be specified on elements without an itemscope attribute.
      </assert>
    </rule>
  </pattern>
</schema>
