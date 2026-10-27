//! Regressionen zu Abweichungen, die der Browserkorpus von auditmysite
//! gefunden hat (casoon/auditmysite#783, Sammlung „783-shared-bugs").
//! Jeder Fall scheitert an der Regel vor der Korrektur.

use super::super::{Fall, fail, na, negativ, review, untested};

pub fn faelle() -> Vec<Fall> {
    vec![
        // --- tables/headers-attr-invalid: versteckte Tabelle ---
        na(
            "tables/headers-attr-invalid",
            r#"<table hidden><tr><th id="h">Year</th></tr><tr><td headers="nothing-here">1</td></tr></table>"#,
            "ACT a25f45: gilt nur für Tabellen, die sichtbar und im Accessibility-Tree sind",
        )
        .variante("Regression: Tabelle mit hidden"),
        // --- lists/empty: Einträge in einem div ---
        negativ(
            "lists/empty",
            r#"<ul><div><li>Two speeds</li><li>LED work light</li></div></ul>"#,
            "HTML 4.4.5: die Liste hat Einträge; der Fehler ist die Struktur (lists/invalid-structure), nicht Leere",
        )
        .variante("Regression: li in div"),
        fail(
            "lists/invalid-structure",
            r#"<ul><div><li>Two speeds</li><li>LED work light</li></div></ul>"#,
            "HTML 4.4.5 (ul: null oder mehr li); WCAG 1.3.1, F46",
        )
        .variante("Regression: li in div")
        .anzahl(1),
        // --- viz/*: Figur in einem versteckten Bereich ---
        na(
            "viz/caption-missing",
            r#"<section hidden><figure data-viz="interactive"><canvas data-viz-live></canvas><div data-viz-text><p>Visitors per year rose.</p></div></figure></section>"#,
            "Darstellungskonvention v0; WCAG 2.2 Konformität: nicht Dargestelltes wird in diesem Zustand nicht beurteilt (wie display/text-hidden, auditmysite#725)",
        )
        .variante("Regression: Figur unter hidden"),
        na(
            "viz/static-missing",
            r#"<section hidden><figure data-viz="interactive"><canvas data-viz-live></canvas><div data-viz-text><p>Visitors per year rose.</p></div></figure></section>"#,
            "wie oben: nicht Dargestelltes braucht kein Standbild",
        )
        .variante("Regression: Figur unter hidden"),
        // --- language/part-unmarked: Zitate sind nicht ausgenommen ---
        review(
            "language/part-unmarked",
            "<blockquote><p>Der Hund und die Katze sind mit der Familie auf einer langen Reise durch die Berge von Bayern unterwegs gewesen.</p></blockquote>",
            "WCAG 3.1.2 nimmt nur Eigennamen, Fachbegriffe, Wörter unbestimmter Sprache und Lehnwörter aus; H58 zeigt ein fremdsprachiges Zitat",
        )
        .variante("Regression: blockquote"),
        // --- text/justified, text/line-height-tight: style-Attribut ---
        review(
            "text/justified",
            r#"<p style="text-align: justify">The new reading room opens on Monday with longer hours for everyone.</p>"#,
            "WCAG 1.4.8 (Blocksatz); CSS Cascade 4: das style-Attribut gilt wie eine Regel",
        )
        .variante("Regression: style-Attribut"),
        review(
            "text/line-height-tight",
            r#"<p style="line-height: 1.2">The new reading room opens on Monday with longer hours for everyone.</p>"#,
            "WCAG 1.4.8 (Zeilenabstand mindestens 1,5); CSS Cascade 4",
        )
        .variante("Regression: style-Attribut"),
        negativ(
            "text/line-height-tight",
            r#"<style>p { line-height: 1.2 }</style><p style="line-height: 1.6">The new reading room opens on Monday with longer hours for everyone.</p>"#,
            "CSS Cascade 4: das style-Attribut schlägt eine Regel ohne !important",
        )
        .variante("Regression: style-Attribut überschreibt"),
        review(
            "text/line-height-tight",
            r#"<style>p { line-height: 1.2 !important }</style><p style="line-height: 1.6">The new reading room opens on Monday with longer hours for everyone.</p>"#,
            "CSS Cascade 4: !important im Stylesheet schlägt das style-Attribut ohne !important",
        )
        .variante("Regression: !important gegen style-Attribut"),
        // --- forms/placeholder-as-label: aria-label ist nicht sichtbar ---
        review(
            "forms/placeholder-as-label",
            r#"<input type="text" aria-label="Porch note" placeholder="Porch note">"#,
            "WCAG 3.3.2 Understanding: der Platzhalter verschwindet bei der Eingabe; aria-label ist keine sichtbare Beschriftung",
        )
        .variante("Regression: aria-label + placeholder"),
        // --- Links über role="link" ---
        review(
            "links/generic-name",
            r#"<div><span role="link" tabindex="0">Click here</span></div>"#,
            "WCAG 2.4.4 gilt für jeden Link; WAI-ARIA 1.2 Rolle link",
        )
        .variante("Regression: role=link"),
        review(
            "links/url-name",
            r#"<div><span role="link" tabindex="0">https://example.org/press</span></div>"#,
            "WCAG 2.4.4 gilt für jeden Link; WAI-ARIA 1.2 Rolle link",
        )
        .variante("Regression: role=link"),
        // --- color/link-indistinct ---
        review(
            "color/link-indistinct",
            r#"<p>Read the <a href="/shipping" style="text-decoration:none;color:#2563c9">shipping terms</a> before you continue with the order.</p>"#,
            "WCAG 1.4.1, G183: 3,71:1 zum Text kann mit Hinweis bei Hover/Fokus genügen — das zeigt erst ein Prüfer",
        )
        .variante("Regression: G183, 3:1 zum Text")
        .anzahl(1),
        fail(
            "color/link-indistinct",
            r#"<p>Compare the <span role="link" tabindex="0" style="color:#0050a0">delivery rates</span> for each country before you order.</p>"#,
            "WCAG 1.4.1, F73: gilt für jeden Link; 2,66:1 zum Text reicht für G183 nicht",
        )
        .variante("Regression: role=link")
        .anzahl(1),
        // --- context/on-input: vorher darauf hingewiesen ---
        review(
            "context/on-input",
            r#"<p id="note">Choosing a store below opens its page right away.</p><label for="s">Store</label><select id="s" aria-describedby="note" onchange="location.href = this.value"><option value="/a">A</option></select>"#,
            "WCAG 3.2.2: erlaubt, wenn der Nutzer vorher darauf hingewiesen wurde (G13); ob die Beschreibung das tut, entscheidet ein Prüfer",
        )
        .variante("Regression: aria-describedby"),
        // --- keyboard/interactive-not-focusable: a ohne href ---
        negativ(
            "keyboard/interactive-not-focusable",
            r#"<a onclick="this.textContent = 'Saved'">Save trip</a>"#,
            "HTML-AAM: a ohne href ist generic, keine interaktive Rolle (meldet keyboard/click-handler-not-focusable)",
        )
        .variante("Regression: a ohne href"),
        // --- landmarks/content-outside: inert hinter modalem Dialog ---
        negativ(
            "landmarks/content-outside",
            r#"<header inert><p>Example shop</p></header><nav inert><a href="/">Home</a></nav><main><p inert>Our offers.</p><dialog open aria-label="Newsletter"><button>Close</button></dialog></main><footer inert><p>Imprint</p></footer>"#,
            "HTML inert: inerte Knoten stehen nicht im Accessibility-Tree; Landmarks behalten ihre Rolle (WAI-ARIA 1.2)",
        )
        .variante("Regression: inert hinter showModal()"),
        // --- label-in-name/mismatch: Name nur Teil des Sichtbaren ---
        fail(
            "label-in-name/mismatch",
            r#"<a href="/pricing" aria-label="Pricing">Pricing plans</a>"#,
            "WCAG 2.5.3: der Name muss den sichtbaren Text enthalten; Pricing enthält Pricing plans nicht",
        )
        .variante("Regression: Name ist Teil des Sichtbaren")
        .anzahl(1),
        // --- names/required-missing: tree, grid, treegrid ---
        fail(
            "names/required-missing",
            r#"<ul role="tree"><li role="treeitem" aria-selected="true">Documents</li></ul>"#,
            "WAI-ARIA 1.2 tree: Accessible Name Required; WCAG 4.1.2",
        )
        .variante("Regression: tree")
        .anzahl(1),
        fail(
            "names/required-missing",
            r#"<div role="grid"><div role="row"><div role="gridcell">1</div></div></div>"#,
            "WAI-ARIA 1.2 grid: Accessible Name Required; WCAG 4.1.2",
        )
        .variante("Regression: grid")
        .anzahl(1),
        negativ(
            "names/required-missing",
            r#"<ul role="tree" aria-label="Folders"><li role="treeitem" aria-selected="true">Documents</li></ul>"#,
            "WAI-ARIA 1.2 tree mit aria-label",
        )
        .variante("tree benannt"),
        // --- targets/*: Link um ein Bild, eingebettete Rahmen ---
        negativ(
            "targets/size-enhanced",
            r#"<main><a href="/maps/click" style="--w:145;--h:18"><img src="/town.png" alt="Clickable town map" style="--w:200;--h:120"></a></main>"#,
            "Understanding 2.5.5/2.5.8: das Ziel ist der Bereich, der den Klick annimmt — das 200 × 120 px große Bild im Link",
        )
        .variante("Regression: Inline-Link um ein Bild"),
        review(
            "targets/size-enhanced",
            r#"<main><a href="/maps/click" style="--w:30;--h:18"><img src="/town.png" alt="Map" style="--w:30;--h:30"></a></main>"#,
            "WCAG 2.5.5: auch mit dem Bild nur 30 × 30 px",
        )
        .variante("Inline-Link um ein kleines Bild")
        .anzahl(1),
        na(
            "targets/size-enhanced",
            r#"<iframe title="Location map" src="/map" style="--w:2;--h:2;border:0"></iframe>"#,
            "Understanding 2.5.8: ein Rahmen ist Behälter; seine Ziele beurteilt der Durchgang durch das eingebettete Dokument",
        )
        .variante("Regression: iframe"),
        na(
            "targets/size",
            r#"<iframe title="Location map" src="/map" style="--w:2;--h:2"></iframe><iframe title="Other" src="/o" style="--w:2;--h:2;--x:3"></iframe>"#,
            "Understanding 2.5.8: ein Rahmen ist kein Ziel",
        )
        .variante("Regression: iframe"),
        // --- keyboard/scrollable-region-not-focusable: leere Seite ---
        na(
            "keyboard/scrollable-region-not-focusable",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><body><!-- comment only --></body></html>"#,
            "WCAG 2.1.1: nichts, was scrollen könnte; gemessen hat der Host trotzdem (html, body)",
        )
        .variante("Regression: leerer body"),
        // --- objects/alt-missing: Ersatzinhalt eines geladenen Objekts ---
        negativ(
            "objects/alt-missing",
            r#"<object data="/catalogue.pdf" type="application/pdf"><a href="/catalogue.pdf" style="display:none">Download the exhibition catalogue (PDF)</a></object>"#,
            "WCAG 1.1.1, H53: der Inhalt des object ist seine Textalternative, auch wenn der Browser ihn nach dem Laden nicht darstellt",
        )
        .variante("Regression: Ersatzinhalt nicht dargestellt"),
        // --- color/link-indistinct: nicht dargestellter Ersatzlink ---
        negativ(
            "color/link-indistinct",
            r#"<!DOCTYPE html><html lang="en" style="--unmeasured: unrendered"><head><title>T</title></head><body><p>Get the <object data="/catalogue.pdf" type="application/pdf"><a href="/catalogue.pdf" style="display:none">catalogue</a></object> before the visit.</p></body></html>"#,
            "WCAG 1.4.1 betrifft Dargestelltes; ein Link ohne Stil neben gestyltem Elternelement wird nicht dargestellt und ist nicht ungemessen",
        )
        .variante("Regression: Ersatzlink eines geladenen object, Host ohne Stil für Unsichtbares"),
        untested(
            "color/link-indistinct",
            r#"<!DOCTYPE html><html lang="en" style="--unmeasured: style"><head><title>T</title></head><body><p>Read the <a href="/terms">terms</a> before you order.</p></body></html>"#,
            "docs/conventions.md: liefert der Host keine Stile, ist der Fall ungemessen, nicht bestanden",
        )
        .variante("Host ohne jeden Stil")
        .anzahl(1),
    ]
}
