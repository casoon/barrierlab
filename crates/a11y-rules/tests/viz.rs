//! Die statischen Regeln der Darstellungskonvention (casoon/barrierlab#22).
//!
//! Die Fälle stammen aus der Referenzumsetzung Geographia (`web-geographia`,
//! gebaute Seiten unter `apps/*/dist`, Stand 2026-10-03) und aus
//! auditmysite#704; der Name jedes Tests nennt die Quelle. Das Markup ist auf
//! das gekürzt, was die Regel liest. Alle Regeln sind Tier 1 und laufen ohne
//! Semantik.

use a11y_dom::{Arena, ArenaBuilder, Node, elements};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn dokument(html_attr: &str, head: &str, body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"{html_attr}><head><title>T</title>{head}</head>\
         <body>{body}</body></html>"
    ))
    .document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    uebernehmen(&doc, wurzel, Arena::builder()).build()
}

fn uebernehmen(
    doc: &html5_parser::Document,
    id: html5_parser::NodeId,
    b: ArenaBuilder,
) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.close()
        }
        H::Text { content } => b.text(content),
        _ => b,
    }
}

/// Geographias Initialisierung in `<head>` (`atmosphere/index.html`).
const INIT: &str = r#"<script src="/atmosphere/_astro/display-init.D8c3faS1.js"></script>"#;

/// Geographias Umschalter in der Kopfzeile.
const UMSCHALTER: &str = r#"<div class="display-toggle" data-display-toggle><button type="button" aria-pressed="true">Visual</button><button type="button" aria-pressed="false">Calm</button><button type="button" aria-pressed="false">Text</button></div>"#;

/// Eine Seite, die die Konvention sonst erfüllt: Initialisierung und
/// Umschalter sind da.
fn konvention(main: &str) -> Arena {
    dokument(
        "",
        INIT,
        &format!("<header>{UMSCHALTER}</header><main>{main}</main>"),
    )
}

/// Die `id`-Attribute der Elemente, an denen `rule` meldet.
fn an(doc: &Arena, r: &Report, rule: &str) -> Vec<String> {
    let knoten: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| f.location.node.clone())
        .collect();
    elements(doc)
        .filter(|n| knoten.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

fn urteile(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

fn konventionsbefunde(r: &Report) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.rule_id.starts_with("viz/") || f.rule_id.starts_with("display/"))
        .map(|f| f.rule_id.as_str())
        .collect()
}

/// `DiagramFigure.astro` mit `VizText.astro`, wie Geographia es baut: die
/// erfüllte Form.
const DIAGRAMM: &str = r#"<figure id="fronten" class="diagram-figure" data-viz="diagram" data-diagram="fronts">
  <div class="frame" data-viz-static><div class="scroll"><svg viewBox="0 0 640 360" role="img" aria-label="Warm- und Kaltfront im Schnitt"></svg></div><p class="schema-note">Schema, nicht maßstäblich</p></div>
  <div class="viz-text" id="fronts-desc" data-viz-text><p class="viz-text-title">Was die Grafik zeigt</p><p>Die Kaltfront schiebt sich unter die warme Luft.</p></div>
  <figcaption>Fronten im Querschnitt</figcaption>
</figure>"#;

// --- Ohne Opt-in -------------------------------------------------------------

#[test]
fn ohne_konvention_meldet_keine_regel() {
    // Canvas, Video und ein großes svg[role=img] — auf einer Seite ohne
    // data-viz, data-display-toggle und html[data-display] kein Befund. So
    // auf allen 48 echten Seiten (Abruf 2026-10-03).
    let doc = dokument(
        "",
        "",
        r#"<main><canvas id="c" width="800" height="400"></canvas><video id="v" src="/a.mp4" controls></video>
           <svg id="s" role="img" aria-label="Umsatz" viewBox="0 0 640 320"></svg>
           <figure><img src="a.png" alt="Ein Bild"></figure></main>"#,
    );
    assert!(konventionsbefunde(&run(&doc)).is_empty());
}

#[test]
fn erfuellte_form_diagram_figure_geographia() {
    let doc = konvention(DIAGRAMM);
    let r = run(&doc);
    assert!(
        konventionsbefunde(&r).is_empty(),
        "{:?}",
        konventionsbefunde(&r)
    );
}

#[test]
fn konventionsregeln_sind_best_practice_auditmysite_704() {
    let doc = dokument(
        "",
        "",
        r#"<main><figure id="f" data-viz="3d"><canvas></canvas></figure><canvas id="c" width="800" height="400"></canvas></main>"#,
    );
    let r = run(&doc);
    let befunde: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("viz/") || f.rule_id.starts_with("display/"))
        .collect();
    assert!(befunde.len() >= 5, "{befunde:?}");
    for f in befunde {
        assert_eq!(f.tags, ["best-practice"], "{}", f.rule_id);
    }
}

