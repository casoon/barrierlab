//! Regeltests gegen die Referenz-Arena.

use a11y_dom::{Arena, ArenaNode, Document, Node, Semantics, elements};
use a11y_report::{Outcome, Report};
#[cfg(feature = "de")]
use a11y_rules::{Locale, run_in};
use a11y_rules::{run, run_with_semantics};
use accname::IdIndex;

/// Ein Tier-2-Host für die Tests: die Arena plus echte Namensberechnung.
///
/// Zeigt zugleich, wie ein Host `accname` in `Semantics` einhängt — der Index
/// wird einmal gebaut und gehalten, nicht je Aufruf.
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

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

/// Wie viele Befund-Kennungen diese Deklarationen zusammen tragen.
fn ids_in(metas: &'static [a11y_rules::Meta]) -> usize {
    metas.iter().map(|m| m.ids.len()).sum()
}

/// Ein Dokument, das möglichst viele Regeln auslöst — Grundlage der
/// Zusicherungen über die Namensmenge.
fn faulty() -> Arena {
    Arena::builder()
        .open("html")
        .open("head")
        .open("title")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "user-scalable=no")
        .close()
        .close()
        .open("body")
        .open("h2")
        .text("Springt")
        .close()
        .open("h4")
        .close()
        .open("img")
        .attr("src", "a.png")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "b.png")
        .close()
        .open("a")
        .attr("href", "/x")
        .close()
        .open("button")
        .close()
        .open("svg")
        .close()
        .open("input")
        .attr("id", "d")
        .close()
        .open("div")
        .attr("id", "d")
        .attr("role", "buton")
        .close()
        .open("div")
        .attr("role", "widget")
        .attr("aria-labelledby", "fehlt")
        .attr("tabindex", "4")
        .close()
        .open("ul")
        .open("div")
        .close()
        .close()
        .open("li")
        .text("verwaist")
        .close()
        .open("table")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build()
}

fn ids(r: &Report) -> Vec<&str> {
    r.findings.iter().map(|f| f.rule_id.as_str()).collect()
}

fn has(r: &Report, id: &str) -> bool {
    r.findings.iter().any(|f| f.rule_id == id)
}

/// Ein minimal korrektes Dokument, von dem die Einzeltests abweichen.
fn clean() -> a11y_dom::ArenaBuilder {
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
}

/// Ein Dokument, das auch die Landmark- und Sprunglink-Regeln zufriedenstellt.
///
/// Ein bloß wohlgeformtes Dokument genügt dafür nicht mehr: main, navigation,
/// banner und contentinfo sind eigene Erwartungen, der Sprunglink und die
/// Ortsangabe (`aria-current`) auch. Die Vorlage hält fest, was „vollständig"
/// heißt.
fn complete() -> a11y_dom::ArenaBuilder {
    clean()
        .open("body")
        .open("a")
        .attr("href", "#inhalt")
        .attr("class", "skip-link")
        .text("Zum Inhalt springen")
        .close()
        .open("header")
        .open("nav")
        .open("a")
        .attr("href", "/")
        .attr("aria-current", "page")
        .text("Start")
        .close()
        .close()
        .close()
        .open("main")
        .attr("id", "inhalt")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .open("footer")
        .text("Impressum")
        .close()
        .close()
}

#[test]
fn complete_document_yields_no_finding() {
    let doc = complete().close().build();
    let r = run(&doc);
    // Außer der Checkliste: Was keine Maschine entscheiden kann, bleibt als
    // UNTESTED stehen, auch auf einem einwandfreien Dokument.
    let verdicts: Vec<&str> = r
        .findings
        .iter()
        .filter(|f| !f.rule_id.starts_with("manual/"))
        .map(|f| f.rule_id.as_str())
        .collect();
    assert!(verdicts.is_empty(), "unerwartet: {verdicts:?}");
    assert!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("manual/"))
            .all(|f| f.outcome == Outcome::Untested)
    );
}

/// Was einem bloß wohlgeformten Dokument fehlt, wird benannt — und zwar
/// getrennt nach „belegt" und „erwartet": main fehlt nachweislich, eine
/// Navigation zu erwarten ist dagegen eine Annahme.
#[test]
fn minimal_document_reports_landmarks_as_review_except_main() {
    let doc = clean()
        .open("body")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    let outcome = |id: &str| {
        r.findings
            .iter()
            .find(|f| f.rule_id == id)
            .map(|f| f.outcome)
    };
    assert_eq!(outcome("landmarks/main-missing"), Some(Outcome::Fail));
    assert_eq!(
        outcome("landmarks/navigation-missing"),
        Some(Outcome::Review)
    );
    assert_eq!(outcome("landmarks/banner-missing"), Some(Outcome::Review));
    assert_eq!(
        outcome("landmarks/contentinfo-missing"),
        Some(Outcome::Review)
    );
    // Ohne Link und ohne Navigation gibt es nichts zu überspringen: die
    // Sprunglink-Regel ist nicht anwendbar (casoon/auditmysite#783).
    assert_eq!(outcome("keyboard/skip-link-missing"), None);
    // Der Viewport steht in sauber() -- diese Regel darf hier nicht feuern.
    assert_eq!(outcome("zoom/viewport-missing"), None);
}

#[test]
fn missing_lang_and_title() {
    let doc = Arena::builder()
        .open("html")
        .open("body")
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "document/lang-missing"));
    assert!(has(&r, "document/title-missing"));
}

#[test]
fn implausible_language_code() {
    // Ein leeres `lang` ist fehlend, nicht ungültig (ACT b5c3f8, bf051a);
    // das prüft der Korpus (casoon/auditmysite#783).
    for code in ["d", "deutsch-", "1de", "de-abcdefghi"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            has(&r, "document/lang-invalid"),
            "Code {code:?} haette auffallen muessen"
        );
    }
    // Gueltige Formen duerfen nicht anschlagen.
    for code in ["de", "de-DE", "en", "zh-Hant-TW"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            !has(&r, "document/lang-invalid"),
            "Code {code:?} ist gueltig"
        );
    }
}

#[test]
fn image_without_alt_is_fail_suspicious_alt_is_review() {
    let doc = clean()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "b.png")
        .close()
        .open("img")
        .attr("src", "c.png")
        .attr("alt", "Ein Hund im Schnee")
        .close()
        .open("img")
        .attr("src", "d.png")
        .attr("alt", "")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    let missing = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(missing.outcome, Outcome::Fail);

    let suspicious = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-suspicious")
        .unwrap();
    assert_eq!(
        suspicious.outcome,
        Outcome::Review,
        "heuristische Regeln liefern Review, nicht Fail"
    );

    // Genau einmal je Regel: der gute Alt-Text und das leere alt schlagen nicht an.
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("images/"))
            .count(),
        2
    );
}

#[test]
fn heading_gap_and_empty_heading() {
    let doc = clean()
        .open("body")
        .open("h1")
        .text("A")
        .close()
        .open("h3")
        .text("B")
        .close()
        .open("h4")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "headings/skip-level"));
    assert!(has(&r, "headings/empty"));
    assert!(!has(&r, "headings/h1-missing"));
}

#[test]
fn heading_with_named_image_is_not_empty() {
    // Eine Überschrift, die aus einem beschrifteten Logo besteht, trägt einen
    // Namen. Bis 0.10.0 meldete die Regel sie als leer, weil sie nur nach Text
    // und `aria-label` an der Überschrift selbst sah.
    let doc = clean()
        .open("body")
        .open("main")
        .open("h1")
        .open("img")
        .attr("src", "logo.png")
        .attr("alt", "Firmenlogo")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "headings/empty"), "{:?}", r.findings);
}

