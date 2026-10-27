//! Gruppe „namen“: Namen von Links, Buttons, SVG, Dialogen, Aufklappern,
//! Rahmen und Pflichtrollen, Linktexte, Live-Regionen, Label in Name
//! (semantics.rs, names.rs) und Bild-/Medienregeln (media.rs).

use super::super::{Fall, Modus, fail, na, negativ, review};

pub fn faelle() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(links_name());
    v.extend(buttons_name());
    v.extend(svg_name());
    v.extend(linktexte());
    v.extend(pflichtnamen());
    v.extend(dialoge());
    v.extend(summary_und_live());
    v.extend(label_in_name());
    v.extend(medien());
    v
}

fn links_name() -> Vec<Fall> {
    const ID: &str = "links/name-missing";
    vec![
        fail(
            ID,
            r#"<a href="/start"></a>"#,
            "WCAG 2.4.4/4.1.2, F89; ACT c487ae: Link ohne zugänglichen Namen",
        )
        .anzahl(1),
        fail(
            ID,
            r#"<a href="/start"><img src="home.png"></a>"#,
            "ACT c487ae, F89: Bild ohne alt als einziger Inhalt stiftet keinen Namen",
        )
        .variante("Bild ohne alt im Link"),
        fail(
            ID,
            r#"<a href="/start" aria-labelledby="fehlt"></a>"#,
            "accname 1.2 Schritt 2B: Verweis ohne gültiges Ziel ergibt keinen Namen",
        )
        .variante("aria-labelledby ins Leere"),
        fail(
            ID,
            r#"<div><template shadowrootmode="open"><a href="/start" aria-labelledby="l"></a></template></div><span id="l">Home</span>"#,
            "accname 1.2 / DOM: IDREFs lösen nur im eigenen Baum auf; über die Shadow-Grenze kein Name",
        )
        .variante("Shadow DOM: Verweis über die Grenze"),
        negativ(
            ID,
            r#"<a href="/start">Home</a>"#,
            "accname 1.2 Schritt 2F: Name aus dem Inhalt (G91)",
        ),
        negativ(
            ID,
            r#"<a href="/start"><img src="home.png" alt="Home"></a>"#,
            "WCAG H30: alt des Bildes im Link stiftet den Namen",
        )
        .variante("alt"),
        negativ(
            ID,
            r#"<a href="/start" aria-label="Home"><svg aria-hidden="true"></svg></a>"#,
            "WCAG ARIA8: aria-label benennt den Link",
        )
        .variante("aria-label"),
        negativ(
            ID,
            r#"<span id="l">Home</span><a href="/start" aria-labelledby="l"></a>"#,
            "WCAG ARIA7: aria-labelledby benennt den Link",
        )
        .variante("aria-labelledby"),
        negativ(
            ID,
            r#"<a href="/start" title="Home"></a>"#,
            "accname 1.2 Schritt 2I: title als letzte Rückfallebene (H33)",
        )
        .variante("title"),
        negativ(
            ID,
            r#"<div><template shadowrootmode="open"><a href="/start"><x-slotted>Home</x-slotted></a></template></div>"#,
            "DOM/accname: eingesetzter Inhalt gehört zum flachen Baum und stiftet den Namen",
        )
        .variante("Shadow DOM: eingesetzter Text"),
        na(
            ID,
            r#"<p>Kein Link hier.</p><a>Anker ohne href</a>"#,
            "HTML: a ohne href ist kein Link (ACT c487ae gilt für Rolle link)",
        ),
        na(
            ID,
            r#"<a href="/start" hidden></a><a href="/x" style="display:none"></a><a href="/y" aria-hidden="true" tabindex="-1"></a>"#,
            "ACT c487ae: nur Links im Accessibility-Tree",
        )
        .variante("versteckt: hidden, display:none, aria-hidden"),
    ]
}

