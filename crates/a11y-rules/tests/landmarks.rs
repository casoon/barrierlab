//! Landmarks, Tastaturerreichbarkeit, Dialogfokus und `headings/none`
//! (casoon/barrierlab#17).
//!
//! Die Fälle stammen aus auditmysite: dem Detection-Korpus
//! (`tests/fixtures/detection_corpus/`), den Tests der portierten Regeln und
//! den Issues #639, #642, #709 und #727; der Name jedes Tests nennt die
//! Quelle. Wo ein Korpusfall ein Element ohne `id` prüft, ist eine `id`
//! ergänzt, damit der Test den Befund verorten kann. Jeder Fall läuft gegen
//! zwei Hosts: Rolle und Name aus `accname` und einen, der sich wie Chrome
//! verhält — samt der Eigenheiten, die die Fehlalarme ausgelöst haben
//! (`<header>`/`<footer>` immer `banner`/`contentinfo`, unbenanntes `<form>`
//! mit Rolle `form`, `<summary>` als `DisclosureTriangle`).

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, Node, Semantics};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run_with_semantics_in;
use accname::IdIndex;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn html(body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body>{body}</body></html>"
    ))
    .document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    uebernehmen(&doc, wurzel, Arena::builder()).build()
}

fn uebernehmen(
    doc: &html5_parser::Document,
    id: html5_parser::NodeId,
    b: ArenaBuilder,
) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.close()
        }
        H::Text { content } => b.text(content),
        H::DocumentFragment => doc.children(id).fold(b, |b, k| uebernehmen(doc, k, b)),
        _ => b,
    }
}

struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    chrome: bool,
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
        if self.chrome && !node.has_attr("role") {
            match node.local_name() {
                // Die Eigenheiten, die auditmysite#639 und #727 auslösten.
                "header" => return Some("banner".into()),
                "footer" => return Some("contentinfo".into()),
                "form" => return Some("form".into()),
                "section" => return Some("section".into()),
                "summary" => return Some("DisclosureTriangle".into()),
                "img" => return Some("image".into()),
                "option" if a11y_dom::closest(node, "select").is_some() => {
                    return Some("MenuListOption".into());
                }
                _ => {}
            }
        }
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

fn beide_in(src: &str, locale: a11y_rules::Locale) -> [Report; 2] {
    let a = html(src);
    [false, true].map(|chrome| {
        run_with_semantics_in(
            &MitSemantik {
                doc: &a,
                ids: IdIndex::build(a.root()),
                chrome,
            },
            locale,
        )
    })
}

fn beide(src: &str) -> [Report; 2] {
    beide_in(src, a11y_rules::Locale::En)
}

fn anzahl(r: &Report, id: &str) -> usize {
    r.findings.iter().filter(|f| f.rule_id == id).count()
}

fn urteile(r: &Report, id: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

/// Die Befunde einer Kennung, verortet an der `id` des Elements.
fn an(src: &str, r: &Report, rule: &str) -> Vec<String> {
    let a = html(src);
    let mut v: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| {
            let knoten = f.location.node.as_deref()?;
            a11y_dom::self_and_descendants(a.root())
                .find(|n| n.id().to_string() == knoten)
                .map(|n| n.attr("id").unwrap_or(n.local_name()).to_string())
        })
        .collect();
    v.sort();
    v
}

// --- landmarks/not-unique, not-top-level, *-duplicate ------------------------

/// Korpus `landmark_granular`.
const LANDMARK_GRANULAR: &str = r#"
    <nav id="nav1">
        <div role="banner" id="b-nested">Banner nested inside a nav landmark (not top-level)</div>
        <div role="main" id="m-nested">Main nested inside a nav landmark (not top-level)</div>
    </nav>
    <header id="h1">First header (duplicate banner 1 of 2)</header>
    <header id="h2">Second header (duplicate banner 2 of 2)</header>
    <footer id="f1">First footer (duplicate contentinfo 1 of 2)</footer>
    <footer id="f2">Second footer (duplicate contentinfo 2 of 2)</footer>
    <main id="m1"><h1>First main region (duplicate main 1 of 2)</h1></main>
    <div role="main" id="m2">Second main region (duplicate main 2 of 2)</div>
    <nav id="navA">Unnamed nav A</nav>
    <nav id="navB">Unnamed nav B</nav>"#;

