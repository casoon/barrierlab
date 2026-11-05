//! Gruppe „checkliste“: die Checkliste (`manual/*`, src/checklist.rs), die
//! Darstellungskonvention (`viz/*`, `display/*`, src/viz.rs) und die Regeln
//! über Stylesheets (src/stylesheet_rules.rs).
//!
//! Die Checkliste urteilt nicht: Positiv heißt hier `Untested`, einmal je
//! Seite, sobald es etwas gibt, wofür das Kriterium gilt. Negativ ist eine
//! ähnliche Seite, auf der das Kriterium nicht gilt.
//!
//! Die Konventionsregeln stützen sich auf die Konvention selbst
//! (docs/a11y/concepts/darstellungsmodi.md, Entwurf v0) und das Kriterium,
//! an dem sie hängen.

use super::super::{Fall, Mode, fail, na, negative, review, untested};

/// Eine Seite, die die Konvention vollständig erfüllt.
const VIZ_FULFILLED: &str = r#"<!DOCTYPE html><html lang="en" data-display="visual"><head><title>T</title><script src="/display-init.js"></script></head><body><header><div data-display-toggle><button type="button" aria-pressed="true">Visual</button></div></header><main><figure data-viz="interactive"><div data-viz-live></div><svg data-viz-static role="img" aria-label="Warming by region"></svg><div data-viz-text><p>Warming is strongest in the north.</p><table><tr><th>Region</th><td>North</td></tr></table><p>Source: DWD</p></div><figcaption>Warming by region</figcaption></figure><figure data-viz="chart"><div data-viz-text><p>Rising.</p><table><tr><td>1</td></tr></table></div><figcaption>Trend</figcaption></figure></main></body></html>"#;

const CONVENTION: &str = "Darstellungskonvention (docs/a11y/concepts/darstellungsmodi.md)";

pub fn cases() -> Vec<Fall> {
    let mut v = checklist();
    v.extend(convention());
    v.extend(stylesheets());
    v
}

