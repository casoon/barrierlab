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

use crate::tree::{Document, NodeId};

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
    /// Die Stylesheets der Seite als geparste Regeln. Unabhängig von den
    /// übrigen Schichten: Ein statischer Host hat sie aus seinen Dateien, ein
    /// Browser aus `document.styleSheets`.
    Stylesheets,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Structure => "structure",
            Tier::Semantics => "semantics",
            Tier::Rendering => "rendering",
            Tier::Interaction => "interaction",
            Tier::Stylesheets => "stylesheets",
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
    pub stylesheets: bool,
}

impl Caps {
    /// Nur Struktur — der statische Fall.
    pub const STRUCTURE_ONLY: Caps = Caps {
        semantics: false,
        rendering: false,
        interaction: false,
        stylesheets: false,
    };

    pub fn has(self, tier: Tier) -> bool {
        match tier {
            Tier::Structure => true,
            Tier::Semantics => self.semantics,
            Tier::Rendering => self.rendering,
            Tier::Interaction => self.interaction,
            Tier::Stylesheets => self.stylesheets,
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

    pub fn with_stylesheets(mut self) -> Self {
        self.stylesheets = true;
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

/// Eine Stichprobe des tatsächlich gezeichneten Hintergrunds hinter einem
/// Text — für die Fälle, in denen keine einzelne Farbe gilt: Verlauf,
/// Hintergrundbild, halbdurchsichtige Überlagerung.
///
/// Der Host tastet die Pixel im Kasten des Elements ab (etwa aus einem
/// Screenshot), verrechnet Transparenz gegen Weiß und gibt ihre relative
/// Leuchtdichte nach WCAG an, 0,0 bis 1,0, in beliebiger Reihenfolge. Die
/// Glyphen selbst dürfen darin sein; die Regel urteilt über Median und
/// 40. Perzentil, nicht über den ungünstigsten Pixel.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Backdrop {
    pub luminance: Vec<f64>,
}

/// Die berechneten Stilwerte, die Accessibility-Regeln tatsächlich brauchen.
/// Bewusst keine vollständige CSSOM-Abbildung.
#[derive(Debug, Clone, Default, PartialEq)]
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
    /// Berechnetes `list-style-type`. An `<ul>`/`<ol>` mit `role="list"`
    /// entscheidet es, ob die Rolle überflüssig ist: Bei `none` nimmt
    /// WebKit der Liste ihre Semantik, die Rolle stellt sie wieder her.
    pub list_style_type: Option<String>,
    /// Berechnetes `text-decoration-line`, etwa `underline` oder `none`.
    pub text_decoration_line: Option<String>,
    /// Berechnetes `font-style`.
    pub font_style: Option<String>,
    /// Berechnetes `font-family`, als Zeichenkette zum Vergleich mit dem
    /// Elternelement — nicht zum Auflösen von Schriften.
    pub font_family: Option<String>,
    /// Berechnetes `border-bottom-style`.
    pub border_bottom_style: Option<String>,
    /// Berechnetes `appearance`. `none` heißt: Der Autor zeichnet das
    /// Bedienelement selbst, sein Kontrast liegt nicht beim Browser.
    pub appearance: Option<String>,
    /// Berechnetes `border-top-style`.
    pub border_top_style: Option<String>,
    /// Berechnetes `border-top-width` in CSS-Pixeln.
    pub border_top_width_px: Option<f32>,
    /// Berechnetes `border-top-color`, mit Alpha.
    pub border_top_color: Option<Color>,
    /// Das eigene `background-color` des Elements, mit Alpha und *nicht*
    /// über die Vorfahren aufgelöst — anders als [`Self::background_color`].
    /// Die Fläche eines selbst gezeichneten Bedienelements.
    pub own_background_color: Option<Color>,
}

/// Layout-Angaben für heuristische Prüfungen — was eine Regel braucht, um eine
/// Barriere zu *vermuten*, nicht um sie zu belegen. Die Regeln darauf melden
/// deshalb `REVIEW`, nie `FAIL`.
///
/// Jedes Feld ist einzeln optional: `None` heißt „nicht gemessen", nicht
/// „nein". So kann ein Host liefern, was er erhebt, ohne dass die Regeln auf
/// den übrigen Feldern mit Vorgabewerten laufen. Eine Regel, deren Feld an
/// keinem Element gemessen ist, meldet das als `UNTESTED`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// Ein Flex-Container mit `flex-direction: row-reverse` oder
    /// `column-reverse`: Die Leserichtung weicht von der Quellreihenfolge ab.
    pub flex_reversed: Option<bool>,
    /// Berechnetes `order`. Ungleich 0 verschiebt das Element gegenüber der
    /// Quellreihenfolge.
    pub order: Option<i32>,
    /// Berechnetes `min-width` in CSS-Pixeln, `0` ohne Angabe.
    pub min_width_px: Option<f32>,
    /// Berechnetes `cursor: pointer` — das Element sieht anklickbar aus.
    pub cursor_pointer: Option<bool>,
    /// Auf dem Element läuft eine Animation mit unendlich vielen Wiederholungen.
    pub infinite_animation: Option<bool>,
    /// Das Element liegt im sichtbaren Bereich, aber seine Mitte wird von
    /// einem fixierten oder klebenden fremden Element überdeckt — etwa einem
    /// Cookie-Banner. Die Kontrastregel meldet überdeckten Text als nicht
    /// bestimmbar.
    pub obscured: Option<bool>,
    /// Ein fixiertes oder klebendes Element am oberen Rand, das tiefer reicht
    /// als `scroll-padding-top`: Was beim Rückwärts-Tabben oben ausgerichtet
    /// wird, kann ganz darunter verschwinden.
    pub hides_focus: Option<bool>,
    /// Ob der Fokus sichtbar wird: `Some(true)`, wenn sich beim Fokussieren
    /// ein Stil ändert, der als Indikator taugt; `None`, wenn nicht gemessen —
    /// Fokussieren ändert den Zustand der Seite und ist deshalb ein eigener,
    /// ausdrücklicher Durchgang.
    pub focus_visible: Option<bool>,
    /// Berechnetes `pointer-events: none`: Ein Klick geht durch das Element
    /// hindurch, es ist kein Ziel.
    pub pointer_events_none: Option<bool>,
    /// Auf dem Element oder einem Vorfahren läuft beim Messen eine *endliche*
    /// Animation — seine Größe ist noch nicht die endgültige. Endlose
    /// Animationen zählen nicht; sie kommen nie zur Ruhe.
    pub animating: Option<bool>,
}

/// Berechnete Transformation eines Elements, als Zeichenketten so, wie
/// der Browser sie serialisiert: `transform` als `none` oder
/// `matrix(…)`/`matrix3d(…)`, `rotate` als `none` oder Winkel (`90deg`).
///
/// Jedes Feld ist einzeln optional, `None` heißt „nicht gemessen". Ein Host
/// baut den Wert mit `BoxTransform { transform: …, ..Default::default() }`,
/// damit künftige Felder keine Quelltextänderung erzwingen.
///
/// Eine eigene Methode ([`Rendering::box_transform`]) statt zweier Felder an
/// [`ComputedStyle`]: neue Felder an dem Struct ohne `non_exhaustive` wären
/// für jeden Host, der es als Struct-Literal baut, ein Bruch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoxTransform {
    pub transform: Option<String>,
    pub rotate: Option<String>,
}