#[test]
fn korpus_landmark_granular() {
    let src = LANDMARK_GRANULAR;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "landmarks/not-top-level"),
            ["b-nested", "m-nested"]
        );
        assert_eq!(
            urteile(&r, "landmarks/not-top-level")[0],
            (Outcome::Fail, Severity::Medium)
        );
        // Die drei banner: die verschachtelte und zwei <header>.
        assert_eq!(an(src, &r, "landmarks/banner-duplicate"), ["h1"]);
        assert_eq!(
            urteile(&r, "landmarks/banner-duplicate"),
            [(Outcome::Fail, Severity::Medium)]
        );
        assert_eq!(an(src, &r, "landmarks/contentinfo-duplicate"), ["f2"]);
        assert_eq!(anzahl(&r, "landmarks/main-duplicate"), 1);
        // auditmysite: 11 (3 navigation, 3 banner, 3 main, 2 contentinfo).
        assert_eq!(anzahl(&r, "landmarks/not-unique"), 11);
        assert!(
            urteile(&r, "landmarks/not-unique")
                .iter()
                .all(|u| *u == (Outcome::Review, Severity::Medium))
        );
    }
}

/// Korpus `landmark_header_in_main`, auditmysite#639: Nur `<header>` und
/// `<footer>` auf Dokumentebene sind `banner`/`contentinfo`.
const HEADER_IN_MAIN: &str = r#"
    <header class="site-header">
        <nav aria-label="Main navigation"><a href="/">Home</a></nav>
    </header>
    <main id="main">
        <div class="chapter">
            <div class="content">
                <header class="grid">
                    <h1>Chapter title</h1>
                    <p>Chapter introduction.</p>
                </header>
                <article>
                    <header><h2>Article heading</h2></header>
                    <p>Article text.</p>
                    <footer>Article footer</footer>
                </article>
            </div>
        </div>
    </main>
    <footer class="site-footer">Site footer</footer>"#;

#[test]
fn korpus_landmark_header_in_main_issue_639() {
    for r in beide(HEADER_IN_MAIN) {
        for id in [
            "landmarks/not-unique",
            "landmarks/banner-duplicate",
            "landmarks/contentinfo-duplicate",
            "landmarks/not-top-level",
            "landmarks/content-outside",
        ] {
            assert_eq!(anzahl(&r, id), 0, "{id}: {:?}", r.findings);
        }
    }
}

/// auditmysite#639, zweiter Fall (barrierlab.eu/experience/): `<header>` und
/// `<footer>` in einem Muster-`div` in `main`; ebenso unter `role="main"`.
#[test]
fn header_footer_im_muster_div_issue_639() {
    let src = r#"
        <header>Site</header>
        <main><h1>Experience</h1>
            <div class="specimen"><header>Demo header</header><footer>Demo footer</footer></div>
        </main>
        <div role="main" id="zweite"><header>In role=main</header></div>
        <footer>Site footer</footer>"#;
    for r in beide(src) {
        assert_eq!(
            anzahl(&r, "landmarks/banner-duplicate"),
            0,
            "{:?}",
            r.findings
        );
        assert_eq!(anzahl(&r, "landmarks/contentinfo-duplicate"), 0);
        assert_eq!(anzahl(&r, "landmarks/not-top-level"), 0);
    }
}

