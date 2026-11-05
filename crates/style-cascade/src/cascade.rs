//! Die Kaskade: welche Angabe an welchem Element gewinnt, mit Vererbung.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use a11y_dom::{Node, NodeId, NodeKind};
use stylesheet_parse::{Context, Rule as CssRule, Stylesheet, parse_stylesheet};

use crate::media::{Environment, eval};
use crate::selector::{Key, Match, Mode, Selector, Specificity};
use crate::shorthand::{PENDING, UNKNOWN, expand};
use crate::ua;

/// Der angegebene Wert einer Eigenschaft an einem Element, nach Kaskade,
/// Vererbung und `var()`, aber vor der Berechnung zu Pixeln und Farben.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Specified {
    /// Der Wert, wie er im Stylesheet steht (`var()` aufgelöst).
    Set(String),
    /// Es gilt der Anfangswert der Eigenschaft.
    Unset,
    /// Nicht bestimmbar: eine Regel mit unbekanntem Selektor oder unbekannter
    /// Bedingung hätte gewinnen können, oder der Wert ist nicht lesbar.
    Unknown,
}

#[derive(Debug, Clone)]
struct Decl {
    prop: String,
    value: String,
    important: bool,
    order: u32,
}

/// Die Regeln, die ein Element treffen oder treffen könnten, nach Index.
type Matches = Rc<Vec<(usize, Match)>>;

struct Rule {
    selector: Selector,
    spec: Specificity,
    layer: Option<u32>,
    /// `Yes` oder `Unknown`; `No` kommt gar nicht erst hinein.
    cond: Match,
    decls: Vec<Decl>,
}

/// Rangfolge zweier Angaben (CSS Cascade 5 §6.1, §6.5): Wichtigkeit,
/// Autorenherkunft (Darstellungsattribute liegen darunter), `style`-Attribut
/// (steht über allen Ebenen), Ebene, Spezifität, Reihenfolge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Weight {
    important: bool,
    author: bool,
    inline: bool,
    layer: u32,
    spec: Specificity,
    order: u32,
}

fn weight(
    important: bool,
    author: bool,
    layer: Option<u32>,
    inline: bool,
    spec: Specificity,
    order: u32,
) -> Weight {
    // Normal: Unlayered schlägt jede Ebene, spätere Ebenen schlagen frühere.
    // Wichtig: umgekehrt.
    let layer = match (important, layer) {
        (false, None) => u32::MAX,
        (false, Some(r)) => r,
        (true, None) => 0,
        (true, Some(r)) => u32::MAX - r,
    };
    Weight {
        important,
        author,
        inline,
        layer,
        spec,
        order,
    }
}

enum Declared {
    Value(String),
    Nothing,
    Unknown,
}

const VERERBT: &[&str] = &[
    "color",
    "font-style",
    "font-family",
    "line-height",
    "visibility",
    "list-style-type",
    "text-align",
    "text-transform",
    "letter-spacing",
    "word-spacing",
    "white-space",
    "cursor",
    "direction",
];

fn vererbt(prop: &str) -> bool {
    prop.starts_with("--") || VERERBT.contains(&prop)
}

/// Die Kaskade über die Stylesheets einer Seite. Gehört zu **einem**
/// Dokument: die Zwischenspeicher sind nach Knotenkennung geordnet.
pub struct Cascade {
    env: Environment,
    rules: Vec<Rule>,
    by_id: HashMap<String, Vec<usize>>,
    by_class: HashMap<String, Vec<usize>>,
    by_type: HashMap<String, Vec<usize>>,
    universal: Vec<usize>,
    matched: RefCell<HashMap<NodeId, Matches>>,
    inline: RefCell<HashMap<NodeId, Rc<Vec<Decl>>>>,
    cache: RefCell<HashMap<(NodeId, String), Specified>>,
    pub(crate) sizes: RefCell<HashMap<NodeId, Option<f32>>>,
    pub(crate) weights: RefCell<HashMap<NodeId, Option<u16>>>,
    depth: Cell<u32>,
    imports: Vec<String>,
}

/// Eine Rekursionstiefe, ab der `var()`-Zyklen und absurde Ketten abbrechen.
const MAX_TIEFE: u32 = 64;

