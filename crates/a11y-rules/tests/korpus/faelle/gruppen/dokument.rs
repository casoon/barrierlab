//! Gruppe „dokument“: Sprache, Titel, Viewport, Überschriften, Bilder,
//! Beschriftung, Sprunglink (`structure.rs`) und Sprach-, Zeit- und
//! Abschnittsregeln (`document.rs`).

use super::super::{Fall, fail, na, negative, review, untested};

/// Ein SVG-Dokument: kein HTML, also kein `lang`, `title` oder Viewport
/// gefordert.
const SVG_DOCUMENT: &str = r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>"#;

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(language());
    v.extend(title());
    v.extend(viewport());
    v.extend(headings());
    v.extend(images());
    v.extend(label());
    v.extend(skip_link());
    v.extend(parts());
    v.extend(abbreviations());
    v.extend(refresh());
    v.extend(sections());
    v
}

fn language() -> Vec<Fall> {
    vec![
        // --- document/lang-missing
        fail(
            "document/lang-missing",
            r#"<!DOCTYPE html><html><head><title>T</title></head><body><p>x</p></body></html>"#,
            "WCAG 3.1.1, ACT b5c3f8: html ohne lang-Attribut",
        ),
        fail(
            "document/lang-missing",
            r#"<!DOCTYPE html><html lang=""><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT b5c3f8: lang muss nicht leer sein; bf051a gilt erst für nicht leere Werte",
        )
        .variant("Regression: leeres lang ist fehlend, nicht ungültig"),
        fail(
            "document/lang-missing",
            r#"<!DOCTYPE html><html lang="  "><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT b5c3f8: nur Leerraum zählt als leer",
        )
        .variant("nur Leerraum"),
        negative(
            "document/lang-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "WCAG 3.1.1, H57",
        ),
        negative(
            "document/lang-missing",
            r#"<!DOCTYPE html><html lang="xx-invalid-tagxx"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT b5c3f8 besteht mit jedem nicht leeren Wert; Gültigkeit prüft bf051a",
        )
        .variant("ungültiger, aber vorhandener Wert"),
        na(
            "document/lang-missing",
            SVG_DOCUMENT,
            "ACT b5c3f8: anwendbar nur auf das html-Element eines HTML-Dokuments",
        ),
        // --- document/lang-invalid
        fail(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="english"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "WCAG 3.1.1, ACT bf051a: kein gültiges BCP-47-Primärsubtag",
        ),
        fail(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="en_US"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "BCP 47 (RFC 5646) trennt mit Bindestrich",
        )
        .variant("Unterstrich"),
        fail(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="engl"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646 2.2.1: vierbuchstabige Primärsubtags sind reserviert",
        )
        .variant("Grenze: 4 Buchstaben"),
        fail(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="e"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646 2.2.1: Primärsubtag hat mindestens 2 Buchstaben (außer x/i)",
        )
        .variant("Grenze: 1 Buchstabe"),
        fail(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="en-abcdefghi"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646 2.1: Untertags haben höchstens 8 Zeichen",
        )
        .variant("Regression: Grenze 9 Zeichen im Untertag"),
        negative(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="eng"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646 2.2.1: dreibuchstabiges Primärsubtag (ISO 639-2/3)",
        )
        .variant("Grenze: 3 Buchstaben"),
        negative(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="zh-Hant-TW"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646: Sprache-Schrift-Region",
        ),
        negative(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html lang="de-abcdefgh"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "RFC 5646 2.1: Untertag mit genau 8 Zeichen ist zulässig",
        )
        .variant("Grenze: 8 Zeichen im Untertag"),
        na(
            "document/lang-invalid",
            r#"<!DOCTYPE html><html><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT bf051a: anwendbar nur auf ein nicht leeres lang",
        ),
        // --- document/lang-mismatch
        fail(
            "document/lang-mismatch",
            r#"<!DOCTYPE html><html lang="en" xml:lang="de"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "WCAG 3.1.1, ACT 5b7ae0: lang und xml:lang mit verschiedenem Primärsubtag",
        ),
        negative(
            "document/lang-mismatch",
            r#"<!DOCTYPE html><html lang="en" xml:lang="en-US"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT 5b7ae0 vergleicht das Primärsubtag",
        ),
        negative(
            "document/lang-mismatch",
            r#"<!DOCTYPE html><html lang="DE" xml:lang="de"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "BCP 47: Schreibung ohne Bedeutung",
        )
        .variant("Groß-/Kleinschreibung"),
        na(
            "document/lang-mismatch",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT 5b7ae0: anwendbar nur mit beiden Attributen",
        ),
    ]
}