/// Korpus `landmark_aside_scoping`: ein unbenanntes `<aside>` in `article`
/// oder `section` ist `generic`; es bleiben zwei `complementary` mit
/// verschiedenen Namen.
#[test]
fn korpus_landmark_aside_scoping() {
    let src = r#"
        <main>
            <h1>Aside scoping</h1>
            <article>
                <h2>Article with a note</h2>
                <p>Article text.</p>
                <aside>Unnamed aside in an article (generic)</aside>
                <aside aria-label="Related reading">Named aside in an article (complementary)</aside>
            </article>
            <section aria-label="Background">
                <p>Section text.</p>
                <aside>Unnamed aside in a section (generic)</aside>
            </section>
        </main>
        <aside>Unnamed top-level aside (complementary)</aside>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "landmarks/not-unique"), 0, "{:?}", r.findings);
    }
}

/// Korpus `landmark_aside_named_duplicate`: ein benanntes `<aside>` bleibt
/// auch im Artikel `complementary`, `title` zählt als Name.
#[test]
fn korpus_landmark_aside_named_duplicate() {
    let src = r#"
        <main>
            <h1>Two articles</h1>
            <article><h2>First article</h2>
                <aside id="a1" aria-label="Related">Related to the first article</aside>
            </article>
            <article><h2>Second article</h2>
                <aside id="a2" title="Related">Related to the second article</aside>
            </article>
        </main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/not-unique"), ["a1", "a2"]);
    }
}

/// Korpus `landmark_unique_one_per_element`: vier betroffene Elemente, vier
/// Befunde.
#[test]
fn korpus_landmark_unique_one_per_element() {
    let src = r#"
        <header><nav id="n1" aria-label="Primary"><a href="/">Home</a></nav></header>
        <main>
            <h1>Landmark names</h1>
            <nav id="n2" aria-label="Primary"><a href="/docs">Docs</a></nav>
            <section id="s1" aria-label="Details"><p>First details block.</p></section>
            <section id="s2" aria-label="Details"><p>Second details block.</p></section>
        </main>
        <footer><nav aria-label="Legal"><a href="/imprint">Imprint</a></nav></footer>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "landmarks/not-unique"),
            ["n1", "n2", "s1", "s2"]
        );
    }
}

/// Korpus `landmarks_and_lists`: ein `<footer role="contentinfo">` in `main`.
#[test]
fn korpus_landmarks_and_lists() {
    let src = r#"
        <header>Site header</header>
        <main>
            <h1>Landmark nesting and list structure</h1>
            <footer role="contentinfo" id="nested">Nested footer</footer>
        </main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/not-top-level"), ["nested"]);
    }
}

/// auditmysite#727: `<form>` und `<section>` sind nur mit Namen Landmarks.
#[test]
fn unbenannte_formulare_sind_keine_landmarks_issue_727() {
    let src = r#"
        <main><h1>Experience</h1>
            <form class="demo__controls"><button>Go</button></form>
            <form id="booking-form"><button>Book</button></form>
            <section><p>One</p></section>
            <section><p>Two</p></section>
        </main>
        <form><div role="banner" id="b">Banner in an unnamed form</div></form>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "landmarks/not-unique"), 0, "{:?}", r.findings);
        assert_eq!(anzahl(&r, "landmarks/not-top-level"), 0);
    }

    // Gegenprobe: benannt sind es Landmarks — gleichnamig nicht
    // unterscheidbar, und ein banner darin ist verschachtelt.
    let src = r#"
        <main><h1>Search</h1>
            <form id="f1" aria-label="Search"><button>Go</button></form>
            <form id="f2" aria-label="Search"><button>Go</button></form>
        </main>
        <form aria-label="Outer"><div role="banner" id="b">Banner</div></form>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/not-unique"), ["f1", "f2"]);
        assert_eq!(an(src, &r, "landmarks/not-top-level"), ["b"]);
    }
}

#[test]
fn verschiedene_namen_sind_unterscheidbar() {
    let src = r#"
        <nav aria-label="Primary"><a href="/">Home</a></nav>
        <main><h1>X</h1></main>
        <nav aria-label="Footer"><a href="/i">Imprint</a></nav>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "landmarks/not-unique"), 0);
    }
}

