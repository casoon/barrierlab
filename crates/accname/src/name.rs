//! Der Namensalgorithmus nach
//! [Accessible Name and Description Computation 1.2](https://w3c.github.io/accname/),
//! Abschnitt 4.3.2.
//!
//! Die Schrittnummern im Code entsprechen denen der Spezifikation, damit sich
//! jeder Zweig gegen den Normtext prüfen lässt.

use std::collections::HashSet;

use a11y_dom::{ComputedStyle, Node, NodeId, NodeKind, Rendering, ancestors, descendants};

use crate::index::IdIndex;
use crate::role::{allows_name_from_content, name_is_prohibited, role};

/// Was gerade berechnet wird. Der Unterschied ist klein, aber er betrifft
/// Schritt 2B und 2C: Beim Namen gelten `aria-labelledby` und `aria-label`,
/// bei der Beschreibung `aria-describedby` und `title`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ziel {
    Name,
    Beschreibung,
}

/// Berechneter Stil je Element, soweit der Host ihn kennt (Tier 3). `None`,
/// wenn der Host keine Rendering-Daten hat — dann gilt das Verhalten ohne
/// Layout: jedes Kind wird abgesetzt, versteckt ist nur, was es per Attribut
/// ist.
type Stil<'s, N> = Option<&'s dyn Fn(N) -> Option<ComputedStyle>>;

struct Ctx<'i, 'a, N: Node<'a>> {
    ids: &'i IdIndex<'a, N>,
    stil: Stil<'i, N>,
    /// Knoten, die in dieser Berechnung schon besucht wurden. Verhindert
    /// Endlosschleifen über `aria-labelledby`-Ringe.
    besucht: HashSet<NodeId>,
    ziel: Ziel,
}

/// Berechnet den Accessible Name.
///
/// Gibt `None` zurück, wenn kein Name zustande kommt — das ist etwas anderes
/// als ein leerer Name.
pub fn name<'a, N: Node<'a>>(node: N, ids: &IdIndex<'a, N>) -> Option<String> {
    name_mit_stil(node, ids, None)
}

/// Berechnet den Accessible Name mit den berechneten Stilen des Hosts.
///
/// Zwei Stellen der Spezifikation hängen am Layout, und nur hier werden sie
/// entschieden statt angenähert:
///
/// - **Trenner im Inhaltsdurchlauf (2F):** Ein Kind mit `display: inline`
///   (oder `contents`) schließt direkt an, jedes andere wird durch Leerzeichen
///   abgesetzt — wie in WPT `accname/name/comp_name_from_content.html`.
///   Ersetzte Elemente (`img`, `svg`, `br`, …) bleiben abgesetzt.
/// - **Versteckt (2A):** `display: none` am Knoten oder einem Vorfahren und
///   `visibility: hidden`/`collapse` am Knoten.
///
/// Wo der Host für ein Element keinen Stil liefert, gilt für dieses Element
/// das Verhalten von [`name`].
pub fn name_rendered<'n, D: Rendering>(
    doc: &'n D,
    node: D::N<'n>,
    ids: &IdIndex<'n, D::N<'n>>,
) -> Option<String> {
    let stil = |n: D::N<'n>| doc.computed_style(n);
    name_mit_stil(node, ids, Some(&stil))
}

fn name_mit_stil<'a, N: Node<'a>>(
    node: N,
    ids: &IdIndex<'a, N>,
    stil: Stil<'_, N>,
) -> Option<String> {
    let mut ctx = Ctx {
        ids,
        stil,
        besucht: HashSet::new(),
        ziel: Ziel::Name,
    };
    let s = flatten(&berechne(&mut ctx, node, false, false));
    (!s.is_empty()).then_some(s)
}

/// Berechnet die Accessible Description.
pub fn description<'a, N: Node<'a>>(node: N, ids: &IdIndex<'a, N>) -> Option<String> {
    // Die Beschreibung folgt demselben Verfahren, nur mit aria-describedby als
    // Einstieg und title als letzter Rückfallebene.
    if let Some(v) = node.attr("aria-describedby") {
        let mut ctx = Ctx {
            ids,
            stil: None,
            besucht: HashSet::new(),
            ziel: Ziel::Beschreibung,
        };
        ctx.besucht.insert(node.id());
        let mut teile = Vec::new();
        for id in v.split_whitespace() {
            if let Some(ziel) = ids.get(id) {
                teile.push(berechne(&mut ctx, ziel, true, true));
            }
        }
        let s = flatten(&teile.join(" "));
        if !s.is_empty() {
            return Some(s);
        }
    }
    // title dient nur dann als Beschreibung, wenn es nicht schon den Namen
    // gestellt hat.
    let title = node.attr("title")?.trim();
    if title.is_empty() {
        return None;
    }
    let name_kommt_woanders_her = node.has_attr("aria-label")
        || node.has_attr("aria-labelledby")
        || natives_label(node, ids, None).is_some();
    name_kommt_woanders_her.then(|| title.to_string())
}