fn title() -> Vec<Fall> {
    vec![
        fail(
            "document/title-missing",
            r#"<!DOCTYPE html><html lang="en"><head></head><body><p>x</p></body></html>"#,
            "WCAG 2.4.2, ACT 2779a5: kein title-Element",
        ),
        fail(
            "document/title-missing",
            r#"<!DOCTYPE html><html lang="en"><head></head><body><svg><title>Icon</title></svg></body></html>"#,
            "ACT 2779a5: nur ein HTML-title zählt, nicht der eines SVG",
        )
        .variant("Regression: nur SVG-title im Dokument"),
        negative(
            "document/title-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>Home</title></head><body><p>x</p></body></html>"#,
            "WCAG 2.4.2, H25",
        ),
        na(
            "document/title-missing",
            SVG_DOCUMENT,
            "ACT 2779a5: anwendbar nur auf HTML-Dokumente",
        ),
        fail(
            "document/title-empty",
            r#"<!DOCTYPE html><html lang="en"><head><title></title></head><body><p>x</p></body></html>"#,
            "WCAG 2.4.2, ACT 2779a5: title ohne Text",
        ),
        fail(
            "document/title-empty",
            r#"<!DOCTYPE html><html lang="en"><head><title>   </title></head><body><p>x</p></body></html>"#,
            "ACT 2779a5: nur Leerraum zählt als leer",
        )
        .variant("nur Leerraum"),
        negative(
            "document/title-empty",
            r#"<!DOCTYPE html><html lang="en"><head><title>Home</title></head><body><p>x</p></body></html>"#,
            "WCAG 2.4.2, H25",
        ),
        na(
            "document/title-empty",
            r#"<!DOCTYPE html><html lang="en"><head></head><body><p>x</p></body></html>"#,
            "Ohne title-Element meldet title-missing, nicht title-empty",
        ),
    ]
}

fn viewport() -> Vec<Fall> {
    vec![
        // --- zoom/viewport-locked
        fail(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, user-scalable=no"></head><body><p>x</p></body></html>"#,
            "WCAG 1.4.4, ACT b4f0c3: user-scalable=no",
        ),
        fail(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width; user-scalable=0"></head><body><p>x</p></body></html>"#,
            "CSS Viewport: user-scalable=0 sperrt wie no; Semikolon als Trenner",
        )
        .variant("user-scalable=0, Semikolon"),
        fail(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="USER-SCALABLE=NO"></head><body><p>x</p></body></html>"#,
            "CSS Viewport: Namen und Werte ohne Rücksicht auf Schreibung",
        )
        .variant("Großschreibung"),
        fail(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=1.99"></head><body><p>x</p></body></html>"#,
            "ACT b4f0c3: maximum-scale unter 2 verhindert 200 %",
        )
        .variant("Grenze: 1.99"),
        negative(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=2"></head><body><p>x</p></body></html>"#,
            "ACT b4f0c3: maximum-scale 2 erlaubt 200 %",
        )
        .variant("Grenze: 2"),
        negative(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, user-scalable=yes"></head><body><p>x</p></body></html>"#,
            "ACT b4f0c3: user-scalable=yes",
        ),
        na(
            "zoom/viewport-locked",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "ACT b4f0c3: anwendbar nur auf meta name=viewport",
        ),
        // --- zoom/viewport-scale-limited (Best Practice, auditmysite#779)
        review(
            "zoom/viewport-scale-limited",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=2"></head><body><p>x</p></body></html>"#,
            "Best Practice (docs/packages/a11y-rules.md): 1.4.4 erfüllt, aber unter 500 % begrenzt",
        )
        .variant("Grenze: 2"),
        review(
            "zoom/viewport-scale-limited",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=4.99"></head><body><p>x</p></body></html>"#,
            "Best Practice: unter 500 %",
        )
        .variant("Grenze: 4.99"),
        negative(
            "zoom/viewport-scale-limited",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=5"></head><body><p>x</p></body></html>"#,
            "500 % ist die Obergrenze, die Browser ohnehin setzen",
        )
        .variant("Grenze: 5"),
        negative(
            "zoom/viewport-scale-limited",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, maximum-scale=1"></head><body><p>x</p></body></html>"#,
            "Unter 200 % ist es zoom/viewport-locked (Verstoß), nicht zusätzlich die Empfehlung",
        )
        .variant("gesperrt statt begrenzt"),
        na(
            "zoom/viewport-scale-limited",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width, initial-scale=1"></head><body><p>x</p></body></html>"#,
            "Ohne maximum-scale keine Begrenzung",
        ),
        // --- zoom/viewport-missing (Best Practice, auditmysite#779)
        review(
            "zoom/viewport-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><body><p>x</p></body></html>"#,
            "Best Practice ohne Kriterium (docs/packages/a11y-rules.md, auditmysite#779)",
        ),
        negative(
            "zoom/viewport-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta name="viewport" content="width=device-width"></head><body><p>x</p></body></html>"#,
            "Viewport-Angabe vorhanden",
        ),
        na(
            "zoom/viewport-missing",
            SVG_DOCUMENT,
            "meta viewport gibt es nur in HTML-Dokumenten",
        ),
    ]
}