#[test]
fn heading_with_aria_labelledby_is_not_empty() {
    let doc = clean()
        .open("body")
        .open("main")
        .open("span")
        .attr("id", "titel")
        .text("Der Name steht hier")
        .close()
        .open("h1")
        .attr("aria-labelledby", "titel")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "headings/empty"), "{:?}", r.findings);
}

#[test]
fn truly_empty_heading_is_still_flagged() {
    // Die Gegenprobe: ein Bild ohne Alternativtext stiftet keinen Namen.
    let doc = clean()
        .open("body")
        .open("main")
        .open("h1")
        .open("img")
        .attr("src", "logo.png")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "headings/empty"));
}

#[test]
fn label_association_via_all_four_ways() {
    let doc = clean()
        .open("body")
        // 1. label[for]
        .open("label")
        .attr("for", "a")
        .text("A")
        .close()
        .open("input")
        .attr("id", "a")
        .close()
        // 2. verschachtelt
        .open("label")
        .text("B")
        .open("input")
        .attr("id", "b")
        .close()
        .close()
        // 3. aria-label
        .open("input")
        .attr("id", "c")
        .attr("aria-label", "C")
        .close()
        // 4. gar nichts
        .open("input")
        .attr("id", "d")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "forms/label-missing")
            .count(),
        1,
        "nur das vierte Feld ist unbeschriftet: {:?}",
        ids(&r)
    );
}

#[test]
fn placeholder_does_not_replace_a_label() {
    let doc = clean()
        .open("body")
        .open("input")
        .attr("id", "a")
        .attr("title", "Suche")
        .attr("placeholder", "Suchen…")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    // title zaehlt als Label, also kein label-missing - aber der Platzhalter
    // bleibt ein eigener Befund.
    assert!(!has(&r, "forms/label-missing"));
    assert!(has(&r, "forms/placeholder-as-label"));
}

#[test]
fn hidden_fields_are_skipped() {
    let doc = clean()
        .open("body")
        .open("input")
        .attr("type", "hidden")
        .attr("name", "csrf")
        .close()
        .open("input")
        .attr("type", "submit")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "forms/label-missing"));
}

#[test]
fn invalid_and_abstract_roles() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("role", "buton")
        .close()
        .open("div")
        .attr("role", "widget")
        .close()
        .open("div")
        .attr("role", "button")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "aria/role-invalid"));
    assert!(has(&r, "aria/role-abstract"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("aria/role"))
            .count(),
        2
    );
}

#[test]
fn aria_references_to_missing_ids() {
    let doc = clean()
        .open("body")
        .open("h2")
        .attr("id", "da")
        .text("Da")
        .close()
        .open("div")
        .attr("aria-labelledby", "da fehlt")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "aria/reference-missing")
        .unwrap();
    assert!(f.message.contains("fehlt"), "{}", f.message);
    assert!(
        !f.message.contains(" da"),
        "vorhandene ID darf nicht gemeldet werden: {}",
        f.message
    );
}

#[test]
fn duplicate_ids_without_reference_report_nothing() {
    // WCAG 2.2 hat 4.1.1 gestrichen: Eine Dopplung, auf die niemand zeigt,
    // verletzt kein Kriterium mehr.
    let doc = clean()
        .open("body")
        .open("div")
        .attr("id", "x")
        .close()
        .open("div")
        .attr("id", "x")
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "ids/duplicate"));
}

#[test]
fn referenced_duplicate_ids_report_only_the_second_hit() {
    let doc = clean()
        .open("body")
        .open("label")
        .attr("for", "x")
        .text("Name")
        .close()
        .open("input")
        .attr("id", "x")
        .close()
        .open("input")
        .attr("id", "x")
        .close()
        .open("input")
        .attr("id", "x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let hit: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "ids/duplicate")
        .collect();
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].wcag, vec!["4.1.2".to_string()]);
}

#[cfg(feature = "de")]
#[test]
fn texts_follow_the_chosen_language() {
    let doc = faulty();
    let en = run(&doc);
    let de = run_in(&doc, Locale::De);

    // Dieselben Befunde, nur anders beschriftet.
    assert_eq!(ids(&en), ids(&de));
    assert!(!en.findings.is_empty());
    for (e, d) in en.findings.iter().zip(&de.findings) {
        assert_ne!(e.message, d.message, "ohne deutsche Fassung: {}", e.rule_id);
    }
    let alt = |r: &Report| {
        r.findings
            .iter()
            .find(|f| f.rule_id == "images/alt-missing")
            .map(|f| f.message.clone())
    };
    assert_eq!(alt(&en).as_deref(), Some("The image has no alt attribute."));
    assert_eq!(alt(&de).as_deref(), Some("Das Bild hat kein alt-Attribut."));

    // Auch der Grund eines nicht gelaufenen Vermerks.
    let reason = de
        .rule_runs
        .iter()
        .find_map(|r| r.reason.clone())
        .expect("ohne Semantik laufen Tier-2-Regeln nicht");
    assert!(reason.starts_with("Host liefert"), "{reason}");
}

#[cfg(feature = "de")]
#[test]
fn semantics_rules_follow_the_chosen_language() {
    let arena = faulty();
    let doc = WithSemantics::new(&arena);
    let en = run_with_semantics(&doc);
    let de = a11y_rules::run_with_semantics_in(&doc, Locale::De);
    assert_eq!(ids(&en), ids(&de));
    assert!(
        ids(&en)
            .iter()
            .any(|id| id.starts_with("links/") || id.starts_with("buttons/"))
    );
    for (e, d) in en.findings.iter().zip(&de.findings) {
        assert_ne!(e.message, d.message, "ohne deutsche Fassung: {}", e.rule_id);
    }
}

#[cfg(feature = "de")]
#[test]
fn every_hint_has_a_german_version() {
    let all = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas());
    for meta in all {
        assert!(!meta.help_de.is_empty(), "{:?}", meta.ids);
        assert_ne!(
            meta.help_in(Locale::En),
            meta.help_in(Locale::De),
            "{:?}",
            meta.ids
        );
    }
}

#[test]
fn positive_tabindex_and_hidden_focusable() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("tabindex", "4")
        .close()
        .open("div")
        .attr("tabindex", "0")
        .close()
        .open("a")
        .attr("href", "/x")
        .attr("aria-hidden", "true")
        .text("X")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "keyboard/positive-tabindex")
            .count(),
        1
    );
    assert!(has(&r, "keyboard/hidden-focusable"));
}

#[test]
fn viewport_lock() {
    for content in [
        "width=device-width, user-scalable=no",
        "width=device-width,maximum-scale=1.0",
    ] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        assert!(has(&run(&doc), "zoom/viewport-locked"), "{content}");
    }
    // Erlaubter Viewport
    let doc = Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("x")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "zoom/viewport-locked"));
}

#[test]
fn lists_and_tables() {
    let doc = clean()
        .open("body")
        .open("ul")
        .open("div")
        .text("falsch")
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        .open("table")
        .attr("role", "presentation")
        .open("tr")
        .open("td")
        .text("Layout")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "lists/invalid-structure"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "tables/header-missing")
            .count(),
        1,
        "role=presentation ist eine Layouttabelle und gemeint"
    );
}

// --- Tier-Mechanik --------------------------------------------------------

#[test]
fn without_semantics_tier2_rules_are_recorded_as_not_run() {
    let doc = clean()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    assert_eq!(
        r.summary.rules_not_run,
        ids_in(a11y_rules::semantics_metas())
            + ids_in(a11y_rules::rendering_metas())
            + ids_in(a11y_rules::stylesheet_metas())
    );
    assert!(
        !has(&r, "links/name-missing"),
        "ohne Accessible Name darf die Regel nicht raten"
    );

    // Der Vermerk laeuft ueber die Befund-Kennung, nicht ueber eine
    // uebergeordnete Regelkennung -- nur so passen rule_runs und findings
    // zusammen.
    let record = r
        .rule_runs
        .iter()
        .find(|x| x.rule_id == "links/name-missing")
        .unwrap();
    assert!(!record.did_run());
    assert_eq!(record.not_run, Some(a11y_report::NotRun::CapabilityMissing));
}