/// Der Kern der Berechnung.
///
/// `rekursion` = der Aufruf kommt aus Schritt 2F (Inhaltsdurchlauf),
/// `via_verweis` = der Knoten wurde über `aria-labelledby`/`aria-describedby`
/// angesprungen. Beide Flaggen ändern das Verhalten der Spezifikation an
/// mehreren Stellen.
fn berechne<'a, N: Node<'a>>(
    ctx: &mut Ctx<'_, 'a, N>,
    node: N,
    rekursion: bool,
    via_verweis: bool,
) -> String {
    // Schritt 1: Zyklenschutz. Nicht in der Spezifikation als eigener Schritt,
    // aber sie verlangt an mehreren Stellen, bereits besuchte Knoten zu
    // überspringen.
    if !ctx.besucht.insert(node.id()) {
        return String::new();
    }

    if node.kind() == NodeKind::Text {
        // Schritt 2G
        return node.text().to_string();
    }

    let rolle = role(node);

    // Schritt 2A: versteckte Knoten zählen nicht mit — es sei denn, sie werden
    // ausdrücklich per IDREF herangezogen.
    if !via_verweis && (ist_versteckt(node) || per_stil_versteckt(ctx.stil, node, !rekursion)) {
        return String::new();
    }

    // Schritt 2B: aria-labelledby. Gilt nur an der Wurzel der Berechnung, nicht
    // innerhalb einer bereits laufenden Verweisauflösung.
    if ctx.ziel == Ziel::Name && !via_verweis {
        if let Some(v) = node.attr("aria-labelledby") {
            let mut teile = Vec::new();
            for id in v.split_whitespace() {
                if let Some(ziel) = ctx.ids.get(id) {
                    teile.push(berechne(ctx, ziel, true, true));
                }
            }
            let s = teile.join(" ");
            if !s.trim().is_empty() {
                return s;
            }
        }
    }

    // Für Rollen mit Namensverbot bleiben 2C und 2D außen vor; ihr Inhalt zählt
    // bei einer Rekursion aber weiterhin mit.
    let verboten = rolle.is_some_and(name_is_prohibited);

    // Schritt 2C: aria-label.
    if !verboten && ctx.ziel == Ziel::Name {
        if let Some(label) = node.attr("aria-label") {
            let label = label.trim();
            // Bei einem eingebetteten Steuerelement innerhalb einer Rekursion
            // gewinnt dessen Wert, nicht sein Label.
            if !label.is_empty() && !(rekursion && ist_eingebettetes_steuerelement(rolle)) {
                return label.to_string();
            }
        }
    }

    // Schritt 2D: die Textalternative aus dem HTML selbst.
    if !verboten {
        if let Some(s) = natives_label(node, ctx.ids, ctx.stil) {
            if !s.trim().is_empty() {
                return s;
            }
        }
    }

    // Schritt 2E: eingebettetes Steuerelement innerhalb einer Rekursion —
    // hier zählt der Wert, nicht die Beschriftung.
    if rekursion {
        if let Some(wert) = eingebetteter_wert(node, rolle) {
            return wert;
        }
    }

    // Schritt 2F: Name aus dem Inhalt. Erlaubt, wenn die Rolle es vorsieht,
    // wenn der Knoten über einen Verweis angesprungen wurde, oder wenn wir
    // ohnehin schon im Inhaltsdurchlauf sind.
    let aus_inhalt = rolle.is_some_and(allows_name_from_content) || via_verweis || rekursion;
    if aus_inhalt {
        let mut s = String::new();
        for kind in node.children() {
            let teil = berechne(ctx, kind, true, false);
            anhaengen(&mut s, &teil, abgesetzt(ctx.stil, kind));
            // Ein Zeilenumbruch trennt, obwohl er selbst nichts beiträgt.
            if kind.is_element("br") {
                s.push(' ');
            }
        }
        if !s.trim().is_empty() {
            return s;
        }
    }

    // Schritt 2I: title als letzte Rückfallebene.
    if let Some(t) = node.attr("title") {
        if !t.trim().is_empty() {
            return t.to_string();
        }
    }

    String::new()
}