fn headings() -> Vec<Fall> {
    vec![
        // --- headings/empty
        fail(
            "headings/empty",
            r#"<h1>Title</h1><h2></h2>"#,
            "WCAG 1.3.1/2.4.6, ACT ffd0e9: Überschrift ohne Namen",
        ),
        fail(
            "headings/empty",
            r#"<h1>Title</h1><h2><span aria-hidden="true">★</span></h2>"#,
            "ACT ffd0e9: aria-hidden-Inhalt trägt nicht zum Namen bei",
        )
        .variant("Inhalt nur unter aria-hidden"),
        fail(
            "headings/empty",
            r#"<h1>Title</h1><h2 aria-label=""></h2>"#,
            "accname 1.2, 2D: leeres aria-label stiftet keinen Namen",
        )
        .variant("Regression: leeres aria-label"),
        fail(
            "headings/empty",
            r#"<h1>Title</h1><div role="heading" aria-level="2"></div>"#,
            "ACT ffd0e9 gilt für die Rolle heading, nicht nur für hN",
        )
        .variant("Regression: ARIA-Rolle heading"),
        negative(
            "headings/empty",
            r#"<h1>Title</h1><h2><img src="logo.png" alt="ACME"></h2>"#,
            "accname 1.2, 2F: Name aus dem Inhalt (alt des Bildes)",
        )
        .variant("Name über alt"),
        negative(
            "headings/empty",
            r#"<h1>Title</h1><h2 aria-label="Products"></h2>"#,
            "accname 1.2, 2C: aria-label",
        )
        .variant("aria-label"),
        negative(
            "headings/empty",
            r#"<span id="l">Products</span><h1>Title</h1><h2 aria-labelledby="l"></h2>"#,
            "accname 1.2, 2B: aria-labelledby",
        )
        .variant("aria-labelledby"),
        negative(
            "headings/empty",
            r#"<h1>Title</h1><div role="heading" aria-level="2">Products</div>"#,
            "ACT ffd0e9: ARIA-Überschrift mit Text",
        )
        .variant("ARIA-Rolle heading"),
        negative(
            "headings/empty",
            r#"<h1>Title</h1><h2 hidden></h2><h2 style="display:none"></h2><h2 aria-hidden="true"></h2>"#,
            "ACT ffd0e9: nur Überschriften im Accessibility-Tree",
        )
        .variant("versteckt: hidden, display:none, aria-hidden"),
        na(
            "headings/empty",
            r#"<p>No headings here</p>"#,
            "ACT ffd0e9: anwendbar nur auf Überschriften",
        ),
        // --- headings/skip-level (Best Practice, auditmysite#779)
        review(
            "headings/skip-level",
            r#"<h1>A</h1><h3>B</h3>"#,
            "Best Practice (G141, axe heading-order; docs/packages/a11y-rules.md)",
        )
        .variant("Grenze: zwei Ebenen"),
        review(
            "headings/skip-level",
            r#"<h1>A</h1><h2 hidden>B</h2><h3>C</h3>"#,
            "Die Gliederung im Accessibility-Tree springt von h1 auf h3",
        )
        .variant("Zwischenebene versteckt"),
        review(
            "headings/skip-level",
            r#"<h1>A</h1><div role="heading" aria-level="3">B</div>"#,
            "WAI-ARIA 1.2: role=heading mit aria-level gehört zur Gliederung",
        )
        .variant("Regression: ARIA-Überschrift"),
        negative(
            "headings/skip-level",
            r#"<h1>A</h1><h2>B</h2><h3>C</h3><h2>D</h2><h1>E</h1>"#,
            "G141: eine Ebene tiefer oder beliebig höher",
        )
        .variant("Grenze: eine Ebene"),
        negative(
            "headings/skip-level",
            r#"<h1>A</h1><div role="heading">B</div>"#,
            "WAI-ARIA 1.2: aria-level von heading ist ohne Angabe 2",
        )
        .variant("ARIA-Überschrift, Vorgabeebene 2"),
        na(
            "headings/skip-level",
            r#"<h2>Only one</h2>"#,
            "Eine einzige Überschrift hat keine Folge",
        ),
        // --- headings/h1-missing (Best Practice, auditmysite#779)
        review(
            "headings/h1-missing",
            r#"<h2>A</h2><h3>B</h3>"#,
            "Best Practice (axe page-has-heading-one; docs/packages/a11y-rules.md)",
        ),
        review(
            "headings/h1-missing",
            r#"<h1 hidden>A</h1><h2>B</h2>"#,
            "Eine versteckte h1 steht nicht im Accessibility-Tree",
        )
        .variant("h1 versteckt"),
        negative(
            "headings/h1-missing",
            r#"<h1>A</h1><h2>B</h2>"#,
            "h1 vorhanden",
        ),
        negative(
            "headings/h1-missing",
            r#"<div role="heading" aria-level="1">A</div><h2>B</h2>"#,
            "WAI-ARIA 1.2: role=heading aria-level=1 ist eine Überschrift erster Ebene (axe zählt sie)",
        )
        .variant("Regression: ARIA-h1"),
        na(
            "headings/h1-missing",
            r#"<p>No headings</p>"#,
            "Ohne Überschriften meldet headings/none",
        ),
        // --- headings/h1-multiple
        review(
            "headings/h1-multiple",
            r#"<h1>A</h1><h1>B</h1>"#,
            "HTML erlaubt mehrere h1; Erwartung, kein Verstoß (WCAG 1.3.1) — REVIEW",
        )
        .count(1),
        negative(
            "headings/h1-multiple",
            r#"<h1>A</h1><h1 hidden>B</h1><h2>C</h2>"#,
            "Nur Überschriften im Accessibility-Tree zählen",
        )
        .variant("Grenze: eine sichtbare h1"),
        na(
            "headings/h1-multiple",
            r#"<h2>A</h2>"#,
            "Ohne h1 nicht anwendbar",
        ),
        // --- headings/none
        fail(
            "headings/none",
            r#"<nav><a href="/">Home</a></nav><p>Text</p>"#,
            "WCAG 2.4.1: weder Überschriften (H69) noch main (ARIA11) noch Sprunglink (G1), Navigation als Landmark",
        ),
        review(
            "headings/none",
            r#"<p>Text</p>"#,
            "WCAG 2.4.1: ob Blöcke sich wiederholen, zeigt ein Dokument nicht (docs/packages/a11y-rules.md)",
        )
        .variant("ohne Navigation"),
        review(
            "headings/none",
            r#"<nav><a href="/">Home</a></nav><div role="dialog" aria-label="Cookies"><p>Accept?</p></div>"#,
            "auditmysite#709: hinter offenem Dialog nur Hinweis",
        )
        .variant("offener Dialog"),
        negative(
            "headings/none",
            r#"<nav><a href="/">Home</a></nav><main><p>Text</p></main>"#,
            "ARIA11: main-Landmark genügt",
        )
        .variant("main"),
        negative(
            "headings/none",
            r##"<a href="#c">Skip to content</a><nav><a href="/">Home</a></nav><div id="c"><p>Text</p></div>"##,
            "G1: Sprunglink genügt",
        )
        .variant("Sprunglink"),
        negative(
            "headings/none",
            r#"<nav><a href="/">Home</a></nav><div role="heading" aria-level="1">Title</div>"#,
            "H69/ARIA12: ARIA-Überschrift genügt",
        )
        .variant("ARIA-Überschrift"),
        na(
            "headings/none",
            r#"<h1>Title</h1><p>Text</p>"#,
            "Mit Überschriften nicht anwendbar",
        ),
    ]
}

