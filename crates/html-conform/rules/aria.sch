<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 06 skeleton rule set — proves the assertion-layer pipeline
  end to end with one known-good rule, not yet the real ARIA rule
  coverage. Reuses one of the Phase 02 dependency-spike's validated
  canary cases (see plan/02-dependency-spike.md and the old
  `spike_tests` module in git history) rather than authoring new,
  unverified ARIA logic here.

  Real ARIA rule coverage is Phase 08
  (plan/08-assertion-refinement-loop.md).

  The first rule below needs no `<ns>` binding (`*` wildcard element
  test plus attribute tests only), but the second (Phase 08's
  aria-multiselectable-on-select, html/warnings/aria-multiselectable-*)
  names the `select` element, so this file now binds the `h:` prefix
  — see rules/README.md.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="aria">
    <rule context="*[@aria-hidden='true']">
      <assert id="aria.hidden-not-focusable" role="error" test="not(@tabindex)">
        An element with aria-hidden="true" must not also have a tabindex attribute.
      </assert>
    </rule>
    <rule context="h:select">
      <report id="aria.multiselectable-on-select" role="warning" test="@aria-multiselectable">
        The aria-multiselectable attribute should not be used with the select element.
      </report>
    </rule>
  </pattern>
</schema>
