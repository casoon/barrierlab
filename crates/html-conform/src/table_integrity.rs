//! Table integrity: the 2D cell grid a table's `colspan`/`rowspan`
//! combinations actually form (Phase 08 backlog item 1, 5 fixtures under
//! `html/elements/table/`).
//!
//! Same category as `src/scripts.rs` and `src/csp_enforcement.rs`: a check
//! whose *logic*, not just its vocabulary, is outside what a `rules/*.sch`
//! XPath 1.0 rule can express. Laying out a table means walking cells in
//! document order while carrying mutable state forward — an insertion
//! point that advances per cell, a set of cells still spanning down from
//! earlier rows, and a shrinking list of column ranges that no cell has
//! started in yet. XPath 1.0 has no loops, no recursion and no mutable
//! accumulators, so there is no sound encoding of this; the existing
//! `rules/tables.sch` row-width rules are explicitly documented as the
//! "simple half" (they sum a row's *own* colspans and ignore cells
//! inherited from an earlier row's `rowspan`), and this module is the
//! other half.
//!
//! Ported from vnu's own implementation (`nu/validator/checker/table/`:
//! `TableChecker`, `Table`, `RowGroup`, `Cell`, `ColumnRange`,
//! `VerticalCellComparator`, `HorizontalCellComparator`), fetched from
//! `validator/validator` rather than reconstructed from the expected
//! message texts — the messages are terse ("Table column 3 established by
//! element "td" has no cells beginning in it.") and the state machine
//! behind them is not guessable from them.
//!
//! **Deliberately only reports the three checks that need the grid**, even
//! though the ported model computes enough for more:
//!
//! - cells overlapping horizontally within a row (`Cell.errOnHorizontalOverlap`),
//! - cells whose `rowspan` reaches past the end of their row group
//!   (`Cell.errIfNotRowspanZero`),
//! - columns established by column markup or by a wide cell that no cell
//!   ever begins in (`Table.end`'s `ColumnRange` walk).
//!
//! vnu's same classes also produce the row-width and empty-row messages,
//! but `rules/tables.sch` already covers those (`tables.row-no-cells`,
//! `tables.row-width-*`), and the `headers` referential-integrity check
//! (`tables.headers-ref-th`) — re-emitting them here would double-report
//! and would mean deleting working, zero-false-positive rules for no gain.
//! The `colspan`/`rowspan`/`span` upper bounds vnu enforces inside these
//! same classes are likewise left to `tables.colspan-max`/`rowspan-max`/
//! `span-max`; the *clamping* they imply is kept here, because it bounds
//! the grid arithmetic.
//!
//! Walks the raw `html5_parser::Document` in document order, turning it
//! back into the start/end element event pairs vnu's SAX-based checker
//! consumes. That is sound because vnu sees a *tree-constructed* stream
//! too: HTML5 parsing has already inserted implied `tbody` elements and
//! foster-parented misplaced content out of the table before either
//! checker sees it.

use html5_parser::{Attribute, Document, NodeId, NodeKind};

use crate::{Finding, Severity, SourceLocation};

const RULE_ID: &str = "tables.integrity";

/// `Cell.MAX_COLSPAN` / `TableChecker.MAX_COLSPAN`.
const MAX_COLSPAN: i64 = 1000;

/// `Cell.MAX_ROWSPAN`, which doubles as vnu's magic "this cell was written
/// `rowspan=0`, i.e. it spans to the end of its row group" marker.
const MAX_ROWSPAN: i64 = 65534;

/// Reports table cells that overlap, span past their row group, or leave a
/// column with no cell beginning in it.
pub(crate) fn findings(document: &Document) -> Vec<Finding> {
    let mut checker = Checker::default();
    checker.walk(document, document.root());
    checker.findings
}

/// `TableChecker`: a stack of open tables (a table nested in a cell of
/// another table is checked independently).
#[derive(Default)]
struct Checker {
    tables: Vec<Table>,
    findings: Vec<Finding>,
}