fn images() -> Vec<Fall> {
    vec![
        // --- images/alt-missing
        fail(
            "images/alt-missing",
            r#"<img src="a.png">"#,
            "WCAG 1.1.1, F65; ACT 23a2a8: img ohne Textalternative",
        ),
        fail(
            "images/alt-missing",
            r#"<div><template shadowrootmode="open"><img src="a.png"></template></div>"#,
            "ACT 23a2a8 gilt auch in Shadow Roots",
        )
        .variant("Shadow DOM"),
        fail(
            "images/alt-missing",
            r#"<img src="a.png" aria-label="  ">"#,
            "accname 1.2, 2C: aria-label aus Leerraum stiftet keinen Namen",
        )
        .variant("leeres aria-label"),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" alt="Logo of ACME">"#,
            "WCAG 1.1.1, H37",
        ),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" alt="">"#,
            "H67/F39: leeres alt kennzeichnet Schmuck",
        )
        .variant("alt leer"),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" aria-label="Logo of ACME">"#,
            "ARIA6",
        )
        .variant("aria-label"),
        negative(
            "images/alt-missing",
            r#"<span id="l">Logo of ACME</span><img src="a.png" aria-labelledby="l">"#,
            "ARIA10",
        )
        .variant("aria-labelledby"),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" title="Logo of ACME">"#,
            "HTML-AAM img: title stiftet den Namen; ACT 23a2a8 besteht",
        )
        .variant("title"),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" role="presentation"><img src="b.png" aria-hidden="true">"#,
            "ACT 23a2a8: nur img mit Rolle img im Accessibility-Tree",
        )
        .variant("ausdrücklich dekorativ"),
        negative(
            "images/alt-missing",
            r#"<img src="a.png" hidden><img src="b.png" style="display:none"><div aria-hidden="true"><img src="c.png"></div>"#,
            "ACT 23a2a8: nur eingebundene, nicht versteckte Bilder",
        )
        .variant("versteckt: hidden, display:none, aria-hidden am Vorfahren"),
        na(
            "images/alt-missing",
            r#"<p>Text only</p>"#,
            "ACT 23a2a8: gilt nur für img-Elemente",
        ),
        // --- images/alt-suspicious
        review(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="image">"#,
            "WCAG 1.1.1, F30: Füllwort statt Beschreibung — REVIEW",
        ),
        review(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="photo_2024.jpg">"#,
            "F30: Dateiname als Textalternative",
        )
        .variant("Dateiname"),
        review(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="ab">"#,
            "F30: zwei Zeichen beschreiben kein Bild (Heuristik-Schwelle 3 Zeichen)",
        )
        .variant("Grenze: 2 Zeichen"),
        negative(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="Cat">"#,
            "H37: knappe, aber beschreibende Alternative",
        )
        .variant("Grenze: 3 Zeichen"),
        negative(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="">"#,
            "H67: leeres alt ist Schmuck, nicht verdächtig",
        )
        .variant("alt leer"),
        negative(
            "images/alt-suspicious",
            r#"<img src="a.png" alt="image" role="presentation">"#,
            "Ausdrücklich dekorativ: nicht im Accessibility-Tree",
        )
        .variant("dekorativ"),
        na(
            "images/alt-suspicious",
            r#"<img src="a.png" aria-label="Chart of sales">"#,
            "Ohne alt-Attribut nichts zu beurteilen",
        ),
        na("images/alt-suspicious", r#"<p>image</p>"#, "Kein Bild"),
    ]
}

