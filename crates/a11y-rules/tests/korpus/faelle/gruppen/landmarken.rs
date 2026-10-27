//! Gruppe „landmarken“: Landmarks (structure.rs `landmarks`, landmarks.rs),
//! Tastaturerreichbarkeit, Dialogfokus und Akkordeon-Verweise.
//!
//! Die Landmark-Regeln sind nach docs/packages/a11y-rules.md („Normative
//! Zuordnung“, auditmysite#779) Best Practice ohne Kriterium (`FAIL` mit
//! Schlagwort `best-practice`) bzw. `REVIEW`; die Erwartungen folgen dieser
//! dokumentierten Entscheidung, die Rollen selbst WAI-ARIA 1.2 und HTML-AAM.

use super::super::{Fall, Modus, fail, na, negativ, review};

pub fn faelle() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(main_missing());
    v.extend(main_duplicate());
    v.extend(seitenweit());
    v.extend(not_unique());
    v.extend(not_top_level());
    v.extend(duplikate());
    v.extend(content_outside());
    v.extend(tastatur());
    v.extend(dialog());
    v.extend(akkordeon());
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
        .anzahl(1),
        fail(
            id,
            r#"<main hidden><p>Old</p></main><p>Visible text</p>"#,
            "HTML: ein main mit hidden ist nicht dargestellt und keine Landmark im AX-Baum; axe landmark-one-main",
        )
        .variante("versteckt per hidden"),
        fail(
            id,
            r#"<main aria-hidden="true"><p>Hidden from AT</p></main><p>Visible text</p>"#,
            "WAI-ARIA 1.2 aria-hidden: der Teilbaum fehlt im AX-Baum, damit auch die Landmark",
        )
        .variante("aria-hidden"),
        fail(
            id,
            r#"<main hidden><p>Old</p></main><p>Visible text</p>"#,
            "HTML hidden; Host ohne Stile wertet das Attribut aus",
        )
        .variante("versteckt per hidden, nur Semantik")
        .modus(Modus::Semantik),
        review(
            id,
            r#"<div role="dialog" aria-label="Consent"><p>Accept cookies?</p><button>OK</button></div><div aria-hidden="true"><main><p>Content</p></main></div>"#,
            "docs/packages/a11y-rules.md: hinter offenem Modal REVIEW statt FAIL (auditmysite#709)",
        )
        .variante("offener ARIA-Dialog"),
        review(
            id,
            r#"<dialog open><p>Accept cookies?</p><button>OK</button></dialog><div aria-hidden="true"><main><p>Content</p></main></div>"#,
            "HTML dialog open; auditmysite#709",
        )
        .variante("offenes dialog-Element"),
        negativ(
            id,
            r#"<header><p>Site</p></header><main><p>Body text</p></main>"#,
            "WAI-ARIA 1.2 main; HTML-AAM main → main",
        ),
        negativ(
            id,
            r#"<div role="main"><p>Body text</p></div>"#,
            "WAI-ARIA 1.2 role=main; WCAG-Technik ARIA11",
        )
        .variante("ARIA-Rolle statt Element"),
        negativ(
            id,
            r#"<div><template shadowrootmode="open"><main><p>Body text</p></main></template></div>"#,
            "DOM: Inhalt einer offenen Shadow Root steht im flachen Baum und damit im AX-Baum",
        )
        .variante("Shadow DOM"),
        na(
            id,
            "",
            "Leere Seite: kein Inhalt, der in eine Landmark gehörte (WAI-ARIA 1.2, Landmark: eine Region mit Inhalt)",
        )
        .variante("Regression: leerer body"),
        na(
            id,
            r#"<p hidden>Not rendered</p><script>var a = 1;</script>"#,
            "HTML hidden, script: nichts Dargestelltes im body",
        )
        .variante("Regression: nur Verstecktes"),
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
        .anzahl(1),
        fail(
            id,
            r#"<main><p>A</p></main><div role="main"><p>B</p></div>"#,
            "WAI-ARIA 1.2 main: ein Dokument SHOULD NOT mehr als eine main-Landmark haben",
        )
        .variante("Element und ARIA-Rolle"),
        negativ(
            id,
            r#"<main><p>A</p></main><main hidden><p>B</p></main>"#,
            "HTML, main-Element: mehrere main erlaubt, wenn alle bis auf eines hidden tragen",
        )
        .variante("zweites main versteckt"),
        negativ(
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
fn seitenweit() -> Vec<Fall> {
    let nav = "landmarks/navigation-missing";
    let banner = "landmarks/banner-missing";
    let info = "landmarks/contentinfo-missing";
    vec![
        review(
            nav,
            r#"<main><p>See <a href="/a">A</a></p></main>"#,
            "WAI-ARIA 1.2 navigation; Erwartung, kein Beleg: REVIEW (Regelkommentar, WCAG 2.4.1 ARIA11)",
        )
        .anzahl(1),
        review(
            nav,
            r#"<nav aria-hidden="true"><a href="/">Home</a></nav><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 aria-hidden: die Landmark fehlt im AX-Baum",
        )
        .variante("aria-hidden"),
        negativ(
            nav,
            r#"<nav><a href="/">Home</a></nav><main><p>Text</p></main>"#,
            "HTML-AAM nav → navigation",
        ),
        negativ(
            nav,
            r#"<div role="navigation"><a href="/">Home</a></div><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 role=navigation",
        )
        .variante("ARIA-Rolle"),
        na(nav, "", "Leere Seite: nichts, was Navigation bräuchte")
            .variante("Regression: leerer body"),
        review(
            banner,
            r#"<main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 banner; Erwartung, kein Beleg: REVIEW",
        )
        .anzahl(1),
        review(
            banner,
            r#"<main><article><header><h2>Post</h2></header><p>Text</p></article></main>"#,
            "HTML-AAM header: nur außerhalb von article/aside/main/nav/section → banner",
        )
        .variante("header im Abschnitt"),
        negativ(
            banner,
            r#"<header><p>Site</p></header><main><p>Text</p></main>"#,
            "HTML-AAM header (dokumentweit) → banner",
        ),
        negativ(
            banner,
            r#"<div><header><p>Site</p></header></div><main><p>Text</p></main>"#,
            "HTML-AAM: ein div begrenzt header nicht",
        )
        .variante("header im div"),
        negativ(
            banner,
            r#"<div role="banner"><p>Site</p></div><main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 role=banner",
        )
        .variante("ARIA-Rolle"),
        na(banner, "", "Leere Seite: nichts für einen Kopfbereich")
            .variante("Regression: leerer body"),
        review(
            info,
            r#"<main><p>Text</p></main>"#,
            "WAI-ARIA 1.2 contentinfo; Erwartung, kein Beleg: REVIEW",
        )
        .anzahl(1),
        review(
            info,
            r#"<main><article><p>Text</p><footer><p>Posted today</p></footer></article></main>"#,
            "HTML-AAM footer: nur außerhalb von article/aside/main/nav/section → contentinfo",
        )
        .variante("footer im Abschnitt"),
        negativ(
            info,
            r#"<main><p>Text</p></main><footer><p>© ACME</p></footer>"#,
            "HTML-AAM footer (dokumentweit) → contentinfo",
        ),
        negativ(
            info,
            r#"<main><p>Text</p></main><div role="contentinfo"><p>© ACME</p></div>"#,
            "WAI-ARIA 1.2 role=contentinfo",
        )
        .variante("ARIA-Rolle"),
        na(info, "", "Leere Seite: nichts für einen Fußbereich")
            .variante("Regression: leerer body"),
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
        .anzahl(2),
        review(
            id,
            r#"<nav aria-label="Menu"><a href="/a">A</a></nav><nav aria-label="menu"><a href="/b">B</a></nav>"#,
            "WAI-ARIA 1.2: gleicher Name (Groß-/Kleinschreibung unerheblich) unterscheidet nicht",
        )
        .variante("gleicher Name")
        .anzahl(2),
        negativ(
            id,
            r#"<nav aria-label="Main"><a href="/a">A</a></nav><nav aria-label="Footer"><a href="/b">B</a></nav>"#,
            "WAI-ARIA 1.2 / APG Landmarks: verschiedene Namen per aria-label",
        ),
        negativ(
            id,
            r#"<nav aria-labelledby="n1"><h2 id="n1">Sections</h2><a href="/a">A</a></nav><nav aria-labelledby="n2"><h2 id="n2">Related</h2><a href="/b">B</a></nav>"#,
            "accname 1.2 aria-labelledby; ARIA13",
        )
        .variante("aria-labelledby"),
        negativ(
            id,
            r#"<nav><a href="/a">A</a></nav><nav hidden><a href="/b">B</a></nav>"#,
            "HTML hidden: die zweite Landmark ist nicht im AX-Baum",
        )
        .variante("versteckt per hidden"),
        negativ(
            id,
            r#"<section><p>A</p></section><section><p>B</p></section><nav><a href="/">x</a></nav><aside><p>y</p></aside>"#,
            "HTML-AAM: section ohne Namen ist keine region-Landmark; verschiedene Rollen sind unterscheidbar",
        )
        .variante("unbenannte section"),
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
        .anzahl(1),
        fail(
            id,
            r#"<main><p>x</p></main><aside><div role="contentinfo"><p>c</p></div></aside>"#,
            "WAI-ARIA 1.2 contentinfo: nicht in einer anderen Landmark; axe landmark-contentinfo-is-top-level",
        )
        .variante("contentinfo per Rolle in complementary"),
        negativ(
            id,
            r#"<main><header><h1>T</h1></header><p>x</p><footer><p>f</p></footer></main>"#,
            "HTML-AAM: header/footer in main sind weder banner noch contentinfo",
        ),
        negativ(
            id,
            r#"<section><main><p>x</p></main></section>"#,
            "HTML-AAM: section ohne Namen ist keine Landmark (auditmysite#727)",
        )
        .variante("unbenannte section"),
        na(
            id,
            r#"<nav><a href="/">Home</a></nav><p>x</p>"#,
            "Kein main, banner oder contentinfo",
        ),
    ]
}

fn duplikate() -> Vec<Fall> {
    let banner = "landmarks/banner-duplicate";
    let info = "landmarks/contentinfo-duplicate";
    vec![
        fail(
            banner,
            r#"<header><p>A</p></header><main><p>x</p></main><header><p>B</p></header>"#,
            "WAI-ARIA 1.2 banner: höchstens eine; axe landmark-no-duplicate-banner (Best Practice); docs/packages/a11y-rules.md",
        )
        .anzahl(1),
        fail(
            banner,
            r#"<header><p>A</p></header><div role="banner"><p>B</p></div>"#,
            "WAI-ARIA 1.2 banner per Rolle zählt mit",
        )
        .variante("Element und ARIA-Rolle"),
        negativ(
            banner,
            r#"<header><p>Site</p></header><main><article><header><h2>Post</h2></header><p>x</p></article></main>"#,
            "HTML-AAM: header im article ist kein banner",
        ),
        negativ(
            banner,
            r#"<header><p>A</p></header><header hidden><p>B</p></header>"#,
            "HTML hidden: nicht im AX-Baum",
        )
        .variante("versteckt per hidden"),
        na(banner, r#"<main><p>x</p></main>"#, "Kein banner"),
        fail(
            info,
            r#"<main><p>x</p></main><footer><p>A</p></footer><footer><p>B</p></footer>"#,
            "WAI-ARIA 1.2 contentinfo: höchstens eine; axe landmark-no-duplicate-contentinfo (Best Practice); docs/packages/a11y-rules.md",
        )
        .anzahl(1),
        fail(
            info,
            r#"<footer><p>A</p></footer><div role="contentinfo"><p>B</p></div>"#,
            "WAI-ARIA 1.2 contentinfo per Rolle zählt mit",
        )
        .variante("Element und ARIA-Rolle"),
        negativ(
            info,
            r#"<main><article><p>x</p><footer><p>Post</p></footer></article></main><footer><p>Site</p></footer>"#,
            "HTML-AAM: footer im article ist kein contentinfo",
        ),
        negativ(
            info,
            r#"<footer><p>A</p></footer><footer style="display:none"><p>B</p></footer>"#,
            "CSS display:none: nicht dargestellt, nicht im AX-Baum",
        )
        .variante("versteckt per CSS"),
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
        .anzahl(1),
        fail(
            id,
            r#"<div><img src="chart.png" alt="Sales chart"></div><main><p>In</p></main>"#,
            "axe region: ein benanntes Bild ist Inhalt",
        )
        .variante("benanntes Bild"),
        negativ(
            id,
            r##"<a href="#main">Skip to content</a><header><p>Site</p></header><main id="main"><p>In</p></main><footer><p>f</p></footer>"##,
            "axe region: Sprunglinks vor den Landmarks sind ausgenommen (isSkipLink)",
        )
        .variante("Sprunglink"),
        negativ(
            id,
            r#"<main><p>In</p></main><p hidden>Out</p><div aria-hidden="true">Decor</div>"#,
            "HTML hidden, aria-hidden: nicht im AX-Baum",
        )
        .variante("versteckt"),
        negativ(
            id,
            r#"<main><p>In</p></main><img src="spacer.gif" alt="">"#,
            "HTML-AAM: img mit alt=\"\" ist präsentational, kein Inhalt",
        )
        .variante("dekoratives Bild"),
        na(id, r#"<script>var a = 1;</script>"#, "Kein Inhalt im body"),
    ]
}

fn tastatur() -> Vec<Fall> {
    let ohne = "keyboard/focusable-no-role";
    let nicht = "keyboard/interactive-not-focusable";
    vec![
        review(
            ohne,
            r#"<main><div tabindex="0">Card</div></main>"#,
            "WCAG 2.1.1, 4.1.2: in der Tabfolge ohne interaktive Rolle; Bedienbarkeit hängt am Skript → REVIEW (auditmysite#569)",
        )
        .anzahl(1),
        review(
            ohne,
            r#"<main><a tabindex="0">Not a link</a></main>"#,
            "HTML-AAM: a ohne href ist generic; tabindex=0 macht es erreichbar",
        )
        .variante("a ohne href"),
        negativ(
            ohne,
            r#"<main><div tabindex="0" role="button">Do</div><button>Native</button><a href="/x" tabindex="0">Link</a></main>"#,
            "WAI-ARIA 1.2 button; HTML: native Bedienelemente",
        ),
        negativ(
            ohne,
            r#"<main><div tabindex="-1" id="target">Skip target</div></main>"#,
            "HTML tabindex=-1: fokussierbar, aber nicht in der sequenziellen Tabfolge",
        )
        .variante("tabindex=-1"),
        negativ(
            ohne,
            r#"<main><div tabindex="0" role="region" aria-label="Scrollable table">Wide</div></main>"#,
            "ACT 0ssw9k / axe scrollable-region-focusable: benannter fokussierbarer Bereich ist das empfohlene Muster",
        )
        .variante("region"),
        negativ(
            ohne,
            r#"<main><div tabindex="0" hidden>x</div><div inert><div tabindex="0">y</div></div></main>"#,
            "HTML hidden und inert: nicht fokussierbar",
        )
        .variante("versteckt/inert"),
        na(
            ohne,
            r#"<main><p>Text</p></main>"#,
            "Kein Element mit tabindex",
        ),
        review(
            nicht,
            r#"<main><div role="button">Save</div></main>"#,
            "WCAG 2.1.1; WAI-ARIA 1.2 button: interaktiv, aber nicht fokussierbar → REVIEW",
        )
        .anzahl(1),
        review(
            nicht,
            r#"<main><span role="link">More</span><div role="checkbox" aria-checked="false">Agree</div></main>"#,
            "WAI-ARIA 1.2 link, checkbox: Widgets müssen fokussierbar sein (APG Keyboard Interface)",
        )
        .variante("link, checkbox")
        .anzahl(2),
        negativ(
            nicht,
            r#"<main><div role="button" tabindex="0">Save</div><button>Native</button><a href="/">Link</a></main>"#,
            "HTML tabindex=0 / native Bedienelemente sind fokussierbar",
        ),
        negativ(
            nicht,
            r#"<main><div role="toolbar" aria-label="Format"><div role="button" tabindex="0">Bold</div><div role="button" tabindex="-1">Italic</div></div></main>"#,
            "APG Keyboard Interface, roving tabindex: tabindex=-1 an inaktiven Einträgen",
        )
        .variante("roving tabindex"),
        negativ(
            nicht,
            r#"<main><div role="listbox" aria-label="Fruit" tabindex="0" aria-activedescendant="o1"><div role="option" id="o1" aria-selected="true">Apple</div><div role="option" id="o2">Pear</div></div></main>"#,
            "WAI-ARIA 1.2 aria-activedescendant: der Container verwaltet den Fokus",
        )
        .variante("aria-activedescendant"),
        negativ(
            nicht,
            r#"<main><select aria-label="Fruit"><option>Apple</option></select><div role="button" hidden>Hidden</div></main>"#,
            "HTML: native option fokussiert über select; hidden ist nicht im AX-Baum",
        )
        .variante("native option, versteckt"),
        na(
            nicht,
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
        .anzahl(1),
        review(
            id,
            r#"<main><p>x</p></main><dialog open><p>Saved.</p></dialog>"#,
            "HTML dialog open",
        )
        .variante("dialog-Element"),
        review(
            id,
            r#"<main><p>x</p></main><div role="alertdialog" aria-label="Error"><p>Failed.</p><button disabled>OK</button></div>"#,
            "HTML disabled: ein deaktivierter Button ist nicht fokussierbar",
        )
        .variante("alertdialog, nur deaktivierter Button"),
        negativ(
            id,
            r#"<main><p>x</p></main><div role="dialog" aria-label="Info"><p>Saved.</p><button>Close</button></div>"#,
            "APG Dialog: ein fokussierbares Element im Dialog",
        ),
        negativ(
            id,
            r#"<main><p>x</p></main><dialog open><div><p>Saved. <a href="/next">Continue</a></p></div></dialog>"#,
            "HTML: a mit href ist fokussierbar, auch tief verschachtelt",
        )
        .variante("verschachtelter Link"),
        negativ(
            id,
            r#"<main><p>x</p></main><dialog><p>Closed.</p></dialog><div role="dialog" aria-label="Old" hidden><p>Gone.</p></div>"#,
            "HTML: geschlossener dialog und hidden sind nicht dargestellt",
        )
        .variante("geschlossen/versteckt"),
        na(id, r#"<main><p>No dialog</p></main>"#, "Kein Dialog"),
    ]
}

fn akkordeon() -> Vec<Fall> {
    let id = "patterns/accordion-controls-missing";
    vec![
        review(
            id,
            r#"<main><h3><button aria-expanded="true">Section 1</button></h3><div><p>Panel</p></div></main>"#,
            "APG Accordion: aria-controls verweist auf das Panel; WAI-ARIA 1.2 verlangt es nicht → REVIEW (Regelkommentar)",
        )
        .anzahl(1),
        review(
            id,
            r#"<main><h3><div role="button" tabindex="0" aria-expanded="true">Section 1</div></h3><div><p>Panel</p></div></main>"#,
            "WAI-ARIA 1.2 role=button mit aria-expanded",
        )
        .variante("ARIA-Rolle"),
        negativ(
            id,
            r#"<main><h3><button aria-expanded="true" aria-controls="p1">Section 1</button></h3><div id="p1"><p>Panel</p></div></main>"#,
            "APG Accordion: aria-controls vorhanden",
        ),
        negativ(
            id,
            r#"<main><h3><button aria-expanded="false">Section 1</button></h3></main>"#,
            "Eingeklappt: kein Bereich sichtbar, auf den verwiesen werden müsste",
        )
        .variante("eingeklappt"),
        negativ(
            id,
            r#"<nav aria-label="Main"><button aria-expanded="true">Menu</button><ul><li><a href="/">Home</a></li></ul></nav><main><p>x</p></main>"#,
            "APG Disclosure Navigation Menu: aria-controls optional",
        )
        .variante("Menü in navigation"),
        negativ(
            id,
            r#"<main><details open><summary aria-expanded="true">More</summary><p>Details</p></details></main>"#,
            "HTML details/summary: der Bereich ist das umgebende details",
        )
        .variante("summary"),
        na(
            id,
            r#"<main><button>Save</button></main>"#,
            "Kein aria-expanded",
        ),
    ]
}
