//! Die statischen Regeln der Darstellungskonvention (Entwurf v0,
//! casoon/barrierlab#22): `figure[data-viz]` mit Text-, Standbild- und
//! Live-Schicht, `html[data-display=visual|calm|text]` mit Umschalter.
//!
//! **Warum hier:** Die Regeln brauchen den DOM — Vorfahren, Nachfahren,
//! Verweise per `aria-describedby` —, also `a11y-dom` und damit dieses Crate,
//! nicht `web-checks`. Sie lesen nur Attribute und Struktur und sind deshalb
//! Tier 1.
//!
//! Die Konvention ist keine WCAG-Anforderung. Jede Regel hängt am nächsten
//! Erfolgskriterium (1.1.1, 1.3.1, 2.2.2) wie in auditmysite
//! (`src/wcag/rules/display_modes.rs`) und trägt das Schlagwort
//! `best-practice`. Die Kennungen `display/*` sind dieselben wie dort; was
//! auditmysite davon an der laufenden Seite misst (berechnete Sichtbarkeit,
//! der Zeitpunkt von `data-display`, die Darstellung im Textmodus), bleibt
//! dort.
//!
//! **Nur mit Opt-in.** Eine Seite, die die Konvention nicht benutzt, löst
//! keine dieser Regeln aus: Alle hängen an `figure[data-viz]`, nur
//! `viz/orphan-media` an einem beliebigen Konventionsmerkmal (siehe
//! [`nutzt_konvention`]). Sonst wäre jedes `<canvas>` und jedes `<video>`
//! im Web ein Befund.
//!
//! Beleg ist die Referenzumsetzung Geographia (`web-geographia`, gebaute
//! Seiten unter `apps/*/dist`); die Fundstelle steht je Regel.

use a11y_dom::{Document, Node, NodeId, ancestors, descendants, elements, subtree_text};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick};
use crate::structure::explizite_rolle;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Die Konventionsregeln haben keine rechtliche Tragweite (auditmysite#704,
/// dort `is_convention_rule`): Sie sind Best Practice, kein WCAG-Verstoß.
const SCHLAGWORTE: [&str; 1] = ["best-practice"];

fn ist_viz<'a, N: Node<'a>>(n: N) -> bool {
    n.is_element("figure") && n.has_attr("data-viz")
}

fn visualisierungen<D: Document>(doc: &D) -> impl Iterator<Item = D::N<'_>> {
    elements(doc).filter(|n| ist_viz(*n))
}