fn label() -> Vec<Fall> {
    vec![
        // --- forms/label-missing
        fail(
            "forms/label-missing",
            r#"<input type="text">"#,
            "WCAG 4.1.2/1.3.1, ACT e086e5: Feld ohne Namen",
        ),
        fail(
            "forms/label-missing",
            r#"<select><option>A</option></select><textarea></textarea><input type="checkbox">"#,
            "ACT e086e5: select, textarea, checkbox",
        )
        .variant("select, textarea, checkbox")
        .count(3),
        fail(
            "forms/label-missing",
            r#"<input type="text" aria-label=" "><input type="text" title="">"#,
            "accname 1.2: leeres aria-label/title stiftet keinen Namen",
        )
        .variant("leeres aria-label und title")
        .count(2),
        fail(
            "forms/label-missing",
            r#"<label for="x">Name</label><div><template shadowrootmode="open"><input type="text" id="x"></template></div>"#,
            "DOM: label for wirkt nicht über die Grenze einer Shadow Root",
        )
        .variant("Shadow DOM: label for über die Grenze"),
        negative(
            "forms/label-missing",
            r#"<label for="x">Name</label><input type="text" id="x">"#,
            "H44",
        )
        .variant("label for"),
        negative(
            "forms/label-missing",
            r#"<label>Name <input type="text"></label>"#,
            "H44: umschließendes label",
        )
        .variant("label umschließend"),
        negative(
            "forms/label-missing",
            r#"<input type="text" aria-label="Name">"#,
            "ARIA14",
        )
        .variant("aria-label"),
        negative(
            "forms/label-missing",
            r#"<span id="l">Name</span><input type="text" aria-labelledby="l">"#,
            "ARIA16",
        )
        .variant("aria-labelledby"),
        negative(
            "forms/label-missing",
            r#"<input type="text" title="Name">"#,
            "H65",
        )
        .variant("title"),
        negative(
            "forms/label-missing",
            r#"<div><template shadowrootmode="open"><label for="x">Name</label><input type="text" id="x"></template></div>"#,
            "label for innerhalb derselben Shadow Root",
        )
        .variant("Shadow DOM: gleiche Root"),
        na(
            "forms/label-missing",
            r#"<input type="hidden" name="t"><input type="submit"><input type="button" value="Go"><input type="reset"><input type="image" src="go.png" alt="Go">"#,
            "ACT e086e5: Schaltflächen und versteckte Felder sind keine beschriftbaren Eingabefelder",
        ),
        na(
            "forms/label-missing",
            r#"<input type="text" hidden><input type="text" style="display:none"><div aria-hidden="true"><input type="text" tabindex="-1"></div>"#,
            "ACT e086e5: nur Felder im Accessibility-Tree",
        )
        .variant("versteckt"),
        // --- forms/placeholder-as-label
        review(
            "forms/placeholder-as-label",
            r#"<input type="text" placeholder="Search">"#,
            "WCAG 3.3.2: Platzhalter verschwindet beim Tippen, stiftet aber den Namen (docs/packages/a11y-rules.md) — REVIEW",
        ),
        review(
            "forms/placeholder-as-label",
            r#"<input type="text" placeholder="Search" title="Search">"#,
            "3.3.2 verlangt sichtbare Beschriftung; title ist nicht sichtbar",
        )
        .variant("mit title"),
        negative(
            "forms/placeholder-as-label",
            r#"<label for="s">Search</label><input type="text" id="s" placeholder="e.g. shoes">"#,
            "H44: sichtbares label, Platzhalter nur als Beispiel",
        )
        .variant("label for"),
        negative(
            "forms/placeholder-as-label",
            r#"<span id="l">Search</span><input type="text" aria-labelledby="l" placeholder="e.g. shoes">"#,
            "ARIA16: aria-labelledby auf sichtbaren Text ist eine sichtbare Beschriftung (aria-label wäre keine, siehe regressionen.rs)",
        )
        .variant("aria-labelledby"),
        na(
            "forms/placeholder-as-label",
            r#"<label>Name <input type="text"></label>"#,
            "Ohne Platzhalter nicht anwendbar",
        ),
    ]
}

