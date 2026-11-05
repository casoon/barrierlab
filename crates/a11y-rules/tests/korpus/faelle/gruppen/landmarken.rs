//! Gruppe „landmarken“: Landmarks (structure.rs `landmarks`, landmarks.rs),
//! Tastaturerreichbarkeit, Dialogfokus und Akkordeon-Verweise.
//!
//! Die Landmark-Regeln sind nach docs/packages/a11y-rules.md („Normative
//! Zuordnung“, auditmysite#779) Best Practice ohne Kriterium (`FAIL` mit
//! Schlagwort `best-practice`) bzw. `REVIEW`; die Erwartungen folgen dieser
//! dokumentierten Entscheidung, die Rollen selbst WAI-ARIA 1.2 und HTML-AAM.

use super::super::{Fall, Mode, fail, na, negative, review};

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(main_missing());
    v.extend(main_duplicate());
    v.extend(page_wide());
    v.extend(not_unique());
    v.extend(not_top_level());
    v.extend(duplicates());
    v.extend(content_outside());
    v.extend(keyboard());
    v.extend(dialog());
    v.extend(accordion());
    v
}

fn main_missing() -> Vec<Fall> {
    let id = "landmarks/main-missing";
    vec![
        fail(
            id,
            r#"<header><p>Site</p></header><p>Body text</p>"#,
            "axe landmark-one-main (Best Practice, FAIL mit best-practice); docs/packages/a11y-rules.md, Normative Zuordnung (auditmysite#779)",
        )
        .count(1),
        fail(
            id,
            r#"<main hidden><p>Old</p></main><p>Visible text</p>"#,
            "HTML: ein main mit hidden ist nicht dargestellt und keine Landmark im AX-Baum; axe landmark-one-main",
        )
        .variant("versteckt per hidden"),
        fail(
            id,
            r#"<main aria-hidden="true"><p>Hidden from AT</p></main><p>Visible text</p>"#,
            "WAI-ARIA 1.2 aria-hidden: der Teilbaum fehlt im AX-Baum, damit auch die Landmark",
        )
        .variant("aria-hidden"),
        fail(
            id,
            r#"<main hidden><p>Old</p></main><p>Visible text</p>"#,
            "HTML hidden; Host ohne Stile wertet das Attribut aus",
        )
        .variant("versteckt per hidden, nur Semantik")
        .mode(Mode::Semantics),
        review(
            id,
            r#"<div role="dialog" aria-label="Consent"><p>Accept cookies?</p><button>OK</button></div><div aria-hidden="true"><main><p>Content</p></main></div>"#,
            "docs/packages/a11y-rules.md: hinter offenem Modal REVIEW statt FAIL (auditmysite#709)",
        )
        .variant("offener ARIA-Dialog"),
        review(
            id,
            r#"<dialog open><p>Accept cookies?</p><button>OK</button></dialog><div aria-hidden="true"><main><p>Content</p></main></div>"#,
            "HTML dialog open; auditmysite#709",
        )
        .variant("offenes dialog-Element"),
        negative(
            id,
            r#"<header><p>Site</p></header><main><p>Body text</p></main>"#,
            "WAI-ARIA 1.2 main; HTML-AAM main → main",
        ),
        negative(
            id,
            r#"<div role="main"><p>Body text</p></div>"#,
            "WAI-ARIA 1.2 role=main; WCAG-Technik ARIA11",
        )
        .variant("ARIA-Rolle statt Element"),
        negative(
            id,
            r#"<div><template shadowrootmode="open"><main><p>Body text</p></main></template></div>"#,
            "DOM: Inhalt einer offenen Shadow Root steht im flachen Baum und damit im AX-Baum",
        )
        .variant("Shadow DOM"),
        na(
            id,
            "",
            "Leere Seite: kein Inhalt, der in eine Landmark gehörte (WAI-ARIA 1.2, Landmark: eine Region mit Inhalt)",
        )
        .variant("Regression: leerer body"),
        na(
            id,
            r#"<p hidden>Not rendered</p><script>var a = 1;</script>"#,
            "HTML hidden, script: nichts Dargestelltes im body",
        )
        .variant("Regression: nur Verstecktes"),
    ]
}