fn checklist() -> Vec<Fall> {
    vec![
        // --- manual/media-alternatives (1.2.1, 1.2.2, 1.2.3, 1.2.5) --------
        untested(
            "manual/media-alternatives",
            r#"<video src="a.mp4" controls></video><audio src="b.mp3" controls></audio>"#,
            "WCAG 1.2.1-1.2.5: gilt für jedes Audio und Video; nur von Hand prüfbar",
        )
        .count(1),
        untested(
            "manual/media-alternatives",
            r#"<iframe src="https://player.vimeo.com/video/1" title="Talk"></iframe>"#,
            "WCAG 1.2.2: eingebettetes Video einer Plattform ist Medieninhalt",
        )
        .variant("Plattform-Player, Subdomain"),
        untested(
            "manual/media-alternatives",
            r#"<div aria-hidden="true"><video src="a.mp4" controls></video></div>"#,
            "WCAG 1.2.2: Untertitel gelten für sichtbares Video, auch wenn es der Assistenztechnik entzogen ist",
        )
        .variant("Regression: aria-hidden, sichtbar"),
        negative(
            "manual/media-alternatives",
            r#"<iframe src="https://maps.example.com/embed" title="Map"></iframe>"#,
            "WCAG 1.2: eine eingebettete Karte ist kein zeitbasiertes Medium",
        ),
        negative(
            "manual/media-alternatives",
            r#"<iframe src="https://notyoutube.com/embed/x" title="Other"></iframe>"#,
            "WCAG 1.2: fremder Host, der nur auf den Plattformnamen endet",
        )
        .variant("ähnlicher Hostname"),
        negative(
            "manual/media-alternatives",
            r#"<div hidden><video src="a.mp4"></video></div><p>Text</p>"#,
            "WCAG 1.2: nicht dargestelltes Video ist keinem Nutzer zugänglich, Kriterium gilt nicht",
        )
        .variant("hidden"),
        negative(
            "manual/media-alternatives",
            r#"<video src="a.mp4" style="display:none"></video>"#,
            "WCAG 1.2: per CSS nicht dargestellt",
        )
        .variant("display:none"),
        na(
            "manual/media-alternatives",
            r#"<p>Only text</p>"#,
            "WCAG 1.2: keine zeitbasierten Medien",
        ),
        // --- manual/image-alternatives (1.1.1) ------------------------------
        untested(
            "manual/image-alternatives",
            r#"<img src="a.png" alt="Logo"><img src="b.png" alt="">"#,
            "WCAG 1.1.1: ob Alt-Text oder alt=\"\" passt, entscheidet ein Mensch",
        )
        .count(1),
        untested(
            "manual/image-alternatives",
            r#"<div role="img" aria-label="Chart"></div>"#,
            "WCAG 1.1.1, WAI-ARIA 1.2 img: ARIA-Bild ist Nicht-Text-Inhalt",
        )
        .variant("role=img"),
        untested(
            "manual/image-alternatives",
            r#"<input type="image" src="go.png" alt="Search">"#,
            "WCAG 1.1.1: Bildschaltfläche",
        )
        .variant("input type=image"),
        negative(
            "manual/image-alternatives",
            r#"<img src="a.png" alt="" hidden><p>Text</p>"#,
            "WCAG 1.1.1: nicht dargestelltes Bild",
        )
        .variant("hidden"),
        negative(
            "manual/image-alternatives",
            r#"<input type="submit" value="Search">"#,
            "WCAG 1.1.1: Textschaltfläche ist kein Nicht-Text-Inhalt",
        ),
        na(
            "manual/image-alternatives",
            r#"<p>Only text</p>"#,
            "WCAG 1.1.1: kein Nicht-Text-Inhalt",
        ),
        // --- manual/visual-structure (1.3.1) --------------------------------
        untested(
            "manual/visual-structure",
            r#"<p>Some text</p><p>More text</p>"#,
            "WCAG 1.3.1: ob sichtbare Struktur ausgezeichnet ist, sieht nur ein Mensch",
        )
        .count(1),
        negative(
            "manual/visual-structure",
            r#"<script>var x = "text";</script><noscript>Enable JS</noscript><img src="a.png" alt="">"#,
            "WCAG 1.3.1: Skript- und noscript-Inhalt wird nicht dargestellt",
        ),
        negative(
            "manual/visual-structure",
            r#"<script>var x = "text";</script><noscript>Enable JS</noscript><img src="a.png" alt="">"#,
            "WCAG 1.3.1: Skript- und noscript-Inhalt wird nicht dargestellt — auch ohne berechnete Stile",
        )
        .mode(Mode::Semantics)
        .variant("ohne Stile"),
        negative(
            "manual/visual-structure",
            r#"<p hidden>Hidden text</p>"#,
            "WCAG 1.3.1: nicht dargestellter Text",
        )
        .variant("hidden"),
        na(
            "manual/visual-structure",
            r#"<div></div>"#,
            "WCAG 1.3.1: kein Inhalt",
        ),
        // --- manual/use-of-color (1.4.1) ------------------------------------
        untested(
            "manual/use-of-color",
            r#"<p>Read <a href="/more">more</a>.</p>"#,
            "WCAG 1.4.1, F73: Links im Text",
        ),
        untested(
            "manual/use-of-color",
            r#"<input type="text" aria-label="Name">"#,
            "WCAG 1.4.1: Pflichtfelder und Fehler nicht nur über Farbe",
        )
        .variant("Feld"),
        negative(
            "manual/use-of-color",
            r#"<a name="top">Top</a><input type="submit" value="Send"><input type="hidden" name="t">"#,
            "WCAG 1.4.1: Anker ohne href ist kein Link, Absende- und versteckte Eingaben sind keine Felder",
        ),
        na(
            "manual/use-of-color",
            r#"<p>Only text</p>"#,
            "WCAG 1.4.1: weder Links noch Felder",
        ),
        // --- manual/timing (2.2.1) ------------------------------------------
        untested(
            "manual/timing",
            r#"<input type="text" aria-label="Name">"#,
            "WCAG 2.2.1: Eingaben können einer Zeitbegrenzung unterliegen",
        ),
        untested(
            "manual/timing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta http-equiv="refresh" content="600"></head><body><p>x</p></body></html>"#,
            "WCAG 2.2.1, F40/F41: automatische Aktualisierung ist eine Zeitbegrenzung",
        )
        .variant("meta refresh, ohne Stile")
        .mode(Mode::Semantics),
        negative(
            "manual/timing",
            r#"<button type="button">Go</button>"#,
            "WCAG 2.2.1: ein Knopf allein setzt keine Zeitbegrenzung",
        ),
        na(
            "manual/timing",
            r#"<p>Only text</p>"#,
            "WCAG 2.2.1: weder Felder noch Aktualisierung",
        ),
        // --- manual/no-timing (2.2.3) ---------------------------------------
        untested(
            "manual/no-timing",
            r#"<textarea aria-label="Message"></textarea>"#,
            "WCAG 2.2.3: Aufgabe mit Eingabe",
        ),
        untested(
            "manual/no-timing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta http-equiv="Refresh" content="30"></head><body><p>x</p></body></html>"#,
            "WCAG 2.2.3: automatische Aktualisierung",
        )
        .variant("meta refresh, ohne Stile")
        .mode(Mode::Semantics),
        negative(
            "manual/no-timing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><meta http-equiv="content-type" content="text/html"></head><body><p>x</p></body></html>"#,
            "WCAG 2.2.3: andere meta-Angabe setzt keine Zeit",
        )
        .mode(Mode::Semantics),
        na(
            "manual/no-timing",
            r#"<p>Only text</p>"#,
            "WCAG 2.2.3: nichts Zeitabhängiges",
        ),
        // --- manual/error-handling (3.3.1, 3.3.3) ---------------------------
        untested(
            "manual/error-handling",
            r#"<select aria-label="Country"><option>DE</option></select>"#,
            "WCAG 3.3.1/3.3.3: Fehler nur beim Absenden mit Eingaben feststellbar",
        ),
        negative(
            "manual/error-handling",
            r#"<input type="reset" value="Reset"><input type="button" value="Go">"#,
            "WCAG 3.3.1: Knöpfe nehmen keine Eingabe an",
        ),
        na(
            "manual/error-handling",
            r#"<p>Only text</p>"#,
            "WCAG 3.3.1: keine Eingabe",
        ),
        // --- manual/help (3.3.5) --------------------------------------------
        untested(
            "manual/help",
            r#"<input type="email" aria-label="Email">"#,
            "WCAG 3.3.5: Hilfe gilt für Eingaben",
        ),
        negative(
            "manual/help",
            r#"<input type="submit" value="Send">"#,
            "WCAG 3.3.5: ein Absendeknopf ist keine Eingabe",
        ),
        na(
            "manual/help",
            r#"<p>Only text</p>"#,
            "WCAG 3.3.5: keine Eingabe",
        ),
        // --- manual/dragging-movements (2.5.7) --------------------------------------------
        untested(
            "manual/dragging-movements",
            r#"<div draggable="true">Drag me</div>"#,
            "WCAG 2.5.7: Ziehen; nur von Hand prüfbar",
        ),
        negative(
            "manual/dragging-movements",
            r#"<div draggable="false">Fixed</div>"#,
            "WCAG 2.5.7: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/dragging-movements",
            r#"<div></div>"#,
            "WCAG 2.5.7: kein Inhalt",
        ),
        // --- manual/consistent-help (3.2.6) --------------------------------------------
        untested(
            "manual/consistent-help",
            r#"<p>Need help? Call us.</p>"#,
            "WCAG 3.2.6: Hilfe, die auf Seiten steht; nur von Hand prüfbar",
        ),
        negative(
            "manual/consistent-help",
            r#"<p style="display:none">Hidden</p><img src="a.png" alt="">"#,
            "WCAG 3.2.6: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/consistent-help",
            r#"<div></div>"#,
            "WCAG 3.2.6: kein Inhalt",
        ),
        // --- manual/keyboard-trap (2.1.2) --------------------------------------------
        untested(
            "manual/keyboard-trap",
            r#"<a href="/x">Link</a>"#,
            "WCAG 2.1.2: Bedienelemente; nur von Hand prüfbar",
        ),
        negative(
            "manual/keyboard-trap",
            r#"<script>var x = 1;</script>"#,
            "WCAG 2.1.2: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/keyboard-trap",
            r#"<div></div>"#,
            "WCAG 2.1.2: kein Inhalt",
        ),
        // --- manual/character-shortcuts (2.1.4) --------------------------------------------
        untested(
            "manual/character-shortcuts",
            r#"<div onkeydown="run(event)">App</div>"#,
            "WCAG 2.1.4: Tastenkürzel; nur von Hand prüfbar",
        ),
        negative(
            "manual/character-shortcuts",
            r#"<p>Text</p><div onclick="go()">Click</div>"#,
            "WCAG 2.1.4: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/character-shortcuts",
            r#"<div></div>"#,
            "WCAG 2.1.4: kein Inhalt",
        ),
        // --- manual/error-prevention (3.3.4) --------------------------------------------
        untested(
            "manual/error-prevention",
            r#"<input type="text" aria-label="Amount">"#,
            "WCAG 3.3.4: Eingaben, die rechtliche oder finanzielle Wirkung haben können; nur von Hand prüfbar",
        ),
        negative(
            "manual/error-prevention",
            r#"<input type="submit" value="Send">"#,
            "WCAG 3.3.4: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/error-prevention",
            r#"<div></div>"#,
            "WCAG 3.3.4: kein Inhalt",
        ),
        // --- manual/pointer-cancellation (2.5.2) --------------------------------------------
        untested(
            "manual/pointer-cancellation",
            r#"<button onmousedown="go()">Go</button>"#,
            "WCAG 2.5.2: Auslösung beim Herunterdrücken; nur von Hand prüfbar",
        ),
        negative(
            "manual/pointer-cancellation",
            r#"<button onclick="go()">Go</button>"#,
            "WCAG 2.5.2: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/pointer-cancellation",
            r#"<div></div>"#,
            "WCAG 2.5.2: kein Inhalt",
        ),
        // --- manual/consistent-navigation (3.2.3) --------------------------------------------
        untested(
            "manual/consistent-navigation",
            r#"<nav><a href="/a">A</a><a href="/b">B</a></nav>"#,
            "WCAG 3.2.3: wiederkehrende Navigation; nur von Hand prüfbar",
        ),
        negative(
            "manual/consistent-navigation",
            r#"<p>Text</p>"#,
            "WCAG 3.2.3: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/consistent-navigation",
            r#"<div></div>"#,
            "WCAG 3.2.3: kein Inhalt",
        ),
        // --- manual/consistent-identification (3.2.4) --------------------------------------------
        untested(
            "manual/consistent-identification",
            r#"<a href="/search">Search</a>"#,
            "WCAG 3.2.4: gleiche Funktion, gleicher Name; nur von Hand prüfbar",
        ),
        negative(
            "manual/consistent-identification",
            r#"<p>Text</p>"#,
            "WCAG 3.2.4: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/consistent-identification",
            r#"<div></div>"#,
            "WCAG 3.2.4: kein Inhalt",
        ),
        // --- manual/sensory-characteristics (1.3.3) --------------------------------------------
        untested(
            "manual/sensory-characteristics",
            r#"<p>Press the round button.</p>"#,
            "WCAG 1.3.3: Anweisungen im Text; nur von Hand prüfbar",
        ),
        negative(
            "manual/sensory-characteristics",
            r#"<div></div>"#,
            "WCAG 1.3.3: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/sensory-characteristics",
            r#"<div></div>"#,
            "WCAG 1.3.3: kein Inhalt",
        ),
        // --- manual/multiple-ways (2.4.5) --------------------------------------------
        untested(
            "manual/multiple-ways",
            r#"<a href="/a">A</a>"#,
            "WCAG 2.4.5: mehrere Wege; nur von Hand prüfbar",
        ),
        negative(
            "manual/multiple-ways",
            r#"<p>Text</p>"#,
            "WCAG 2.4.5: ähnliche Seite ohne Anlass, das Kriterium gilt nicht",
        ),
        na(
            "manual/multiple-ways",
            r#"<div></div>"#,
            "WCAG 2.4.5: kein Inhalt",
        ),
        // --- manual/authentication (3.3.8) ----------------------------------
        untested(
            "manual/authentication",
            r#"<input type="text" aria-label="User"><input type="password" aria-label="Password">"#,
            "WCAG 3.3.8: Anmeldung mit Passwort",
        ),
        untested(
            "manual/authentication",
            r#"<div class="g-recaptcha"></div>"#,
            "WCAG 3.3.8: Captcha ist ein möglicher kognitiver Test",
        )
        .variant("Captcha"),
        untested(
            "manual/authentication",
            r#"<div class="cf-turnstile"></div>"#,
            "WCAG 3.3.8: Captcha-Dienst",
        )
        .variant("Turnstile"),
        negative(
            "manual/authentication",
            r#"<input type="text" aria-label="Search"><input type="search" aria-label="Find">"#,
            "WCAG 3.3.8: Suchfelder sind keine Anmeldung",
        ),
        na(
            "manual/authentication",
            r#"<p>Only text</p>"#,
            "WCAG 3.3.8: kein Anmeldevorgang",
        ),
        // --- manual/re-authenticating (2.2.5) -------------------------------
        untested(
            "manual/re-authenticating",
            r#"<input type="password" aria-label="Password">"#,
            "WCAG 2.2.5: Sitzung mit Anmeldung",
        ),
        negative(
            "manual/re-authenticating",
            r#"<input type="text" aria-label="Name">"#,
            "WCAG 2.2.5: Formular ohne Anmeldung",
        ),
        na(
            "manual/re-authenticating",
            r#"<p>Only text</p>"#,
            "WCAG 2.2.5: keine Anmeldung",
        ),
        // --- manual/pointer-gestures (2.5.1) --------------------------------
        untested(
            "manual/pointer-gestures",
            r#"<div class="swiper-container"><div class="swiper-slide">1</div></div>"#,
            "WCAG 2.5.1: Karussell mit Wischgeste",
        )
        .count(1),
        untested(
            "manual/pointer-gestures",
            r#"<div draggable="true">Drag me</div>"#,
            "WCAG 2.5.1: Ziehen",
        )
        .variant("draggable"),
        untested(
            "manual/pointer-gestures",
            r#"<div ontouchstart="start()">Map</div>"#,
            "WCAG 2.5.1: Touch-Handler",
        )
        .variant("ontouchstart"),
        negative(
            "manual/pointer-gestures",
            r#"<div class="swiperight" draggable="false">x</div>"#,
            "WCAG 2.5.1: kein Gestenmuster (Klasse nur ähnlich, draggable=false)",
        ),
        na(
            "manual/pointer-gestures",
            r#"<p>Only text</p>"#,
            "WCAG 2.5.1: keine Gesten",
        ),
        // --- manual/interruptions (2.2.4) -----------------------------------
        untested(
            "manual/interruptions",
            r#"<div aria-live="polite">Saved</div>"#,
            "WCAG 2.2.4: Live-Region aktualisiert sich",
        ),
        untested(
            "manual/interruptions",
            r#"<div role="alert">Error</div>"#,
            "WCAG 2.2.4, WAI-ARIA 1.2 alert",
        )
        .variant("role=alert"),
        untested(
            "manual/interruptions",
            r#"<dialog open><p>Newsletter?</p></dialog>"#,
            "WCAG 2.2.4: Einblendung",
        )
        .variant("dialog"),
        negative(
            "manual/interruptions",
            r#"<div aria-live="off">Ticker</div><div role="note">Note</div>"#,
            "WAI-ARIA 1.2 aria-live=off: keine Ansage, keine Unterbrechung",
        ),
        na(
            "manual/interruptions",
            r#"<p>Only text</p>"#,
            "WCAG 2.2.4: keine Aktualisierungen",
        ),
        // --- manual/unusual-words (3.1.3) -----------------------------------
        untested(
            "manual/unusual-words",
            r#"<p>The albedo changes.</p>"#,
            "WCAG 3.1.3: gilt für Text",
        ),
        negative(
            "manual/unusual-words",
            r#"<p style="display:none">Hidden</p><img src="a.png" alt="">"#,
            "WCAG 3.1.3: kein dargestellter Text",
        ),
        na(
            "manual/unusual-words",
            r#"<div></div>"#,
            "WCAG 3.1.3: kein Text",
        ),
        // --- manual/motion-actuation (2.5.4) --------------------------------
        untested(
            "manual/motion-actuation",
            r#"<p>x</p><script>window.addEventListener("devicemotion", shake);</script>"#,
            "WCAG 2.5.4: Funktion über Gerätebewegung",
        ),
        untested(
            "manual/motion-actuation",
            r#"<script>addEventListener("deviceorientation", tilt)</script>"#,
            "WCAG 2.5.4: Neigung",
        )
        .variant("deviceorientation"),
        negative(
            "manual/motion-actuation",
            r#"<script>addEventListener("mousemove", follow)</script><script src="motion.js"></script>"#,
            "WCAG 2.5.4: Zeigerbewegung ist keine Gerätebewegung; externes Skript statisch nicht lesbar (dokumentierte Grenze)",
        ),
        na(
            "manual/motion-actuation",
            r#"<p>No script</p>"#,
            "WCAG 2.5.4: kein Skript",
        ),
    ]
}