fn skip_link() -> Vec<Fall> {
    vec![
        review(
            "keyboard/skip-link-missing",
            r#"<nav><a href="/">Home</a><a href="/about">About</a></nav><main><h1>T</h1></main>"#,
            "WCAG 2.4.1, G1: kein Sprunglink erkannt — heuristisch, REVIEW (docs/packages/a11y-rules.md)",
        ),
        review(
            "keyboard/skip-link-missing",
            r##"<nav><a href="/">Home</a></nav><a href="#c">Aller au contenu</a><main><h1>T</h1><h2 id="c">Section</h2></main>"##,
            "G1: ein Anker mitten im Inhalt hinter der Navigation überspringt nichts",
        )
        .variant("Anker hinter der Navigation, Ziel nicht am Anfang des Inhalts"),
        negative(
            "keyboard/skip-link-missing",
            r##"<a href="#main">Skip to main content</a><nav><a href="/">Home</a></nav><main id="main"><h1>T</h1></main>"##,
            "G1: Sprunglink auf den Hauptinhalt",
        ),
        negative(
            "keyboard/skip-link-missing",
            r##"<nav><a href="/">Home</a></nav><a href="#m">Contenu</a><main id="m"><h1>T</h1></main>"##,
            "G1/ARIA11: Fragmentlink auf den Anfang von main, gleich wo er steht",
        )
        .variant("Ziel ist main"),
        negative(
            "keyboard/skip-link-missing",
            r##"<a href="#c">Aller au contenu</a><nav><a href="/fr/">Accueil</a></nav><div id="c"><h1>T</h1></div>"##,
            "G1: Fragmentlink vor dem ersten Link, der die Seite verlässt (auditmysite#642)",
        )
        .variant("Stellung, fremde Sprache"),
        na(
            "keyboard/skip-link-missing",
            r#"<main><h1>Title</h1><p>Plain text without links.</p></main>"#,
            "Understanding 2.4.1: ohne Navigationslinks oder Kopfbereich nichts zu überspringen",
        )
        .variant("Regression: Seite ohne Links"),
    ]
}