#[test]
fn with_semantics_tier2_rules_run_too() {
    let arena = clean()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .close()
        .open("a")
        .attr("href", "/y")
        .text("Mehr")
        .close()
        .open("button")
        .close()
        .close()
        .close()
        .build();
    let doc = WithSemantics::new(&arena);
    let r = run_with_semantics(&doc);

    // Tier 2 laeuft, Tier 3 nicht -- dieser Host liefert keine Darstellung
    // und keine Stylesheets.
    assert_eq!(
        r.summary.rules_not_run,
        ids_in(a11y_rules::rendering_metas()) + ids_in(a11y_rules::stylesheet_metas())
    );
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "links/name-missing")
            .count(),
        1
    );
    assert!(has(&r, "buttons/name-missing"));
}

#[test]
fn same_link_text_different_targets() {
    let arena = clean()
        .open("body")
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/b")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/c")
        .text("Zum Bericht")
        .close()
        .close()
        .close()
        .build();
    let doc = WithSemantics::new(&arena);
    let r = run_with_semantics(&doc);
    let hit: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "links/ambiguous-name")
        .collect();
    assert_eq!(hit.len(), 2, "beide mehrdeutigen Links werden markiert");
    assert_eq!(hit[0].outcome, Outcome::Review);
}

#[test]
fn same_link_text_same_target_is_fine() {
    let arena = clean()
        .open("body")
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .close()
        .close()
        .build();
    let doc = WithSemantics::new(&arena);
    assert!(!has(&run_with_semantics(&doc), "links/ambiguous-name"));
}

#[test]
fn aria_hidden_elements_stay_out_of_tier2() {
    let arena = clean()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .attr("aria-hidden", "true")
        .close()
        .close()
        .close()
        .build();
    let doc = WithSemantics::new(&arena);
    assert!(!has(&run_with_semantics(&doc), "links/name-missing"));
}

#[test]
fn every_id_leaves_exactly_one_run_record() {
    let doc = clean().open("body").close().close().build();
    let r = run(&doc);
    let mut all_ids: Vec<&str> = r.rule_runs.iter().map(|x| x.rule_id.as_str()).collect();
    let count = all_ids.len();
    all_ids.sort_unstable();
    all_ids.dedup();
    assert_eq!(all_ids.len(), count, "doppelte Vermerke");
    let declared: usize = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas())
        .chain(a11y_rules::stylesheet_metas())
        .map(|m| m.ids.len())
        .sum();
    assert_eq!(count, declared, "ein Vermerk je deklarierter Kennung");
}

#[test]
fn every_run_record_carries_its_rules_criteria() {
    // `run` liefert gelaufene (Tier 1) und nicht gelaufene Vermerke (Tier 2+).
    let doc = clean().open("body").close().close().build();
    let r = run(&doc);
    let metas = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .chain(a11y_rules::rendering_metas())
        .chain(a11y_rules::stylesheet_metas());
    let mut checked = (0, 0);
    for m in metas {
        for id in m.ids {
            let rule_run = r.rule_runs.iter().find(|l| l.rule_id == *id).unwrap();
            assert_eq!(rule_run.wcag, m.wcag, "{id}: Kriterien im Vermerk");
            if !m.wcag.is_empty() {
                if rule_run.did_run() {
                    checked.0 += 1;
                } else {
                    checked.1 += 1;
                }
            }
        }
    }
    assert!(
        checked.0 > 0 && checked.1 > 0,
        "gelaufen und nicht gelaufen"
    );
}

#[test]
fn findings_carry_wcag_criteria_and_a_location() {
    let doc = clean()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(f.wcag, vec!["1.1.1"]);
    assert!(f.location.node.is_some());

    // Die Verortung laesst sich zum Knoten zurueckaufloesen.
    let id: u32 = f.location.node.as_ref().unwrap().parse().unwrap();
    let node = doc.get(a11y_dom::NodeId(id)).unwrap();
    assert_eq!(node.local_name(), "img");
    let _ = elements(&doc).count();
}

// --- Die Zusicherung, die den Bericht auswertbar macht --------------------

/// `rule_runs` und `findings` müssen dieselbe Namensmenge benutzen. Sonst
/// liefert ein Join über `rule_id` stillschweigend nichts — genau der Fehler,
/// der hier einmal drinsteckte: Vermerke trugen `images/alt`, Befunde
/// `images/alt-missing`.
#[test]
fn every_finding_has_a_matching_run_record() {
    let arena = faulty();
    let doc = WithSemantics::new(&arena);
    let r = run_with_semantics(&doc);

    let records: std::collections::HashSet<&str> =
        r.rule_runs.iter().map(|x| x.rule_id.as_str()).collect();

    for f in &r.findings {
        assert!(
            records.contains(f.rule_id.as_str()),
            "Befund {:?} hat keinen Ausfuehrungsvermerk — rule_runs und findings \
             benutzen verschiedene Namensmengen",
            f.rule_id
        );
    }
    assert!(!r.findings.is_empty(), "Testdokument muss Befunde erzeugen");
}

/// Jede erzeugte Kennung muss in `Meta::ids` deklariert sein. Fehlt eine, ist
/// sie im Bericht als „nie gelaufen" unsichtbar.
#[test]
fn every_produced_id_is_declared() {
    let arena = faulty();
    let doc = WithSemantics::new(&arena);
    let r = run_with_semantics(&doc);

    let declared: std::collections::HashSet<&str> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect();

    for f in &r.findings {
        assert!(
            declared.contains(f.rule_id.as_str()),
            "Kennung {:?} wird erzeugt, aber in keinem Meta::ids deklariert",
            f.rule_id
        );
    }
    // Erzeugt von einer Hilfsfunktion der Listenregel, nicht von der Prüfung
    // über die Listen selbst — fehlte deshalb einmal in `Meta::ids`.
    assert!(has(&r, "lists/item-outside-list"), "{:?}", ids(&r));
}

/// Keine Kennung darf doppelt deklariert sein — sonst gäbe es zwei Vermerke
/// für denselben Befundtyp.
#[test]
fn no_id_is_declared_twice() {
    let mut all: Vec<&str> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect();
    let count = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), count, "doppelt deklarierte Kennung");
}

// --- Verschaerfungen gegenueber 0.3.0 ------------------------------------

/// HTML-Attributwerte sind nicht normiert. `user-scalable=NO` sperrt den Zoom
/// genauso wie die Kleinschreibung -- bis 0.3.0 verglich die Regel den
/// `content`-Wert unveraendert und sah darueber hinweg.
#[test]
fn viewport_lock_is_case_insensitive() {
    for content in [
        "width=device-width, user-scalable=NO",
        "width=device-width, User-Scalable=No",
        "width=device-width, MAXIMUM-SCALE=1.0",
    ] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        assert!(has(&run(&doc), "zoom/viewport-locked"), "{content}");
    }
}

/// `role="list"` steht im Markup und ist damit Tier-1-entscheidbar. Bis 0.3.0
/// sah die Regel nur `<ul>`/`<ol>` und war fuer ARIA-Listen blind.
#[test]
fn list_by_role_is_checked() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("role", "list")
        .open("span")
        .text("kein Eintrag")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&doc), "lists/invalid-structure"));
}