// --- viz/text-missing --------------------------------------------------------

#[test]
fn beschreibung_ohne_marke_geographia_startseite() {
    // home/index.html (EarthNow.astro): Die Beschreibung steht in
    // <span class="viz-desc"> in der figcaption, ohne [data-viz-text].
    let doc = konvention(
        r#"<figure id="streifen" class="viz" data-viz="chart"><div class="stripes" data-viz-static aria-hidden="true"><span></span></div>
           <figcaption class="caption"><a href="/atmosphere/chapters/1850-to-today/">Global temperature 1850–2025, one stripe per year</a>
           <span class="viz-desc">Coldest year in the series: 1904 (−0.24 °C).</span></figcaption><table><tr><th>Jahr</th></tr></table></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "viz/text-missing"), ["streifen"]);
    assert_eq!(
        urteile(&r, "viz/text-missing"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn leere_textschicht() {
    let doc = konvention(
        r#"<figure id="leer" data-viz="diagram"><svg viewBox="0 0 640 360" role="img" aria-label="x"></svg>
           <div data-viz-text>  <p> </p> </div><figcaption>Leer</figcaption></figure>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "viz/text-missing"), ["leer"]);
}

#[test]
fn ohne_text_und_standbild_geographia_build_earth() {
    // space/solar-system/settlement/ (EarthTwoFigure.astro): interaktiv,
    // mit figcaption und Tabelle, aber ohne Text- und Standbildschicht.
    let doc = konvention(
        r#"<figure id="earth2" class="my-8" data-viz="interactive"><figcaption class="mb-4"><strong class="block">Build Earth 2.0</strong>
           <span>Which world comes closest to Earth in each property?</span></figcaption>
           <astro-island><table><tr><th>Gravity</th><td>1 g</td></tr></table></astro-island></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "viz/text-missing"), ["earth2"]);
    assert_eq!(an(&doc, &r, "viz/static-missing"), ["earth2"]);
    assert_eq!(
        urteile(&r, "viz/static-missing"),
        [(Outcome::Fail, Severity::Medium)]
    );
    assert!(an(&doc, &r, "viz/caption-missing").is_empty());
}

// --- display/text-hidden (auditmysite#704) -----------------------------------

/// Der Fall aus auditmysite#704 (geographia.eu/atmosphere/): eine Textschicht,
/// die drei Elemente derselben Visualisierung referenzieren. `verbergen`
/// steht an der Textschicht, `bezug` ist das Attribut der Verweise.
fn schichten(verbergen: &str, bezug: &str) -> Arena {
    konvention(&format!(
        r#"<figure id="layers" data-viz="3d"><div data-viz-static><svg role="img" aria-label="Die Stockwerke der Atmosphäre" {bezug}="layers-home-desc" viewBox="0 0 360 300"></svg>
           <svg class="profile" role="img" aria-label="Temperatur nach Höhe" {bezug}="layers-home-desc" viewBox="0 0 280 300"></svg></div>
           <div role="application" aria-label="Bühne" {bezug}="layers-home-desc" data-viz-live></div>
           <div id="layers-home-desc" data-viz-text {verbergen}><p>Troposphäre, Stratosphäre, Mesosphäre.</p></div>
           <figcaption>Die Stockwerke</figcaption></figure>"#
    ))
}

#[test]
fn verborgen_aber_beschrieben_ist_hinweis_auditmysite_704_fall_1() {
    for verbergen in ["hidden", r#"aria-hidden="true""#] {
        let doc = schichten(verbergen, "aria-describedby");
        let r = run(&doc);
        assert_eq!(an(&doc, &r, "display/text-hidden"), ["layers-home-desc"]);
        assert_eq!(
            urteile(&r, "display/text-hidden"),
            [(Outcome::Review, Severity::Low)],
            "{verbergen}"
        );
    }
}