/// Deutscher Absatz, eindeutig: viele deutsche, keine englischen Funktionswörter.
const DE_PARAGRAPH: &str = "Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.";

fn parts() -> Vec<Fall> {
    vec![
        // --- language/part-unmarked (Heuristik: 100 Zeichen, 16 Wörter, 5 Funktionswörter)
        review(
            "language/part-unmarked",
            "<p>Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p>",
            "WCAG 3.1.2, H58: deutscher Absatz auf englischer Seite ohne lang — heuristisch, REVIEW",
        ),
        review(
            "language/part-unmarked",
            "<p>der die das und ist Wort Wort Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik-Schwelle der Regel (aus auditmysite): ab 100 Zeichen, 16 Wörtern, 5 Funktionswörtern",
        )
        .variant("Grenze: genau 100 Zeichen, 16 Wörter, 5 Marker"),
        review(
            "language/part-unmarked",
            "<p>der die das und ist the Wort Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik: höchstens ein Funktionswort der Seitensprache",
        )
        .variant("Grenze: ein englisches Funktionswort"),
        review(
            "language/part-unmarked",
            r#"<!DOCTYPE html><html lang="de"><head><title>T</title></head><body><p>The dog and the cat are with the family on a long journey through the mountains, and this is not over.</p></body></html>"#,
            "H58: englischer Absatz auf deutscher Seite",
        )
        .variant("Seite deutsch, Passage englisch"),
        negative(
            "language/part-unmarked",
            "<p>der die das und ist Wort Wort Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik-Schwelle: unter 100 Zeichen keine Aussage",
        )
        .variant("Grenze: 99 Zeichen"),
        negative(
            "language/part-unmarked",
            "<p>der die das und ist Wort Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik-Schwelle: unter 16 Wörtern keine Aussage",
        )
        .variant("Grenze: 15 Wörter"),
        negative(
            "language/part-unmarked",
            "<p>der die das und Wort Wort Wort Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik-Schwelle: vier Funktionswörter genügen nicht",
        )
        .variant("Grenze: 4 Marker"),
        negative(
            "language/part-unmarked",
            "<p>der die das und ist the and Wort Wort Wort Wort Wort Wort Wort Wort Worteeeeeeeeeeeeeeeeeeeeeeeeeeee</p>",
            "Heuristik: zwei Funktionswörter der Seitensprache machen es uneindeutig",
        )
        .variant("Grenze: zwei englische Funktionswörter"),
        negative(
            "language/part-unmarked",
            r#"<p lang="de">Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p>"#,
            "H58: Passage mit eigenem lang",
        )
        .variant("lang ausgezeichnet"),
        negative(
            "language/part-unmarked",
            r#"<p translate="no">Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p><address><p>Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p></address>"#,
            "translate=no und Adressen sind ausgenommen (3.1.2: Eigennamen); Zitate nicht mehr, siehe regressionen.rs",
        )
        .variant("ausgenommen"),
        negative(
            "language/part-unmarked",
            r#"<p hidden>Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p>"#,
            "3.1.2 betrifft wahrgenommenen Inhalt",
        )
        .variant("versteckt"),
        negative(
            "language/part-unmarked",
            "<p>The dog and the cat are with the family on a long journey through the mountains, and this is not over.</p>",
            "Passage in der Seitensprache",
        ),
        na(
            "language/part-unmarked",
            "<p>Short text.</p>",
            "Kein Textblock lang genug",
        ),
        // --- language/part-undetermined
        untested(
            "language/part-undetermined",
            r#"<!DOCTYPE html><html lang="fr"><head><title>T</title></head><body><p>Bonjour.</p></body></html>"#,
            "WCAG 3.1.2: auf Seiten außer de/en kann die Regel nichts sagen — nicht geprüft ist nicht bestanden",
        ),
        negative(
            "language/part-undetermined",
            r#"<!DOCTYPE html><html lang="en-GB"><head><title>T</title></head><body><p>Hello.</p></body></html>"#,
            "Englische Seite: die Erkennung läuft",
        ),
        negative(
            "language/part-undetermined",
            r#"<!DOCTYPE html><html lang="DE-at"><head><title>T</title></head><body><p>Hallo.</p></body></html>"#,
            "BCP 47: Primärsubtag ohne Rücksicht auf Schreibung",
        )
        .variant("Großschreibung"),
        na(
            "language/part-undetermined",
            r#"<!DOCTYPE html><html><head><title>T</title></head><body><p>Hello.</p></body></html>"#,
            "Ohne Seitensprache meldet document/lang-missing",
        ),
    ]
}