/// ... und eine korrekt ausgezeichnete ARIA-Liste darf nicht auffallen.
#[test]
fn correct_role_list_yields_no_finding() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("role", "list")
        .open("div")
        .attr("role", "listitem")
        .text("Eintrag")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!has(&r, "lists/empty"), "{:?}", r.findings);
}

/// Eine Liste ohne Eintraege kuendigt der Assistenztechnik eine Struktur an,
/// die es nicht gibt. Neue Kennung `lists/empty`.
#[test]
fn empty_list_is_reported() {
    let doc = clean()
        .open("body")
        .open("ul")
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&doc), "lists/empty"));
}

/// `role="presentation"` sagt ausdruecklich, dass hier keine Liste gemeint
/// ist -- ein Strukturbefund darauf waere falsch.
#[test]
fn presentation_list_yields_no_structure_finding() {
    let doc = clean()
        .open("body")
        .open("ul")
        .attr("role", "presentation")
        .open("div")
        .text("Layout")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!has(&r, "lists/empty"), "{:?}", r.findings);
}

/// Eine per `role` umgedeutete `<ul>` ist keine Liste mehr. Beleg: gov.uk,
/// APG-Autocomplete `<ul id="…__listbox" role="listbox">`, leer, bis getippt
/// wird. Weder leer noch mit `option`-Kindern ist das ein Listenbefund.
#[test]
fn reinterpreted_list_is_not_checked_as_a_list() {
    let empty = clean()
        .open("body")
        .open("ul")
        .attr("role", "listbox")
        .attr("aria-label", "Vorschläge")
        .close()
        .close()
        .close()
        .build();
    let r = run(&empty);
    assert!(!has(&r, "lists/empty"), "{:?}", r.findings);

    let filled = clean()
        .open("body")
        .open("ul")
        .attr("role", "listbox")
        .attr("aria-label", "Vorschläge")
        .open("li")
        .attr("role", "option")
        .attr("aria-selected", "false")
        .text("Steuer")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&filled);
    assert!(!has(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!has(&r, "lists/item-outside-list"), "{:?}", r.findings);
}

/// Shadow DOM kommt als flacher Baum herein: Die Liste steht im Shadow Root
/// der Komponente, ihre Einträge sind Light-DOM-Kinder des Hosts, die per
/// `<slot>` in sie hineinprojiziert und dort unter dem `<slot>` eingehängt
/// werden. Beleg: sachsen-anhalt.de, `muse-link-list` mit
/// `<ul class="link-list__list"><slot></slot></ul>` und
/// `<muse-link role="listitem">` als Light-DOM-Kindern. Der `<slot>` selbst
/// ist `display: contents` und für die Liste unsichtbar.
#[test]
fn entries_via_slot_count_as_list_items() {
    let doc = clean()
        .open("body")
        .open("muse-link-list")
        .open("ul")
        .open("slot")
        .open("muse-link")
        .attr("role", "listitem")
        .open("a")
        .attr("href", "/a")
        .text("Impressum")
        .close()
        .close()
        .open("muse-link")
        .attr("role", "listitem")
        .open("a")
        .attr("href", "/b")
        .text("Datenschutz")
        .close()
        .close()
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!has(&r, "lists/empty"), "{:?}", r.findings);
    assert!(!has(&r, "lists/item-outside-list"), "{:?}", r.findings);
}

/// Ein `<slot>`, dem nichts zugewiesen ist und der keinen Ersatzinhalt hat,
/// lässt die Liste im flachen Baum leer — das bleibt ein Befund.
#[test]
fn list_with_empty_slot_stays_empty() {
    let doc = clean()
        .open("body")
        .open("ul")
        .open("slot")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "lists/empty"), "{:?}", r.findings);
    assert!(!has(&r, "lists/invalid-structure"), "{:?}", r.findings);
}

/// Eine Kopfzelle kann per Rolle ausgezeichnet sein. Bis 0.3.0 suchte die
/// Regel nur `<th>` und meldete solche Tabellen faelschlich als kopflos.
#[test]
fn header_cell_by_role_counts_as_header() {
    let doc = clean()
        .open("body")
        .open("table")
        .open("tr")
        .open("td")
        .attr("role", "columnheader")
        .text("Spalte")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "tables/header-missing"), "{:?}", r.findings);
}

/// ... und eine Tabelle, die nur per Rolle eine ist, wird ueberhaupt geprueft.
#[test]
fn role_table_without_header_is_flagged() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("role", "table")
        .open("div")
        .attr("role", "row")
        .open("div")
        .attr("role", "cell")
        .text("x")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&doc), "tables/header-missing"));
}

/// Die Einstufungen, bei denen eine stille Absenkung fachlich etwas kaputt
/// machen würde. Nicht der ganze Katalog — nur die Fälle, über die schon
/// einmal entschieden wurde.
#[test]
fn ratings_stay_where_they_were_justified() {
    use a11y_report::Severity;

    let metas: Vec<_> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .collect();
    let sev = |id: &str| {
        metas
            .iter()
            .find(|m| m.ids.contains(&id))
            .unwrap_or_else(|| panic!("Kennung {id} nicht deklariert"))
            .severity
    };

    // Bricht die Tabreihenfolge reproduzierbar und fuer jeden, der mit der
    // Tastatur navigiert.
    assert_eq!(sev("keyboard/positive-tabindex"), Severity::High);

    // WCAG 4.1.1 wurde in WCAG 2.2 entfernt; der echte Schaden entsteht erst
    // bei einer Referenz, und dafuer gibt es aria/reference-missing.
    assert_eq!(sev("ids/duplicate"), Severity::Medium);

    // Ein Feld ohne Label ist fuer Screenreader-Nutzer unbenutzbar.
    assert_eq!(sev("forms/label-missing"), Severity::Critical);
}

// --- Die Luecken, die beim Abloesen der auditmysite-Regeln auffielen -----

#[test]
fn term_without_definition() {
    let doc = clean()
        .open("body")
        .open("dl")
        // Vollstaendig: Begriff mit Definition.
        .open("dt")
        .attr("id", "gut")
        .text("HTML")
        .close()
        .open("dd")
        .text("Auszeichnungssprache")
        .close()
        .close()
        .open("dl")
        // Unvollstaendig: Begriff allein.
        .open("dt")
        .attr("id", "schlecht")
        .text("CSS")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let hit: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "lists/term-without-definition")
        .collect();
    assert_eq!(
        hit.len(),
        1,
        "nur der zweite Begriff ist unvollstaendig: {:?}",
        ids(&r)
    );
}

#[test]
fn term_in_a_div_group_needs_its_definition_there() {
    // HTML erlaubt <div>-Gruppen in einer <dl>. Ein <dt> in einer solchen
    // Gruppe braucht sein <dd> dort, nicht irgendwo in der Liste.
    let doc = clean()
        .open("body")
        .open("dl")
        .open("div")
        .open("dt")
        .text("A")
        .close()
        .close()
        .open("div")
        .open("dt")
        .text("B")
        .close()
        .open("dd")
        .text("b")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "lists/term-without-definition")
            .count(),
        1,
        "nur die erste Gruppe ist unvollstaendig"
    );
}

#[test]
fn roles_count_for_term_and_definition_too() {
    let doc = clean()
        .open("body")
        .open("div")
        .open("div")
        .attr("role", "term")
        .text("A")
        .close()
        .open("div")
        .attr("role", "definition")
        .text("a")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "lists/term-without-definition"));
}

#[test]
fn presentational_table_with_header_cells() {
    let doc = clean()
        .open("body")
        // Sauber: Layouttabelle ohne Koepfe.
        .open("table")
        .attr("role", "presentation")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        // Widerspruechlich: als praesentational ausgezeichnet, aber mit Kopf.
        .open("table")
        .attr("role", "none")
        .open("tr")
        .open("th")
        .text("Kopf")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "tables/presentational-with-headers")
            .count(),
        1
    );
    // Und keine der beiden wird als kopflose Datentabelle gemeldet.
    assert!(!has(&r, "tables/header-missing"));
}

