<?xml version="1.0" encoding="UTF-8"?>
<!--
  Phase 06 skeleton rule set — see rules/aria.sch's header comment for
  why this reuses a Phase 02 canary case and why real coverage is
  Phase 08.

  Uses a named element test (`th`), so it needs the `h:` namespace
  prefix bound below — see rules/README.md: this crate's infoset gives
  every plain HTML element the XHTML namespace
  (http://www.w3.org/1999/xhtml), and XPath 1.0's unprefixed name
  tests only ever match nodes with *no* namespace (the `xmlns` default
  namespace is explicitly not used for name-test expansion, per the
  XPath 1.0 spec, §2.3) — an unprefixed `th` context would silently
  match nothing.
-->
<schema xmlns="http://purl.oclc.org/dsdl/schematron">
  <ns prefix="h" uri="http://www.w3.org/1999/xhtml"/>
  <pattern id="tables">
    <rule context="h:th[@scope]">
      <assert id="tables.th-scope-enum" role="error" test="@scope='row' or @scope='col' or @scope='rowgroup' or @scope='colgroup'">
        A th element's scope attribute must be one of row, col, rowgroup, or colgroup.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-colspan-max">
    <rule context="h:td[@colspan] | h:th[@colspan]">
      <assert id="tables.colspan-max" role="error" test="not(number(@colspan) &gt; 1000)">
        The value of the "colspan" attribute must be less than or equal to 1000.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-rowspan-max">
    <rule context="h:td[@rowspan] | h:th[@rowspan]">
      <assert id="tables.rowspan-max" role="error" test="not(number(@rowspan) &gt; 65534)">
        The value of the "rowspan" attribute must be less than or equal to 65534.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-span-max">
    <rule context="h:col[@span] | h:colgroup[@span]">
      <assert id="tables.span-max" role="error" test="not(number(@span) &gt; 1000)">
        The value of the "span" attribute must be less than or equal to 1000.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-td-role">
    <rule context="h:td[@role]">
      <assert id="tables.td-role-in-table" role="error"
        test="not(ancestor::h:table[1][not(@role) or @role = 'table' or @role = 'grid' or @role = 'treegrid'])">
        The "role" attribute must not be used on a "td" element which has a "table" ancestor with no "role" attribute, or with a "role" attribute whose value is "table", "grid", or "treegrid".
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-headers">
    <rule context="h:td[@headers] | h:th[@headers]">
      <let name="me" value="."/>
      <let name="table" value="ancestor::h:table[1]"/>
      <assert id="tables.headers-ref-th" role="error"
        test="id(@headers)[self::h:th and ancestor::h:table[1] = $table]">
        The "headers" attribute on the element refers to an ID, but there is no "th" element with that ID in the same table.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-row-no-cells">
    <rule context="h:tr[not(h:td) and not(h:th)]">
      <assert id="tables.row-no-cells" role="error" test="false()">
        A row of a row group has no cells beginning on it.
      </assert>
    </rule>
  </pattern>

  <!--
    Row width (number of columns a row spans, ignoring rowspan-inherited
    cells from earlier rows — not evidenced by any corpus fixture that a
    "row width" comparison needs to account for those too, so this stays
    the simple "sum this row's own td/th colspans" reading) vs. the two
    ways HTML5 can establish a table's column count: an explicit
    `colgroup`/`col` structure, or (when there is none) the first row's
    own width — https://html.spec.whatwg.org/#column-groups,
    #forming-a-table. `sum()` of an empty node-set is 0 per the XPath 1.0
    spec (verified against this engine's implementation, not assumed),
    so the "no colspan attribute" `count()` terms and the "has a colspan
    attribute" `sum()` terms combine correctly even when one side is
    empty.
  -->
  <pattern id="tables-row-width-vs-column-markup">
    <rule context="h:tr[ancestor::h:table[1]/h:colgroup]">
      <let name="table" value="ancestor::h:table[1]"/>
      <let name="row-width"
        value="count(h:td[not(@colspan)]) + count(h:th[not(@colspan)]) + sum(h:td/@colspan) + sum(h:th/@colspan)"/>
      <let name="column-count-from-markup"
        value="count($table/h:colgroup/h:col[not(@span)]) + sum($table/h:colgroup/h:col/@span) + count($table/h:colgroup[not(h:col)][not(@span)]) + sum($table/h:colgroup[not(h:col)]/@span)"/>
      <assert id="tables.row-width-not-less-than-column-markup" role="error"
        test="$row-width &gt;= $column-count-from-markup">
        A table row was narrower than the column count established using column markup.
      </assert>
      <assert id="tables.row-width-not-exceeding-column-markup" role="error"
        test="$row-width &lt;= $column-count-from-markup">
        A table row exceeded the column count established using column markup.
      </assert>
    </rule>
  </pattern>

  <pattern id="tables-row-width-vs-first-row">
    <!--
      `$table//h:tr[1]` is a trap (found while writing this rule, see the
      commit that introduced it): the `[1]` on the LAST step of a `//`
      path filters *per intermediate context node* produced by
      `descendant-or-self::node()`, not the whole result set in document
      order — for a table with a *nested* table inside a cell, it
      returns the first `tr` of EVERY tbody-like element it passes
      through, both the outer table's and the nested one's. Explicit
      structural alternatives (`$table/h:tr`, `$table/h:thead/h:tr`,
      etc. — never descending into a nested table, which can't be a
      direct child or grandchild of `$table` this way) unioned and THEN
      wrapped in `(...)[1]` gives the true first row in document order
      (a parenthesized node-set expression's `[1]` IS whole-set
      position, unlike a bare location step's).
    -->
    <rule context="h:tr[not(ancestor::h:table[1]/h:colgroup)]">
      <let name="table" value="ancestor::h:table[1]"/>
      <let name="first-row"
        value="($table/h:tr | $table/h:thead/h:tr | $table/h:tbody/h:tr | $table/h:tfoot/h:tr)[1]"/>
      <let name="row-width"
        value="count(h:td[not(@colspan)]) + count(h:th[not(@colspan)]) + sum(h:td/@colspan) + sum(h:th/@colspan)"/>
      <let name="first-row-width"
        value="count($first-row/h:td[not(@colspan)]) + count($first-row/h:th[not(@colspan)]) + sum($first-row/h:td/@colspan) + sum($first-row/h:th/@colspan)"/>
      <assert id="tables.row-width-not-less-than-first-row" role="warning"
        test="$row-width &gt;= $first-row-width">
        A table row was narrower than the column count established by the first row.
      </assert>
      <assert id="tables.row-width-not-exceeding-first-row" role="warning"
        test="$row-width &lt;= $first-row-width">
        A table row exceeded the column count established by the first row.
      </assert>
    </rule>
  </pattern>
</schema>
