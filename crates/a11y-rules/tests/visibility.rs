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
struct WithSemantics<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl<'a> WithSemantics<'a> {
    fn new(doc: &'a Arena) -> Self {
        WithSemantics {
            ids: IdIndex::build(doc.root()),
            doc,
        }
    }
}

impl Document for WithSemantics<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for WithSemantics<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }
}

/// Wie [`WithSemantics`], dazu Stile: `display` aus `data-display`, sonst
/// `block`. Die Namensberechnung läuft mit Stil.
struct WithStyle<'a>(WithSemantics<'a>);

impl Document for WithStyle<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

impl Semantics for WithStyle<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        self.0.role(node)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name_rendered(self, node, &self.0.ids)
    }
}

impl Rendering for WithStyle<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: None,
            font_weight: None,
            display: Some(node.attr("data-display").unwrap_or("block").to_string()),
            visibility: Some("visible".to_string()),
            ..Default::default()
        })
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }
}

fn count(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

/// Ein Dokument mit allem, was die dokumentweiten Regeln verlangen, und einem
/// offenen `<body>`: Der Test hängt seinen Inhalt an und schließt zweimal.
fn page() -> ArenaBuilder {
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
fn hidden_banner_yields_no_name_findings() {
    let arena = page()
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
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "buttons/name-missing"), 1, "{:?}", r.findings);
    assert_eq!(count(&r, "links/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn nested_svg_under_aria_hidden_needs_no_name() {
    let arena = page()
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
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "svg/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn icon_in_named_link_needs_no_name_of_its_own() {
    let arena = page()
        .open("a")
        .attr("href", "/t")
        .attr("aria-label", "Twitter")
        .open("svg")
        .open("path")
        .close()
        .close()
        .close()
        // Zur Kontrolle: Das freistehende SVG fällt weiter auf.
        .open("svg")
        .open("path")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "svg/name-missing"), 1, "{:?}", r.findings);
}

/// geographia.eu: 22 Karten-Links in einem geschlossenen `details.more`.
/// Chrome stellt sie nicht dar und führt sie nicht im Accessibility-Tree;
/// die berechneten Stile halten sie trotzdem für sichtbar.
#[test]
fn content_of_closed_details_is_not_rendered() {
    let arena = page()
        .open("details")
        .open("summary")
        .text("Mehr Fragen")
        .close()
        .open("a")
        .attr("href", "/x")
        .close()
        .close()
        // Geöffnet ist der Inhalt da, der leere Link fällt auf.
        .open("details")
        .attr("open", "")
        .open("summary")
        .text("Offen")
        .close()
        .open("a")
        .attr("href", "/y")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "links/name-missing"), 1, "{:?}", r.findings);
}

#[test]
fn empty_hidden_list_is_no_finding() {
    let arena = page()
        .open("div")
        .attr("hidden", "")
        .open("ol")
        .attr("aria-label", "Transkript")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(count(&run(&arena), "lists/empty"), 0);
}

#[test]
fn hidden_by_style_counts_with_rendering() {
    // `display: none` ist nur mit Stilen erkennbar. Mit ihnen fällt der
    // leere Button weg, ohne sie bleibt er stehen — die Strukturregel sieht
    // ihn dann, und das ist die ehrliche Grenze des statischen Falls.
    let arena = page()
        .open("div")
        .attr("data-display", "none")
        .open("button")
        .close()
        .close()
        .close()
        .close()
        .build();
    let without = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&without, "buttons/name-missing"), 1);
    let with = run_full(&WithStyle(WithSemantics::new(&arena)));
    assert_eq!(
        count(&with, "buttons/name-missing"),
        0,
        "{:?}",
        with.findings
    );
}