fn buttons_name() -> Vec<Fall> {
    const ID: &str = "buttons/name-missing";
    vec![
        fail(
            ID,
            r#"<button type="button"></button>"#,
            "WCAG 4.1.2, F68; ACT 97a4e1: Button ohne zugänglichen Namen",
        )
        .anzahl(1),
        fail(
            ID,
            r#"<div role="button" tabindex="0"></div>"#,
            "ACT 97a4e1: gilt auch für die ARIA-Rolle button",
        )
        .variante("ARIA-Rolle"),
        fail(
            ID,
            r#"<input type="button">"#,
            "HTML-AAM input type=button: ohne value kein Name; ACT 97a4e1",
        )
        .variante("input type=button ohne value"),
        negativ(
            ID,
            r#"<button type="button">Save</button>"#,
            "accname 1.2 Schritt 2F: Name aus dem Inhalt",
        ),
        negativ(
            ID,
            r#"<button type="button" aria-label="Close"><svg aria-hidden="true"></svg></button>"#,
            "WCAG ARIA14: aria-label benennt den Icon-Button",
        )
        .variante("aria-label"),
        negativ(
            ID,
            r#"<span id="t">Close</span><div role="button" tabindex="0" aria-labelledby="t"></div>"#,
            "WCAG ARIA16: aria-labelledby",
        )
        .variante("ARIA-Rolle, aria-labelledby"),
        negativ(
            ID,
            r#"<input type="submit">"#,
            "HTML-AAM input type=submit: ohne value der Vorgabename „Submit“",
        )
        .variante("input type=submit"),
        negativ(
            ID,
            r#"<button type="button" title="Close"></button>"#,
            "accname 1.2 Schritt 2I: title",
        )
        .variante("title"),
        na(
            ID,
            r#"<p>Text</p><a href="/x">Link</a>"#,
            "ACT 97a4e1: nur Elemente mit Rolle button",
        ),
        na(
            ID,
            r#"<button hidden></button><button style="display:none"></button><div aria-hidden="true"><button tabindex="-1"></button></div>"#,
            "ACT 97a4e1: nur Buttons im Accessibility-Tree",
        )
        .variante("versteckt"),
    ]
}

