//! Fähigkeits-Tiers.
//!
//! Die drei Oberflächen unterscheiden sich nicht darin, wie sie dieselben Daten
//! darstellen, sondern darin, **welche Daten es überhaupt gibt**:
//!
//! | | statisches HTML | Chrome via CDP | In-Page WASM |
//! |---|---|---|---|
//! | Struktur ([`Document`]) | ✓ | ✓ | ✓ |
//! | [`Semantics`] | berechnet | nativ | berechnet |
//! | [`Rendering`] | — | ✓ | ✓ |
//! | [`Interaction`] | — | ✓ | ✓ |
//!
//! Ein einziges flaches Trait dafür wäre voller `Option`, die je nach Host
//! `None` liefern — Regeln würden stillschweigend nicht laufen. Stattdessen
//! deklariert jede Regel ihren Tier, jeder Host implementiert die Tiers, die er
//! bedienen kann, und ein nicht erfüllter Tier wird zu `UNTESTED` statt zu
//! Schweigen.
//!
//! [`Document`]: crate::Document

use crate::tree::Document;

/// Welche Datenschicht eine Regel braucht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Tags, Attribute, Text, Hierarchie. Immer verfügbar.
    Structure,
    /// Rolle und Accessible Name.
    Semantics,
    /// Berechnete Stile und Geometrie.
    Rendering,
    /// Fokus, Ereignisse, veränderlicher DOM.
    Interaction,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Structure => "structure",
            Tier::Semantics => "semantics",
            Tier::Rendering => "rendering",
            Tier::Interaction => "interaction",
        }
    }
}

/// Was ein Host liefern kann. [`Tier::Structure`] ist immer dabei — ohne
/// Baum gäbe es nichts zu prüfen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Caps {
    pub semantics: bool,
    pub rendering: bool,
    pub interaction: bool,
}

impl Caps {
    /// Nur Struktur — der statische Fall.
    pub const STRUCTURE_ONLY: Caps = Caps {
        semantics: false,
        rendering: false,
        interaction: false,
    };

    pub fn has(self, tier: Tier) -> bool {
        match tier {
            Tier::Structure => true,
            Tier::Semantics => self.semantics,
            Tier::Rendering => self.rendering,
            Tier::Interaction => self.interaction,
        }
    }

    pub fn with_semantics(mut self) -> Self {
        self.semantics = true;
        self
    }

    pub fn with_rendering(mut self) -> Self {
        self.rendering = true;
        self
    }

    pub fn with_interaction(mut self) -> Self {
        self.interaction = true;
        self
    }
}

/// Woher ein Accessible Name stammt. Nur der native Accessibility-Tree kennt
/// das; berechnende Hosts lassen es bei `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    AriaLabel,
    AriaLabelledBy,
    Label,
    Title,
    Alt,
    Placeholder,
    Contents,
    Value,
}

/// **Tier 2** — Rolle und Accessible Name.
///
/// auditmysite liefert beides nativ aus dem Accessibility-Tree des Browsers,
/// inklusive [`NameSource`] und `is_ignored`. astro-post-audit und LiveAudit
/// berechnen es über das `accname`-Crate; dort bleibt `name_source` `None`.
/// Die Lebenszeit des Knotens ist an die Ausleihe des Dokuments gekoppelt
/// (`&'n self`, `Self::N<'n>`). Ohne diese Kopplung könnte ein Host, der seinen
/// Baum nur *ausleiht* und daneben abgeleitete Daten hält — etwa einen
/// vorberechneten ID-Index —, das Trait gar nicht erfüllen: `Self: 'n` wäre
/// nicht herleitbar.
pub trait Semantics: Document {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String>;

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String>;

    fn name_source<'n>(&'n self, _node: Self::N<'n>) -> Option<NameSource> {
        None
    }

    /// Ob der Accessibility-Tree diesen Knoten auslässt — etwa wegen
    /// `aria-hidden`, `display: none` oder weil er rein präsentational ist.
    fn is_ignored<'n>(&'n self, _node: Self::N<'n>) -> bool {
        false
    }
}

/// Ein Rechteck in CSS-Pixeln, Ursprung links oben im Dokument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// sRGB mit Alpha, Kanäle 0–255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Die berechneten Stilwerte, die Accessibility-Regeln tatsächlich brauchen.
/// Bewusst keine vollständige CSSOM-Abbildung.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub color: Option<Color>,
    /// Die *effektive* Hintergrundfarbe — der Host löst Transparenz über die
    /// Vorfahren auf. Eine reine `background-color`-Ablesung pro Knoten wäre
    /// für Kontrastprüfungen unbrauchbar.
    pub background_color: Option<Color>,
    pub font_size_px: Option<f32>,
    pub font_weight: Option<u16>,
    pub display: Option<String>,
    pub visibility: Option<String>,
}

/// **Tier 3** — berechnete Stile und Geometrie.
///
/// Statische HTML-Analyse kann das nicht; Kontrast-, Target-Size- und
/// Reflow-Regeln melden dort `UNTESTED`.
pub trait Rendering: Document {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle>;

    fn bounds<'n>(&'n self, node: Self::N<'n>) -> Option<Rect>;

    /// Ob der Knoten tatsächlich sichtbar gerendert wird — nicht dasselbe wie
    /// „steht im Markup".
    fn is_rendered<'n>(&'n self, node: Self::N<'n>) -> bool {
        self.bounds(node).is_some_and(|b| !b.is_empty())
    }
}

/// **Tier 4** — Fokus, Ereignisse, veränderlicher DOM.
///
/// In-Page ist das billig und genau, weil der Prüfer *in* der Seite sitzt und
/// Ereignisse real auslösen kann. Über CDP geht es auch, kostet aber einen
/// Roundtrip je Schritt.
pub trait Interaction: Document {
    /// Die tatsächliche Tabreihenfolge, in der Reihenfolge des Durchlaufs.
    fn tab_order(&self) -> Vec<crate::NodeId>;

    /// Ob der Knoten bei Fokus einen sichtbaren Indikator zeigt.
    fn has_visible_focus<'n>(&'n self, _node: Self::N<'n>) -> Option<bool> {
        None
    }
}
