//! Gruppe „struktur“: Rollen, Verweise, Pflichtattribute, IDs, Tabfolge,
//! Listen und Tabellen (`structure.rs`, `tables.rs`).

use super::super::{Fall, fail, na, negative, review, untested};

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(roles());
    v.extend(references());
    v.extend(required_attributes());
    v.extend(ids());
    v.extend(tab_order());
    v.extend(lists());
    v.extend(tables());
    v
}

// --- aria/role-invalid, aria/role-abstract ----------------------------------

fn roles() -> Vec<Fall> {
    const ACT: &str = "ACT 674b10 (Role attribute has valid value); WAI-ARIA 1.2 §5.3 Kategorien";
    vec![
        fail("aria/role-invalid", r#"<div role="buton">Senden</div>"#, ACT),
        fail(
            "aria/role-invalid",
            r#"<div role="foo bar">Inhalt</div>"#,
            "ACT 674b10: kein Token ist eine gültige Rolle",
        )
        .variant("mehrere ungültige Token")
        .count(2),
        negative(
            "aria/role-invalid",
            r#"<div role="button" tabindex="0">Senden</div>"#,
            ACT,
        ),
        negative(
            "aria/role-invalid",
            r#"<div role="switch-x button" tabindex="0">Senden</div>"#,
            "ACT 674b10: mindestens ein gültiges Token genügt; WAI-ARIA 1.2 Rückfallrollen (Handling Author Errors)",
        )
        .variant("Regression: ungültiges Token mit gültiger Rückfallrolle"),
        negative(
            "aria/role-invalid",
            r#"<svg role="graphics-document" aria-label="Diagramm"></svg><section role="doc-chapter" aria-label="Eins">x</section>"#,
            "WAI-ARIA Graphics Module 1.0, DPUB-ARIA 1.1: Modulrollen sind gültig",
        )
        .variant("Modulrollen"),
        negative(
            "aria/role-invalid",
            r#"<div aria-hidden="true"><div role="buton">x</div></div>"#,
            "ACT 674b10, Anwendbarkeit: nur Elemente im Accessibility-Tree",
        )
        .variant("aria-hidden"),
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
        .variant("landmark"),
        negative(
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

fn references() -> Vec<Fall> {
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
        .variant("leeres aria-controls"),
        fail(
            ID,
            r#"<input aria-invalid="true" aria-errormessage="fehler" aria-label="Mail">"#,
            "WAI-ARIA 1.2 aria-errormessage: maßgeblich, sobald aria-invalid gesetzt ist",
        )
        .variant("aria-errormessage bei aria-invalid"),
        fail(
            ID,
            r#"<span id="beschr">Hilfe</span><div><template shadowrootmode="open"><input aria-describedby="beschr" aria-label="X"></template></div>"#,
            "DOM: IDREFs gelten im eigenen Baum (Shadow Root), nicht über die Grenze",
        )
        .variant("Shadow DOM: Verweis über die Grenze"),
        negative(
            ID,
            r#"<input aria-describedby="hilfe" aria-label="Name"><p id="hilfe">Vor- und Nachname</p>"#,
            Q,
        ),
        negative(
            ID,
            r#"<input aria-describedby="hilfe" aria-label="Name"><p id="hilfe" hidden>Vor- und Nachname</p>"#,
            "accname 1.2 Schritt 2B: versteckte Ziele von Verweisen zählen",
        )
        .variant("Ziel versteckt"),
        negative(
            ID,
            r#"<div><template shadowrootmode="open"><input aria-describedby="b" aria-label="X"><span id="b">Hilfe</span></template></div>"#,
            "DOM: Verweis innerhalb derselben Shadow Root",
        )
        .variant("Shadow DOM: Ziel im selben Baum"),
        negative(
            ID,
            r#"<input aria-errormessage="fehler" aria-label="Mail">"#,
            "WAI-ARIA 1.2 aria-errormessage: ohne aria-invalid nicht maßgeblich",
        )
        .variant("aria-errormessage ohne aria-invalid"),
        negative(
            ID,
            r#"<button aria-labelledby="" aria-label="Senden">Senden</button>"#,
            "accname 1.2 Schritt 2B: leeres aria-labelledby fällt auf die übrigen Quellen zurück",
        )
        .variant("leeres aria-labelledby"),
        negative(
            ID,
            r#"<button aria-expanded="false" aria-controls="menu-spaeter">Menü</button>"#,
            "Dokumentierte Ausnahme (wie axe aria-valid-attr-value): eingeklapptes Ziel darf erst beim Öffnen entstehen",
        )
        .variant("aria-controls eingeklappt"),
        na(
            ID,
            r#"<p>Kein Verweisattribut</p><input aria-label="Name">"#,
            "WAI-ARIA 1.2 ID Reference: nur IDREF-Attribute",
        ),
    ]
}

// --- aria/required-attribute-missing -----------------------------------------

fn required_attributes() -> Vec<Fall> {
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
        .variant("slider"),
        fail(
            ID,
            r#"<div role="separator" tabindex="0" aria-label="Teiler"></div>"#,
            "WAI-ARIA 1.2 separator: fokussierbar ist er ein Widget und braucht aria-valuenow",
        )
        .variant("fokussierbarer separator"),
        fail(
            ID,
            r#"<div role="scrollbar" aria-valuenow="0"></div>"#,
            "WAI-ARIA 1.2 scrollbar: aria-controls und aria-valuenow erforderlich",
        )
        .variant("scrollbar ohne aria-controls"),
        negative(
            ID,
            r#"<div role="checkbox" tabindex="0" aria-checked="false">Zustimmen</div>"#,
            Q,
        ),
        negative(
            ID,
            r#"<input type="checkbox" role="switch" aria-label="WLAN">"#,
            "ARIA in HTML / ACT 4e8ab6: nativer Zustand ersetzt aria-checked",
        )
        .variant("nativ: input checkbox als switch"),
        negative(
            ID,
            r#"<input type="range" role="slider" aria-label="Lautstärke">"#,
            "ACT 4e8ab6: nativer Wert ersetzt aria-valuenow",
        )
        .variant("nativ: input range"),
        negative(
            ID,
            r#"<div role="separator"></div>"#,
            "WAI-ARIA 1.2 separator: der nicht fokussierbare Trenner braucht keinen Wert",
        )
        .variant("statischer separator"),
        negative(
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
        .count(1),
        review(
            ID,
            r#"<label for="f">Name</label><input id="f"><input id="f">"#,
            "HTML label for: IDREF auf doppelte ID",
        )
        .variant("label for"),
        negative(
            ID,
            r#"<p id="x">A</p><p id="x">B</p>"#,
            "WCAG 2.2 ohne 4.1.1: eine nicht referenzierte doppelte ID ist kein Befund",
        )
        .variant("nicht referenziert"),
        negative(
            ID,
            r#"<input aria-labelledby="l"><span id="l">Name</span><div><template shadowrootmode="open"><span id="l">Innen</span></template></div>"#,
            "DOM: IDs sind je Baum eindeutig; Shadow Root ist ein eigener Baum",
        )
        .variant("Shadow DOM"),
        negative(
            ID,
            r#"<input aria-labelledby="l"><span id="l">Name</span><span id="m">Anderes</span>"#,
            Q,
        ),
        na(ID, r#"<p>Keine IDs</p>"#, "Keine IDs, keine Verweise"),
    ]
}

// --- keyboard/positive-tabindex, keyboard/hidden-focusable -------------------

fn tab_order() -> Vec<Fall> {
    const POS: &str = "keyboard/positive-tabindex";
    const F44: &str = "WCAG 2.4.3, F44; Normative Zuordnung (auditmysite#779): REVIEW";
    const VER: &str = "keyboard/hidden-focusable";
    const ACT: &str = "ACT 6cfa84 (aria-hidden ohne fokussierbaren Inhalt); WCAG 4.1.2";
    vec![
        review(POS, r#"<button tabindex="1">Zuerst</button>"#, F44)
            .variant("Grenze: 1")
            .count(1),
        review(POS, r#"<a href="/x" tabindex="32767">Weit hinten</a>"#, F44)
            .variant("großer Wert"),
        negative(POS, r#"<div tabindex="0">Fokussierbar</div>"#, F44).variant("Grenze: 0"),
        negative(
            POS,
            r#"<div tabindex="-1">Programmatisch</div>"#,
            "HTML tabindex: negativ nimmt aus der Tabfolge",
        )
        .variant("-1"),
        negative(
            POS,
            r#"<button tabindex="2" style="display:none">Weg</button><button tabindex="3" hidden>Weg</button>"#,
            "Nicht dargestellt: nicht in der Tabfolge",
        )
        .variant("versteckt"),
        na(POS, r#"<button>OK</button>"#, "Ohne tabindex-Attribut"),
        fail(
            VER,
            r#"<div aria-hidden="true"><a href="/x">Link</a></div>"#,
            ACT,
        ),
        fail(VER, r#"<button aria-hidden="true">X</button>"#, ACT).variant("am Element selbst"),
        fail(
            VER,
            r#"<div aria-hidden="true"><span tabindex="0">Fokus</span></div>"#,
            ACT,
        )
        .variant("tabindex=0"),
        negative(
            VER,
            r#"<div aria-hidden="true"><a href="/x" tabindex="-1">Link</a></div>"#,
            "ACT 6cfa84: tabindex=-1 nimmt aus der sequentiellen Navigation",
        )
        .variant("tabindex=-1"),
        negative(
            VER,
            r#"<div aria-hidden="true" inert><button>X</button></div>"#,
            "HTML inert: nicht fokussierbar",
        )
        .variant("inert"),
        negative(
            VER,
            r#"<div aria-hidden="true"><button disabled>X</button><input type="hidden" name="t"></div>"#,
            "HTML: disabled und type=hidden sind nicht fokussierbar",
        )
        .variant("disabled"),
        negative(
            VER,
            r#"<div aria-hidden="true" style="display:none"><a href="/x">Link</a></div>"#,
            "ACT 6cfa84: nicht dargestellt, nicht fokussierbar",
        )
        .variant("display:none"),
        negative(
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

fn lists() -> Vec<Fall> {
    const INV: &str = "lists/invalid-structure";
    const EMPTY: &str = "lists/empty";
    const OUTSIDE: &str = "lists/item-outside-list";
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
        .variant("ARIA-Liste"),
        fail(
            INV,
            r#"<ul><li role="group">A</li></ul>"#,
            "WAI-ARIA 1.2: li mit fremder Rolle ist kein listitem",
        )
        .variant("li mit fremder Rolle"),
        fail(
            INV,
            r#"<menu><li>A</li><p>B</p></menu>"#,
            "HTML menu: Inhaltsmodell li; HTML-AAM menu → list",
        )
        .variant("Regression: menu"),
        negative(
            INV,
            r#"<ul><li>A</li><script></script><template><p>x</p></template><li>B</li></ul>"#,
            UL,
        ),
        negative(
            INV,
            r#"<ul role="tablist"><li role="tab">A</li></ul>"#,
            "WAI-ARIA 1.2: eine andere Rolle macht aus ul keine Liste",
        )
        .variant("ul mit anderer Rolle"),
        negative(
            INV,
            r#"<ul hidden><div>B</div></ul>"#,
            "Nicht im Accessibility-Tree",
        )
        .variant("hidden"),
        na(INV, r#"<p>Keine Liste</p>"#, UL),
        fail(
            EMPTY,
            r#"<ul></ul>"#,
            "Normative Zuordnung (auditmysite#779): Best Practice, Befund ohne Kriterium",
        ),
        fail(
            EMPTY,
            r#"<div role="list"></div>"#,
            "Normative Zuordnung (auditmysite#779): Best Practice",
        )
        .variant("ARIA-Liste"),
        negative(EMPTY, r#"<ul><li>A</li></ul>"#, "Liste mit Eintrag"),
        negative(
            EMPTY,
            r#"<div><template shadowrootmode="open"><ul><slot><x-slotted><li>A</li></x-slotted></slot></ul></template></div>"#,
            "DOM flat tree: per slot eingesetzte Einträge sind Kinder der Liste",
        )
        .variant("Shadow DOM: slot"),
        negative(
            EMPTY,
            r#"<ul role="presentation"></ul>"#,
            "WAI-ARIA 1.2: präsentational ist keine Liste",
        )
        .variant("role=presentation"),
        na(EMPTY, r#"<p>Keine Liste</p>"#, "Keine Liste"),
        fail(
            OUTSIDE,
            r#"<div><li>Allein</li></div>"#,
            "HTML li: Elternelement ol, ul oder menu; WAI-ARIA 1.2 listitem: Required Context list",
        ),
        fail(
            OUTSIDE,
            r#"<div role="listitem">Allein</div>"#,
            "WAI-ARIA 1.2 listitem: Required Context Role list",
        )
        .variant("ARIA listitem"),
        fail(
            OUTSIDE,
            r#"<ul role="tablist"><li>Kein Tab</li></ul>"#,
            "WAI-ARIA 1.2: ul[role=tablist] ist keine Liste (auditmysite#715)",
        )
        .variant("Liste mit fremder Rolle"),
        negative(OUTSIDE, r#"<ol><li>Eins</li></ol>"#, "HTML li in ol"),
        negative(
            OUTSIDE,
            r#"<menu><li><button>Kopieren</button></li></menu>"#,
            "HTML li: menu ist zulässiges Elternelement; HTML-AAM menu → list",
        )
        .variant("Regression: menu"),
        negative(
            OUTSIDE,
            r#"<ul role="none"><li>Eins</li></ul>"#,
            "WAI-ARIA 1.2 Presentational Roles Conflict Resolution: li erbt none",
        )
        .variant("Liste role=none"),
        na(OUTSIDE, r#"<p>Kein Listeneintrag</p>"#, "Kein li"),
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
        .variant("Regression: dd vor dt"),
        fail(
            DT,
            r#"<div role="term">Begriff</div>"#,
            "WAI-ARIA 1.2 term mit definition",
        )
        .variant("ARIA term"),
        negative(
            DT,
            r#"<dl><dt>A</dt><dt>B</dt><dd>Beide</dd></dl>"#,
            "HTML dl: mehrere dt teilen sich ein dd",
        ),
        negative(
            DT,
            r#"<dl><div><dt>A</dt><dd>a</dd></div></dl>"#,
            "HTML dl: div-Gruppen",
        )
        .variant("div-Gruppe"),
        na(DT, r#"<p>Keine Begriffe</p>"#, "Kein dt"),
    ]
}

// --- tables/* ----------------------------------------------------------------

fn tables() -> Vec<Fall> {
    const HEAD: &str = "tables/header-missing";
    const NAME: &str = "tables/name-missing";
    const PRESENTATION: &str = "tables/presentational-with-headers";
    const WITHOUT: &str = "tables/header-without-data";
    const UNB: &str = "tables/data-undetermined";
    const HDR: &str = "tables/headers-attr-invalid";
    vec![
        review(
            HEAD,
            r#"<table><caption>Preise</caption><tr><td>A</td><td>1</td></tr></table>"#,
            "WCAG 1.3.1, F91; Normative Zuordnung (auditmysite#779): REVIEW",
        ),
        review(
            HEAD,
            r#"<div role="grid" aria-label="G"><div role="row"><div role="gridcell">1</div></div></div>"#,
            "F91, ARIA grid",
        )
        .variant("ARIA grid"),
        negative(
            HEAD,
            r#"<table><caption>Preise</caption><tr><th>Artikel</th></tr><tr><td>A</td></tr></table>"#,
            "WCAG 1.3.1, H51",
        ),
        negative(
            HEAD,
            r#"<div role="table" aria-label="T"><div role="row"><div role="columnheader">K</div></div><div role="row"><div role="cell">1</div></div></div>"#,
            "WAI-ARIA 1.2 columnheader",
        )
        .variant("ARIA columnheader"),
        negative(
            HEAD,
            r#"<table role="presentation"><tr><td>Layout</td></tr></table>"#,
            "WCAG F46: Layouttabelle ohne Kopfzellen ist zulässig",
        )
        .variant("Layouttabelle"),
        na(HEAD, r#"<p>Keine Tabelle</p>"#, "Keine Tabelle"),
        review(
            NAME,
            r#"<table><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table name; WCAG 1.3.1 (H39); Heuristik → REVIEW",
        ),
        negative(
            NAME,
            r#"<table><caption>Preise</caption><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: caption",
        )
        .variant("caption"),
        negative(
            NAME,
            r#"<table aria-label="Preise"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: aria-label",
        )
        .variant("aria-label"),
        negative(
            NAME,
            r#"<h2 id="t">Preise</h2><table aria-labelledby="t"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM table: aria-labelledby",
        )
        .variant("aria-labelledby"),
        negative(
            NAME,
            r#"<table title="Preise"><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "HTML-AAM „table Element Accessible Name Computation“, Schritt title",
        )
        .variant("Regression: title"),
        review(
            NAME,
            r#"<table aria-labelledby=" "><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "accname 1.2 Schritt 2B: leeres aria-labelledby liefert keinen Namen",
        )
        .variant("Regression: leeres aria-labelledby"),
        na(NAME, r#"<p>Keine Tabelle</p>"#, "Keine Tabelle"),
        fail(
            PRESENTATION,
            r#"<table role="presentation"><tr><th>Kopf</th></tr><tr><td>1</td></tr></table>"#,
            "WCAG 1.3.1, F46 (Kopfauszeichnung in Layouttabelle)",
        ),
        fail(
            PRESENTATION,
            r#"<table role="none"><tr><td role="columnheader">Kopf</td></tr></table>"#,
            "F46, Rolle columnheader in präsentationaler Tabelle",
        )
        .variant("role=none, ARIA columnheader"),
        negative(
            PRESENTATION,
            r#"<table role="presentation"><tr><td>Layout</td></tr></table>"#,
            "F46: Layouttabelle ohne Kopfauszeichnung",
        ),
        na(
            PRESENTATION,
            r#"<table><caption>T</caption><tr><th>K</th></tr><tr><td>1</td></tr></table>"#,
            "Nicht präsentational",
        ),
        fail(
            WITHOUT,
            r#"<table><caption>T</caption><tr><th>A</th><th>B</th></tr></table>"#,
            "ACT d0f69e (Table header cell has assigned cells); WCAG 1.3.1",
        )
        .count(2),
        negative(
            WITHOUT,
            r#"<table><caption>T</caption><tr><th>A</th></tr><tr><td></td></tr></table>"#,
            "ACT d0f69e: auch eine leere Zelle ist zugeordnet",
        )
        .variant("Grenze: eine leere Datenzelle"),
        negative(
            WITHOUT,
            r#"<div role="grid" aria-label="G"><div role="row"><div role="columnheader">A</div></div><div role="row"><div role="gridcell">1</div></div></div>"#,
            "ACT d0f69e, ARIA grid mit gridcell",
        )
        .variant("ARIA grid"),
        na(
            WITHOUT,
            r#"<table><caption>T</caption><tr><td>1</td></tr></table>"#,
            "ACT d0f69e, Anwendbarkeit: Kopfzellen",
        ),
        untested(
            UNB,
            r#"<table><caption>T</caption><thead><tr><th>A</th></tr></thead><tbody></tbody></table>"#,
            "Nicht bestimmbar: Zeilengruppe ohne Zeilen (auditmysite#654); „Lief nicht ist nicht bestanden“",
        ),
        negative(
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
        .variant("Ziel ist td"),
        fail(
            HDR,
            r#"<table><tr><th id="s" headers="s">K</th></tr><tr><td>1</td></tr></table>"#,
            "ACT a25f45 Erwartung 2: kein Verweis auf die Zelle selbst",
        )
        .variant("Selbstverweis"),
        fail(
            HDR,
            r#"<table><tr><th>K</th></tr><tr><td headers="fremd">1</td></tr></table><table><tr><th id="fremd">F</th></tr><tr><td>2</td></tr></table>"#,
            "HTML headers: th derselben Tabelle",
        )
        .variant("andere Tabelle"),
        negative(
            HDR,
            r#"<table><tr><th id="a">A</th><th id="b">B</th></tr><tr><td headers="a b">1</td></tr></table>"#,
            "HTML headers; H43",
        ),
        negative(
            HDR,
            r#"<p headers="nirgends">Kein Zellelement</p>"#,
            "HTML: headers ist nur an td/th definiert",
        )
        .variant("kein Zellelement"),
        na(
            HDR,
            r#"<table><tr><th>A</th></tr><tr><td>1</td></tr></table>"#,
            "Kein headers-Attribut",
        ),
    ]
}
