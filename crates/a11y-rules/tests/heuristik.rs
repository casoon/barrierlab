//! Die heuristischen Tier-3-Regeln (liveaudit#6): Jede meldet `REVIEW`, nie
//! `FAIL`, und ohne Layout-Daten des Hosts nichts — außer dem ungemessenen
//! Fokus, der als `UNTESTED` stehen bleibt.
//!
//! Der Test-Host liest Layout und Geometrie aus `data-*`-Attributen.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, ComputedStyle, Document, Layout, Node, Rect, Rendering,
    Semantics,
};
use a11y_report::{Outcome, Report};
use a11y_rules::run_full;
use accname::IdIndex;

struct Host<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    /// Ob der Host überhaupt Layout liefert.
    layout: bool,
}

impl<'a> Host<'a> {
    fn new(doc: &'a Arena) -> Self {
        Host {
            ids: IdIndex::build(doc.root()),
            doc,
            layout: true,
        }
    }
}

impl Document for Host<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for Host<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }
}

impl Rendering for Host<'_> {
    /// Wie Chrome: `<area>` und `<audio>` ohne `controls` haben aus dem
    /// UA-Stylesheet `display: none`.
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        let ua_none =
            node.is_element("area") || (node.is_element("audio") && !node.has_attr("controls"));
        Some(ComputedStyle {
            color: None,
            background_color: None,
            font_size_px: None,
            font_weight: None,
            display: Some(if ua_none { "none" } else { "block" }.into()),
            visibility: Some("visible".into()),
            ..Default::default()
        })
    }

    /// `data-size="BxH"` oder `"BxH@X,Y"`; ohne Position bei (0, 0).
    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect> {
        let wert = node.attr("data-size")?;
        let (groesse, ort) = wert.split_once('@').unwrap_or((wert, "0,0"));
        let (w, h) = groesse.split_once('x')?;
        let (x, y) = ort.split_once(',')?;
        Some(Rect {
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            width: w.parse().ok()?,
            height: h.parse().ok()?,
        })
    }

    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        if !self.layout {
            return None;
        }
        Some(Layout {
            flex_reversed: Some(node.has_attr("data-reversed")),
            order: Some(node.attr("data-order").map_or(0, |o| o.parse().unwrap())),
            min_width_px: Some(
                node.attr("data-min-width")
                    .map_or(0.0, |o| o.parse().unwrap()),
            ),
            cursor_pointer: Some(node.has_attr("data-pointer")),
            infinite_animation: Some(node.has_attr("data-endlos")),
            obscured: Some(node.has_attr("data-verdeckt")),
            hides_focus: Some(node.has_attr("data-leiste")),
            focus_visible: node.attr("data-fokus").map(|f| f == "sichtbar"),
            pointer_events_none: Some(node.has_attr("data-no-pointer")),
            animating: Some(node.has_attr("data-waechst")),
        })
    }
}

fn seite() -> ArenaBuilder {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .close()
        .open("body")
}

fn befunde<'r>(r: &'r Report, id: &str) -> Vec<&'r a11y_report::Finding> {
    r.findings.iter().filter(|f| f.rule_id == id).collect()
}

fn nur_review(r: &Report, id: &str) -> usize {
    let b = befunde(r, id);
    assert!(
        b.iter().all(|f| f.outcome == Outcome::Review),
        "{id}: {b:?}"
    );
    b.len()
}

#[test]
fn umgekehrte_reihenfolge_ueber_bedienbarem_inhalt() {
    let arena = seite()
        .open("div")
        .attr("data-reversed", "")
        .open("a")
        .attr("href", "/1")
        .text("Eins")
        .close()
        .open("a")
        .attr("href", "/2")
        .text("Zwei")
        .close()
        .close()
        // Umgekehrt, ohne Bedienbares, aber mit Text: zählt für 1.3.2, die
        // Lesereihenfolge (auditmysite-Korpus `text_and_layout`). Bis 0.21.0
        // galt das als harmlos.
        .open("div")
        .attr("data-reversed", "")
        .open("p")
        .text("a")
        .close()
        .open("p")
        .text("b")
        .close()
        .close()
        .open("div")
        .open("button")
        .attr("data-order", "-1")
        .text("Vor")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "order/visual-mismatch"),
        3
    );
}

