//! Ein Host für den Korpus, der alle Tiers aus dem HTML eines Falls bedient.
//!
//! - **Struktur:** `html5-parser` → `Arena`. Ein `<template shadowrootmode>`
//!   wird zur Shadow Root seines Elternelements; die Kinder eines
//!   `<x-slotted>` darin gelten als per `<slot>` eingesetzter Inhalt des
//!   äußeren Bereichs.
//! - **Semantik:** Rolle und Name über `accname` (mit Stilen), `is_ignored`
//!   bei `aria-hidden="true"` oder `inert` am Knoten oder einem Vorfahren.
//! - **Darstellung:** aus dem `style`-Attribut, vererbt wie in CSS, darunter
//!   die UA-Vorgaben für `display` (`hidden` → `none`). Was CSS nicht als
//!   Eigenschaft kennt, kommt über eigene Eigenschaften:
//!   `--x`/`--y`/`--w`/`--h` (Kasten, Vorgabe 0/0/100/30),
//!   `--obscured`, `--hides-focus`, `--focus-visible`, `--animating`,
//!   `--infinite-animation`, `--flex-reversed`, `--visually-hidden` (je `1`/`0`),
//!   `--scroll-overflow` (px), `--backdrop` (Leuchtdichten, kommagetrennt),
//!   `--bg-unknown: 1` (effektiver Hintergrund nicht auflösbar).
//!   `--unmeasured` am `<html>` nennt Felder, die der Host an keinem Element
//!   misst: `layout`, `focus-visible`, `obscured`, `order`, `min-width`,
//!   `cursor`, `animation`, `pointer-events`, `hides-focus`, `scroll`,
//!   `visually-hidden`, `style`, `bounds`, `unrendered` (kein Stil für nicht
//!   Dargestelltes, wie ein Host aus einem Layout-Schnappschuss).
//! - **Stylesheets:** der Text aller `<style>`-Elemente, in Dokumentreihenfolge.

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, Backdrop, Color, ComputedStyle, Document, Layout, Node,
    NodeKind, Rect, Rendering, Semantics, ancestors, elements,
};
use accname::IdIndex;
use html5_parser::NodeKind as H;

/// Ein Fall als Dokument. Ein Fragment kommt in ein vollständiges Gerüst
/// (Sprache, Titel, Viewport), ein ganzes Dokument (`<!DOCTYPE` oder
/// `<html`) bleibt, wie es ist.
pub fn dokument(src: &str) -> Arena {
    let s = src.trim_start();
    // Ein Fremddokument (`<?xml …?>` vorweg, etwa ein SVG-Dokument): Wurzel ist
    // sein erstes Element, nicht ein `<html>` — für Regeln, die nur auf
    // HTML-Dokumente anwendbar sind.
    if let Some(rest) = s.strip_prefix("<?xml") {
        let rest = rest.split_once("?>").map_or(rest, |(_, r)| r);
        let doc = html5_parser::parse(rest).document;
        fn erstes(
            d: &html5_parser::Document,
            id: html5_parser::NodeId,
        ) -> Option<html5_parser::NodeId> {
            d.children(id).find_map(|c| match &d.node(c).kind {
                H::Element { name, .. } if !matches!(name.as_str(), "html" | "head" | "body") => {
                    Some(c)
                }
                _ => erstes(d, c),
            })
        }
        let wurzel = erstes(&doc, doc.root()).expect("Fremddokument ohne Element");
        return uebernehmen(&doc, wurzel, Arena::builder()).build();
    }
    let quelle =
        if s.starts_with("<!DOCTYPE") || s.starts_with("<!doctype") || s.starts_with("<html") {
            src.to_string()
        } else {
            format!(
                "<!DOCTYPE html><html lang=\"en\"><head><title>Test</title>\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"></head>\
             <body>{src}</body></html>"
            )
        };
    let doc = html5_parser::parse(&quelle).document;
    let wurzel = doc
        .children(doc.root())
        .find(|&c| matches!(doc.node(c).kind, H::Element { .. }))
        .expect("Dokument ohne Wurzelelement");
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
        } if name == "template" && attributes.iter().any(|a| a.name == "shadowrootmode") => {
            let mut b = b.shadow_root();
            for k in doc.children(id) {
                // Der Inhalt eines `template` hängt unter einem Fragment.
                for kk in doc.children(k) {
                    b = uebernehmen(doc, kk, b);
                }
            }
            b.end_scope()
        }
        H::Element { name, .. } if name == "x-slotted" => {
            let mut b = b.slotted();
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.end_scope()
        }
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
        H::DocumentFragment => {
            let mut b = b;
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b
        }
        H::Text { content } => b.text(content),
        _ => b,
    }
}