fn main_duplicate() -> Vec<Fall> {
    let id = "landmarks/main-duplicate";
    vec![
        fail(
            id,
            r#"<main><p>A</p></main><main><p>B</p></main>"#,
            "HTML: höchstens ein main ohne hidden; axe landmark-no-duplicate-main (Best Practice); docs/packages/a11y-rules.md",
        )
        .count(1),
        fail(
            id,
            r#"<main><p>A</p></main><div role="main"><p>B</p></div>"#,
            "WAI-ARIA 1.2 main: ein Dokument SHOULD NOT mehr als eine main-Landmark haben",
        )
        .variant("Element und ARIA-Rolle"),
        negative(
            id,
            r#"<main><p>A</p></main><main hidden><p>B</p></main>"#,
            "HTML, main-Element: mehrere main erlaubt, wenn alle bis auf eines hidden tragen",
        )
        .variant("zweites main versteckt"),
        negative(
            id,
            r#"<main><p>A</p></main><section><p>B</p></section>"#,
            "WAI-ARIA 1.2: genau eine main-Landmark",
        ),
        na(
            id,
            r#"<p>No main landmark at all</p>"#,
            "Ohne main nichts zu verdoppeln",
        ),
    ]
}

/// navigation, banner, contentinfo: REVIEW, weil eine Seite ohne sie
/// auskommen darf (Kommentar der Regel; WCAG 1.3.1/2.4.1 verlangen keine
/// bestimmte Landmark).
fn page_wide() -> Vec<Fall> {
    let nav = "landmarks/navigation-missing";
    let banner = "landmarks/banner-missing";
    let info = "landmarks/contentinfo-missing";
    vec![
        review(
            nav,
            r#"<main><p>See <a href="/a">A</a></p></main>"#,
            "WAI-ARIA 1.2 navigation; Erwartung, kein Beleg: REVIEW (Regelkommentar, WCAG 2.4.1 ARIA11)",
        )
        .count(1),
        review(
            nav,
            r#"<nav aria-hidden="true"><a href="/">Home</a></nav><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 aria-hidden: die Landmark fehlt im AX-Baum",
        )
        .variant("aria-hidden"),
        negative(
            nav,
            r#"<nav><a href="/">Home</a></nav><main><p>Text</p></main>"#,
            "HTML-AAM nav → navigation",
        ),
        negative(
            nav,
            r#"<div role="navigation"><a href="/">Home</a></div><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 role=navigation",
        )
        .variant("ARIA-Rolle"),
        na(nav, "", "Leere Seite: nichts, was Navigation bräuchte")
            .variant("Regression: leerer body"),
        review(
            banner,
            r#"<main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 banner; Erwartung, kein Beleg: REVIEW",
        )
        .count(1),
        review(
            banner,
            r#"<main><article><header><h2>Post</h2></header><p>Text</p></article></main>"#,
            "HTML-AAM header: nur außerhalb von article/aside/main/nav/section → banner",
        )
        .variant("header im Abschnitt"),
        negative(
            banner,
            r#"<header><p>Site</p></header><main><p>Text</p></main>"#,
            "HTML-AAM header (dokumentweit) → banner",
        ),
        negative(
            banner,
            r#"<div><header><p>Site</p></header></div><main><p>Text</p></main>"#,
            "HTML-AAM: ein div begrenzt header nicht",
        )
        .variant("header im div"),
        negative(
            banner,
            r#"<div role="banner"><p>Site</p></div><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 role=banner",
        )
        .variant("ARIA-Rolle"),
        na(banner, "", "Leere Seite: nichts für einen Kopfbereich")
            .variant("Regression: leerer body"),
        review(
            info,
            r#"<main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 contentinfo; Erwartung, kein Beleg: REVIEW",
        )
        .count(1),
        review(
            info,
            r#"<main><article><p>Text</p><footer><p>Posted today</p></footer></article></main>"#,
            "HTML-AAM footer: nur außerhalb von article/aside/main/nav/section → contentinfo",
        )
        .variant("footer im Abschnitt"),
        negative(
            info,
            r#"<main><p>Text</p></main><footer><p>© ACME</p></footer>"#,
            "HTML-AAM footer (dokumentweit) → contentinfo",
        ),
        negative(
            info,
            r#"<main><p>Text</p></main><div role="contentinfo"><p>© ACME</p></div>"#,
            "WAI-ARIA 1.2 role=contentinfo",
        )
        .variant("ARIA-Rolle"),
        na(info, "", "Leere Seite: nichts für einen Fußbereich")
            .variant("Regression: leerer body"),
    ]
}

