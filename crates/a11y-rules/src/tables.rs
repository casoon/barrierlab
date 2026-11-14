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

// --- Tabellenmodell -----------------------------------------------------------

/// Der `scope` einer Kopfzelle (HTML, „The th element"). Ohne gültiges
/// `scope` gilt die explizite Rolle: `columnheader` wie `col`, `rowheader`
/// wie `row` (WAI-ARIA 1.2).
#[derive(Clone, Copy, PartialEq, Eq)]
enum HeaderScope {
    Auto,
    Row,
    Col,
    RowGroup,
    ColGroup,
}

fn header_scope<'a, N: Node<'a>>(n: N) -> HeaderScope {
    let scope = n
        .is_element("th")
        .then(|| n.attr("scope"))
        .flatten()
        .map(|s| s.trim().to_ascii_lowercase());
    match scope.as_deref() {
        Some("row") => HeaderScope::Row,
        Some("col") => HeaderScope::Col,
        Some("rowgroup") => HeaderScope::RowGroup,
        Some("colgroup") => HeaderScope::ColGroup,
        _ => match explicit_role(n) {
            Some("columnheader") => HeaderScope::Col,
            Some("rowheader") => HeaderScope::Row,
            _ => HeaderScope::Auto,
        },
    }
}

/// Eine Zelle im Raster der Tabelle (HTML, „Forming a table").
struct Slot<N> {
    node: N,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    header: bool,
    scope: HeaderScope,
    group: usize,
}

/// Ein Spannenattribut von `<td>`/`<th>` nach HTML: `colspan` 1 bis 1000,
/// `rowspan` 0 bis 65534 (0: bis zum Ende der Zeilengruppe), sonst 1.
fn span<'a, N: Node<'a>>(n: N, attr: &str, max: usize) -> usize {
    if !(n.is_element("td") || n.is_element("th")) {
        return 1;
    }
    n.attr(attr)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map_or(1, |v| v.min(max))
}

/// Die Zeilen einer Tabelle in Rasterordnung, je mit ihrer Zeilengruppe.
/// `<tfoot>` rückt ans Ende (HTML, „Forming a table"); Zeilen direkt in der
/// Tabelle bilden je Folge eine eigene Gruppe. Verschachtelte Tabellen
/// gehören nicht dazu.
fn rows<'a, N: Node<'a>>(table: N) -> Vec<(N, usize)> {
    fn walk<'a, N: Node<'a>>(
        n: N,
        group: &mut usize,
        loose: &mut bool,
        out: &mut Vec<(N, usize)>,
        foot: &mut Vec<N>,
    ) {
        for k in n.children() {
            if k.kind() != NodeKind::Element || is_table(k) {
                continue;
            }
            if is_row(k) {
                if !*loose {
                    *group += 1;
                    *loose = true;
                }
                out.push((k, *group));
            } else if is_row_group(k) {
                if k.is_element("tfoot") && explicit_role(k).is_none() {
                    foot.push(k);
                    continue;
                }
                *group += 1;
                *loose = false;
                let g = *group;
                out.extend(k.children().filter(|r| is_row(*r)).map(|r| (r, g)));
            } else if !is_header_cell(k) && !is_data_cell(k) {
                walk(k, group, loose, out, foot);
            }
        }
    }
    let (mut group, mut loose) = (0, false);
    let (mut out, mut foot) = (Vec::new(), Vec::new());
    walk(table, &mut group, &mut loose, &mut out, &mut foot);
    for f in foot {
        group += 1;
        out.extend(f.children().filter(|r| is_row(*r)).map(|r| (r, group)));
    }
    out
}

/// Die Zellen einer Zeile, ohne in Zellen, Zeilen oder Tabellen
/// hinabzusteigen.
fn row_cells<'a, N: Node<'a>>(row: N, out: &mut Vec<N>) {
    for k in row.children() {
        if k.kind() != NodeKind::Element || is_table(k) || is_row(k) {
            continue;
        }
        if is_header_cell(k) || is_data_cell(k) {
            out.push(k);
        } else {
            row_cells(k, out);
        }
    }
}

/// Ob eine Kopfzelle nichts zeigt: kein Text, kein Element darin. Leere
/// Zellen streicht HTML aus jeder Kopfzellenliste; die leere Ecke über den
/// Zeilenköpfen ist keine Beschriftung, die etwas beschriften müsste.
fn is_empty_cell<'a, N: Node<'a>>(n: N) -> bool {
    !n.children().any(|k| {
        k.kind() == NodeKind::Element || (k.kind() == NodeKind::Text && !k.text().trim().is_empty())
    })
}

