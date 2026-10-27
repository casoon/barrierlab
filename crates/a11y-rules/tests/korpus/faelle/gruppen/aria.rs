//! Gruppe „aria“: ARIA-Attribute, Widgets, Popover, inert, Kontext und
//! Bestandteile (src/aria.rs) sowie überflüssige Rollen, title als einziger
//! Name und Tooltips (src/rollen.rs).

use super::super::{Fall, fail, na, negativ, review};

const ARIA12_ATTR: &str = "WAI-ARIA 1.2 §6.6 Definitions of States and Properties; ACT 5f99a7";
const ARIA12_WERT: &str = "WAI-ARIA 1.2 §6.6/§6.7 Werttypen; ACT 6a7281";
const ARIA12_OWNS: &str = "WAI-ARIA 1.2 aria-owns: an element's ID MUST NOT be specified in more than one other element's aria-owns";
const ARIA12_TAB: &str =
    "WAI-ARIA 1.2 tab/tablist; Entscheidung REVIEW (docs/packages/a11y-rules.md, Widgets)";
const ARIA12_COMBO: &str =
    "WAI-ARIA 1.2 combobox: Authors MUST set aria-controls ... to the combobox popup";
const HTML_POPOVER: &str =
    "HTML Living Standard, popover target attributes (popovertarget → Element mit popover)";
const HTML_INERT: &str = "HTML Living Standard, inert attribute; WCAG 2.1.1, 4.1.2; Entscheidung FAIL nur für <dialog open> (docs/packages/a11y-rules.md)";
const ARIA12_ERLAUBT: &str =
    "WAI-ARIA 1.2 Supported States and Properties je Rolle; ARIA in HTML §3.2; ACT 5c01ea";
const ARIA12_VERBOTEN: &str =
    "WAI-ARIA 1.2 Prohibited States and Properties (Name From: prohibited); ACT 5c01ea";
const ARIA12_KONTEXT: &str = "WAI-ARIA 1.2 Required Context Role; ACT ff89c9";
const ARIA12_BESTAND: &str = "WAI-ARIA 1.2 Required Owned Elements; ACT bc4a75 (leerer Behälter, aria-busy: kein Befund, docs/packages/a11y-rules.md)";
const ARIA_IN_HTML_ROLLE: &str =
    "ARIA in HTML §2 (redundante Rollen: SHOULD NOT); Best Practice, auditmysite#779";
const TITLE_ONLY: &str = "WCAG 4.1.2, H65 (title ist gültige Namensquelle, deshalb REVIEW); accname 1.2 §4.3.2 Schritt 2I";
const TOOLTIP: &str = "WAI-ARIA 1.2 tooltip (referenced by aria-describedby); APG Tooltip Pattern; Best Practice, auditmysite#779";