#[test]
fn references_to_hidden_ids_stay_valid() {
    // Ein Name aus verstecktem Text ist per aria-labelledby ausdrücklich
    // erlaubt. Die Verweisprüfung sieht deshalb das ganze Markup.
    let arena = page()
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
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "aria/reference-missing"), 0, "{:?}", r.findings);
    assert_eq!(count(&r, "buttons/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn unrendered_label_does_not_count_as_label() {
    let arena = page()
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
    let r = run_full(&WithStyle(WithSemantics::new(&arena)));
    assert_eq!(count(&r, "forms/label-missing"), 1, "{:?}", r.findings);

    // Ohne Stil ist das Label nicht als versteckt erkennbar.
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(count(&r, "forms/label-missing"), 0);
}

#[test]
fn focusable_under_aria_hidden_ancestor_is_flagged() {
    let arena = page()
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
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(
        count(&r, "keyboard/hidden-focusable"),
        2,
        "{:?}",
        r.findings
    );
    // Der Befund gehört zu hidden-focusable, nicht zu den Namensregeln.
    assert_eq!(count(&r, "links/name-missing"), 0, "{:?}", r.findings);
}

#[test]
fn unrendered_focusable_under_aria_hidden_is_no_finding() {
    // Ein ausgeblendetes Menü mit aria-hidden: nicht fokussierbar, weil nicht
    // dargestellt.
    let arena = page()
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
    assert_eq!(count(&run(&arena), "keyboard/hidden-focusable"), 0);
}

// --- Checkliste (liveaudit#7) ---------------------------------------------

fn manual(r: &Report) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.rule_id.starts_with("manual/"))
        .map(|f| f.rule_id.as_str())
        .collect()
}

#[test]
fn hover_focus_content_item_needs_a_trigger() {
    let plain = page()
        .open("p")
        .text("Text")
        .close()
        .close()
        .close()
        .build();
    assert!(!manual(&run(&plain)).contains(&"manual/content-on-hover-focus"));
    assert!(manual(&run(&plain)).contains(&"manual/text-spacing"));

    let tip = page()
        .open("span")
        .attr("role", "tooltip")
        .text("Hinweis")
        .close()
        .close()
        .close()
        .build();
    assert!(manual(&run(&tip)).contains(&"manual/content-on-hover-focus"));
}

#[test]
fn checklist_appears_only_with_trigger() {
    let empty = page().close().close().build();
    assert!(manual(&run(&empty)).is_empty());

    let arena = page()
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
    let mut points = manual(&r);
    points.sort_unstable();
    assert_eq!(
        points,
        [
            "manual/authentication",
            "manual/consistent-help",
            "manual/consistent-identification",
            "manual/error-handling",
            "manual/error-prevention",
            "manual/help",
            "manual/image-alternatives",
            "manual/keyboard-trap",
            "manual/media-alternatives",
            "manual/no-timing",
            "manual/re-authenticating",
            "manual/sensory-characteristics",
            "manual/text-spacing",
            "manual/timing",
            "manual/unusual-words",
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
fn gestures_interruptions_and_motion_only_with_trigger() {
    let arena = page()
        .open("div")
        .attr("class", "swiper-container")
        .close()
        .open("div")
        .attr("aria-live", "polite")
        .close()
        .open("script")
        .text("addEventListener('devicemotion', shake)")
        .close()
        .close()
        .close()
        .build();
    let r = run(&arena);
    let mut points = manual(&r);
    points.sort_unstable();
    assert_eq!(
        points,
        [
            "manual/dragging-movements",
            "manual/interruptions",
            "manual/keyboard-trap",
            "manual/motion-actuation",
            "manual/pointer-gestures",
        ]
    );

    let still = page()
        .open("div")
        .attr("class", "swipe")
        .close()
        .open("div")
        .attr("aria-live", "off")
        .close()
        .open("script")
        .text("console.log(1)")
        .close()
        .close()
        .close()
        .build();
    assert!(manual(&run(&still)).is_empty());
}

#[test]
fn hidden_media_set_no_point() {
    let arena = page()
        .open("div")
        .attr("hidden", "")
        .open("video")
        .attr("src", "a.mp4")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!manual(&run(&arena)).contains(&"manual/media-alternatives"));
}