// --- landmarks/content-outside -------------------------------------------------

/// Korpus `missing_main_landmark`: ohne jede Landmark steht aller Inhalt
/// außerhalb; Überschriften gibt es (`bypass` besteht).
#[test]
fn korpus_missing_main_landmark() {
    let src = r#"
        <div id="wrapper">
            <h1>Page without a main landmark</h1>
            <p>No main element or role="main" anywhere on this page.</p>
        </div>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/content-outside"), ["wrapper"]);
        assert_eq!(
            urteile(&r, "landmarks/content-outside"),
            [(Outcome::Fail, Severity::Medium)]
        );
        assert_eq!(anzahl(&r, "headings/none"), 0);
    }
}

/// Korpus `skip_link_language`, auditmysite#642: Der Sprunglink wird am Ziel
/// erkannt, nicht am (französischen) Text. Derselbe Link nach den regulären
/// Links ist gewöhnlicher Inhalt.
#[test]
fn korpus_skip_link_language_issue_642() {
    let src = r##"
        <a href="#main" class="skip-link" id="skip">Aller au contenu</a>
        <header><a href="/">Accueil</a></header>
        <nav aria-label="Navigation principale">
            <ul><li><a href="/apprendre/">Apprendre</a></li><li><a href="/outils/">Outils</a></li></ul>
        </nav>
        <main id="main"><h1>L'accessibilité numérique</h1><p>Le contenu principal de la page.</p></main>
        <div id="stray"><a href="#main" id="stray-link">Aller au contenu</a></div>
        <footer><p>Pied de page</p></footer>"##;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/content-outside"), ["stray"]);
        assert_eq!(anzahl(&r, "keyboard/skip-link-missing"), 0);
    }

    // Eingepackt in einen Container gilt dasselbe.
    let src = r##"
        <div class="skip"><a href="#main">Aller au contenu</a><a href="#nav">Aller à la navigation</a></div>
        <nav id="nav" aria-label="Site"><a href="/">Accueil</a></nav>
        <main id="main"><h1>Titre</h1></main>"##;
    for r in beide(src) {
        assert_eq!(
            anzahl(&r, "landmarks/content-outside"),
            0,
            "{:?}",
            r.findings
        );
    }
}

/// auditmysite `region`: Inhalt in einer Landmark besteht, die Landmark
/// selbst wird nicht gemeldet, ein Fragmentlink nach einem regulären Link
/// schon.
#[test]
fn region_einzelfaelle() {
    let src = "<main><h1>Inside</h1></main><nav aria-label=\"Site\"></nav>";
    for r in beide(src) {
        assert_eq!(anzahl(&r, "landmarks/content-outside"), 0);
    }

    let src = r##"
        <a href="/start" id="home">Home</a>
        <a href="#top" id="top">Skip to content</a>
        <main><h1>X</h1></main>"##;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/content-outside"), ["home", "top"]);
    }

    // Text unmittelbar neben einer Landmark: der Container trägt den Befund,
    // einmal. Versteckter, leerer und Skript-Inhalt zählen nicht.
    let src = r#"
        <div id="page">Loose text<main><h1>X</h1></main>more text</div>
        <div hidden>Hidden</div>
        <div aria-hidden="true">Decorative</div>
        <div id="leer"><span> </span></div>
        <script>var x = 1;</script>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/content-outside"), ["page"]);
    }

    // Ein benanntes Bild zählt als Inhalt, ein dekoratives nicht.
    let src = r#"
        <div id="logo"><img src="l.png" alt="Logo"></div>
        <div><img src="d.png" alt=""></div>
        <main><h1>X</h1></main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "landmarks/content-outside"), ["logo"]);
    }
}

// --- keyboard/* ------------------------------------------------------------------