#[test]
fn table_without_name_is_review_not_fail() {
    let doc = clean()
        .open("body")
        .open("table")
        .open("caption")
        .text("Umsätze")
        .close()
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .open("table")
        .attr("aria-label", "Kosten")
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let hit: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "tables/name-missing")
        .collect();
    assert_eq!(hit.len(), 1, "caption und aria-label zaehlen als Name");
    // Ob eine Tabelle einen Namen braucht, ist nicht zwingend entscheidbar.
    assert_eq!(hit[0].outcome, Outcome::Review);
}

#[test]
fn viewport_distinguishes_violation_from_limit() {
    let fall = |content: &str| {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        ids(&r)
            .iter()
            .filter(|i| i.starts_with("zoom/"))
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
    };

    // Unter 200 %: Verstoss gegen 1.4.4.
    assert_eq!(fall("maximum-scale=1.5"), vec!["zoom/viewport-locked"]);
    assert_eq!(fall("user-scalable=NO"), vec!["zoom/viewport-locked"]);

    // Semikolon und Leerraum trennen wie das Komma, Leerraum um `=` gehört
    // zum Paar (CSS Viewport). Das Semikolon ist in der Praxis verbreitet.
    assert_eq!(
        fall("width=device-width; maximum-scale=1"),
        vec!["zoom/viewport-locked"]
    );
    assert_eq!(
        fall("width=device-width user-scalable = no"),
        vec!["zoom/viewport-locked"]
    );
    assert!(fall("width=device-width; user-scalable=yes").is_empty());

    // Zwischen 200 und 500 %: erfuellt 1.4.4, begrenzt aber. Eigene Kennung,
    // und nicht beide zugleich.
    assert_eq!(fall("maximum-scale=3"), vec!["zoom/viewport-scale-limited"]);

    // Ab 500 % oder ohne Begrenzung: nichts.
    assert!(fall("maximum-scale=5").is_empty());
    assert!(fall("width=device-width, initial-scale=1").is_empty());
}

#[test]
fn viewport_limit_is_best_practice_without_1_4_4() {
    // Zwischen 200 und 500 % ist 1.4.4 erfüllt; der Befund darf das
    // Kriterium nicht tragen, sonst zählen Hosts ihn als Verstoß. Der
    // Verstoß unter 200 % trägt es weiter.
    let finding = |content: &str, id: &str| {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        run(&doc)
            .findings
            .into_iter()
            .find(|f| f.rule_id == id)
            .expect("Befund")
    };
    let limited = finding("maximum-scale=2", "zoom/viewport-scale-limited");
    assert!(limited.wcag.is_empty(), "{:?}", limited.wcag);
    assert_eq!(limited.tags, vec!["best-practice".to_string()]);

    let locked = finding("maximum-scale=1", "zoom/viewport-locked");
    assert_eq!(locked.wcag, vec!["1.4.4".to_string()]);
}

#[test]
fn list_item_outside_a_list() {
    // Die Listenpruefung laeuft ueber Listen und sieht nur, was darin steht.
    // Ein verwaistes <li> wird dabei nie besucht.
    let doc = clean()
        .open("body")
        .open("ul")
        .open("li")
        .text("drin")
        .close()
        .close()
        .open("div")
        .open("li")
        .text("verwaist")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "lists/item-outside-list")
            .count(),
        1,
        "nur der verwaiste Eintrag: {:?}",
        ids(&r)
    );
}

#[test]
fn nested_entry_is_not_orphaned() {
    // <li> in einer Unterliste hat die aeussere Liste als Vorfahren.
    let doc = clean()
        .open("body")
        .open("ul")
        .open("li")
        .text("a")
        .open("ul")
        .open("li")
        .text("a1")
        .close()
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "lists/item-outside-list"));
}

// ===========================================================================
// Tier 3 — Kontrast
// ===========================================================================

/// Ein Tier-3-Host für die Tests: die Arena plus eine Stiltabelle je Knoten.
///
/// Die Stile kommen als Tabelle herein, nicht aus einer CSS-Engine — geprüft
/// wird die Regel, nicht das Auflösen der Kaskade. Genau die Aufteilung gilt
/// auch in echt: Der Host löst auf, die Regel rechnet.
struct WithPresentation<'a> {
    doc: &'a Arena,
    styles: std::collections::HashMap<u32, a11y_dom::ComputedStyle>,
}

impl a11y_dom::Document for WithPresentation<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl a11y_dom::Rendering for WithPresentation<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<a11y_dom::ComputedStyle> {
        self.styles.get(&node.id().0).cloned()
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<a11y_dom::Rect> {
        None
    }
}

fn make_color(r: u8, g: u8, b: u8) -> a11y_dom::Color {
    a11y_dom::Color { r, g, b, a: 255 }
}

/// Stil für normalen Text in den übergebenen Farben.
fn style(
    front: Option<a11y_dom::Color>,
    back: Option<a11y_dom::Color>,
    px: f32,
) -> a11y_dom::ComputedStyle {
    a11y_dom::ComputedStyle {
        color: front,
        background_color: back,
        font_size_px: Some(px),
        font_weight: Some(400),
        display: Some("block".into()),
        visibility: Some("visible".into()),
        ..Default::default()
    }
}

/// Baut ein Dokument mit einem `<p>` im Body und legt dessen Stil fest.
fn with_paragraph(
    text: &str,
    s: a11y_dom::ComputedStyle,
) -> (
    Arena,
    std::collections::HashMap<u32, a11y_dom::ComputedStyle>,
) {
    let arena = clean()
        .open("body")
        .open("h1")
        .text("Titel")
        .close()
        .open("p")
        .text(text)
        .close()
        .close()
        .close()
        .build();
    // Der Absatz ist das einzige Element mit eigenem Text außer h1 und title.
    let mut styles = std::collections::HashMap::new();
    for n in elements(&arena) {
        if n.is_element("p") {
            styles.insert(n.id().0, s.clone());
        }
    }
    (arena, styles)
}

#[test]
fn weak_contrast_is_flagged() {
    let (arena, styles) = with_paragraph(
        "Kaum zu lesen",
        style(
            Some(make_color(0x99, 0x99, 0x99)),
            Some(make_color(255, 255, 255)),
            16.0,
        ),
    );
    let doc = WithPresentation {
        doc: &arena,
        styles,
    };
    let r = a11y_rules::run_with_rendering(&doc);
    assert!(
        has(&r, "contrast/text-insufficient"),
        "unerwartet: {:?}",
        ids(&r)
    );
}

#[test]
fn sufficient_contrast_reports_nothing() {
    let (arena, styles) = with_paragraph(
        "Gut zu lesen",
        style(
            Some(make_color(0x33, 0x33, 0x33)),
            Some(make_color(255, 255, 255)),
            16.0,
        ),
    );
    let doc = WithPresentation {
        doc: &arena,
        styles,
    };
    let r = a11y_rules::run_with_rendering(&doc);
    assert!(!has(&r, "contrast/text-insufficient"), "{:?}", ids(&r));
}

