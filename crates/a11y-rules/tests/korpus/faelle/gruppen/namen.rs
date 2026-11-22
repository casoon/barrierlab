//! Gruppe „namen“: Namen von Links, Buttons, SVG, Dialogen, Aufklappern,
//! Rahmen und Pflichtrollen, Linktexte, Live-Regionen, Label in Name
//! (semantics.rs, names.rs) und Bild-/Medienregeln (media.rs).

use super::super::{Fall, Mode, fail, na, negative, not_run, review};

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(links_name());
    v.extend(buttons_name());
    v.extend(svg_name());
    v.extend(link_texts());
    v.extend(required_names());
    v.extend(dialogs());
    v.extend(summary_and_live());
    v.extend(label_in_name());
    v.extend(media());
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
        .count(1),
        fail(
            ID,
            r#"<a href="/start"><img src="home.png"></a>"#,
            "ACT c487ae, F89: Bild ohne alt als einziger Inhalt stiftet keinen Namen",
        )
        .variant("Bild ohne alt im Link"),
        fail(
            ID,
            r#"<a href="/start" aria-labelledby="fehlt"></a>"#,
            "accname 1.2 Schritt 2B: Verweis ohne gültiges Ziel ergibt keinen Namen",
        )
        .variant("aria-labelledby ins Leere"),
        fail(
            ID,
            r#"<div><template shadowrootmode="open"><a href="/start" aria-labelledby="l"></a></template></div><span id="l">Home</span>"#,
            "accname 1.2 / DOM: IDREFs lösen nur im eigenen Baum auf; über die Shadow-Grenze kein Name",
        )
        .variant("Shadow DOM: Verweis über die Grenze"),
        negative(
            ID,
            r#"<a href="/start">Home</a>"#,
            "accname 1.2 Schritt 2F: Name aus dem Inhalt (G91)",
        ),
        negative(
            ID,
            r#"<a href="/start"><img src="home.png" alt="Home"></a>"#,
            "WCAG H30: alt des Bildes im Link stiftet den Namen",
        )
        .variant("alt"),
        negative(
            ID,
            r#"<a href="/start" aria-label="Home"><svg aria-hidden="true"></svg></a>"#,
            "WCAG ARIA8: aria-label benennt den Link",
        )
        .variant("aria-label"),
        negative(
            ID,
            r#"<span id="l">Home</span><a href="/start" aria-labelledby="l"></a>"#,
            "WCAG ARIA7: aria-labelledby benennt den Link",
        )
        .variant("aria-labelledby"),
        negative(
            ID,
            r#"<a href="/start" title="Home"></a>"#,
            "accname 1.2 Schritt 2I: title als letzte Rückfallebene (H33)",
        )
        .variant("title"),
        negative(
            ID,
            r#"<div><template shadowrootmode="open"><a href="/start"><x-slotted>Home</x-slotted></a></template></div>"#,
            "DOM/accname: eingesetzter Inhalt gehört zum flachen Baum und stiftet den Namen",
        )
        .variant("Shadow DOM: eingesetzter Text"),
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
        .variant("versteckt: hidden, display:none, aria-hidden"),
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
        .count(1),
        fail(
            ID,
            r#"<div role="button" tabindex="0"></div>"#,
            "ACT 97a4e1: gilt auch für die ARIA-Rolle button",
        )
        .variant("ARIA-Rolle"),
        fail(
            ID,
            r#"<input type="button">"#,
            "HTML-AAM input type=button: ohne value kein Name; ACT 97a4e1",
        )
        .variant("input type=button ohne value"),
        negative(
            ID,
            r#"<button type="button">Save</button>"#,
            "accname 1.2 Schritt 2F: Name aus dem Inhalt",
        ),
        negative(
            ID,
            r#"<button type="button" aria-label="Close"><svg aria-hidden="true"></svg></button>"#,
            "WCAG ARIA14: aria-label benennt den Icon-Button",
        )
        .variant("aria-label"),
        negative(
            ID,
            r#"<span id="t">Close</span><div role="button" tabindex="0" aria-labelledby="t"></div>"#,
            "WCAG ARIA16: aria-labelledby",
        )
        .variant("ARIA-Rolle, aria-labelledby"),
        negative(
            ID,
            r#"<input type="submit">"#,
            "HTML-AAM input type=submit: ohne value der Vorgabename „Submit“",
        )
        .variant("input type=submit"),
        negative(
            ID,
            r#"<button type="button" title="Close"></button>"#,
            "accname 1.2 Schritt 2I: title",
        )
        .variant("title"),
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
        .variant("versteckt"),
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
        .count(1),
        fail(
            ID,
            r#"<svg role="graphics-document"></svg>"#,
            "ACT 7d6734, Graphics-ARIA: graphics-document ist Bildrolle",
        )
        .variant("graphics-document"),
        fail(
            ID,
            r#"<svg><rect role="graphics-symbol" width="4" height="4"></rect></svg>"#,
            "ACT 7d6734: auch ein SVG-Element mit Bildrolle im SVG braucht einen Namen",
        )
        .variant("Teil mit Bildrolle"),
        negative(
            ID,
            r#"<svg><rect role="graphics-symbol" aria-label="Square" width="4" height="4"></rect></svg>"#,
            "ACT 7d6734, ARIA6: benannter Teil trägt die Aussage, das svg ohne Rolle bleibt ohne Befund",
        )
        .variant("benannter Teil mit Bildrolle"),
        review(
            ID,
            r#"<svg><circle r="5"></circle></svg>"#,
            "docs/packages/a11y-rules.md (auditmysite#779): SVG ohne Rolle kann Schmuck sein → REVIEW",
        )
        .variant("ohne Rolle"),
        negative(
            ID,
            r#"<svg role="img" aria-label="Sales chart"></svg>"#,
            "WCAG ARIA6: aria-label",
        ),
        negative(
            ID,
            r#"<svg role="img"><title>Sales chart</title></svg>"#,
            "SVG-AAM: title-Kind stiftet den Namen",
        )
        .variant("title-Element"),
        negative(
            ID,
            r#"<a href="/cart" aria-label="Cart"><svg><path d="M0 0"></path></svg></a>"#,
            "WCAG 1.1.1 Understanding: Icon in benanntem Bedienelement trägt keine eigene Aussage",
        )
        .variant("in benanntem Link"),
        na(
            ID,
            r#"<svg aria-hidden="true"></svg><svg role="presentation"></svg><svg role="none"></svg>"#,
            "WCAG H67/ARIA: ausdrücklich dekorativ ausgezeichnete Grafik",
        )
        .variant("dekorativ"),
        na(ID, r#"<p>Kein SVG</p>"#, "ACT 7d6734: gilt nur für svg"),
    ]
}

