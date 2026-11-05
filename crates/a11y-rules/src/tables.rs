//! Tabellenregeln nach WCAG 2.2 (1.3.1), dem HTML-Standard („The table
//! element", Zuordnung von Kopfzellen über `headers`) und WAI-ARIA 1.2
//! (`table`, `grid`, `treegrid` und ihre Zellen).
//!
//! Portiert aus auditmysite (`table_extended`: `td-headers-attr` und
//! `th-has-data-cells`; casoon/barrierlab#20). auditmysite ist der
//! Vergleichspunkt, nicht die Norm — Abweichungen stehen an der jeweiligen
//! Stelle und im CHANGELOG.
//!
//! Beide Regeln lesen nur Tags, `role` und `id`: Tier 1. Chrome blendet ein
//! schlichtes `<tbody>` im Accessibility-Tree aus (auditmysite#638); hier gibt
//! es diese Eigenheit nicht, die Zeilen stehen im Markup unter ihm.

use std::collections::HashMap;

use a11y_dom::{Document, Node, NodeId, NodeKind, ScopedId, ancestors, elements, scoped_id};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::explicit_role;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Eine Tabelle: `<table>` ohne fremde Rolle oder `table`, `grid`,
/// `treegrid` per `role`. Eine präsentationale Tabelle ist keine — ihre
/// Kopfzellen meldet `tables/presentational-with-headers`.
fn is_table<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => matches!(r, "table" | "grid" | "treegrid"),
        None => n.is_element("table"),
    }
}

/// Eine Kopfzelle: `<th>` oder `columnheader`, `rowheader` per `role`.
fn is_header_cell<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => matches!(r, "columnheader" | "rowheader"),
        None => n.is_element("th"),
    }
}

/// Eine Datenzelle: `<td>` oder `cell`, `gridcell` per `role`.
fn is_data_cell<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => matches!(r, "cell" | "gridcell"),
        None => n.is_element("td"),
    }
}

fn is_row<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => r == "row",
        None => n.is_element("tr"),
    }
}

fn is_row_group<'a, N: Node<'a>>(n: N) -> bool {
    match explicit_role(n) {
        Some(r) => r == "rowgroup",
        None => matches!(n.local_name(), "thead" | "tbody" | "tfoot"),
    }
}

/// Was in einer Tabelle steht, ohne verschachtelte Tabellen.
struct Cells<N> {
    head: Vec<N>,
    data: usize,
    /// Eine Zeilengruppe ohne eine einzige Zeile: Die Zeilen stehen noch aus
    /// oder sind ausgeblendet.
    empty_group: bool,
}

fn cells<'a, N: Node<'a>>(table: N) -> Cells<N> {
    let mut z = Cells {
        head: Vec::new(),
        data: 0,
        empty_group: false,
    };
    let mut stack: Vec<N> = table.children().collect();
    while let Some(n) = stack.pop() {
        if n.kind() != NodeKind::Element || is_table(n) {
            continue;
        }
        if is_header_cell(n) {
            z.head.push(n);
        } else if is_data_cell(n) {
            z.data += 1;
        } else if is_row_group(n) && !n.children().any(is_row) {
            z.empty_group = true;
        }
        stack.extend(n.children());
    }
    z.head.sort_by(|a, b| a.document_order(*b));
    z
}

// --- tables/header-without-data -------------------------------------------