#[test]
fn large_text_may_be_weaker() {
    // 3,5:1 — zu wenig für normalen Text, genug für großen.
    let gray = make_color(0x8C, 0x8C, 0x8C);
    let white = make_color(255, 255, 255);

    let (arena, styles) = with_paragraph("Klein", style(Some(gray), Some(white), 16.0));
    let small = WithPresentation {
        doc: &arena,
        styles,
    };
    assert!(has(
        &a11y_rules::run_with_rendering(&small),
        "contrast/text-insufficient"
    ));

    let (arena2, styles2) = with_paragraph("Gross", style(Some(gray), Some(white), 32.0));
    let large = WithPresentation {
        doc: &arena2,
        styles: styles2,
    };
    assert!(!has(
        &a11y_rules::run_with_rendering(&large),
        "contrast/text-insufficient"
    ));
}

/// Der fachliche Kern: Ein Host, der die Hintergrundfarbe nicht auflösen kann,
/// bekommt `UNTESTED` — nicht `PASS` und nicht Schweigen.
#[test]
fn indeterminable_background_is_untested_not_passed() {
    let (arena, styles) = with_paragraph(
        "Auf einem Bild",
        style(Some(make_color(0, 0, 0)), None, 16.0),
    );
    let doc = WithPresentation {
        doc: &arena,
        styles,
    };
    let r = a11y_rules::run_with_rendering(&doc);

    let finding = r
        .findings
        .iter()
        .find(|f| f.rule_id == "contrast/text-undetermined")
        .expect("unbestimmbarer Hintergrund muss einen Befund erzeugen");
    assert_eq!(finding.outcome, Outcome::Untested);
    assert!(
        !has(&r, "contrast/text-insufficient"),
        "kein geratenes FAIL"
    );
}

#[test]
fn invisible_text_is_no_contrast_problem() {
    let mut s = style(
        Some(make_color(0xEE, 0xEE, 0xEE)),
        Some(make_color(255, 255, 255)),
        16.0,
    );
    s.display = Some("none".into());
    let (arena, styles) = with_paragraph("Versteckt", s);
    let doc = WithPresentation {
        doc: &arena,
        styles,
    };
    let r = a11y_rules::run_with_rendering(&doc);
    assert!(!has(&r, "contrast/text-insufficient"), "{:?}", ids(&r));
    assert!(!has(&r, "contrast/text-undetermined"), "{:?}", ids(&r));
}

/// Ohne Tier 3 liefert die Regel `UNTESTED` als Vermerk, nicht als Schweigen.
#[test]
fn without_rendering_contrast_is_recorded_as_not_run() {
    let doc = clean().open("body").close().close().build();
    let r = run(&doc);
    let record = r
        .rule_runs
        .iter()
        .find(|x| x.rule_id == "contrast/text-insufficient")
        .expect("die Kennung muss im Bericht stehen");
    assert!(!record.did_run());
    assert_eq!(record.not_run, Some(a11y_report::NotRun::CapabilityMissing));
}

// ===========================================================================
// Landmarks, Sprunglink, erforderliche ARIA-Attribute
// ===========================================================================

#[test]
fn role_counts_before_the_tag_for_landmarks() {
    // <div role="main"> ist eine main-Landmark, <main role="presentation">
    // ist keine. Wer nur auf den Tagnamen sieht, liegt in beiden Faellen falsch.
    let with_role = clean()
        .open("body")
        .open("div")
        .attr("role", "main")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&with_role), "landmarks/main-missing"));

    let redeclared = clean()
        .open("body")
        .open("main")
        .attr("role", "presentation")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&redeclared), "landmarks/main-missing"));
}

#[test]
fn two_main_landmarks_are_flagged() {
    let doc = complete()
        .open("main")
        .text("noch eine")
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(has(&r, "landmarks/main-duplicate"), "{:?}", ids(&r));
}

/// Ein `<footer>` in einem `<article>` gehoert zu diesem Artikel, nicht zum
/// Dokument. Ein `<div>` dazwischen disqualifiziert dagegen nicht.
#[test]
fn footer_in_article_is_no_contentinfo_landmark() {
    let in_article = clean()
        .open("body")
        .open("main")
        .open("h1")
        .text("T")
        .close()
        .open("article")
        .open("footer")
        .text("Autor")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&in_article), "landmarks/contentinfo-missing"));

    let in_div = clean()
        .open("body")
        .open("main")
        .open("h1")
        .text("T")
        .close()
        .close()
        .open("div")
        .open("footer")
        .text("Impressum")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&in_div), "landmarks/contentinfo-missing"));
}

#[test]
fn skip_link_is_detected_by_text_or_class() {
    let over_text = clean()
        .open("body")
        .open("a")
        .attr("href", "#inhalt")
        .text("Skip to content")
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&over_text), "keyboard/skip-link-missing"));

    // Ein gewoehnlicher Anker ist kein Sprunglink.
    let anchor = clean()
        .open("body")
        .open("a")
        .attr("href", "#kapitel-3")
        .text("Kapitel 3")
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&anchor), "keyboard/skip-link-missing"));
}

/// auditmysite#642: Erkannt am Ziel und an der Stellung, nicht am Wortlaut —
/// ein Fragmentlink vor dem ersten Link, der die Seite verlässt.
#[test]
fn skip_link_is_detected_by_target_and_position() {
    let french = clean()
        .open("body")
        .open("a")
        .attr("href", "#contenu")
        .text("Aller au contenu")
        .close()
        .open("nav")
        .open("a")
        .attr("href", "/fr/")
        .text("Accueil")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!has(&run(&french), "keyboard/skip-link-missing"));

    // Derselbe Anker hinter der Navigation überspringt nichts.
    let behind = clean()
        .open("body")
        .open("nav")
        .open("a")
        .attr("href", "/fr/")
        .text("Accueil")
        .close()
        .close()
        .open("a")
        .attr("href", "#contenu")
        .text("Aller au contenu")
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&behind), "keyboard/skip-link-missing"));
}

/// Der Sprunglink ist heuristisch erkannt — der Befund ist deshalb REVIEW.
#[test]
fn missing_skip_link_is_review_not_fail() {
    // Eine Navigation, die sich wiederholen kann — ohne sie ist die Regel
    // nicht anwendbar (casoon/auditmysite#783).
    let doc = clean()
        .open("body")
        .open("nav")
        .open("a")
        .attr("href", "/")
        .text("Start")
        .close()
        .close()
        .close()
        .close()
        .build();
    let f = run(&doc)
        .findings
        .into_iter()
        .find(|f| f.rule_id == "keyboard/skip-link-missing")
        .expect("muss gemeldet werden");
    assert_eq!(f.outcome, Outcome::Review);
}

#[test]
fn roles_without_their_required_attributes_are_flagged() {
    let doc = complete()
        .open("div")
        .attr("role", "checkbox")
        .text("Newsletter")
        .close()
        .open("div")
        .attr("role", "slider")
        .attr("aria-valuenow", "3")
        .close()
        .close()
        .build();
    let r = run(&doc);
    let findings: Vec<&str> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "aria/required-attribute-missing")
        .map(|f| f.message.as_str())
        .collect();
    // Der Slider hat valuenow; valuemin und valuemax haben seit ARIA 1.2
    // Vorgaben und fehlen deshalb nicht.
    assert_eq!(findings.len(), 1, "{:?}", ids(&r));
    assert!(findings.iter().any(|m| m.contains("aria-checked")));
}

#[test]
fn native_state_and_static_separator_need_no_aria() {
    // auditmysite#656: Das native Feld übermittelt Wert bzw. Haken selbst.
    let doc = complete()
        .open("input")
        .attr("type", "range")
        .attr("role", "slider")
        .close()
        .open("input")
        .attr("type", "checkbox")
        .attr("role", "switch")
        .close()
        .open("div")
        .attr("role", "separator")
        .close()
        .open("div")
        .attr("role", "option")
        .text("A")
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "aria/required-attribute-missing"));
}