/// Korpus `keyboard_and_targets`: ein `div` mit `tabindex="0"`.
#[test]
fn korpus_keyboard_and_targets() {
    let src = r#"
        <main>
            <h1>Keyboard access and target size issues</h1>
            <div tabindex="0" id="fokus">Focusable div without an interactive role</div>
            <div onclick="console.log('clicked')">Clickable div with no keyboard handler</div>
            <p><button id="tight-a">X</button><button id="tight-b">Y</button></p>
            <p>Read the <a id="inline" href="/terms">terms of use</a> before you continue.</p>
        </main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "keyboard/focusable-no-role"), ["fokus"]);
        assert_eq!(
            urteile(&r, "keyboard/focusable-no-role"),
            [(Outcome::Review, Severity::Low)]
        );
        assert_eq!(anzahl(&r, "keyboard/interactive-not-focusable"), 0);
    }
}

/// Korpus `patterns_disclosure` und `widget_patterns`: interaktive Rollen
/// ohne Fokus.
#[test]
fn korpus_interaktiv_ohne_fokus() {
    let src = r#"
        <main>
            <h1>Accordion, disclosure menu, and focus order</h1>
            <span role="link" aria-expanded="true" id="span-link">Non-button accordion trigger</span>
            <span role="link" tabindex="0">Main Menu <button tabindex="0">Item</button></span>
            <div role="tablist" aria-label="Sections">
                <div role="tab" id="tab1">Tab 1</div>
            </div>
            <ul id="listbox1" role="listbox"><li role="option" id="apple">Apple</li></ul>
        </main>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "keyboard/interactive-not-focusable"),
            ["apple", "span-link", "tab1"]
        );
        assert!(
            urteile(&r, "keyboard/interactive-not-focusable")
                .iter()
                .all(|u| *u == (Outcome::Review, Severity::High))
        );
        assert_eq!(anzahl(&r, "keyboard/focusable-no-role"), 0);
    }
}

/// Was nicht gemeldet wird: Ziel eines Sprunglinks (`tabindex="-1"`),
/// benannter scrollbarer Bereich (Korpus `scrollable_region_focusable`),
/// Tabs mit rovingem `tabindex`, Optionen unter `aria-activedescendant`,
/// deaktivierte Felder, native `<option>`, Inertes.
#[test]
fn keyboard_ausnahmen() {
    let src = r#"
        <main id="main" tabindex="-1">
            <h1>X</h1>
            <div id="content" tabindex="-1">Skip target</div>
            <div class="box" tabindex="0" role="region" aria-label="Terms"><div class="tall">Terms text.</div></div>
            <div role="tablist" aria-label="T">
                <button role="tab" aria-selected="true">A</button>
                <div role="tab" tabindex="-1">B</div>
            </div>
            <div role="listbox" tabindex="0" aria-label="Fruit" aria-activedescendant="o1">
                <div role="option" id="o1">Apple</div><div role="option">Pear</div>
            </div>
            <button role="tab" disabled>Off</button>
            <select aria-label="S"><option>One</option></select>
            <div inert><div role="button">Inert</div><div tabindex="0">Inert</div></div>
            <div contenteditable="true">Edit</div>
        </main>"#;
    for r in beide(src) {
        assert_eq!(
            anzahl(&r, "keyboard/focusable-no-role"),
            0,
            "{:?}",
            r.findings
        );
        assert_eq!(
            anzahl(&r, "keyboard/interactive-not-focusable"),
            0,
            "{:?}",
            r.findings
        );
    }
}

// --- dialog/focusable-missing ----------------------------------------------------

/// Korpus `aria_naming_roles`: ein leerer `role="dialog"`.
#[test]
fn korpus_dialog_ohne_fokussierbares() {
    let src = r#"<main><h1>X</h1><div role="dialog" id="leer"></div></main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "dialog/focusable-missing"), ["leer"]);
        assert_eq!(
            urteile(&r, "dialog/focusable-missing"),
            [(Outcome::Fail, Severity::Medium)]
        );
    }

    // Ein Button tief im Dialog genügt; ein geschlossenes <dialog> ist nicht
    // da; nur ein deaktivierter Button genügt nicht.
    let src = r#"
        <main><h1>X</h1>
            <div role="dialog" aria-label="A"><div><p>Text <button>Close</button></p></div></div>
            <dialog aria-label="B"><p>Closed</p></dialog>
            <dialog open aria-label="C" id="aus"><button disabled>OK</button></dialog>
        </main>"#;
    for r in beide(src) {
        assert_eq!(an(src, &r, "dialog/focusable-missing"), ["aus"]);
    }
}