impl Checker {
    fn walk(&mut self, document: &Document, id: NodeId) {
        let node = document.node(id);
        // vnu's `TableChecker` only reacts to XHTML-namespace elements —
        // an SVG `<foreignObject>` subtree can hold a real HTML table, and
        // conversely nothing outside the XHTML namespace is one. A missing
        // namespace is read as XHTML, the same way `src/infoset.rs`'s
        // `normalize` does.
        let name = match &node.kind {
            NodeKind::Element {
                name,
                namespace,
                attributes,
            } if namespace
                .as_deref()
                .is_none_or(|namespace| namespace == crate::infoset::XHTML_NAMESPACE) =>
            {
                let location = node.position.map(|position| SourceLocation {
                    line: position.line,
                    column: position.column,
                    byte_offset: position.byte_offset,
                });
                self.start_element(name, attributes, location);
                Some(name.clone())
            }
            _ => None,
        };

        for child in document.children(id) {
            self.walk(document, child);
        }

        if let Some(name) = name {
            self.end_element(&name);
        }
    }

    fn start_element(
        &mut self,
        name: &str,
        attributes: &[Attribute],
        location: Option<SourceLocation>,
    ) {
        if name == "table" {
            self.tables.push(Table::default());
            return;
        }
        let Checker { tables, findings } = self;
        let Some(table) = tables.last_mut() else {
            return;
        };
        match name {
            "td" => table.start_cell(false, attributes, location, findings),
            "th" => table.start_cell(true, attributes, location, findings),
            "tr" => table.start_row(),
            "tbody" | "thead" | "tfoot" => table.start_row_group(name, findings),
            "col" => table.start_col(clamp_span(attributes), location),
            "colgroup" => table.start_col_group(clamp_span(attributes)),
            _ => {}
        }
    }

    fn end_element(&mut self, name: &str) {
        if name == "table" {
            let Checker { tables, findings } = self;
            if let Some(mut table) = tables.pop() {
                table.end(findings);
            }
            return;
        }
        let Checker { tables, findings } = self;
        let Some(table) = tables.last_mut() else {
            return;
        };
        match name {
            "td" | "th" => table.end_cell(),
            "tr" => table.end_row(),
            "tbody" | "thead" | "tfoot" => table.end_row_group(findings),
            "col" => table.end_col(),
            "colgroup" => table.end_col_group(),
            _ => {}
        }
    }
}

/// `Table.State`, ported one-for-one.
// The shared `In` prefix is vnu's own naming (`IN_TABLE_AT_START`, ...);
// keeping it makes each arm greppable against the Java source, which is
// worth more here than clippy's shorter-name preference.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    InTableAtStart,
    InTableAtPotentialRowGroupStart,
    InColgroup,
    InColInColgroup,
    InColInImplicitGroup,
    InRowGroup,
    InRowInRowGroup,
    InCellInRowGroup,
    InRowInImplicitRowGroup,
    InImplicitRowGroup,
    InCellInImplicitRowGroup,
    InTableColsSeen,
}

/// A positioned cell — `Cell` after `setPosition`. `left`/`right` are
/// column slots (`right` exclusive), `bottom` is the first row of the row
/// group this cell does *not* cover (or [`MAX_ROWSPAN`] for `rowspan=0`).
#[derive(Clone)]
struct Cell {
    left: i64,
    right: i64,
    bottom: i64,
    header: bool,
    location: Option<SourceLocation>,
}

impl Cell {
    fn element_name(&self) -> &'static str {
        if self.header { "th" } else { "td" }
    }
}

/// `ColumnRange`: a contiguous half-open column range `[left, right)`
/// established by one element that no cell has begun in yet.
#[derive(Clone)]
struct ColumnRange {
    element: &'static str,
    location: Option<SourceLocation>,
    left: i64,
    right: i64,
}

impl ColumnRange {
    /// `ColumnRange.hits`: -1 left of the range, 0 inside it, 1 right of it.
    fn hits(&self, column: i64) -> i32 {
        if column < self.left {
            -1
        } else if column >= self.right {
            1
        } else {
            0
        }
    }

    fn is_single_col(&self) -> bool {
        self.left + 1 == self.right
    }

    /// `ColumnRange.toString`, which the two messages below interpolate.
    fn describe(&self) -> String {
        if self.is_single_col() {
            self.right.to_string()
        } else {
            format!("{}\u{2026}{}", self.left + 1, self.right)
        }
    }
}

