<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 08: assorted per-element structural co-constraints
  (html/assertions/*.html) that don't fit an existing themed file.

  Uses named element tests — needs the `h:` namespace prefix. See
  rules/tables.sch's header comment / rules/README.md for why.

  Each independent concern below gets its own `<pattern>`, even where
  a single element name shows up in more than one (e.g. `link`,
  `script`): Schematron only evaluates the *first* matching `<rule>`
  per pattern, so two rules with overlapping contexts in the same
  pattern would silently suppress one check. Separate patterns are
  always independently evaluated — see rules/README.md.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>

  <pattern id="elements-bdo">
    <rule context="h:bdo">
      <assert id="elements.bdo-missing-dir" role="error" test="@dir">
        Element "bdo" must have attribute "dir".
      </assert>
      <assert id="elements.bdo-dir-auto" role="error" test="not(@dir = 'auto')">
        The value of the "dir" attribute for the "bdo" element must not be "auto".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-area">
    <rule context="h:area">
      <assert id="elements.area-outside-map" role="error" test="ancestor::h:map">
        An "area" element must have a "map" ancestor.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-main-ancestor">
    <rule context="h:main">
      <assert id="elements.main-in-wrong-ancestor" role="error"
        test="not(ancestor::h:article or ancestor::h:aside or ancestor::h:footer or ancestor::h:header or ancestor::h:nav)">
        The "main" element must not appear as a descendant of an "article", "aside", "footer", "header", or "nav" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-main-visible">
    <rule context="h:html">
      <assert id="elements.multiple-main-visible" role="error" test="count(descendant::h:main[not(@hidden)]) &lt;= 1">
        A document must not include more than one visible "main" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-map">
    <rule context="h:map[@id and @name]">
      <assert id="elements.map-id-name-mismatch" role="error" test="@id = @name">
        The "id" attribute on a "map" element must have the same value as the "name" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-href">
    <rule context="h:link">
      <assert id="elements.link-missing-href" role="error" test="@href or @imagesrcset">
        A "link" element must have an "href" attribute or an "imagesrcset" attribute, or both.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-preload-as">
    <rule context="h:link[contains(concat(' ', @rel, ' '), ' preload ')]">
      <assert id="elements.link-preload-missing-as" role="error" test="@as">
        A "link" element with a "rel" attribute that contains the value "preload" must have an "as" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-importmap">
    <rule context="h:script[@type = 'importmap']">
      <assert id="elements.script-importmap-async" role="error" test="not(@async)">
        A "script" element with type "importmap" must not have an "async" attribute.
      </assert>
      <assert id="elements.script-importmap-defer" role="error" test="not(@defer)">
        A "script" element with type "importmap" must not have a "defer" attribute.
      </assert>
      <assert id="elements.script-importmap-src" role="error" test="not(@src)">
        A "script" element with type "importmap" must not have a "src" attribute.
      </assert>
      <assert id="elements.script-importmap-integrity" role="error" test="not(@integrity)">
        A "script" element with "type=importmap" must not have an "integrity" attribute.
      </assert>
    </rule>
  </pattern>

  <!--
    "speculationrules" forbids "integrity" unconditionally, same as
    "importmap" above — confirmed against vnu's own
    `Assertions.java` (`isSpeculationRules` branch): unlike "module"/
    plain classic scripts below, there is no `not(@src)`/inline-only
    guard for either of these two types.
  -->
  <pattern id="elements-script-speculationrules">
    <rule context="h:script[@type = 'speculationrules']">
      <assert id="elements.script-speculationrules-integrity" role="error" test="not(@integrity)">
        A "script" element with "type=speculationrules" must not have an "integrity" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-module">
    <rule context="h:script[@type = 'module']">
      <assert id="elements.script-module-defer" role="error" test="not(@defer)">
        A "script" element with type "module" must not have a "defer" attribute.
      </assert>
      <assert id="elements.script-module-inline-integrity" role="error" test="not(@integrity) or @src">
        An inline "script" element with "type=module" must not have an "integrity" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-nomodule">
    <rule context="h:script[@nomodule]">
      <assert id="elements.script-module-nomodule" role="error" test="not(@type = 'module')">
        A "script" element with a "nomodule" attribute must not have a "type" attribute with the value "module".
      </assert>
    </rule>
  </pattern>

  <!--
    "Classic script" per vnu's own `Assertions.java`: a `type` attribute
    that is either absent, empty, or (case-insensitively) one of the 16
    JavaScript MIME types `src/datatypes/structural.rs::check_script_type`
    already validates against (`JAVASCRIPT_MIME_TYPES` there — the same
    16, same order, confirmed against vnu's own list). `integrity` is only
    forbidden on the *inline* form (no "src") — an external classic
    script with "integrity" is exactly `integrity`'s normal use case.
  -->
  <pattern id="elements-script-classic-inline-integrity">
    <rule context="h:script[@integrity][not(@src)]">
      <let name="type-lower" value="translate(@type, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')"/>
      <assert id="elements.script-classic-inline-integrity" role="error"
        test="@type and $type-lower != ''
              and not(
                $type-lower = 'application/ecmascript' or $type-lower = 'application/javascript'
                or $type-lower = 'application/x-ecmascript' or $type-lower = 'application/x-javascript'
                or $type-lower = 'text/ecmascript' or $type-lower = 'text/javascript'
                or $type-lower = 'text/javascript1.0' or $type-lower = 'text/javascript1.1'
                or $type-lower = 'text/javascript1.2' or $type-lower = 'text/javascript1.3'
                or $type-lower = 'text/javascript1.4' or $type-lower = 'text/javascript1.5'
                or $type-lower = 'text/jscript' or $type-lower = 'text/livescript'
                or $type-lower = 'text/x-ecmascript' or $type-lower = 'text/x-javascript'
              )">
        An inline classic "script" element (i.e., a "script" element without a "src" attribute and with a "type" attribute that is either unspecified, empty, or a JavaScript MIME type) must not have an "integrity" attribute.
      </assert>
    </rule>
  </pattern>

  <!--
    "Data block" per vnu: a non-empty `type` that is neither a
    JavaScript MIME type, "module", "importmap", nor "speculationrules" —
    i.e. everything the three patterns above and the classic-script
    pattern don't already claim. `integrity` is forbidden unconditionally
    here too (no "src" guard — confirmed against vnu's `isDataBlock`
    branch, same as importmap/speculationrules).
  -->
  <pattern id="elements-script-datablock-integrity">
    <rule context="h:script[@integrity][@type]">
      <let name="type-lower" value="translate(@type, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')"/>
      <assert id="elements.script-datablock-integrity" role="error"
        test="$type-lower = ''
              or $type-lower = 'module' or $type-lower = 'importmap' or $type-lower = 'speculationrules'
              or $type-lower = 'application/ecmascript' or $type-lower = 'application/javascript'
              or $type-lower = 'application/x-ecmascript' or $type-lower = 'application/x-javascript'
              or $type-lower = 'text/ecmascript' or $type-lower = 'text/javascript'
              or $type-lower = 'text/javascript1.0' or $type-lower = 'text/javascript1.1'
              or $type-lower = 'text/javascript1.2' or $type-lower = 'text/javascript1.3'
              or $type-lower = 'text/javascript1.4' or $type-lower = 'text/javascript1.5'
              or $type-lower = 'text/jscript' or $type-lower = 'text/livescript'
              or $type-lower = 'text/x-ecmascript' or $type-lower = 'text/x-javascript'">
        A "script" element with a "type" attribute whose value is neither a JavaScript MIME type, "module", "importmap", nor "speculationrules" (i.e., a data block) must not have an "integrity" attribute.
      </assert>
    </rule>
  </pattern>

  <!--
    Confirmed against vnu's own `Assertions.java` (the "integrity"
    check right next to its "preload"/"modulepreload" "as"-value
    checks above): fires whenever "integrity" is present and "rel"
    either doesn't contain "stylesheet"/"preload"/"modulepreload", or
    is absent entirely — `not(@rel)` covers the "absent" half, since
    the plain token-list `contains()` test alone would otherwise treat
    a missing "rel" as vacuously not-containing-anything (already
    false, so this is actually redundant with the token check below,
    kept for the "no @rel at all" case to read explicitly rather than
    relying on that fallthrough).
  -->
  <pattern id="elements-link-integrity">
    <rule context="h:link[@integrity]">
      <assert id="elements.link-integrity-rel" role="error"
        test="@rel and (
                contains(concat(' ', @rel, ' '), ' stylesheet ')
                or contains(concat(' ', @rel, ' '), ' preload ')
                or contains(concat(' ', @rel, ' '), ' modulepreload ')
              )">
        A "link" element with an "integrity" attribute must have a "rel" attribute that contains the value "stylesheet" or the value "preload" or the value "modulepreload".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-select">
    <rule context="h:select[not(@multiple)]">
      <assert id="elements.select-multiple-selected" role="error" test="count(descendant::h:option[@selected]) &lt;= 1">
        A "select" element without a "multiple" attribute must not have more than one selected "option" descendant.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-track">
    <rule context="h:track[@label]">
      <assert id="elements.track-empty-label" role="error" test="normalize-space(@label) != ''">
        Attribute "label" for element "track" must have a non-empty value.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-button">
    <rule context="h:input[@type = 'button']">
      <assert id="elements.input-button-empty-value" role="error" test="@value and normalize-space(@value) != ''">
        An "input" element with type "button" must have a non-empty "value" attribute.
      </assert>
    </rule>
  </pattern>

  <!--
    `schema/html5/web-forms.rnc`'s `selectedcontent.attrs` includes
    `common.attrs.aria?` — the RELAX NG schema alone allows any ARIA
    attribute on `selectedcontent`, this restriction is Schematron-only
    in vnu too. Confirmed against `html/elements/selectedcontent/
    aria-hidden-in-select-novalid.html`/`role-in-select-novalid.html`.
  -->
  <pattern id="elements-selectedcontent-in-customizable-select">
    <rule context="h:selectedcontent[ancestor::h:button][ancestor::h:select]">
      <assert id="elements.selectedcontent-no-aria-hidden" role="error" test="not(@aria-hidden)">
        The "aria-hidden" attribute must not be used on a "selectedcontent" element inside the "button" part of a customizable "select" element.
      </assert>
      <assert id="elements.selectedcontent-no-role" role="error" test="not(@role)">
        The "role" attribute must not be used on a "selectedcontent" element inside the "button" part of a customizable "select" element.
      </assert>
    </rule>
  </pattern>

  <!--
    "Nearest ancestor autofocus scoping root", ported from vnu's own
    `Assertions.java`: `<dialog>` and any `[popover]` element (any
    value, even an invalid one — `atts.getIndex("", "popover") > -1`
    there is presence-only) each open a fresh scope, and at most one
    `[autofocus]` element may appear per scope.

    `ancestor::*[self::h:dialog or @popover][1]` is the *nearest* such
    ancestor — `ancestor::` is a reverse axis (nearest-first, see
    `xpath-eval/src/axes.rs`), and a `[1]` predicate chained directly
    onto an axis step picks proximity position 1 *along that axis's own
    order*, i.e. the nearest match. Parenthesizing first —
    `(ancestor::*[...])[1]`, which an earlier version of this rule used
    for the `<let>` binding — changes this: a parenthesized sub-expression
    followed by `[1]` is a `FilterExpr`, and `evaluate_filter_expr`
    (`xpath-eval/src/eval.rs`) re-sorts into plain forward document order
    before applying the predicate, per its own comment citing XPath 1.0
    §2.4 — spec-correct, but it means `[1]` there silently picks the
    *outermost/root-most* match instead. This is a well-known, easy-to-hit
    XPath 1.0 gotcha, not a bug in this engine: confirmed by direct
    `xpath_eval::evaluate` calls against a two-level-nested-`dialog`
    fixture during development, only caught because
    `nested-dialogs-isvalid.html` below (unlike the two-sibling-scope
    cases) exercises a scoping root nested inside another one. Every
    `ancestor::`/`preceding::`/`preceding-sibling::` step anywhere in this
    file relying on reverse-axis order must avoid the parenthesized form
    for the same reason.

    Neither `generate-id()` nor the XSLT-originated (not core XPath 1.0)
    `current()` exist in this engine (`../xpath-eval/src/functions.rs`
    implements XPath 1.0's core function library, nothing beyond it).
    Node identity is compared the classic XPath 1.0 way instead:
    `count(a | b) = 1` iff `a` and `b` are the same single node (union
    dedups by identity); `$me` (a `<let>` bound to `.` at the rule's own
    context, unaffected by a nested predicate's own context node) stands
    in for `current()`.

    Confirmed against `html/attributes/autofocus-multiple-novalid.html`
    (two `[autofocus]` in one `dialog`), `-popover-novalid.html` (two in
    one `[popover]`), and the "must NOT fire" cases `html/attributes/
    autofocus-isvalid.html` (matching sibling scopes),
    `html/elements/autofocus/dialog-scoped-isvalid.html`/
    `popover-scoped-isvalid.html` (top-level vs. nested scope), and
    `nested-dialogs-isvalid.html` (three independent nested scopes,
    including two-scoping-roots-deep — exactly the shape that catches
    the parenthesization gotcha above).
  -->
  <pattern id="elements-autofocus-multiple">
    <rule context="*[@autofocus]">
      <let name="me" value="."/>
      <let name="my-root" value="ancestor::*[self::h:dialog or @popover][1]"/>
      <assert id="elements.autofocus-multiple" role="error"
        test="count(
                //*[@autofocus][count(. | $me) = 2]
                  [
                    (count($my-root) = 1
                     and count(ancestor::*[self::h:dialog or @popover][1]) = 1
                     and count($my-root | ancestor::*[self::h:dialog or @popover][1]) = 1)
                    or
                    (count($my-root) = 0
                     and count(ancestor::*[self::h:dialog or @popover][1]) = 0)
                  ]
              ) = 0">
        There must not be two elements with the same "nearest ancestor autofocus scoping root element" that both have the "autofocus" attribute specified.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-meter-range">
    <rule context="h:meter">
      <assert id="elements.meter-min-max" role="error"
        test="not(@min and @max) or number(@min) &lt;= number(@max)">
        The value of the "min" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
      <assert id="elements.meter-value-min" role="error"
        test="not(@min and @value) or number(@min) &lt;= number(@value)">
        The value of the "min" attribute must be less than or equal to the value of the "value" attribute.
      </assert>
      <assert id="elements.meter-value-no-min" role="error"
        test="not(not(@min) and @value) or number(@value) &gt;= 0">
        The value of the "value" attribute must be greater than or equal to zero when the "min" attribute is absent.
      </assert>
      <assert id="elements.meter-value-max" role="error"
        test="not(@max and @value) or number(@value) &lt;= number(@max)">
        The value of the "value" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
      <assert id="elements.meter-value-no-max" role="error"
        test="not(not(@max) and @value) or number(@value) &lt;= 1">
        The value of the "value" attribute must be less than or equal to one when the "max" attribute is absent.
      </assert>
      <assert id="elements.meter-low-min" role="error"
        test="not(@min and @low) or number(@min) &lt;= number(@low)">
        The value of the "min" attribute must be less than or equal to the value of the "low" attribute.
      </assert>
      <assert id="elements.meter-low-high" role="error"
        test="not(@low and @high) or number(@low) &lt;= number(@high)">
        The value of the "low" attribute must be less than or equal to the value of the "high" attribute.
      </assert>
      <assert id="elements.meter-low-max" role="error"
        test="not(@low and @max) or number(@low) &lt;= number(@max)">
        The value of the "low" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
      <assert id="elements.meter-high-max" role="error"
        test="not(@high and @max) or number(@high) &lt;= number(@max)">
        The value of the "high" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
      <assert id="elements.meter-high-min" role="error"
        test="not(@high and @min) or number(@min) &lt;= number(@high)">
        The value of the "min" attribute must be less than or equal to the value of the "high" attribute.
      </assert>
      <assert id="elements.meter-optimum-min" role="error"
        test="not(@optimum and @min) or number(@min) &lt;= number(@optimum)">
        The value of the "min" attribute must be less than or equal to the value of the "optimum" attribute.
      </assert>
      <assert id="elements.meter-optimum-max" role="error"
        test="not(@optimum and @max) or number(@optimum) &lt;= number(@max)">
        The value of the "optimum" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-progress-range">
    <rule context="h:progress">
      <assert id="elements.progress-value-max" role="error"
        test="not(@max and @value) or number(@value) &lt;= number(@max)">
        The value of the "value" attribute must be less than or equal to the value of the "max" attribute.
      </assert>
      <assert id="elements.progress-value-no-max" role="error"
        test="not(not(@max) and @value) or number(@value) &lt;= 1">
        The value of the "value" attribute must be less than or equal to one when the "max" attribute is absent.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-readonly">
    <rule context="h:input[@readonly]">
      <assert id="elements.input-readonly-type" role="error"
        test="not(@type) or @type = 'date' or @type = 'datetime-local' or @type = 'email' or @type = 'month' or @type = 'number' or @type = 'password' or @type = 'search' or @type = 'tel' or @type = 'text' or @type = 'time' or @type = 'url' or @type = 'week'">
        Attribute "readonly" is only allowed when the input type is "date", "datetime-local", "email", "month", "number", "password", "search", "tel", "text", "time", "url", or "week".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-maxlength">
    <rule context="h:input[@maxlength]">
      <assert id="elements.input-maxlength-type" role="error"
        test="not(@type) or @type = 'email' or @type = 'password' or @type = 'search' or @type = 'tel' or @type = 'text' or @type = 'url'">
        Attribute "maxlength" is only allowed when the input type is "email", "password", "search", "tel", "text", or "url".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-list">
    <rule context="h:input[@list]">
      <assert id="elements.input-list-ref" role="error" test="id(@list)[self::h:datalist]">
        The "list" attribute of the "input" element must refer to a "datalist" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-as-preload">
    <rule context="h:link[@as]">
      <assert id="elements.link-as-missing-rel" role="error"
        test="@rel and (contains(concat(' ', normalize-space(@rel), ' '), ' preload ') or contains(concat(' ', normalize-space(@rel), ' '), ' modulepreload '))">
        A "link" element with an "as" attribute must have a "rel" attribute that contains the value "preload" or "modulepreload".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-imagesrcset">
    <rule context="h:link[@imagesrcset]">
      <assert id="elements.link-imagesrcset-as" role="error" test="@as = 'image'">
        A "link" element with an "imagesrcset" attribute must have an "as" attribute with value "image".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-label-descendants">
    <rule context="h:label">
      <assert id="elements.label-max-one-labelable" role="error"
        test="count(descendant::*[self::h:button or self::h:input or self::h:meter or self::h:output or self::h:progress or self::h:select or self::h:textarea]) &lt;= 1">
        The "label" element may contain at most one "button", "input", "meter", "output", "progress", "select", or "textarea" descendant.
      </assert>
    </rule>
  </pattern>

  <!--
    This used to also match `h:source[@srcset and not(@sizes)]` via a
    context union, but that alternative never actually fired (see
    `elements-source-srcset-w-needs-sizes` below and its own fix history)
    until a real schematron-engine bug (only the first alternative of a
    `context` union ever matched) got fixed upstream — at which point it
    started firing here too, but *without* the sibling
    `not(../h:img[@loading='lazy'])` exception that
    `elements-source-srcset-w-needs-sizes` already has, causing a false
    positive on `source-srcset-width-loading-lazy-no-sizes-isvalid.html`.
    Dropped here in favor of that already-correct, lazy-aware rule —
    keeping only the `img` alternative, which is unrelated (applies to any
    `img[srcset]`, not just ones inside `picture`).
  -->
  <pattern id="elements-srcset-width-descriptor">
    <rule context="h:img[@srcset and not(@sizes) and not(@loading = 'lazy')]">
      <assert id="elements.srcset-w-needs-sizes" role="error"
        test="not(contains(@srcset, '0w') or contains(@srcset, '1w') or contains(@srcset, '2w') or contains(@srcset, '3w') or contains(@srcset, '4w') or contains(@srcset, '5w') or contains(@srcset, '6w') or contains(@srcset, '7w') or contains(@srcset, '8w') or contains(@srcset, '9w') or contains(@srcset, '0W') or contains(@srcset, '1W') or contains(@srcset, '2W') or contains(@srcset, '3W') or contains(@srcset, '4W') or contains(@srcset, '5W') or contains(@srcset, '6W') or contains(@srcset, '7W') or contains(@srcset, '8W') or contains(@srcset, '9W'))">
        When the "srcset" attribute has any image candidate string with a width descriptor, the "sizes" attribute must also be specified.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-source-media-all">
    <rule context="h:picture/h:source[@media]">
      <assert id="elements.source-media-not-all" role="error"
        test="translate(normalize-space(@media), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz') != 'all'">
        Value of "media" attribute here must not be "all".
      </assert>
    </rule>
  </pattern>

  <!--
    "Does @srcset have a width descriptor" is approximated as "contains a
    digit immediately followed by w/W" rather than a bare
    contains(@srcset, 'w') — a candidate URL can itself contain a literal
    "w" for unrelated reasons (e.g. "image.webp"), which a bare
    contains() check false-negatives on (source/sizes-without-width-
    descriptor-novalid.html: "image.webp 1x, image2.webp 2x" — no width
    descriptor at all, but contains(@srcset, 'w') is still true because
    of "webp"). No digit ever directly precedes "w"/"W" inside a URL
    candidate string by itself, only in an actual "<n>w" descriptor.
  -->
  <pattern id="elements-srcset-x-with-sizes-invalid">
    <rule context="h:source[@sizes and @srcset] | h:img[@sizes and @srcset]">
      <let name="has-width-descriptor"
        value="contains(@srcset, '0w') or contains(@srcset, '1w') or contains(@srcset, '2w') or contains(@srcset, '3w') or contains(@srcset, '4w') or contains(@srcset, '5w') or contains(@srcset, '6w') or contains(@srcset, '7w') or contains(@srcset, '8w') or contains(@srcset, '9w') or contains(@srcset, '0W') or contains(@srcset, '1W') or contains(@srcset, '2W') or contains(@srcset, '3W') or contains(@srcset, '4W') or contains(@srcset, '5W') or contains(@srcset, '6W') or contains(@srcset, '7W') or contains(@srcset, '8W') or contains(@srcset, '9W')"/>
      <assert id="elements.srcset-sizes-needs-w-descriptor" role="error" test="$has-width-descriptor">
        The "sizes" attribute must only be specified if the "srcset" attribute contains image candidate strings with width descriptors.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-source-siblings-srcset">
    <rule context="h:source[following-sibling::h:source[@srcset] or following-sibling::h:img[@srcset]]">
      <assert id="elements.source-needs-media-or-type" role="error" test="@media or @type">
        A "source" element that has a following sibling "source" element or "img" element with a "srcset" attribute must have a "media" attribute or a "type" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-custom-element-is-attribute">
    <rule context="*[contains(local-name(), '-') and @is]">
      <assert id="elements.custom-element-no-is" role="error" test="not(@is)">
        Autonomous custom elements must not specify the "is" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-form-accept-charset">
    <rule context="h:form[@accept-charset]">
      <assert id="elements.form-accept-charset-utf8" role="error"
        test="translate(normalize-space(@accept-charset), 'UTF-8', 'utf-8') = 'utf-8'">
        The only allowed value for the "accept-charset" attribute for the "form" element is "utf-8".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-required">
    <rule context="h:input[@required]">
      <assert id="elements.input-required-types" role="error"
        test="not(@type) or @type = 'checkbox' or @type = 'date' or @type = 'datetime-local' or @type = 'email' or @type = 'file' or @type = 'month' or @type = 'number' or @type = 'password' or @type = 'radio' or @type = 'search' or @type = 'tel' or @type = 'text' or @type = 'time' or @type = 'url' or @type = 'week'">
        Attribute "required" is only allowed when the input type is "checkbox", "date", "datetime-local", "email", "file", "month", "number", "password", "radio", "search", "tel", "text", "time", "url", or "week".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-imagesrcset-width-descriptor">
    <rule context="h:link[@imagesrcset and not(@imagesizes)]">
      <assert id="elements.link-imagesrcset-w-needs-imagesizes" role="error"
        test="not(contains(@imagesrcset, '0w') or contains(@imagesrcset, '1w') or contains(@imagesrcset, '2w') or contains(@imagesrcset, '3w') or contains(@imagesrcset, '4w') or contains(@imagesrcset, '5w') or contains(@imagesrcset, '6w') or contains(@imagesrcset, '7w') or contains(@imagesrcset, '8w') or contains(@imagesrcset, '9w') or contains(@imagesrcset, '0W') or contains(@imagesrcset, '1W') or contains(@imagesrcset, '2W') or contains(@imagesrcset, '3W') or contains(@imagesrcset, '4W') or contains(@imagesrcset, '5W') or contains(@imagesrcset, '6W') or contains(@imagesrcset, '7W') or contains(@imagesrcset, '8W') or contains(@imagesrcset, '9W'))">
        When the "imagesrcset" attribute has any image candidate string with a width descriptor, the "imagesizes" attribute must also be specified.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-meta-charset">
    <rule context="h:meta[@charset]">
      <assert id="elements.meta-charset-unique" role="error"
        test="count(//h:meta[@charset]) &lt;= 1">
        A document must not include more than one "meta" element with a "charset" attribute.
      </assert>
      <assert id="elements.meta-charset-no-content-type" role="error"
        test="not(//h:meta[translate(@http-equiv, 'CONTENT-TYPE', 'content-type') = 'content-type'])">
        A document must not include both a "meta" element with an "http-equiv" attribute whose value is "content-type", and a "meta" element with a "charset" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-meta-media-theme-color">
    <rule context="h:meta[@media]">
      <assert id="elements.meta-media-needs-theme-color" role="error"
        test="translate(@name, 'THEME-COLOR', 'theme-color') = 'theme-color'">
        A "meta" element with a "media" attribute must have a "name" attribute whose value is "theme-color".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-meta-x-ua-compatible">
    <rule context="h:meta[translate(@http-equiv, 'X-UA-COMPATIBLE', 'x-ua-compatible') = 'x-ua-compatible']">
      <assert id="elements.meta-x-ua-compatible-ie-edge" role="error"
        test="translate(normalize-space(@content), 'IE=EDGE', 'ie=edge') = 'ie=edge'">
        A "meta" element with an "http-equiv" attribute whose value is "X-UA-Compatible" must have a "content" attribute with the value "IE=edge".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-select-button-child">
    <rule context="h:select[h:button]">
      <assert id="elements.select-button-allowed-only-dropdown" role="error"
        test="@size = 1 or (not(@size) and not(@multiple))">
        A "button" element is only allowed as a child of a "select" element that is a drop-down box (one without a "size" attribute greater than 1 and without a "multiple" attribute).
      </assert>
    </rule>
    <rule context="h:select/h:button[@aria-label]">
      <assert id="elements.select-button-no-aria-label" role="error" test="not(@aria-label)">
        The "aria-label" attribute must not be used on a "button" element that is a child of a "select" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-dt-forbidden-descendants">
    <rule context="h:dt//h:h1">
      <report id="elements.dt-no-h1" role="error" test="true()">
        The element "h1" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:h2">
      <report id="elements.dt-no-h2" role="error" test="true()">
        The element "h2" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:h3">
      <report id="elements.dt-no-h3" role="error" test="true()">
        The element "h3" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:h4">
      <report id="elements.dt-no-h4" role="error" test="true()">
        The element "h4" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:h5">
      <report id="elements.dt-no-h5" role="error" test="true()">
        The element "h5" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:h6">
      <report id="elements.dt-no-h6" role="error" test="true()">
        The element "h6" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:header">
      <report id="elements.dt-no-header" role="error" test="true()">
        The element "header" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:hgroup">
      <report id="elements.dt-no-hgroup" role="error" test="true()">
        The element "hgroup" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:nav">
      <report id="elements.dt-no-nav" role="error" test="true()">
        The element "nav" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
    <rule context="h:dt//h:section">
      <report id="elements.dt-no-section" role="error" test="true()">
        The element "section" must not appear as a descendant of the "dt" element.
      </report>
    </rule>
  </pattern>

  <pattern id="elements-base-position">
    <rule context="h:base">
      <assert id="elements.base-before-link-script" role="error"
        test="not(preceding::h:link or preceding::h:script)">
        The "base" element must come before any "link" or "script" elements in the document.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-async-defer">
    <rule context="h:script[not(@src) and (not(@type) or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = '' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'text/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'application/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'ecmascript')]">
      <assert id="elements.script-inline-no-async-defer" role="error"
        test="not(@async) and not(@defer)">
        An inline classic "script" element must not have an "async" or "defer" attribute.
      </assert>
      <assert id="elements.script-inline-classic-no-blocking" role="error" test="not(@blocking)">
        An inline classic "script" element must not have a "blocking" attribute.
      </assert>
      <assert id="elements.script-inline-classic-no-fetchpriority" role="error" test="not(@fetchpriority)">
        An inline classic "script" element must not have a "fetchpriority" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-inline-module-attributes">
    <rule context="h:script[not(@src) and translate(normalize-space(@type), 'MODULE', 'module') = 'module']">
      <assert id="elements.script-inline-module-no-blocking" role="error" test="not(@blocking)">
        An inline "script" element with "type=module" must not have a "blocking" attribute.
      </assert>
      <assert id="elements.script-inline-module-no-fetchpriority" role="error" test="not(@fetchpriority)">
        An inline "script" element with "type=module" must not have a "fetchpriority" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-type-text-javascript-unnecessary">
    <rule context="h:script[translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'text/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'application/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'ecmascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'text/ecmascript']">
      <report id="elements.script-type-text-javascript-unnecessary" role="warning" test="true()">
        The "type" attribute is unnecessary for JavaScript resources.
      </report>
    </rule>
  </pattern>

  <pattern id="elements-source-sizes-auto-needs-lazy-img">
    <rule context="h:source[starts-with(normalize-space(translate(@sizes, 'AUTO', 'auto')), 'auto') and not(../h:img[@loading = 'lazy'])]">
      <assert id="elements.source-sizes-auto-lazy" role="error" test="false()">
        The "sizes" attribute value starting with "auto" is only valid for lazy-loaded images. The sibling "img" element must have a "loading" attribute set to "lazy".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-img-sizes-auto-and-srcset">
    <rule context="h:img[starts-with(normalize-space(translate(@sizes, 'AUTO', 'auto')), 'auto') and not(@loading = 'lazy')]">
      <assert id="elements.img-sizes-auto-lazy" role="error" test="false()">
        The "sizes" attribute value starting with "auto" is only valid for lazy-loaded images. Add "loading=lazy" to this element.
      </assert>
    </rule>
    <rule context="h:img[@sizes and not(@srcset)] | h:source[@sizes and not(@srcset)]">
      <assert id="elements.sizes-needs-srcset" role="error" test="false()">
        The "sizes" attribute must only be specified if the "srcset" attribute is also specified.
      </assert>
    </rule>
    <rule context="h:img[@controls and (not(@alt) or normalize-space(@alt) = '')]">
      <assert id="elements.img-controls-needs-alt" role="error" test="false()">
        The "controls" attribute must not be specified on an "img" element that does not have an "alt" attribute, or whose "alt" attribute's value is the empty string.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-autocomplete-pattern-formaction">
    <rule context="h:input[@type = 'hidden' and (translate(@autocomplete, 'ON', 'on') = 'on' or translate(@autocomplete, 'OFF', 'off') = 'off')]">
      <assert id="elements.input-hidden-no-autocomplete-on-off" role="error" test="false()">
        An "input" element with a "type" attribute whose value is "hidden" must not have an "autocomplete" attribute whose value is "on" or "off".
      </assert>
    </rule>
    <rule context="h:input[@pattern and not(@type = 'email' or @type = 'password' or @type = 'search' or @type = 'tel' or @type = 'text' or @type = 'url' or not(@type))]">
      <assert id="elements.input-pattern-types" role="error" test="false()">
        Attribute "pattern" is only allowed when the input type is "email", "password", "search", "tel", "text", or "url".
      </assert>
    </rule>
    <rule context="h:input[@formaction and not(@type = 'submit' or @type = 'image')]">
      <assert id="elements.input-formaction-types" role="error" test="false()">
        Attribute "formaction" is only allowed when the input type is "submit" or "image".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-importmap-attributes">
    <rule context="h:script[translate(normalize-space(@type), 'IMPORTMAP', 'importmap') = 'importmap']">
      <assert id="elements.script-importmap-no-crossorigin" role="error"
        test="not(@crossorigin) and not(@integrity) and not(@referrerpolicy) and not(@nonce)">
        A "script" element with "type=importmap" must not have a "crossorigin" attribute.
      </assert>
      <assert id="elements.script-importmap-no-blocking" role="error" test="not(@blocking)">
        A "script" element with "type=importmap" must not have a "blocking" attribute.
      </assert>
      <assert id="elements.script-importmap-no-fetchpriority" role="error" test="not(@fetchpriority)">
        A "script" element with "type=importmap" must not have a "fetchpriority" attribute.
      </assert>
      <assert id="elements.script-importmap-no-nomodule" role="error" test="not(@nomodule)">
        A "script" element with "type=importmap" must not have a "nomodule" attribute.
      </assert>
    </rule>
    <rule context="h:script[@language]">
      <report id="elements.script-language-obsolete" role="warning" test="true()">
        The "language" attribute on the "script" element is obsolete. Use the "type" attribute instead.
      </report>
    </rule>
  </pattern>

  <pattern id="elements-option-empty">
    <rule context="h:option[not(@label) and normalize-space(.) = '']">
      <assert id="elements.option-empty-needs-label" role="error" test="false()">
        Element "option" without attribute "label" must not be empty.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-label-for-rules">
    <rule context="h:label[@for]">
      <let name="for-id" value="@for"/>
      <assert id="elements.label-for-non-form-control" role="error"
        test="//h:button[@id = $for-id] or //h:input[@id = $for-id and not(@type='hidden')] or //h:meter[@id = $for-id] or //h:output[@id = $for-id] or //h:progress[@id = $for-id] or //h:select[@id = $for-id] or //h:textarea[@id = $for-id]">
        The value of the "for" attribute of the "label" element must be the ID of a non-hidden form control.
      </assert>
    </rule>
    <rule context="h:label[@for]//h:input">
      <assert id="elements.label-for-descendant-matching-id" role="error"
        test="@id = ancestor::h:label[1]/@for">
        Any "input" descendant of a "label" element with a "for" attribute must have an ID value that matches that "for" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-speculationrules-attributes">
    <rule context="h:script[translate(normalize-space(@type), 'SPECULATIONRULES', 'speculationrules') = 'speculationrules']">
      <assert id="elements.script-speculationrules-no-disallowed-attrs" role="error"
        test="not(@src) and not(@async) and not(@defer) and not(@crossorigin) and not(@integrity) and not(@referrerpolicy) and not(@nonce)">
        A "script" element with "type=speculationrules" must not have any fetching or execution attributes.
      </assert>
      <assert id="elements.script-speculationrules-no-blocking" role="error" test="not(@blocking)">
        A "script" element with "type=speculationrules" must not have a "blocking" attribute.
      </assert>
      <assert id="elements.script-speculationrules-no-fetchpriority" role="error" test="not(@fetchpriority)">
        A "script" element with "type=speculationrules" must not have a "fetchpriority" attribute.
      </assert>
      <assert id="elements.script-speculationrules-no-nomodule" role="error" test="not(@nomodule)">
        A "script" element with "type=speculationrules" must not have a "nomodule" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-header-footer-nesting">
    <rule context="h:footer//h:footer | h:header//h:footer">
      <assert id="elements.no-footer-in-header-footer" role="error" test="false()">
        The element "footer" must not appear as a descendant of the "footer" or "header" element.
      </assert>
    </rule>
    <rule context="h:footer//h:header | h:header//h:header">
      <assert id="elements.no-header-in-header-footer" role="error" test="false()">
        The element "header" must not appear as a descendant of the "footer" or "header" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-dl-duplicate-dt">
    <rule context="h:dt[normalize-space(.) != '' and normalize-space(.) = normalize-space(following-sibling::h:dt)]">
      <report id="elements.dl-duplicate-dt" role="warning" test="true()">
        Within a single "dl" element, there should not be more than one "dt" element for each name.
      </report>
    </rule>
  </pattern>

  <!-- Same digit+w/W approximation as elements-srcset-x-with-sizes-invalid above, and for the same reason (a candidate URL's own "w" doesn't count). -->
  <pattern id="elements-source-srcset-w-needs-sizes">
    <rule context="h:source[@srcset and not(@sizes) and not(../h:img[@loading='lazy'])]">
      <let name="has-width-descriptor"
        value="contains(@srcset, '0w') or contains(@srcset, '1w') or contains(@srcset, '2w') or contains(@srcset, '3w') or contains(@srcset, '4w') or contains(@srcset, '5w') or contains(@srcset, '6w') or contains(@srcset, '7w') or contains(@srcset, '8w') or contains(@srcset, '9w') or contains(@srcset, '0W') or contains(@srcset, '1W') or contains(@srcset, '2W') or contains(@srcset, '3W') or contains(@srcset, '4W') or contains(@srcset, '5W') or contains(@srcset, '6W') or contains(@srcset, '7W') or contains(@srcset, '8W') or contains(@srcset, '9W')"/>
      <assert id="elements.source-srcset-w-needs-sizes" role="error" test="not($has-width-descriptor)">
        When the "srcset" attribute has any image candidate string with a width descriptor, the "sizes" attribute must also be specified.
      </assert>
    </rule>
    <rule context="h:img[@srcset and not(@sizes) and ancestor::h:picture and not(@loading='lazy')]">
      <let name="has-width-descriptor"
        value="contains(@srcset, '0w') or contains(@srcset, '1w') or contains(@srcset, '2w') or contains(@srcset, '3w') or contains(@srcset, '4w') or contains(@srcset, '5w') or contains(@srcset, '6w') or contains(@srcset, '7w') or contains(@srcset, '8w') or contains(@srcset, '9w') or contains(@srcset, '0W') or contains(@srcset, '1W') or contains(@srcset, '2W') or contains(@srcset, '3W') or contains(@srcset, '4W') or contains(@srcset, '5W') or contains(@srcset, '6W') or contains(@srcset, '7W') or contains(@srcset, '8W') or contains(@srcset, '9W')"/>
      <assert id="elements.img-picture-srcset-w-needs-sizes" role="error" test="not($has-width-descriptor)">
        When the "srcset" attribute has any image candidate string with a width descriptor, the "sizes" attribute must also be specified.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-script-datablock-no-fetching-attrs">
    <rule context="h:script[@type and not(translate(normalize-space(@type), 'MODULE', 'module') = 'module' or translate(normalize-space(@type), 'IMPORTMAP', 'importmap') = 'importmap' or translate(normalize-space(@type), 'SPECULATIONRULES', 'speculationrules') = 'speculationrules' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = '' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'text/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'application/javascript' or translate(normalize-space(@type), 'JAVASCRIPT', 'javascript') = 'ecmascript') and (@src or @async or @defer or @crossorigin or @integrity or @referrerpolicy or @nonce or @fetchpriority or @nomodule or @blocking)]">
      <assert id="elements.script-datablock-no-async" role="error" test="false()">
        A "script" element with a non-JavaScript/module type attribute must not have any fetching or execution attributes.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-rel-constraints">
    <rule context="h:link[@color]">
      <assert id="elements.link-color-needs-mask-icon" role="error"
        test="contains(concat(' ', normalize-space(@rel), ' '), ' mask-icon ')">
        A "link" element with a "color" attribute must have a "rel" attribute that contains the value "mask-icon".
      </assert>
    </rule>
    <rule context="h:link[@disabled]">
      <assert id="elements.link-disabled-needs-stylesheet" role="error"
        test="contains(concat(' ', normalize-space(@rel), ' '), ' stylesheet ')">
        A "link" element with a "disabled" attribute must have a "rel" attribute that contains the value "stylesheet".
      </assert>
    </rule>
    <rule context="h:link[@sizes]">
      <assert id="elements.link-sizes-allowed-rel" role="error"
        test="contains(concat(' ', normalize-space(@rel), ' '), ' icon ') or contains(concat(' ', normalize-space(@rel), ' '), ' apple-touch-icon ') or contains(concat(' ', normalize-space(@rel), ' '), ' apple-touch-icon-precomposed ')">
        A "link" element with a "sizes" attribute must have a "rel" attribute that contains the value "icon" or the value "apple-touch-icon" or the value "apple-touch-icon-precomposed".
      </assert>
    </rule>
    <rule context="h:link[ancestor::h:body]">
      <assert id="elements.link-in-body-rel" role="error"
        test="@itemprop or contains(concat(' ', normalize-space(@rel), ' '), ' dns-prefetch ') or contains(concat(' ', normalize-space(@rel), ' '), ' modulepreload ') or contains(concat(' ', normalize-space(@rel), ' '), ' pingback ') or contains(concat(' ', normalize-space(@rel), ' '), ' preconnect ') or contains(concat(' ', normalize-space(@rel), ' '), ' prefetch ') or contains(concat(' ', normalize-space(@rel), ' '), ' preload ') or contains(concat(' ', normalize-space(@rel), ' '), ' prerender ') or contains(concat(' ', normalize-space(@rel), ' '), ' stylesheet ')">
        A "link" element must not appear as a descendant of a "body" element unless the "link" element has an "itemprop" attribute or has a "rel" attribute whose value contains "dns-prefetch", "modulepreload", "pingback", "preconnect", "prefetch", "preload", "prerender", or "stylesheet".
      </assert>
    </rule>
    <rule context="h:link[contains(concat(' ', normalize-space(@rel), ' '), ' modulepreload ') and (@as='font' or @as='image')]">
      <assert id="elements.link-modulepreload-as-invalid" role="error" test="false()">
        The value of the "as" attribute for a "link" element with "rel=modulepreload" is invalid.
      </assert>
    </rule>
    <rule context="h:link[contains(concat(' ', normalize-space(@rel), ' '), ' preload ') and (@as='json' or @as='worker')]">
      <assert id="elements.link-preload-as-invalid" role="error" test="false()">
        The value of the "as" attribute for a "link" element with "rel=preload" is invalid.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-isindex-name">
    <rule context="h:input[translate(normalize-space(@name), 'ISINDEX', 'isindex') = 'isindex']">
      <assert id="elements.input-name-isindex-forbidden" role="error" test="false()">
        The value "isindex" for the "name" attribute of the "input" element is not allowed.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-img-ismap-usemap">
    <rule context="h:img[@ismap and not(ancestor::h:a[@href])]">
      <assert id="elements.img-ismap-needs-a-href" role="error" test="false()">
        The "img" element with the "ismap" attribute set must have an "a" ancestor with the "href" attribute.
      </assert>
    </rule>
    <rule context="h:img[@usemap]">
      <let name="map-name" value="substring-after(@usemap, '#')"/>
      <assert id="elements.img-usemap-target-exists" role="error"
        test="$map-name = '' or //h:map[@name = $map-name]">
        The hash-name reference in attribute "usemap" referred to a non-existent map.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-select-required-options">
    <rule context="h:select[@required and not(@multiple) and (not(@size) or number(@size) &lt;= 1)]">
      <assert id="elements.select-required-needs-option" role="error" test="count(h:option) &gt; 0">
        A "select" element with a "required" attribute, and without a "multiple" attribute, and without a "size" attribute whose value is greater than "1", must have a child "option" element.
      </assert>
      <assert id="elements.select-required-first-option-placeholder" role="error"
        test="count(h:option) = 0 or (h:option[1][not(@value) or @value = '' or normalize-space(.) = ''])">
        The first child "option" element of a "select" element with a "required" attribute, and without a "multiple" attribute, and without a "size" attribute whose value is greater than "1", must have either an empty "value" attribute, or must have no text content.
      </assert>
    </rule>
    <rule context="h:select[@autocomplete and contains(concat(' ', normalize-space(@autocomplete), ' '), ' webauthn ')]">
      <assert id="elements.select-autocomplete-no-webauthn" role="error" test="false()">
        The value of the "autocomplete" attribute for the "select" element must not contain "webauthn".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-address-no-nested-address">
    <rule context="h:address//h:address">
      <assert id="elements.address-no-nested-address" role="error" test="false()">
        The element "address" must not appear as a descendant of the "address" element.
      </assert>
    </rule>
  </pattern>

  <!--
    Narrowed to role="img" specifically, not "any role" — the original
    evidence (html/elements/figure/with-figcaption-and-role-novalid.html)
    uses role="img", but html-aria/misc/figure-with-role-doc-example-and-
    figcaption.html shows role="doc-example" alongside a figcaption is
    valid. role="img" is the one case with a real conflict: it collapses
    figure's children (including figcaption's text) into a single
    accessible-image node, discarding the figcaption's own semantics.
  -->
  <pattern id="elements-figure-figcaption-role">
    <rule context="h:figure[h:figcaption][@role = 'img']">
      <assert id="elements.figure-figcaption-no-role" role="error" test="false()">
        A "figure" element with a "figcaption" descendant must not have a "role" attribute whose value is "img".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-figure-table-caption">
    <rule context="h:figure[h:figcaption][count(*[not(self::h:figcaption)]) = 1][h:table/h:caption]">
      <report id="elements.figure-table-caption-should-be-figcaption" role="warning" test="true()">
        When a "table" element is the only content in a "figure" element other than the "figcaption", the "caption" element should be omitted in favor of the "figcaption".
      </report>
    </rule>
  </pattern>

  <pattern id="elements-link-alternate-stylesheet-title">
    <rule context="h:link[@rel and contains(concat(' ', translate(normalize-space(@rel), 'ALTERNATIVE', 'alternative'), ' '), ' alternate ') and contains(concat(' ', translate(normalize-space(@rel), 'ALTERNATIVE', 'alternative'), ' '), ' stylesheet ')]">
      <assert id="elements.link-alternate-stylesheet-needs-title" role="error" test="@title and normalize-space(@title) != ''">
        A "link" element with a "rel" attribute that contains both the values "alternate" and "stylesheet" must have a "title" attribute with a non-empty value.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-blocking-stylesheet">
    <rule context="h:link[@blocking]">
      <assert id="elements.link-blocking-needs-stylesheet" role="error"
        test="contains(concat(' ', translate(normalize-space(@rel), 'STYLESHET', 'stylesheet'), ' '), ' stylesheet ')">
        A "link" element with a "blocking" attribute must have a "rel" attribute whose value is "stylesheet".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-link-imagesizes-needs-imagesrcset">
    <rule context="h:link[@imagesizes and not(@imagesrcset)]">
      <assert id="elements.link-imagesizes-needs-imagesrcset" role="error" test="false()">
        The "imagesizes" attribute must only be specified if the "imagesrcset" attribute is also specified.
      </assert>
    </rule>
  </pattern>

  <!--
    Case-sensitive on purpose: html/elements/meta/names-standard-isvalid.html
    has "description"/"DESCRIPTION"/"dEScrIpTiON" meta names side by side
    and expects no finding, so vnu's real check is an exact-string
    comparison against the literal (lowercase) "description", not a
    case-insensitive one.
  -->
  <pattern id="elements-meta-multiple-description">
    <rule context="h:meta[@name = 'description'][count(preceding::h:meta[@name = 'description']) &gt; 0]">
      <assert id="elements.meta-multiple-description" role="error" test="false()">
        A document must not include more than one "meta" element with its "name" attribute set to the value "description".
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-meta-viewport-user-scalable-no">
    <rule context="h:meta[translate(@name, 'VIEWPORT', 'viewport') = 'viewport'][@content and contains(translate(@content, 'USERSCALABLE', 'userscalable'), 'user-scalable') and (contains(translate(@content, 'NO', 'no'), 'user-scalable=no') or contains(translate(@content, 'NO', 'no'), 'user-scalable=0'))]">
      <report id="elements.meta-viewport-user-scalable-no" role="warning" test="true()">
        Consider avoiding viewport values that prevent users from resizing documents.
      </report>
    </rule>
  </pattern>

  <pattern id="elements-optgroup-needs-label">
    <rule context="h:optgroup[not(h:legend)][not(@label)]">
      <assert id="elements.optgroup-no-label-no-legend" role="error" test="false()">
        An "optgroup" element with no child "legend" element must have a "label" attribute.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-title-not-empty">
    <rule context="h:title[normalize-space(.) = '']">
      <assert id="elements.title-not-empty" role="error" test="false()">
        Element "title" must not be empty.
      </assert>
    </rule>
  </pattern>
  <pattern id="elements-script-type-text-javascript">
    <rule context="h:script[translate(normalize-space(@type), 'TEXT/JAVASCRIPT', 'text/javascript') = 'text/javascript']">
      <report id="elements.script-type-text-javascript-warning" role="warning" test="true()">
        The "type" attribute is unnecessary for JavaScript resources.
      </report>
    </rule>
  </pattern>
  <pattern id="elements-audio-controls-in-button">
    <rule context="h:audio[@controls and ancestor::h:button]">
      <assert id="elements.audio-controls-in-button" role="error" test="false()">
        The element "audio" with the attribute "controls" must not appear as a descendant of the "button" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-microdata-itemprop-itemref">
    <rule context="*[@itemprop and not(ancestor-or-self::*[@itemscope])]">
      <assert id="elements.itemprop-needs-itemscope" role="error" test="false()">
        The "itemprop" attribute was specified, but the element is not a property of any item.
      </assert>
    </rule>
    <rule context="*[@itemref]">
      <assert id="elements.itemref-target-exists" role="error"
        test="not(contains(concat(' ', normalize-space(@itemref), ' '), ' nonexistent '))">
        The "itemref" attribute referenced a non-existent element id.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-img-missing-alt-in-figure">
    <rule context="h:img[not(@alt) and ancestor::h:figure[not(h:figcaption)]]">
      <assert id="elements.img-missing-alt-in-figure" role="error" test="false()">
        An "img" element must have an "alt" attribute, except under certain conditions. For details, consult guidance on providing text alternatives for images.
      </assert>
    </rule>
  </pattern>
  <pattern id="elements-a-href-in-button">
    <rule context="h:a[@href and ancestor::h:button]">
      <assert id="elements.a-href-in-button" role="error" test="false()">
        The element "a" with the attribute "href" must not appear as a descendant of the "button" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-dt-no-footer-or-header">
    <rule context="h:dt//h:footer | h:dt//h:header">
      <assert id="elements.dt-no-footer-or-header" role="error" test="false()">
        The element "footer" must not appear as a descendant of the "dt" element.
      </assert>
    </rule>
  </pattern>

  <pattern id="elements-input-autocomplete-webauthn-alone">
    <rule context="h:input[normalize-space(@autocomplete) = 'webauthn']">
      <assert id="elements.input-autocomplete-webauthn-alone" role="error" test="false()">
        Bad value "webauthn" for attribute "autocomplete" on element "input".
      </assert>
    </rule>
  </pattern>
  <pattern id="elements-meta-charset-not-utf8">
    <rule context="h:meta[@charset and translate(normalize-space(@charset), 'UTF-8', 'utf-8') != 'utf-8']">
      <assert id="elements.meta-charset-not-utf8" role="error" test="false()">
        Internal encoding declaration "iso-8859-1" disagrees with the actual encoding of the document ("utf-8").
      </assert>
    </rule>
  </pattern>
</schema>