/// Die Art der Visualisierung aus `data-viz`, klein geschrieben.
fn art<'a, N: Node<'a>>(n: N) -> String {
    n.attr("data-viz")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn aria_hidden<'a, N: Node<'a>>(n: N) -> bool {
    n.attr("aria-hidden")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Ob die Seite die Konvention benutzt: ein Element mit `data-viz`, ein
/// `[data-display-toggle]` oder `html[data-display]`.
///
/// `data-viz` zählt an jedem Element, nicht nur an `<figure>`: Geographia
/// trägt es auch an `<div>` (die Zeitachse in
/// `atmosphere/chapters/1850-to-today/`, der 3D-Erkunder in `space/explore/`).
fn nutzt_konvention<D: Document>(doc: &D) -> bool {
    doc.root().has_attr("data-display")
        || elements(doc).any(|n| n.has_attr("data-viz") || n.has_attr("data-display-toggle"))
}

// --- viz/text-missing --------------------------------------------------------

/// 1.1.1: Eine Visualisierung ohne Textschicht, oder mit einer, die keinen
/// Text enthält.
///
/// Geprüft wird nur, **dass** Text da ist, nicht ob er die Kernaussage trifft
/// — das kann keine Maschine entscheiden. Ob die Schicht erreichbar ist, prüft
/// `display/text-hidden`. Die erfüllte Form ist `VizText.astro` in Geographia
/// (`<div class="viz-text" data-viz-text>` mit Titel und Absätzen). Beleg für
/// den Befund: Die Startseite (`home/index.html`) beschreibt ihre Diagramme
/// in `<span class="viz-desc">` bzw. `<div class="viz-desc">` ohne die Marke
/// (`EarthNow.astro`, `LayerTrack.astro`), und „Build Earth 2.0" in
/// `space/solar-system/settlement/` hat gar keine Textschicht
/// (`EarthTwoFigure.astro`).
pub(crate) fn text<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for fig in visualisierungen(doc) {
        let schichten: Vec<_> = descendants(fig)
            .filter(|n| n.has_attr("data-viz-text"))
            .collect();
        if schichten
            .iter()
            .any(|s| !subtree_text(*s).trim().is_empty())
        {
            continue;
        }
        let message = if schichten.is_empty() {
            pick!(
                locale,
                "This visualisation (figure[data-viz]) has no text layer marked \
                 [data-viz-text]; text mode has nothing to show in place of the graphic.",
                "Die Visualisierung (figure[data-viz]) hat keine mit [data-viz-text] markierte \
                 Textschicht; der Textmodus hat nichts, was statt der Grafik erscheint.",
            )
        } else {
            pick!(
                locale,
                "This visualisation's text layer [data-viz-text] is empty; text mode has \
                 nothing to show in place of the graphic.",
                "Die Textschicht [data-viz-text] der Visualisierung ist leer; der Textmodus hat \
                 nichts, was statt der Grafik erscheint.",
            )
        };
        out.push(
            Finding::fail("viz/text-missing", message)
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .with_tags(SCHLAGWORTE)
                .at(at(fig.id())),
        );
    }
}

// --- display/text-hidden ------------------------------------------------------

/// Was ein Element samt Vorfahren vor der Assistenztechnik verbirgt: das
/// nächstgelegene `hidden`, `aria-hidden="true"` oder `inert`.
fn verborgen_durch<'a, N: Node<'a>>(n: N) -> Option<&'static str> {
    std::iter::once(n).chain(ancestors(n)).find_map(|a| {
        if a.has_attr("hidden") {
            Some("hidden")
        } else if aria_hidden(a) {
            Some("aria-hidden")
        } else if a.has_attr("inert") {
            Some("inert")
        } else {
            None
        }
    })
}

fn mit_vorfahren<'a, N: Node<'a>>(n: N) -> impl Iterator<Item = N> + 'a {
    std::iter::once(n).chain(ancestors(n))
}

/// Ob ein nicht verborgenes Element derselben Visualisierung (sie selbst
/// eingeschlossen) die Textschicht per `aria-describedby` oder `aria-details`
/// referenziert.
fn beschrieben_in<'a, N: Node<'a>>(fig: N, text: N) -> bool {
    let Some(id) = text.attr("id").filter(|i| !i.is_empty()) else {
        return false;
    };
    std::iter::once(fig).chain(descendants(fig)).any(|r| {
        r != text
            && !ancestors(r).any(|a| a == text)
            && verborgen_durch(r).is_none()
            && ["aria-describedby", "aria-details"]
                .iter()
                .filter_map(|a| r.attr(a))
                .any(|v| v.split_ascii_whitespace().any(|t| t == id))
    })
}