/// Ob ein Kind im Inhaltsdurchlauf durch Leerzeichen abgesetzt wird. Ohne Stil
/// jedes, auch Text — Inline-Elemente lassen sich ohne Layout nicht erkennen,
/// und eine Entscheidung nach dem Tag traf echte Seiten falsch (0.11.1).
fn abgesetzt<'a, N: Node<'a>>(stil: Stil<'_, N>, kind: N) -> bool {
    let Some(stil) = stil else {
        return true;
    };
    if kind.kind() == NodeKind::Text {
        return false;
    }
    if ist_ersetztes_element(kind.local_name()) {
        return true;
    }
    match stil(kind).and_then(|s| s.display) {
        Some(display) => !matches!(display.as_str(), "inline" | "contents"),
        None => true,
    }
}

/// Ersetzte Elemente sind im Stil oft `inline`, werden aber wie ein eigener
/// Block benannt.
fn ist_ersetztes_element(tag: &str) -> bool {
    matches!(
        tag,
        "img" | "svg" | "canvas" | "video" | "audio" | "iframe" | "object" | "embed"
    )
}

fn anhaengen(ziel: &mut String, teil: &str, abgesetzt: bool) {
    if teil.is_empty() {
        return;
    }
    if abgesetzt {
        ziel.push(' ');
        ziel.push_str(teil);
        ziel.push(' ');
    } else {
        ziel.push_str(teil);
    }
}

/// Versteckt per berechnetem Stil. `display: none` vererbt sich nicht als
/// Wert, nur als Wirkung — deshalb zählen die Vorfahren, allerdings nur an der
/// Wurzel der Berechnung (`mit_vorfahren`): im Inhaltsdurchlauf wäre ein
/// versteckter Vorfahre schon selbst ausgeschieden.
fn per_stil_versteckt<'a, N: Node<'a>>(stil: Stil<'_, N>, node: N, mit_vorfahren: bool) -> bool {
    let Some(stil) = stil else {
        return false;
    };
    let eigen = stil(node);
    let unsichtbar = eigen
        .as_ref()
        .and_then(|s| s.visibility.as_deref())
        .is_some_and(|v| matches!(v, "hidden" | "collapse"));
    let ohne_anzeige =
        |s: Option<ComputedStyle>| s.and_then(|s| s.display).as_deref() == Some("none");
    unsichtbar
        || ohne_anzeige(eigen)
        || (mit_vorfahren && ancestors(node).any(|a| ohne_anzeige(stil(a))))
}

/// Ohne Rendering-Daten ist nur die ausdrückliche Verstecktheit erkennbar:
/// `aria-hidden="true"` und das `hidden`-Attribut. `display: none` und
/// `visibility: hidden` brauchen Tier 3 und bleiben hier unsichtbar.
fn ist_versteckt<'a, N: Node<'a>>(node: N) -> bool {
    if node.attr("aria-hidden") == Some("true") || node.has_attr("hidden") {
        return true;
    }
    ancestors(node).any(|a| a.attr("aria-hidden") == Some("true") || a.has_attr("hidden"))
}

fn ist_eingebettetes_steuerelement(rolle: Option<&str>) -> bool {
    matches!(
        rolle,
        Some(
            "textbox"
                | "searchbox"
                | "combobox"
                | "listbox"
                | "slider"
                | "spinbutton"
                | "progressbar"
                | "scrollbar"
                | "checkbox"
                | "radio"
                | "switch"
        )
    )
}

/// Schritt 2E: der Wert eines eingebetteten Steuerelements.
fn eingebetteter_wert<'a, N: Node<'a>>(node: N, rolle: Option<&str>) -> Option<String> {
    match rolle? {
        "textbox" | "searchbox" => node
            .attr("value")
            .map(str::to_string)
            .or_else(|| Some(subtree_plain(node))),
        "combobox" | "listbox" => {
            // Die ausgewählte Option; ohne Auswahl die erste.
            let gewaehlt = descendants(node)
                .filter(|d| d.is_element("option"))
                .find(|d| d.has_attr("selected"))
                .or_else(|| descendants(node).find(|d| d.is_element("option")))?;
            Some(subtree_plain(gewaehlt))
        }
        "slider" | "spinbutton" | "progressbar" | "scrollbar" => node
            .attr("aria-valuetext")
            .or_else(|| node.attr("aria-valuenow"))
            .or_else(|| node.attr("value"))
            .map(str::to_string),
        _ => None,
    }
}