/// Die nicht leeren Kopfzellen der Tabelle, die keiner Zelle zugeordnet sind —
/// nach HTML, „Forming relationships between data cells and header cells":
/// Eine Zelle mit `headers` bekommt genau die dort genannten Zellen, jede
/// andere die Spalten- und Zeilenköpfe, die die Suche nach oben und nach
/// links findet, und die Zeilengruppenköpfe ihrer Gruppe.
///
/// Spaltengruppenköpfe (`scope="colgroup"`) stehen nie in der Liste: Ohne
/// das Modell der `<colgroup>`-Elemente ist ihre Zuordnung nicht bestimmt,
/// und ein Verstoß dort wäre geraten.
fn unassigned<'a, N: Node<'a>>(table: N) -> Vec<N> {
    let rows = rows(table);
    let mut group_end: HashMap<usize, usize> = HashMap::new();
    for (y, (_, g)) in rows.iter().enumerate() {
        group_end.insert(*g, y + 1);
    }
    let mut slots: Vec<Slot<N>> = Vec::new();
    let mut grid: Vec<Vec<Option<usize>>> = Vec::new();
    for (y, (row, g)) in rows.iter().enumerate() {
        let mut cells = Vec::new();
        row_cells(*row, &mut cells);
        let mut x = 0;
        for c in cells {
            if grid.len() <= y {
                grid.resize(y + 1, Vec::new());
            }
            while grid[y].get(x).is_some_and(Option::is_some) {
                x += 1;
            }
            let w = span(c, "colspan", 1000).max(1);
            let h = match span(c, "rowspan", 65534) {
                0 => group_end[g].saturating_sub(y).max(1),
                h => h,
            };
            let i = slots.len();
            if grid.len() < y + h {
                grid.resize(y + h, Vec::new());
            }
            for line in &mut grid[y..y + h] {
                if line.len() < x + w {
                    line.resize(x + w, None);
                }
                for slot in &mut line[x..x + w] {
                    slot.get_or_insert(i);
                }
            }
            slots.push(Slot {
                node: c,
                x,
                y,
                w,
                h,
                header: is_header_cell(c),
                scope: header_scope(c),
                group: *g,
            });
            x += w;
        }
    }
    let at = |x: usize, y: usize| grid.get(y).and_then(|l| l.get(x)).copied().flatten();
    let data_in_rows = |y: usize, h: usize| {
        (y..y + h).any(|r| {
            grid.get(r)
                .is_some_and(|l| l.iter().flatten().any(|i| !slots[*i].header))
        })
    };
    let data_in_cols = |x: usize, w: usize| {
        grid.iter().any(|l| {
            l.iter()
                .skip(x)
                .take(w)
                .flatten()
                .any(|i| !slots[*i].header)
        })
    };
    let column_header: Vec<bool> = slots
        .iter()
        .map(|s| {
            s.header
                && (s.scope == HeaderScope::Col
                    || (s.scope == HeaderScope::Auto && !data_in_rows(s.y, s.h)))
        })
        .collect();
    let row_header: Vec<bool> = slots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.header
                && (s.scope == HeaderScope::Row
                    || (s.scope == HeaderScope::Auto
                        && !column_header[i]
                        && !data_in_cols(s.x, s.w)))
        })
        .collect();

    let mut assigned = vec![false; slots.len()];
    // HTML, „internal algorithm for scanning and assigning header cells".
    let scan = |p: usize, x0: usize, y0: usize, dx: bool, assigned: &mut Vec<bool>| {
        let mut opaque: Vec<usize> = Vec::new();
        let mut block: Vec<usize> = Vec::new();
        let mut in_block = slots[p].header;
        if in_block {
            block.push(p);
        }
        let (mut x, mut y) = (x0, y0);
        loop {
            if dx {
                let Some(nx) = x.checked_sub(1) else { break };
                x = nx;
            } else {
                let Some(ny) = y.checked_sub(1) else { break };
                y = ny;
            }
            let Some(c) = at(x, y) else { continue };
            if slots[c].header {
                in_block = true;
                block.push(c);
                let blocked = if dx {
                    opaque
                        .iter()
                        .any(|o| slots[*o].y == slots[c].y && slots[*o].h == slots[c].h)
                        || !row_header[c]
                } else {
                    opaque
                        .iter()
                        .any(|o| slots[*o].x == slots[c].x && slots[*o].w == slots[c].w)
                        || !column_header[c]
                };
                if !blocked && c != p {
                    assigned[c] = true;
                }
            } else if in_block {
                in_block = false;
                opaque.append(&mut block);
            }
        }
    };
    let mut ids: HashMap<&str, usize> = HashMap::new();
    for (i, s) in slots.iter().enumerate() {
        if let Some(id) = s.node.attr("id") {
            ids.entry(id).or_insert(i);
        }
    }
    for p in 0..slots.len() {
        let s = &slots[p];
        if let Some(headers) = s.node.attr("headers") {
            for id in headers.split_whitespace() {
                if let Some(&c) = ids.get(id)
                    && c != p
                {
                    assigned[c] = true;
                }
            }
            continue;
        }
        for y in s.y..s.y + s.h {
            scan(p, s.x, y, true, &mut assigned);
        }
        for x in s.x..s.x + s.w {
            scan(p, x, s.y, false, &mut assigned);
        }
        for (c, g) in slots.iter().enumerate() {
            if g.header
                && g.scope == HeaderScope::RowGroup
                && g.group == s.group
                && g.x < s.x + s.w
                && g.y < s.y + s.h
                && c != p
            {
                assigned[c] = true;
            }
        }
    }
    slots
        .iter()
        .zip(assigned)
        .filter(|(s, a)| {
            s.header && !a && s.scope != HeaderScope::ColGroup && !is_empty_cell(s.node)
        })
        .map(|(s, _)| s.node)
        .collect()
}