/// Rohe Beobachtungen einer Einfüge-Probe an einem Passwort- oder
/// Code-Feld, kein Urteil. Der Host schickt ein synthetisches, abbrechbares
/// `paste`-Ereignis mit einem Probetext, stellt den Wert des Feldes danach
/// wieder her und unterdrückt `alert`/`confirm`/`prompt`.
///
/// Das Ereignis ist synthetisch (`isTrusted` ist falsch): Ein Handler, der nur
/// auf vertrauenswürdige Ereignisse oder Tastendrücke reagiert, bleibt
/// unsichtbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteProbe {
    /// `dispatchEvent` lieferte `false`: ein Handler hat das Ereignis
    /// abgebrochen.
    pub cancelled: bool,
    /// Der Probetext stand danach im Wert des Feldes.
    pub inserted: bool,
}

/// Messung nach dem Fokussieren eines Bedienelements: wie viel von seinem
/// Kasten ein fixiertes oder klebendes fremdes Element überdeckt.
///
/// Der Host fokussiert das Element, lässt den Browser es so in den
/// sichtbaren Bereich scrollen, wie ein Tastaturnutzer es sähe, tastet ein
/// festes Raster im Kasten ab (etwa 3 mal 3, 1 px eingerückt, alle Punkte im
/// Viewport) und fragt je Punkt die Malreihenfolge ab (`elementsFromPoint`).
/// Er hält fest, **wie** er gescrollt hat (etwa `scrollIntoView({block:
/// 'nearest'})`), und stellt Scrollposition und Fokus danach wieder her.
/// Statische Geometrie ohne Fokuslauf ist keine solche Messung und darf nicht
/// als eine geliefert werden: Der Fall, um den es dem Kriterium geht — ein
/// Element, das unter eine klebende Kopfzeile scrollt —, entsteht erst beim
/// Fokussieren.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusCover {
    /// Anzahl der Abtastpunkte im Kasten des Elements.
    pub points: u32,
    /// Wie viele davon ein Element überdeckt, das weder Vorfahr noch Nachfahr
    /// des Bedienelements ist und `position: fixed` oder `sticky` hat und
    /// darüber gemalt wird. Eine Überlagerung unter dem Element zählt nicht.
    pub covered: u32,
    /// Das oberste solche Element.
    pub blocker: Option<NodeId>,
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

    /// Layout-Angaben für die heuristischen Regeln. Vorgabe `None`: Ein Host,
    /// der sie nicht erhebt, bekommt diese Regeln nicht.
    fn layout<'n>(&'n self, _node: Self::N<'n>) -> Option<Layout> {
        None
    }

    /// Um wie viele CSS-Pixel der Inhalt über den Kasten hinausreicht, wenn
    /// `overflow` auf der Achse `auto` oder `scroll` ist — das Größere von
    /// `scrollHeight − clientHeight` und `scrollWidth − clientWidth`; `0`,
    /// wenn nichts zu scrollen ist oder `overflow` es nicht zulässt.
    ///
    /// Eine eigene Methode, nicht Teil von [`Layout`]: Ein Host, der nur den
    /// Überhang misst, müsste sonst ein ganzes `Layout` liefern, und die
    /// Heuristiken darauf liefen mit Vorgabewerten statt gar nicht.
    /// Vorgabe `None`: nicht gemessen.
    fn scroll_overflow_px<'n>(&'n self, _node: Self::N<'n>) -> Option<f32> {
        None
    }

    /// Ob der Text des Knotens optisch verborgen ist, obwohl `display` und
    /// `visibility` ihn darstellen: abgeschnitten (`clip`, `clip-path`), aus
    /// dem Kasten geschoben (`text-indent` weit ins Negative) oder in einem
    /// Kasten von höchstens 1 px mit `overflow: hidden` — die üblichen
    /// „nur für Screenreader"-Muster. Solcher Text hat keinen sichtbaren
    /// Kontrast. Vorgabe `None`: nicht gemessen.
    fn visually_hidden<'n>(&'n self, _node: Self::N<'n>) -> Option<bool> {
        None
    }

    /// Abgetasteter Hintergrund hinter dem Text des Knotens. Die
    /// Kontrastregel greift darauf nur zurück, wenn
    /// [`ComputedStyle::background_color`] fehlt. Vorgabe `None`: nicht
    /// abgetastet — dann bleibt der Kontrast nicht bestimmbar.
    fn sampled_backdrop<'n>(&'n self, _node: Self::N<'n>) -> Option<Backdrop> {
        None
    }

    /// Ob das Dokument eines `<iframe>`/`<frame>` lesbar ist — die Messung des
    /// Hosts je Rahmen, einschließlich Rahmen in offenen Shadow Roots.
    ///
    /// - `Some(true)`: lesbar, der Rahmen wird geprüft oder lässt sich prüfen.
    /// - `Some(false)`: nicht lesbar — fremder Ursprung, oder die Navigation
    ///   läuft noch; ein Host in der Seite kann beides nicht unterscheiden.
    /// - `None`: der Host sieht sich Rahmen nicht an. Vorgabe.
    ///
    /// Ob der Rahmen dargestellt wird, leitet die Regel aus `display`,
    /// `visibility` und `bounds` ab.
    fn frame_reachable<'n>(&'n self, _node: Self::N<'n>) -> Option<bool> {
        None
    }

    /// Berechnetes `transform` und `rotate` des Knotens, mindestens für
    /// `html` und `body`. Ein Quarterdreh der Wurzel ist der Fehler, den
    /// `orientation/root-rotated` sieht. Vorgabe `None`: nicht gemessen.
    fn box_transform<'n>(&'n self, _node: Self::N<'n>) -> Option<BoxTransform> {
        None
    }

    /// Ergebnis der Einfüge-Probe an einem Passwort- oder Einmalcode-Feld
    /// (`input[type=password]`, `autocomplete` mit `one-time-code` oder
    /// `current-password`). Die Probe führt Seiten-Handler aus und schreibt
    /// `value`; sie ist deshalb ein eigener, ausdrücklicher Durchgang des
    /// Hosts. Vorgabe `None`: nicht untersucht.
    fn paste_probe<'n>(&'n self, _node: Self::N<'n>) -> Option<PasteProbe> {
        None
    }

    /// Abdeckung eines fokussierten Bedienelements durch fixierte oder
    /// klebende Elemente, nach einem Fokuslauf des Hosts (siehe
    /// [`FocusCover`]). Der Lauf ändert Fokus und Scrollposition und ist
    /// deshalb ein eigener, ausdrücklicher Durchgang; ein Deckel auf die Zahl
    /// der Bedienelemente ist Sache des Hosts und soll gemeldet werden.
    /// Vorgabe `None`: nicht untersucht.
    fn focus_cover<'n>(&'n self, _node: Self::N<'n>) -> Option<FocusCover> {
        None
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