/// Schritt 2D: die vom HTML selbst gestellte Textalternative, nach
/// [HTML-AAM](https://www.w3.org/TR/html-aam-1.0/).
fn natives_label<'a, N: Node<'a>>(
    node: N,
    ids: &IdIndex<'a, N>,
    stil: Stil<'_, N>,
) -> Option<String> {
    // Beschriftende Elemente: mit Stil nach denselben Trennregeln wie 2F.
    let beschriftung = |n: N| match stil {
        Some(_) => teilbaum_text(stil, n, None),
        None => subtree_plain(n),
    };
    let tag = node.local_name();
    match tag {
        "img" | "area" => node.attr("alt").map(str::to_string),
        "input" => {
            let ty = node
                .attr("type")
                .map(|t| t.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "text".into());
            match ty.as_str() {
                "button" => node.attr("value").map(str::to_string),
                "submit" => Some(node.attr("value").unwrap_or("Submit").to_string()),
                "reset" => Some(node.attr("value").unwrap_or("Reset").to_string()),
                "image" => node
                    .attr("alt")
                    .map(str::to_string)
                    .or_else(|| label_elemente(node, ids, stil))
                    .or_else(|| Some("Submit Query".to_string())),
                _ => label_elemente(node, ids, stil).or_else(|| {
                    // placeholder ist die letzte Rückfallebene vor title.
                    node.attr("placeholder").map(str::to_string)
                }),
            }
        }
        "select" | "textarea" | "meter" | "progress" | "output" => label_elemente(node, ids, stil),
        "fieldset" => erstes_kind_mit_tag(node, "legend").map(beschriftung),
        "figure" => descendants(node)
            .find(|d| d.is_element("figcaption"))
            .map(beschriftung),
        "table" => erstes_kind_mit_tag(node, "caption").map(beschriftung),
        "svg" => erstes_kind_mit_tag(node, "title").map(subtree_plain),
        "iframe" => node.attr("title").map(str::to_string),
        _ => None,
    }
}

fn erstes_kind_mit_tag<'a, N: Node<'a>>(node: N, tag: &str) -> Option<N> {
    node.children().find(|c| c.is_element(tag))
}

/// Die `<label>`-Elemente eines Formularelements: umschließend oder per `for`.
fn label_elemente<'a, N: Node<'a>>(
    node: N,
    ids: &IdIndex<'a, N>,
    stil: Stil<'_, N>,
) -> Option<String> {
    let mut teile: Vec<String> = Vec::new();

    if let Some(id) = node.attr("id") {
        for &l in ids.labels_for(id) {
            let t = teilbaum_text(stil, l, Some(node));
            if !t.trim().is_empty() {
                teile.push(t);
            }
        }
    }
    if teile.is_empty() {
        if let Some(l) = a11y_dom::closest(node, "label") {
            let t = teilbaum_text(stil, l, Some(node));
            if !t.trim().is_empty() {
                teile.push(t);
            }
        }
    }

    (!teile.is_empty()).then(|| teile.join(" "))
}

/// Text eines beschriftenden Teilbaums, ohne den Teilbaum von `aussparen` —
/// damit der Wert des beschrifteten Feldes nicht in sein eigenes Label
/// zurückfließt. Trenner nach [`abgesetzt`]; mit Stil fallen per Stil
/// versteckte Teilbäume weg.
fn teilbaum_text<'a, N: Node<'a>>(stil: Stil<'_, N>, wurzel: N, aussparen: Option<N>) -> String {
    fn sammle<'a, N: Node<'a>>(stil: Stil<'_, N>, node: N, aussparen: Option<N>, out: &mut String) {
        for kind in node.children() {
            if Some(kind) == aussparen {
                continue;
            }
            let teil = match kind.kind() {
                NodeKind::Text => kind.text().to_string(),
                _ if per_stil_versteckt(stil, kind, false) => continue,
                _ => {
                    let mut innen = String::new();
                    sammle(stil, kind, aussparen, &mut innen);
                    innen
                }
            };
            anhaengen(out, &teil, abgesetzt(stil, kind));
            if kind.is_element("br") {
                out.push(' ');
            }
        }
    }
    let mut out = String::new();
    sammle(stil, wurzel, aussparen, &mut out);
    flatten(&out)
}

fn subtree_plain<'a, N: Node<'a>>(node: N) -> String {
    flatten(&a11y_dom::subtree_text(node))
}

/// Die Spezifikation verlangt einen „flat string": Zeilenumbrüche und
/// Mehrfach-Leerzeichen werden zu einem Leerzeichen, Ränder fallen weg.
pub(crate) fn flatten(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