/// 1.3.1: Eine Tabelle mit Kopfzellen, aber ohne eine einzige Datenzelle. Die
/// Kopfzellen beschriften nichts.
///
/// Wie in auditmysite eine Aussage über die ganze Tabelle, nicht je Spalte:
/// Eine Spalte aus Zeilenköpfen (`<th scope="row">`) unter einem
/// Spaltenkopf ist das übliche Muster „Tabelle mit zwei Kopfrichtungen"
/// (auditmysite#638, barrierlab.eu). Leere Datenzellen zählen mit.
///
/// Steht eine Zeilengruppe ohne Zeile in der Tabelle — die Zeilen werden noch
/// geladen oder sind ausgeblendet, etwa der Zeilenvorrat eines virtualisierten
/// Grids —, ist das nicht entscheidbar: `tables/data-undetermined` statt
/// `FAIL` (auditmysite#654). Beleg für den Verstoß: auditmysite-Korpus
/// `table_headers_no_data` und `table_grid_rows_unrendered` (`#header-only`).
pub(crate) fn header_without_data<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for t in elements(doc).filter(|n| is_table(*n)) {
        let z = cells(t);
        if z.head.is_empty() || z.data > 0 {
            continue;
        }
        if z.empty_group {
            out.push(
                Finding::untested(
                    "tables/data-undetermined",
                    pick!(
                        locale,
                        "The table has header cells and a row group without rows — the rows are \
                         still loading or hidden. Whether the headers get data cells cannot be \
                         determined; check the loaded table.",
                        "Die Tabelle hat Kopfzellen und eine Zeilengruppe ohne Zeilen — die \
                         Zeilen werden noch geladen oder sind ausgeblendet. Ob die Kopfzellen \
                         Datenzellen bekommen, ist nicht bestimmbar; die geladene Tabelle prüfen.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1"])
                .at(at(t.id())),
            );
            continue;
        }
        for k in z.head {
            out.push(
                Finding::fail(
                    "tables/header-without-data",
                    pick!(
                        locale,
                        "The header cell has no data cells: the table contains no <td> at all.",
                        "Die Kopfzelle hat keine Datenzellen: Die Tabelle enthält kein einziges \
                         <td>.",
                    ),
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1"])
                .at(at(k.id())),
            );
        }
    }
}

// --- tables/headers-attr-invalid ------------------------------------------

/// Die nächste umgebende Tabelle, die Zelle selbst ausgenommen.
fn table_of<'a, N: Node<'a>>(n: N) -> Option<N> {
    ancestors(n).find(|a| is_table(*a))
}

/// 1.3.1: Eine Zelle verweist mit `headers` ins Leere — auf eine ID, die es
/// nicht gibt, die keine Kopfzelle ist oder die in einer anderen Tabelle
/// steht. HTML verlangt dort die IDs von `<th>` derselben Tabelle; die
/// Assistenztechnik sagt sonst keine oder eine falsche Kopfzelle an.
///
/// Anders als auditmysite nur an `<td>` und `<th>` (bzw. `cell`,
/// `gridcell`, `columnheader`, `rowheader`): Nur dort definiert HTML das
/// Attribut. Beleg: auditmysite-Korpus `forms_extended`
/// (`<td headers="does-not-exist">`).
pub(crate) fn headers_attr<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut ids: HashMap<ScopedId, D::N<'_>> = HashMap::new();
    for n in elements(doc) {
        if let Some(id) = n.attr("id") {
            ids.entry(scoped_id(n, id)).or_insert(n);
        }
    }
    for cell in elements(doc) {
        let Some(headers) = cell.attr("headers") else {
            continue;
        };
        if !(is_data_cell(cell) || is_header_cell(cell)) {
            continue;
        }
        let table = table_of(cell);
        let invalid = headers.split_whitespace().find(|r| {
            ids.get(&scoped_id(cell, r)).is_none_or(|k| {
                !is_header_cell(*k) || *k == cell || table.is_some_and(|t| table_of(*k) != Some(t))
            })
        });
        let text = match (headers.trim().is_empty(), invalid) {
            (true, _) => pick!(
                locale,
                "The cell has an empty headers attribute.",
                "Die Zelle hat ein leeres headers-Attribut.",
            )
            .to_string(),
            (false, Some(r)) => tr!(
                locale,
                "The headers attribute references \"{r}\", which is not a header cell of this \
                 table.",
                "Das headers-Attribut verweist auf \"{r}\", das keine Kopfzelle dieser Tabelle \
                 ist."
            ),
            (false, None) => continue,
        };
        out.push(
            Finding::fail("tables/headers-attr-invalid", text)
                .with_severity(Severity::High)
                .with_wcag(["1.3.1"])
                .at(at(cell.id())),
        );
    }
}