fn not_unique() -> Vec<Fall> {
    let id = "landmarks/not-unique";
    vec![
        review(
            id,
            r#"<nav><a href="/a">A</a></nav><main><p>x</p></main><nav><a href="/b">B</a></nav>"#,
            "WAI-ARIA 1.2 navigation: mehrere Landmarks derselben Rolle SHOULD unterschiedliche Namen tragen (SHOULD → REVIEW, Regelkommentar)",
        )
        .count(2),
        review(
            id,
            r#"<nav aria-label="Menu"><a href="/a">A</a></nav><nav aria-label="menu"><a href="/b">B</a></nav>"#,
            "WAI-ARIA 1.2: gleicher Name (Groß-/Kleinschreibung unerheblich) unterscheidet nicht",
        )
        .variant("gleicher Name")
        .count(2),
        negative(
            id,
            r#"<nav aria-label="Main"><a href="/a">A</a></nav><nav aria-label="Footer"><a href="/b">B</a></nav>"#,
            "WAI-ARIA 1.2 / APG Landmarks: verschiedene Namen per aria-label",
        ),
        negative(
            id,
            r#"<nav aria-labelledby="n1"><h2 id="n1">Sections</h2><a href="/a">A</a></nav><nav aria-labelledby="n2"><h2 id="n2">Related</h2><a href="/b">B</a></nav>"#,
            "accname 1.2 aria-labelledby; ARIA13",
        )
        .variant("aria-labelledby"),
        negative(
            id,
            r#"<nav><a href="/a">A</a></nav><nav hidden><a href="/b">B</a></nav>"#,
            "HTML hidden: die zweite Landmark ist nicht im AX-Baum",
        )
        .variant("versteckt per hidden"),
        negative(
            id,
            r#"<section><p>A</p></section><section><p>B</p></section><nav><a href="/">x</a></nav><aside><p>y</p></aside>"#,
            "HTML-AAM: section ohne Namen ist keine region-Landmark; verschiedene Rollen sind unterscheidbar",
        )
        .variant("unbenannte section"),
        na(
            id,
            r#"<p>No landmarks</p>"#,
            "Ohne Landmarks nichts zu unterscheiden",
        ),
    ]
}

fn not_top_level() -> Vec<Fall> {
    let id = "landmarks/not-top-level";
    vec![
        fail(
            id,
            r#"<section aria-label="Intro"><main><p>x</p></main></section>"#,
            "WAI-ARIA 1.2 main: SHOULD top-level; axe landmark-main-is-top-level (Best Practice); benannte section ist region (HTML-AAM)",
        )
        .count(1),
        fail(
            id,
            r#"<main><p>x</p></main><aside><div role="contentinfo"><p>c</p></div></aside>"#,
            "WAI-ARIA 1.2 contentinfo: nicht in einer anderen Landmark; axe landmark-contentinfo-is-top-level",
        )
        .variant("contentinfo per Rolle in complementary"),
        negative(
            id,
            r#"<main><header><h1>T</h1></header><p>x</p><footer><p>f</p></footer></main>"#,
            "HTML-AAM: header/footer in main sind weder banner noch contentinfo",
        ),
        negative(
            id,
            r#"<section><main><p>x</p></main></section>"#,
            "HTML-AAM: section ohne Namen ist keine Landmark (auditmysite#727)",
        )
        .variant("unbenannte section"),
        na(
            id,
            r#"<nav><a href="/">Home</a></nav><p>x</p>"#,
            "Kein main, banner oder contentinfo",
        ),
    ]
}