/// 1.1.1: Die Textschicht ist per `hidden`, `aria-hidden="true"` oder `inert`
/// (an ihr oder einem Vorfahren) vor der Assistenztechnik verborgen.
///
/// Der Attribut-Teil von auditmysites `display/text-hidden`, im Entwurf
/// `viz/text-hidden`; die Kennung folgt auditmysite. Mit den Fällen aus
/// auditmysite#704:
///
/// - Referenziert ein nicht verborgenes Element derselben Visualisierung die
///   Schicht per `aria-describedby` oder `aria-details`, kommt die Aussage als
///   Beschreibung an — accname 1.2 nimmt direkt referenzierte Knoten auch
///   verborgen (ARIA15). Dann `REVIEW`, niedrig: Die Struktur (Tabelle,
///   Liste) geht dabei verloren.
/// - `inert` bleibt ein Verstoß, auch referenziert: HTML erlaubt, inerte
///   Inhalte ganz aus dem Accessibility-Tree zu nehmen.
/// - Ist die Visualisierung selbst per `hidden` ausgeblendet (inaktives
///   Panel), fehlt mit ihr auch die Grafik — kein Befund (auditmysite#725).
///
/// Beleg ist der Fall aus auditmysite#704 (geographia.eu/atmosphere/):
/// `div#layers-home-desc`, verborgen und von drei Elementen derselben
/// `figure[data-viz]` per `aria-describedby` referenziert. Dort war es
/// `display: none` — ein per CSS verborgener Text steht nicht im Markup, den
/// misst auditmysite an der laufenden Seite. Seitdem blendet Geographia die
/// Schicht nur visuell aus (`.viz-text` in `styles/app.css`); im heutigen
/// Build gibt es keinen Befund.
pub(crate) fn text_hidden<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for fig in visualisierungen(doc) {
        if mit_vorfahren(fig).any(|a| a.has_attr("hidden")) {
            continue;
        }
        for text in descendants(fig).filter(|n| n.has_attr("data-viz-text")) {
            let Some(grund) = verborgen_durch(text) else {
                continue;
            };
            let inert = mit_vorfahren(text).any(|a| a.has_attr("inert"));
            let finding = if !inert && beschrieben_in(fig, text) {
                Finding::review(
                    "display/text-hidden",
                    pick!(
                        locale,
                        format!(
                            "The text layer [data-viz-text] is hidden ({grund}), but an element \
                             of the visualisation references it by aria-describedby or \
                             aria-details: screen readers get the statement only as a \
                             description, with its structure flattened. Hiding it visually is \
                             recommended."
                        ),
                        format!(
                            "Die Textschicht [data-viz-text] ist verborgen ({grund}), aber ein \
                             Element der Visualisierung verweist per aria-describedby oder \
                             aria-details darauf: Screenreader bekommen die Aussage nur als \
                             Beschreibung, ohne ihre Struktur. Empfohlen ist, sie nur visuell \
                             auszublenden."
                        ),
                    ),
                )
                .with_severity(Severity::Low)
            } else {
                Finding::fail(
                    "display/text-hidden",
                    pick!(
                        locale,
                        format!(
                            "The text layer [data-viz-text] is removed from assistive \
                             technology ({grund}); screen reader users lose the statement."
                        ),
                        format!(
                            "Die Textschicht [data-viz-text] ist der Assistenztechnik entzogen \
                             ({grund}); Screenreader-Nutzer verlieren die Aussage."
                        ),
                    ),
                )
                .with_severity(Severity::High)
            };
            out.push(
                finding
                    .with_wcag(["1.1.1"])
                    .with_tags(SCHLAGWORTE)
                    .at(at(text.id())),
            );
        }
    }
}

// --- viz/caption-missing -----------------------------------------------------

/// 1.1.1: Die Visualisierung hat kein `<figcaption>` als Kind und damit keinen
/// Namen, der sie kurz benennt.
///
/// Nur das direkte Kind zählt: Nach HTML ist ein `<figcaption>` die
/// Beschriftung der `<figure>`, deren Kind es ist. Geprüft wird das
/// Vorhandensein, nicht der Inhalt. Die erfüllte Form ist
/// `DiagramFigure.astro` in Geographia. Beleg für den Befund: `MeanChart.astro`
/// (`atmosphere/climate-monitor/`) und `GermanyMap.svelte`
/// (`atmosphere/germany/`) beschriften ihre `figure[data-viz]` nicht.
pub(crate) fn caption<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for fig in visualisierungen(doc) {
        if fig.children().any(|k| k.is_element("figcaption")) {
            continue;
        }
        out.push(
            Finding::fail(
                "viz/caption-missing",
                pick!(
                    locale,
                    "This visualisation (figure[data-viz]) has no <figcaption>, so nothing names \
                     it.",
                    "Die Visualisierung (figure[data-viz]) hat kein <figcaption>; nichts benennt \
                     sie.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.1.1"])
            .with_tags(SCHLAGWORTE)
            .at(at(fig.id())),
        );
    }
}

// --- viz/static-missing ------------------------------------------------------