// --- patterns/accordion-controls-missing ------------------------------------------

/// Korpus `patterns_disclosure` und `widget_patterns`: aufgeklappter Button
/// ohne `aria-controls`; zugeklappt nicht.
#[test]
fn korpus_accordion_ohne_controls() {
    let src = r#"
        <main><h1>X</h1>
            <button aria-expanded="true" id="offen">Accordion trigger without controls</button>
            <button aria-expanded="false">Section 2</button>
            <button aria-expanded="true" aria-controls="p">With controls</button><div id="p">Panel</div>
            <details open><summary aria-expanded="true">Native</summary>Body</details>
        </main>
        <nav aria-label="Site"><button aria-expanded="true">Menu</button><ul><li><a href="/">Home</a></li></ul></nav>
        <header><button aria-expanded="true">Search</button></header>"#;
    for r in beide(src) {
        assert_eq!(
            an(src, &r, "patterns/accordion-controls-missing"),
            ["offen"]
        );
        assert_eq!(
            urteile(&r, "patterns/accordion-controls-missing"),
            [(Outcome::Review, Severity::Low)]
        );
    }
}

// --- headings/none ------------------------------------------------------------

/// auditmysite `bypass_blocks`: eine Seite ganz ohne Überschriften.
#[test]
fn seite_ohne_ueberschriften() {
    let src = "<main><p>Some text</p></main>";
    for r in beide(src) {
        assert_eq!(
            urteile(&r, "headings/none"),
            [(Outcome::Fail, Severity::Medium)]
        );
        assert_eq!(
            r.findings
                .iter()
                .find(|f| f.rule_id == "headings/none")
                .unwrap()
                .wcag,
            ["2.4.1"]
        );
        assert_eq!(anzahl(&r, "headings/h1-missing"), 0);
    }

    // role="heading" zählt; versteckte Überschriften nicht.
    let src = r#"<main><div role="heading" aria-level="1">Title</div></main>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "headings/none"), 0);
    }
    let src = r#"<main><h1 hidden>Hidden</h1><p>Text</p></main>"#;
    for r in beide(src) {
        assert_eq!(anzahl(&r, "headings/none"), 1);
    }
}

/// auditmysite#709: Hinter einem offenen Dialog fehlen die Überschriften für
/// den Moment der Messung — ein Hinweis, kein Verstoß.
#[test]
fn ohne_ueberschriften_hinter_offenem_dialog_issue_709() {
    let src = r#"
        <div role="dialog" aria-label="Cookies"><button>Accept</button></div>
        <main aria-hidden="true"><h1>Behind the dialog</h1></main>"#;
    for r in beide(src) {
        assert_eq!(
            urteile(&r, "headings/none"),
            [(Outcome::Review, Severity::Low)]
        );
    }
}

// --- Sprache --------------------------------------------------------------------

#[cfg(feature = "de")]
#[test]
fn texte_auf_deutsch() {
    let src = LANDMARK_GRANULAR;
    let [en, _] = beide(src);
    let [de, _] = beide_in(src, a11y_rules::Locale::De);
    let texte = |r: &Report| -> Vec<String> {
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("landmarks/"))
            .map(|f| f.message.clone())
            .collect()
    };
    let (en, de) = (texte(&en), texte(&de));
    assert_eq!(en.len(), de.len());
    assert!(de.iter().any(|t| t.contains("Landmark")));
    assert!(en.iter().zip(&de).all(|(e, d)| e != d));
}