fn svg_name() -> Vec<Fall> {
    const ID: &str = "svg/name-missing";
    vec![
        fail(
            ID,
            r#"<svg role="img"><circle r="5"></circle></svg>"#,
            "ACT 7d6734: SVG mit Bildrolle braucht einen Namen (1.1.1)",
        )
        .anzahl(1),
        fail(
            ID,
            r#"<svg role="graphics-document"></svg>"#,
            "ACT 7d6734, Graphics-ARIA: graphics-document ist Bildrolle",
        )
        .variante("graphics-document"),
        review(
            ID,
            r#"<svg><circle r="5"></circle></svg>"#,
            "docs/packages/a11y-rules.md (auditmysite#779): SVG ohne Rolle kann Schmuck sein → REVIEW",
        )
        .variante("ohne Rolle"),
        negativ(
            ID,
            r#"<svg role="img" aria-label="Sales chart"></svg>"#,
            "WCAG ARIA6: aria-label",
        ),
        negativ(
            ID,
            r#"<svg role="img"><title>Sales chart</title></svg>"#,
            "SVG-AAM: title-Kind stiftet den Namen",
        )
        .variante("title-Element"),
        negativ(
            ID,
            r#"<a href="/cart" aria-label="Cart"><svg><path d="M0 0"></path></svg></a>"#,
            "WCAG 1.1.1 Understanding: Icon in benanntem Bedienelement trägt keine eigene Aussage",
        )
        .variante("in benanntem Link"),
        na(
            ID,
            r#"<svg aria-hidden="true"></svg><svg role="presentation"></svg><svg role="none"></svg>"#,
            "WCAG H67/ARIA: ausdrücklich dekorativ ausgezeichnete Grafik",
        )
        .variante("dekorativ"),
        na(ID, r#"<p>Kein SVG</p>"#, "ACT 7d6734: gilt nur für svg"),
    ]
}

fn linktexte() -> Vec<Fall> {
    vec![
        // --- links/ambiguous-name ---
        review(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b">Pricing</a>"#,
            "WCAG 2.4.4/2.4.9, ACT fd3a94: gleicher Name, verschiedene Ziele → Prüfung",
        )
        .anzahl(2),
        review(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b">pricing</a>"#,
            "ACT fd3a94: Namen werden ohne Groß-/Kleinschreibung verglichen (Vorlesen unterscheidet sie nicht)",
        )
        .variante("Schreibung")
        .anzahl(2),
        negativ(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/a">Pricing</a>"#,
            "ACT fd3a94: gleicher Name, gleiches Ziel ist gleicher Zweck",
        ),
        negativ(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b" hidden>Pricing</a>"#,
            "ACT fd3a94: nur Links im Accessibility-Tree zählen",
        )
        .variante("zweiter versteckt"),
        na(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a>"#,
            "ACT fd3a94: braucht mindestens zwei gleichnamige Links (Grenze 1)",
        )
        .variante("Grenze: ein Link"),
        // --- links/generic-name ---
        review(
            "links/generic-name",
            r#"<div><a href="/report">Read more</a></div>"#,
            "WCAG 2.4.4 Understanding, F84: nichtssagender Linktext",
        )
        .anzahl(1),
        review(
            "links/generic-name",
            r#"<p>Our annual report is out. <a href="/report">Read more →</a></p>"#,
            "WCAG 2.4.4, G53: Kontext kann klären — deshalb REVIEW, nicht FAIL; Pfeil zählt nicht zum Text",
        )
        .variante("mit Kontext, nachgestellter Pfeil"),
        review(
            "links/generic-name",
            r#"<div><a href="/report">Weiterlesen</a></div>"#,
            "WCAG 2.4.4: nichtssagend auch auf Deutsch",
        )
        .variante("deutsch"),
        negativ(
            "links/generic-name",
            r#"<div><a href="/report">Annual report 2025</a></div>"#,
            "WCAG 2.4.4, G91: Linktext beschreibt das Ziel",
        ),
        na(
            "links/generic-name",
            r#"<p>Read more</p>"#,
            "WCAG 2.4.4 gilt nur für Links",
        ),
        // --- links/url-name ---
        review(
            "links/url-name",
            r#"<div><a href="https://example.com/r">https://example.com/r</a></div>"#,
            "WCAG 2.4.4 Understanding: nackte Adresse nennt den Zweck nicht",
        )
        .anzahl(1),
        review(
            "links/url-name",
            r#"<div><a href="https://example.org">www.example.org</a></div>"#,
            "WCAG 2.4.4: Adresse ohne Schema",
        )
        .variante("www."),
        negativ(
            "links/url-name",
            r#"<p>The full report is available at <a href="https://example.com/r">https://example.com/r</a>.</p>"#,
            "WCAG 2.4.4, G53/H77: Zweck aus dem Satz bestimmbar",
        )
        .variante("mit Kontext"),
        negativ(
            "links/url-name",
            r#"<div><a href="https://example.com/r">Visit example.com today</a></div>"#,
            "WCAG 2.4.4: Satz mit Adresse ist kein nackter Adresslink",
        ),
        na(
            "links/url-name",
            r#"<div>https://example.com/r</div>"#,
            "WCAG 2.4.4 gilt nur für Links",
        ),
        // --- links/single-character-name ---
        review(
            "links/single-character-name",
            r#"<div><a href="/close">x</a></div>"#,
            "WCAG 2.4.4: ein einzelnes Zeichen nennt keinen Zweck",
        )
        .anzahl(1),
        negativ(
            "links/single-character-name",
            r#"<div><a href="/close">xy</a></div>"#,
            "Grenze: zwei Zeichen sind kein Einzelzeichen",
        )
        .variante("Grenze: 2 Zeichen"),
        negativ(
            "links/single-character-name",
            r#"<nav><a href="/p/2">2</a></nav>"#,
            "WCAG 2.4.4 Understanding: Seitenzahl im Blätterkontext verständlich",
        )
        .variante("Ziffer"),
        na(
            "links/single-character-name",
            r#"<p>x</p>"#,
            "WCAG 2.4.4 gilt nur für Links",
        ),
        // --- links/new-window-unannounced ---
        review(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_blank">Annual report</a>"#,
            "WCAG 3.2.5 (AAA), G201: neues Fenster ohne Ankündigung",
        )
        .anzahl(1),
        review(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target=" _BLANK ">Annual report</a>"#,
            "HTML: Browsing-Context-Schlüsselwörter ASCII-case-insensitive",
        )
        .variante("Schreibung"),
        negativ(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_blank">Annual report (opens in new tab)</a>"#,
            "WCAG G201: Ankündigung im Linktext",
        ),
        negativ(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_blank" aria-label="Annual report, opens in new window">Annual report</a>"#,
            "WCAG G201: Ankündigung im zugänglichen Namen",
        )
        .variante("aria-label"),
        na(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_self">Annual report</a><a href="/x">Other</a>"#,
            "G201 gilt nur für Links, die ein neues Fenster öffnen",
        ),
        // --- links/purpose-link-only ---
        review(
            "links/purpose-link-only",
            r#"<p>Our annual report is out. <a href="/report">Read more</a></p>"#,
            "WCAG 2.4.9 (AAA): Linktext allein muss den Zweck nennen, Kontext zählt nicht",
        )
        .anzahl(1),
        negativ(
            "links/purpose-link-only",
            r#"<p><a href="/report">Read the annual report</a></p>"#,
            "WCAG 2.4.9, G91: Linktext nennt den Zweck",
        ),
        na(
            "links/purpose-link-only",
            r#"<p>Read more</p>"#,
            "WCAG 2.4.9 gilt nur für Links",
        ),
    ]
}

fn pflichtnamen() -> Vec<Fall> {
    const ID: &str = "names/required-missing";
    vec![
        fail(
            ID,
            r#"<div role="checkbox" aria-checked="false" tabindex="0"></div>"#,
            "WAI-ARIA 1.2 checkbox: Accessible Name Required; WCAG 4.1.2, ACT e086e5",
        )
        .anzahl(1),
        fail(
            ID,
            r#"<div role="slider" aria-valuenow="5" tabindex="0"></div>"#,
            "WAI-ARIA 1.2 slider: Accessible Name Required",
        )
        .variante("slider"),
        fail(
            ID,
            r#"<span role="link" tabindex="0"></span>"#,
            "WAI-ARIA 1.2 link: Name Required (ARIA-Rolle, nicht a[href])",
        )
        .variante("ARIA-Link"),
        fail(
            ID,
            r#"<span id="leer"></span><input type="checkbox" aria-labelledby="leer">"#,
            "ACT e086e5: behauptete Beschriftung, die leer ausgeht",
        )
        .variante("natives Feld, leere Beschriftung"),
        negativ(
            ID,
            r#"<div role="checkbox" aria-checked="false" tabindex="0" aria-label="Accept terms"></div>"#,
            "WCAG ARIA14: aria-label",
        ),
        negativ(
            ID,
            r#"<span id="v">Volume</span><div role="slider" aria-valuenow="5" tabindex="0" aria-labelledby="v"></div>"#,
            "WCAG ARIA16: aria-labelledby",
        )
        .variante("aria-labelledby"),
        negativ(
            ID,
            r#"<label><input type="checkbox"> Accept terms</label>"#,
            "WCAG H44: label-Element",
        )
        .variante("label"),
        negativ(
            ID,
            r#"<div role="radio" aria-checked="false" tabindex="0">Yes</div>"#,
            "WAI-ARIA 1.2 radio: Name aus dem Inhalt",
        )
        .variante("Inhalt"),
        na(
            ID,
            r#"<div>Text</div>"#,
            "WAI-ARIA 1.2: keine Rolle mit Namenspflicht",
        ),
        na(
            ID,
            r#"<input type="checkbox">"#,
            "Abgrenzung: natives Feld ganz ohne Beschriftung meldet forms/label-missing (structure.rs)",
        )
        .variante("abgegrenzt an forms/label-missing"),
        na(
            ID,
            r#"<div role="checkbox" aria-checked="false" hidden></div>"#,
            "WAI-ARIA 1.2: nur Elemente im Accessibility-Tree",
        )
        .variante("versteckt"),
        // --- names/symbol-only ---
        review(
            "names/symbol-only",
            r#"<button type="button" aria-label="×"></button>"#,
            "WCAG 4.1.2 Understanding / 2.4.6: Name aus einem Symbol ist nicht beschreibend",
        )
        .anzahl(1),
        review(
            "names/symbol-only",
            r#"<a href="/menu" aria-label="☰"></a>"#,
            "WCAG 4.1.2: gilt für jede interaktive Rolle",
        )
        .variante("Link"),
        negativ(
            "names/symbol-only",
            r#"<button type="button" aria-label="Close">×</button>"#,
            "WCAG ARIA14: beschreibender Name, sichtbares Symbol",
        ),
        negativ(
            "names/symbol-only",
            r#"<button type="button" aria-label="Ä"></button>"#,
            "Unicode: Buchstaben sind keine Symbole",
        )
        .variante("Buchstabe"),
        negativ(
            "names/symbol-only",
            r#"<button type="button" aria-label="××"></button>"#,
            "Grenze: zwei Zeichen sind kein Einzelsymbol",
        )
        .variante("Grenze: 2 Zeichen"),
        na(
            "names/symbol-only",
            r#"<p aria-label="×">Text</p>"#,
            "WAI-ARIA 1.2: keine interaktive Rolle",
        ),
    ]
}

fn dialoge() -> Vec<Fall> {
    vec![
        fail(
            "dialog/name-missing",
            r#"<div role="dialog" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2 dialog: Accessible Name Required; WCAG 4.1.2",
        )
        .anzahl(1),
        fail(
            "dialog/name-missing",
            r#"<div role="alertdialog" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2 alertdialog: Accessible Name Required",
        )
        .variante("alertdialog"),
        fail(
            "dialog/name-missing",
            r#"<dialog open><p>Body</p></dialog>"#,
            "HTML-AAM: dialog hat Rolle dialog; Name Required",
        )
        .variante("natives dialog"),
        negativ(
            "dialog/name-missing",
            r#"<div role="dialog" aria-modal="true" aria-labelledby="h"><h2 id="h">Settings</h2></div>"#,
            "WAI-ARIA APG: Dialog über seine Überschrift benannt (ARIA16)",
        ),
        negativ(
            "dialog/name-missing",
            r#"<dialog open aria-label="Settings"><p>Body</p></dialog>"#,
            "WCAG ARIA6: aria-label",
        )
        .variante("natives dialog, aria-label"),
        na(
            "dialog/name-missing",
            r#"<dialog><p>Body</p></dialog><div role="dialog" hidden></div>"#,
            "HTML: geschlossener dialog ist nicht dargestellt; versteckt nicht im Baum",
        )
        .variante("geschlossen/versteckt"),
        na(
            "dialog/name-missing",
            r#"<div><p>Body</p></div>"#,
            "keine Dialogrolle",
        ),
        // --- dialog/modal-unmarked ---
        review(
            "dialog/modal-unmarked",
            r#"<div role="dialog" aria-label="Settings"><p>Body</p></div>"#,
            "WAI-ARIA 1.2 aria-modal; docs/packages/a11y-rules.md: Modalität zeigt erst das Verhalten → REVIEW",
        )
        .anzahl(1),
        negativ(
            "dialog/modal-unmarked",
            r#"<div role="dialog" aria-label="Settings" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2: aria-modal=true",
        ),
        na(
            "dialog/modal-unmarked",
            r#"<dialog open aria-label="Settings"><p>Body</p></dialog>"#,
            "natives dialog: Modalität über showModal(), nicht im Markup (Regeldoku)",
        )
        .variante("natives dialog"),
        na(
            "dialog/modal-unmarked",
            r#"<div><p>Body</p></div>"#,
            "keine Dialogrolle",
        ),
    ]
}

fn summary_und_live() -> Vec<Fall> {
    vec![
        fail(
            "summary/name-missing",
            r#"<details><summary></summary><p>Hidden</p></details>"#,
            "HTML-AAM summary; WCAG 4.1.2: Umschalter ohne Namen",
        )
        .anzahl(1),
        negativ(
            "summary/name-missing",
            r#"<details><summary>Shipping details</summary><p>Hidden</p></details>"#,
            "accname 1.2: Name aus dem Inhalt",
        ),
        negativ(
            "summary/name-missing",
            r#"<details><summary aria-label="Shipping details"></summary><p>Hidden</p></details>"#,
            "WCAG ARIA14: aria-label",
        )
        .variante("aria-label"),
        negativ(
            "summary/name-missing",
            r#"<details open><summary>More</summary><summary></summary></details>"#,
            "HTML: nur die erste summary bedient das details-Element",
        )
        .variante("zweite summary"),
        na(
            "summary/name-missing",
            r#"<details><p>Text</p></details><summary></summary>"#,
            "HTML: summary außerhalb von details ist kein Umschalter",
        ),
        // --- status/live-overridden ---
        fail(
            "status/live-overridden",
            r#"<div role="status" aria-live="off">Saved</div>"#,
            "WAI-ARIA 1.2 aria-live=off: Änderungen nicht ansagen; WCAG 4.1.3",
        )
        .anzahl(1),
        fail(
            "status/live-overridden",
            r#"<output aria-live="off">3 results</output>"#,
            "HTML-AAM: output hat Rolle status; WCAG 4.1.3",
        )
        .variante("natives output"),
        review(
            "status/live-overridden",
            r#"<div role="alert" aria-live="polite">Error</div>"#,
            "WAI-ARIA 1.2: Dringlichkeit darf überschrieben werden; Regeldoku → REVIEW",
        )
        .variante("andere Dringlichkeit"),
        negativ(
            "status/live-overridden",
            r#"<div role="status" aria-live="polite">Saved</div>"#,
            "WAI-ARIA 1.2: Vorgabe von status ist polite",
        ),
        negativ(
            "status/live-overridden",
            r#"<div role="log" aria-live="assertive">Chat</div>"#,
            "Regeldoku: Dringlichkeit von log umzustellen wird nicht bemängelt",
        )
        .variante("log"),
        na(
            "status/live-overridden",
            r#"<div aria-live="off">Ticker</div>"#,
            "WAI-ARIA 1.2: ohne Live-Rolle keine Vorgabe, die überschrieben würde",
        ),
    ]
}

fn label_in_name() -> Vec<Fall> {
    const ID: &str = "label-in-name/mismatch";
    vec![
        fail(
            ID,
            r#"<button type="button" aria-label="Submit form">Send</button>"#,
            "WCAG 2.5.3, F96; ACT 2ee8b8: Name enthält den sichtbaren Text nicht",
        )
        .anzahl(1),
        fail(
            ID,
            r#"<a href="/x" aria-label="Open">Settings</a>"#,
            "Grenze: sichtbarer Text genau doppelt so lang wie der Name → noch Verstoß (Regeldoku, auditmysite#513)",
        )
        .variante("Grenze: 8 zu 4 Zeichen"),
        review(
            ID,
            r#"<a href="/x" aria-label="Open">Settings 2</a>"#,
            "Grenze: mehr als doppelt so lang → zusammengesetztes Widget, REVIEW (auditmysite#513)",
        )
        .variante("Grenze: 9 zu 4 Zeichen"),
        review(
            ID,
            r#"<a href="/x" aria-label="annual report 2025">Report annual</a>"#,
            "WCAG 2.5.3 Understanding: alle Wörter im Namen, andere Reihenfolge → REVIEW",
        )
        .variante("umgestellt"),
        negativ(
            ID,
            r#"<button type="button" aria-label="Send message">Send</button>"#,
            "WCAG 2.5.3, G208: sichtbarer Text am Anfang des Namens",
        ),
        negativ(
            ID,
            r#"<a href="/next" aria-label="Next page">→</a>"#,
            "WCAG 2.5.3 Understanding: Symbole sind ausgenommen",
        )
        .variante("Symbol"),
        negativ(
            ID,
            r#"<button type="button">Send</button>"#,
            "ACT 2ee8b8: Name aus dem Inhalt enthält ihn",
        )
        .variante("Name aus Inhalt"),
        na(
            ID,
            r#"<p aria-label="Other">Text</p>"#,
            "ACT 2ee8b8: nur Rollen mit Name aus Inhalt",
        ),
    ]
}

fn medien() -> Vec<Fall> {
    vec![
        // --- images/area-alt-missing ---
        fail(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG 1.1.1, H24; ACT c487ae: aktiver area-Link ohne Textalternative",
        )
        .anzahl(1),
        fail(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt=" " shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: leeres alt am Link-Bereich ergibt keinen Namen",
        )
        .variante("alt leer"),
        negativ(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: alt am area",
        ),
        negativ(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" aria-label="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "HTML-AAM area: aria-label vor alt",
        )
        .variante("aria-label"),
        na(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area shape="rect" coords="0,0,10,10"></map>"##,
            "HTML: area ohne href ist kein Link (H24 gilt für aktive Bereiche)",
        ),
        // --- images/input-alt-missing ---
        fail(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png">"#,
            "WCAG 1.1.1, H36; ACT 59796f: Grafik-Button ohne alt",
        )
        .anzahl(1),
        fail(
            "images/input-alt-missing",
            r#"<input type="IMAGE" src="go.png" alt="">"#,
            "H36: leeres alt; Typ ASCII-case-insensitive",
        )
        .variante("alt leer, Schreibung"),
        negativ(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" alt="Search">"#,
            "WCAG H36: alt beschreibt die Aktion",
        ),
        negativ(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" aria-label="Search">"#,
            "HTML-AAM input type=image: aria-label vor alt",
        )
        .variante("aria-label"),
        na(
            "images/input-alt-missing",
            r#"<input type="submit" value="Search">"#,
            "H36 gilt nur für input type=image",
        ),
        // --- images/server-side-map ---
        review(
            "images/server-side-map",
            r#"<a href="/map"><img src="map.png" alt="Map" ismap></a>"#,
            "WCAG 2.1.1, docs/packages/a11y-rules.md: serverseitige Imagemap → REVIEW",
        )
        .anzahl(1),
        negativ(
            "images/server-side-map",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: clientseitige Imagemap ist per Tastatur bedienbar",
        ),
        na(
            "images/server-side-map",
            r#"<img src="map.png" alt="Map" ismap>"#,
            "HTML (img, ismap): wirkt nur am Nachfahren eines a mit href — sonst keine serverseitige Imagemap",
        )
        .variante("Regression: ismap ohne Link"),
        na(
            "images/server-side-map",
            r#"<img src="photo.png" alt="Photo">"#,
            "keine Imagemap",
        ),
        // --- objects/alt-missing ---
        fail(
            "objects/alt-missing",
            r#"<object data="report.pdf" type="application/pdf"></object>"#,
            "WCAG 1.1.1; ACT 8fc3b6: object ohne Textalternative",
        )
        .anzahl(1),
        negativ(
            "objects/alt-missing",
            r#"<object data="report.pdf" type="application/pdf">Annual report (PDF)</object>"#,
            "WCAG H53: Ersatzinhalt im object",
        ),
        negativ(
            "objects/alt-missing",
            r#"<object data="chart.svg" type="image/svg+xml" aria-label="Sales chart"></object>"#,
            "ACT 8fc3b6: aria-label",
        )
        .variante("aria-label"),
        negativ(
            "objects/alt-missing",
            r#"<object data="chart.svg" type="image/svg+xml" title="Sales chart"></object>"#,
            "HTML-AAM object: title",
        )
        .variante("title"),
        na(
            "objects/alt-missing",
            r#"<img src="a.png" alt="A">"#,
            "ACT 8fc3b6 gilt nur für object",
        ),
        // --- media/audio-autoplay ---
        review(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay></audio>"#,
            "WCAG 1.4.2, F23: selbststartender Ton; Dauer/Bedienung nicht im Markup → REVIEW",
        )
        .anzahl(1),
        review(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay hidden></audio><div aria-hidden="true"><audio src="b.mp3" autoplay controls></audio></div>"#,
            "WCAG 1.4.2: Ton spielt unabhängig von Sichtbarkeit und Accessibility-Tree",
        )
        .variante("Regression: versteckt (hidden, aria-hidden)")
        .anzahl(2),
        negativ(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay muted controls></audio>"#,
            "WCAG 1.4.2: stummgeschaltet startet kein Ton",
        ),
        na(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" controls></audio>"#,
            "WCAG 1.4.2: ohne autoplay startet nichts von selbst",
        ),
        // --- frames/name-missing ---
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget"></iframe>"#,
            "WCAG 4.1.2; ACT cae760: iframe ohne Namen",
        )
        .anzahl(1),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" title=" "></iframe>"#,
            "ACT cae760: leerer title ist kein Name",
        )
        .variante("title leer"),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" width="2" height="2"></iframe>"#,
            "Grenze: 2×2 ist dargestellt (Ausnahme nur bis 1×1)",
        )
        .variante("Grenze: 2×2"),
        negativ(
            "frames/name-missing",
            r#"<iframe src="/widget" title="Weather forecast"></iframe>"#,
            "WCAG H64: title",
        ),
        negativ(
            "frames/name-missing",
            r#"<iframe src="/widget" aria-label="Weather forecast"></iframe>"#,
            "ACT cae760: aria-label",
        )
        .variante("aria-label"),
        negativ(
            "frames/name-missing",
            r#"<h2 id="w">Weather</h2><iframe src="/widget" aria-labelledby="w"></iframe>"#,
            "ACT cae760: aria-labelledby",
        )
        .variante("aria-labelledby"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" width="1" height="1"></iframe><iframe src="/t" width="0" height="0"></iframe>"#,
            "Regeldoku: technische Rahmen bis 1×1 (Grenze) sind nie zu sehen",
        )
        .variante("Grenze: 1×1"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" hidden></iframe><iframe src="/t" style="display:none"></iframe>"#,
            "ACT cae760: nur Rahmen im Accessibility-Tree",
        )
        .variante("versteckt"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" style="display: none !important"></iframe>"#,
            "ohne Stile: Inline display:none erkennt die Regel selbst",
        )
        .variante("ohne Stile")
        .modus(Modus::Semantik),
    ]
}
