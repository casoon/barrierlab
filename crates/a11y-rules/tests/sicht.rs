//! Der Geltungsbereich: Was versteckt ist, prüft keine Regel, der es nicht um
//! das Verstecken selbst geht.
//!
//! Die Fälle stammen aus der Messung von LiveAudit gegen barrierlab.eu am
//! 29.09.2026 (liveaudit#1, #3, #4): ein Banner hinter `hidden`, ein
//! verschachteltes SVG unter `aria-hidden`, eine leere versteckte Liste, per
//! `display: none` ausgeblendete Labels.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, ComputedStyle, Document, Node, Rect, Rendering, Semantics,
};
use a11y_report::Report;
use a11y_rules::{run, run_full, run_with_semantics};
use accname::IdIndex;

/// Tier 2 über der Arena, mit echter Namensberechnung und ohne eigenes
/// `is_ignored` — wie der WASM-Host. Die Sicht darf sich nicht darauf
/// verlassen, dass ein Host versteckte Knoten selbst meldet.
struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl<'a> MitSemantik<'a> {
    fn new(doc: &'a Arena) -> Self {
        MitSemantik {
            ids: IdIndex::build(doc.root()),
            doc,
        }
    }
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }
}

/// Wie [`MitSemantik`], dazu Stile: `display` aus `data-display`, sonst
/// `block`. Die Namensberechnung läuft mit Stil.
struct MitStil<'a>(MitSemantik<'a>);

impl Document for MitStil<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

impl Semantics for MitStil<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        self.0.role(node)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name_rendered(self, node, &self.0.ids)
    }
}

impl Rendering for MitStil<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: None,
            font_weight: None,
            display: Some(node.attr("data-display").unwrap_or("block").to_string()),
            visibility: Some("visible".to_string()),
        })
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }
}