/// `RowGroup`, explicit (`tbody`/`thead`/`tfoot`) or implicit.
struct RowGroup {
    /// `null` type in vnu — an implicit row group.
    element: Option<String>,
    current_row: i64,
    insertion_point: i64,
    next_old_cell: usize,
    row_had_cells: bool,
    /// `cellsIfEffect`: cells from earlier rows still spanning downwards,
    /// kept in vnu's `VerticalCellComparator` order (by `bottom`, then by
    /// `left`) so that end-of-group reporting comes out in the same order.
    cells_in_effect: Vec<Cell>,
    /// `cellsOnCurrentRow`: the `cells_in_effect` snapshot taken at row
    /// start, re-sorted by `left` (`HorizontalCellComparator`).
    cells_on_current_row: Vec<Cell>,
}

impl RowGroup {
    fn new(element: Option<String>) -> Self {
        Self {
            element,
            current_row: -1,
            insertion_point: 0,
            next_old_cell: 0,
            row_had_cells: false,
            cells_in_effect: Vec::new(),
            cells_on_current_row: Vec::new(),
        }
    }

    fn describe(&self) -> String {
        match &self.element {
            Some(element) => format!("row group established by a \"{element}\" element"),
            None => "implicit row group".to_owned(),
        }
    }

    /// `RowGroup.startRow`.
    fn start_row(&mut self) {
        self.current_row += 1;
        self.insertion_point = 0;
        self.next_old_cell = 0;
        self.row_had_cells = false;
        self.cells_on_current_row = self.cells_in_effect.clone();
        self.cells_on_current_row.sort_by_key(|cell| cell.left);
    }

    /// `RowGroup.findInsertionPoint`.
    fn find_insertion_point(&mut self) {
        while self.next_old_cell < self.cells_on_current_row.len() {
            let other = &self.cells_on_current_row[self.next_old_cell];
            if self.insertion_point < other.left {
                break;
            }
            if other.right > self.insertion_point {
                self.insertion_point = other.right;
            }
            self.next_old_cell += 1;
        }
    }

    /// `RowGroup.cell` — positions the cell, reports horizontal overlaps
    /// against the still-pending cells of earlier rows, and hands the
    /// positioned cell to the table's column bookkeeping.
    fn cell(&mut self, mut cell: Cell, columns: &mut Columns, findings: &mut Vec<Finding>) {
        self.row_had_cells = true;
        self.find_insertion_point();

        // `Cell.setPosition(top, left)`: `right`/`bottom` still hold the
        // raw colspan/rowspan at this point and become absolute here.
        cell.left = self.insertion_point;
        cell.right += self.insertion_point;
        if cell.bottom != MAX_ROWSPAN {
            cell.bottom += self.current_row;
        }

        columns.cell(&cell);

        if cell.bottom > self.current_row + 1 {
            self.insert_in_effect(cell.clone());
        }
        self.insertion_point = cell.right;

        for other in &self.cells_on_current_row[self.next_old_cell..] {
            // `Cell.errOnHorizontalOverlap`, which reports on both cells.
            if !(cell.right <= other.left || other.right <= cell.left) {
                findings.push(error(
                    "Table cell is overlapped by later table cell.".to_owned(),
                    other.location,
                ));
                findings.push(error(
                    "Table cell overlaps an earlier table cell.".to_owned(),
                    cell.location,
                ));
            }
        }
    }

    /// `cellsIfEffect.add` — a `TreeSet` ordered by `VerticalCellComparator`.
    fn insert_in_effect(&mut self, cell: Cell) {
        let position = self
            .cells_in_effect
            .partition_point(|other| (other.bottom, other.left) < (cell.bottom, cell.left));
        self.cells_in_effect.insert(position, cell);
    }

    /// `RowGroup.endRow`, minus the empty-row and row-width messages
    /// (`rules/tables.sch` owns those — see the module comment). The
    /// column-count bookkeeping itself is kept: it is what makes a row
    /// group's first row establish the table width.
    fn end_row(&mut self, columns: &mut Columns) {
        self.find_insertion_point();
        self.cells_on_current_row = Vec::new();

        if !columns.hard_width && columns.column_count == -1 {
            columns.column_count = self.insertion_point;
        }

        let current_row = self.current_row;
        self.cells_in_effect
            .retain(|cell| current_row + 1 < cell.bottom);
    }

