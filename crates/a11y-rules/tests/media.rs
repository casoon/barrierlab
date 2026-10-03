//! Bild- und Medienregeln (casoon/barrierlab#19).
//!
//! Die Fälle stammen aus auditmysites Detection-Korpus
//! (`tests/fixtures/detection_corpus/`) und aus echten Seiten (Abruf
//! 2026-10-03); der Name jedes Tests nennt die Quelle. Wo ein Korpusfall ein
//! Element ohne `id` prüft, ist eine `id` ergänzt, damit der Test den Befund
//! verorten kann. Alle Fälle laufen mit Semantik aus `accname`, weil
//! `frames/name-missing` Tier 2 ist; die übrigen Regeln sind Tier 1 und laufen
//! dabei mit.

use a11y_dom::{Arena, ArenaBuilder, ArenaNode, Document, Node, Semantics};
use a11y_report::{Outcome, Report, Severity};
use a11y_rules::run_with_semantics_in;
use accname::IdIndex;
use html5_parser::NodeKind as H;

// --- Hilfen ---------------------------------------------------------------

fn html(body: &str) -> Arena {
    let doc = html5_parser::parse(&format!(
        "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body><main>{body}</main></body></html>"
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

struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

fn pruefe_in(src: &str, locale: a11y_rules::Locale) -> Report {
    let a = html(src);
    run_with_semantics_in(
        &MitSemantik {
            doc: &a,
            ids: IdIndex::build(a.root()),
        },
        locale,
    )
}

fn pruefe(src: &str) -> Report {
    pruefe_in(src, a11y_rules::Locale::En)
}

fn urteile(r: &Report, id: &str) -> Vec<(Outcome, Severity)> {
    r.findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(|f| (f.outcome, f.severity))
        .collect()
}

/// Die Befunde einer Kennung, verortet an der `id` des Elements.
fn an(src: &str, r: &Report, rule: &str) -> Vec<String> {
    let a = html(src);
    let mut v: Vec<String> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .filter_map(|f| {
            let knoten = f.location.node.as_deref()?;
            a11y_dom::self_and_descendants(a.root())
                .find(|n| n.id().to_string() == knoten)
                .map(|n| n.attr("id").unwrap_or(n.local_name()).to_string())
        })
        .collect();
    v.sort();
    v
}

// --- images/area-alt-missing, images/input-alt-missing ----------------------

/// Korpus `misc_content_checks`.
const MISC_CONTENT_CHECKS: &str = r##"
    <img src="/floorplan.png" usemap="#floor-map" alt="Floor plan">
    <map name="floor-map">
        <area id="raum" href="/room1" shape="rect" coords="0,0,50,50">
    </map>
    <input id="absenden" type="image" src="/submit.png">"##;

#[test]
fn korpus_misc_content_checks() {
    let r = pruefe(MISC_CONTENT_CHECKS);
    assert_eq!(
        an(MISC_CONTENT_CHECKS, &r, "images/area-alt-missing"),
        ["raum"]
    );
    assert_eq!(
        urteile(&r, "images/area-alt-missing"),
        [(Outcome::Fail, Severity::High)]
    );
    assert_eq!(
        an(MISC_CONTENT_CHECKS, &r, "images/input-alt-missing"),
        ["absenden"]
    );
    assert_eq!(
        urteile(&r, "images/input-alt-missing"),
        [(Outcome::Fail, Severity::High)]
    );
    // Der Ersatzname „Submit Query" verdeckt das fehlende alt —
    // buttons/name-missing schweigt, deshalb die eigene Regel.
    assert!(urteile(&r, "buttons/name-missing").is_empty());
}

#[test]
fn benannte_bereiche_und_grafik_buttons_auditmysite_image_input_rules() {
    let src = r##"
        <span id="ziel">Raum 2</span>
        <map name="m">
            <area id="mit-alt" href="/r1" alt="Raum 1">
            <area id="leeres-alt" href="/r2" alt=" ">
            <area id="labelledby" href="/r2" aria-labelledby="ziel">
            <area id="inaktiv" shape="rect" coords="0,0,1,1">
        </map>
        <input id="i-alt" type="image" src="/a.png" alt="Suchen">
        <input id="i-label" type="IMAGE" src="/b.png" aria-label="Senden">
        <input id="i-leer" type="image" src="/c.png" alt="">"##;
    let r = pruefe(src);
    assert_eq!(an(src, &r, "images/area-alt-missing"), ["leeres-alt"]);
    assert_eq!(an(src, &r, "images/input-alt-missing"), ["i-leer"]);
}

// --- images/server-side-map -------------------------------------------------

#[test]
fn korpus_forms_and_misc() {
    let src = r#"<a href="/map"><img id="karte" src="map.png" ismap alt="Site map"></a>
                 <img id="normal" src="x.png" alt="Normal">"#;
    let r = pruefe(src);
    assert_eq!(an(src, &r, "images/server-side-map"), ["karte"]);
    assert_eq!(
        urteile(&r, "images/server-side-map"),
        [(Outcome::Fail, Severity::Medium)]
    );
}

// --- objects/alt-missing ----------------------------------------------------

#[test]
fn korpus_object_no_alt() {
    let src = r#"<object id="pdf" data="/report.pdf" type="application/pdf" width="400" height="300"></object>"#;
    let r = pruefe(src);
    assert_eq!(an(src, &r, "objects/alt-missing"), ["pdf"]);
    assert_eq!(
        urteile(&r, "objects/alt-missing"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn objekte_mit_alternative_auditmysite_image_input_rules() {
    let src = r#"
        <object id="ersatz" data="/a.pdf"><p>Der <a href="/a.pdf">Bericht als PDF</a>.</p></object>
        <object id="label" data="/b.pdf" aria-label="Bericht"></object>
        <object id="titel" data="/c.pdf" title="Bericht"></object>
        <object id="versteckt" data="/d.swf" aria-hidden="true"></object>
        <object id="leer" data="/e.pdf"> </object>"#;
    assert_eq!(an(src, &pruefe(src), "objects/alt-missing"), ["leer"]);
}

// --- media/audio-autoplay ---------------------------------------------------

#[test]
fn korpus_media_and_visual() {
    let src = r#"<audio id="hintergrund" autoplay src="/bg.mp3"></audio>"#;
    let r = pruefe(src);
    assert_eq!(an(src, &r, "media/audio-autoplay"), ["hintergrund"]);
    assert_eq!(
        urteile(&r, "media/audio-autoplay"),
        [(Outcome::Review, Severity::Medium)]
    );
}

#[test]
fn vorlesefunktion_zeit_de() {
    // Echte Seite: zeit.de bietet Artikel zum Anhören an — mit Bedienelementen,
    // ohne autoplay.
    let src = r#"<audio id="vorlesen" class="audio-player__audio" data-src="https://example.org/full.mp3" controls preload="none"></audio>
                 <audio id="stumm" autoplay muted src="/x.mp3"></audio>"#;
    assert!(urteile(&pruefe(src), "media/audio-autoplay").is_empty());
}

// --- frames/name-missing ----------------------------------------------------

#[test]
fn korpus_frame_missing_title() {
    let src = r#"<iframe id="widget" src="/embedded-widget" width="300" height="200"></iframe>"#;
    let r = pruefe(src);
    assert_eq!(an(src, &r, "frames/name-missing"), ["widget"]);
    assert_eq!(
        urteile(&r, "frames/name-missing"),
        [(Outcome::Fail, Severity::High)]
    );
}

#[test]
fn sportdaten_spiegel_de_und_n_tv_de() {
    // Echte Seiten: spiegel.de setzt den Rahmen per Alpine aus einem
    // <template> ein (hier wie dargestellt), n-tv.de mit leerem title.
    let src = r#"
        <iframe id="spiegel" scrolling="no" class="w-full h-full" src="https://sportdaten.spiegel.de/iframe/top-matches/topspiele"></iframe>
        <iframe id="ntv" class="iframeResizer_iframe___MR3N" src="https://sportdaten.n-tv.de/iframe/startseitenticker-fussball/wf0/ls-full/to3377" title="" width="100%" height="20" loading="lazy" scrolling="no" frameBorder="0"></iframe>"#;
    assert_eq!(
        an(src, &pruefe(src), "frames/name-missing"),
        ["ntv", "spiegel"]
    );
}

#[test]
fn benannte_und_unsichtbare_rahmen() {
    // Echte Seiten: w3.org/WAI (title="Video"), faz.net (title),
    // craigslist.org (display:none), duckduckgo.com (0 × 0),
    // auditmysite-Korpus iframe_widget_rules (aria-hidden).
    let src = r#"
        <iframe id="w3c" title="Video" width="560" height="315" src="https://www.youtube-nocookie.com/embed/iWO5N3n1DXU" allowfullscreen=""></iframe>
        <iframe id="faz" title="newsletterslider" src="https://dynamic.faz.net/red/2025/newsletterslider/stage/index.html" frameborder="0" style="width: 100%; height: 200px"></iframe>
        <iframe id="cl-local-storage" src="https://www.craigslist.org/static/www/localStorage.html" style="display:none;"></iframe>
        <iframe id="ddg" name="ifr" width="0" height="0" border="0" class="hidden"></iframe>
        <iframe id="hidden-widget" title="Hidden widget" aria-hidden="true" style="width:300px;height:100px;border:0"></iframe>
        <span id="beschriftung">Karte</span>
        <iframe id="labelledby" src="/karte" aria-labelledby="beschriftung"></iframe>
        <iframe id="praesentation" src="/x" role="presentation"></iframe>"#;
    assert!(an(src, &pruefe(src), "frames/name-missing").is_empty());
}

// --- manual/media-alternatives ----------------------------------------------

#[test]
fn eingebettetes_video_w3_org_wai() {
    // Echte Seite: w3.org/WAI bettet ein YouTube-Video ein. Ob es Untertitel
    // hat, steuert der Player der Plattform — also auf die Checkliste.
    let video = r#"<iframe title="Video" width="560" height="315" src="https://www.youtube-nocookie.com/embed/iWO5N3n1DXU"></iframe>"#;
    assert_eq!(
        urteile(&pruefe(video), "manual/media-alternatives"),
        [(Outcome::Untested, Severity::High)]
    );
    let karte =
        r#"<iframe title="Karte" src="https://maps.example.org/embed?youtube.com"></iframe>"#;
    assert!(urteile(&pruefe(karte), "manual/media-alternatives").is_empty());
}

#[cfg(feature = "de")]
#[test]
fn befunde_folgen_der_sprache() {
    let r = pruefe_in(r#"<iframe src="/x"></iframe>"#, a11y_rules::Locale::De);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "frames/name-missing")
        .unwrap();
    assert!(f.message.contains("Rahmen"), "{}", f.message);
}