impl Cascade {
    /// Baut die Kaskade. Die Reihenfolge der Stylesheets ist die im
    /// Dokument.
    pub fn new(sheets: &[Stylesheet], env: Environment) -> Self {
        let mut c = Cascade {
            env,
            rules: Vec::new(),
            by_id: HashMap::new(),
            by_class: HashMap::new(),
            by_type: HashMap::new(),
            universal: Vec::new(),
            matched: RefCell::default(),
            inline: RefCell::default(),
            cache: RefCell::default(),
            sizes: RefCell::default(),
            weights: RefCell::default(),
            depth: Cell::new(0),
            imports: Vec::new(),
        };
        c.lade(sheets);
        c
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Die Vorspänne der `@import`-Regeln, die nicht aufgelöst sind. Solange
    /// es welche gibt, ist jede Antwort `Unknown`, außer sie kommt aus einem
    /// `!important` im `style`-Attribut. Der Host ersetzt dafür jede
    /// `@import`-Regel durch die Regeln des importierten Stylesheets (in
    /// Dokumentreihenfolge, vor den eigenen Regeln des importierenden Blatts).
    pub fn unresolved_imports(&self) -> &[String] {
        &self.imports
    }

    fn lade(&mut self, sheets: &[Stylesheet]) {
        for s in sheets {
            for r in &s.rules {
                if let CssRule::Other(a) = r
                    && a.name == "import"
                {
                    self.imports.push(a.prelude.trim().to_string());
                }
            }
        }
        // Reihenfolge der Ebenen: `@layer a, b;` legt sie vorab fest, sonst
        // gilt das erste Auftreten.
        let mut ebenen: Vec<String> = Vec::new();
        for s in sheets {
            for r in &s.rules {
                if let CssRule::Other(a) = r
                    && a.name == "layer"
                {
                    for n in a.prelude.split(',') {
                        ebene(&mut ebenen, n.trim());
                    }
                }
            }
        }
        let mut order = 0u32;
        for s in sheets {
            for scoped in s.style_rules() {
                let (cond, layer) = kontext(&scoped.context, &self.env, &mut ebenen);
                if cond == Match::No {
                    continue;
                }
                let decls: Vec<Decl> = scoped
                    .declarations
                    .iter()
                    .flat_map(|d| {
                        order += 1;
                        let o = order;
                        expand(&d.name, &d.value)
                            .into_iter()
                            .map(move |(prop, value)| Decl {
                                prop,
                                value,
                                important: d.important,
                                order: o,
                            })
                    })
                    .collect();
                if decls.is_empty() {
                    continue;
                }
                for sel in &scoped.selectors {
                    let selector = Selector::parse(sel);
                    let idx = self.rules.len();
                    match selector.key() {
                        Some(Key::Id(i)) => self.by_id.entry(i).or_default().push(idx),
                        Some(Key::Class(c)) => self.by_class.entry(c).or_default().push(idx),
                        Some(Key::Type(t)) => self.by_type.entry(t).or_default().push(idx),
                        None => self.universal.push(idx),
                    }
                    // Ein Selektor, den wir nicht lesen, könnte treffen.
                    let cond = if selector.is_valid() {
                        cond
                    } else {
                        Match::Unknown
                    };
                    self.rules.push(Rule {
                        spec: selector.specificity(),
                        selector,
                        layer,
                        cond,
                        decls: decls.clone(),
                    });
                }
            }
        }
    }

    fn matched<'a, N: Node<'a>>(&self, n: N) -> Matches {
        if let Some(m) = self.matched.borrow().get(&n.id()) {
            return Rc::clone(m);
        }
        let mut kandidaten: Vec<usize> = self.universal.clone();
        if let Some(id) = n.attr("id")
            && let Some(v) = self.by_id.get(id)
        {
            kandidaten.extend(v);
        }
        if let Some(klassen) = n.attr("class") {
            for k in klassen.split_ascii_whitespace() {
                if let Some(v) = self.by_class.get(k) {
                    kandidaten.extend(v);
                }
            }
        }
        if let Some(v) = self.by_type.get(&n.local_name().to_ascii_lowercase()) {
            kandidaten.extend(v);
        }
        kandidaten.sort_unstable();
        kandidaten.dedup();
        let treffer: Vec<(usize, Match)> = kandidaten
            .into_iter()
            .filter_map(|i| {
                let r = &self.rules[i];
                let m = r.selector.matches(n, Mode::Resting).und(r.cond);
                (m != Match::No).then_some((i, m))
            })
            .collect();
        let treffer = Rc::new(treffer);
        self.matched
            .borrow_mut()
            .insert(n.id(), Rc::clone(&treffer));
        treffer
    }

    fn inline<'a, N: Node<'a>>(&self, n: N) -> Rc<Vec<Decl>> {
        if let Some(d) = self.inline.borrow().get(&n.id()) {
            return Rc::clone(d);
        }
        let decls: Vec<Decl> = match n.attr("style") {
            Some(text) if !text.trim().is_empty() => {
                let sheet = parse_stylesheet(&format!("*{{{text}}}"));
                sheet
                    .style_rules()
                    .first()
                    .map(|r| r.declarations)
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .flat_map(|(i, d)| {
                        expand(&d.name, &d.value)
                            .into_iter()
                            .map(move |(prop, value)| Decl {
                                prop,
                                value,
                                important: d.important,
                                order: i as u32,
                            })
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let d = Rc::new(decls);
        self.inline.borrow_mut().insert(n.id(), Rc::clone(&d));
        d
    }

    fn declared<'a, N: Node<'a>>(&self, n: N, prop: &str) -> Declared {
        let mut best: Option<(Weight, &str)> = None;
        let mut unsicher: Option<Weight> = None;
        let treffer = self.matched(n);
        for (idx, m) in treffer.iter() {
            let r = &self.rules[*idx];
            for d in r.decls.iter().filter(|d| d.prop == prop) {
                let w = weight(d.important, true, r.layer, false, r.spec, d.order);
                if *m == Match::Yes {
                    if best.is_none_or(|(b, _)| w > b) {
                        best = Some((w, &d.value));
                    }
                } else if unsicher.is_none_or(|u| w > u) {
                    unsicher = Some(w);
                }
            }
        }
        let inline = self.inline(n);
        let inline_best = inline
            .iter()
            .filter(|d| d.prop == prop)
            .map(|d| {
                (
                    weight(
                        d.important,
                        true,
                        None,
                        true,
                        Specificity::default(),
                        d.order,
                    ),
                    d.value.clone(),
                )
            })
            .max_by_key(|(w, _)| *w);
        let (best_w, best_v): (Option<Weight>, Option<String>) = match (best, inline_best) {
            (Some((w, v)), Some((iw, iv))) => {
                if iw > w {
                    (Some(iw), Some(iv))
                } else {
                    (Some(w), Some(v.to_string()))
                }
            }
            (Some((w, v)), None) => (Some(w), Some(v.to_string())),
            (None, Some((iw, iv))) => (Some(iw), Some(iv)),
            (None, None) => (None, None),
        };
        // Darstellungsattribute stehen unter jeder Regel der Seite.
        let (best_w, best_v) = match ua::hint(n, prop) {
            Some(h) => {
                let w = weight(false, false, None, false, Specificity::default(), 0);
                if best_w.is_none_or(|b| w > b) {
                    (Some(w), Some(h))
                } else {
                    (best_w, best_v)
                }
            }
            None => (best_w, best_v),
        };
        // Ein unaufgelöstes `@import` kann jede Angabe überstimmen, nur nicht
        // ein `!important` im `style`-Attribut.
        if !self.imports.is_empty() && !best_w.is_some_and(|b| b.important && b.inline) {
            return Declared::Unknown;
        }
        if let Some(u) = unsicher
            && best_w.is_none_or(|b| u >= b)
        {
            return Declared::Unknown;
        }
        match best_v {
            Some(v) if v == UNKNOWN => Declared::Unknown,
            Some(v) => Declared::Value(v),
            None => match ua::declared(n, prop) {
                Some(v) => Declared::Value(v),
                None => Declared::Nothing,
            },
        }
    }

    /// Der angegebene Wert von `prop` an `n`, mit Vererbung.
    ///
    /// `prop` ist klein geschrieben; eigene Eigenschaften (`--x`) behalten ihre
    /// Schreibung. Kurzschreibweisen gibt es hier nicht: sie sind in ihre
    /// Einzeleigenschaften zerlegt.
    pub fn specified<'a, N: Node<'a>>(&self, n: N, prop: &str) -> Specified {
        let key = (n.id(), prop.to_string());
        if let Some(s) = self.cache.borrow().get(&key) {
            return s.clone();
        }
        if self.depth.get() >= MAX_TIEFE {
            return Specified::Unknown;
        }
        self.depth.set(self.depth.get() + 1);
        let s = self.berechne(n, prop);
        self.depth.set(self.depth.get() - 1);
        self.cache.borrow_mut().insert(key, s.clone());
        s
    }

    fn vom_eltern<'a, N: Node<'a>>(&self, n: N, prop: &str) -> Specified {
        match n.parent().filter(|p| p.kind() == NodeKind::Element) {
            Some(p) => self.specified(p, prop),
            None => Specified::Unset,
        }
    }

    fn berechne<'a, N: Node<'a>>(&self, n: N, prop: &str) -> Specified {
        let erbt = vererbt(prop);
        match self.declared(n, prop) {
            Declared::Unknown => Specified::Unknown,
            Declared::Nothing => {
                if erbt {
                    self.vom_eltern(n, prop)
                } else {
                    Specified::Unset
                }
            }
            Declared::Value(v) if v.starts_with(PENDING) => {
                let rest = &v[PENDING.len_utf8()..];
                let (kurz, roh) = rest.split_once(PENDING).unwrap_or(("", rest));
                match self.loese_var(n, roh, 0) {
                    Err(Fehler::Unbekannt) => Specified::Unknown,
                    Err(Fehler::Ungueltig) => {
                        if erbt {
                            self.vom_eltern(n, prop)
                        } else {
                            Specified::Unset
                        }
                    }
                    Ok(w) => match expand(kurz, &w).into_iter().find(|(p, _)| p == prop) {
                        Some((_, x)) if x != UNKNOWN => Specified::Set(x),
                        _ => Specified::Unknown,
                    },
                }
            }
            Declared::Value(v) => {
                let t = v.trim();
                match t.to_ascii_lowercase().as_str() {
                    // Größe und Gewicht werden relativ zum Elternwert berechnet;
                    // wer sie erbt, braucht dessen berechneten Wert, nicht den
                    // Text. Der Aufrufer löst `inherit` auf.
                    "inherit" | "unset" if prop == "font-size" || prop == "font-weight" => {
                        Specified::Set("inherit".into())
                    }
                    "inherit" => self.vom_eltern(n, prop),
                    "initial" | "revert" | "revert-layer" => Specified::Unset,
                    "unset" => {
                        if erbt {
                            self.vom_eltern(n, prop)
                        } else {
                            Specified::Unset
                        }
                    }
                    _ if t.to_ascii_lowercase().contains("var(") => match self.loese_var(n, t, 0) {
                        Ok(w) => Specified::Set(w),
                        Err(Fehler::Unbekannt) => Specified::Unknown,
                        // „Ungültig zur Berechnungszeit": verhält sich wie `unset`.
                        Err(Fehler::Ungueltig) => {
                            if erbt {
                                self.vom_eltern(n, prop)
                            } else {
                                Specified::Unset
                            }
                        }
                    },
                    _ => Specified::Set(t.to_string()),
                }
            }
        }
    }

    /// Ersetzt jedes `var(--x, ersatz)` im Wert.
    fn loese_var<'a, N: Node<'a>>(&self, n: N, wert: &str, tiefe: u32) -> Result<String, Fehler> {
        if tiefe > 16 {
            return Err(Fehler::Unbekannt);
        }
        let mut out = String::new();
        let mut rest = wert;
        while let Some(i) = rest.to_ascii_lowercase().find("var(") {
            out.push_str(&rest[..i]);
            let nach = &rest[i + 4..];
            let ende = klammer_ende(nach).ok_or(Fehler::Unbekannt)?;
            let inhalt = &nach[..ende];
            let (name, ersatz) = match inhalt.split_once(',') {
                Some((a, b)) => (a.trim(), Some(b.trim())),
                None => (inhalt.trim(), None),
            };
            let w = match self.specified(n, name) {
                Specified::Set(w) => w,
                Specified::Unknown => return Err(Fehler::Unbekannt),
                Specified::Unset => match ersatz {
                    Some(e) if e.to_ascii_lowercase().contains("var(") => {
                        self.loese_var(n, e, tiefe + 1)?
                    }
                    Some(e) => e.to_string(),
                    None => return Err(Fehler::Ungueltig),
                },
            };
            out.push_str(&w);
            rest = &nach[ende + 1..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

enum Fehler {
    Unbekannt,
    Ungueltig,
}

/// Position der schließenden Klammer zu einer Klammer, die schon geöffnet ist.
fn klammer_ende(s: &str) -> Option<usize> {
    let mut tiefe = 1u32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => tiefe += 1,
            ')' => {
                tiefe -= 1;
                if tiefe == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn ebene(ebenen: &mut Vec<String>, name: &str) -> u32 {
    match ebenen.iter().position(|e| e == name) {
        Some(i) => i as u32,
        None => {
            ebenen.push(name.to_string());
            (ebenen.len() - 1) as u32
        }
    }
}

/// Gilt der Kontext einer Regel (Medien, `@supports`, Ebene …)?
fn kontext(ctx: &Context<'_>, env: &Environment, ebenen: &mut Vec<String>) -> (Match, Option<u32>) {
    let mut cond = Match::Yes;
    for m in &ctx.media {
        cond = cond.und(eval(m, env));
    }
    let mut layer = None;
    for (name, prelude) in &ctx.groups {
        match *name {
            "layer" => {
                let p = prelude.trim();
                let name = if p.is_empty() {
                    format!("\u{0}anon{}", ebenen.len())
                } else {
                    p.to_string()
                };
                layer = Some(ebene(ebenen, &name));
            }
            // Eine Bedingung für Funktionen, die moderne Browser kennen; nur
            // das Negierte (Ersatz für alte Browser) greift nicht.
            "supports" => {
                if prelude.trim_start().to_ascii_lowercase().starts_with("not") {
                    cond = cond.und(Match::No);
                }
            }
            "starting-style" => cond = cond.und(Match::No),
            _ => cond = cond.und(Match::Unknown),
        }
    }
    (cond, layer)
}
