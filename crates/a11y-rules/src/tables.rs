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

use a11y_dom::{Document, Node, NodeId, NodeKind, ancestors, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Eine Tabelle: `<table>` ohne fremde Rolle oder `table`, `grid`,
/// `treegrid` per `role`. Eine präsentationale Tabelle ist keine — ihre
/// Kopfzellen meldet `tables/presentational-with-headers`.
fn ist_tabelle<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => matches!(r, "table" | "grid" | "treegrid"),
        None => n.is_element("table"),
    }
}

/// Eine Kopfzelle: `<th>` oder `columnheader`, `rowheader` per `role`.
fn ist_kopfzelle<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => matches!(r, "columnheader" | "rowheader"),
        None => n.is_element("th"),
    }
}

/// Eine Datenzelle: `<td>` oder `cell`, `gridcell` per `role`.
fn ist_datenzelle<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => matches!(r, "cell" | "gridcell"),
        None => n.is_element("td"),
    }
}

fn ist_zeile<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => r == "row",
        None => n.is_element("tr"),
    }
}

fn ist_zeilengruppe<'a, N: Node<'a>>(n: N) -> bool {
    match explizite_rolle(n) {
        Some(r) => r == "rowgroup",
        None => matches!(n.local_name(), "thead" | "tbody" | "tfoot"),
    }
}

/// Was in einer Tabelle steht, ohne verschachtelte Tabellen.
struct Zellen<N> {
    kopf: Vec<N>,
    daten: usize,
    /// Eine Zeilengruppe ohne eine einzige Zeile: Die Zeilen stehen noch aus
    /// oder sind ausgeblendet.
    leere_gruppe: bool,
}

fn zellen<'a, N: Node<'a>>(tabelle: N) -> Zellen<N> {
    let mut z = Zellen {
        kopf: Vec::new(),
        daten: 0,
        leere_gruppe: false,
    };
    let mut stapel: Vec<N> = tabelle.children().collect();
    while let Some(n) = stapel.pop() {
        if n.kind() != NodeKind::Element || ist_tabelle(n) {
            continue;
        }
        if ist_kopfzelle(n) {
            z.kopf.push(n);
        } else if ist_datenzelle(n) {
            z.daten += 1;
        } else if ist_zeilengruppe(n) && !n.children().any(ist_zeile) {
            z.leere_gruppe = true;
        }
        stapel.extend(n.children());
    }
    z.kopf.sort_by(|a, b| a.document_order(*b));
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
    for t in elements(doc).filter(|n| ist_tabelle(*n)) {
        let z = zellen(t);
        if z.kopf.is_empty() || z.daten > 0 {
            continue;
        }
        if z.leere_gruppe {
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
        for k in z.kopf {
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
fn tabelle_von<'a, N: Node<'a>>(n: N) -> Option<N> {
    ancestors(n).find(|a| ist_tabelle(*a))
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
    let mut ids: HashMap<&str, D::N<'_>> = HashMap::new();
    for n in elements(doc) {
        if let Some(id) = n.attr("id") {
            ids.entry(id).or_insert(n);
        }
    }
    for zelle in elements(doc) {
        let Some(headers) = zelle.attr("headers") else {
            continue;
        };
        if !(ist_datenzelle(zelle) || ist_kopfzelle(zelle)) {
            continue;
        }
        let tabelle = tabelle_von(zelle);
        let ungueltig = headers.split_whitespace().find(|r| {
            ids.get(r).is_none_or(|k| {
                !ist_kopfzelle(*k)
                    || *k == zelle
                    || tabelle.is_some_and(|t| tabelle_von(*k) != Some(t))
            })
        });
        let text = match (headers.trim().is_empty(), ungueltig) {
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
                .at(at(zelle.id())),
        );
    }
}