    /// `RowGroup.end`: whatever is still spanning downwards at the end of
    /// the group reached past it (`Cell.errIfNotRowspanZero`; `rowspan=0`
    /// means "to the end of the group" by definition and is exempt).
    fn end(&self, findings: &mut Vec<Finding>) {
        for cell in &self.cells_in_effect {
            if cell.bottom != MAX_ROWSPAN {
                findings.push(error(
                    format!(
                        "Table cell spans past the end of its {}; clipped to the end of the row group.",
                        self.describe()
                    ),
                    cell.location,
                ));
            }
        }
    }
}

/// The column-tracking half of `Table` — split out from [`Table`] only so
/// that the open [`RowGroup`] can borrow it while vnu's `RowGroup` simply
/// calls back into its owner.
#[derive(Default)]
struct Columns {
    /// Whether column markup (`col`/`colgroup`) established the width.
    hard_width: bool,
    /// `-1` until the first row or column markup establishes it.
    column_count: i64,
    real_column_count: i64,
    /// The linked list of not-yet-hit column ranges, in list order.
    ranges: Vec<ColumnRange>,
    /// `currentColRange`, as an index into `ranges`; `ranges.len()` is
    /// vnu's `null`.
    cursor: usize,
}

impl Columns {
    fn new() -> Self {
        Self {
            hard_width: false,
            column_count: -1,
            real_column_count: 0,
            ranges: Vec::new(),
            cursor: 0,
        }
    }

    /// `Table.appendColumnRange`.
    ///
    /// vnu's `ColumnRange` constructor asserts `right > left`; Java
    /// assertions are off in production, so a `span="0"` `col` would
    /// silently append an empty range there that can never be hit and is
    /// therefore always reported at `Table.end`. Honouring the asserted
    /// invariant instead — an empty range is not a column anybody can put
    /// a cell in, and `span="0"` is already rejected by the schema
    /// (`tables.rnc`'s `span` is a positive integer).
    fn append(&mut self, range: ColumnRange) {
        if range.right > range.left {
            self.ranges.push(range);
        }
    }

    /// `Table.cell`: records that a positioned cell begins in a column,
    /// widening the table and splitting/shrinking/dropping column ranges.
    fn cell(&mut self, cell: &Cell) {
        let (left, right) = (cell.left, cell.right);
        if right > self.real_column_count {
            if left == self.real_column_count {
                // Entirely past the last known column: only the slots
                // *after* the one this cell begins in are unaccounted for.
                if left + 1 != right {
                    self.append(ColumnRange {
                        element: cell.element_name(),
                        location: cell.location,
                        left: left + 1,
                        right,
                    });
                }
                self.real_column_count = right;
                return;
            }
            self.append(ColumnRange {
                element: cell.element_name(),
                location: cell.location,
                left: self.real_column_count,
                right,
            });
            self.real_column_count = right;
        }

        while self.cursor < self.ranges.len() {
            match self.ranges[self.cursor].hits(left) {
                // `ColumnRange.removeColumn`.
                0 => {
                    let range = &mut self.ranges[self.cursor];
                    if range.is_single_col() {
                        self.ranges.remove(self.cursor);
                    } else if left == range.left {
                        range.left += 1;
                    } else if left + 1 == range.right {
                        range.right -= 1;
                    } else {
                        let split = ColumnRange {
                            element: range.element,
                            location: range.location,
                            left: left + 1,
                            right: range.right,
                        };
                        range.right = left;
                        self.ranges.insert(self.cursor + 1, split);
                        self.cursor += 1;
                    }
                    return;
                }
                -1 => return,
                _ => self.cursor += 1,
            }
        }
    }
}

/// `Table`.
struct Table {
    state: State,
    suppressed_starts: u32,
    pending_col_group_span: i64,
    columns: Columns,
    group: Option<RowGroup>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            state: State::default(),
            suppressed_starts: 0,
            pending_col_group_span: 0,
            columns: Columns::new(),
            group: None,
        }
    }
}