pub struct Host<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
    ungemessen: Vec<String>,
}

impl<'a> Host<'a> {
    pub fn new(doc: &'a Arena) -> Self {
        let ungemessen = deklarationen(doc.root())
            .into_iter()
            .find(|(k, _)| k == "--unmeasured")
            .map(|(_, v)| v.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        Host {
            ids: IdIndex::build(doc.root()),
            doc,
            ungemessen,
        }
    }

    fn misst(&self, feld: &str) -> bool {
        !self.ungemessen.iter().any(|u| u == feld)
    }

    /// Die Stylesheets der Seite: der Text jedes `<style>`.
    pub fn stylesheets(&self) -> Vec<stylesheet_parse::Stylesheet> {
        elements(self.doc)
            .filter(|n| n.is_element("style"))
            .map(|n| {
                let text: String = n
                    .children()
                    .filter(|k| k.kind() == NodeKind::Text)
                    .map(|k| k.text())
                    .collect();
                stylesheet_parse::parse_stylesheet(&text)
            })
            .collect()
    }
}

impl Document for Host<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for Host<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name_rendered(self, node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        std::iter::once(node)
            .chain(ancestors(node))
            // HTML „inert": inerte Knoten stellt der Browser der
            // Assistenztechnik nicht aus.
            .any(|n| n.attr("aria-hidden") == Some("true") || n.has_attr("inert"))
    }
}

// --- CSS aus dem style-Attribut ---------------------------------------------

/// Die Deklarationen des `style`-Attributs, Namen klein geschrieben.
fn deklarationen<'a>(n: ArenaNode<'a>) -> Vec<(String, String)> {
    n.attr("style")
        .unwrap_or("")
        .split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            Some((k.trim().to_ascii_lowercase(), v.trim().to_string()))
        })
        .collect()
}

/// Der Wert einer Eigenschaft am Element selbst.
fn eigen(n: ArenaNode<'_>, prop: &str) -> Option<String> {
    deklarationen(n)
        .into_iter()
        .rev()
        .find(|(k, _)| k == prop)
        .map(|(_, v)| v)
}

/// Der Wert einer vererbten Eigenschaft: am Element oder am nächsten Vorfahren.
fn geerbt(n: ArenaNode<'_>, prop: &str) -> Option<String> {
    std::iter::once(n)
        .chain(ancestors(n))
        .find_map(|a| eigen(a, prop))
}

fn schalter(n: ArenaNode<'_>, prop: &str) -> bool {
    eigen(n, prop).is_some_and(|v| v == "1" || v == "true")
}

fn zahl(v: &str) -> Option<f32> {
    let v = v.trim();
    if let Some(p) = v.strip_suffix("px") {
        return p.trim().parse().ok();
    }
    if let Some(p) = v.strip_suffix("pt") {
        return p.trim().parse::<f32>().ok().map(|x| x * 4.0 / 3.0);
    }
    v.parse().ok()
}

pub fn farbe(v: &str) -> Option<Color> {
    let v = v.trim().to_ascii_lowercase();
    let c = |r, g, b, a| Some(Color { r, g, b, a });
    match v.as_str() {
        "transparent" => return c(0, 0, 0, 0),
        "black" => return c(0, 0, 0, 255),
        "white" => return c(255, 255, 255, 255),
        "red" => return c(255, 0, 0, 255),
        "blue" => return c(0, 0, 255, 255),
        "gray" | "grey" => return c(128, 128, 128, 255),
        _ => {}
    }
    if let Some(hex) = v.strip_prefix('#') {
        let k = |s: &str| u8::from_str_radix(s, 16).ok();
        return match hex.len() {
            3 => {
                let d = |i: usize| k(&hex[i..=i].repeat(2));
                c(d(0)?, d(1)?, d(2)?, 255)
            }
            6 => c(k(&hex[0..2])?, k(&hex[2..4])?, k(&hex[4..6])?, 255),
            8 => c(
                k(&hex[0..2])?,
                k(&hex[2..4])?,
                k(&hex[4..6])?,
                k(&hex[6..8])?,
            ),
            _ => None,
        };
    }
    let innen = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let teile: Vec<&str> = innen
        .split([',', ' ', '/'])
        .filter(|s| !s.is_empty())
        .collect();
    let kanal = |s: &str| s.trim().parse::<u8>().ok();
    let alpha = match teile.get(3) {
        Some(a) => (a.trim().parse::<f32>().ok()? * 255.0).round() as u8,
        None => 255,
    };
    c(kanal(teile[0])?, kanal(teile[1])?, kanal(teile[2])?, alpha)
}