#[test]
fn meter_and_focusable_separator_need_valuenow() {
    let doc = complete()
        .open("div")
        .attr("role", "meter")
        .close()
        .open("div")
        .attr("role", "separator")
        .attr("tabindex", "0")
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "aria/required-attribute-missing")
        .count();
    assert_eq!(n, 2, "{:?}", ids(&r));
}

#[test]
fn complete_role_reports_nothing() {
    let doc = complete()
        .open("div")
        .attr("role", "checkbox")
        .attr("aria-checked", "false")
        .text("Newsletter")
        .close()
        .close()
        .build();
    assert!(!has(&run(&doc), "aria/required-attribute-missing"));
}

#[test]
fn missing_viewport_is_flagged() {
    let doc = Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&doc), "zoom/viewport-missing"));
}

/// Mehrere h1 sind in HTML zulaessig — das ist eine Erwartung, kein Verstoss.
#[test]
fn several_h1_are_review_not_fail() {
    let doc = complete()
        .open("h1")
        .text("Noch ein Titel")
        .close()
        .close()
        .build();
    let f = run(&doc)
        .findings
        .into_iter()
        .find(|f| f.rule_id == "headings/h1-multiple")
        .expect("muss gemeldet werden");
    assert_eq!(f.outcome, Outcome::Review);
}

/// Nichtssagender Linktext und mehrdeutiger Linktext sind zwei verschiedene
/// Regeln. Der eine Befund sagt „dieser Text hilft niemandem", der andere
/// „zwei Links heißen gleich, führen aber woandershin".
#[test]
fn meaningless_link_text_is_its_own_rule() {
    let arena = complete()
        .open("a")
        .attr("href", "/eins")
        .text("mehr")
        .close()
        .open("a")
        .attr("href", "/zwei")
        .text("Click here")
        .close()
        .close()
        .build();
    let doc = WithSemantics::new(&arena);
    let r = run_with_semantics(&doc);

    let generic: Vec<&str> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "links/generic-name")
        .map(|f| f.message.as_str())
        .collect();
    // Dazu „Start" aus der Navigation der Vorlage: Seit der Übernahme von
    // auditmysites Liste (Plan 98) zählen auch Beschriftungen ohne Ziel.
    assert_eq!(generic.len(), 3, "{:?}", ids(&r));
    // Heuristisch -- die Liste kann einen Namen treffen, der im Zusammenhang
    // doch eindeutig ist.
    assert!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "links/generic-name")
            .all(|f| f.outcome == Outcome::Review)
    );
    // Verschiedene Ziele, verschiedene Namen -- nicht mehrdeutig.
    assert!(!has(&r, "links/ambiguous-name"));
}

/// Ein ausdruecklich dekoratives Bild braucht kein alt. Ihm eins abzuverlangen
/// hiesse, eine bewusste Angabe des Autors zu ignorieren.
#[test]
fn explicitly_decorative_images_need_no_alt() {
    let doc = complete()
        .open("img")
        .attr("src", "spacer.gif")
        .attr("role", "presentation")
        .close()
        .open("img")
        .attr("src", "bg.jpg")
        .attr("aria-hidden", "true")
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!has(&r, "images/alt-missing"), "{:?}", ids(&r));

    // Ein gewoehnliches Bild ohne alt faellt weiterhin auf.
    let without = complete()
        .open("img")
        .attr("src", "foto.jpg")
        .close()
        .close()
        .build();
    assert!(has(&run(&without), "images/alt-missing"));
}

#[test]
fn empty_or_orphaned_active_descendant_is_flagged() {
    let doc = complete()
        .open("div")
        .attr("role", "listbox")
        .attr("tabindex", "0")
        .attr("aria-activedescendant", "gibt-es-nicht")
        .close()
        .open("button")
        .attr("aria-controls", "  ")
        .text("Menü")
        .close()
        // Leeres aria-labelledby fällt auf andere Namensquellen zurück.
        .open("button")
        .attr("aria-labelledby", "")
        .text("OK")
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "aria/reference-missing")
        .count();
    assert_eq!(n, 2, "{:?}", r.findings);
}

#[test]
fn disabled_link_under_aria_hidden_stays_focusable() {
    // `disabled` wirkt nur an Formularfeldern.
    let doc = complete()
        .open("div")
        .attr("aria-hidden", "true")
        .open("a")
        .attr("href", "/x")
        .attr("disabled", "")
        .text("x")
        .close()
        .open("button")
        .attr("disabled", "")
        .text("y")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "keyboard/hidden-focusable")
        .count();
    assert_eq!(n, 1, "{:?}", r.findings);
}

#[test]
fn image_with_aria_label_or_title_needs_no_alt() {
    let doc = complete()
        .open("img")
        .attr("src", "logo.png")
        .attr("aria-label", "Logo")
        .close()
        .open("img")
        .attr("src", "karte.png")
        .attr("title", "Anfahrt")
        .close()
        .open("img")
        .attr("src", "leer.png")
        .attr("title", " ")
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "images/alt-missing")
        .count();
    assert_eq!(n, 1, "{:?}", r.findings);
}

/// auditmysite#639: Die Grenze gilt auch über die Rolle des Vorfahren.
#[test]
fn header_under_role_main_is_no_banner() {
    let doc = clean()
        .open("body")
        .open("div")
        .attr("role", "main")
        .open("header")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(has(&run(&doc), "landmarks/banner-missing"));
}

/// bund.de, sachsen-anhalt.de: Das Menü einer Web-Komponente entsteht erst
/// beim Öffnen. Eingeklappt darf `aria-controls` ins Leere zeigen (wie axe).
#[test]
fn collapsed_aria_controls_may_point_to_later_target() {
    let doc = complete()
        .open("button")
        .attr("aria-controls", "menue-spaeter")
        .attr("aria-expanded", "false")
        .text("Menü")
        .close()
        .open("button")
        .attr("aria-controls", "menue-fehlt")
        .attr("aria-expanded", "true")
        .text("Offen")
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "aria/reference-missing")
        .count();
    assert_eq!(n, 1, "{:?}", r.findings);
}

/// barrierlab#26 (bund.de): Die Sprunglinks stehen hinter den Links des
/// Cookie-Banners. Ein Fragmentlink auf den Anfang des Hauptinhalts zählt
/// trotzdem; ein Anker mitten im Inhalt nicht.
#[test]
fn skip_link_to_main_content_counts_behind_other_links_too() {
    let page = |target: &str| {
        clean()
            .open("body")
            .open("div")
            .attr("class", "cookie-banner")
            .open("a")
            .attr("href", "/datenschutz")
            .text("Datenschutz")
            .close()
            .open("a")
            .attr("href", "")
            .close()
            .close()
            .open("a")
            .attr("href", target)
            .text("Inhalt")
            .close()
            .open("nav")
            .open("a")
            .attr("href", "/")
            .text("Start")
            .close()
            .close()
            .open("main")
            .attr("id", "main")
            .open("h1")
            .attr("id", "titel")
            .text("Titel")
            .close()
            .open("p")
            .text("Text")
            .close()
            .open("h2")
            .attr("id", "abschnitt")
            .text("Abschnitt")
            .close()
            .close()
            .close()
            .close()
            .build()
    };
    assert!(!has(&run(&page("#main")), "keyboard/skip-link-missing"));
    assert!(!has(&run(&page("#titel")), "keyboard/skip-link-missing"));
    assert!(has(&run(&page("#abschnitt")), "keyboard/skip-link-missing"));
}