impl Table {
    fn need_suppress_start(&mut self) -> bool {
        if self.suppressed_starts > 0 {
            self.suppressed_starts += 1;
            true
        } else {
            false
        }
    }

    fn need_suppress_end(&mut self) -> bool {
        if self.suppressed_starts > 0 {
            self.suppressed_starts -= 1;
            true
        } else {
            false
        }
    }

    fn start_row_group(&mut self, element: &str, findings: &mut Vec<Finding>) {
        if self.need_suppress_start() {
            return;
        }
        match self.state {
            State::InImplicitRowGroup => {
                if let Some(group) = self.group.take() {
                    group.end(findings);
                }
                self.group = Some(RowGroup::new(Some(element.to_owned())));
                self.state = State::InRowGroup;
            }
            State::InTableAtStart
            | State::InTableColsSeen
            | State::InTableAtPotentialRowGroupStart => {
                self.group = Some(RowGroup::new(Some(element.to_owned())));
                self.state = State::InRowGroup;
            }
            _ => self.suppressed_starts = 1,
        }
    }

    fn end_row_group(&mut self, findings: &mut Vec<Finding>) {
        if self.need_suppress_end() {
            return;
        }
        if self.state == State::InRowGroup {
            if let Some(group) = self.group.take() {
                group.end(findings);
            }
            self.state = State::InTableAtPotentialRowGroupStart;
        }
    }

    fn start_row(&mut self) {
        if self.need_suppress_start() {
            return;
        }
        match self.state {
            State::InTableAtStart
            | State::InTableColsSeen
            | State::InTableAtPotentialRowGroupStart => {
                self.group = Some(RowGroup::new(None));
                self.state = State::InRowInImplicitRowGroup;
            }
            State::InImplicitRowGroup => self.state = State::InRowInImplicitRowGroup,
            State::InRowGroup => self.state = State::InRowInRowGroup,
            _ => {
                self.suppressed_starts = 1;
                return;
            }
        }
        self.columns.cursor = 0;
        if let Some(group) = self.group.as_mut() {
            group.start_row();
        }
    }

    fn end_row(&mut self) {
        if self.need_suppress_end() {
            return;
        }
        match self.state {
            State::InRowInRowGroup => self.state = State::InRowGroup,
            State::InRowInImplicitRowGroup => self.state = State::InImplicitRowGroup,
            _ => return,
        }
        let Table { columns, group, .. } = self;
        if let Some(group) = group.as_mut() {
            group.end_row(columns);
        }
    }

    fn start_cell(
        &mut self,
        header: bool,
        attributes: &[Attribute],
        location: Option<SourceLocation>,
        findings: &mut Vec<Finding>,
    ) {
        if self.need_suppress_start() {
            return;
        }
        match self.state {
            State::InRowInRowGroup => self.state = State::InCellInRowGroup,
            State::InRowInImplicitRowGroup => self.state = State::InCellInImplicitRowGroup,
            _ => {
                self.suppressed_starts = 1;
                return;
            }
        }

        // `Cell`'s constructor: a missing/malformed span is 1 (a missing
        // `rowspan` becomes -1 and then `Math.abs`s to 1), and both are
        // clamped — vnu errors on the excess here, `rules/tables.sch`
        // does it for this crate.
        let colspan = parse_positive_integer(attribute_value(attributes, "colspan"))
            .abs()
            .min(MAX_COLSPAN);
        let rowspan = parse_non_negative_integer(attribute_value(attributes, "rowspan"))
            .abs()
            .min(MAX_ROWSPAN);

        let cell = Cell {
            left: 0,
            right: colspan,
            bottom: if rowspan == 0 { MAX_ROWSPAN } else { rowspan },
            header,
            location,
        };

        let Table { columns, group, .. } = self;
        if let Some(group) = group.as_mut() {
            group.cell(cell, columns, findings);
        }
    }

    fn end_cell(&mut self) {
        if self.need_suppress_end() {
            return;
        }
        match self.state {
            State::InCellInRowGroup => self.state = State::InRowInRowGroup,
            State::InCellInImplicitRowGroup => self.state = State::InRowInImplicitRowGroup,
            _ => {}
        }
    }