#[test]
fn endlos_animation_nur_am_aeussersten_element() {
    let arena = seite()
        .open("div")
        .attr("data-endlos", "")
        .open("span")
        .attr("data-endlos", "")
        .text("Laufschrift")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        1
    );
}

#[test]
fn min_width_ueber_320_ausser_tabellen() {
    let arena = seite()
        .open("div")
        .attr("data-min-width", "480")
        .text("Breit")
        .close()
        .open("div")
        .attr("data-min-width", "300")
        .text("Schmal genug")
        .close()
        .open("table")
        .attr("data-min-width", "900")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(nur_review(&r, "reflow/min-width"), 1);
    assert!(befunde(&r, "reflow/min-width")[0].message.contains("480"));
}

#[test]
fn anklickbares_div_ohne_rolle() {
    let arena = seite()
        // Das „Suchen" auf barrierlab.eu: sieht aus wie ein Knopf.
        .open("div")
        .attr("data-pointer", "")
        .open("span")
        .attr("data-pointer", "")
        .text("Suchen")
        .close()
        .close()
        .open("div")
        .attr("onclick", "ablehnen()")
        .text("Ablehnen")
        .close()
        // Richtig gebaut und Zeiger darin: kein Befund.
        .open("button")
        .open("span")
        .attr("data-pointer", "")
        .text("Senden")
        .close()
        .close()
        .open("div")
        .attr("role", "button")
        .attr("tabindex", "0")
        .attr("data-pointer", "")
        .text("Ok")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    // Das „Ablehnen" mit Inline-onclick meldet die Tier-1-Regel, nicht diese.
    assert_eq!(nur_review(&r, "keyboard/pointer-only"), 1);
    assert_eq!(befunde(&r, "keyboard/click-handler-not-focusable").len(), 1);
}

#[test]
fn verdecktes_bedienelement() {
    let arena = seite()
        .open("a")
        .attr("href", "/x")
        .attr("data-verdeckt", "")
        .text("Unter der Leiste")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "focus/obscured"),
        1
    );
}

#[test]
fn leiste_ohne_scroll_padding() {
    let arena = seite()
        .open("header")
        .attr("data-leiste", "")
        .text("Leiste")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(nur_review(&r, "focus/obscured"), 1);
    assert!(
        befunde(&r, "focus/obscured")[0]
            .message
            .contains("scroll-padding-top")
    );
}

#[test]
fn zu_kleine_ziele_ausser_im_fliesstext() {
    let arena = seite()
        .open("button")
        .attr("data-size", "16x16")
        .text("x")
        .close()
        .open("button")
        .attr("data-size", "44x44")
        .text("groß")
        .close()
        .open("p")
        .text("Mehr im ")
        .open("a")
        .attr("href", "/g")
        .attr("data-size", "40x14")
        .text("Glossar")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 1);
}

#[test]
fn fokus_ohne_indikator_und_ungemessen() {
    let gemessen = seite()
        .open("button")
        .attr("data-fokus", "unsichtbar")
        .text("A")
        .close()
        .open("button")
        .attr("data-fokus", "sichtbar")
        .text("B")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&gemessen));
    assert_eq!(nur_review(&r, "focus/indicator-missing"), 1);
    assert!(befunde(&r, "focus/indicator-unmeasured").is_empty());

    // Nicht gemessen ist nicht bestanden: ein UNTESTED für die Seite.
    let ungemessen = seite()
        .open("button")
        .text("A")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&ungemessen));
    let b = befunde(&r, "focus/indicator-unmeasured");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].outcome, Outcome::Untested);
}