fn abbreviations() -> Vec<Fall> {
    vec![
        review(
            "language/abbreviation-unexpanded",
            r#"<p><abbr>W3C</abbr></p>"#,
            "WCAG 3.1.4, H28: Langform kann auch im Text oder Glossar stehen (G97, G55) — REVIEW",
        ),
        review(
            "language/abbreviation-unexpanded",
            r#"<p><acronym>NASA</acronym> <abbr title=" ">EU</abbr></p>"#,
            "H28: acronym; title aus Leerraum nennt keine Langform",
        )
        .variant("acronym, leeres title")
        .count(2),
        negative(
            "language/abbreviation-unexpanded",
            r#"<p><abbr title="World Wide Web Consortium">W3C</abbr></p>"#,
            "H28",
        ),
        negative(
            "language/abbreviation-unexpanded",
            r#"<p hidden><abbr>W3C</abbr></p>"#,
            "Versteckter Inhalt wird nicht wahrgenommen",
        )
        .variant("versteckt"),
        na(
            "language/abbreviation-unexpanded",
            r#"<p>World Wide Web Consortium</p>"#,
            "Keine Abkürzung ausgezeichnet",
        ),
    ]
}

fn refresh() -> Vec<Fall> {
    vec![
        fail(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="120">"#,
            "WCAG 2.2.1, F41, ACT bc659a",
        ),
        fail(
            "timing/meta-refresh",
            r#"<meta http-equiv="Refresh" content="5; url=/next">"#,
            "F40: verzögerte Weiterleitung; http-equiv ohne Rücksicht auf Schreibung",
        )
        .variant("Weiterleitung"),
        fail(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="1">"#,
            "ACT bc659a: jede Frist über 0",
        )
        .variant("Grenze: 1 s"),
        fail(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="72000">"#,
            "WCAG 2.2.1 „longer than 20 hours“, ACT bc659a „greater than 72000“",
        )
        .variant("Regression: Grenze genau 20 h"),
        negative(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="0; url=/start">"#,
            "ACT bc659a, H76: sofortige Weiterleitung ist keine Frist",
        )
        .variant("Grenze: 0 s"),
        negative(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="72001">"#,
            "WCAG 2.2.1, 20-Stunden-Ausnahme",
        )
        .variant("Grenze: 20 h + 1 s"),
        negative(
            "timing/meta-refresh",
            r#"<meta http-equiv="refresh" content="soon">"#,
            "HTML shared declarative refresh steps: ohne Ziffern ignoriert",
        )
        .variant("ungültiger Wert"),
        na(
            "timing/meta-refresh",
            r#"<meta name="description" content="120">"#,
            "ACT bc659a: nur meta http-equiv=refresh",
        ),
    ]
}

fn sections() -> Vec<Fall> {
    vec![
        review(
            "headings/section-without-heading",
            r#"<h1>T</h1><article><p>Story</p></article><article><p>Story 2</p></article>"#,
            "WCAG 2.4.10 (AAA), G141: Artikel ohne Überschrift — inhaltlich zu entscheiden, REVIEW",
        )
        .count(1),
        review(
            "headings/section-without-heading",
            r#"<h1>T</h1><section aria-label="News"><p>Item</p></section>"#,
            "2.4.10: benannter Abschnitt (region) ohne Überschrift",
        )
        .variant("benanntes section"),
        negative(
            "headings/section-without-heading",
            r#"<h1>T</h1><article><h2>Story</h2><p>Text</p></article>"#,
            "G141: Abschnitt mit Überschrift",
        ),
        negative(
            "headings/section-without-heading",
            r#"<h1>T</h1><section aria-label="News"><div role="heading" aria-level="2">News</div></section>"#,
            "ARIA12: Überschrift über role=heading",
        )
        .variant("ARIA-Überschrift"),
        na(
            "headings/section-without-heading",
            r#"<h1>T</h1><section><p>Unnamed</p></section><nav><a href="/">Home</a></nav>"#,
            "Unbenanntes section ist keine region; Navigation braucht keine Überschrift",
        ),
    ]
}

#[test]
fn german_paragraph_is_long_enough() {
    assert!(DE_PARAGRAPH.chars().count() >= 100);
}