    fn start_col_group(&mut self, span: i64) {
        if self.need_suppress_start() {
            return;
        }
        match self.state {
            State::InTableAtStart => {
                self.columns.hard_width = true;
                self.columns.column_count = 0;
                self.pending_col_group_span = span;
                self.state = State::InColgroup;
            }
            State::InTableColsSeen => {
                self.pending_col_group_span = span;
                self.state = State::InColgroup;
            }
            _ => self.suppressed_starts = 1,
        }
    }

    fn end_col_group(&mut self) {
        if self.need_suppress_end() {
            return;
        }
        if self.state == State::InColgroup {
            if self.pending_col_group_span != 0 {
                // A `colgroup` with no `col` children establishes its own
                // columns; a negative pending span is vnu's marker for an
                // *implied* `span="1"`.
                let right = self.columns.column_count + self.pending_col_group_span.abs();
                self.columns.append(ColumnRange {
                    element: "colgroup",
                    location: None,
                    left: self.columns.column_count,
                    right,
                });
                self.columns.column_count = right;
            }
            self.columns.real_column_count = self.columns.column_count;
            self.state = State::InTableColsSeen;
        }
    }

    fn start_col(&mut self, span: i64, location: Option<SourceLocation>) {
        if self.need_suppress_start() {
            return;
        }
        match self.state {
            State::InTableAtStart => {
                self.columns.hard_width = true;
                self.columns.column_count = 0;
                self.state = State::InColInImplicitGroup;
            }
            State::InTableColsSeen => self.state = State::InColInImplicitGroup,
            State::InColgroup => {
                // vnu additionally warns that a `col` child makes the
                // parent `colgroup`'s own `span` be ignored — a separate
                // message, not one of the three this module reports.
                self.pending_col_group_span = 0;
                self.state = State::InColInColgroup;
            }
            _ => {
                self.suppressed_starts = 1;
                return;
            }
        }
        let right = self.columns.column_count + span.abs();
        self.columns.append(ColumnRange {
            element: "col",
            location,
            left: self.columns.column_count,
            right,
        });
        self.columns.column_count = right;
        self.columns.real_column_count = self.columns.column_count;
    }

    fn end_col(&mut self) {
        if self.need_suppress_end() {
            return;
        }
        match self.state {
            State::InColInImplicitGroup => self.state = State::InTableColsSeen,
            State::InColInColgroup => self.state = State::InColgroup,
            _ => {}
        }
    }

    /// `Table.end`: closes an implicit row group, then reports every
    /// column range no cell ever began in.
    fn end(&mut self, findings: &mut Vec<Finding>) {
        if self.state == State::InImplicitRowGroup
            && let Some(group) = self.group.take()
        {
            group.end(findings);
        }

        for range in &self.columns.ranges {
            let message = if range.is_single_col() {
                format!(
                    "Table column {} established by element \"{}\" has no cells beginning in it.",
                    range.describe(),
                    range.element
                )
            } else {
                format!(
                    "Table columns in range {} established by element \"{}\" have no cells beginning in them.",
                    range.describe(),
                    range.element
                )
            };
            findings.push(error(message, range.location));
        }
    }
}

fn attribute_value<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value.as_str())
}

/// `TableChecker.clampSpan`: a `col`/`colgroup` `span`, clamped, with a
/// missing or malformed value staying at vnu's `-1` "implied" marker.
fn clamp_span(attributes: &[Attribute]) -> i64 {
    parse_non_negative_integer(attribute_value(attributes, "span")).min(MAX_COLSPAN)
}

/// `AttributeUtil.parseInteger`: the value must match
/// `^[ \t\n\r]*(-?[0-9]+)$` in full (the regex is applied with Java's
/// `matches()`, which anchors both ends, so trailing garbage is an error
/// despite the comment above it in vnu's source). `None` on error, which
/// stands in for vnu's `Integer.MIN_VALUE`.
fn parse_integer(value: Option<&str>) -> Option<i32> {
    let rest = value?.trim_start_matches([' ', '\t', '\n', '\r']);
    let digits = rest.strip_prefix('-').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    // `Integer.parseInt` throws on overflow, which vnu also maps to the
    // error value.
    rest.parse().ok()
}