fn anzahl(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

/// Ein Dokument mit allem, was die dokumentweiten Regeln verlangen, und einem
/// offenen `<body>`: Der Test hängt seinen Inhalt an und schließt zweimal.
fn seite() -> ArenaBuilder {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .open("body")
}

#[test]
fn versteckter_banner_erzeugt_keine_namensbefunde() {
    let arena = seite()
        .open("div")
        .attr("hidden", "")
        .open("button")
        .text("Weiter")
        .close()
        .open("a")
        .attr("href", "/x")
        .text("Mehr dazu")
        .close()
        .close()
        // Zur Kontrolle: Ein sichtbarer Button ohne Namen fällt weiter auf.
        .open("button")
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(anzahl(&r, "buttons/name-missing"), 1, "{:?}", r.findings);
    assert_eq!(anzahl(&r, "links/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn verschachteltes_svg_unter_aria_hidden_braucht_keinen_namen() {
    let arena = seite()
        .open("svg")
        .attr("aria-hidden", "true")
        .open("svg")
        .open("rect")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(anzahl(&r, "svg/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn leere_versteckte_liste_ist_kein_befund() {
    let arena = seite()
        .open("div")
        .attr("hidden", "")
        .open("ol")
        .attr("aria-label", "Transkript")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(anzahl(&run(&arena), "lists/empty"), 0);
}

#[test]
fn per_stil_verstecktes_zaehlt_mit_darstellung() {
    // `display: none` ist nur mit Stilen erkennbar. Mit ihnen fällt der
    // leere Button weg, ohne sie bleibt er stehen — die Strukturregel sieht
    // ihn dann, und das ist die ehrliche Grenze des statischen Falls.
    let arena = seite()
        .open("div")
        .attr("data-display", "none")
        .open("button")
        .close()
        .close()
        .close()
        .close()
        .build();
    let ohne = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(anzahl(&ohne, "buttons/name-missing"), 1);
    let mit = run_full(&MitStil(MitSemantik::new(&arena)));
    assert_eq!(
        anzahl(&mit, "buttons/name-missing"),
        0,
        "{:?}",
        mit.findings
    );
}

#[test]
fn verweise_auf_versteckte_ids_bleiben_gueltig() {
    // Ein Name aus verstecktem Text ist per aria-labelledby ausdrücklich
    // erlaubt. Die Verweisprüfung sieht deshalb das ganze Markup.
    let arena = seite()
        .open("span")
        .attr("id", "l")
        .attr("hidden", "")
        .text("Suche")
        .close()
        .open("button")
        .attr("aria-labelledby", "l")
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(anzahl(&r, "aria/reference-missing"), 0, "{:?}", r.findings);
    assert_eq!(anzahl(&r, "buttons/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn nicht_dargestelltes_label_zaehlt_nicht_als_label() {
    let arena = seite()
        .open("label")
        .attr("for", "e")
        .attr("data-display", "none")
        .text("E-Mail")
        .close()
        .open("input")
        .attr("id", "e")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&MitStil(MitSemantik::new(&arena)));
    assert_eq!(anzahl(&r, "forms/label-missing"), 1, "{:?}", r.findings);

    // Ohne Stil ist das Label nicht als versteckt erkennbar.
    let r = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(anzahl(&r, "forms/label-missing"), 0);
}

#[test]
fn fokussierbares_unter_aria_hidden_vorfahren_faellt_auf() {
    let arena = seite()
        .open("div")
        .attr("aria-hidden", "true")
        .open("a")
        .attr("href", "/x")
        .text("Eins")
        .close()
        .open("button")
        .text("Zwei")
        .close()
        // Richtig gebaut: aus der Tabfolge genommen, ohne Ziel, oder inert.
        .open("a")
        .attr("href", "/y")
        .attr("tabindex", "-1")
        .text("Drei")
        .close()
        .open("a")
        .text("Anker ohne Ziel")
        .close()
        .open("div")
        .attr("inert", "")
        .open("a")
        .attr("href", "/z")
        .text("Vier")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&MitSemantik::new(&arena));
    assert_eq!(
        anzahl(&r, "keyboard/hidden-focusable"),
        2,
        "{:?}",
        r.findings
    );
    // Der Befund gehört zu hidden-focusable, nicht zu den Namensregeln.
    assert_eq!(anzahl(&r, "links/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn nicht_dargestelltes_fokussierbares_unter_aria_hidden_ist_kein_befund() {
    // Ein ausgeblendetes Menü mit aria-hidden: nicht fokussierbar, weil nicht
    // dargestellt.
    let arena = seite()
        .open("nav")
        .attr("aria-hidden", "true")
        .attr("hidden", "")
        .open("a")
        .attr("href", "/x")
        .text("Eins")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(anzahl(&run(&arena), "keyboard/hidden-focusable"), 0);
}

// --- Checkliste (liveaudit#7) ---------------------------------------------

fn manuell(r: &Report) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.rule_id.starts_with("manual/"))
        .map(|f| f.rule_id.as_str())
        .collect()
}

#[test]
fn checkliste_erscheint_nur_mit_anlass() {
    let leer = seite().close().close().build();
    assert!(manuell(&run(&leer)).is_empty());

    let arena = seite()
        .open("video")
        .attr("src", "a.mp4")
        .close()
        .open("img")
        .attr("src", "a.png")
        .attr("alt", "")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "")
        .close()
        .open("form")
        .open("label")
        .text("Passwort")
        .open("input")
        .attr("type", "password")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&arena);
    let mut punkte = manuell(&r);
    punkte.sort_unstable();
    assert_eq!(
        punkte,
        [
            "manual/authentication",
            "manual/error-handling",
            "manual/image-alternatives",
            "manual/media-alternatives",
            "manual/timing",
            "manual/use-of-color",
            "manual/visual-structure",
        ],
        "je Kriterium genau einmal, auch bei zwei Bildern"
    );
    assert!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("manual/"))
            .all(|f| f.outcome == a11y_report::Outcome::Untested
                && f.location.node.as_deref() == Some("0"))
    );
}

#[test]
fn versteckte_medien_setzen_keinen_punkt() {
    let arena = seite()
        .open("div")
        .attr("hidden", "")
        .open("video")
        .attr("src", "a.mp4")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!manuell(&run(&arena)).contains(&"manual/media-alternatives"));
}
