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

fn document(html_attr: &str, head: &str, body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"{html_attr}><head><title>T</title>{head}</head>\
         <body>{body}</body></html>"
    ))
    .document;
    let root = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .unwrap();
    adopt(&doc, root, Arena::builder()).build()
}

fn adopt(doc: &html5_parser::Document, id: html5_parser::NodeId, b: ArenaBuilder) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = adopt(doc, k, b);
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
const TOGGLE: &str = r#"<div class="display-toggle" data-display-toggle><button type="button" aria-pressed="true">Visual</button><button type="button" aria-pressed="false">Calm</button><button type="button" aria-pressed="false">Text</button></div>"#;

/// Eine Seite, die die Konvention sonst erfüllt: Initialisierung und
/// Umschalter sind da.
fn convention(main: &str) -> Arena {
    document(
        "",
        INIT,
        &format!("<header>{TOGGLE}</header><main>{main}</main>"),
    )
}

/// Die `id`-Attribute der Elemente, an denen `rule` meldet.
fn findings_at(doc: &Arena, r: &Report, rule: &str) -> Vec<String> {
    let node: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| f.location.node.clone())
        .collect();
    elements(doc)
        .filter(|n| node.contains(&n.id().to_string()))
        .map(|n| n.attr("id").unwrap_or("?").to_string())
        .collect()
}

fn verdicts(r: &Report, rule: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

fn convention_findings(r: &Report) -> Vec<&str> {
    r.findings
        .iter()
        .filter(|f| f.rule_id.starts_with("viz/") || f.rule_id.starts_with("display/"))
        .map(|f| f.rule_id.as_str())
        .collect()
}

/// `DiagramFigure.astro` mit `VizText.astro`, wie Geographia es baut: die
/// erfüllte Form.
const CHART: &str = r#"<figure id="fronten" class="diagram-figure" data-viz="diagram" data-diagram="fronts">
  <div class="frame" data-viz-static><div class="scroll"><svg viewBox="0 0 640 360" role="img" aria-label="Warm- und Kaltfront im Schnitt"></svg></div><p class="schema-note">Schema, nicht maßstäblich</p></div>
  <div class="viz-text" id="fronts-desc" data-viz-text><p class="viz-text-title">Was die Grafik zeigt</p><p>Die Kaltfront schiebt sich unter die warme Luft.</p></div>
  <figcaption>Fronten im Querschnitt</figcaption>
</figure>"#;

// --- Ohne Opt-in -------------------------------------------------------------

#[test]
fn without_convention_no_rule_reports() {
    // Canvas, Video und ein großes svg[role=img] — auf einer Seite ohne
    // data-viz, data-display-toggle und html[data-display] kein Befund. So
    // auf allen 48 echten Seiten (Abruf 2026-10-03).
    let doc = document(
        "",
        "",
        r#"<main><canvas id="c" width="800" height="400"></canvas><video id="v" src="/a.mp4" controls></video>
           <svg id="s" role="img" aria-label="Umsatz" viewBox="0 0 640 320"></svg>
           <figure><img src="a.png" alt="Ein Bild"></figure></main>"#,
    );
    assert!(convention_findings(&run(&doc)).is_empty());
}

#[test]
fn fulfilled_form_diagram_figure_geographia() {
    let doc = convention(CHART);
    let r = run(&doc);
    assert!(
        convention_findings(&r).is_empty(),
        "{:?}",
        convention_findings(&r)
    );
}

#[test]
fn convention_rules_are_best_practice_auditmysite_704() {
    let doc = document(
        "",
        "",
        r#"<main><figure id="f" data-viz="3d"><canvas></canvas></figure><canvas id="c" width="800" height="400"></canvas></main>"#,
    );
    let r = run(&doc);
    let findings: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("viz/") || f.rule_id.starts_with("display/"))
        .collect();
    assert!(findings.len() >= 5, "{findings:?}");
    for f in findings {
        assert_eq!(f.tags, ["best-practice"], "{}", f.rule_id);
    }
}

// --- viz/text-missing --------------------------------------------------------