#[test]
fn aria_details_wie_describedby_auditmysite_704_fall_2() {
    let r = run(&schichten("hidden", "aria-details"));
    assert_eq!(
        urteile(&r, "display/text-hidden"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn inert_bleibt_verstoss_auditmysite_704_fall_3() {
    let r = run(&schichten("inert", "aria-describedby"));
    assert_eq!(
        urteile(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );

    // inert an einem Vorfahren, die Schicht selbst per hidden verborgen.
    let doc = konvention(
        r#"<figure data-viz="chart"><svg role="img" aria-label="x" aria-describedby="t" viewBox="0 0 640 320"></svg>
           <div inert><div id="t" data-viz-text hidden><p>Werte</p></div></div><table></table><figcaption>K</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "display/text-hidden"), ["t"]);
    assert_eq!(
        urteile(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn nur_von_verborgenem_referenziert_auditmysite_704_fall_4() {
    let doc = konvention(
        r#"<figure data-viz="chart"><div data-viz-static aria-hidden="true"><svg role="img" aria-label="x" aria-describedby="t" viewBox="0 0 640 320"></svg></div>
           <div id="t" data-viz-text hidden><p>Werte</p></div><table></table><figcaption>K</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(
        urteile(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn unreferenziert_verborgen_auditmysite_704_fall_5() {
    for verbergen in ["hidden", r#"aria-hidden="true""#, "inert"] {
        let doc = konvention(&format!(
            r#"<figure data-viz="diagram"><svg role="img" aria-label="x" viewBox="0 0 640 320"></svg>
               <div id="t" data-viz-text {verbergen}><p>Aussage</p></div><figcaption>K</figcaption></figure>"#
        ));
        let r = run(&doc);
        assert_eq!(an(&doc, &r, "display/text-hidden"), ["t"], "{verbergen}");
        assert_eq!(
            urteile(&r, "display/text-hidden"),
            [(Outcome::Fail, Severity::High)]
        );
    }
}

#[test]
fn mit_der_visualisierung_verborgen_auditmysite_725() {
    // Ein inaktives Panel: Die ganze figure ist per hidden ausgeblendet, mit
    // ihr die Grafik. Kein Befund.
    let doc = konvention(
        r#"<div role="tabpanel" hidden><figure data-viz="chart"><svg role="img" aria-label="x" viewBox="0 0 640 320"></svg>
           <div id="t" data-viz-text aria-hidden="true"><p>Werte</p></div><table></table><figcaption>K</figcaption></figure></div>"#,
    );
    assert!(urteile(&run(&doc), "display/text-hidden").is_empty());
}

#[test]
fn nur_visuell_ausgeblendet_geographia_heute() {
    // VizText.astro: die Klasse .viz-text blendet nur visuell aus.
    let doc = konvention(DIAGRAMM);
    assert!(urteile(&run(&doc), "display/text-hidden").is_empty());
}

// --- viz/caption-missing -----------------------------------------------------

#[test]
fn ohne_figcaption_geographia_klima_monitor() {
    // atmosphere/climate-monitor/ (MeanChart.astro): figure mit Grafik und
    // VizText, aber ohne figcaption.
    let doc = konvention(
        r#"<figure id="mittel" class="mean-chart" data-viz="chart"><svg role="img" aria-label="Mittel" viewBox="0 0 640 320" data-viz-static></svg>
           <div class="viz-text" data-viz-text><p>Das Mittel steigt.</p><table><tr><th>Jahr</th></tr></table></div></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "viz/caption-missing"), ["mittel"]);
    assert_eq!(
        urteile(&r, "viz/caption-missing"),
        [(Outcome::Fail, Severity::Low)]
    );
}

#[test]
fn verschachteltes_figcaption_beschriftet_nicht() {
    let doc = konvention(
        r#"<figure id="tief" data-viz="diagram"><div><figcaption>Zu tief</figcaption></div><div data-viz-text>Aussage</div></figure>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "viz/caption-missing"), ["tief"]);
}

// --- viz/static-missing ------------------------------------------------------

#[test]
fn standbild_nur_fuer_bewegtes_und_bedienbares() {
    let doc = konvention(
        r#"<figure id="drei" data-viz="3D"><canvas></canvas><div data-viz-text>A</div><figcaption>A</figcaption></figure>
           <figure id="mit" data-viz="interactive"><div data-viz-static><svg viewBox="0 0 640 320" aria-hidden="true"></svg></div><div data-viz-text>B</div><figcaption>B</figcaption></figure>
           <figure id="diagramm" data-viz="diagram"><svg viewBox="0 0 640 320" aria-hidden="true"></svg><div data-viz-text>C</div><figcaption>C</figcaption></figure>"#,
    );
    assert_eq!(an(&doc, &run(&doc), "viz/static-missing"), ["drei"]);
}

// --- viz/table-missing -------------------------------------------------------

#[test]
fn sparkline_ohne_tabelle_geographia() {
    // Die Sparklines in Geographia (<figure class="spark">) nennen Anfang,
    // Ende und Trend in Worten, ohne Tabelle: ein Fall zum Prüfen.
    let doc = konvention(
        r#"<figure id="spark" class="spark" data-viz="chart"><svg viewBox="0 0 200 40" aria-hidden="true" data-viz-static></svg>
           <div data-viz-text><p>Von 280 ppm auf 424 ppm, seit 1960 steiler.</p></div><figcaption>CO₂</figcaption></figure>
           <figure id="mit" data-viz="chart"><svg viewBox="0 0 640 320" aria-hidden="true" data-viz-static></svg>
           <div data-viz-text><table><tr><th>Jahr</th><th>ppm</th></tr></table></div><figcaption>CO₂</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(an(&doc, &r, "viz/table-missing"), ["spark"]);
    assert_eq!(
        urteile(&r, "viz/table-missing"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn tabelle_nur_fuer_diagramme() {
    let doc = konvention(DIAGRAMM);
    assert!(urteile(&run(&doc), "viz/table-missing").is_empty());
}

// --- display/toggle-missing --------------------------------------------------

#[test]
fn ohne_umschalter_geographia_laborseite() {
    // atmosphere/labor/globus/ (noindex): Visualisierung, Initialisierung,
    // aber keine Kopfzeile mit Umschalter.
    let doc = dokument("", INIT, &format!("<main>{DIAGRAMM}</main>"));
    let r = run(&doc);
    assert_eq!(
        urteile(&r, "display/toggle-missing"),
        [(Outcome::Fail, Severity::Medium)]
    );
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "display/toggle-missing")
        .unwrap();
    // auditmysite#704: Die Marke fehlt, nicht die Möglichkeit umzuschalten.
    assert!(
        f.message
            .contains("no toggle marked [data-display-toggle] was found"),
        "{}",
        f.message
    );
    assert!(!f.message.contains("cannot switch"), "{}", f.message);
}

#[test]
fn umschalter_ohne_visualisierung_braucht_nichts() {
    let doc = dokument("", "", "<main><p>Nur Text</p></main>");
    assert!(urteile(&run(&doc), "display/toggle-missing").is_empty());
}

// --- display/init-missing ----------------------------------------------------

#[test]
fn initialisierung_in_head_geographia() {
    // Geographia: display-init.*.js als klassisches Skript in <head>.
    let doc = konvention(DIAGRAMM);
    assert!(urteile(&run(&doc), "display/init-missing").is_empty());

    let inline = dokument(
        "",
        r#"<script>document.documentElement.dataset.display = localStorage.getItem("display") || "visual";</script>"#,
        &format!("{UMSCHALTER}<main>{DIAGRAMM}</main>"),
    );
    assert!(urteile(&run(&inline), "display/init-missing").is_empty());

    let gesetzt = dokument(
        r#" data-display="calm""#,
        "",
        &format!("{UMSCHALTER}<main>{DIAGRAMM}</main>"),
    );
    assert!(urteile(&run(&gesetzt), "display/init-missing").is_empty());
}

#[test]
fn ohne_blockierendes_skript_vor_body() {
    for head in [
        "",
        r#"<script src="/display-init.js" defer></script>"#,
        r#"<script src="/display-init.js" async></script>"#,
        r#"<script type="module" src="/display-init.js"></script>"#,
        r#"<script type="application/ld+json">{}</script>"#,
    ] {
        let doc = dokument("", head, &format!("{UMSCHALTER}<main>{DIAGRAMM}</main>"));
        assert_eq!(
            urteile(&run(&doc), "display/init-missing"),
            [(Outcome::Review, Severity::Low)],
            "{head}"
        );
    }

    // Ein Skript erst in <body> kommt zu spät.
    let im_body = dokument(
        "",
        "",
        &format!("<script src=\"/display-init.js\"></script>{UMSCHALTER}<main>{DIAGRAMM}</main>"),
    );
    assert_eq!(urteile(&run(&im_body), "display/init-missing").len(), 1);
}

#[test]
fn nur_umschalter_genuegt_fuer_init() {
    // Wie in auditmysite: Auch ein Umschalter allein ist Konvention.
    let doc = dokument("", "", &format!("{UMSCHALTER}<main></main>"));
    assert_eq!(urteile(&run(&doc), "display/init-missing").len(), 1);
}

#[cfg(feature = "de")]
#[test]
fn befunde_folgen_der_sprache() {
    let doc = dokument("", "", r#"<main><figure data-viz="chart"></figure></main>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "display/toggle-missing")
        .unwrap();
    assert!(f.message.contains("Umschalter"), "{}", f.message);
}