fn duplicates() -> Vec<Fall> {
    let banner = "landmarks/banner-duplicate";
    let info = "landmarks/contentinfo-duplicate";
    vec![
        fail(
            banner,
            r#"<header><p>A</p></header><main><p>x</p></main><header><p>B</p></header>"#,
            "WAI-ARIA 1.2 banner: höchstens eine; axe landmark-no-duplicate-banner (Best Practice); docs/packages/a11y-rules.md",
        )
        .count(1),
        fail(
            banner,
            r#"<header><p>A</p></header><div role="banner"><p>B</p></div>"#,
            "WAI-ARIA 1.2 banner per Rolle zählt mit",
        )
        .variant("Element und ARIA-Rolle"),
        negative(
            banner,
            r#"<header><p>Site</p></header><main><article><header><h2>Post</h2></header><p>x</p></article></main>"#,
            "HTML-AAM: header im article ist kein banner",
        ),
        negative(
            banner,
            r#"<header><p>A</p></header><header hidden><p>B</p></header>"#,
            "HTML hidden: nicht im AX-Baum",
        )
        .variant("versteckt per hidden"),
        na(banner, r#"<main><p>x</p></main>"#, "Kein banner"),
        fail(
            info,
            r#"<main><p>x</p></main><footer><p>A</p></footer><footer><p>B</p></footer>"#,
            "WAI-ARIA 1.2 contentinfo: höchstens eine; axe landmark-no-duplicate-contentinfo (Best Practice); docs/packages/a11y-rules.md",
        )
        .count(1),
        fail(
            info,
            r#"<footer><p>A</p></footer><div role="contentinfo"><p>B</p></div>"#,
            "WAI-ARIA 1.2 contentinfo per Rolle zählt mit",
        )
        .variant("Element und ARIA-Rolle"),
        negative(
            info,
            r#"<main><article><p>x</p><footer><p>Post</p></footer></article></main><footer><p>Site</p></footer>"#,
            "HTML-AAM: footer im article ist kein contentinfo",
        ),
        negative(
            info,
            r#"<footer><p>A</p></footer><footer style="display:none"><p>B</p></footer>"#,
            "CSS display:none: nicht dargestellt, nicht im AX-Baum",
        )
        .variant("versteckt per CSS"),
        na(info, r#"<main><p>x</p></main>"#, "Kein contentinfo"),
    ]
}

fn content_outside() -> Vec<Fall> {
    let id = "landmarks/content-outside";
    vec![
        fail(
            id,
            r#"<main><p>In</p></main><p>Out</p>"#,
            "axe region (Best Practice): Inhalt außerhalb jeder Landmark; docs/packages/a11y-rules.md",
        )
        .count(1),
        fail(
            id,
            r#"<div><img src="chart.png" alt="Sales chart"></div><main><p>In</p></main>"#,
            "axe region: ein benanntes Bild ist Inhalt",
        )
        .variant("benanntes Bild"),
        negative(
            id,
            r##"<a href="#main">Skip to content</a><header><p>Site</p></header><main id="main"><p>In</p></main><footer><p>f</p></footer>"##,
            "axe region: Sprunglinks vor den Landmarks sind ausgenommen (isSkipLink)",
        )
        .variant("Sprunglink"),
        negative(
            id,
            r#"<main><p>In</p></main><p hidden>Out</p><div aria-hidden="true">Decor</div>"#,
            "HTML hidden, aria-hidden: nicht im AX-Baum",
        )
        .variant("versteckt"),
        negative(
            id,
            r#"<main><p>In</p></main><img src="spacer.gif" alt="">"#,
            "HTML-AAM: img mit alt=\"\" ist präsentational, kein Inhalt",
        )
        .variant("dekoratives Bild"),
        na(id, r#"<script>var a = 1;</script>"#, "Kein Inhalt im body"),
    ]
}

fn keyboard() -> Vec<Fall> {
    let without = "keyboard/focusable-no-role";
    let not = "keyboard/interactive-not-focusable";
    vec![
        review(
            without,
            r#"<main><div tabindex="0">Card</div></main>"#,
            "WCAG 2.1.1, 4.1.2: in der Tabfolge ohne interaktive Rolle; Bedienbarkeit hängt am Skript → REVIEW (auditmysite#569)",
        )
        .count(1),
        review(
            without,
            r#"<main><a tabindex="0">Not a link</a></main>"#,
            "HTML-AAM: a ohne href ist generic; tabindex=0 macht es erreichbar",
        )
        .variant("a ohne href"),
        negative(
            without,
            r#"<main><div tabindex="0" role="button">Do</div><button>Native</button><a href="/x" tabindex="0">Link</a></main>"#,
            "WAI-ARIA 1.2 button; HTML: native Bedienelemente",
        ),
        negative(
            without,
            r#"<main><div tabindex="-1" id="target">Skip target</div></main>"#,
            "HTML tabindex=-1: fokussierbar, aber nicht in der sequenziellen Tabfolge",
        )
        .variant("tabindex=-1"),
        negative(
            without,
            r#"<main><div tabindex="0" role="region" aria-label="Scrollable table">Wide</div></main>"#,
            "ACT 0ssw9k / axe scrollable-region-focusable: benannter fokussierbarer Bereich ist das empfohlene Muster",
        )
        .variant("region"),
        negative(
            without,
            r#"<main><div tabindex="0" hidden>x</div><div inert><div tabindex="0">y</div></div></main>"#,
            "HTML hidden und inert: nicht fokussierbar",
        )
        .variant("versteckt/inert"),
        na(
            without,
            r#"<main><p>Text</p></main>"#,
            "Kein Element mit tabindex",
        ),
        review(
            not,
            r#"<main><div role="button">Save</div></main>"#,
            "WCAG 2.1.1; WAI-ARIA 1.2 button: interaktiv, aber nicht fokussierbar → REVIEW",
        )
        .count(1),
        review(
            not,
            r#"<main><span role="link">More</span><div role="checkbox" aria-checked="false">Agree</div></main>"#,
            "WAI-ARIA 1.2 link, checkbox: Widgets müssen fokussierbar sein (APG Keyboard Interface)",
        )
        .variant("link, checkbox")
        .count(2),
        negative(
            not,
            r#"<main><div role="button" tabindex="0">Save</div><button>Native</button><a href="/">Link</a></main>"#,
            "HTML tabindex=0 / native Bedienelemente sind fokussierbar",
        ),
        negative(
            not,
            r#"<main><div role="toolbar" aria-label="Format"><div role="button" tabindex="0">Bold</div><div role="button" tabindex="-1">Italic</div></div></main>"#,
            "APG Keyboard Interface, roving tabindex: tabindex=-1 an inaktiven Einträgen",
        )
        .variant("roving tabindex"),
        negative(
            not,
            r#"<main><div role="listbox" aria-label="Fruit" tabindex="0" aria-activedescendant="o1"><div role="option" id="o1" aria-selected="true">Apple</div><div role="option" id="o2">Pear</div></div></main>"#,
            "WAI-ARIA 1.2 aria-activedescendant: der Container verwaltet den Fokus",
        )
        .variant("aria-activedescendant"),
        negative(
            not,
            r#"<main><select aria-label="Fruit"><option>Apple</option></select><div role="button" hidden>Hidden</div></main>"#,
            "HTML: native option fokussiert über select; hidden ist nicht im AX-Baum",
        )
        .variant("native option, versteckt"),
        na(
            not,
            r#"<main><p>Text</p></main>"#,
            "Keine interaktive Rolle",
        ),
    ]
}

fn dialog() -> Vec<Fall> {
    let id = "dialog/focusable-missing";
    vec![
        review(
            id,
            r#"<main><p>x</p></main><div role="dialog" aria-label="Info"><p>Saved.</p></div>"#,
            "WCAG 2.4.3, APG Dialog: Fokus gehört in den Dialog; REVIEW, weil der Dialog selbst den Fokus tragen kann (docs/packages/a11y-rules.md, auditmysite#779)",
        )
        .count(1),
        review(
            id,
            r#"<main><p>x</p></main><dialog open><p>Saved.</p></dialog>"#,
            "HTML dialog open",
        )
        .variant("dialog-Element"),
        review(
            id,
            r#"<main><p>x</p></main><div role="alertdialog" aria-label="Error"><p>Failed.</p><button disabled>OK</button></div>"#,
            "HTML disabled: ein deaktivierter Button ist nicht fokussierbar",
        )
        .variant("alertdialog, nur deaktivierter Button"),
        negative(
            id,
            r#"<main><p>x</p></main><div role="dialog" aria-label="Info"><p>Saved.</p><button>Close</button></div>"#,
            "APG Dialog: ein fokussierbares Element im Dialog",
        ),
        negative(
            id,
            r#"<main><p>x</p></main><dialog open><div><p>Saved. <a href="/next">Continue</a></p></div></dialog>"#,
            "HTML: a mit href ist fokussierbar, auch tief verschachtelt",
        )
        .variant("verschachtelter Link"),
        negative(
            id,
            r#"<main><p>x</p></main><dialog><p>Closed.</p></dialog><div role="dialog" aria-label="Old" hidden><p>Gone.</p></div>"#,
            "HTML: geschlossener dialog und hidden sind nicht dargestellt",
        )
        .variant("geschlossen/versteckt"),
        na(id, r#"<main><p>No dialog</p></main>"#, "Kein Dialog"),
    ]
}

fn accordion() -> Vec<Fall> {
    let id = "patterns/accordion-controls-missing";
    vec![
        review(
            id,
            r#"<main><h3><button aria-expanded="true">Section 1</button></h3><div><p>Panel</p></div></main>"#,
            "APG Accordion: aria-controls verweist auf das Panel; WAI-ARIA 1.2 verlangt es nicht → REVIEW (Regelkommentar)",
        )
        .count(1),
        review(
            id,
            r#"<main><h3><div role="button" tabindex="0" aria-expanded="true">Section 1</div></h3><div><p>Panel</p></div></main>"#,
            "WAI-ARIA 1.2 role=button mit aria-expanded",
        )
        .variant("ARIA-Rolle"),
        negative(
            id,
            r#"<main><h3><button aria-expanded="true" aria-controls="p1">Section 1</button></h3><div id="p1"><p>Panel</p></div></main>"#,
            "APG Accordion: aria-controls vorhanden",
        ),
        negative(
            id,
            r#"<main><h3><button aria-expanded="false">Section 1</button></h3></main>"#,
            "Eingeklappt: kein Bereich sichtbar, auf den verwiesen werden müsste",
        )
        .variant("eingeklappt"),
        negative(
            id,
            r#"<nav aria-label="Main"><button aria-expanded="true">Menu</button><ul><li><a href="/">Home</a></li></ul></nav><main><p>x</p></main>"#,
            "APG Disclosure Navigation Menu: aria-controls optional",
        )
        .variant("Menü in navigation"),
        negative(
            id,
            r#"<main><details open><summary aria-expanded="true">More</summary><p>Details</p></details></main>"#,
            "HTML details/summary: der Bereich ist das umgebende details",
        )
        .variant("summary"),
        na(
            id,
            r#"<main><button>Save</button></main>"#,
            "Kein aria-expanded",
        ),
    ]
}