fn ueber(vorn: Color, hinten: Color) -> Color {
    let a = f32::from(vorn.a) / 255.0;
    let m = |v: u8, h: u8| (f32::from(v) * a + f32::from(h) * (1.0 - a)).round() as u8;
    Color {
        r: m(vorn.r, hinten.r),
        g: m(vorn.g, hinten.g),
        b: m(vorn.b, hinten.b),
        a: 255,
    }
}

/// `display` nach dem UA-Stylesheet (HTML, „Rendering"), soweit die Fälle es
/// brauchen.
fn ua_display(n: ArenaNode<'_>) -> &'static str {
    if n.has_attr("hidden") {
        return "none";
    }
    match n.local_name() {
        "head" | "title" | "style" | "script" | "meta" | "link" | "template" | "noscript"
        | "datalist" | "area" | "param" | "source" | "track" => "none",
        "audio" if !n.has_attr("controls") => "none",
        "dialog" if !n.has_attr("open") => "none",
        "a" | "span" | "strong" | "em" | "b" | "i" | "u" | "s" | "code" | "abbr" | "small"
        | "sub" | "sup" | "mark" | "q" | "cite" | "label" | "time" | "kbd" | "var" | "bdi"
        | "bdo" | "dfn" | "data" | "img" | "svg" | "br" | "wbr" | "input" | "button" | "select"
        | "textarea" | "output" | "iframe" | "embed" | "object" | "video" | "canvas" | "audio"
        | "picture" | "meter" | "progress" => "inline",
        "li" => "list-item",
        "table" => "table",
        "tr" => "table-row",
        "td" | "th" => "table-cell",
        "thead" => "table-header-group",
        "tbody" => "table-row-group",
        "tfoot" => "table-footer-group",
        "caption" => "table-caption",
        _ => "block",
    }
}

impl Host<'_> {
    /// Der effektive Hintergrund: Farben vom Element nach oben übereinander
    /// gelegt bis zur ersten deckenden, darunter Weiß. `None`, wenn ein
    /// Hintergrundbild oder `--bg-unknown` im Weg ist.
    fn hintergrund(&self, n: ArenaNode<'_>) -> Option<Color> {
        let mut schichten = Vec::new();
        for a in std::iter::once(n).chain(ancestors(n)) {
            if schalter(a, "--bg-unknown") || eigen(a, "background-image").is_some() {
                return None;
            }
            if let Some(c) = eigen(a, "background-color")
                .or_else(|| eigen(a, "background"))
                .and_then(|v| farbe(&v))
            {
                schichten.push(c);
                if c.a == 255 {
                    break;
                }
            }
        }
        let mut c = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        for s in schichten.into_iter().rev() {
            c = ueber(s, c);
        }
        Some(c)
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        if !self.misst("style") {
            return None;
        }
        // Ein Host aus einem Layout-Schnappschuss kennt für nicht Dargestelltes
        // keinen Stil.
        if !self.misst("unrendered")
            && eigen(node, "display").unwrap_or_else(|| ua_display(node).into()) == "none"
        {
            return None;
        }
        let deko = eigen(node, "text-decoration-line")
            .or_else(|| eigen(node, "text-decoration"))
            .unwrap_or_else(|| {
                if node.is_element("a") && node.has_attr("href") {
                    "underline".into()
                } else {
                    "none".into()
                }
            });
        let gewicht = geerbt(node, "font-weight").map(|w| match w.as_str() {
            "bold" => 700,
            "normal" => 400,
            w => w.parse().unwrap_or(400),
        });
        let fett_nach_ua = matches!(
            node.local_name(),
            "b" | "strong" | "th" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        );
        Some(ComputedStyle {
            color: Some(
                geerbt(node, "color")
                    .and_then(|v| farbe(&v))
                    .unwrap_or(Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    }),
            ),
            background_color: self.hintergrund(node),
            font_size_px: Some(
                geerbt(node, "font-size")
                    .and_then(|v| zahl(&v))
                    .unwrap_or(16.0),
            ),
            font_weight: Some(gewicht.unwrap_or(if fett_nach_ua { 700 } else { 400 })),
            display: Some(eigen(node, "display").unwrap_or_else(|| ua_display(node).into())),
            visibility: Some(geerbt(node, "visibility").unwrap_or_else(|| "visible".into())),
            list_style_type: Some(geerbt(node, "list-style-type").unwrap_or_else(|| {
                if ancestors(node).any(|a| a.is_element("ol")) || node.is_element("ol") {
                    "decimal".into()
                } else {
                    "disc".into()
                }
            })),
            text_decoration_line: Some(deko),
            font_style: Some(geerbt(node, "font-style").unwrap_or_else(|| "normal".into())),
            font_family: Some(geerbt(node, "font-family").unwrap_or_else(|| "serif".into())),
            border_bottom_style: Some(
                eigen(node, "border-bottom-style").unwrap_or_else(|| "none".into()),
            ),
            appearance: Some(eigen(node, "appearance").unwrap_or_else(|| "auto".into())),
            border_top_style: Some(
                eigen(node, "border-top-style").unwrap_or_else(|| "none".into()),
            ),
            border_top_width_px: Some(
                eigen(node, "border-top-width")
                    .and_then(|v| zahl(&v))
                    .unwrap_or(0.0),
            ),
            border_top_color: eigen(node, "border-top-color")
                .and_then(|v| farbe(&v))
                .or(Some(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                })),
            own_background_color: Some(
                eigen(node, "background-color")
                    .and_then(|v| farbe(&v))
                    .unwrap_or(Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    }),
            ),
        })
    }

    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect> {
        if !self.misst("bounds") {
            return None;
        }
        let w = |p: &str, d: f32| eigen(node, p).and_then(|v| zahl(&v)).unwrap_or(d);
        Some(Rect {
            x: w("--x", 0.0),
            y: w("--y", 0.0),
            width: w("--w", 100.0),
            height: w("--h", 30.0),
        })
    }

    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        if !self.misst("layout") {
            return None;
        }
        let feld = |name: &str, wert: bool| self.misst(name).then_some(wert);
        let focus_visible = match eigen(node, "--focus-visible").as_deref() {
            Some("unmeasured") => None,
            Some(v) => Some(v == "1" || v == "true"),
            None => Some(true),
        };
        Some(Layout {
            flex_reversed: feld(
                "flex-reversed",
                schalter(node, "--flex-reversed")
                    || eigen(node, "flex-direction").is_some_and(|v| v.ends_with("-reverse")),
            ),
            order: self.misst("order").then(|| {
                eigen(node, "order")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
            }),
            min_width_px: self.misst("min-width").then(|| {
                eigen(node, "min-width")
                    .and_then(|v| zahl(&v))
                    .unwrap_or(0.0)
            }),
            cursor_pointer: feld(
                "cursor",
                geerbt(node, "cursor").is_some_and(|v| v == "pointer"),
            ),
            infinite_animation: feld(
                "animation",
                schalter(node, "--infinite-animation")
                    || eigen(node, "animation-iteration-count").is_some_and(|v| v == "infinite"),
            ),
            obscured: feld("obscured", schalter(node, "--obscured")),
            hides_focus: feld("hides-focus", schalter(node, "--hides-focus")),
            focus_visible: if self.misst("focus-visible") {
                focus_visible
            } else {
                None
            },
            pointer_events_none: feld(
                "pointer-events",
                geerbt(node, "pointer-events").is_some_and(|v| v == "none"),
            ),
            animating: feld("animation", schalter(node, "--animating")),
        })
    }

    fn scroll_overflow_px<'n>(&'n self, node: Self::N<'n>) -> Option<f32> {
        // Was nicht dargestellt wird (`head`, `display: none`), hat keinen
        // Kasten und damit keinen gemessenen Überhang — wie in Chrome.
        let anzeige = eigen(node, "display").unwrap_or_else(|| ua_display(node).into());
        if anzeige == "none" {
            return None;
        }
        self.misst("scroll").then(|| {
            eigen(node, "--scroll-overflow")
                .and_then(|v| zahl(&v))
                .unwrap_or(0.0)
        })
    }

    fn visually_hidden<'n>(&'n self, node: Self::N<'n>) -> Option<bool> {
        self.misst("visually-hidden")
            .then(|| schalter(node, "--visually-hidden"))
    }

    fn sampled_backdrop<'n>(&'n self, node: Self::N<'n>) -> Option<Backdrop> {
        eigen(node, "--backdrop").map(|v| Backdrop {
            luminance: v.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
        })
    }
}