/// Ohne Layout laufen die Heuristiken nicht — und sagen das: je eine
/// `UNTESTED`-Meldung für die Seite statt eines stillen Nichts. Bis 0.20.0
/// meldeten sie hier gar nichts, und der Bericht zählte sie als gelaufen.
#[test]
fn ohne_layout_melden_die_heuristiken_ungeprueft() {
    let arena = seite()
        .open("div")
        .attr("data-reversed", "")
        .attr("data-endlos", "")
        .attr("data-min-width", "999")
        .attr("data-pointer", "")
        .text("Alles")
        .close()
        .close()
        .close()
        .build();
    let mut host = Host::new(&arena);
    host.layout = false;
    let r = run_full(&host);
    for id in [
        "order/visual-mismatch",
        "motion/infinite-animation",
        "reflow/min-width",
        "keyboard/pointer-only",
    ] {
        let b = befunde(&r, id);
        assert_eq!(b.len(), 1, "{id}: {b:?}");
        assert_eq!(b[0].outcome, Outcome::Untested, "{id}");
    }
}

#[test]
fn kleines_ziel_mit_genug_abstand_ist_kein_befund() {
    // Zwei 16-px-Ziele, 40 px Mittenabstand: Die 24-px-Kreise berühren sich nicht.
    let arena = seite()
        .open("button")
        .attr("data-size", "16x16@0,0")
        .text("a")
        .close()
        .open("button")
        .attr("data-size", "16x16@40,0")
        .text("b")
        .close()
        // Zwei 16-px-Ziele, 4 px Lücke (20 px Mittenabstand): zu eng.
        .open("button")
        .attr("data-size", "16x16@0,100")
        .text("c")
        .close()
        .open("button")
        .attr("data-size", "16x16@20,100")
        .text("d")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 2);
}

#[test]
fn checkbox_im_label_hat_das_label_als_ziel() {
    let arena = seite()
        .open("label")
        .open("input")
        .attr("type", "checkbox")
        .attr("data-size", "16x16@0,0")
        .close()
        .text("Pause motion")
        .close()
        .open("button")
        .attr("data-size", "44x44@20,0")
        .text("Daneben")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(nur_review(&run_full(&Host::new(&arena)), "targets/size"), 0);
}

#[test]
fn animiertes_svg_ist_eine_bewegung() {
    let arena = seite()
        .open("svg")
        .open("circle")
        .attr("data-endlos", "")
        .close()
        .open("circle")
        .attr("data-endlos", "")
        .close()
        .open("rect")
        .attr("data-endlos", "")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        1
    );
}

#[test]
fn mit_pause_schalter_keine_bewegungsmeldung() {
    let arena = seite()
        .open("span")
        .attr("data-endlos", "")
        .text("*")
        .close()
        .open("label")
        .open("input")
        .attr("type", "checkbox")
        .close()
        .text("Pause motion")
        .close()
        .close()
        .close()
        .build();
    assert_eq!(
        nur_review(&run_full(&Host::new(&arena)), "motion/infinite-animation"),
        0
    );
}

/// auditmysite#743 (B6-Umstellung): Mit berechneten Stilen fielen `<area>` und
/// `<audio>` ohne `controls` aus der Sicht, weil das UA-Stylesheet ihnen
/// `display: none` gibt. Korpusfälle `misc_content_checks` und
/// `media_and_visual`.
#[test]
fn ua_display_none_verbirgt_area_und_audio_nicht() {
    let arena = seite()
        .open("img")
        .attr("src", "plan.png")
        .attr("alt", "Plan")
        .attr("usemap", "#m")
        .close()
        .open("map")
        .attr("name", "m")
        .open("area")
        .attr("href", "/room1")
        .close()
        .close()
        .open("audio")
        .attr("autoplay", "")
        .attr("src", "/bg.mp3")
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(befunde(&r, "images/area-alt-missing").len(), 1);
    assert_eq!(befunde(&r, "media/audio-autoplay").len(), 1);
}

// --- Korpusfälle aus casoon/auditmysite#698 (Vergleich 2026-10-05) ---------

