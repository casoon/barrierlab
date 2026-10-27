//! Gruppe „struktur“: Rollen, Verweise, Pflichtattribute, IDs, Tabfolge,
//! Listen und Tabellen (`structure.rs`, `tables.rs`).

use super::super::{Fall, fail, na, negativ, review, untested};

pub fn faelle() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(rollen());
    v.extend(verweise());
    v.extend(pflichtattribute());
    v.extend(ids());
    v.extend(tabfolge());
    v.extend(listen());
    v.extend(tabellen());
    v
}

// --- aria/role-invalid, aria/role-abstract ----------------------------------

fn rollen() -> Vec<Fall> {
    const ACT: &str = "ACT 674b10 (Role attribute has valid value); WAI-ARIA 1.2 §5.3 Kategorien";
    vec![
        fail("aria/role-invalid", r#"<div role="buton">Senden</div>"#, ACT),
        fail(
            "aria/role-invalid",
            r#"<div role="foo bar">Inhalt</div>"#,
            "ACT 674b10: kein Token ist eine gültige Rolle",
        )
        .variante("mehrere ungültige Token")
        .anzahl(2),
        negativ(
            "aria/role-invalid",
            r#"<div role="button" tabindex="0">Senden</div>"#,
            ACT,
        ),
        negativ(
            "aria/role-invalid",
            r#"<div role="switch-x button" tabindex="0">Senden</div>"#,
            "ACT 674b10: mindestens ein gültiges Token genügt; WAI-ARIA 1.2 Rückfallrollen (Handling Author Errors)",
        )
        .variante("Regression: ungültiges Token mit gültiger Rückfallrolle"),
        negativ(
            "aria/role-invalid",
            r#"<svg role="graphics-document" aria-label="Diagramm"></svg><section role="doc-chapter" aria-label="Eins">x</section>"#,
            "WAI-ARIA Graphics Module 1.0, DPUB-ARIA 1.1: Modulrollen sind gültig",
        )
        .variante("Modulrollen"),
        negativ(
            "aria/role-invalid",
            r#"<div aria-hidden="true"><div role="buton">x</div></div>"#,
            "ACT 674b10, Anwendbarkeit: nur Elemente im Accessibility-Tree",
        )
        .variante("aria-hidden"),
        na(
            "aria/role-invalid",
            r#"<div>Kein role-Attribut</div><button>OK</button>"#,
            "ACT 674b10, Anwendbarkeit: Elemente mit role-Attribut",
        ),
        fail(
            "aria/role-abstract",
            r#"<div role="widget">x</div>"#,
            "WAI-ARIA 1.2 Abstract Roles: Autoren dürfen abstrakte Rollen nicht verwenden",
        ),
        fail(
            "aria/role-abstract",
            r#"<div role="landmark">x</div>"#,
            "WAI-ARIA 1.2 Abstract Roles: landmark",
        )
        .variante("landmark"),
        negativ(
            "aria/role-abstract",
            r#"<div role="region" aria-label="Bereich">x</div>"#,
            "WAI-ARIA 1.2: region ist die konkrete Unterklasse von landmark",
        ),
        na(
            "aria/role-abstract",
            r#"<main><p>Ohne Rollen</p></main>"#,
            "WAI-ARIA 1.2 Abstract Roles: betrifft nur role-Werte",
        ),
    ]
}

// --- aria/reference-missing --------------------------------------------------

fn verweise() -> Vec<Fall> {
    const ID: &str = "aria/reference-missing";
    const Q: &str =
        "WAI-ARIA 1.2 ID Reference: Verweis auf ein Element im selben Baum; WCAG 1.3.1/4.1.2";
    vec![
        fail(
            ID,
            r#"<input aria-describedby="hilfe-fehlt" aria-label="Name">"#,
            Q,
        ),
        fail(
            ID,
            r#"<button aria-controls="" aria-expanded="true">Menü</button>"#,
            "WAI-ARIA 1.2 aria-controls: IDREF-Liste; leer behauptet eine Beziehung ohne Ziel",
        )
        .variante("leeres aria-controls"),
        fail(
            ID,
            r#"<input aria-invalid="true" aria-errormessage="fehler" aria-label="Mail">"#,
            "WAI-ARIA 1.2 aria-errormessage: maßgeblich, sobald aria-invalid gesetzt ist",
        )
        .variante("aria-errormessage bei aria-invalid"),
        fail(
            ID,
            r#"<span id="beschr">Hilfe</span><div><template shadowrootmode="open"><input aria-describedby="beschr" aria-label="X"></template></div>"#,
            "DOM: IDREFs gelten im eigenen Baum (Shadow Root), nicht über die Grenze",
        )
        .variante("Shadow DOM: Verweis über die Grenze"),
        negativ(
            ID,
            r#"<input aria-describedby="hilfe" aria-label="Name"><p id="hilfe">Vor- und Nachname</p>"#,
            Q,
        ),
        negativ(
            ID,
            r#"<input aria-describedby="hilfe" aria-label="Name"><p id="hilfe" hidden>Vor- und Nachname</p>"#,
            "accname 1.2 Schritt 2B: versteckte Ziele von Verweisen zählen",
        )
        .variante("Ziel versteckt"),
        negativ(
            ID,
            r#"<div><template shadowrootmode="open"><input aria-describedby="b" aria-label="X"><span id="b">Hilfe</span></template></div>"#,
            "DOM: Verweis innerhalb derselben Shadow Root",
        )
        .variante("Shadow DOM: Ziel im selben Baum"),
        negativ(
            ID,
            r#"<input aria-errormessage="fehler" aria-label="Mail">"#,
            "WAI-ARIA 1.2 aria-errormessage: ohne aria-invalid nicht maßgeblich",
        )
        .variante("aria-errormessage ohne aria-invalid"),
        negativ(
            ID,
            r#"<button aria-labelledby="" aria-label="Senden">Senden</button>"#,
            "accname 1.2 Schritt 2B: leeres aria-labelledby fällt auf die übrigen Quellen zurück",
        )
        .variante("leeres aria-labelledby"),
        negativ(
            ID,
            r#"<button aria-expanded="false" aria-controls="menu-spaeter">Menü</button>"#,
            "Dokumentierte Ausnahme (wie axe aria-valid-attr-value): eingeklapptes Ziel darf erst beim Öffnen entstehen",
        )
        .variante("aria-controls eingeklappt"),
        na(
            ID,
            r#"<p>Kein Verweisattribut</p><input aria-label="Name">"#,
            "WAI-ARIA 1.2 ID Reference: nur IDREF-Attribute",
        ),
    ]
}

// --- aria/required-attribute-missing -----------------------------------------

fn pflichtattribute() -> Vec<Fall> {
    const ID: &str = "aria/required-attribute-missing";
    const Q: &str = "WAI-ARIA 1.2 Required States and Properties; ACT 4e8ab6";
    vec![
        fail(
            ID,
            r#"<div role="checkbox" tabindex="0">Zustimmen</div>"#,
            Q,
        ),
        fail(
            ID,
            r#"<div role="slider" tabindex="0" aria-label="Lautstärke"></div>"#,
            "WAI-ARIA 1.2 slider: aria-valuenow erforderlich",
        )
        .variante("slider"),
        fail(
            ID,
            r#"<div role="separator" tabindex="0" aria-label="Teiler"></div>"#,
            "WAI-ARIA 1.2 separator: fokussierbar ist er ein Widget und braucht aria-valuenow",
        )
        .variante("fokussierbarer separator"),
        fail(
            ID,
            r#"<div role="scrollbar" aria-valuenow="0"></div>"#,
            "WAI-ARIA 1.2 scrollbar: aria-controls und aria-valuenow erforderlich",
        )
        .variante("scrollbar ohne aria-controls"),
        negativ(
            ID,
            r#"<div role="checkbox" tabindex="0" aria-checked="false">Zustimmen</div>"#,
            Q,
        ),
        negativ(
            ID,
            r#"<input type="checkbox" role="switch" aria-label="WLAN">"#,
            "ARIA in HTML / ACT 4e8ab6: nativer Zustand ersetzt aria-checked",
        )
        .variante("nativ: input checkbox als switch"),
        negativ(
            ID,
            r#"<input type="range" role="slider" aria-label="Lautstärke">"#,
            "ACT 4e8ab6: nativer Wert ersetzt aria-valuenow",
        )
        .variante("nativ: input range"),
        negativ(
            ID,
            r#"<div role="separator"></div>"#,
            "WAI-ARIA 1.2 separator: der nicht fokussierbare Trenner braucht keinen Wert",
        )
        .variante("statischer separator"),
        negativ(
            ID,
            r#"<div role="option" aria-label="A">A</div>"#,
            "WAI-ARIA 1.2: aria-selected an option ist nicht mehr erforderlich",
        ),
        na(
            ID,
            r#"<div role="button" tabindex="0">Senden</div>"#,
            "WAI-ARIA 1.2 button: keine erforderlichen Attribute",
        ),
    ]
}

// --- ids/duplicate -----------------------------------------------------------

fn ids() -> Vec<Fall> {
    const ID: &str = "ids/duplicate";
    const Q: &str = "WCAG 4.1.2 (4.1.1 entfallen); axe duplicate-id-aria; Normative Zuordnung (auditmysite#779): REVIEW";
    vec![
        review(
            ID,
            r#"<input aria-labelledby="l"><span id="l">Name</span><span id="l">Anderes</span>"#,
            Q,
        )
        .anzahl(1),
        review(
            ID,
            r#"<label for="f">Name</label><input id="f"><input id="f">"#,
            "HTML label for: IDREF auf doppelte ID",
        )
        .variante("label for"),
        negativ(
            ID,
            r#"<p id="x">A</p><p id="x">B</p>"#,
            "WCAG 2.2 ohne 4.1.1: eine nicht referenzierte doppelte ID ist kein Befund",
        )
        .variante("nicht referenziert"),
        negativ(
            ID,
            r#"<input aria-labelledby="l"><span id="l">Name</span><div><template shadowrootmode="open"><span id="l">Innen</span></template></div>"#,
            "DOM: IDs sind je Baum eindeutig; Shadow Root ist ein eigener Baum",
        )
        .variante("Shadow DOM"),
        negativ(
            ID,
            r#"<input aria-labelledby="l"><span id="l">Name</span><span id="m">Anderes</span>"#,
            Q,
        ),
        na(ID, r#"<p>Keine IDs</p>"#, "Keine IDs, keine Verweise"),
    ]
}

// --- keyboard/positive-tabindex, keyboard/hidden-focusable -------------------

fn tabfolge() -> Vec<Fall> {
    const POS: &str = "keyboard/positive-tabindex";
    const F44: &str = "WCAG 2.4.3, F44; Normative Zuordnung (auditmysite#779): REVIEW";
    const VER: &str = "keyboard/hidden-focusable";
    const ACT: &str = "ACT 6cfa84 (aria-hidden ohne fokussierbaren Inhalt); WCAG 4.1.2";
    vec![
        review(POS, r#"<button tabindex="1">Zuerst</button>"#, F44)
            .variante("Grenze: 1")
            .anzahl(1),
        review(POS, r#"<a href="/x" tabindex="32767">Weit hinten</a>"#, F44)
            .variante("großer Wert"),
        negativ(POS, r#"<div tabindex="0">Fokussierbar</div>"#, F44).variante("Grenze: 0"),
        negativ(
            POS,
            r#"<div tabindex="-1">Programmatisch</div>"#,
            "HTML tabindex: negativ nimmt aus der Tabfolge",
        )
        .variante("-1"),
        negativ(
            POS,
            r#"<button tabindex="2" style="display:none">Weg</button><button tabindex="3" hidden>Weg</button>"#,
            "Nicht dargestellt: nicht in der Tabfolge",
        )
        .variante("versteckt"),
        na(POS, r#"<button>OK</button>"#, "Ohne tabindex-Attribut"),
        fail(
            VER,
            r#"<div aria-hidden="true"><a href="/x">Link</a></div>"#,
            ACT,
        ),
        fail(VER, r#"<button aria-hidden="true">X</button>"#, ACT).variante("am Element selbst"),
        fail(
            VER,
            r#"<div aria-hidden="true"><span tabindex="0">Fokus</span></div>"#,
            ACT,
        )
        .variante("tabindex=0"),
        negativ(
            VER,
            r#"<div aria-hidden="true"><a href="/x" tabindex="-1">Link</a></div>"#,
            "ACT 6cfa84: tabindex=-1 nimmt aus der sequentiellen Navigation",
        )
        .variante("tabindex=-1"),
        negativ(
            VER,
            r#"<div aria-hidden="true" inert><button>X</button></div>"#,
            "HTML inert: nicht fokussierbar",
        )
        .variante("inert"),
        negativ(
            VER,
            r#"<div aria-hidden="true"><button disabled>X</button><input type="hidden" name="t"></div>"#,
            "HTML: disabled und type=hidden sind nicht fokussierbar",
        )
        .variante("disabled"),
        negativ(
            VER,
            r#"<div aria-hidden="true" style="display:none"><a href="/x">Link</a></div>"#,
            "ACT 6cfa84: nicht dargestellt, nicht fokussierbar",
        )
        .variante("display:none"),
        negativ(
            VER,
            r#"<div aria-hidden="true"><p>Nur Text</p></div><a href="/x">Sichtbar</a>"#,
            ACT,
        ),
        na(
            VER,
            r#"<a href="/x">Link</a>"#,
            "ACT 6cfa84, Anwendbarkeit: aria-hidden=\"true\"",
        ),
    ]
}

// --- lists/* -----------------------------------------------------------------

fn listen() -> Vec<Fall> {
    const INV: &str = "lists/invalid-structure";
    const LEER: &str = "lists/empty";
    const AUS: &str = "lists/item-outside-list";
    const DT: &str = "lists/term-without-definition";
    const UL: &str =
        "HTML ul/ol: Inhaltsmodell li und skriptunterstützende Elemente; WCAG 1.3.1 (F2)";
    vec![
        fail(INV, r#"<ul><li>A</li><div>B</div></ul>"#, UL),
        fail(
            INV,
            r#"<div role="list"><div role="listitem">A</div><div>B</div></div>"#,
            "WAI-ARIA 1.2 list: Required Owned Elements listitem",
        )
        .variante("ARIA-Liste"),
        fail(
            INV,
            r#"<ul><li role="group">A</li></ul>"#,
            "WAI-ARIA 1.2: li mit fremder Rolle ist kein listitem",
        )
        .variante("li mit fremder Rolle"),
        fail(
            INV,
            r#"<menu><li>A</li><p>B</p></menu>"#,
            "HTML menu: Inhaltsmodell li; HTML-AAM menu → list",
        )
        .variante("Regression: menu"),
        negativ(
            INV,
            r#"<ul><li>A</li><script></script><template><p>x</p></template><li>B</li></ul>"#,
            UL,
        ),
        negativ(
            INV,
            r#"<ul role="tablist"><li role="tab">A</li></ul>"#,
            "WAI-ARIA 1.2: eine andere Rolle macht aus ul keine Liste",
        )
        .variante("ul mit anderer Rolle"),
        negativ(
            INV,
            r#"<ul hidden><div>B</div></ul>"#,
            "Nicht im Accessibility-Tree",
        )
        .variante("hidden"),
        na(INV, r#"<p>Keine Liste</p>"#, UL),
        fail(
            LEER,
            r#"<ul></ul>"#,
            "Normative Zuordnung (auditmysite#779): Best Practice, Befund ohne Kriterium",
        ),
        fail(
            LEER,
            r#"<div role="list"></div>"#,
            "Normative Zuordnung (auditmysite#779): Best Practice",
        )
        .variante("ARIA-Liste"),
        negativ(LEER, r#"<ul><li>A</li></ul>"#, "Liste mit Eintrag"),
        negativ(
            LEER,
            r#"<div><template shadowrootmode="open"><ul><slot><x-slotted><li>A</li></x-slotted></slot></ul></template></div>"#,
            "DOM flat tree: per slot eingesetzte Einträge sind Kinder der Liste",
        )
        .variante("Shadow DOM: slot"),
        negativ(
            LEER,
            r#"<ul role="presentation"></ul>"#,
            "WAI-ARIA 1.2: präsentational ist keine Liste",
        )
        .variante("role=presentation"),
        na(LEER, r#"<p>Keine Liste</p>"#, "Keine Liste"),
        fail(
            AUS,
            r#"<div><li>Allein</li></div>"#,
            "HTML li: Elternelement ol, ul oder menu; WAI-ARIA 1.2 listitem: Required Context list",
        ),
        fail(
            AUS,
            r#"<div role="listitem">Allein</div>"#,
            "WAI-ARIA 1.2 listitem: Required Context Role list",
        )
        .variante("ARIA listitem"),
        fail(
            AUS,
            r#"<ul role="tablist"><li>Kein Tab</li></ul>"#,
            "WAI-ARIA 1.2: ul[role=tablist] ist keine Liste (auditmysite#715)",
        )
        .variante("Liste mit fremder Rolle"),
        negativ(AUS, r#"<ol><li>Eins</li></ol>"#, "HTML li in ol"),
        negativ(
            AUS,
            r#"<menu><li><button>Kopieren</button></li></menu>"#,
            "HTML li: menu ist zulässiges Elternelement; HTML-AAM menu → list",
        )
        .variante("Regression: menu"),
        negativ(
            AUS,
            r#"<ul role="none"><li>Eins</li></ul>"#,
            "WAI-ARIA 1.2 Presentational Roles Conflict Resolution: li erbt none",
        )
        .variante("Liste role=none"),
        na(AUS, r#"<p>Kein Listeneintrag</p>"#, "Kein li"),
        fail(
            DT,
            r#"<dl><dt>Begriff</dt></dl>"#,
            "HTML dl: Inhaltsmodell Gruppen aus dt gefolgt von dd; WCAG 1.3.1",
        ),
        fail(
            DT,
            r#"<dl><dd>Definition</dd><dt>Begriff</dt></dl>"#,
            "HTML dl: auf dt muss ein dd folgen; ein vorangehendes dd gehört zu keiner Gruppe mit ihm",
        )
        .variante("Regression: dd vor dt"),
        fail(
            DT,
            r#"<div role="term">Begriff</div>"#,
            "WAI-ARIA 1.2 term mit definition",
        )
        .variante("ARIA term"),
        negativ(
            DT,
            r#"<dl><dt>A</dt><dt>B</dt><dd>Beide</dd></dl>"#,
            "HTML dl: mehrere dt teilen sich ein dd",
        ),
        negativ(
            DT,
            r#"<dl><div><dt>A</dt><dd>a</dd></div></dl>"#,
            "HTML dl: div-Gruppen",
        )
        .variante("div-Gruppe"),
        na(DT, r#"<p>Keine Begriffe</p>"#, "Kein dt"),
    ]
}

// --- tables/* ----------------------------------------------------------------

fn tabellen() -> Vec<Fall> {
    const KOPF: &str = "tables/header-missing";
    const NAME: &str = "tables/name-missing";
    const PRAES: &str = "tables/presentational-with-headers";
    const OHNE: &str = "tables/header-without-data";
    const UNB: &str = "tables/data-undetermined";
    const HDR: &str = "tables/headers-attr-invalid";
    vec![
        review(
            KOPF,
            r#"<table><caption>Preise</caption><tr><td>A</td><td>1</td></tr></table>"#,
            "WCAG 1.3.1, F91; Normative Zuordnung (auditmysite#779): REVIEW",
        ),
        review(
            KOPF,
            r#"<div role="grid" aria-label="G"><div role="row"><div role="gridcell">1</div></div></div>"#,
            "F91, ARIA grid",
        )
        .variante("ARIA grid"),
        negativ(
            KOPF,
            r#"<table><caption>Preise</caption><tr><th>Artikel</th></tr><tr><td>A</td></tr></table>"#,
            "WCAG 1.3.1, H51",
        ),
        negativ(
            KOPF,
            r#"<div role="table" aria-label="T"><div role="row"><div role="columnheader">K</div></div><div role="row"><div role="cell">1</div></div></div>"#,
            "WAI-ARIA 1.2 columnheader",
        )
        .variante("ARIA columnheader"),
        negativ(
            KOPF,
            r#"<table role="presentation"><tr><td>Layout</td></tr></table>"#,
            "WCAG F46: Layouttabelle ohne Kopfzellen ist zulässig",
        )
        .variante("Layouttabelle"),
        na(KOPF, r#"<p>Keine Tabelle</p>"#, "Keine Tabelle"),
        review(
            NAME,
            r#"<table><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table name; WCAG 1.3.1 (H39); Heuristik → REVIEW",
        ),
        negativ(
            NAME,
            r#"<table><caption>Preise</caption><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: caption",
        )
        .variante("caption"),
        negativ(
            NAME,
            r#"<table aria-label="Preise"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: aria-label",
        )
        .variante("aria-label"),
        negativ(
            NAME,
            r#"<h2 id="t">Preise</h2><table aria-labelledby="t"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: aria-labelledby",
        )
        .variante("aria-labelledby"),
        negativ(
            NAME,
            r#"<table title="Preise"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM „table Element Accessible Name Computation“, Schritt title",
        )
        .variante("Regression: title"),
        review(
            NAME,
            r#"<table aria-labelledby=" "><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "accname 1.2 Schritt 2B: leeres aria-labelledby liefert keinen Namen",
        )
        .variante("Regression: leeres aria-labelledby"),
        na(NAME, r#"<p>Keine Tabelle</p>"#, "Keine Tabelle"),
        fail(
            PRAES,
            r#"<table role="presentation"><tr><th>Kopf</th></tr><tr><td>1</td></tr></table>"#,
            "WCAG 1.3.1, F46 (Kopfauszeichnung in Layouttabelle)",
        ),
        fail(
            PRAES,
            r#"<table role="none"><tr><td role="columnheader">Kopf</td></tr></table>"#,
            "F46, Rolle columnheader in präsentationaler Tabelle",
        )
        .variante("role=none, ARIA columnheader"),
        negativ(
            PRAES,
            r#"<table role="presentation"><tr><td>Layout</td></tr></table>"#,
            "F46: Layouttabelle ohne Kopfauszeichnung",
        ),
        na(
            PRAES,
            r#"<table><caption>T</caption><tr><th>K</th></tr><tr><td>1</td></tr></table>"#,
            "Nicht präsentational",
        ),
        fail(
            OHNE,
            r#"<table><caption>T</caption><tr><th>A</th><th>B</th></tr></table>"#,
            "ACT d0f69e (Table header cell has assigned cells); WCAG 1.3.1",
        )
        .anzahl(2),
        negativ(
            OHNE,
            r#"<table><caption>T</caption><tr><th>A</th></tr><tr><td></td></tr></table>"#,
            "ACT d0f69e: auch eine leere Zelle ist zugeordnet",
        )
        .variante("Grenze: eine leere Datenzelle"),
        negativ(
            OHNE,
            r#"<div role="grid" aria-label="G"><div role="row"><div role="columnheader">A</div></div><div role="row"><div role="gridcell">1</div></div></div>"#,
            "ACT d0f69e, ARIA grid mit gridcell",
        )
        .variante("ARIA grid"),
        na(
            OHNE,
            r#"<table><caption>T</caption><tr><td>1</td></tr></table>"#,
            "ACT d0f69e, Anwendbarkeit: Kopfzellen",
        ),
        untested(
            UNB,
            r#"<table><caption>T</caption><thead><tr><th>A</th></tr></thead><tbody></tbody></table>"#,
            "Nicht bestimmbar: Zeilengruppe ohne Zeilen (auditmysite#654); „Lief nicht ist nicht bestanden“",
        ),
        negativ(
            UNB,
            r#"<table><caption>T</caption><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>"#,
            "ACT d0f69e: geladene Zeilen",
        ),
        na(
            UNB,
            r#"<table><caption>T</caption><tbody></tbody></table>"#,
            "Ohne Kopfzellen nicht anwendbar",
        ),
        fail(
            HDR,
            r#"<table><tr><th id="k">K</th></tr><tr><td headers="fehlt">1</td></tr></table>"#,
            "HTML headers: IDs von th derselben Tabelle; ACT a25f45",
        ),
        fail(
            HDR,
            r#"<table><tr><td id="d">x</td><td headers="d">1</td></tr></table>"#,
            "HTML headers: Ziel muss th sein",
        )
        .variante("Ziel ist td"),
        fail(
            HDR,
            r#"<table><tr><th id="s" headers="s">K</th></tr><tr><td>1</td></tr></table>"#,
            "ACT a25f45 Erwartung 2: kein Verweis auf die Zelle selbst",
        )
        .variante("Selbstverweis"),
        fail(
            HDR,
            r#"<table><tr><th>K</th></tr><tr><td headers="fremd">1</td></tr></table><table><tr><th id="fremd">F</th></tr><tr><td>2</td></tr></table>"#,
            "HTML headers: th derselben Tabelle",
        )
        .variante("andere Tabelle"),
        negativ(
            HDR,
            r#"<table><tr><th id="a">A</th><th id="b">B</th></tr><tr><td headers="a b">1</td></tr></table>"#,
            "HTML headers; H43",
        ),
        negativ(
            HDR,
            r#"<p headers="nirgends">Kein Zellelement</p>"#,
            "HTML: headers ist nur an td/th definiert",
        )
        .variante("kein Zellelement"),
        na(
            HDR,
            r#"<table><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "Kein headers-Attribut",
        ),
    ]
}