#[test]
fn description_without_mark_geographia_home_page() {
    // home/index.html (EarthNow.astro): Die Beschreibung steht in
    // <span class="viz-desc"> in der figcaption, ohne [data-viz-text].
    let doc = convention(
        r#"<figure id="streifen" class="viz" data-viz="chart"><div class="stripes" data-viz-static aria-hidden="true"><span></span></div>
           <figcaption class="caption"><a href="/atmosphere/chapters/1850-to-today/">Global temperature 1850–2025, one stripe per year</a>
           <span class="viz-desc">Coldest year in the series: 1904 (−0.24 °C).</span></figcaption><table><tr><th>Jahr</th></tr></table></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "viz/text-missing"), ["streifen"]);
    assert_eq!(
        verdicts(&r, "viz/text-missing"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn empty_text_layer() {
    let doc = convention(
        r#"<figure id="leer" data-viz="diagram"><svg viewBox="0 0 640 360" role="img" aria-label="x"></svg>
           <div data-viz-text>  <p> </p> </div><figcaption>Leer</figcaption></figure>"#,
    );
    assert_eq!(findings_at(&doc, &run(&doc), "viz/text-missing"), ["leer"]);
}

#[test]
fn without_text_and_still_image_geographia_build_earth() {
    // space/solar-system/settlement/ (EarthTwoFigure.astro): interaktiv,
    // mit figcaption und Tabelle, aber ohne Text- und Standbildschicht.
    let doc = convention(
        r#"<figure id="earth2" class="my-8" data-viz="interactive"><figcaption class="mb-4"><strong class="block">Build Earth 2.0</strong>
           <span>Which world comes closest to Earth in each property?</span></figcaption>
           <astro-island><table><tr><th>Gravity</th><td>1 g</td></tr></table></astro-island></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "viz/text-missing"), ["earth2"]);
    assert_eq!(findings_at(&doc, &r, "viz/static-missing"), ["earth2"]);
    assert_eq!(
        verdicts(&r, "viz/static-missing"),
        [(Outcome::Fail, Severity::Medium)]
    );
    assert!(findings_at(&doc, &r, "viz/caption-missing").is_empty());
}

// --- display/text-hidden (auditmysite#704) -----------------------------------

/// Der Fall aus auditmysite#704 (geographia.eu/atmosphere/): eine Textschicht,
/// die drei Elemente derselben Visualisierung referenzieren. `hide`
/// steht an der Textschicht, `reference` ist das Attribut der Verweise.
fn layers(hide: &str, reference: &str) -> Arena {
    convention(&format!(
        r#"<figure id="layers" data-viz="3d"><div data-viz-static><svg role="img" aria-label="Die Stockwerke der Atmosphäre" {reference}="layers-home-desc" viewBox="0 0 360 300"></svg>
           <svg class="profile" role="img" aria-label="Temperatur nach Höhe" {reference}="layers-home-desc" viewBox="0 0 280 300"></svg></div>
           <div role="application" aria-label="Bühne" {reference}="layers-home-desc" data-viz-live></div>
           <div id="layers-home-desc" data-viz-text {hide}><p>Troposphäre, Stratosphäre, Mesosphäre.</p></div>
           <figcaption>Die Stockwerke</figcaption></figure>"#
    ))
}

#[test]
fn hidden_but_described_is_hint_auditmysite_704_case_1() {
    for hide in ["hidden", r#"aria-hidden="true""#] {
        let doc = layers(hide, "aria-describedby");
        let r = run(&doc);
        assert_eq!(
            findings_at(&doc, &r, "display/text-hidden"),
            ["layers-home-desc"]
        );
        assert_eq!(
            verdicts(&r, "display/text-hidden"),
            [(Outcome::Review, Severity::Low)],
            "{hide}"
        );
    }
}

#[test]
fn aria_details_like_describedby_auditmysite_704_case_2() {
    let r = run(&layers("hidden", "aria-details"));
    assert_eq!(
        verdicts(&r, "display/text-hidden"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn inert_stays_violation_auditmysite_704_case_3() {
    let r = run(&layers("inert", "aria-describedby"));
    assert_eq!(
        verdicts(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );

    // inert an einem Vorfahren, die Schicht selbst per hidden verborgen.
    let doc = convention(
        r#"<figure data-viz="chart"><svg role="img" aria-label="x" aria-describedby="t" viewBox="0 0 640 320"></svg>
           <div inert><div id="t" data-viz-text hidden><p>Werte</p></div></div><table></table><figcaption>K</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "display/text-hidden"), ["t"]);
    assert_eq!(
        verdicts(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn referenced_only_by_hidden_auditmysite_704_case_4() {
    let doc = convention(
        r#"<figure data-viz="chart"><div data-viz-static aria-hidden="true"><svg role="img" aria-label="x" aria-describedby="t" viewBox="0 0 640 320"></svg></div>
           <div id="t" data-viz-text hidden><p>Werte</p></div><table></table><figcaption>K</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(
        verdicts(&r, "display/text-hidden"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn unreferenced_hidden_auditmysite_704_case_5() {
    for hide in ["hidden", r#"aria-hidden="true""#, "inert"] {
        let doc = convention(&format!(
            r#"<figure data-viz="diagram"><svg role="img" aria-label="x" viewBox="0 0 640 320"></svg>
               <div id="t" data-viz-text {hide}><p>Aussage</p></div><figcaption>K</figcaption></figure>"#
        ));
        let r = run(&doc);
        assert_eq!(
            findings_at(&doc, &r, "display/text-hidden"),
            ["t"],
            "{hide}"
        );
        assert_eq!(
            verdicts(&r, "display/text-hidden"),
            [(Outcome::Fail, Severity::High)]
        );
    }
}

#[test]
fn hidden_with_the_visualisation_auditmysite_725() {
    // Ein inaktives Panel: Die ganze figure ist per hidden ausgeblendet, mit
    // ihr die Grafik. Kein Befund.
    let doc = convention(
        r#"<div role="tabpanel" hidden><figure data-viz="chart"><svg role="img" aria-label="x" viewBox="0 0 640 320"></svg>
           <div id="t" data-viz-text aria-hidden="true"><p>Werte</p></div><table></table><figcaption>K</figcaption></figure></div>"#,
    );
    assert!(verdicts(&run(&doc), "display/text-hidden").is_empty());
}

#[test]
fn only_visually_hidden_geographia_today() {
    // VizText.astro: die Klasse .viz-text blendet nur visuell aus.
    let doc = convention(CHART);
    assert!(verdicts(&run(&doc), "display/text-hidden").is_empty());
}

// --- viz/caption-missing -----------------------------------------------------

#[test]
fn without_figcaption_geographia_climate_monitor() {
    // atmosphere/climate-monitor/ (MeanChart.astro): figure mit Grafik und
    // VizText, aber ohne figcaption.
    let doc = convention(
        r#"<figure id="mittel" class="mean-chart" data-viz="chart"><svg role="img" aria-label="Mittel" viewBox="0 0 640 320" data-viz-static></svg>
           <div class="viz-text" data-viz-text><p>Das Mittel steigt.</p><table><tr><th>Jahr</th></tr></table></div></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "viz/caption-missing"), ["mittel"]);
    assert_eq!(
        verdicts(&r, "viz/caption-missing"),
        [(Outcome::Fail, Severity::Low)]
    );
}

#[test]
fn nested_figcaption_does_not_label() {
    let doc = convention(
        r#"<figure id="tief" data-viz="diagram"><div><figcaption>Zu tief</figcaption></div><div data-viz-text>Aussage</div></figure>"#,
    );
    assert_eq!(
        findings_at(&doc, &run(&doc), "viz/caption-missing"),
        ["tief"]
    );
}

// --- viz/static-missing ------------------------------------------------------

#[test]
fn still_image_only_for_moving_and_operable() {
    let doc = convention(
        r#"<figure id="drei" data-viz="3D"><canvas></canvas><div data-viz-text>A</div><figcaption>A</figcaption></figure>
           <figure id="mit" data-viz="interactive"><div data-viz-static><svg viewBox="0 0 640 320" aria-hidden="true"></svg></div><div data-viz-text>B</div><figcaption>B</figcaption></figure>
           <figure id="diagramm" data-viz="diagram"><svg viewBox="0 0 640 320" aria-hidden="true"></svg><div data-viz-text>C</div><figcaption>C</figcaption></figure>"#,
    );
    assert_eq!(
        findings_at(&doc, &run(&doc), "viz/static-missing"),
        ["drei"]
    );
}

// --- viz/table-missing -------------------------------------------------------

#[test]
fn sparkline_without_table_geographia() {
    // Die Sparklines in Geographia (<figure class="spark">) nennen Anfang,
    // Ende und Trend in Worten, ohne Tabelle: ein Fall zum Prüfen.
    let doc = convention(
        r#"<figure id="spark" class="spark" data-viz="chart"><svg viewBox="0 0 200 40" aria-hidden="true" data-viz-static></svg>
           <div data-viz-text><p>Von 280 ppm auf 424 ppm, seit 1960 steiler.</p></div><figcaption>CO₂</figcaption></figure>
           <figure id="mit" data-viz="chart"><svg viewBox="0 0 640 320" aria-hidden="true" data-viz-static></svg>
           <div data-viz-text><table><tr><th>Jahr</th><th>ppm</th></tr></table></div><figcaption>CO₂</figcaption></figure>"#,
    );
    let r = run(&doc);
    assert_eq!(findings_at(&doc, &r, "viz/table-missing"), ["spark"]);
    assert_eq!(
        verdicts(&r, "viz/table-missing"),
        [(Outcome::Review, Severity::Low)]
    );
}

#[test]
fn table_only_for_charts() {
    let doc = convention(CHART);
    assert!(verdicts(&run(&doc), "viz/table-missing").is_empty());
}

// --- display/toggle-missing --------------------------------------------------

#[test]
fn without_toggle_geographia_lab_page() {
    // atmosphere/labor/globus/ (noindex): Visualisierung, Initialisierung,
    // aber keine Kopfzeile mit Umschalter.
    let doc = document("", INIT, &format!("<main>{CHART}</main>"));
    let r = run(&doc);
    assert_eq!(
        verdicts(&r, "display/toggle-missing"),
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
fn toggle_without_visualisation_needs_nothing() {
    let doc = document("", "", "<main><p>Nur Text</p></main>");
    assert!(verdicts(&run(&doc), "display/toggle-missing").is_empty());
}

// --- display/init-missing ----------------------------------------------------

#[test]
fn initialisation_in_head_geographia() {
    // Geographia: display-init.*.js als klassisches Skript in <head>.
    let doc = convention(CHART);
    assert!(verdicts(&run(&doc), "display/init-missing").is_empty());

    let inline = document(
        "",
        r#"<script>document.documentElement.dataset.display = localStorage.getItem("display") || "visual";</script>"#,
        &format!("{TOGGLE}<main>{CHART}</main>"),
    );
    assert!(verdicts(&run(&inline), "display/init-missing").is_empty());

    let set = document(
        r#" data-display="calm""#,
        "",
        &format!("{TOGGLE}<main>{CHART}</main>"),
    );
    assert!(verdicts(&run(&set), "display/init-missing").is_empty());
}

#[test]
fn without_blocking_script_before_body() {
    for head in [
        "",
        r#"<script src="/display-init.js" defer></script>"#,
        r#"<script src="/display-init.js" async></script>"#,
        r#"<script type="module" src="/display-init.js"></script>"#,
        r#"<script type="application/ld+json">{}</script>"#,
    ] {
        let doc = document("", head, &format!("{TOGGLE}<main>{CHART}</main>"));
        assert_eq!(
            verdicts(&run(&doc), "display/init-missing"),
            [(Outcome::Review, Severity::Low)],
            "{head}"
        );
    }

    // Ein Skript erst in <body> kommt zu spät.
    let in_body = document(
        "",
        "",
        &format!("<script src=\"/display-init.js\"></script>{TOGGLE}<main>{CHART}</main>"),
    );
    assert_eq!(verdicts(&run(&in_body), "display/init-missing").len(), 1);
}

#[test]
fn toggle_alone_suffices_for_init() {
    // Wie in auditmysite: Auch ein Umschalter allein ist Konvention.
    let doc = document("", "", &format!("{TOGGLE}<main></main>"));
    assert_eq!(verdicts(&run(&doc), "display/init-missing").len(), 1);
}

#[cfg(feature = "de")]
#[test]
fn findings_follow_the_language() {
    let doc = document("", "", r#"<main><figure data-viz="chart"></figure></main>"#);
    let r = a11y_rules::run_in(&doc, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "display/toggle-missing")
        .unwrap();
    assert!(f.message.contains("Umschalter"), "{}", f.message);
}