fn link_texts() -> Vec<Fall> {
    vec![
        // --- links/ambiguous-name ---
        review(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b">Pricing</a>"#,
            "WCAG 2.4.4/2.4.9, ACT fd3a94: gleicher Name, verschiedene Ziele → Prüfung",
        )
        .count(2),
        review(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b">pricing</a>"#,
            "ACT fd3a94: Namen werden ohne Groß-/Kleinschreibung verglichen (Vorlesen unterscheidet sie nicht)",
        )
        .variant("Schreibung")
        .count(2),
        negative(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/a">Pricing</a>"#,
            "ACT fd3a94: gleicher Name, gleiches Ziel ist gleicher Zweck",
        ),
        negative(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a><a href="/b" hidden>Pricing</a>"#,
            "ACT fd3a94: nur Links im Accessibility-Tree zählen",
        )
        .variant("zweiter versteckt"),
        na(
            "links/ambiguous-name",
            r#"<a href="/a">Pricing</a>"#,
            "ACT fd3a94: braucht mindestens zwei gleichnamige Links (Grenze 1)",
        )
        .variant("Grenze: ein Link"),
        // --- links/generic-name ---
        review(
            "links/generic-name",
            r#"<div><a href="/report">Read more</a></div>"#,
            "WCAG 2.4.4 Understanding, F84: nichtssagender Linktext",
        )
        .count(1),
        review(
            "links/generic-name",
            r#"<p>Our annual report is out. <a href="/report">Read more →</a></p>"#,
            "WCAG 2.4.4, G53: Kontext kann klären — deshalb REVIEW, nicht FAIL; Pfeil zählt nicht zum Text",
        )
        .variant("mit Kontext, nachgestellter Pfeil"),
        review(
            "links/generic-name",
            r#"<div><a href="/report">Weiterlesen</a></div>"#,
            "WCAG 2.4.4: nichtssagend auch auf Deutsch",
        )
        .variant("deutsch"),
        negative(
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
        .count(1),
        review(
            "links/url-name",
            r#"<div><a href="https://example.org">www.example.org</a></div>"#,
            "WCAG 2.4.4: Adresse ohne Schema",
        )
        .variant("www."),
        negative(
            "links/url-name",
            r#"<p>The full report is available at <a href="https://example.com/r">https://example.com/r</a>.</p>"#,
            "WCAG 2.4.4, G53/H77: Zweck aus dem Satz bestimmbar",
        )
        .variant("mit Kontext"),
        negative(
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
        .count(1),
        negative(
            "links/single-character-name",
            r#"<div><a href="/close">xy</a></div>"#,
            "Grenze: zwei Zeichen sind kein Einzelzeichen",
        )
        .variant("Grenze: 2 Zeichen"),
        negative(
            "links/single-character-name",
            r#"<nav><a href="/p/2">2</a></nav>"#,
            "WCAG 2.4.4 Understanding: Seitenzahl im Blätterkontext verständlich",
        )
        .variant("Ziffer"),
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
        .count(1),
        review(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target=" _BLANK ">Annual report</a>"#,
            "HTML: Browsing-Context-Schlüsselwörter ASCII-case-insensitive",
        )
        .variant("Schreibung"),
        negative(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_blank">Annual report (opens in new tab)</a>"#,
            "WCAG G201: Ankündigung im Linktext",
        ),
        negative(
            "links/new-window-unannounced",
            r#"<a href="/report.pdf" target="_blank" aria-label="Annual report, opens in new window">Annual report</a>"#,
            "WCAG G201: Ankündigung im zugänglichen Namen",
        )
        .variant("aria-label"),
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
        .count(1),
        negative(
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

fn required_names() -> Vec<Fall> {
    const ID: &str = "names/required-missing";
    vec![
        fail(
            ID,
            r#"<div role="checkbox" aria-checked="false" tabindex="0"></div>"#,
            "WAI-ARIA 1.2 checkbox: Accessible Name Required; WCAG 4.1.2, ACT e086e5",
        )
        .count(1),
        fail(
            ID,
            r#"<div role="slider" aria-valuenow="5" tabindex="0"></div>"#,
            "WAI-ARIA 1.2 slider: Accessible Name Required",
        )
        .variant("slider"),
        fail(
            ID,
            r#"<span role="link" tabindex="0"></span>"#,
            "WAI-ARIA 1.2 link: Name Required (ARIA-Rolle, nicht a[href])",
        )
        .variant("ARIA-Link"),
        fail(
            ID,
            r#"<span id="leer"></span><input type="checkbox" aria-labelledby="leer">"#,
            "ACT e086e5: behauptete Beschriftung, die leer ausgeht",
        )
        .variant("natives Feld, leere Beschriftung"),
        fail(
            ID,
            r#"<span role="img" class="hero"></span>"#,
            "ACT 23a2a8, WAI-ARIA 1.2 img: Accessible Name Required (1.1.1)",
        )
        .variant("role=img"),
        negative(
            ID,
            r#"<span role="img" aria-label="Sunset over the bay"></span>"#,
            "WCAG ARIA6: role=img mit aria-label",
        )
        .variant("role=img mit Namen"),
        na(
            ID,
            r#"<img src="a.png" role="img"><svg role="img"></svg>"#,
            "Regeldoku: img und SVG melden images/alt-missing bzw. svg/name-missing",
        )
        .variant("img und svg mit eigener Regel"),
        negative(
            ID,
            r#"<div role="checkbox" aria-checked="false" tabindex="0" aria-label="Accept terms"></div>"#,
            "WCAG ARIA14: aria-label",
        ),
        negative(
            ID,
            r#"<span id="v">Volume</span><div role="slider" aria-valuenow="5" tabindex="0" aria-labelledby="v"></div>"#,
            "WCAG ARIA16: aria-labelledby",
        )
        .variant("aria-labelledby"),
        negative(
            ID,
            r#"<label><input type="checkbox"> Accept terms</label>"#,
            "WCAG H44: label-Element",
        )
        .variant("label"),
        negative(
            ID,
            r#"<div role="radio" aria-checked="false" tabindex="0">Yes</div>"#,
            "WAI-ARIA 1.2 radio: Name aus dem Inhalt",
        )
        .variant("Inhalt"),
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
        .variant("abgegrenzt an forms/label-missing"),
        na(
            ID,
            r#"<div role="checkbox" aria-checked="false" hidden></div>"#,
            "WAI-ARIA 1.2: nur Elemente im Accessibility-Tree",
        )
        .variant("versteckt"),
        // --- names/symbol-only ---
        review(
            "names/symbol-only",
            r#"<button type="button" aria-label="×"></button>"#,
            "WCAG 4.1.2 Understanding / 2.4.6: Name aus einem Symbol ist nicht beschreibend",
        )
        .count(1),
        review(
            "names/symbol-only",
            r#"<a href="/menu" aria-label="☰"></a>"#,
            "WCAG 4.1.2: gilt für jede interaktive Rolle",
        )
        .variant("Link"),
        negative(
            "names/symbol-only",
            r#"<button type="button" aria-label="Close">×</button>"#,
            "WCAG ARIA14: beschreibender Name, sichtbares Symbol",
        ),
        negative(
            "names/symbol-only",
            r#"<button type="button" aria-label="Ä"></button>"#,
            "Unicode: Buchstaben sind keine Symbole",
        )
        .variant("Buchstabe"),
        negative(
            "names/symbol-only",
            r#"<button type="button" aria-label="××"></button>"#,
            "Grenze: zwei Zeichen sind kein Einzelsymbol",
        )
        .variant("Grenze: 2 Zeichen"),
        na(
            "names/symbol-only",
            r#"<p aria-label="×">Text</p>"#,
            "WAI-ARIA 1.2: keine interaktive Rolle",
        ),
        // --- names/description-duplicates-name ---
        review(
            "names/description-duplicates-name",
            r#"<button type="button" aria-label="Save" aria-describedby="d">x</button><span id="d">Save</span>"#,
            "accname 1.2; auditmysite-Korpus aria_names_missing: Beschreibung gleich Name ist überflüssig, 4.1.2 verlangt keinen Unterschied, also REVIEW",
        )
        .count(1),
        negative(
            "names/description-duplicates-name",
            r#"<button type="button" aria-label="Save" aria-describedby="d">x</button><span id="d">Saves the draft</span>"#,
            "accname 1.2: die Beschreibung ergänzt den Namen",
        )
        .variant("ergänzt"),
        na(
            "names/description-duplicates-name",
            r#"<article title="Story" aria-label="Story"><p>Text</p></article>"#,
            "auditmysite-Korpus aria_na_plain_article: keine interaktive Rolle",
        ),
    ]
}

fn dialogs() -> Vec<Fall> {
    vec![
        fail(
            "dialog/name-missing",
            r#"<div role="dialog" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2 dialog: Accessible Name Required; WCAG 4.1.2",
        )
        .count(1),
        fail(
            "dialog/name-missing",
            r#"<div role="alertdialog" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2 alertdialog: Accessible Name Required",
        )
        .variant("alertdialog"),
        fail(
            "dialog/name-missing",
            r#"<dialog open><p>Body</p></dialog>"#,
            "HTML-AAM: dialog hat Rolle dialog; Name Required",
        )
        .variant("natives dialog"),
        negative(
            "dialog/name-missing",
            r#"<div role="dialog" aria-modal="true" aria-labelledby="h"><h2 id="h">Settings</h2></div>"#,
            "WAI-ARIA APG: Dialog über seine Überschrift benannt (ARIA16)",
        ),
        negative(
            "dialog/name-missing",
            r#"<dialog open aria-label="Settings"><p>Body</p></dialog>"#,
            "WCAG ARIA6: aria-label",
        )
        .variant("natives dialog, aria-label"),
        na(
            "dialog/name-missing",
            r#"<dialog><p>Body</p></dialog><div role="dialog" hidden></div>"#,
            "HTML: geschlossener dialog ist nicht dargestellt; versteckt nicht im Baum",
        )
        .variant("geschlossen/versteckt"),
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
        .count(1),
        negative(
            "dialog/modal-unmarked",
            r#"<div role="dialog" aria-label="Settings" aria-modal="true"><p>Body</p></div>"#,
            "WAI-ARIA 1.2: aria-modal=true",
        ),
        na(
            "dialog/modal-unmarked",
            r#"<dialog open aria-label="Settings"><p>Body</p></dialog>"#,
            "natives dialog: Modalität über showModal(), nicht im Markup (Regeldoku)",
        )
        .variant("natives dialog"),
        na(
            "dialog/modal-unmarked",
            r#"<div><p>Body</p></div>"#,
            "keine Dialogrolle",
        ),
    ]
}

fn summary_and_live() -> Vec<Fall> {
    vec![
        fail(
            "summary/name-missing",
            r#"<details><summary></summary><p>Hidden</p></details>"#,
            "HTML-AAM summary; WCAG 4.1.2: Umschalter ohne Namen",
        )
        .count(1),
        negative(
            "summary/name-missing",
            r#"<details><summary>Shipping details</summary><p>Hidden</p></details>"#,
            "accname 1.2: Name aus dem Inhalt",
        ),
        negative(
            "summary/name-missing",
            r#"<details><summary aria-label="Shipping details"></summary><p>Hidden</p></details>"#,
            "WCAG ARIA14: aria-label",
        )
        .variant("aria-label"),
        negative(
            "summary/name-missing",
            r#"<details open><summary>More</summary><summary></summary></details>"#,
            "HTML: nur die erste summary bedient das details-Element",
        )
        .variant("zweite summary"),
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
        .count(1),
        fail(
            "status/live-overridden",
            r#"<output aria-live="off">3 results</output>"#,
            "HTML-AAM: output hat Rolle status; WCAG 4.1.3",
        )
        .variant("natives output"),
        review(
            "status/live-overridden",
            r#"<div role="alert" aria-live="polite">Error</div>"#,
            "WAI-ARIA 1.2: Dringlichkeit darf überschrieben werden; Regeldoku → REVIEW",
        )
        .variant("andere Dringlichkeit"),
        negative(
            "status/live-overridden",
            r#"<div role="status" aria-live="polite">Saved</div>"#,
            "WAI-ARIA 1.2: Vorgabe von status ist polite",
        ),
        negative(
            "status/live-overridden",
            r#"<div role="log" aria-live="assertive">Chat</div>"#,
            "Regeldoku: Dringlichkeit von log umzustellen wird nicht bemängelt",
        )
        .variant("log"),
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
        .count(1),
        fail(
            ID,
            r#"<a href="/x" aria-label="Open">Settings</a>"#,
            "Grenze: sichtbarer Text genau doppelt so lang wie der Name → noch Verstoß (Regeldoku, auditmysite#513)",
        )
        .variant("Grenze: 8 zu 4 Zeichen"),
        review(
            ID,
            r#"<a href="/x" aria-label="Open">Settings 2</a>"#,
            "Grenze: mehr als doppelt so lang → zusammengesetztes Widget, REVIEW (auditmysite#513)",
        )
        .variant("Grenze: 9 zu 4 Zeichen"),
        review(
            ID,
            r#"<a href="/x" aria-label="annual report 2025">Report annual</a>"#,
            "WCAG 2.5.3 Understanding: alle Wörter im Namen, andere Reihenfolge → REVIEW",
        )
        .variant("umgestellt"),
        negative(
            ID,
            r#"<button type="button" aria-label="Send message">Send</button>"#,
            "WCAG 2.5.3, G208: sichtbarer Text am Anfang des Namens",
        ),
        negative(
            ID,
            r#"<a href="/next" aria-label="Next page">→</a>"#,
            "WCAG 2.5.3 Understanding: Symbole sind ausgenommen",
        )
        .variant("Symbol"),
        negative(
            ID,
            r#"<button type="button" aria-label="Dismiss">x</button>"#,
            "ACT 2ee8b8: ein einzelnes Zeichen drückt Nicht-Text aus",
        )
        .variant("ein Buchstabe als Symbol"),
        negative(
            ID,
            r#"<button type="button">Send</button>"#,
            "ACT 2ee8b8: Name aus dem Inhalt enthält ihn",
        )
        .variant("Name aus Inhalt"),
        negative(
            ID,
            r#"<button aria-label="Find" style="font-family: 'Material Icons'">search</button>"#,
            "ACT 2ee8b8 (Passed, 65be00): Ligatur in einer Symbolschrift zeichnet eine Lupe, keinen Text",
        )
        .variant("Ligatur-Symbolschrift"),
        fail(
            ID,
            r#"<button aria-label="Find" style="font-family: 'Material Icons'">search</button>"#,
            "Ohne berechnete Stile ist die Symbolschrift unbekannt: der Text zählt",
        )
        .variant("Ligatur ohne Stile")
        .mode(Mode::Semantics),
        negative(
            ID,
            r#"<button aria-label="Search all"><span style="font-family: 'Material Symbols Outlined'">search</span> Search all</button>"#,
            "ACT 2ee8b8: das Symbol fällt weg, der übrige sichtbare Text steht im Namen",
        )
        .variant("Symbol plus Text"),
        na(
            ID,
            r#"<p aria-label="Other">Text</p>"#,
            "ACT 2ee8b8: nur Rollen mit Name aus Inhalt",
        ),
    ]
}

fn media() -> Vec<Fall> {
    vec![
        // --- images/area-alt-missing ---
        fail(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG 1.1.1, H24; ACT c487ae: aktiver area-Link ohne Textalternative",
        )
        .count(1),
        fail(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt=" " shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: leeres alt am Link-Bereich ergibt keinen Namen",
        )
        .variant("alt leer"),
        negative(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: alt am area",
        ),
        negative(
            "images/area-alt-missing",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" aria-label="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "HTML-AAM area: aria-label vor alt",
        )
        .variant("aria-label"),
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
        .count(1),
        fail(
            "images/input-alt-missing",
            r#"<input type="IMAGE" src="go.png" alt="">"#,
            "H36: leeres alt; Typ ASCII-case-insensitive",
        )
        .variant("alt leer, Schreibung"),
        fail(
            "images/input-alt-missing",
            r#"<span id="leer"></span><input type="image" src="go.png" aria-labelledby="leer">"#,
            "ACT 59796f, accname 1.2 2B: Verweis auf ein leeres Element stiftet keinen Namen",
        )
        .variant("aria-labelledby auf leeres Ziel"),
        review(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" aria-labelledby="fehlt">"#,
            "Regeldoku: Ziel fehlt in der Sicht (versteckt oder nicht vorhanden) → REVIEW",
        )
        .variant("aria-labelledby ins Leere"),
        negative(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" title="Search">"#,
            "ACT 59796f, HTML-AAM input type=image: title nach alt",
        )
        .variant("title"),
        negative(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" alt="Search">"#,
            "WCAG H36: alt beschreibt die Aktion",
        ),
        negative(
            "images/input-alt-missing",
            r#"<input type="image" src="go.png" aria-label="Search">"#,
            "HTML-AAM input type=image: aria-label vor alt",
        )
        .variant("aria-label"),
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
        .count(1),
        negative(
            "images/server-side-map",
            r##"<img src="map.png" alt="Map" usemap="#m"><map name="m"><area href="/a" alt="Berlin" shape="rect" coords="0,0,10,10"></map>"##,
            "WCAG H24: clientseitige Imagemap ist per Tastatur bedienbar",
        ),
        na(
            "images/server-side-map",
            r#"<img src="map.png" alt="Map" ismap>"#,
            "HTML (img, ismap): wirkt nur am Nachfahren eines a mit href — sonst keine serverseitige Imagemap",
        )
        .variant("Regression: ismap ohne Link"),
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
        .count(1),
        negative(
            "objects/alt-missing",
            r#"<object data="report.pdf" type="application/pdf">Annual report (PDF)</object>"#,
            "WCAG H53: Ersatzinhalt im object",
        ),
        negative(
            "objects/alt-missing",
            r#"<object data="chart.svg" type="image/svg+xml" aria-label="Sales chart"></object>"#,
            "ACT 8fc3b6: aria-label",
        )
        .variant("aria-label"),
        negative(
            "objects/alt-missing",
            r#"<object data="chart.svg" type="image/svg+xml" title="Sales chart"></object>"#,
            "HTML-AAM object: title",
        )
        .variant("title"),
        fail(
            "objects/alt-missing",
            r#"<p id="leer"></p><object data="chart.png" type="image/png" aria-labelledby="leer"></object>"#,
            "ACT 8fc3b6, accname 1.2 2B: Verweis auf ein leeres Element stiftet keinen Namen",
        )
        .variant("aria-labelledby auf leeres Ziel"),
        review(
            "objects/alt-missing",
            r#"<p id="v" hidden>Sales chart</p><object data="chart.png" type="image/png" aria-labelledby="v"></object>"#,
            "accname 1.2 2B: verstecktes Ziel zählt; Tier 1 sieht es nicht → REVIEW, kein FAIL",
        )
        .variant("aria-labelledby auf verstecktes Ziel"),
        na(
            "objects/alt-missing",
            r#"<object data="deko.png" type="image/png" role="none"></object>"#,
            "ACT 8fc3b6: role=none kennzeichnet Schmuck (WAI-ARIA 1.2 § 9.3 ohne Konflikt)",
        )
        .variant("role=none"),
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
        .count(1),
        review(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay hidden></audio><div aria-hidden="true"><audio src="b.mp3" autoplay controls></audio></div>"#,
            "WCAG 1.4.2: Ton spielt unabhängig von Sichtbarkeit und Accessibility-Tree",
        )
        .variant("Regression: versteckt (hidden, aria-hidden)")
        .count(2),
        review(
            "media/audio-autoplay",
            r#"<video autoplay><source src="v.mp4" type="video/mp4"></video>"#,
            "WCAG 1.4.2, ACT 80f0bf: auch ein Video spielt seinen Ton von selbst; ob es Ton hat, steht nicht im Markup",
        )
        .variant("Regression: video")
        .count(1),
        negative(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay controls></audio>"#,
            "WCAG 1.4.2, ACT 80f0bf/4c31df: controls gibt den Mechanismus zum Anhalten und Stummschalten",
        )
        .variant("controls"),
        negative(
            "media/audio-autoplay",
            r#"<video autoplay><source src="v.mp4#t=4,6" type="video/mp4"><source src="v.webm#t=npt:4,6" type="video/webm"></video>"#,
            "WCAG 1.4.2 (drei Sekunden), Media Fragments URI 1.0: das Fragment spielt zwei Sekunden",
        )
        .variant("Fragment unter drei Sekunden"),
        review(
            "media/audio-autoplay",
            r#"<video autoplay><source src="v.mp4#t=4,6" type="video/mp4"><source src="v.webm" type="video/webm"></video>"#,
            "Media Fragments: eine Quelle ohne Fragment spielt ganz",
        )
        .variant("Grenze: nur eine Quelle mit Fragment"),
        negative(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay muted></audio>"#,
            "WCAG 1.4.2: stummgeschaltet startet kein Ton",
        ),
        na(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" controls></audio>"#,
            "WCAG 1.4.2: ohne autoplay startet nichts von selbst",
        ),
        negative(
            "media/audio-autoplay",
            r#"<audio src="a.mp3" autoplay style="--media-paused:0;--media-duration:12;--media-audio:1"></audio>"#,
            "Gemessen: die Messung entscheidet in media/audio-autoplay-measured, die Vermutung aus dem Markup entfällt",
        )
        .variant("gemessen"),
        // --- media/video-captions-missing ---
        review(
            "media/video-captions-missing",
            r#"<video controls src="t.mp4"></video><video controls src="r.mp4"><track kind="descriptions" src="r.vtt" srclang="en"></video>"#,
            "WCAG 1.2.2; auditmysite-Korpus content_video_uncaptioned: ohne Untertitelspur REVIEW (eingebrannte Untertitel stehen nicht im Markup); eine Beschreibungsspur ersetzt sie nicht",
        )
        .count(2),
        negative(
            "media/video-captions-missing",
            r#"<video controls src="t.mp4"><track kind="captions" src="t.vtt" srclang="en"></video><video controls src="w.mp4"><track kind="subtitles" src="w.vtt" srclang="de"></video>"#,
            "WCAG 1.2.2; auditmysite-Korpus content_video_captioned: captions und subtitles bestehen",
        )
        .variant("captions und subtitles"),
        negative(
            "media/video-captions-missing",
            r#"<video autoplay muted loop src="bg.mp4"></video>"#,
            "WCAG 1.2.2: stumm und ohne controls, niemand schaltet den Ton ein",
        )
        .variant("muted ohne controls"),
        na(
            "media/video-captions-missing",
            r#"<div hidden><video controls src="a.mp4"></video></div>"#,
            "auditmysite-Korpus content_video_hidden: versteckt, nicht im Accessibility-Tree",
        ),
        // --- media/audio-autoplay-measured ---
        fail(
            "media/audio-autoplay-measured",
            r#"<audio src="moon-speech.mp3" autoplay style="--media-paused:0;--media-muted:0;--media-duration:12;--media-audio:1"></audio>"#,
            "ACT 80f0bf Failed 1: spielt mit Ton länger als drei Sekunden, ohne Bedienelemente",
        )
        .count(1),
        fail(
            "media/audio-autoplay-measured",
            r#"<video autoplay style="--media-paused:0;--media-duration:16;--media-audio:1"><source src="video.mp4" type="video/mp4"></video>"#,
            "ACT 80f0bf Failed 2: Video mit Tonspur",
        )
        .variant("video"),
        fail(
            "media/audio-autoplay-measured",
            r#"<audio src="radio" autoplay style="--media-paused:0;--media-duration:inf;--media-audio:1"></audio>"#,
            "WCAG 1.4.2: ein Datenstrom endet nie",
        )
        .variant("Strom"),
        negative(
            "media/audio-autoplay-measured",
            r#"<audio src="moon-speech.mp3" autoplay controls style="--media-paused:0;--media-duration:12;--media-audio:1"></audio>"#,
            "ACT 80f0bf Passed 1 / 4c31df: controls zum Anhalten",
        )
        .variant("controls"),
        negative(
            "media/audio-autoplay-measured",
            r#"<video autoplay style="--media-paused:1;--media-duration:16;--media-audio:1"><source src="video.mp4" type="video/mp4"></video>"#,
            "WCAG 1.4.2: ein Skript hat das Video angehalten",
        )
        .variant("angehalten"),
        review(
            "media/audio-autoplay-measured",
            r#"<video id="video" autoplay style="--media-paused:0;--media-duration:16;--media-audio:1"><source src="video.mp4" type="video/mp4"></video><div id="video-controls"><button type="button" id="play-pause">Play</button><button type="button" id="mute">Mute</button></div>"#,
            "ACT 80f0bf Passed 3: eigene Knöpfe mit Skript; ob sie wirken, steht nicht in der Messung → REVIEW",
        )
        .variant("eigene Knöpfe"),
        review(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" autoplay style="--media-paused:0;--media-duration:12;--media-audio:1"></audio><div role="button" aria-label="Ton aus" tabindex="0"></div>"#,
            "Eigener Knopf über role=button und aria-label (deutsch) → REVIEW",
        )
        .variant("eigener Knopf, Rolle"),
        fail(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" autoplay style="--media-paused:0;--media-duration:12;--media-audio:1"></audio><button>Submit</button><button>Play</button>"#,
            "Knöpfe ohne Anhalten oder Stummschalten zählen nicht („button“ ist kein „Ton“)",
        )
        .variant("fremde Knöpfe"),
        negative(
            "media/audio-autoplay-measured",
            r#"<video autoplay style="--media-paused:0;--media-duration:16;--media-audio:0"><source src="silent.mp4" type="video/mp4"></video>"#,
            "ACT 80f0bf Inapplicable 2: keine Tonspur",
        )
        .variant("ohne Ton"),
        negative(
            "media/audio-autoplay-measured",
            r#"<audio src="beep.mp3" autoplay style="--media-paused:0;--media-duration:3;--media-audio:1"></audio>"#,
            "WCAG 1.4.2: höchstens drei Sekunden",
        )
        .variant("Grenze: 3 s"),
        negative(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" autoplay style="--media-paused:0;--media-muted:1;--media-duration:12;--media-audio:1"></audio>"#,
            "ACT 80f0bf: stumm",
        )
        .variant("stumm"),
        review(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" autoplay style="--media-paused:0;--media-duration:12"></audio>"#,
            "Tonspur nicht bestimmbar → REVIEW",
        )
        .variant("Tonspur unbekannt"),
        not_run(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" autoplay></audio>"#,
            "Host misst die Wiedergabe nicht → nicht gelaufen",
        ),
        na(
            "media/audio-autoplay-measured",
            r#"<audio src="a.mp3" controls></audio>"#,
            "WCAG 1.4.2: ohne autoplay startet nichts von selbst",
        ),
        // --- frames/name-missing ---
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget"></iframe>"#,
            "WCAG 4.1.2; ACT cae760: iframe ohne Namen",
        )
        .count(1),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" title=" "></iframe>"#,
            "ACT cae760: leerer title ist kein Name",
        )
        .variant("title leer"),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" width="2" height="2"></iframe>"#,
            "Grenze: 2×2 ist dargestellt (Ausnahme nur bis 1×1)",
        )
        .variant("Grenze: 2×2"),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" role="presentation"></iframe>"#,
            "ACT cae760, WAI-ARIA 1.2 § 9.3: fokussierbarer Rahmen behält seine Rolle",
        )
        .variant("role=presentation"),
        fail(
            "frames/name-missing",
            r#"<iframe src="/widget" tabindex="0"></iframe>"#,
            "Grenze: tabindex=0 bleibt in der Tab-Reihenfolge",
        )
        .variant("Grenze: tabindex=0"),
        na(
            "frames/name-missing",
            r#"<iframe src="/widget" tabindex="-1"></iframe>"#,
            "ACT cae760: nur Rahmen in der sequenziellen Fokusreihenfolge",
        )
        .variant("tabindex=-1"),
        negative(
            "frames/name-missing",
            r#"<iframe src="/widget" title="Weather forecast"></iframe>"#,
            "WCAG H64: title",
        ),
        negative(
            "frames/name-missing",
            r#"<iframe src="/widget" aria-label="Weather forecast"></iframe>"#,
            "ACT cae760: aria-label",
        )
        .variant("aria-label"),
        negative(
            "frames/name-missing",
            r#"<h2 id="w">Weather</h2><iframe src="/widget" aria-labelledby="w"></iframe>"#,
            "ACT cae760: aria-labelledby",
        )
        .variant("aria-labelledby"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" width="1" height="1"></iframe><iframe src="/t" width="0" height="0"></iframe>"#,
            "Regeldoku: technische Rahmen bis 1×1 (Grenze) sind nie zu sehen",
        )
        .variant("Grenze: 1×1"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" hidden></iframe><iframe src="/t" style="display:none"></iframe>"#,
            "ACT cae760: nur Rahmen im Accessibility-Tree",
        )
        .variant("versteckt"),
        na(
            "frames/name-missing",
            r#"<iframe src="/s" style="display: none !important"></iframe>"#,
            "ohne Stile: Inline display:none erkennt die Regel selbst",
        )
        .variant("ohne Stile")
        .mode(Mode::Semantics),
    ]
}