/// Die `id`-Attribute der Elemente, an denen `rule` mit `outcome` meldet.
fn an_mit(arena: &Arena, r: &Report, rule: &str, outcome: Outcome) -> Vec<String> {
    let knoten: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule && f.outcome == outcome)
        .filter_map(|f| f.location.node.clone())
        .collect();
    a11y_dom::elements(arena)
        .filter(|n| knoten.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

fn link(b: ArenaBuilder, id: &str, href: &str, groesse: &str) -> ArenaBuilder {
    b.open("a")
        .attr("id", id)
        .attr("href", href)
        .attr("data-size", groesse)
        .text(id)
        .close()
}

/// `target_size_equivalent`: 16 × 10 px Links dicht an dicht; „Equivalent"
/// greift nur mit einem sichtbaren, großen Link zum selben Dokument.
#[test]
fn korpus_target_size_equivalent() {
    let mut b = seite();
    for (i, (id, href)) in [
        ("eq-small", "/home"),
        ("hidden-only", "/about"),
        ("both-small", "/contact"),
        ("no-eq", "/imprint"),
        ("doc-fragment", "/guide#setup"),
        ("page-fragment", "#setup"),
        ("hash", "#"),
    ]
    .iter()
    .enumerate()
    {
        b = link(b, id, href, &format!("16x10@{},0", i * 16));
    }
    b = link(b, "eq-large", "/home", "48x48@0,100");
    b = link(b, "guide-large", "/guide", "48x48@0,200");
    b = link(b, "contact-klein", "/contact", "16x10@0,300");
    // Gegenstücke, die nicht zählen: unter aria-hidden, inert, außerhalb.
    b = b
        .open("div")
        .attr("aria-hidden", "true")
        .open("a")
        .attr("href", "/about")
        .attr("data-size", "48x48@100,100")
        .text("a")
        .close()
        .close()
        .open("div")
        .attr("inert", "")
        .open("a")
        .attr("href", "/about")
        .attr("data-size", "48x48@200,100")
        .text("b")
        .close()
        .close();
    b = link(b, "about-weg", "/about", "48x48@-9999,0");
    let arena = b.close().close().build();
    let r = run_full(&Host::new(&arena));
    let mut gemeldet = an_mit(&arena, &r, "targets/size", Outcome::Review);
    gemeldet.sort();
    assert_eq!(
        gemeldet,
        [
            "both-small",
            "hash",
            "hidden-only",
            "no-eq",
            "page-fragment"
        ]
    );
}

/// `target_size_hidden_neighbours` (auditmysite#705): Nachbarn unter `inert`
/// oder mit `pointer-events: none` engen nicht ein.
#[test]
fn korpus_target_size_hidden_neighbours() {
    let mut b = seite();
    b = link(b, "control", "/control", "16x16@0,0");
    b = link(b, "control-n", "/control-next", "48x48@16,0");
    b = link(b, "beside-inert", "/inert", "16x16@0,100");
    b = b
        .open("div")
        .attr("inert", "")
        .open("a")
        .attr("href", "/inert-next")
        .attr("data-size", "48x48@16,100")
        .text("N")
        .close()
        .close();
    b = link(b, "beside-no-pointer", "/no-pointer", "16x16@0,200");
    b = b
        .open("a")
        .attr("href", "/no-pointer-next")
        .attr("data-size", "48x48@16,200")
        .attr("data-no-pointer", "")
        .text("N")
        .close();
    let arena = b.close().close().build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(
        an_mit(&arena, &r, "targets/size", Outcome::Review),
        ["control"]
    );
}

/// `target_size_animation` (auditmysite#706): Ein Ziel, das beim Messen noch
/// wächst, bleibt ungemessen statt zu klein.
#[test]
fn korpus_target_size_animation() {
    let mut b = seite();
    b = b
        .open("a")
        .attr("id", "slow")
        .attr("href", "/slow")
        .attr("data-size", "12x48@0,0")
        .attr("data-waechst", "")
        .text("Slow intro")
        .close();
    b = link(b, "slow-next", "/slow-next", "48x48@16,0");
    let arena = b.close().close().build();
    let r = run_full(&Host::new(&arena));
    assert!(an_mit(&arena, &r, "targets/size", Outcome::Review).is_empty());
    assert_eq!(
        an_mit(&arena, &r, "targets/size", Outcome::Untested),
        ["slow"]
    );
}

/// 2.5.5 (AAA): 44 px, ohne Abstandsausnahme; was 2.5.8 schon meldet, nicht
/// doppelt.
#[test]
fn erhoehte_zielgroesse() {
    let mut b = seite();
    b = link(b, "allein", "/allein", "30x30@0,0");
    b = link(b, "eng-a", "/a", "16x16@0,200");
    b = link(b, "eng-b", "/b", "16x16@16,200");
    b = link(b, "gross", "/gross", "48x48@0,400");
    let arena = b.close().close().build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(
        an_mit(&arena, &r, "targets/size-enhanced", Outcome::Review),
        ["allein"]
    );
    let mut eng = an_mit(&arena, &r, "targets/size", Outcome::Review);
    eng.sort();
    assert_eq!(eng, ["eng-a", "eng-b"]);
}

/// `text_and_layout` (`div.flex-reorder`): umgeordneter Text ohne
/// Bedienelemente zählt für 1.3.2.
#[test]
fn korpus_umgeordneter_text() {
    let arena = seite()
        .open("div")
        .attr("id", "flex-reorder")
        .open("div")
        .attr("data-order", "2")
        .text("First in DOM")
        .close()
        .open("div")
        .attr("data-order", "1")
        .text("Second in DOM but shown first visually")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert_eq!(
        an_mit(&arena, &r, "order/visual-mismatch", Outcome::Review),
        ["flex-reorder"]
    );
}

/// spiegel.de: Teaserkarte mit `flex-row-reverse`, Bild- und Titellink zum
/// selben Artikel — kein Hinweis.
#[test]
fn umgekehrte_karte_mit_einem_ziel_spiegel_de() {
    let arena = seite()
        .open("article")
        .attr("id", "karte")
        .attr("data-reversed", "")
        .open("a")
        .attr("href", "/artikel")
        .text("Bild")
        .close()
        .open("a")
        .attr("href", "/artikel")
        .text("Titel")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert!(an_mit(&arena, &r, "order/visual-mismatch", Outcome::Review).is_empty());
}

/// `no_focus_targets`: `<marquee>` meldet auch ohne gemessenes Layout.
#[test]
fn korpus_marquee_ohne_layout() {
    let arena = seite()
        .open("marquee")
        .attr("id", "laufschrift")
        .text("Breaking news")
        .close()
        .close()
        .close()
        .build();
    let mut host = Host::new(&arena);
    host.layout = false;
    let r = run_full(&host);
    assert_eq!(
        an_mit(&arena, &r, "motion/infinite-animation", Outcome::Review),
        ["laufschrift"]
    );
}

/// bundesregierung.de: `cursor: pointer` auf einer Teaserkarte mit Link
/// darin — der Klick landet beim Link.
#[test]
fn karte_mit_link_ist_kein_nur_zeiger_bundesregierung_de() {
    let arena = seite()
        .open("div")
        .attr("id", "teaser")
        .attr("class", "bpa-teaser")
        .attr("data-pointer", "")
        .open("h3")
        .text("Überschrift")
        .close()
        .open("a")
        .attr("href", "/artikel")
        .text("Weiterlesen")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run_full(&Host::new(&arena));
    assert!(an_mit(&arena, &r, "keyboard/pointer-only", Outcome::Review).is_empty());
}

/// bundesregierung.de (mobil): Karussellpunkte mit „roving tabindex" — nur
/// der aktive hat `tabindex="0"`. Die übrigen sind trotzdem Zeigerziele und
/// engen den aktiven ein; alle drei sind zu klein.
#[test]
fn karussellpunkte_mit_roving_tabindex_bundesregierung_de() {
    let mut b = seite();
    for (i, tab) in ["0", "-1", "-1"].iter().enumerate() {
        b = b
            .open("button")
            .attr("id", &format!("slick-slide-control0{i}"))
            .attr("tabindex", tab)
            .attr("data-size", &format!("10x10@{},0", i * 14))
            .text(&format!("{}", i + 1))
            .close();
    }
    let arena = b.close().close().build();
    let r = run_full(&Host::new(&arena));
    let mut gemeldet = an_mit(&arena, &r, "targets/size", Outcome::Review);
    gemeldet.sort();
    assert_eq!(
        gemeldet,
        [
            "slick-slide-control00",
            "slick-slide-control01",
            "slick-slide-control02"
        ]
    );
}
