//! Gruppe „formulare“: Formularfelder, Kontextwechsel, Anmeldung (src/forms.rs)
//! und Klick-Handler, Links als Knöpfe, Ortsangabe (src/links.rs).

use super::super::{Fall, fail, na, negativ, review};

pub fn faelle() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(autocomplete());
    v.extend(fehler());
    v.extend(gruppen());
    v.extend(absenden());
    v.extend(wiederholung());
    v.extend(kontext());
    v.extend(captcha());
    v.extend(zweck());
    v.extend(anleitung());
    v.extend(titel_und_gruppenname());
    v.extend(links());
    v
}

const AC: &str = "forms/autocomplete-invalid";
const ACT_AC: &str = "WCAG 1.3.5; ACT 73f2c2; HTML „Autofill“ (autofill detail tokens)";

fn autocomplete() -> Vec<Fall> {
    vec![
        fail(AC, r#"<label for="e">Email</label><input id="e" autocomplete="emial">"#, ACT_AC)
            .variante("unbekanntes Token")
            .anzahl(1),
        fail(AC, r#"<label for="n">Name</label><input id="n" autocomplete="work name">"#, ACT_AC)
            .variante("Grenze: Kontaktangabe vor Nicht-Kontaktfeld"),
        fail(AC, r#"<label for="e">Email</label><input id="e" autocomplete="email work">"#, ACT_AC)
            .variante("Grenze: falsche Reihenfolge"),
        fail(AC, r#"<label for="c">Country</label><select id="c" autocomplete="nation"><option>DE</option></select>"#, ACT_AC)
            .variante("select"),
        fail(AC, r#"<label for="m">Message</label><textarea id="m" autocomplete="comment"></textarea>"#, ACT_AC)
            .variante("textarea"),
        negativ(AC, r#"<label for="e">Email</label><input id="e" autocomplete="work email">"#, ACT_AC)
            .variante("Kontaktangabe vor Kontaktfeld"),
        negativ(AC, r#"<label for="s">Street</label><input id="s" autocomplete="section-a shipping street-address">"#, ACT_AC)
            .variante("section + shipping"),
        negativ(AC, r#"<label for="p">Password</label><input id="p" type="password" autocomplete="current-password webauthn">"#, ACT_AC)
            .variante("webauthn"),
        negativ(AC, r#"<label for="x">Code</label><input id="x" autocomplete="off">"#, "HTML „Autofill“: on/off sind gültig; ACT 73f2c2 nimmt sie aus"),
        negativ(AC, r#"<label for="e">Email</label><input id="e" autocomplete="EMAIL">"#, "HTML „Autofill“: Tokens ASCII-case-insensitiv")
            .variante("Großschreibung"),
        na(AC, r#"<label for="e">Email</label><input id="e">"#, ACT_AC),
        na(AC, r#"<input type="hidden" name="t" autocomplete="token">"#, ACT_AC)
            .variante("type=hidden"),
        na(AC, r#"<input type="submit" value="Send" autocomplete="sendit">"#, "ACT 73f2c2: input type submit/button/reset nicht anwendbar")
            .variante("Regression: type=submit"),
        na(AC, r#"<label for="e">Email</label><input id="e" disabled autocomplete="emial">"#, "ACT 73f2c2: deaktivierte Elemente nicht anwendbar")
            .variante("Regression: disabled"),
        na(AC, r#"<label for="e">Email</label><input id="e" aria-disabled="true" autocomplete="emial">"#, "ACT 73f2c2: aria-disabled=\"true\" nicht anwendbar")
            .variante("Regression: aria-disabled"),
        na(AC, r#"<div hidden><label for="e">Email</label><input id="e" autocomplete="emial"></div>"#, "ACT 73f2c2: weder sichtbar noch im Accessibility-Tree")
            .variante("hidden-Attribut"),
        na(AC, r#"<div style="display:none"><input aria-label="Email" autocomplete="emial"></div>"#, "ACT 73f2c2: weder sichtbar noch im Accessibility-Tree")
            .variante("display:none"),
    ]
}

const EU: &str = "forms/error-unidentified";
const Q_EU: &str = "WCAG 3.3.1, ARIA21; WAI-ARIA 1.2 aria-invalid; REVIEW nach a11y-rules.md (G83, auditmysite#779)";

fn fehler() -> Vec<Fall> {
    vec![
        review(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="true">"#, Q_EU).anzahl(1),
        review(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="yes">"#, "WAI-ARIA 1.2 aria-invalid: unbekannter Wert gilt als true")
            .variante("unbekannter Wert"),
        review(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="true" aria-describedby="m"><span id="m"></span>"#, Q_EU)
            .variante("Grenze: Verweis auf leeres Element beschreibt nichts"),
        review(EU, r#"<div role="textbox" aria-label="Code" tabindex="0" aria-invalid="grammar"></div>"#, Q_EU)
            .variante("ARIA-Rolle textbox"),
        negativ(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="true" aria-describedby="m"><span id="m">Enter an address like name@example.com</span>"#, "WCAG 3.3.1, ARIA21 (aria-describedby)")
            .variante("aria-describedby"),
        negativ(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="true" aria-errormessage="m"><span id="m">The address is missing an @</span>"#, "WCAG 3.3.1, ARIA21; WAI-ARIA 1.2 aria-errormessage")
            .variante("aria-errormessage"),
        negativ(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="true" aria-describedby="m"><span id="m" hidden>Address is missing an @</span>"#, "accname 1.2 2B: versteckte verwiesene Inhalte zählen für die Beschreibung")
            .variante("verstecktes Beschreibungsziel"),
        na(EU, r#"<label for="e">Email</label><input id="e" aria-invalid="false">"#, "WAI-ARIA 1.2: aria-invalid=false heißt gültig")
            .variante("aria-invalid=false"),
        na(EU, r#"<label for="e">Email</label><input id="e">"#, Q_EU),
        na(EU, r#"<div hidden><label for="e">Email</label><input id="e" aria-invalid="true"></div>"#, "Nicht dargestellt: kein Feld für Nutzer")
            .variante("hidden-Attribut"),
    ]
}

const GM: &str = "forms/group-missing";
const Q_GM: &str = "WCAG 1.3.1, H71/ARIA17; REVIEW nach a11y-rules.md (fieldset hinreichend, nicht Pflicht; auditmysite#779)";

fn gruppen() -> Vec<Fall> {
    vec![
        review(GM, r#"<p>Size</p><label><input type="radio" name="s" value="s"> Small</label><label><input type="radio" name="s" value="l"> Large</label>"#, Q_GM)
            .anzahl(2),
        review(GM, r#"<form><label><input type="checkbox" name="t" value="a"> News</label><label><input type="checkbox" name="t" value="b"> Offers</label></form>"#, Q_GM)
            .variante("Grenze: zwei Kontrollkästchen mit demselben name")
            .anzahl(2),
        review(GM, r#"<div role="radio" aria-checked="true" tabindex="0">Yes</div><div role="radio" aria-checked="false" tabindex="-1">No</div>"#, Q_GM)
            .variante("ARIA-Rolle radio"),
        negativ(GM, r#"<fieldset><legend>Size</legend><label><input type="radio" name="s"> Small</label><label><input type="radio" name="s"> Large</label></fieldset>"#, "WCAG 1.3.1, H71")
            .variante("fieldset"),
        negativ(GM, r#"<div role="radiogroup" aria-label="Size"><label><input type="radio" name="s"> Small</label><label><input type="radio" name="s"> Large</label></div>"#, "WCAG 1.3.1, ARIA17")
            .variante("role=radiogroup"),
        negativ(GM, r#"<div role="group" aria-label="Topics"><label><input type="checkbox" name="t"> News</label><label><input type="checkbox" name="t"> Offers</label></div>"#, "WCAG 1.3.1, ARIA17")
            .variante("role=group"),
        negativ(GM, r#"<form><label><input type="checkbox" name="terms"> I accept the terms</label></form>"#, "Ein einzelnes Kontrollkästchen ist keine Menge (auditmysite#643)")
            .variante("Grenze: ein Kontrollkästchen"),
        negativ(GM, r#"<form><label><input type="checkbox" name="a"> News</label><label><input type="checkbox" name="b"> Terms</label></form>"#, "Verschiedene name: keine zusammengehörige Menge")
            .variante("verschiedene name"),
        negativ(GM, r#"<form><label><input type="checkbox" name="t"> News</label></form><form><label><input type="checkbox" name="t"> News</label></form>"#, "HTML form owner: gleiche name in zwei Formularen sind zwei Mengen")
            .variante("zwei Formulare"),
        na(GM, r#"<label for="q">Search</label><input id="q" type="text">"#, Q_GM),
        na(GM, r#"<div hidden><label><input type="radio" name="s"> A</label><label><input type="radio" name="s"> B</label></div>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}

const NS: &str = "forms/no-submit";
const Q_NS: &str = "WCAG 3.2.2, H32; REVIEW nach a11y-rules.md (auditmysite#779)";

fn absenden() -> Vec<Fall> {
    vec![
        review(NS, r#"<form action="/s"><label for="q">Query</label><input id="q" name="q"></form>"#, Q_NS).anzahl(1),
        review(NS, r#"<form><label for="c">Sort</label><select id="c"><option>A</option></select></form>"#, Q_NS)
            .variante("ohne action, ohne Textfeld"),
        review(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><button type="button">Go</button></form>"#, "HTML: button type=button sendet nicht ab")
            .variante("Grenze: type=button"),
        review(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><button disabled>Go</button></form>"#, "HTML: deaktivierter Button sendet nicht ab")
            .variante("Grenze: disabled"),
        negativ(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><button>Go</button></form>"#, "HTML: button ohne type ist Submit Button (missing value default); H32")
            .variante("button ohne type"),
        negativ(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><button type="foo">Go</button></form>"#, "HTML: invalid value default von button type ist Submit Button; H32")
            .variante("Regression: ungültiger type"),
        negativ(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><input type="submit" value="Go"></form>"#, "WCAG 3.2.2, H32")
            .variante("input type=submit"),
        negativ(NS, r#"<form action="/s"><label for="q">Query</label><input id="q"><input type="image" src="go.png" alt="Go"></form>"#, "WCAG 3.2.2, H32")
            .variante("input type=image"),
        negativ(NS, r#"<form id="f" action="/s"><label for="q">Query</label><input id="q"></form><button form="f">Go</button>"#, "HTML form owner: form-Attribut")
            .variante("Button über form-Attribut"),
        na(NS, r#"<form action="/s"><input type="hidden" name="t" value="1"></form>"#, "Kein Bedienelement im Formular")
            .variante("nur hidden"),
        na(NS, r#"<label for="q">Query</label><input id="q">"#, "Kein Formular"),
        na(NS, r#"<form action="/s" hidden><input name="q"></form>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}

const RE: &str = "forms/redundant-entry";
const Q_RE: &str =
    "WCAG 3.3.7 Redundant Entry; REVIEW: ob dieselbe Angabe gemeint ist, entscheidet ein Mensch";

fn wiederholung() -> Vec<Fall> {
    vec![
        review(RE, r#"<form action="/o"><label for="a">Email</label><input id="a"><label for="b">Billing email</label><input id="b"><button>Order</button></form>"#, Q_RE)
            .anzahl(1),
        review(RE, r#"<form action="/o"><input aria-label="Phone" name="p1"><input aria-label="Mobile phone" name="p2"><button>Order</button></form>"#, Q_RE)
            .variante("aria-label"),
        negativ(RE, r#"<form action="/o"><label for="a">Email</label><input id="a"><label for="b">Billing email</label><input id="b" value="a@example.com"><button>Order</button></form>"#, "WCAG 3.3.7: auto-populated")
            .variante("vorbelegt"),
        negativ(RE, r#"<form action="/o"><label for="a">Email</label><input id="a"><label><input type="checkbox"> Same as above</label><label for="b">Billing email</label><input id="b"><button>Order</button></form>"#, "WCAG 3.3.7: available for the user to select (G221)")
            .variante("„same as“"),
        negativ(RE, r#"<form action="/o"><label for="a">Email</label><input id="a"><button>Order</button></form>"#, Q_RE)
            .variante("Grenze: einmal gefragt"),
        negativ(RE, r#"<form action="/o"><label for="a">Email</label><input id="a"><button>Go</button></form><form action="/n"><label for="b">Email</label><input id="b"><button>Go</button></form>"#, "WCAG 3.3.7: gilt innerhalb eines Vorgangs; je Formular")
            .variante("zwei Formulare"),
        na(RE, r#"<form action="/s"><label for="q">Query</label><input id="q"><button>Go</button></form>"#, Q_RE),
    ]
}

const OI: &str = "context/on-input";
const OF: &str = "context/on-focus";
const AF: &str = "context/autofocus";

fn kontext() -> Vec<Fall> {
    vec![
        // context/on-input
        fail(OI, r#"<label for="l">Language</label><select id="l" onchange="location.href=this.value"><option value="/de">DE</option></select>"#, "WCAG 3.2.2, F37/F36: Auswahl navigiert ohne Vorwarnung")
            .anzahl(1),
        fail(OI, r#"<label><input type="radio" name="r" onchange="this.form.submit()"> A</label>"#, "WCAG 3.2.2, F37 (Optionsfeld)")
            .variante("radio, submit"),
        fail(OI, r#"<label><input type="checkbox" name="c" onchange="window.open('/x')"> A</label>"#, "WCAG 3.2.2, F37 nennt ausdrücklich check box")
            .variante("Regression: checkbox"),
        fail(OI, r#"<label for="l">Language</label><select id="l" onchange="location . href = this.value"><option>DE</option></select>"#, "WCAG 3.2.2, F37")
            .variante("Leerraum im Handler"),
        review(OI, r#"<label for="l">Sort</label><select id="l" onchange="updateList(this.value)"><option>A</option></select>"#, "WCAG 3.2.2: Handler ohne erkennbaren Kontextwechsel, Inhalt der Funktion unbekannt")
            .variante("unbekannte Funktion"),
        review(OI, r#"<div role="listbox" aria-label="Sort" tabindex="0" onchange="updateList()"><div role="option">A</div></div>"#, "WCAG 3.2.2")
            .variante("ARIA-Rolle listbox"),
        negativ(OI, r#"<form action="/s"><label for="l">Language</label><select id="l" name="l"><option>DE</option></select><button>Change language</button></form>"#, "WCAG 3.2.2, G80/H32: Wechsel erst über Absende-Knopf"),
        na(OI, r#"<label for="q">Query</label><input id="q" onchange="location.href=this.value">"#, "F37 gilt für Auswahlelemente (Optionsfeld, Kontrollkästchen, Auswahlliste)")
            .variante("Textfeld"),
        na(OI, r#"<p>No controls</p>"#, "Kein Auswahlelement"),
        na(OI, r#"<div hidden><select aria-label="L" onchange="location.href=this.value"><option>DE</option></select></div>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
        // context/on-focus
        review(OF, r#"<div tabindex="0" onfocus="openPanel()">Info</div>"#, "WCAG 3.2.1, G107: onfocus an nicht interaktivem Element; REVIEW, Handler unbekannt")
            .anzahl(1),
        review(OF, r#"<div aria-hidden="true" tabindex="0" onfocus="go()">Info</div>"#, "WCAG 3.2.1: gilt für sichtbare Komponenten unabhängig vom Accessibility-Tree (Scope Rendered)")
            .variante("aria-hidden"),
        negativ(OF, r#"<label for="q">Query</label><input id="q" onfocus="showHint()">"#, "WCAG 3.2.1, G107: Fokus-Handler an Eingabefeld zeigt Hinweis, kein Kontextwechsel")
            .variante("input"),
        negativ(OF, r#"<div role="button" tabindex="0" onfocus="highlight()">Go</div>"#, "WCAG 3.2.1, G107; ARIA-Rolle button ist interaktiv")
            .variante("ARIA-Rolle button"),
        na(OF, r#"<div tabindex="0">Info</div>"#, "Kein onfocus"),
        na(OF, r#"<div hidden tabindex="0" onfocus="go()">Info</div>"#, "Nicht dargestellt: nicht fokussierbar")
            .variante("hidden-Attribut"),
        // context/autofocus
        review(AF, r#"<button autofocus>Start</button>"#, "WCAG 3.2.1 / 2.4.3: autofocus an Nicht-Eingabefeld kann desorientieren; REVIEW nach Regeldoku")
            .anzahl(1),
        review(AF, r#"<div tabindex="-1" autofocus>Panel</div>"#, "WCAG 3.2.1 / 2.4.3")
            .variante("div"),
        negativ(AF, r#"<label for="q">Search</label><input id="q" autofocus>"#, "HTML autofocus: Eingabefeld, Fokus erwartet (G107)")
            .variante("input"),
        negativ(AF, r#"<div role="searchbox" aria-label="Search" contenteditable="true" autofocus></div>"#, "ARIA-Rolle searchbox ist ein Eingabefeld")
            .variante("ARIA-Rolle searchbox"),
        na(AF, r#"<button>Start</button>"#, "Kein autofocus"),
        na(AF, r#"<button hidden autofocus>Start</button>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}

const CA: &str = "auth/captcha";
const Q_CA: &str = "WCAG 3.3.8 Accessible Authentication (Minimum); REVIEW: Objekterkennung oder Alternative sind erlaubt";

fn captcha() -> Vec<Fall> {
    vec![
        review(CA, r#"<form action="/login"><label for="u">User</label><input id="u" autocomplete="username"><label for="p">Password</label><input id="p" type="password" autocomplete="current-password"><div class="g-recaptcha" data-sitekey="x"></div><button>Sign in</button></form>"#, Q_CA)
            .anzahl(1),
        review(CA, r#"<form action="/verify"><label for="c">Code</label><input id="c" autocomplete="one-time-code"><img src="/captcha.png" alt="Captcha"><button>Verify</button></form>"#, Q_CA)
            .variante("Einmalcode + Captcha-Bild"),
        review(CA, r#"<form action="/login"><label for="p">Password</label><input id="p" type="password"><iframe title="reCAPTCHA" src="https://www.google.com/recaptcha/api2/anchor?k=x"></iframe><button>Sign in</button></form>"#, Q_CA)
            .variante("iframe"),
        negativ(CA, r#"<form action="/login"><label for="p">Password</label><input id="p" type="password"><div class="g-recaptcha" data-size="invisible"></div><button>Sign in</button></form>"#, "WCAG 3.3.8: unsichtbare Variante stellt keinen Test")
            .variante("unsichtbar"),
        negativ(CA, r#"<form action="/contact"><label for="m">Message</label><textarea id="m"></textarea><div class="g-recaptcha"></div><button>Send</button></form>"#, "WCAG 3.3.8 gilt nur für Authentifizierung; Kontaktformular")
            .variante("Kontaktformular"),
        na(CA, r#"<form action="/login"><label for="p">Password</label><input id="p" type="password"><button>Sign in</button></form>"#, Q_CA),
        na(CA, r#"<form action="/login" hidden><input type="password" aria-label="Password"><div class="g-recaptcha"></div></form>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}

const PM: &str = "forms/purpose-missing";
const Q_PM: &str = "WCAG 1.3.5, H98: Feld zur Person ohne autocomplete; REVIEW (Zweck geraten)";

fn zweck() -> Vec<Fall> {
    vec![
        review(PM, r#"<label for="e">Email</label><input id="e">"#, Q_PM).anzahl(1),
        review(PM, r#"<input type="tel" aria-label="Number">"#, Q_PM).variante("type=tel"),
        review(PM, r#"<input aria-label="Street">"#, Q_PM).variante("aria-label"),
        review(
            PM,
            r#"<label for="n">Name</label><input id="n" autocomplete="off">"#,
            "WCAG 1.3.5: off gibt keinen Zweck an",
        )
        .variante("autocomplete=off"),
        negativ(
            PM,
            r#"<label for="e">Email</label><input id="e" autocomplete="email">"#,
            "WCAG 1.3.5, H98",
        )
        .variante("autocomplete"),
        negativ(
            PM,
            r#"<label for="v">Project name</label><input id="v">"#,
            "WCAG 1.3.5 betrifft Angaben zur Person, nicht Namen von Dingen (auditmysite#658)",
        )
        .variante("Name eines Dings"),
        negativ(
            PM,
            r#"<label for="h">Hotel</label><input id="h">"#,
            "WCAG 1.3.5; „Hotel“ ist keine Telefonangabe",
        )
        .variante("Grenze: Teilwort"),
        na(
            PM,
            r#"<label><input type="checkbox"> Email me</label>"#,
            "WCAG 1.3.5: Eingabefelder, die Text annehmen",
        )
        .variante("checkbox"),
        na(PM, r#"<p>No fields</p>"#, Q_PM),
        na(
            PM,
            r#"<div aria-hidden="true"><input aria-label="Email" tabindex="-1"></div>"#,
            "Nicht im Accessibility-Tree",
        )
        .variante("aria-hidden"),
    ]
}

const RU: &str = "forms/required-unmarked";
const IM: &str = "forms/instructions-missing";
const Q_RU: &str =
    "WCAG 3.3.2, H90/ARIA2; REVIEW nach Regeldoku (sichtbarer Hinweis kann woanders stehen)";
const Q_IM: &str =
    "WCAG 3.3.2, G89 (Formatangabe); REVIEW: Formatbedarf aus der Beschriftung geraten";

fn anleitung() -> Vec<Fall> {
    vec![
        review(RU, r#"<label for="n">Full name</label><input id="n" required>"#, Q_RU).anzahl(1),
        review(RU, r#"<div role="textbox" aria-label="Comment" aria-required="true" tabindex="0"></div>"#, Q_RU)
            .variante("ARIA-Rolle textbox, aria-required"),
        review(RU, r#"<label><input type="checkbox" required> I accept the terms</label>"#, Q_RU)
            .variante("checkbox"),
        negativ(RU, r#"<label for="n">Full name *</label><input id="n" required>"#, "WCAG 3.3.2, H90: Stern in der Beschriftung")
            .variante("Stern"),
        negativ(RU, r#"<label for="n">Full name (required)</label><input id="n" required>"#, "WCAG 3.3.2, H90")
            .variante("Wort"),
        negativ(RU, r#"<label for="n">Full name</label><input id="n" required aria-describedby="h"><span id="h">Required</span>"#, "WCAG 3.3.2, ARIA1")
            .variante("aria-describedby"),
        na(RU, r#"<label for="n">Full name</label><input id="n">"#, "Kein Pflichtfeld"),
        na(RU, r#"<label for="n">Full name</label><input id="n" aria-required="false">"#, "WAI-ARIA 1.2: aria-required=false")
            .variante("aria-required=false"),
        na(RU, r#"<div hidden><label for="n">Full name</label><input id="n" required></div>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
        review(IM, r#"<label for="d">Date of birth</label><input id="d">"#, Q_IM).anzahl(1),
        review(IM, r#"<label for="p">Passport number</label><input id="p">"#, Q_IM)
            .variante("Zusammensetzung"),
        negativ(IM, r#"<label for="d">Date of birth (DD.MM.YYYY)</label><input id="d">"#, "WCAG 3.3.2, G89")
            .variante("Format in der Beschriftung"),
        negativ(IM, r#"<label for="d">Date of birth</label><input id="d" aria-describedby="h"><span id="h">Day, month, year</span>"#, "WCAG 3.3.2, G89 + ARIA1")
            .variante("aria-describedby"),
        negativ(IM, r#"<label for="d">Date of birth</label><input id="d" placeholder="31.12.1990">"#, "WCAG 3.3.2: sichtbares Beispiel (Grenzfall nach Regeldoku)")
            .variante("placeholder"),
        negativ(IM, r#"<label for="d">Date of birth</label><input id="d" type="date">"#, "HTML input type=date: der Browser gibt das Format vor (auditmysite#656)")
            .variante("type=date"),
        negativ(IM, r#"<label for="w">What happened?</label><input id="w">"#, "Kurzer Begriff nur als ganzes Wort (auditmysite#643)")
            .variante("Grenze: „pass“ in „happened“ nicht"),
        na(IM, r#"<label for="c">Comment</label><input id="c">"#, "Keine Formatvorgabe"),
        na(IM, r#"<label><input type="checkbox"> Date night</label>"#, "Kontrollkästchen haben kein Eingabeformat")
            .variante("checkbox"),
    ]
}

const TO: &str = "forms/title-only-label";
const GN: &str = "forms/group-name-missing";

fn titel_und_gruppenname() -> Vec<Fall> {
    vec![
        review(TO, r#"<input title="Search">"#, "WCAG 1.3.1/3.3.2, H65: title nur bei Zeigerkontakt sichtbar; REVIEW nach Regeldoku")
            .anzahl(1),
        review(TO, r#"<select title="Country"><option>DE</option></select>"#, "WCAG 1.3.1, H65")
            .variante("select (combobox)"),
        negativ(TO, r#"<label for="s">Search</label><input id="s" title="Search">"#, "WCAG 1.3.1, H44: label-Element")
            .variante("label for"),
        negativ(TO, r#"<label>Search <input title="Search"></label>"#, "WCAG 1.3.1, H44: umschließendes label")
            .variante("umschließendes label"),
        negativ(TO, r#"<input title="Search" aria-label="Search">"#, "WCAG 1.3.1, ARIA14")
            .variante("aria-label"),
        negativ(TO, r#"<span id="l">Search</span><input title="Search" aria-labelledby="l">"#, "WCAG 1.3.1, ARIA16")
            .variante("aria-labelledby"),
        na(TO, r#"<label><input type="checkbox" title="Agree"> Agree</label>"#, "Kein Texteingabefeld"),
        na(TO, r#"<p title="Hint">Text</p>"#, "Kein Feld"),
        na(TO, r#"<div aria-hidden="true"><input title="Search" tabindex="-1"></div>"#, "Nicht im Accessibility-Tree")
            .variante("aria-hidden"),
        review(GN, r#"<fieldset><label><input type="radio" name="a"> A</label><label><input type="radio" name="a"> B</label></fieldset>"#, "WCAG 1.3.1/3.3.2, H71: fieldset ohne legend; REVIEW nach a11y-rules.md (auditmysite#779)")
            .anzahl(1),
        review(GN, r#"<div role="group"><label for="d">Day</label><input id="d"><label for="m">Month</label><input id="m"></div>"#, "WCAG 1.3.1, ARIA17: role=group ohne Namen")
            .variante("role=group"),
        negativ(GN, r#"<fieldset><legend>Size</legend><label><input type="radio" name="a"> A</label></fieldset>"#, "WCAG 1.3.1, H71")
            .variante("legend"),
        negativ(GN, r#"<div role="group" aria-label="Birth date"><label for="d">Day</label><input id="d"></div>"#, "WCAG 1.3.1, ARIA17")
            .variante("aria-label"),
        negativ(GN, r#"<h3 id="g">Birth date</h3><div role="group" aria-labelledby="g"><label for="d">Day</label><input id="d"></div>"#, "WCAG 1.3.1, ARIA17")
            .variante("aria-labelledby"),
        na(GN, r#"<details><summary>More</summary><p>Text</p></details>"#, "details ist keine Formulargruppe"),
        na(GN, r#"<div role="group"><button>Bold</button><button>Italic</button></div>"#, "Gruppe ohne Eingabefelder")
            .variante("Werkzeugleiste"),
        na(GN, r#"<fieldset hidden><input aria-label="A"></fieldset>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}

const CH: &str = "keyboard/click-handler-not-focusable";
const UB: &str = "links/used-as-button";
const LM: &str = "navigation/location-missing";

fn links() -> Vec<Fall> {
    vec![
        // keyboard/click-handler-not-focusable
        fail(CH, r#"<div onclick="openMenu()">Menu</div>"#, "WCAG 2.1.1, F54: nur per Zeiger auslösbar")
            .anzahl(1),
        fail(CH, r#"<a onclick="openMenu()">Menu</a>"#, "HTML: a ohne href ist Platzhalter, nicht fokussierbar; WCAG 2.1.1")
            .variante("a ohne href"),
        fail(CH, r#"<span tabindex="-1" onclick="openMenu()">Menu</span>"#, "WCAG 2.1.1: tabindex=-1 nicht in der Tabfolge")
            .variante("Grenze: tabindex=-1"),
        fail(CH, r#"<div aria-hidden="true" onclick="openMenu()">Menu</div>"#, "WCAG 2.1.1 gilt für sichtbare Funktion unabhängig vom Accessibility-Tree (Scope Rendered)")
            .variante("aria-hidden"),
        review(CH, r#"<div onclick="track()"><a href="/article">Read the article</a></div>"#, "WCAG 2.1.1: per Tab erreichbarer Link darin; REVIEW nach a11y-rules.md (auditmysite#779)")
            .variante("Karte mit Link"),
        negativ(CH, r#"<button onclick="openMenu()">Menu</button>"#, "WCAG 2.1.1: natives Bedienelement")
            .variante("button"),
        negativ(CH, r#"<div role="button" tabindex="0" onclick="openMenu()" onkeydown="key(event)">Menu</div>"#, "WCAG 2.1.1, ARIA-Muster button mit tabindex=0")
            .variante("ARIA-Rolle + tabindex=0"),
        na(CH, r#"<div>Menu</div>"#, "Kein Klick-Handler"),
        na(CH, r#"<div hidden onclick="openMenu()">Menu</div>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
        // links/used-as-button
        review(UB, r##"<a href="#" onclick="openMenu()">Menu</a>"##, "WCAG 4.1.2: Rolle link, Funktion Knopf; REVIEW nach a11y-rules.md (auditmysite#779)")
            .anzahl(1),
        review(UB, r#"<a href="javascript:void(0)" onclick="openMenu()">Menu</a>"#, "WCAG 4.1.2")
            .variante("javascript:"),
        review(UB, r#"<a href="JavaScript:void(0)" onclick="x()">Menu</a>"#, "URL-Schema ASCII-case-insensitiv (URL Standard)")
            .variante("Großschreibung"),
        review(UB, r#"<a href="" onclick="x()">Menu</a>"#, "WCAG 4.1.2: leeres href führt nirgendwohin")
            .variante("leer"),
        negativ(UB, r##"<a href="#" role="button" onclick="openMenu()">Menu</a>"##, "WCAG 4.1.2, ARIA: Rolle button ausgezeichnet")
            .variante("role=button"),
        negativ(UB, r#"<a href="/page" onclick="track()">Page</a>"#, "WCAG 4.1.2: echter Link mit Zusatzhandler")
            .variante("echtes Ziel"),
        negativ(UB, r##"<a href="#top" onclick="smooth()">Back to top</a>"##, "Fragment mit Ziel navigiert innerhalb der Seite")
            .variante("Grenze: #top"),
        na(UB, r#"<p>No links</p>"#, "Kein Link"),
        na(UB, r##"<div aria-hidden="true"><a href="#" onclick="x()" tabindex="-1">Menu</a></div>"##, "Nicht im Accessibility-Tree")
            .variante("aria-hidden"),
        // navigation/location-missing
        review(LM, r#"<nav aria-label="Main"><a href="/a">A</a><a href="/b">B</a></nav>"#, "WCAG 2.4.8 (AAA), G65/G128; REVIEW: auch Titel oder Sitemap erfüllen")
            .anzahl(1),
        review(LM, r#"<div role="navigation"><a href="/a">A</a></div>"#, "WCAG 2.4.8")
            .variante("role=navigation"),
        review(LM, r#"<nav><a href="/a" aria-current="true">A</a></nav>"#, "WAI-ARIA 1.2 aria-current: true markiert nicht die Seite")
            .variante("Grenze: aria-current=true"),
        negativ(LM, r#"<nav><a href="/a" aria-current="page">A</a><a href="/b">B</a></nav>"#, "WCAG 2.4.8, G128")
            .variante("aria-current=page"),
        negativ(LM, r#"<nav aria-label="Breadcrumb"><a href="/">Home</a> / <a href="/a">A</a></nav>"#, "WCAG 2.4.8, G65")
            .variante("Brotkrumen"),
        na(LM, r#"<p><a href="/a">A</a></p>"#, "Keine Navigation"),
        na(LM, r#"<nav hidden><a href="/a">A</a></nav>"#, "Nicht dargestellt")
            .variante("hidden-Attribut"),
    ]
}