// --- tables/header-without-data -------------------------------------------

/// 1.3.1: Eine Kopfzelle, die keine Zelle beschriftet — in einer Tabelle
/// ganz ohne Datenzelle jede Kopfzelle, sonst jede, der das Tabellenmodell
/// von HTML keine Zelle zuordnet ([`unassigned`]; ACT d0f69e: eine Spalte
/// ohne Zelle darunter, ein `headers`, das die Spalte umgeht, ein Grid mit
/// kürzeren Zeilen). Eine Spalte aus Zeilenköpfen (`<th scope="row">`)
/// unter einem Spaltenkopf ist das übliche Muster „Tabelle mit zwei
/// Kopfrichtungen" (auditmysite#638, barrierlab.eu): Der Spaltenkopf ist
/// den Zeilenköpfen zugeordnet und besteht. Leere Datenzellen zählen mit.
///
/// Steht eine Zeilengruppe ohne Zeile in der Tabelle — die Zeilen werden noch
/// geladen oder sind ausgeblendet, etwa der Zeilenvorrat eines virtualisierten
/// Grids —, ist das nicht entscheidbar: `tables/data-undetermined` statt
/// `FAIL` (auditmysite#654). Beleg für den Verstoß: auditmysite-Korpus
/// `table_headers_no_data` und `table_grid_rows_unrendered` (`#header-only`).
pub(crate) fn header_without_data<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for t in elements(doc).filter(|n| is_table(*n)) {
        let z = cells(t);
        if z.head.is_empty() {
            continue;
        }
        let lonely = if z.data > 0 { unassigned(t) } else { z.head };
        if lonely.is_empty() {
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
        let text = if z.data > 0 {
            pick!(
                locale,
                "The header cell labels no cell: no cell in its column or row, and no headers \
                 attribute, is assigned to it.",
                "Die Kopfzelle beschriftet keine Zelle: Keine Zelle ihrer Spalte oder Zeile und \
                 kein headers-Attribut ist ihr zugeordnet.",
            )
        } else {
            pick!(
                locale,
                "The header cell has no data cells: the table contains no <td> at all.",
                "Die Kopfzelle hat keine Datenzellen: Die Tabelle enthält kein einziges <td>.",
            )
        };
        for k in lonely {
            out.push(
                Finding::fail("tables/header-without-data", text)
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
        // In einer Tabelle mit `role="none"`/`"presentation"` erben die
        // Zellen ohne eigene Rolle das: Sie sind keine Zellen mehr (WAI-ARIA
        // 1.2, `presentation`, „required owned elements"; ACT a25f45).
        if cell.attr("role").is_none()
            && ancestors(cell)
                .find(|a| a.is_element("table"))
                .is_some_and(crate::aria::presentation_applies)
        {
            continue;
        }
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
