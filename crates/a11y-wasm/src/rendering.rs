//! Tier 3 über der Arena: berechnete Stile, wie der Collector sie liefert.
//!
//! Dieses Modul rechnet nichts aus. Es reicht durch, was `packages/browser`
//! in einem eigenen Durchgang gesammelt hat — insbesondere den **effektiven**
//! Hintergrund, den der Collector über die Vorfahren auflöst. Genau das
//! verlangt `a11y_dom::ComputedStyle::background_color`, und genau dort sitzt
//! auch die Merkliste, ohne die Tier 3 nach der Messung vom 20.09.2026 nicht
//! tragbar wäre.
//!
//! Layout und Geometrie kommen in eigenen Spalten ([`LayoutColumns`]) für die
//! heuristischen Regeln. Geometrie (`bounds`) erhebt der Collector nur an
//! Bedienelementen: `getBoundingClientRect()` je Knoten kostet laut derselben
//! Messung so viel wie der ganze Collector.

use a11y_dom::{
    Color, ComputedStyle, Document, Layout, NameSource, Node, Rect, Rendering, Semantics,
};

use crate::arena::{Arena, ArenaNode};
use crate::semantics::SemanticArena;

/// Bit 0: `display: none`. Bit 1: `visibility: hidden`. Bit 2: Stil erfasst.
const FLAG_DISPLAY_NONE: u8 = 1;
const FLAG_VISIBILITY_HIDDEN: u8 = 2;
const FLAG_ERFASST: u8 = 4;

/// Ein Farbwert aus dem Collector, gepackt als `0xRRGGBBAA`. `0` heißt
/// „nicht bestimmbar" — nicht „schwarz und durchsichtig".
fn farbe(gepackt: u32) -> Option<Color> {
    if gepackt == 0 {
        return None;
    }
    Some(Color {
        r: (gepackt >> 24) as u8,
        g: (gepackt >> 16) as u8,
        b: (gepackt >> 8) as u8,
        a: gepackt as u8,
    })
}

/// Die Tier-3-Spalten, parallel zu den Arena-Indizes.
pub struct RenderingColumns {
    pub color: Vec<u32>,
    pub background: Vec<u32>,
    pub font_size_px: Vec<f32>,
    pub font_weight: Vec<u16>,
    pub flags: Vec<u8>,
}

/// Bits der Layout-Spalte. Ohne `LAYOUT_ERFASST` hat der Collector für den
/// Knoten nichts erhoben.
const LAYOUT_ERFASST: u8 = 1;
const LAYOUT_UMGEKEHRT: u8 = 2;
const LAYOUT_ZEIGER: u8 = 4;
const LAYOUT_ENDLOS: u8 = 8;
const LAYOUT_VERDECKT: u8 = 16;
const LAYOUT_FOKUS_GEMESSEN: u8 = 32;
const LAYOUT_FOKUS_SICHTBAR: u8 = 64;
const LAYOUT_VERSTECKT_FOKUS: u8 = 128;

/// Die Layout-Spalten für die heuristischen Regeln, parallel zu den
/// Arena-Indizes. `bounds` hält vier Werte je Knoten (x, y, Breite, Höhe);
/// eine Breite `NaN` heißt „nicht erhoben" — der Collector misst Geometrie nur
/// an Bedienelementen, weil sie je Knoten so viel kostet wie der ganze Scan.
pub struct LayoutColumns {
    pub flags: Vec<u8>,
    pub order: Vec<i32>,
    pub min_width_px: Vec<f32>,
    pub bounds: Vec<f32>,
}

/// Die Arena plus Semantik plus Darstellung — der vollständige Host.
///
/// Erfüllt `Document`, `Semantics` und `Rendering` und ist damit der Fall, für
/// den `a11y_rules::run_full` gedacht ist: Kein Tier fehlt, der Bericht enthält
/// keinen `CapabilityMissing`-Vermerk mehr.
pub struct RenderArena<'a> {
    semantik: SemanticArena<'a>,
    spalten: &'a RenderingColumns,
    layout: Option<&'a LayoutColumns>,
}