fn convention() -> Vec<Fall> {
    vec![
        // --- viz/text-missing (1.1.1) ---------------------------------------
        fail(
            "viz/text-missing",
            r#"<figure data-viz="diagram"><img src="a.png" alt="x"><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: [data-viz-text] Pflicht; WCAG 1.1.1",
        )
        .count(1),
        fail(
            "viz/text-missing",
            r#"<figure data-viz="diagram"><div data-viz-text> <p> </p></div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        )
        .variant("leere Textschicht"),
        fail(
            "viz/text-missing",
            r#"<figure data-viz="chart"><div class="viz-desc">Rising.</div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        )
        .variant("Beschreibung ohne Marke"),
        negative(
            "viz/text-missing",
            r#"<figure data-viz="diagram"><div data-viz-text><p><span>Rising.</span></p></div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        ),
        negative("viz/text-missing", VIZ_FULFILLED, CONVENTION).variant("vollständige Seite"),
        na(
            "viz/text-missing",
            r#"<figure><img src="a.png" alt="A"><figcaption>Plain</figcaption></figure><div data-viz="chart"></div>"#,
            "Darstellungskonvention: nur figure[data-viz] trägt die Prüfungen",
        ),
        // --- display/text-hidden (1.1.1) ------------------------------------
        fail(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><div data-viz-text hidden><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: Textschicht nie per hidden verborgen; WCAG 1.1.1",
        ),
        fail(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><div aria-hidden="true"><div data-viz-text><p>Rising.</p></div></div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        )
        .variant("aria-hidden am Vorfahren"),
        fail(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><svg role="img" aria-label="X" aria-describedby="t"></svg><div id="t" data-viz-text inert><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "HTML inert: inerte Inhalte dürfen ganz fehlen, auch referenziert",
        )
        .variant("inert, referenziert"),
        fail(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><svg hidden aria-describedby="t"></svg><div id="t" data-viz-text hidden><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "accname 1.2: nur ein nicht verborgener Verweis holt die Beschreibung",
        )
        .variant("nur von Verborgenem referenziert"),
        review(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><svg role="img" aria-label="X" aria-describedby="t"></svg><div id="t" data-viz-text hidden><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "accname 1.2 / ARIA15: aria-describedby nimmt verborgene Ziele; Struktur geht verloren (auditmysite#704)",
        )
        .variant("aria-describedby"),
        review(
            "display/text-hidden",
            r#"<figure data-viz="diagram" aria-details="t"><div id="t" data-viz-text aria-hidden="true"><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "WAI-ARIA 1.2 aria-details (auditmysite#704)",
        )
        .variant("aria-details an der figure"),
        negative(
            "display/text-hidden",
            r#"<figure data-viz="diagram"><div data-viz-text style="position:absolute;clip:rect(0 0 0 0)" aria-hidden="false"><p>Rising.</p></div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: nur visuell ausgeblendet ist die erfüllte Form",
        ),
        negative(
            "display/text-hidden",
            r#"<section hidden><figure data-viz="diagram"><div data-viz-text hidden><p>Rising.</p></div><figcaption>X</figcaption></figure></section><p>x</p>"#,
            "auditmysite#725: mit der Grafik verborgen, keine Grafik ohne Text",
        )
        .variant("inaktives Panel"),
        na(
            "display/text-hidden",
            r#"<div data-viz-text hidden><p>Orphan.</p></div>"#,
            "Darstellungskonvention: Textschicht außerhalb figure[data-viz]",
        ),
        // --- viz/caption-missing (1.1.1) ------------------------------------
        fail(
            "viz/caption-missing",
            r#"<figure data-viz="diagram"><div data-viz-text>Rising.</div></figure>"#,
            "Darstellungskonvention: <figcaption> immer; HTML figcaption",
        ),
        fail(
            "viz/caption-missing",
            r#"<figure data-viz="diagram"><div data-viz-text>Rising.</div><div><figcaption>Nested</figcaption></div></figure>"#,
            "HTML: figcaption beschriftet nur die figure, deren Kind es ist",
        )
        .variant("verschachteltes figcaption"),
        negative(
            "viz/caption-missing",
            r#"<figure data-viz="diagram"><figcaption>Trend</figcaption><div data-viz-text>Rising.</div></figure>"#,
            "HTML figcaption als erstes Kind",
        ),
        na(
            "viz/caption-missing",
            r#"<figure><img src="a.png" alt="A"></figure>"#,
            "Darstellungskonvention: figure ohne data-viz",
        ),
        // --- viz/static-missing (2.2.2) -------------------------------------
        fail(
            "viz/static-missing",
            r#"<figure data-viz="interactive"><div data-viz-live></div><div data-viz-text>x</div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: Standbild Pflicht bei interactive; WCAG 2.2.2",
        ),
        fail(
            "viz/static-missing",
            r#"<figure data-viz="3d"><canvas></canvas><div data-viz-text>x</div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        )
        .variant("3d"),
        negative(
            "viz/static-missing",
            r#"<figure data-viz="3d"><canvas data-viz-live></canvas><img data-viz-static src="s.png" alt=""><div data-viz-text>x</div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        ),
        negative(
            "viz/static-missing",
            r#"<figure data-viz="diagram"><svg></svg><div data-viz-text>x</div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: Standbild nur bei 3d und interactive",
        )
        .variant("diagram"),
        na(
            "viz/static-missing",
            r#"<canvas></canvas>"#,
            "Darstellungskonvention: Medien außerhalb der Konvention prüft keine Regel",
        ),
        // --- viz/table-missing (1.3.1) --------------------------------------
        review(
            "viz/table-missing",
            r#"<figure data-viz="chart"><div data-viz-text><p>Rising from 1 to 5.</p></div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: Werte bei chart als <table>; WCAG 1.3.1, REVIEW (Satz kann genügen)",
        ),
        negative(
            "viz/table-missing",
            r#"<figure data-viz="chart"><div data-viz-text><table><tr><th>Year</th><td>5</td></tr></table></div><figcaption>X</figcaption></figure>"#,
            CONVENTION,
        ),
        negative(
            "viz/table-missing",
            r#"<figure data-viz="diagram"><div data-viz-text><p>Flow.</p></div><figcaption>X</figcaption></figure>"#,
            "Darstellungskonvention: Tabelle nur bei chart",
        )
        .variant("diagram"),
        na("viz/table-missing", r#"<p>No chart</p>"#, CONVENTION),
        // --- display/toggle-missing (2.2.2) ---------------------------------
        fail(
            "display/toggle-missing",
            r#"<figure data-viz="diagram"><div data-viz-text>a</div><figcaption>A</figcaption></figure><figure data-viz="chart"><div data-viz-text>b</div><figcaption>B</figcaption></figure>"#,
            "Darstellungskonvention: Umschalter [data-display-toggle]; einmal je Seite",
        )
        .count(1),
        fail(
            "display/toggle-missing",
            r#"<figure data-viz="image"><img src="a.png" alt="A"><div data-viz-text>a</div><figcaption>A</figcaption></figure>"#,
            "Darstellungskonvention: schon eine Visualisierung braucht den Umschalter",
        )
        .variant("Grenze: eine Visualisierung"),
        negative("display/toggle-missing", VIZ_FULFILLED, CONVENTION),
        na(
            "display/toggle-missing",
            r#"<div data-display-toggle><button type="button">Text</button></div><p>No viz</p>"#,
            "Darstellungskonvention: Umschalter ohne Visualisierung",
        ),
        // --- display/init-missing (2.2.2) -----------------------------------
        review(
            "display/init-missing",
            r#"<figure data-viz="diagram"><div data-viz-text>a</div><figcaption>A</figcaption></figure>"#,
            "Darstellungskonvention: Modus vor dem ersten Zeichnen; REVIEW, weil nur die Abwesenheit statisch feststeht",
        ),
        review(
            "display/init-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><script src="/init.js" defer></script><script type="module" src="/m.js"></script><script type="application/json">{}</script></head><body><figure data-viz="diagram"><div data-viz-text>a</div><figcaption>A</figcaption></figure><script src="/late.js"></script></body></html>"#,
            "HTML scripting: defer, module und Datenblock halten das Parsen nicht an; Skript im body zu spät",
        )
        .variant("nicht blockierende Skripte"),
        review(
            "display/init-missing",
            r#"<div data-display-toggle><button type="button">Text</button></div>"#,
            "Darstellungskonvention: Umschalter allein benutzt die Konvention",
        )
        .variant("nur Umschalter"),
        negative(
            "display/init-missing",
            r#"<!DOCTYPE html><html lang="en" data-display="calm"><head><title>T</title></head><body><figure data-viz="diagram"><div data-viz-text>a</div><figcaption>A</figcaption></figure></body></html>"#,
            CONVENTION,
        )
        .variant("data-display im Markup"),
        negative(
            "display/init-missing",
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title><script async>document.documentElement.dataset.display = "visual";</script></head><body><figure data-viz="diagram"><div data-viz-text>a</div><figcaption>A</figcaption></figure></body></html>"#,
            "HTML: async wirkt an Inline-Skripten nicht, das Skript blockiert",
        )
        .variant("Inline-Skript in head"),
        negative("display/init-missing", VIZ_FULFILLED, CONVENTION).variant("klassisches Skript"),
        na("display/init-missing", r#"<p>No convention</p>"#, CONVENTION),
    ]
}

fn stylesheets() -> Vec<Fall> {
    vec![
        // --- focus/outline-removed (2.4.7, F78) -----------------------------
        fail(
            "focus/outline-removed",
            r#"<style>a:focus { outline: none }</style><a href="/">x</a>"#,
            "WCAG 2.4.7, F78: Fokusrahmen ohne Ersatz entfernt",
        )
        .count(1),
        fail(
            "focus/outline-removed",
            r#"<style>button:focus { outline: 0 }</style><button>x</button>"#,
            "F78",
        )
        .variant("outline: 0"),
        fail(
            "focus/outline-removed",
            r#"<style>button:focus { outline-width: 0px }</style><button>x</button>"#,
            "F78",
        )
        .variant("outline-width: 0px"),
        fail(
            "focus/outline-removed",
            r#"<style>a:focus, a:focus-visible { outline: none }</style><a href="/">x</a>"#,
            "F78: ein :focus-visible, das den Rahmen selbst entfernt, ist kein Ersatz",
        )
        .variant("Regression: :focus-visible entfernt mit"),
        fail(
            "focus/outline-removed",
            r#"<style>*:focus-visible { outline: none }</style><a href="/">x</a>"#,
            "F78: entfernt den Fokushinweis gerade für die Tastatur",
        )
        .variant("Regression: nur :focus-visible"),
        negative(
            "focus/outline-removed",
            r#"<style>a:focus { outline: 0 } a:focus-visible { outline: 2px solid #005fcc }</style><a href="/">x</a>"#,
            "WCAG 2.4.7, C40: Ersatz unter :focus-visible",
        ),
        negative(
            "focus/outline-removed",
            r#"<style>a:focus { outline: none } button:focus { box-shadow: 0 0 0 3px #005fcc }</style><a href="/">x</a>"#,
            "WCAG 2.4.7: Ersatz per box-shadow (seitenweit, dokumentiert)",
        )
        .variant("box-shadow"),
        negative(
            "focus/outline-removed",
            r#"<style>.btn:focus:not(:focus-visible) { outline: none }</style><button class="btn">x</button>"#,
            "CSS Selectors 4 :focus-visible: der Hinweis des Browsers unter :focus-visible bleibt",
        )
        .variant(":focus:not(:focus-visible)"),
        negative(
            "focus/outline-removed",
            r#"<style>nav:focus-within { outline: none } a:focus { outline: 3px solid }</style><a href="/">x</a>"#,
            "WCAG 2.4.7: :focus-within betrifft den Container, nicht den Fokushinweis",
        ),
        na(
            "focus/outline-removed",
            r#"<style>p { color: #222 }</style><p>x</p>"#,
            "WCAG 2.4.7: kein Fokusstil",
        ),
        // --- motion/reduced-motion-ignored (2.3.3) --------------------------
        review(
            "motion/reduced-motion-ignored",
            r#"<style>.card { transition: transform 0.3s ease } .card:hover { transform: translateY(-4px) }</style><div class="card">x</div>"#,
            "WCAG 2.3.3, C39: Bewegung ohne prefers-reduced-motion; REVIEW (dokumentiert)",
        ),
        review(
            "motion/reduced-motion-ignored",
            r#"<style>.in { animation: slide 1s } @keyframes slide { from { transform: translateX(-100%) } to { transform: none } }</style><div class="in">x</div>"#,
            "WCAG 2.3.3: Keyframe-Animation mit Bewegung",
        )
        .variant("animation"),
        review(
            "motion/reduced-motion-ignored",
            r#"<style>.card { transition: transform 0.3s } @media (prefers-reduced-motion: reduce) { .card { transition-duration: 51ms } }</style><div class="card">x</div>"#,
            "WCAG 2.3.3: 51 ms laufen noch; Schwelle der Regel 50 ms (übliche Resets 0,01 ms/1 ms)",
        )
        .variant("Grenze 51 ms"),
        negative(
            "motion/reduced-motion-ignored",
            r#"<style>.card { transition: transform 0.3s } @media (prefers-reduced-motion: reduce) { .card { transition-duration: 50ms } }</style><div class="card">x</div>"#,
            "WCAG 2.3.3, C39: abgeschaltet bis 50 ms",
        )
        .variant("Grenze 50 ms"),
        negative(
            "motion/reduced-motion-ignored",
            r#"<style>.card { transition: transform 0.3s } @media (prefers-reduced-motion: reduce) { .card { transition: none } }</style><div class="card">x</div>"#,
            "WCAG 2.3.3, C39",
        ),
        negative(
            "motion/reduced-motion-ignored",
            r#"<style>.card { transition: color 0.3s } .card:hover { color: red }</style><div class="card">x</div>"#,
            "Understanding 2.3.3: Farbwechsel ist keine Bewegung",
        )
        .variant("Farbe"),
        negative(
            "motion/reduced-motion-ignored",
            r#"<style>@media (prefers-reduced-motion: no-preference) { .card { transition: transform 0.3s } }</style><div class="card">x</div>"#,
            "WCAG 2.3.3, C39: Bewegung nur ohne Einschränkungswunsch",
        )
        .variant("no-preference"),
        negative(
            "motion/reduced-motion-ignored",
            r#"<style>.gone { transition: transform 0.3s }</style><div class="card">x</div><div class="gone" hidden>y</div>"#,
            "WCAG 2.3.3: Regel trifft kein dargestelltes Element",
        )
        .variant("kein Treffer, hidden"),
        na(
            "motion/reduced-motion-ignored",
            r#"<p>No styles</p>"#,
            "WCAG 2.3.3: keine Stylesheets",
        ),
        // --- orientation/content-hidden (1.3.4) -----------------------------
        negative(
            "orientation/content-hidden",
            r#"<style>@media (orientation: portrait) { li::marker { display: none } }</style><ul><li>Item</li></ul>"#,
            "WCAG 1.3.4: ein Pseudo-Element (Listenzeichen) ist kein Inhalt, den die Ausrichtung sperrt",
        )
        .variant("Pseudo-Element ::marker"),
        review(
            "orientation/content-hidden",
            r#"<style>@media (orientation: landscape) { main { display: none } }</style><main>x</main>"#,
            "WCAG 1.3.4: Inhalt nur in einer Ausrichtung; REVIEW (dokumentiert, Umbau meist zulässig)",
        ),
        review(
            "orientation/content-hidden",
            r#"<style>@media (orientation: portrait) { .app { transform: rotate(90deg) } }</style><div class="app">x</div>"#,
            "WCAG 1.3.4: Drehen erzwingt eine Ausrichtung",
        )
        .variant("rotate"),
        negative(
            "orientation/content-hidden",
            r#"<style>@media (orientation: landscape) { body::before { display: none } main { flex-direction: row } }</style><main>x</main>"#,
            "WCAG 1.3.4: Pseudo-Element ohne Inhalt, Umbau des Layouts",
        ),
        negative(
            "orientation/content-hidden",
            r#"<style>@media (min-width: 600px) { main { display: none } }</style><main>x</main>"#,
            "WCAG 1.3.4: keine Ausrichtungsabfrage",
        )
        .variant("Breite statt Ausrichtung"),
        negative(
            "orientation/content-hidden",
            r#"<style>@media (orientation: landscape) { .nothing { display: none } }</style><main>x</main>"#,
            "WCAG 1.3.4: trifft kein Element",
        )
        .variant("kein Treffer"),
        na(
            "orientation/content-hidden",
            r#"<main>x</main>"#,
            "WCAG 1.3.4: keine Stylesheets",
        ),
        // --- text/justified (1.4.8) -----------------------------------------
        review(
            "text/justified",
            r#"<style>p { text-align: justify }</style><p>Running text.</p>"#,
            "WCAG 1.4.8: Text nicht im Blocksatz; REVIEW (AAA, dokumentiert)",
        ),
        review(
            "text/justified",
            r#"<style>article { text-align: justify }</style><article><p>Running text.</p></article>"#,
            "WCAG 1.4.8: vererbt",
        )
        .variant("vererbt"),
        negative(
            "text/justified",
            r#"<style>p { text-align: left }</style><p>Running text.</p>"#,
            "WCAG 1.4.8, C19: links ausgerichtet",
        ),
        negative(
            "text/justified",
            r#"<style>.j { text-align: justify }</style><p class="j" hidden>Hidden.</p><p>Shown.</p>"#,
            "WCAG 1.4.8: nur dargestellte Absätze",
        )
        .variant("hidden"),
        na(
            "text/justified",
            r#"<style>h1 { text-align: justify }</style><h1>Title</h1>"#,
            "WCAG 1.4.8: kein Fließtextabsatz",
        ),
        // --- text/line-height-tight (1.4.8) ---------------------------------
        review(
            "text/line-height-tight",
            r#"<style>p { line-height: 1.2 }</style><p>Running text.</p>"#,
            "WCAG 1.4.8: Zeilenabstand mindestens 1,5; REVIEW (AAA, dokumentiert)",
        ),
        review(
            "text/line-height-tight",
            r#"<style>p { line-height: 1.49 }</style><p>Running text.</p>"#,
            "WCAG 1.4.8: 1,49 < 1,5",
        )
        .variant("Grenze 1,49"),
        review(
            "text/line-height-tight",
            r#"<style>p { line-height: 149% }</style><p>Running text.</p>"#,
            "WCAG 1.4.8",
        )
        .variant("Grenze 149 %"),
        review(
            "text/line-height-tight",
            r#"<style>body { line-height: 1.4em }</style><p>Running text.</p>"#,
            "WCAG 1.4.8: vererbt, em",
        )
        .variant("em, vererbt"),
        negative(
            "text/line-height-tight",
            r#"<style>p { line-height: 1.5 }</style><p>Running text.</p>"#,
            "WCAG 1.4.8, C21: genau 1,5 genügt",
        )
        .variant("Grenze 1,5"),
        negative(
            "text/line-height-tight",
            r#"<style>p { line-height: 150% }</style><p>Running text.</p>"#,
            "WCAG 1.4.8, C21",
        )
        .variant("Grenze 150 %"),
        negative(
            "text/line-height-tight",
            r#"<style>html { line-height: 1.15 } p { line-height: 1.6 }</style><p>Running text.</p>"#,
            "WCAG 1.4.8: am Absatz kommt 1,6 an",
        )
        .variant("näherer Wert gewinnt"),
        na(
            "text/line-height-tight",
            r#"<style>h1 { line-height: 1 }</style><h1>Title</h1>"#,
            "WCAG 1.4.8: kein Fließtextabsatz",
        ),
    ]
}