/// auditmysite#709: Ein offener Consent-Dialog blendet `<main>` per
/// `aria-hidden` aus. Das ist korrektes Verhalten, kein fehlendes main.
#[test]
fn main_behind_open_modal_is_hint_not_violation() {
    let doc = clean()
        .open("body")
        .open("main")
        .attr("aria-hidden", "true")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .open("section")
        .attr("role", "dialog")
        .attr("aria-modal", "true")
        .attr("aria-label", "Cookies")
        .open("button")
        .text("Akzeptieren")
        .close()
        .close()
        .close()
        .close()
        .build();
    let f = run(&doc)
        .findings
        .into_iter()
        .find(|f| f.rule_id == "landmarks/main-missing")
        .expect("als Hinweis gemeldet");
    assert_eq!(f.outcome, Outcome::Review);

    // Auch ohne aria-modal (administracion.gob.es).
    let without_modal = clean()
        .open("body")
        .open("main")
        .attr("aria-hidden", "true")
        .close()
        .open("section")
        .attr("role", "dialog")
        .attr("aria-label", "Cookies")
        // Inhalt, sonst ist die Seite leer und die Regel nicht anwendbar.
        .open("button")
        .text("Akzeptieren")
        .close()
        .close()
        .close()
        .close()
        .build();
    let f = run(&without_modal)
        .findings
        .into_iter()
        .find(|f| f.rule_id == "landmarks/main-missing")
        .unwrap();
    assert_eq!(f.outcome, Outcome::Review);

    // Ohne Dialog bleibt es ein Verstoß.
    let without = clean()
        .open("body")
        .open("div")
        .text("Inhalt")
        .close()
        .close()
        .close()
        .build();
    let f = run(&without)
        .findings
        .into_iter()
        .find(|f| f.rule_id == "landmarks/main-missing")
        .unwrap();
    assert_eq!(f.outcome, Outcome::Fail);
}

/// berlin.de: Swiper setzt `role="group"` auf die `<li>` der Slider-Liste.
/// Dann hat die Liste keine Einträge mehr, die die Assistenztechnik als
/// solche sieht.
#[test]
fn li_with_other_role_is_no_list_item() {
    let doc = complete()
        .open("ul")
        .attr("class", "swiper-wrapper")
        .open("li")
        .attr("role", "group")
        .text("Folie 1")
        .close()
        .open("li")
        .attr("role", "group")
        .text("Folie 2")
        .close()
        .close()
        // Zur Kontrolle: role="listitem" bleibt ein Eintrag.
        .open("ul")
        .open("li")
        .attr("role", "listitem")
        .text("A")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let n = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "lists/invalid-structure")
        .count();
    assert_eq!(n, 1, "{:?}", r.findings);
}

// --- Linkzweck aus auditmysites link_purpose (Plan 98, barrierlab#13) ------

/// Ein Dokument aus `<html><body>…</body></html>` mit dem gegebenen Inhalt.
fn with_body(content: impl FnOnce(a11y_dom::ArenaBuilder) -> a11y_dom::ArenaBuilder) -> Arena {
    content(
        Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("body"),
    )
    .close()
    .close()
    .build()
}

fn findings<'r>(r: &'r Report, rule: &str) -> Vec<&'r a11y_report::Finding> {
    r.findings.iter().filter(|f| f.rule_id == rule).collect()
}

/// Ohne erkennbaren Zusammenhang Medium, auch in fremden Sprachen.
#[test]
fn generic_link_without_context_is_medium() {
    let arena = with_body(|b| {
        b.open("a")
            .attr("href", "/a")
            .text("Click here")
            .close()
            .open("a")
            .attr("href", "/b")
            .text("Lire la suite")
            .close()
            .open("a")
            .attr("href", "/c")
            .text("Mehr ›")
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    let g = findings(&r, "links/generic-name");
    assert_eq!(g.len(), 3, "{:?}", ids(&r));
    assert!(
        g.iter()
            .all(|f| f.outcome == Outcome::Review && f.severity == a11y_report::Severity::Medium)
    );
}

/// auditmysite #569: „Weiter" hinter beschreibendem Text im selben Absatz.
#[test]
fn generic_link_with_text_next_to_it_is_low() {
    let arena = with_body(|b| {
        b.open("p")
            .text("Zum Blogartikel über Barrierefreiheit ")
            .open("a")
            .attr("href", "/blog")
            .text("weiter")
            .close()
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    let g = findings(&r, "links/generic-name");
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].severity, a11y_report::Severity::Low);
}

/// auditmysite Plan 71 (inros-lackner.de): Teaserkarte, Überschrift zwei
/// Ebenen über dem Link, davor — Kontext nach H80. Danach zählt sie nicht.
#[test]
fn heading_before_the_link_is_context_after_it_is_not() {
    let card = |before: bool| {
        with_body(|b| {
            let b = b.open("div");
            let b = if before {
                b.open("h3").text("Building Information Modeling").close()
            } else {
                b
            };
            let b = b
                .open("div")
                .open("a")
                .attr("href", "/bim")
                .text("Mehr erfahren")
                .close()
                .close();
            let b = if before {
                b
            } else {
                b.open("h3").text("Nächster Abschnitt").close()
            };
            b.close()
        })
    };
    let with = card(true);
    let r = run_with_semantics(&WithSemantics::new(&with));
    assert_eq!(
        findings(&r, "links/generic-name")[0].severity,
        a11y_report::Severity::Low
    );
    let without = card(false);
    let r = run_with_semantics(&WithSemantics::new(&without));
    assert_eq!(
        findings(&r, "links/generic-name")[0].severity,
        a11y_report::Severity::Medium
    );
}

/// Eine Adresse als Linktext: ohne Kontext gemeldet, mit Text daneben nicht.
#[test]
fn address_as_link_text() {
    let arena = with_body(|b| {
        b.open("div")
            .open("a")
            .attr("href", "https://example.com/page")
            .attr("id", "nackt")
            .text("https://example.com/page")
            .close()
            .close()
            .open("p")
            .text("Go to ")
            .open("a")
            .attr("href", "https://example.com")
            .text("www.example.com")
            .close()
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(findings(&r, "links/url-name").len(), 1, "{:?}", ids(&r));
}

/// Ein Zeichen ist kein Name; Seitenzahlen einer Blätternavigation schon.
#[test]
fn single_character_but_no_page_number() {
    let arena = with_body(|b| {
        b.open("a")
            .attr("href", "/x")
            .text("x")
            .close()
            .open("a")
            .attr("href", "/seite/2")
            .text("2")
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(findings(&r, "links/single-character-name").len(), 1);
}

/// G201: `target="_blank"` ohne Ankündigung. auditmysite prüfte
/// `aria-haspopup` und traf diesen Fall nie.
#[test]
fn new_window_without_announcement() {
    let arena = with_body(|b| {
        b.open("a")
            .attr("href", "/a")
            .attr("target", "_blank")
            .text("Preisliste")
            .close()
            .open("a")
            .attr("href", "/b")
            .attr("target", "_blank")
            .text("Preisliste (öffnet in neuem Fenster)")
            .close()
            .open("a")
            .attr("href", "/c")
            .attr("aria-haspopup", "true")
            .text("Menü")
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    let f = findings(&r, "links/new-window-unannounced");
    assert_eq!(f.len(), 1, "{:?}", ids(&r));
    assert_eq!(f[0].wcag, vec!["3.2.5".to_string()]);
}

/// 2.4.9 (AAA): Zusammenhang hilft hier nicht.
#[test]
fn only_link_text_ignores_context() {
    let arena = with_body(|b| {
        b.open("p")
            .text("Unser Jahresbericht 2026: ")
            .open("a")
            .attr("href", "/bericht")
            .text("Öffnen")
            .close()
            .close()
            .open("a")
            .attr("href", "/agb")
            .text("Allgemeine Geschäftsbedingungen")
            .close()
    });
    let r = run_with_semantics(&WithSemantics::new(&arena));
    assert_eq!(findings(&r, "links/purpose-link-only").len(), 1);
}