impl<'a> RenderArena<'a> {
    pub fn new(arena: &'a Arena, spalten: &'a RenderingColumns) -> Self {
        RenderArena {
            semantik: SemanticArena::new(arena),
            spalten,
            layout: None,
        }
    }

    /// Mit den Layout-Spalten für die heuristischen Regeln.
    pub fn with_layout(mut self, layout: &'a LayoutColumns) -> Self {
        self.layout = Some(layout);
        self
    }

    fn erfasst(&self, index: usize) -> bool {
        self.spalten
            .flags
            .get(index)
            .is_some_and(|f| f & FLAG_ERFASST != 0)
    }
}

impl Document for RenderArena<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.semantik.root()
    }

    fn node_count(&self) -> Option<usize> {
        self.semantik.node_count()
    }
}

impl Semantics for RenderArena<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        self.semantik.role(node)
    }

    /// Mit Stil gerechnet: Per `display: none` versteckte Labels und
    /// Teilbäume tragen dann nichts bei, wie im Browser.
    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name_rendered(self, node, &self.semantik.ids)
    }

    fn name_source<'n>(&'n self, node: Self::N<'n>) -> Option<NameSource> {
        self.semantik.name_source(node)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        self.semantik.is_ignored(node)
    }
}

impl Rendering for RenderArena<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        let index = node.id().0 as usize;
        if !self.erfasst(index) {
            return None;
        }
        let flags = self.spalten.flags[index];
        Some(ComputedStyle {
            color: farbe(self.spalten.color[index]),
            background_color: farbe(self.spalten.background[index]),
            font_size_px: Some(self.spalten.font_size_px[index]).filter(|px| *px > 0.0),
            font_weight: Some(self.spalten.font_weight[index]).filter(|w| *w > 0),
            display: Some(if flags & FLAG_DISPLAY_NONE != 0 {
                "none".to_string()
            } else {
                "block".to_string()
            }),
            visibility: Some(if flags & FLAG_VISIBILITY_HIDDEN != 0 {
                "hidden".to_string()
            } else {
                "visible".to_string()
            }),
        })
    }

    /// Nur an Bedienelementen erhoben, und nur mit Layout-Spalten.
    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect> {
        let i = node.id().0 as usize * 4;
        let b = self.layout?.bounds.get(i..i + 4)?;
        if b[2].is_nan() {
            return None;
        }
        Some(Rect {
            x: b[0],
            y: b[1],
            width: b[2],
            height: b[3],
        })
    }

    fn layout<'n>(&'n self, node: Self::N<'n>) -> Option<Layout> {
        let spalten = self.layout?;
        let i = node.id().0 as usize;
        let f = *spalten.flags.get(i)?;
        if f & LAYOUT_ERFASST == 0 {
            return None;
        }
        Some(Layout {
            flex_reversed: f & LAYOUT_UMGEKEHRT != 0,
            order: spalten.order[i],
            min_width_px: spalten.min_width_px[i],
            cursor_pointer: f & LAYOUT_ZEIGER != 0,
            infinite_animation: f & LAYOUT_ENDLOS != 0,
            obscured: f & LAYOUT_VERDECKT != 0,
            hides_focus: f & LAYOUT_VERSTECKT_FOKUS != 0,
            focus_visible: (f & LAYOUT_FOKUS_GEMESSEN != 0)
                .then_some(f & LAYOUT_FOKUS_SICHTBAR != 0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::build;

    /// Baut Spalten, in denen jedes Element dieselben Farben trägt.
    fn spalten(nodes: usize, vorn: u32, hinten: u32) -> RenderingColumns {
        RenderingColumns {
            color: vec![vorn; nodes],
            background: vec![hinten; nodes],
            font_size_px: vec![16.0; nodes],
            font_weight: vec![400; nodes],
            flags: vec![FLAG_ERFASST; nodes],
        }
    }

    fn dokument() -> Arena {
        build(|b| {
            b.open("html")
                .attr("lang", "de")
                .open("head")
                .open("title")
                .text("Seite")
                .close()
                .close()
                .open("body")
                .open("h1")
                .text("Titel")
                .close()
                .open("p")
                .text("Ein Absatz")
                .close()
                .close()
                .close();
        })
    }

    #[test]
    fn schwacher_kontrast_wird_gemeldet() {
        let arena = dokument();
        // #999999 auf Weiß: 2,85:1.
        let s = spalten(arena.len(), 0x9999_99ff, 0xffff_ffff);
        let host = RenderArena::new(&arena, &s);
        let report = a11y_rules::run_full(&host);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == "contrast/text-insufficient"),
            "unerwartet: {:?}",
            report
                .findings
                .iter()
                .map(|f| &f.rule_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn guter_kontrast_meldet_nichts() {
        let arena = dokument();
        let s = spalten(arena.len(), 0x3333_33ff, 0xffff_ffff);
        let host = RenderArena::new(&arena, &s);
        let report = a11y_rules::run_full(&host);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.rule_id == "contrast/text-insufficient")
        );
    }

    /// Der fachliche Kern: Was der Collector nicht auflösen konnte, wird
    /// `UNTESTED` — nicht stillschweigend bestanden.
    #[test]
    fn unbestimmbarer_hintergrund_wird_untested() {
        let arena = dokument();
        let s = spalten(arena.len(), 0x0000_00ff, 0);
        let host = RenderArena::new(&arena, &s);
        let report = a11y_rules::run_full(&host);
        let befund = report
            .findings
            .iter()
            .find(|f| f.rule_id == "contrast/text-undetermined")
            .expect("muss einen Befund erzeugen");
        assert_eq!(befund.outcome, a11y_report::Outcome::Untested);
    }

    /// Mit vollständigem Tier-Satz bleiben nur die Regeln über Stylesheets
    /// als „nicht gelaufen" vermerkt — dieses Paket gibt noch keine weiter.
    #[test]
    fn run_full_laesst_keine_regel_ungeprueft() {
        let arena = dokument();
        let s = spalten(arena.len(), 0x3333_33ff, 0xffff_ffff);
        let host = RenderArena::new(&arena, &s);
        let report = a11y_rules::run_full(&host);
        let offen: Vec<&str> = report
            .rule_runs
            .iter()
            .filter(|r| !r.did_run())
            .map(|r| r.rule_id.as_str())
            .collect();
        let stile: Vec<&str> = a11y_rules::stylesheet_metas()
            .iter()
            .flat_map(|m| m.ids.iter().copied())
            .collect();
        assert_eq!(offen, stile);
    }

    /// Die Layout-Spalten kommen bei den Regeln an: jedes Feld einmal gefüllt
    /// gesehen, nicht nur deklariert.
    #[test]
    fn layout_spalten_erreichen_die_heuristiken() {
        let arena = dokument();
        let n = arena.len();
        let s = spalten(n, 0x3333_33ff, 0xffff_ffff);
        let absatz = elements_mit_tag(&arena, "p");
        let mut layout = LayoutColumns {
            flags: vec![LAYOUT_ERFASST; n],
            order: vec![0; n],
            min_width_px: vec![0.0; n],
            bounds: vec![f32::NAN; n * 4],
        };
        layout.min_width_px[absatz] = 640.0;
        layout.flags[absatz] |= LAYOUT_ENDLOS | LAYOUT_ZEIGER;
        let host = RenderArena::new(&arena, &s).with_layout(&layout);
        let report = a11y_rules::run_full(&host);
        let ids: Vec<&str> = report.findings.iter().map(|f| f.rule_id.as_str()).collect();
        for erwartet in [
            "reflow/min-width",
            "motion/infinite-animation",
            "keyboard/pointer-only",
        ] {
            assert!(ids.contains(&erwartet), "{erwartet} fehlt in {ids:?}");
        }
    }

    fn elements_mit_tag(arena: &Arena, tag: &str) -> usize {
        a11y_dom::elements(arena)
            .find(|n| n.local_name() == tag)
            .map(|n| n.id().0 as usize)
            .unwrap()
    }
}