pub fn faelle() -> Vec<Fall> {
    vec![
        // --- aria/attribute-unknown -----------------------------------------
        fail(
            "aria/attribute-unknown",
            r#"<div aria-labeledby="a">Tippfehler</div><p id="a">A</p>"#,
            ARIA12_ATTR,
        )
        .variante("Tippfehler aria-labeledby")
        .anzahl(1),
        fail(
            "aria/attribute-unknown",
            r#"<button aria-nonexistentattr="true">x</button>"#,
            ARIA12_ATTR,
        ),
        negativ(
            "aria/attribute-unknown",
            r#"<div aria-labelledby="a">Richtig geschrieben</div><p id="a">A</p>
               <button aria-pressed="true">ok</button>"#,
            ARIA12_ATTR,
        ),
        negativ(
            "aria/attribute-unknown",
            r#"<div aria-description="Beschreibung">ok</div>"#,
            "WAI-ARIA 1.3 (Entwurf) aria-description, in Browsern umgesetzt; kein Tippfehler",
        )
        .variante("ARIA-1.3-Attribut"),
        negativ(
            "aria/attribute-unknown",
            r#"<div ARIA-LABEL="Groß">x</div><div data-aria-foo="1">y</div>"#,
            "HTML: Attributnamen ASCII-case-insensitive; data-* ist kein ARIA-Attribut",
        )
        .variante("Großschreibung, data-Präfix"),
        na(
            "aria/attribute-unknown",
            r#"<p class="x" id="y">Ohne ARIA</p>"#,
            ARIA12_ATTR,
        ),
        // --- aria/attribute-value-invalid -----------------------------------
        fail(
            "aria/attribute-value-invalid",
            r#"<button aria-expanded="maybe" aria-controls="p">Toggle</button><div id="p">P</div>"#,
            ARIA12_WERT,
        )
        .variante("true/false/undefined")
        .anzahl(1),
        fail(
            "aria/attribute-value-invalid",
            r#"<div role="heading" aria-level="2.5">Titel</div>"#,
            ARIA12_WERT,
        )
        .variante("Ganzzahl mit Bruchteil"),
        fail(
            "aria/attribute-value-invalid",
            r#"<div role="slider" tabindex="0" aria-label="L" aria-valuenow="viel" aria-valuemin="0" aria-valuemax="10">s</div>"#,
            ARIA12_WERT,
        )
        .variante("Zahl"),
        fail(
            "aria/attribute-value-invalid",
            r#"<div aria-live="polite" aria-relevant="additions everything">x</div>"#,
            ARIA12_WERT,
        )
        .variante("Tokenliste mit fremdem Token"),
        fail(
            "aria/attribute-value-invalid",
            r#"<div role="checkbox" tabindex="0" aria-checked="yes">c</div>"#,
            ARIA12_WERT,
        )
        .variante("Tristate"),
        negativ(
            "aria/attribute-value-invalid",
            r#"<button aria-expanded="true" aria-haspopup="menu">a</button>
               <div role="checkbox" tabindex="0" aria-checked="mixed">b</div>
               <div role="slider" tabindex="0" aria-label="L" aria-valuenow="1e2" aria-valuemin="-5" aria-valuemax="200.5">c</div>
               <div aria-live="polite" aria-relevant="additions text">d</div>
               <div role="heading" aria-level="2">e</div>"#,
            ARIA12_WERT,
        )
        .variante("gültige Werte aller Typen"),
        negativ(
            "aria/attribute-value-invalid",
            r#"<button aria-pressed="">f</button><a href="/" aria-current="yes">g</a>
               <input aria-invalid="bogus" aria-label="h">"#,
            "WAI-ARIA 1.2 §6.2: leerer Wert wie fehlend; aria-current/aria-invalid: unbekannter Wert gilt als true",
        )
        .variante("leer, aria-current/aria-invalid"),
        na(
            "aria/attribute-value-invalid",
            r#"<div aria-label="Name">x</div>"#,
            "Freie Zeichenkette ohne Wertebereich (WAI-ARIA 1.2 §6.7 string)",
        ),
        // --- aria/owns-conflict ---------------------------------------------
        fail(
            "aria/owns-conflict",
            r#"<div role="combobox" aria-expanded="false" aria-owns="liste" aria-label="A">a</div>
               <div role="combobox" aria-expanded="false" aria-owns="liste" aria-label="B">b</div>
               <div role="listbox" id="liste"><div role="option">o</div></div>"#,
            ARIA12_OWNS,
        )
        .anzahl(1),
        negativ(
            "aria/owns-conflict",
            r#"<div role="group" aria-owns="x y x">c</div><div id="x">x</div><div id="y">y</div>"#,
            ARIA12_OWNS,
        )
        .variante("dieselbe ID zweimal am selben Element"),
        negativ(
            "aria/owns-conflict",
            r#"<div role="group" aria-owns="x">a</div><div role="group" aria-owns="y">b</div>
               <div id="x">x</div><div id="y">y</div>"#,
            ARIA12_OWNS,
        )
        .variante("verschiedene IDs"),
        negativ(
            "aria/owns-conflict",
            r#"<div role="group" aria-owns="x">a</div><div id="x">x</div>
               <div><template shadowrootmode="open"><div role="group" aria-owns="x">b</div><div id="x">y</div></template></div>"#,
            "DOM: IDs gelten je Baumbereich (Shadow Root); WAI-ARIA 1.2 aria-owns",
        )
        .variante("gleiche ID in Shadow Root"),
        na(
            "aria/owns-conflict",
            r#"<div role="group">ohne aria-owns</div>"#,
            ARIA12_OWNS,
        ),
        // --- aria/tab-selected-missing --------------------------------------
        review(
            "aria/tab-selected-missing",
            r#"<div role="tablist" aria-label="T">
                 <div role="tab" tabindex="0" aria-controls="p">A</div>
                 <div role="tab" aria-selected="false">B</div>
               </div><div role="tabpanel" id="p">P</div>"#,
            ARIA12_TAB,
        )
        .anzahl(1),
        negativ(
            "aria/tab-selected-missing",
            r#"<div role="tablist" aria-label="T">
                 <div role="tab" tabindex="0" aria-selected="true">A</div>
                 <div role="tab">B</div>
               </div><div role="tabpanel">P</div>"#,
            ARIA12_TAB,
        )
        .variante("ein Tab gewählt, die übrigen mit Vorgabe false"),
        negativ(
            "aria/tab-selected-missing",
            r#"<div role="tablist" aria-label="T" aria-multiselectable="true">
                 <div role="tab" tabindex="0">A</div><div role="tab">B</div>
               </div><div role="tabpanel">P</div>"#,
            "WAI-ARIA 1.2 tablist aria-multiselectable: keine Einzelauswahl vorgeschrieben",
        )
        .variante("aria-multiselectable"),
        na(
            "aria/tab-selected-missing",
            r#"<button>Kein Tab</button>"#,
            ARIA12_TAB,
        ),
        // --- aria/tabpanel-missing ------------------------------------------
        review(
            "aria/tabpanel-missing",
            r#"<div role="tablist" aria-label="Navigation">
                 <a role="tab" href="/a" aria-selected="true">A</a>
               </div>"#,
            ARIA12_TAB,
        )
        .anzahl(1),
        negativ(
            "aria/tabpanel-missing",
            r#"<div role="tablist" aria-label="T">
                 <div role="tab" tabindex="0" aria-selected="true" aria-controls="p">A</div>
               </div><section role="tabpanel" id="p">Inhalt</section>"#,
            ARIA12_TAB,
        ),
        na(
            "aria/tabpanel-missing",
            r#"<section role="tabpanel">Panel ohne Tabliste</section>"#,
            ARIA12_TAB,
        ),
        // --- aria/combobox-popup-missing ------------------------------------
        fail(
            "aria/combobox-popup-missing",
            r#"<div role="combobox" aria-expanded="true" aria-label="Frucht" tabindex="0"></div>"#,
            ARIA12_COMBO,
        )
        .anzahl(1),
        fail(
            "aria/combobox-popup-missing",
            r#"<input role="combobox" aria-expanded="true" aria-controls="  " aria-label="Suche">"#,
            ARIA12_COMBO,
        )
        .variante("aria-controls nur Leerraum"),
        negativ(
            "aria/combobox-popup-missing",
            r#"<input role="combobox" aria-expanded="true" aria-controls="lb" aria-label="Suche">
               <div role="listbox" id="lb"><div role="option">o</div></div>"#,
            ARIA12_COMBO,
        )
        .variante("aria-controls"),
        negativ(
            "aria/combobox-popup-missing",
            r#"<div role="combobox" aria-expanded="true" aria-label="alt"><input aria-label="x"><div role="listbox"><div role="option">o</div></div></div>"#,
            "WAI-ARIA 1.1 combobox: Popup im eigenen Teilbaum",
        )
        .variante("ARIA-1.1-Muster, Popup enthalten"),
        negativ(
            "aria/combobox-popup-missing",
            r#"<div role="combobox" aria-expanded="true" aria-owns="lb" aria-label="o"></div>
               <div role="listbox" id="lb"><div role="option">o</div></div>"#,
            "WAI-ARIA 1.1 combobox: Popup per aria-owns",
        )
        .variante("aria-owns"),
        negativ(
            "aria/combobox-popup-missing",
            r#"<input role="combobox" aria-expanded="false" aria-label="zu">"#,
            "Mehrdeutig: WAI-ARIA 1.2 führt aria-controls als erforderlich, APG und ARIA-1.3-Entwurf verlangen den Verweis erst, wenn das Popup angezeigt wird (eingeklappt existiert es oft nicht). Regel folgt Letzterem.",
        )
        .variante("eingeklappt"),
        na(
            "aria/combobox-popup-missing",
            r#"<input type="text" aria-label="Kein Combobox">"#,
            ARIA12_COMBO,
        ),
        // --- popover/target-missing -----------------------------------------
        fail(
            "popover/target-missing",
            r#"<button popovertarget="weg">Öffnen</button>"#,
            HTML_POPOVER,
        )
        .anzahl(1),
        fail(
            "popover/target-missing",
            r#"<button popovertarget="p">Öffnen</button>
               <div><template shadowrootmode="open"><div id="p" popover>P</div></template></div>"#,
            "HTML: popovertarget ist eine IDREF im selben Baum; DOM: Shadow Root ist eigener Bereich",
        )
        .variante("Ziel nur in fremder Shadow Root"),
        negativ(
            "popover/target-missing",
            r#"<button popovertarget="p">Öffnen</button><div id="p" popover>Inhalt</div>"#,
            HTML_POPOVER,
        ),
        na(
            "popover/target-missing",
            r#"<button>Ohne Popover</button><div id="p" popover>Inhalt</div>"#,
            HTML_POPOVER,
        ),
        // --- popover/target-invalid -----------------------------------------
        fail(
            "popover/target-invalid",
            r#"<button popovertarget="p">Öffnen</button><div id="p">Kein Popover</div>"#,
            HTML_POPOVER,
        )
        .anzahl(1),
        negativ(
            "popover/target-invalid",
            r#"<button popovertarget="p">Öffnen</button><div id="p" popover="manual">Inhalt</div>"#,
            HTML_POPOVER,
        )
        .variante("popover=manual"),
        na(
            "popover/target-invalid",
            r#"<div id="p">Ohne Auslöser</div>"#,
            HTML_POPOVER,
        ),
        // --- inert/dialog-inert ---------------------------------------------
        fail(
            "inert/dialog-inert",
            r#"<dialog open inert aria-label="D"><button>OK</button></dialog>"#,
            HTML_INERT,
        )
        .anzahl(1),
        fail(
            "inert/dialog-inert",
            r#"<div inert><dialog open aria-label="D"><button>OK</button></dialog></div>"#,
            HTML_INERT,
        )
        .variante("inert am Vorfahren"),
        review(
            "inert/dialog-inert",
            r#"<div role="dialog" aria-label="D" inert><button>OK</button></div>"#,
            HTML_INERT,
        )
        .variante("ARIA-Rolle: Offenheit nicht im Markup → REVIEW"),
        negativ(
            "inert/dialog-inert",
            r#"<dialog open aria-label="D"><button>OK</button></dialog><main inert><p>Hintergrund</p></main>"#,
            "HTML inert: der Hintergrund eines modalen Dialogs ist richtig inert",
        ),
        negativ(
            "inert/dialog-inert",
            r#"<div role="dialog" aria-label="D" inert hidden><button>OK</button></div>"#,
            "HTML hidden: nicht dargestellt, kein sichtbarer, unbedienbarer Dialog",
        )
        .variante("versteckt per hidden"),
        na(
            "inert/dialog-inert",
            r#"<div inert><p>Kein Dialog</p></div>"#,
            HTML_INERT,
        ),
        // --- aria/attribute-not-allowed -------------------------------------
        fail(
            "aria/attribute-not-allowed",
            r#"<div role="img" aria-label="Icon" aria-checked="true">I</div>"#,
            ARIA12_ERLAUBT,
        )
        .variante("ARIA-Rolle")
        .anzahl(1),
        fail(
            "aria/attribute-not-allowed",
            r#"<div aria-expanded="false">Akkordeon aus div</div>"#,
            ARIA12_ERLAUBT,
        )
        .variante("implizite Rolle generic"),
        fail(
            "aria/attribute-not-allowed",
            r#"<input type="checkbox" aria-checked="true" aria-label="a">"#,
            "ARIA in HTML §3.2: Authors MUST NOT use aria-checked on input type=checkbox",
        )
        .variante("nativ: aria-checked an Checkbox"),
        fail(
            "aria/attribute-not-allowed",
            r#"<button disabled aria-disabled="false">d</button>"#,
            "ARIA in HTML §3.2: aria-disabled=false widerspricht disabled",
        )
        .variante("nativ: aria-disabled=false bei disabled"),
        negativ(
            "aria/attribute-not-allowed",
            r#"<div role="checkbox" tabindex="0" aria-checked="true" aria-label="c">c</div>
               <button aria-expanded="false">Akkordeon</button>
               <button disabled aria-disabled="true">e</button>"#,
            ARIA12_ERLAUBT,
        ),
        negativ(
            "aria/attribute-not-allowed",
            r#"<div role="img" aria-label="Icon" aria-checked="">I</div>"#,
            "WAI-ARIA 1.2 §6.2: leerer Wert gilt wie ein fehlendes Attribut",
        )
        .variante("leerer Wert"),
        na(
            "aria/attribute-not-allowed",
            r#"<div aria-hidden="true"><div role="img" aria-checked="true">I</div></div>"#,
            "ACT 5c01ea: gilt nur für Elemente im Accessibility-Tree",
        )
        .variante("unter aria-hidden"),
        na(
            "aria/attribute-not-allowed",
            r#"<p>Ohne ARIA</p>"#,
            ARIA12_ERLAUBT,
        ),
        // --- aria/attribute-prohibited --------------------------------------
        fail(
            "aria/attribute-prohibited",
            r#"<div aria-label="Kein Name erlaubt">x</div>"#,
            ARIA12_VERBOTEN,
        )
        .variante("implizit generic")
        .anzahl(1),
        fail(
            "aria/attribute-prohibited",
            r#"<span role="presentation" aria-label="Redundant">Deko</span>"#,
            ARIA12_VERBOTEN,
        )
        .variante("role=presentation"),
        fail(
            "aria/attribute-prohibited",
            r#"<p aria-labelledby="x">Absatz</p><span id="x">X</span>"#,
            ARIA12_VERBOTEN,
        )
        .variante("paragraph, aria-labelledby"),
        negativ(
            "aria/attribute-prohibited",
            r#"<div role="region" aria-label="Bereich">x</div><nav aria-label="Haupt"><a href="/">A</a></nav>"#,
            "WAI-ARIA 1.2 region/navigation: Name from author erlaubt",
        ),
        na(
            "aria/attribute-prohibited",
            r#"<div class="x">kein ARIA</div>"#,
            ARIA12_VERBOTEN,
        ),
        // --- aria/required-parent-missing -----------------------------------
        fail(
            "aria/required-parent-missing",
            r#"<div role="tab" tabindex="0">Verwaist</div>"#,
            ARIA12_KONTEXT,
        )
        .anzahl(1),
        fail(
            "aria/required-parent-missing",
            r##"<ul role="tablist"><li><a role="tab" href="#x" aria-selected="true">X</a></li></ul><div role="tabpanel" id="x">X</div>"##,
            "ARIA in HTML: li in ul ist listitem; WAI-ARIA 1.2 tab braucht tablist (auditmysite#715)",
        )
        .variante("li dazwischen"),
        negativ(
            "aria/required-parent-missing",
            r#"<div role="tablist" aria-label="T"><span><div role="tab" tabindex="0" aria-selected="true">A</div></span></div><div role="tabpanel">P</div>"#,
            "WAI-ARIA 1.2: generic-Zwischenknoten werden übersprungen",
        )
        .variante("durch generic hindurch"),
        negativ(
            "aria/required-parent-missing",
            r#"<div role="tablist" aria-label="T" aria-owns="t1"></div><div role="tab" id="t1" tabindex="0" aria-selected="true">A</div><div role="tabpanel">P</div>"#,
            "WAI-ARIA 1.2: contained in, or owned by (aria-owns)",
        )
        .variante("aria-owns"),
        negativ(
            "aria/required-parent-missing",
            r##"<ul role="tablist"><li role="presentation"><a role="tab" href="#x" aria-selected="true">X</a></li></ul><div role="tabpanel" id="x">X</div>"##,
            "WAI-ARIA 1.2 presentation: li ist durchlässig",
        )
        .variante("li role=presentation"),
        negativ(
            "aria/required-parent-missing",
            r#"<div role="tablist" aria-label="T"><template shadowrootmode="open"><div role="tab" tabindex="0" aria-selected="true">A</div></template></div><div role="tabpanel">P</div>"#,
            "DOM flat tree: Shadow-Inhalt steht unter seinem Host",
        )
        .variante("Shadow DOM"),
        na(
            "aria/required-parent-missing",
            r#"<div aria-hidden="true"><div role="tab">Versteckt</div></div><p>Text</p>"#,
            "ACT ff89c9: gilt nur für Elemente im Accessibility-Tree",
        )
        .variante("unter aria-hidden"),
        na(
            "aria/required-parent-missing",
            r#"<button>Ohne Kontextrolle</button>"#,
            ARIA12_KONTEXT,
        ),
        // --- aria/required-children-missing ---------------------------------
        fail(
            "aria/required-children-missing",
            r#"<div role="tablist" aria-label="T"><div role="button" tabindex="0">A</div></div>"#,
            ARIA12_BESTAND,
        )
        .anzahl(1),
        fail(
            "aria/required-children-missing",
            r#"<div role="table" aria-label="Kunden"><div role="caption">Kunden</div></div>"#,
            ARIA12_BESTAND,
        )
        .variante("Tabelle nur mit Beschriftung"),
        negativ(
            "aria/required-children-missing",
            r#"<div role="listbox" aria-label="L"><div role="group"><div role="option">o</div></div></div>
               <div role="list"><span><div role="listitem">x</div></span></div>"#,
            ARIA12_BESTAND,
        ),
        negativ(
            "aria/required-children-missing",
            r#"<div role="listbox" aria-label="L" aria-owns="o1"></div><div role="option" id="o1">o</div>"#,
            "WAI-ARIA 1.2 Required Owned Elements: contain or own (aria-owns)",
        )
        .variante("aria-owns"),
        negativ(
            "aria/required-children-missing",
            r#"<div role="listbox" aria-busy="true" aria-label="lädt"><p>Lädt …</p></div>"#,
            "WAI-ARIA 1.2 aria-busy: Bestandteile dürfen während des Ladens fehlen",
        )
        .variante("aria-busy"),
        negativ(
            "aria/required-children-missing",
            r#"<div role="tablist" aria-label="leer"></div>"#,
            ARIA12_BESTAND,
        )
        .variante("leer"),
        na(
            "aria/required-children-missing",
            r#"<table><tbody><tr><td>nativ</td></tr></tbody></table><ul><li>nativ</li></ul>"#,
            "Native Tabellen und Listen: Bestandteile durch HTML gegeben (auditmysite#659)",
        ),
        // --- aria/role-redundant --------------------------------------------
        fail(
            "aria/role-redundant",
            r#"<button type="button" role="button">Senden</button>"#,
            ARIA_IN_HTML_ROLLE,
        )
        .anzahl(1),
        fail(
            "aria/role-redundant",
            r#"<a href="/" role="link">Start</a>"#,
            ARIA_IN_HTML_ROLLE,
        )
        .variante("a mit href"),
        negativ(
            "aria/role-redundant",
            r#"<a role="link" tabindex="0">ohne href</a>
               <img src="a.png" alt="" role="img">
               <input type="text" list="l" role="textbox" aria-label="x"><datalist id="l"></datalist>"#,
            "HTML-AAM: a ohne href ist generic, img alt=\"\" presentation, input mit list combobox",
        )
        .variante("Rolle weicht von der impliziten ab"),
        negativ(
            "aria/role-redundant",
            r#"<ul role="list" style="list-style-type:none"><li role="listitem">a</li></ul>"#,
            "WebKit entfernt Listensemantik bei list-style none; role=list ist Abhilfe (auditmysite#644)",
        )
        .variante("ul role=list, li role=listitem"),
        na(
            "aria/role-redundant",
            r#"<button>Ohne role</button>"#,
            ARIA_IN_HTML_ROLLE,
        ),
        // --- names/title-only -----------------------------------------------
        review(
            "names/title-only",
            r#"<a href="/" title="Startseite"><img src="h.png" alt=""></a>"#,
            TITLE_ONLY,
        )
        .anzahl(1),
        review(
            "names/title-only",
            r#"<button title="Schließen"><span aria-hidden="true">×</span></button>"#,
            "accname 1.2 Schritt 2A: aria-hidden-Inhalt zählt nicht; Name nur aus title",
        )
        .variante("Text nur unter aria-hidden"),
        review(
            "names/title-only",
            r#"<input type="checkbox" title="Zustimmen">"#,
            TITLE_ONLY,
        )
        .variante("Checkbox ohne label"),
        negativ(
            "names/title-only",
            r#"<button title="Speichern">Speichern</button>
               <button title="x" aria-label="Suchen"></button>
               <a href="/" title="Start"><img src="h.png" alt="Start"></a>"#,
            "accname 1.2: Inhalt, aria-label und alt gehen title vor",
        )
        .variante("Inhalt, aria-label, alt"),
        negativ(
            "names/title-only",
            r#"<input type="checkbox" id="c" title="Zustimmen"><label for="c">Ich stimme zu</label>
               <label><input type="radio" name="r" title="A"> Option A</label>
               <input type="submit" title="Senden">"#,
            "HTML-AAM: label-Element benennt; submit hat Vorgabenamen",
        )
        .variante("label for, umschließendes label, submit"),
        na(
            "names/title-only",
            r#"<button>Ohne title</button><p title="Hinweis">Kein Bedienelement</p>"#,
            TITLE_ONLY,
        ),
        // --- patterns/tooltip-unreferenced ----------------------------------
        fail(
            "patterns/tooltip-unreferenced",
            r#"<div role="tooltip" id="orphan-tip">Hilfe</div>"#,
            TOOLTIP,
        )
        .anzahl(1),
        fail(
            "patterns/tooltip-unreferenced",
            r#"<div role="tooltip" id="tip" hidden>Hilfe</div>"#,
            "Versteckte Tooltips erscheinen erst beim Überfahren; APG Tooltip",
        )
        .variante("versteckt"),
        fail(
            "patterns/tooltip-unreferenced",
            r#"<button aria-describedby="tip">?</button>
               <div><template shadowrootmode="open"><div role="tooltip" id="tip">Hilfe</div></template></div>"#,
            "DOM: IDREFs gelten nur im eigenen Baumbereich",
        )
        .variante("Verweis über die Shadow-Grenze"),
        negativ(
            "patterns/tooltip-unreferenced",
            r#"<button aria-describedby="tip">?</button><div role="tooltip" id="tip">Hilfe</div>"#,
            TOOLTIP,
        )
        .variante("aria-describedby"),
        negativ(
            "patterns/tooltip-unreferenced",
            r#"<button aria-labelledby="tip"><svg aria-hidden="true"></svg></button><div role="tooltip" id="tip">Löschen</div>"#,
            "accname 1.2: aria-labelledby macht den Tooltip zum Namen, er wird angesagt",
        )
        .variante("aria-labelledby"),
        na(
            "patterns/tooltip-unreferenced",
            r#"<div role="note" id="n">Kein Tooltip</div>"#,
            TOOLTIP,
        ),
    ]
}