/// `AttributeUtil.parseNonNegativeInteger`: -1 on error or a negative value.
fn parse_non_negative_integer(value: Option<&str>) -> i64 {
    match parse_integer(value) {
        Some(parsed) if parsed >= 0 => i64::from(parsed),
        _ => -1,
    }
}

/// `AttributeUtil.parsePositiveInteger`: -1 on error or a value below 1.
fn parse_positive_integer(value: Option<&str>) -> i64 {
    match parse_integer(value) {
        Some(parsed) if parsed >= 1 => i64::from(parsed),
        _ => -1,
    }
}

fn error(message: String, location: Option<SourceLocation>) -> Finding {
    Finding {
        rule_id: RULE_ID.to_owned(),
        severity: Severity::Error,
        message,
        location,
    }
}

#[cfg(test)]
mod tests {
    fn table_findings(html: &str) -> Vec<String> {
        crate::check(html)
            .expect("HTML5 parsing should recover")
            .findings
            .into_iter()
            .filter(|finding| finding.rule_id == super::RULE_ID)
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn plain_rectangular_table_is_clean() {
        assert!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td>a<td>b</tr><tr><td>c<td>d</tr></table>"
            )
            .is_empty()
        );
    }

    #[test]
    fn rowspan_and_colspan_that_tile_exactly_are_clean() {
        assert!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td rowspan=2>a<td>b<td>c</tr>\
                 <tr><td colspan=2>d</tr></table>"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_cell_pushed_past_the_table_width_leaves_an_empty_column() {
        // The colspan=2 cell can only start in column 2 (column 1 is
        // still covered by the rowspan), so it reaches column 3, which
        // then has no cell of its own beginning in it.
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td rowspan=2>a<td>b</tr>\
                 <tr><td colspan=2>c</tr></table>"
            ),
            ["Table column 3 established by element \"td\" has no cells beginning in it."]
        );
    }

    #[test]
    fn a_wide_cell_leaves_a_range_of_empty_columns() {
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td>a<td colspan=5>b</tr></table>"
            ),
            [
                "Table columns in range 3\u{2026}6 established by element \"td\" have no cells \
                 beginning in them."
            ]
        );
    }

    #[test]
    fn a_rowspan_reaching_past_its_row_group_is_reported() {
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tbody><tr><td rowspan=3>a</tr><tr><td>b</tr></tbody></table>"
            ),
            [
                "Table cell spans past the end of its row group established by a \"tbody\" \
                 element; clipped to the end of the row group."
            ]
        );
    }

    #[test]
    fn rowspan_zero_never_reaches_past_its_row_group() {
        // `rowspan=0` means "to the end of the row group" by definition.
        assert!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tbody><tr><td rowspan=0>a<td>b</tr><tr><td>c</tr></tbody></table>"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_cell_landing_on_a_cell_spanning_down_from_an_earlier_row_overlaps() {
        // The colspan=2 cell starts in the free column 1 and runs into
        // column 2, which the first row's rowspan=3 cell still covers.
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td><td rowspan=3><td rowspan=3>a</tr>\
                 <tr><td colspan=2><td rowspan=2>b</tr></table>"
            )
            .into_iter()
            .filter(|message| message.contains("overlap"))
            .collect::<Vec<_>>(),
            [
                "Table cell is overlapped by later table cell.",
                "Table cell overlaps an earlier table cell.",
            ]
        );
    }

    #[test]
    fn column_markup_matching_the_rows_is_clean() {
        assert!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><colgroup><col><col></colgroup>\
                 <tr><td>a<td>b</tr></table>"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_col_no_cell_ever_starts_in_is_reported() {
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><colgroup><col><col><col></colgroup>\
                 <tr><td>a<td colspan=2>b</tr></table>"
            ),
            ["Table column 3 established by element \"col\" has no cells beginning in it."]
        );
    }

    #[test]
    fn a_nested_table_is_tracked_independently() {
        // The inner table is well-formed; only the outer one's empty
        // third column should be reported.
        assert_eq!(
            table_findings(
                "<!doctype html><title>t</title>\
                 <table><tr><td rowspan=2><table><tr><td>x</tr></table><td>b</tr>\
                 <tr><td colspan=2>c</tr></table>"
            ),
            ["Table column 3 established by element \"td\" has no cells beginning in it."]
        );
    }
}