/// 2.2.2: Eine bewegte oder bedienbare Visualisierung (`data-viz="3d"` oder
/// `"interactive"`) ohne Standbild `[data-viz-static]` — im Modus `calm` gibt
/// es dann nichts, was statt der Bewegung erscheint. Beleg: „Build Earth 2.0"
/// (`data-viz="interactive"`, Geographia `space/solar-system/settlement/`,
/// `EarthTwoFigure.astro`).
pub(crate) fn static_layer<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for fig in visualisierungen(doc) {
        let art = art(fig);
        if !matches!(art.as_str(), "3d" | "interactive")
            || descendants(fig).any(|n| n.has_attr("data-viz-static"))
        {
            continue;
        }
        out.push(
            Finding::fail(
                "viz/static-missing",
                pick!(
                    locale,
                    format!(
                        "This visualisation (data-viz=\"{art}\") has no still image \
                         [data-viz-static]; in calm mode nothing replaces the motion."
                    ),
                    format!(
                        "Die Visualisierung (data-viz=\"{art}\") hat kein Standbild \
                         [data-viz-static]; im ruhigen Modus ersetzt nichts die Bewegung."
                    ),
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.2.2"])
            .with_tags(SCHLAGWORTE)
            .at(at(fig.id())),
        );
    }
}

// --- viz/table-missing -------------------------------------------------------

/// 1.3.1: Ein Diagramm (`data-viz="chart"`) ohne `<table>`. Die Werte gehören
/// in eine Tabelle, damit ihre Beziehungen erhalten bleiben.
///
/// `REVIEW`, niedrig: Bei wenigen Werten trägt auch ein Satz die Aussage —
/// das kann nur ein Mensch beurteilen. Beleg: In Geographia meldet die Regel
/// 196 Diagramme auf 56 von 371 Seiten, vor allem Sparklines
/// (`<figure class="spark">`), deren Textschicht Anfang, Ende und Trend in
/// Worten nennt.
pub(crate) fn table<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for fig in visualisierungen(doc) {
        if art(fig) != "chart" || descendants(fig).any(|n| n.is_element("table")) {
            continue;
        }
        out.push(
            Finding::review(
                "viz/table-missing",
                pick!(
                    locale,
                    "This chart (data-viz=\"chart\") has no <table> with its values. Check \
                     whether the text layer conveys the values without one.",
                    "Das Diagramm (data-viz=\"chart\") hat keine <table> mit seinen Werten. \
                     Prüfen, ob die Textschicht die Werte auch ohne Tabelle vermittelt.",
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.3.1"])
            .with_tags(SCHLAGWORTE)
            .at(at(fig.id())),
        );
    }
}

// --- viz/orphan-media --------------------------------------------------------

/// Die kürzere Seite, bis zu der ein `<svg>` als Symbol oder Wortmarke gilt.
/// 32 ist die Symbolgrenze, die auditmysite im Textmodus am gemessenen
/// Element zieht (dort als Fläche 32 × 32).
const SYMBOL_SEITE: f64 = 32.0;

/// Breite und Höhe eines `<svg>` aus `width`/`height` (Zahl oder `px`),
/// sonst aus der `viewBox`.
fn svg_groesse<'a, N: Node<'a>>(n: N) -> Option<(f64, f64)> {
    let px = |a: &str| {
        let v = n.attr(a)?.trim();
        v.strip_suffix("px").unwrap_or(v).trim().parse::<f64>().ok()
    };
    if let (Some(b), Some(h)) = (px("width"), px("height")) {
        return Some((b, h));
    }
    let vb: Vec<f64> = n
        .attr("viewBox")?
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    (vb.len() == 4).then(|| (vb[2], vb[3]))
}

/// Rollen, die ein Element zum Bedienelement machen; ein Symbol darin
/// beschriftet das Bedienelement, es ist keine Visualisierung.
const BEDIEN_ROLLEN: &[&str] = &[
    "button",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "tab",
    "switch",
    "checkbox",
    "radio",
    "option",
];

fn ist_bedienelement<'a, N: Node<'a>>(n: N) -> bool {
    matches!(n.local_name(), "a" | "button" | "label" | "summary")
        || explizite_rolle(n).is_some_and(|r| BEDIEN_ROLLEN.contains(&r))
}

/// Ob ein `<svg role="img">` ein Symbol ist und keine Grafik.
///
/// Statisch gibt es keine gemessene Größe. Ein Symbol erkennt die Regel
/// deshalb an zwei Merkmalen: Es steckt in einem Bedienelement (Link, Button,
/// Label, `summary` oder eine Bedienrolle) und beschriftet dieses, oder die
/// kürzere Seite seiner Größe aus `width`/`height`, ersatzweise der `viewBox`,
/// misst höchstens 32. Die kürzere Seite statt der Fläche, weil Wortmarken
/// breit und flach sind (`viewBox="0 0 229 32"`); ein Diagramm braucht in
/// beiden Richtungen Platz. Ein `aria-hidden`-Symbol sieht die Regel gar
/// nicht erst (Geltungsbereich Accessibility-Tree).
///
/// Gegenprobe auf den 48 echten Seiten (ohne Opt-in, nur das Kriterium):
/// Von 928 `<svg role="img">` blieben 28 übrig, Illustrationen und Logos ab
/// 37 Einheiten Höhe; alle übrigen waren verborgen, steckten in einem
/// Bedienelement oder waren klein.
fn ist_symbol<'a, N: Node<'a>>(svg: N) -> bool {
    ancestors(svg).any(ist_bedienelement)
        || svg_groesse(svg).is_some_and(|(b, h)| b.min(h) <= SYMBOL_SEITE)
}

/// Medien, die eine Visualisierung tragen können, außerhalb der Konvention:
/// `<canvas>`, `<video>` und `<svg role="img">` ohne Vorfahren mit `data-viz`.
///
/// Läuft nur auf Seiten, die die Konvention benutzen ([`nutzt_konvention`]);
/// sonst meldete sie jedes Video im Web. `REVIEW`, niedrig: Ob das Medium eine
/// Visualisierung ist oder schmückt (ein Sternenhimmel als Hintergrund, ein
/// Erklärvideo), entscheidet ein Mensch. `aria-hidden` und `hidden` nimmt der
/// Geltungsbereich heraus — wer das Medium so markiert, erklärt es zur Zierde.
///
/// Ein `[data-viz]` an einem anderen Element als `<figure>` zählt als
/// Einbindung: Geographia hängt seine Zeitachse und den 3D-Erkunder so ein.
/// Einen echten Treffer gibt es noch nicht — in Geographias 371 Seiten steckt
/// jedes `<canvas>` und jedes `<svg role="img">` in einem `[data-viz]` oder
/// unter `aria-hidden`. Belegt ist bisher nur, dass die Regel dort und auf den
/// 48 Seiten ohne Konvention nichts meldet.
pub(crate) fn orphan_media<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    if !nutzt_konvention(doc) {
        return;
    }
    for n in elements(doc) {
        let medium = match n.local_name() {
            "canvas" | "video" => true,
            "svg" => explizite_rolle(n) == Some("img") && !ist_symbol(n),
            _ => false,
        };
        if !medium || ancestors(n).any(|a| a.has_attr("data-viz")) {
            continue;
        }
        let tag = n.local_name();
        out.push(
            Finding::review(
                "viz/orphan-media",
                pick!(
                    locale,
                    format!(
                        "This page uses the display-mode convention, but this <{tag}> is not \
                         part of a visualisation (figure[data-viz]). If it carries information, \
                         it has no text layer and does not follow the display mode."
                    ),
                    format!(
                        "Die Seite benutzt die Darstellungskonvention, aber dieses <{tag}> \
                         gehört zu keiner Visualisierung (figure[data-viz]). Trägt es \
                         Information, fehlt ihm die Textschicht, und es folgt dem \
                         Darstellungsmodus nicht."
                    ),
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.1.1"])
            .with_tags(SCHLAGWORTE)
            .at(at(n.id())),
        );
    }
}

// --- display/toggle-missing --------------------------------------------------

/// 2.2.2: Die Seite hat Visualisierungen, aber kein `[data-display-toggle]`.
///
/// Der Text sagt, dass der markierte Umschalter fehlt, nicht, dass sich der
/// Modus nicht umschalten lässt (auditmysite#704): Ein Umschalter ohne Marke
/// ist statisch nicht zu erkennen. Die erfüllte Form ist Geographias
/// `<div class="display-toggle" data-display-toggle>` in der Kopfzeile. Beleg
/// für den Befund: die Laborseiten `atmosphere/labor/globus/` und
/// `atmosphere/labor/klimasystem/` (`noindex`) zeigen Visualisierungen ohne
/// diese Kopfzeile.
pub(crate) fn toggle<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let anzahl = visualisierungen(doc).count();
    if anzahl == 0 || elements(doc).any(|n| n.has_attr("data-display-toggle")) {
        return;
    }
    out.push(
        Finding::fail(
            "display/toggle-missing",
            pick!(
                locale,
                format!(
                    "The page contains {anzahl} visualisation(s) (figure[data-viz]), but no \
                     toggle marked [data-display-toggle] was found."
                ),
                format!(
                    "Die Seite enthält {anzahl} Visualisierung(en) (figure[data-viz]), aber es \
                     wurde kein mit [data-display-toggle] markierter Umschalter gefunden."
                ),
            ),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["2.2.2"])
        .with_tags(SCHLAGWORTE)
        .at(at(doc.root().id())),
    );
}

// --- display/init-missing ----------------------------------------------------

/// Ob ein `<script>` das Parsen anhält, also vor `<body>` läuft: klassisch
/// (kein `type="module"`, kein Datenblock) und, wenn extern, ohne `async` und
/// `defer`. An einem Inline-Skript wirken `async` und `defer` nicht.
fn blockiert<'a, N: Node<'a>>(script: N) -> bool {
    let klassisch = script.attr("type").is_none_or(|t| {
        let t = t.trim().to_ascii_lowercase();
        t.is_empty() || t.contains("javascript") || t.contains("ecmascript")
    });
    klassisch
        && (!script.has_attr("src") || !(script.has_attr("async") || script.has_attr("defer")))
}

/// 2.2.2: Die Seite benutzt die Konvention, aber `html[data-display]` steht
/// weder im Markup, noch läuft ein blockierendes Skript in `<head>`, das es
/// vor dem ersten Zeichnen setzen könnte. Die Seite erscheint dann zuerst im
/// falschen Modus, etwa mit Animation.
///
/// Statisch prüfbar ist nur die Abwesenheit: Ob ein vorhandenes Skript den
/// Modus wirklich setzt, zeigt erst die laufende Seite (auditmysite misst
/// das). Deshalb `REVIEW`, niedrig. Beleg für die Form: Geographia lädt
/// `display-init.*.js` als klassisches Skript in `<head>`; `<html>` trägt im
/// gebauten HTML noch kein `data-display`. Alle 371 Seiten erfüllen das, ein
/// Befund kam dort nicht vor.
pub(crate) fn init<D: Document>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let benutzt = elements(doc).any(|n| ist_viz(n) || n.has_attr("data-display-toggle"));
    if !benutzt || doc.root().has_attr("data-display") {
        return;
    }
    let skript_vorher = elements(doc).any(|n| {
        n.is_element("script") && ancestors(n).any(|a| a.is_element("head")) && blockiert(n)
    });
    if skript_vorher {
        return;
    }
    out.push(
        Finding::review(
            "display/init-missing",
            pick!(
                locale,
                "The page uses the display-mode convention, but html[data-display] is not in the \
                 markup and no blocking script in <head> could set it before the body renders. \
                 Check that the chosen mode applies from the first paint.",
                "Die Seite benutzt die Darstellungskonvention, aber html[data-display] steht \
                 nicht im Markup, und kein blockierendes Skript in <head> kann es vor dem \
                 Zeichnen des Inhalts setzen. Prüfen, ob der gewählte Modus ab dem ersten Bild \
                 gilt.",
            ),
        )
        .with_severity(Severity::Low)
        .with_wcag(["2.2.2"])
        .with_tags(SCHLAGWORTE)
        .at(at(doc.root().id())),
    );
}
