# Changelog

All notable changes to this project will be documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Releases before 0.4.0 are described
in the git history only.

## [Unreleased]

## [0.5.1] - 2026-11-10

### Added

- **Document:** `Node::end_position` gives the position just past an element's start tag (exclusive end),
  so a host can name the start tag's source range; `None` for synthesized and non-element nodes.

## [0.5.0] - 2026-11-09

### Added

- **ParseErrorKind:** `StrayCharacter`, `NestedHeading`, `NestedButton`, `OptionInScope` and
  `ForeignContentBreakout` for the spec's remaining unnamed parse errors: ignored characters in
  the frameset modes, a heading or `button` start tag directly inside one, `option`/`optgroup`/`hr`
  with an option in scope, and HTML tags that break out of SVG/MathML (§13.2.6.5).
- **Tests:** the html5lib tree-construction harness also compares the reported parse errors with
  each case's `#errors`/`#new-errors` (presence and WHATWG codes; exact count and positions are
  reported only). 1,591 of 1,602 cases agree on presence (1,486 before); the 11 exceptions
  (files without error expectations) are listed in `tests/html5lib_errors_known_failures.txt`.

### Changed

- **Tree construction:** `parse()` reports more tree-construction parse errors for the same input;
  the tree is unchanged (html5lib conformance stays at 1,726/1,726).
- **Tree construction:** an end tag whose element still has open children after implied end tags
  (`<div><span></div>`; also `li`, `dd`/`dt`, headings, `form`, `button`, `td`/`th`, `caption`,
  `template`, any other end tag and foreign content) is reported as
  `ImpliedEndTagWithUnclosedElements`, as are `li`/`dd`/`dt` and `rb`/`rtc`/`rp`/`rt` start tags
  with an unclosed current node.
- **Tree construction:** content after `</html>` is `TokenAfterBody`; ignored tokens in the
  frameset modes are `StrayStartTag`/`StrayEndTag`/`StrayCharacter`, and EOF with a frameset or
  `template` open is `EofWithUnclosedElements`; a head-only start tag after `</head>` and `input`
  inside `select` are `StrayStartTag`; `td`/`th` in a table body is `MisplacedTokenInTable`.

- **Tokenizer:** `UnexpectedNullCharacter` is reported in every state the spec lists it for (RCDATA,
  RAWTEXT, PLAINTEXT, script data, tag/attribute/comment/DOCTYPE states), plus
  `EofInScriptHtmlCommentLikeText` and `MissingQuoteBeforeDoctypeSystemIdentifier` where they were
  missing. Token streams are unchanged.

## [0.4.2] - 2026-11-06

### Added

- **Document:** `NodeId::as_u32` returns the node's arena index as a plain number.

### Changed

- **Tree construction:** `tree_builder.rs` is split into a `tree_builder/` module directory
  (insertion modes, adoption agency, foreign content, scope helpers, tests). Internal only, no
  API or behaviour change.
- **Document:** `Document::node` now documents that it panics on a `NodeId` from another
  document.

## [0.4.1] - 2026-10-27

### Fixed

- **Tree construction:** Cloning a selected `option` into `selectedcontent` and finding that
  `selectedcontent` no longer recurse once per nesting level, so deeply nested option content
  cannot overflow the stack (auditmysite#783).

## [0.4.0] - 2026-09-14

### Added

- **ParseErrorKind:** `StrayStartTag`: a start tag the spec ignores (or merges into an existing
  element) with a parse error: `html`, `body` and `frameset` in body, a second `head`,
  `caption`/`col`/`colgroup`/`frame`/`tbody`/`td`/`tfoot`/`th`/`thead`/`tr` outside a table, and a
  `select` start tag while a `select` is open.
- **ParseErrorKind:** `NestedFormattingElement`: an `a` start tag while an `a` is still in the list
  of active formatting elements, or `nobr` while a `nobr` is in scope (§13.2.6.4.7).
- **ParseErrorKind:** `MisnestedFormattingElement`: adoption agency algorithm step 4.6, e.g. `</b>`
  in `<b><i>x</b></i>`.
- **ParseErrorKind:** `FormattingElementNotInScope`: adoption agency algorithm step 4.5, e.g.
  `</b>` in `<b><table></b>`.

### Changed

- **Tree construction:** `parse()` now reports tree-construction parse errors (WHATWG §13.2.6) that
  0.3.0 dropped silently. The parsed tree is unchanged (html5lib-tests tree-construction
  conformance stays at 1,726/1,726), but `ParseResult::errors` contains more entries for the same
  input, hence the minor version bump.
- **Tree construction:** `StrayEndTag` is now also reported when an end tag has no matching
  element in scope: `</div>`, `</header>`, `</li>`, `</dd>`/`</dt>`, `</h1>`–`</h6>`, `</form>`,
  `</applet>`/`</marquee>`/`</object>`, `</body>`/`</html>` (the whole "in body" end-tag group).
  Previously only the "any other end tag" path (`</span>`) reported it.
- **Tree construction:** `StrayEndTag` also covers any other end tag in "before html", "before
  head", "in head", "in head noscript", "after head" and "in template", and adoption agency step
  4.4 (e.g. `</i>` in `<b><i>x</b></i>`).
- **Tree construction:** `StrayEndTagInTable` is now also reported by "in caption", "in column
  group", "in table body", "in row" and "in cell" for the end tags they ignore, and
  `MisplacedTokenInTable` for the start tags they ignore and for a `form` start tag in a table.
- **Tree construction:** `StrayDoctype` is now also reported inside foreign content (§13.2.6.5).
