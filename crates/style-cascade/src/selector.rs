//! Ob ein CSS-Selektor ein Element trifft — eine Teilmenge von Selectors
//! Level 4, genug für die Frage „gibt es auf dieser Seite etwas, für das die
//! Regel gilt?".
//!
//! Die Stylesheet-Regeln brauchen das, weil ein Stylesheet mehr beschreibt,
//! als eine Seite benutzt: Keyframes für einen Lade-Kreisel, den die Seite nie
//! einbindet, sind keine Bewegung auf dieser Seite (auditmysite#712,
//! bundesregierung.de). auditmysite fragte dafür `querySelectorAll`; hier
//! steht dieselbe Frage ohne Browser.
//!
//! Drei Antworten, nicht zwei: Was dieser Abgleich nicht versteht (`:has()`,
//! Namensräume), ist [`Match::Unknown`]. Eine
//! Regel behandelt das wie einen Match — lieber ein Hinweis zu viel als
//! eine stille Lücke.
//!
//! Zustände (`:hover`, `:focus`, …) und Pseudo-Elemente (`::before`, …)
//! gelten als erfüllt: `.card:hover` trifft, wo `.card` steht. So rechnete
//! auch auditmysite.

use a11y_dom::{Document, Node, NodeKind};

/// Ergebnis eines Abgleichs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    Yes,
    No,
    Unknown,
}

/// Der Zustand, in dem abgeglichen wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// „Kann diese Regel hier je greifen?": Zustände (`:hover`, `:focus` …)
    /// und Pseudo-Elemente gelten als erfüllt.
    Possible,
    /// Das Element im Ruhezustand, wie die Kaskade es braucht: Bedienzustände
    /// und Pseudo-Elemente treffen nicht.
    Resting,
}

impl Match {
    pub(crate) fn und(self, other: Match) -> Match {
        match (self, other) {
            (Match::No, _) | (_, Match::No) => Match::No,
            (Match::Yes, Match::Yes) => Match::Yes,
            _ => Match::Unknown,
        }
    }

    pub(crate) fn oder(self, other: Match) -> Match {
        match (self, other) {
            (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
            (Match::No, Match::No) => Match::No,
            _ => Match::Unknown,
        }
    }

    pub(crate) fn nicht(self) -> Match {
        match self {
            Match::Yes => Match::No,
            Match::No => Match::Yes,
            Match::Unknown => Match::Unknown,
        }
    }
}

/// Pseudoklassen, die einen Zustand der Bedienung meinen. Sie gelten als
/// erfüllt.
const ZUSTAENDE: &[&str] = &[
    "hover",
    "focus",
    "focus-visible",
    "focus-within",
    "active",
    "checked",
    "target",
    "visited",
    "link",
    "any-link",
    "enabled",
    "disabled",
    "open",
    "closed",
    "popover-open",
    "modal",
    "placeholder-shown",
    "valid",
    "invalid",
    "required",
    "optional",
    "user-invalid",
    "user-valid",
    "indeterminate",
    "default",
    "autofill",
    "-webkit-autofill",
    "playing",
    "paused",
    "fullscreen",
];

/// Pseudo-Elemente, die mit einem Doppelpunkt geschrieben werden dürfen.
const ALTE_PSEUDO_ELEMENTE: &[&str] = &["before", "after", "first-line", "first-letter"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kombinator {
    Nachfahre,
    Kind,
    Nachbar,
    Geschwister,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AttrOp {
    Da,
    Gleich,
    Wort,
    Bindestrich,
    Anfang,
    Ende,
    Teil,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Einfach {
    Typ(String),
    Id(String),
    Klasse(String),
    Attr {
        name: String,
        op: AttrOp,
        wert: String,
        gross_klein_egal: bool,
    },
    Nicht(Vec<Komplex>),
    Eines(Vec<Komplex>),
    /// `:where()`: trifft wie `:is()`, zählt aber nichts zur Spezifität.
    Wo(Vec<Komplex>),
    /// Pseudoklasse eines Bedienzustands (`:hover`, `:link` …).
    Zustand(String),
    /// Pseudo-Element (`::before`, `:after` …).
    PseudoElement,
    Wurzel,
    ErstesKind,
    LetztesKind,
    EinzigesKind,
    Leer,
    /// `:nth-child(an+b [of S])`, `:nth-last-child`, `:nth-of-type`,
    /// `:nth-last-of-type`.
    Nth {
        a: i64,
        b: i64,
        /// Von hinten zählen.
        letzter: bool,
        /// Nur Geschwister desselben Typs zählen.
        typ: bool,
        von: Option<Vec<Komplex>>,
    },
    Erfuellt,
    Unbekannt,
}

type Zusammengesetzt = Vec<Einfach>;

/// Ein komplexer Selektor, von rechts nach links: das Subjekt zuerst.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Komplex {
    subjekt: Zusammengesetzt,
    davor: Vec<(Kombinator, Zusammengesetzt)>,
}

/// Eine Selektorliste. `None`, wenn sie sich nicht lesen ließ.
fn liste(text: &str) -> Option<Vec<Komplex>> {
    let mut p = Leser {
        z: text.chars().collect(),
        i: 0,
    };
    let l = p.liste()?;
    p.leer();
    (p.i == p.z.len()).then_some(l)
}

struct Leser {
    z: Vec<char>,
    i: usize,
}

impl Leser {
    fn sieh(&self) -> Option<char> {
        self.z.get(self.i).copied()
    }

    fn leer(&mut self) -> bool {
        let start = self.i;
        while self.sieh().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
        self.i > start
    }

    /// Ein Bezeichner, mit CSS-Escapes (`\\[`, `\\31 0`; CSS Syntax §4.3.7).
    fn name(&mut self) -> Option<String> {
        let mut out = String::new();
        loop {
            match self.sieh() {
                Some('\\') => {
                    self.i += 1;
                    let c = self.sieh()?;
                    if c.is_ascii_hexdigit() {
                        let start = self.i;
                        while self.i - start < 6
                            && self.sieh().is_some_and(|h| h.is_ascii_hexdigit())
                        {
                            self.i += 1;
                        }
                        let hex: String = self.z[start..self.i].iter().collect();
                        let cp = u32::from_str_radix(&hex, 16).ok()?;
                        // Ein Leerzeichen beendet die Escape-Folge.
                        if self.sieh().is_some_and(char::is_whitespace) {
                            self.i += 1;
                        }
                        out.push(
                            char::from_u32(cp)
                                .filter(|&c| c != '\0')
                                .unwrap_or('\u{FFFD}'),
                        );
                    } else {
                        out.push(c);
                        self.i += 1;
                    }
                }
                Some(c) if c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() => {
                    out.push(c);
                    self.i += 1;
                }
                _ => break,
            }
        }
        (!out.is_empty()).then_some(out)
    }

    fn liste(&mut self) -> Option<Vec<Komplex>> {
        let mut out = vec![self.komplex()?];
        loop {
            self.leer();
            if self.sieh() == Some(',') {
                self.i += 1;
                out.push(self.komplex()?);
            } else {
                return Some(out);
            }
        }
    }

    fn komplex(&mut self) -> Option<Komplex> {
        self.leer();
        let mut teile = vec![self.zusammengesetzt()?];
        let mut kombis = Vec::new();
        loop {
            let leerraum = self.leer();
            let k = match self.sieh() {
                Some('>') => Kombinator::Kind,
                Some('+') => Kombinator::Nachbar,
                Some('~') => Kombinator::Geschwister,
                Some(',') | Some(')') | None => break,
                _ if leerraum => {
                    kombis.push(Kombinator::Nachfahre);
                    teile.push(self.zusammengesetzt()?);
                    continue;
                }
                _ => return None,
            };
            self.i += 1;
            self.leer();
            kombis.push(k);
            teile.push(self.zusammengesetzt()?);
        }
        let subjekt = teile.pop()?;
        let davor = kombis
            .into_iter()
            .rev()
            .zip(teile.into_iter().rev())
            .collect();
        Some(Komplex { subjekt, davor })
    }

    fn zusammengesetzt(&mut self) -> Option<Zusammengesetzt> {
        let mut out = Vec::new();
        match self.sieh() {
            Some('*') => {
                self.i += 1;
                out.push(Einfach::Erfuellt);
            }
            Some(c) if c.is_alphabetic() || c == '_' || c == '-' => {
                let n = self.name()?;
                if self.sieh() == Some('|') {
                    return Some(vec![Einfach::Unbekannt]);
                }
                out.push(Einfach::Typ(n.to_ascii_lowercase()));
            }
            _ => {}
        }
        loop {
            match self.sieh() {
                Some('#') => {
                    self.i += 1;
                    out.push(Einfach::Id(self.name()?));
                }
                Some('.') => {
                    self.i += 1;
                    out.push(Einfach::Klasse(self.name()?));
                }
                Some('[') => {
                    self.i += 1;
                    out.push(self.attribut()?);
                }
                Some(':') => {
                    self.i += 1;
                    out.push(self.pseudo()?);
                }
                Some('\\') | Some('|') => return Some(vec![Einfach::Unbekannt]),
                _ => break,
            }
        }
        (!out.is_empty()).then_some(out)
    }

    fn attribut(&mut self) -> Option<Einfach> {
        self.leer();
        let name = self.name()?.to_ascii_lowercase();
        self.leer();
        let op = match self.sieh()? {
            ']' => {
                self.i += 1;
                return Some(Einfach::Attr {
                    name,
                    op: AttrOp::Da,
                    wert: String::new(),
                    gross_klein_egal: false,
                });
            }
            '=' => AttrOp::Gleich,
            '~' => AttrOp::Wort,
            '|' => AttrOp::Bindestrich,
            '^' => AttrOp::Anfang,
            '$' => AttrOp::Ende,
            '*' => AttrOp::Teil,
            _ => return None,
        };
        self.i += 1;
        if op != AttrOp::Gleich {
            if self.sieh() != Some('=') {
                return None;
            }
            self.i += 1;
        }
        self.leer();
        let wert = match self.sieh()? {
            q @ ('"' | '\'') => {
                self.i += 1;
                let start = self.i;
                while self.sieh().is_some_and(|c| c != q) {
                    self.i += 1;
                }
                let w: String = self.z[start..self.i].iter().collect();
                self.i += 1;
                w
            }
            _ => self.name()?,
        };
        self.leer();
        let mut gross_klein_egal = false;
        if let Some(f) = self.sieh().filter(|c| c.is_alphabetic()) {
            gross_klein_egal = f.eq_ignore_ascii_case(&'i');
            self.i += 1;
            self.leer();
        }
        (self.sieh()? == ']').then(|| self.i += 1)?;
        Some(Einfach::Attr {
            name,
            op,
            wert,
            gross_klein_egal,
        })
    }

    fn pseudo(&mut self) -> Option<Einfach> {
        let element = self.sieh() == Some(':');
        if element {
            self.i += 1;
        }
        let name = self.name()?.to_ascii_lowercase();
        let argument = if self.sieh() == Some('(') {
            self.i += 1;
            let start = self.i;
            let mut tiefe = 1;
            while let Some(c) = self.sieh() {
                match c {
                    '(' => tiefe += 1,
                    ')' => {
                        tiefe -= 1;
                        if tiefe == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                self.i += 1;
            }
            let arg: String = self.z[start..self.i].iter().collect();
            self.i += 1;
            Some(arg)
        } else {
            None
        };
        if element || ALTE_PSEUDO_ELEMENTE.contains(&name.as_str()) {
            return Some(Einfach::PseudoElement);
        }
        Some(match (name.as_str(), argument) {
            ("not", Some(a)) => liste(&a).map_or(Einfach::Unbekannt, Einfach::Nicht),
            ("where", Some(a)) => liste(&a).map_or(Einfach::Unbekannt, Einfach::Wo),
            ("is" | "matches" | "-webkit-any" | "-moz-any", Some(a)) => {
                liste(&a).map_or(Einfach::Unbekannt, Einfach::Eines)
            }
            ("root", None) => Einfach::Wurzel,
            ("first-child", None) => Einfach::ErstesKind,
            ("last-child", None) => Einfach::LetztesKind,
            ("only-child", None) => Einfach::EinzigesKind,
            ("empty", None) => Einfach::Leer,
            ("first-of-type", None) => nth_typ(0, 1, false),
            ("last-of-type", None) => nth_typ(0, 1, true),
            ("only-of-type", None) => Einfach::Eines(vec![Komplex {
                subjekt: vec![nth_typ(0, 1, false), nth_typ(0, 1, true)],
                davor: Vec::new(),
            }]),
            ("nth-child", Some(a)) => nth(&a, false, false, true),
            ("nth-last-child", Some(a)) => nth(&a, true, false, true),
            ("nth-of-type", Some(a)) => nth(&a, false, true, false),
            ("nth-last-of-type", Some(a)) => nth(&a, true, true, false),
            (n, None) if ZUSTAENDE.contains(&n) => Einfach::Zustand(n.to_string()),
            ("lang" | "dir", Some(_)) => Einfach::Unbekannt,
            _ => Einfach::Unbekannt,
        })
    }
}

fn nth_typ(a: i64, b: i64, letzter: bool) -> Einfach {
    Einfach::Nth {
        a,
        b,
        letzter,
        typ: true,
        von: None,
    }
}

/// `an+b` (CSS Syntax Level 3 §6): `odd`, `even`, `3`, `n`, `-n+3`, `2n + 1`.
fn an_plus_b(text: &str) -> Option<(i64, i64)> {
    let t: String = text
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    match t.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let zahl = |z: &str| -> Option<i64> {
        let ziffern = z.strip_prefix(['+', '-']).unwrap_or(z);
        (!ziffern.is_empty() && ziffern.chars().all(|c| c.is_ascii_digit()))
            .then(|| z.parse().ok())?
    };
    let Some(n) = t.find('n') else {
        return zahl(&t).map(|b| (0, b));
    };
    let (vor, nach) = (&t[..n], &t[n + 1..]);
    let a = match vor {
        "" | "+" => 1,
        "-" => -1,
        _ => zahl(vor)?,
    };
    let b = if nach.is_empty() {
        0
    } else {
        // Nach `n` muss ein Vorzeichen stehen.
        nach.starts_with(['+', '-']).then_some(())?;
        zahl(nach)?
    };
    Some((a, b))
}

/// Das Argument von `:nth-*()`; mit `of S` nur bei `von_erlaubt`.
fn nth(arg: &str, letzter: bool, typ: bool, von_erlaubt: bool) -> Einfach {
    let (formel, von) = match arg.to_ascii_lowercase().find(" of ") {
        Some(i) if von_erlaubt => (&arg[..i], Some(liste(arg[i + 4..].trim()))),
        _ => (arg, None),
    };
    let Some((a, b)) = an_plus_b(formel) else {
        return Einfach::Unbekannt;
    };
    match von {
        Some(Some(l)) => Einfach::Nth {
            a,
            b,
            letzter,
            typ,
            von: Some(l),
        },
        Some(None) => Einfach::Unbekannt,
        None => Einfach::Nth {
            a,
            b,
            letzter,
            typ,
            von: None,
        },
    }
}

// --- Abgleich -----------------------------------------------------------------

fn elementkinder<'a, N: Node<'a>>(n: N) -> impl Iterator<Item = N> + 'a {
    n.children().filter(|k| k.kind() == NodeKind::Element)
}

fn geschwister_davor<'a, N: Node<'a>>(n: N) -> Vec<N> {
    let Some(p) = n.parent() else {
        return Vec::new();
    };
    let mut davor = Vec::new();
    for k in elementkinder(p) {
        if k.id() == n.id() {
            break;
        }
        davor.push(k);
    }
    davor
}

fn einfach<'a, N: Node<'a>>(n: N, e: &Einfach, m: Mode) -> Match {
    let ja = |b: bool| if b { Match::Yes } else { Match::No };
    match e {
        Einfach::Typ(t) => ja(n.local_name().eq_ignore_ascii_case(t)),
        Einfach::Id(id) => ja(n.attr("id") == Some(id.as_str())),
        Einfach::Klasse(k) => ja(n
            .attr("class")
            .is_some_and(|c| c.split_ascii_whitespace().any(|x| x == k))),
        Einfach::Attr {
            name,
            op,
            wert,
            gross_klein_egal,
        } => {
            let Some(ist) = n.attr(name) else {
                return Match::No;
            };
            let (ist, wert) = if *gross_klein_egal {
                (ist.to_lowercase(), wert.to_lowercase())
            } else {
                (ist.to_string(), wert.clone())
            };
            ja(match op {
                AttrOp::Da => true,
                AttrOp::Gleich => ist == wert,
                AttrOp::Wort => ist.split_ascii_whitespace().any(|w| w == wert),
                AttrOp::Bindestrich => ist == wert || ist.starts_with(&format!("{wert}-")),
                AttrOp::Anfang => !wert.is_empty() && ist.starts_with(&wert),
                AttrOp::Ende => !wert.is_empty() && ist.ends_with(&wert),
                AttrOp::Teil => !wert.is_empty() && ist.contains(&wert),
            })
        }
        Einfach::Nicht(l) => komplex_liste(n, l, m).nicht(),
        Einfach::Eines(l) | Einfach::Wo(l) => komplex_liste(n, l, m),
        Einfach::Wurzel => ja(n.parent().is_none_or(|p| p.kind() != NodeKind::Element)),
        Einfach::ErstesKind => ja(geschwister_davor(n).is_empty()),
        Einfach::LetztesKind => ja(n
            .parent()
            .and_then(|p| elementkinder(p).last())
            .is_some_and(|l| l.id() == n.id())),
        Einfach::EinzigesKind => ja(n.parent().is_some_and(|p| elementkinder(p).count() == 1)),
        Einfach::Leer => ja(n.children().all(|k| {
            k.kind() != NodeKind::Element && (k.kind() != NodeKind::Text || k.text().is_empty())
        })),
        Einfach::Nth {
            a,
            b,
            letzter,
            typ,
            von,
        } => nth_abgleich(n, *a, *b, *letzter, *typ, von.as_deref(), m),
        Einfach::Erfuellt => Match::Yes,
        Einfach::PseudoElement => ja(m == Mode::Possible),
        Einfach::Zustand(z) => zustand(n, z, m),
        Einfach::Unbekannt => Match::Unknown,
    }
}

/// Position unter den Elementgeschwistern (1-basiert) gegen `an+b`.
fn nth_abgleich<'a, N: Node<'a>>(
    n: N,
    a: i64,
    b: i64,
    letzter: bool,
    typ: bool,
    von: Option<&[Komplex]>,
    m: Mode,
) -> Match {
    let mut geschwister: Vec<N> = match n.parent() {
        Some(p) => elementkinder(p).collect(),
        None => vec![n],
    };
    if letzter {
        geschwister.reverse();
    }
    // Zählt ein Geschwister mit? Bei `of S` kann das unbekannt sein.
    let zaehlt = |k: N| -> Match {
        if typ {
            if k.local_name().eq_ignore_ascii_case(n.local_name()) {
                Match::Yes
            } else {
                Match::No
            }
        } else if let Some(l) = von {
            komplex_liste(k, l, m)
        } else {
            Match::Yes
        }
    };
    if zaehlt(n) == Match::No {
        return Match::No;
    }
    let mut sicher = 0i64;
    let mut unsicher = 0i64;
    for k in geschwister {
        if k.id() == n.id() {
            break;
        }
        match zaehlt(k) {
            Match::Yes => sicher += 1,
            Match::Unknown => unsicher += 1,
            Match::No => {}
        }
    }
    // Die Position liegt in sicher+1 ..= sicher+unsicher+1.
    let trifft = |pos: i64| {
        if a == 0 {
            pos == b
        } else {
            (pos - b) % a == 0 && (pos - b) / a >= 0
        }
    };
    let mut moegliche = ((sicher + 1)..=(sicher + unsicher + 1)).filter(|&p| trifft(p));
    let ergebnis = match (moegliche.next(), unsicher) {
        (None, _) => Match::No,
        (Some(_), 0) => Match::Yes,
        (Some(_), _) => Match::Unknown,
    };
    ergebnis.und(zaehlt(n))
}

/// Bedienzustände, die im Ruhezustand nicht gelten.
const INTERAKTIV: &[&str] = &[
    "hover",
    "focus",
    "focus-visible",
    "focus-within",
    "active",
    "target",
    "visited",
    "fullscreen",
    "modal",
    "popover-open",
    "user-invalid",
    "user-valid",
    "autofill",
    "-webkit-autofill",
    "playing",
    "paused",
];

/// Ist der Wert von `type` (ASCII-unabhängig von der Schreibweise) einer von `arten`?
fn input_art<'a, N: Node<'a>>(n: N, arten: &[&str]) -> bool {
    n.attr("type")
        .is_some_and(|t| arten.iter().any(|a| t.trim().eq_ignore_ascii_case(a)))
}

/// Ist das Element der erste `legend`-Elementkind von `fieldset`?
fn erste_legende<'a, N: Node<'a>>(fieldset: N, k: N) -> bool {
    elementkinder(fieldset)
        .find(|c| c.is_element("legend"))
        .is_some_and(|l| l.id() == k.id())
}

fn ist_bedienelement<'a, N: Node<'a>>(n: N) -> bool {
    matches!(
        n.local_name(),
        "button" | "input" | "select" | "textarea" | "fieldset" | "optgroup" | "option"
    )
}

/// `:disabled` nach HTML §4.16.3: Bedienelement mit `disabled`, oder unter
/// einem deaktivierten `fieldset` (außer in dessen erster `legend`).
fn deaktiviert<'a, N: Node<'a>>(n: N) -> Match {
    let name = n.local_name();
    if !ist_bedienelement(n) {
        // Ein formularassoziiertes Custom Element kennt nur der Host.
        return if name.contains('-') {
            Match::Unknown
        } else {
            Match::No
        };
    }
    if n.has_attr("disabled") && name != "option" {
        return Match::Yes;
    }
    if name == "option" {
        let eigen = n.has_attr("disabled");
        let gruppe = n
            .parent()
            .is_some_and(|p| p.is_element("optgroup") && p.has_attr("disabled"));
        return if eigen || gruppe {
            Match::Yes
        } else {
            Match::No
        };
    }
    if name == "optgroup" {
        return Match::No;
    }
    let mut kind = n;
    while let Some(p) = kind.parent() {
        if p.is_element("fieldset") && p.has_attr("disabled") && !erste_legende(p, kind) {
            return Match::Yes;
        }
        kind = p;
    }
    Match::No
}

/// `:checked`: angekreuzte Checkbox oder Radio, ausgewählte `option`.
/// Der Ruhezustand ist der ursprüngliche Zustand aus den Attributen.
fn angekreuzt<'a, N: Node<'a>>(n: N) -> Match {
    if n.is_element("input") {
        return if input_art(n, &["checkbox", "radio"]) && n.has_attr("checked") {
            Match::Yes
        } else {
            Match::No
        };
    }
    if !n.is_element("option") {
        return Match::No;
    }
    let select = n
        .parent()
        .filter(|p| p.is_element("optgroup"))
        .and_then(|g| g.parent())
        .or_else(|| n.parent())
        .filter(|s| s.is_element("select"));
    let Some(select) = select else {
        return Match::Unknown;
    };
    if select.has_attr("multiple") {
        return if n.has_attr("selected") {
            Match::Yes
        } else {
            Match::No
        };
    }
    // Einfachauswahl: die letzte `selected`-Option gewinnt; ohne eine
    // wählt der Browser selbst.
    let mut optionen = Vec::new();
    sammle_optionen(select, &mut optionen);
    let letzte = optionen.iter().rev().find(|o| o.has_attr("selected"));
    match letzte {
        Some(l) if l.id() == n.id() => Match::Yes,
        Some(_) => Match::No,
        None => Match::Unknown,
    }
}

fn sammle_optionen<'a, N: Node<'a>>(n: N, out: &mut Vec<N>) {
    for k in elementkinder(n) {
        if k.is_element("option") {
            out.push(k);
        } else {
            sammle_optionen(k, out);
        }
    }
}

/// `:required`; das zweite Feld sagt, ob das Element überhaupt `required`
/// tragen kann (dann ist es sonst `:optional`).
fn erforderlich<'a, N: Node<'a>>(n: N) -> (Match, bool) {
    let kann = match n.local_name() {
        "select" | "textarea" => true,
        "input" => !input_art(
            n,
            &[
                "hidden", "range", "color", "submit", "reset", "button", "image",
            ],
        ),
        _ => false,
    };
    if kann && n.has_attr("required") {
        (Match::Yes, true)
    } else {
        (Match::No, kann)
    }
}

fn zustand<'a, N: Node<'a>>(n: N, z: &str, m: Mode) -> Match {
    if m == Mode::Possible {
        return Match::Yes;
    }
    if INTERAKTIV.contains(&z) {
        return Match::No;
    }
    match z {
        "link" | "any-link" => {
            if (n.is_element("a") || n.is_element("area")) && n.has_attr("href") {
                Match::Yes
            } else {
                Match::No
            }
        }
        "disabled" => deaktiviert(n),
        "enabled" if !ist_bedienelement(n) => deaktiviert(n),
        "enabled" => deaktiviert(n).nicht(),
        "checked" => angekreuzt(n),
        "required" => erforderlich(n).0,
        "optional" => {
            let (erforderlich, kann) = erforderlich(n);
            if kann && erforderlich == Match::No {
                Match::Yes
            } else {
                Match::No
            }
        }
        // Was vom Inhalt des Formulars oder vom Benutzer abhängt, weiß der
        // Abgleich nicht.
        _ => Match::Unknown,
    }
}

fn zusammen<'a, N: Node<'a>>(n: N, z: &Zusammengesetzt, m: Mode) -> Match {
    z.iter().fold(Match::Yes, |t, e| t.und(einfach(n, e, m)))
}

/// Gleicht `rest` (von rechts nach links) ab, ausgehend von `n`, das schon
/// zum zusammengesetzten Selektor rechts davon passt.
fn kette<'a, N: Node<'a>>(n: N, rest: &[(Kombinator, Zusammengesetzt)], m: Mode) -> Match {
    let Some(((k, z), weiter)) = rest.split_first() else {
        return Match::Yes;
    };
    let kandidaten: Vec<N> = match k {
        Kombinator::Kind => n
            .parent()
            .filter(|p| p.kind() == NodeKind::Element)
            .into_iter()
            .collect(),
        Kombinator::Nachfahre => a11y_dom::ancestors(n)
            .filter(|p| p.kind() == NodeKind::Element)
            .collect(),
        Kombinator::Nachbar => geschwister_davor(n).pop().into_iter().collect(),
        Kombinator::Geschwister => geschwister_davor(n),
    };
    kandidaten
        .into_iter()
        .map(|c| zusammen(c, z, m).und(kette(c, weiter, m)))
        .fold(Match::No, Match::oder)
}

fn komplex<'a, N: Node<'a>>(n: N, k: &Komplex, m: Mode) -> Match {
    let t = zusammen(n, &k.subjekt, m);
    if t == Match::No {
        return t;
    }
    t.und(kette(n, &k.davor, m))
}

fn komplex_liste<'a, N: Node<'a>>(n: N, l: &[Komplex], m: Mode) -> Match {
    l.iter()
        .map(|k| komplex(n, k, m))
        .fold(Match::No, Match::oder)
}

/// Spezifität nach Selectors Level 4 §17: (Ids, Klassen/Attribute/Pseudoklassen,
/// Typen/Pseudo-Elemente). Die Ordnung ist die der Kaskade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    fn plus(self, o: Specificity) -> Specificity {
        Specificity(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
}

fn spez_liste(l: &[Komplex]) -> Specificity {
    l.iter().map(spez_komplex).max().unwrap_or_default()
}

fn spez_komplex(k: &Komplex) -> Specificity {
    std::iter::once(&k.subjekt)
        .chain(k.davor.iter().map(|(_, z)| z))
        .flatten()
        .map(spez_einfach)
        .fold(Specificity::default(), Specificity::plus)
}

fn spez_einfach(e: &Einfach) -> Specificity {
    match e {
        Einfach::Id(_) => Specificity(1, 0, 0),
        Einfach::Klasse(_)
        | Einfach::Attr { .. }
        | Einfach::Zustand(_)
        | Einfach::Wurzel
        | Einfach::ErstesKind
        | Einfach::LetztesKind
        | Einfach::EinzigesKind
        | Einfach::Leer
        | Einfach::Unbekannt => Specificity(0, 1, 0),
        Einfach::Nth { von, .. } => {
            Specificity(0, 1, 0).plus(von.as_deref().map(spez_liste).unwrap_or_default())
        }
        Einfach::Typ(_) | Einfach::PseudoElement => Specificity(0, 0, 1),
        Einfach::Nicht(l) | Einfach::Eines(l) => spez_liste(l),
        Einfach::Wo(_) | Einfach::Erfuellt => Specificity::default(),
    }
}

/// Wonach sich ein Selektor indizieren lässt: das Auffälligste am Subjekt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Id(String),
    Class(String),
    Type(String),
}

/// Eine einmal gelesene Selektorliste, für viele Abgleiche.
#[derive(Debug, Clone)]
pub struct Selector(Option<Vec<Komplex>>);

impl Selector {
    /// Liest die Liste. Was sich nicht lesen lässt, gleicht als
    /// [`Match::Unknown`] ab.
    pub fn parse(text: &str) -> Self {
        Selector(liste(text.trim()))
    }

    /// Ließ sich der Text lesen?
    pub fn is_valid(&self) -> bool {
        self.0.is_some()
    }

    /// Trifft die Liste dieses Element?
    pub fn matches<'a, N: Node<'a>>(&self, n: N, mode: Mode) -> Match {
        match &self.0 {
            Some(l) => komplex_liste(n, l, mode),
            None => Match::Unknown,
        }
    }

    /// Die Spezifität; bei einer Liste die größte.
    pub fn specificity(&self) -> Specificity {
        self.0.as_deref().map(spez_liste).unwrap_or_default()
    }

    /// Der Index-Schlüssel eines einzelnen Selektors; `None` bei einer Liste
    /// oder wenn das Subjekt nichts Unterscheidendes hat.
    pub fn key(&self) -> Option<Key> {
        let [k] = self.0.as_deref()? else {
            return None;
        };
        k.subjekt
            .iter()
            .find_map(|e| match e {
                Einfach::Id(i) => Some(Key::Id(i.clone())),
                _ => None,
            })
            .or_else(|| {
                k.subjekt.iter().find_map(|e| match e {
                    Einfach::Klasse(c) => Some(Key::Class(c.clone())),
                    _ => None,
                })
            })
            .or_else(|| {
                k.subjekt.iter().find_map(|e| match e {
                    Einfach::Typ(t) => Some(Key::Type(t.clone())),
                    _ => None,
                })
            })
    }
}

/// Trifft der Selektor irgendein Element im Dokument?
///
/// Ein Selektor, der sich nicht lesen lässt, ist [`Match::Unknown`].
pub fn any_match<D: Document>(doc: &D, selector: &str, mode: Mode) -> Match {
    let sel = Selector::parse(selector);
    let Some(l) = sel.0.as_deref() else {
        return Match::Unknown;
    };
    let mut ergebnis = Match::No;
    for n in a11y_dom::elements(doc) {
        ergebnis = ergebnis.oder(komplex_liste(n, l, mode));
        if ergebnis == Match::Yes {
            break;
        }
    }
    ergebnis
}

#[cfg(test)]
mod tests {
    use super::*;
    use a11y_dom::Arena;

    fn seite() -> Arena {
        Arena::builder()
            .open("html")
            .open("body")
            .open("main")
            .attr("id", "inhalt")
            .open("article")
            .attr("class", "card teaser")
            .open("h2")
            .text("Titel")
            .close()
            .open("a")
            .attr("href", "/x")
            .attr("data-track", "teaser-link")
            .text("Mehr")
            .close()
            .close()
            .open("ul")
            .open("li")
            .text("eins")
            .close()
            .open("li")
            .attr("class", "aktiv")
            .text("zwei")
            .close()
            .close()
            .close()
            .close()
            .close()
            .build()
    }

    fn t(sel: &str) -> Match {
        any_match(&seite(), sel, Mode::Possible)
    }

    #[test]
    fn einfache_selektoren() {
        assert_eq!(t("article"), Match::Yes);
        assert_eq!(t("ARTICLE.card"), Match::Yes);
        assert_eq!(t("#inhalt"), Match::Yes);
        assert_eq!(t(".spinner"), Match::No);
        assert_eq!(t("*"), Match::Yes);
        assert_eq!(t("video"), Match::No);
    }

    #[test]
    fn escapes_im_bezeichner() {
        let d = Arena::builder()
            .open("div")
            .attr("class", "bg-error/20 a:b 10x")
            .close()
            .build();
        let t = |s: &str| any_match(&d, s, Mode::Resting);
        assert_eq!(t(r".bg-error\/20"), Match::Yes);
        assert_eq!(t(r".a\:b"), Match::Yes);
        assert_eq!(t(r".\31 0x"), Match::Yes);
        assert_eq!(t(r".bg-error\/30"), Match::No);
    }

    #[test]
    fn ruhezustand_und_spezifitaet() {
        let d = seite();
        let r = |s: &str| any_match(&d, s, Mode::Resting);
        assert_eq!(r("a:hover"), Match::No);
        assert_eq!(r("a:link"), Match::Yes);
        assert_eq!(r("a::before"), Match::No);
        assert_eq!(r("li:checked"), Match::No);
        assert_eq!(r("li:nth-child(2)"), Match::Yes);
        assert_eq!(any_match(&d, "a:hover", Mode::Possible), Match::Yes);
        let s = |t: &str| Selector::parse(t).specificity();
        assert_eq!(s("#a .b c"), Specificity(1, 1, 1));
        assert_eq!(s("a:hover::before"), Specificity(0, 1, 2));
        assert_eq!(s(":where(#a) b"), Specificity(0, 0, 1));
        assert_eq!(s(":is(#a, .b)"), Specificity(1, 0, 0));
        assert_eq!(s(":not(.x)"), Specificity(0, 1, 0));
        assert!(Specificity(0, 2, 0) > Specificity(0, 1, 9));
    }

    #[test]
    fn kombinatoren() {
        assert_eq!(t("main article > a"), Match::Yes);
        assert_eq!(t("main > a"), Match::No);
        assert_eq!(t("h2 + a"), Match::Yes);
        assert_eq!(t("a + h2"), Match::No);
        assert_eq!(t("li ~ li.aktiv"), Match::Yes);
        assert_eq!(t("body .card a"), Match::Yes);
    }

    #[test]
    fn attribute() {
        assert_eq!(t("a[href]"), Match::Yes);
        assert_eq!(t("[data-track^=teaser]"), Match::Yes);
        assert_eq!(t("[data-track$=\"link\"]"), Match::Yes);
        assert_eq!(t("[data-track*=x]"), Match::No);
        assert_eq!(t("[class~=teaser]"), Match::Yes);
        assert_eq!(t("[data-track='TEASER-LINK' i]"), Match::Yes);
        assert_eq!(t("[data-track='TEASER-LINK']"), Match::No);
    }

    #[test]
    fn zustaende_und_pseudo_elemente_gelten_als_erfuellt() {
        assert_eq!(t(".card:hover"), Match::Yes);
        assert_eq!(t("a:focus-visible::after"), Match::Yes);
        assert_eq!(t(".spinner:hover"), Match::No);
        assert_eq!(t("li:before"), Match::Yes);
    }

    #[test]
    fn logische_pseudoklassen() {
        assert_eq!(t("li:not(.aktiv)"), Match::Yes);
        assert_eq!(t("a:not([href])"), Match::No);
        assert_eq!(t(":is(video, .card)"), Match::Yes);
        assert_eq!(t(":where(.x, .y)"), Match::No);
        assert_eq!(t("li:first-child"), Match::Yes);
        assert_eq!(t("h2:last-child"), Match::No);
        assert_eq!(t("html:root"), Match::Yes);
    }

    #[test]
    fn unbekanntes_ist_unbekannt() {
        assert_eq!(t("article:has(a)"), Match::Unknown);
        assert_eq!(t("li:nth-child(zwei)"), Match::Unknown);
        assert_eq!(t("svg|rect"), Match::Unknown);
        assert_eq!(t("a[href"), Match::Unknown);
        // Ein sicheres Nein bleibt Nein, auch neben Unbekanntem.
        assert_eq!(t("video:nth-child(2)"), Match::No);
    }

    #[test]
    fn nth_formeln() {
        assert_eq!(an_plus_b("odd"), Some((2, 1)));
        assert_eq!(an_plus_b("EVEN"), Some((2, 0)));
        assert_eq!(an_plus_b("3"), Some((0, 3)));
        assert_eq!(an_plus_b("n"), Some((1, 0)));
        assert_eq!(an_plus_b("-n+3"), Some((-1, 3)));
        assert_eq!(an_plus_b("2n + 1"), Some((2, 1)));
        assert_eq!(an_plus_b("+3n-2"), Some((3, -2)));
        assert_eq!(an_plus_b("2n1"), None);
        assert_eq!(an_plus_b(""), None);
        assert_eq!(an_plus_b("n+"), None);
    }

    #[test]
    fn nth_ueber_geschwister() {
        let d = Arena::builder()
            .open("ul")
            .open("li")
            .close()
            .open("p")
            .close()
            .open("li")
            .attr("class", "x")
            .close()
            .open("li")
            .close()
            .close()
            .build();
        let r = |s: &str| any_match(&d, s, Mode::Resting);
        assert_eq!(r("li:nth-child(1)"), Match::Yes);
        assert_eq!(r("li:nth-child(2)"), Match::No);
        assert_eq!(r("li:nth-child(odd)"), Match::Yes);
        assert_eq!(r("li:nth-child(5)"), Match::No);
        assert_eq!(r("p:nth-child(2n)"), Match::Yes);
        assert_eq!(r("li:nth-last-child(1)"), Match::Yes);
        assert_eq!(r("p:nth-last-child(3)"), Match::Yes);
        assert_eq!(r("li:nth-of-type(3)"), Match::Yes);
        assert_eq!(r("li:nth-of-type(4)"), Match::No);
        assert_eq!(r("p:nth-of-type(1)"), Match::Yes);
        assert_eq!(r("li.x:nth-of-type(2)"), Match::Yes);
        assert_eq!(r("li.x:nth-of-type(3)"), Match::No);
        assert_eq!(r("li:nth-last-of-type(2)"), Match::Yes);
        assert_eq!(r("p:only-of-type"), Match::Yes);
        assert_eq!(r("li:only-of-type"), Match::No);
        assert_eq!(r("li:first-of-type"), Match::Yes);
        assert_eq!(r("li:nth-child(-n+2)"), Match::Yes);
        // `of S` zählt nur Geschwister, die S treffen.
        assert_eq!(r("li:nth-child(2 of .x)"), Match::No);
        assert_eq!(r(".x:nth-child(1 of .x)"), Match::Yes);
        assert_eq!(r(".x:nth-child(2 of .x)"), Match::No);
        assert_eq!(r(":nth-child(2 of li)"), Match::Yes);
        assert_eq!(
            Selector::parse("li:nth-child(2 of .x)").specificity(),
            Specificity(0, 2, 1)
        );
    }

    fn formular() -> Arena {
        let mut b = Arena::builder().open("form");
        let el = |b: Arena2, name: &str, attrs: &[(&str, &str)]| -> Arena2 {
            let mut b = b.open(name);
            for (k, v) in attrs {
                b = b.attr(k, v);
            }
            b.close()
        };
        b = el(b, "button", &[("id", "b1")]);
        b = el(b, "button", &[("disabled", "")]);
        b = el(b, "input", &[("type", "CHECKBOX"), ("checked", "")]);
        b = el(b, "input", &[("type", "radio")]);
        b = el(b, "input", &[("type", "hidden"), ("required", "")]);
        b = el(b, "input", &[("required", "")]);
        b = el(b, "input", &[("type", "range"), ("required", "")]);
        b = el(b, "textarea", &[]);
        b = b.open("fieldset").attr("disabled", "");
        b = b.open("legend");
        b = el(b, "input", &[("id", "inleg")]);
        b = b.close();
        b = b.open("legend");
        b = el(b, "input", &[("id", "inleg2")]);
        b = b.close();
        b = b.open("div");
        b = el(b, "input", &[("id", "infs")]);
        b = el(b, "my-input", &[]);
        b = b.close();
        b = b.close();
        b = b.open("select").attr("id", "s1");
        b = el(b, "option", &[]);
        b = el(b, "option", &[("selected", "")]);
        b = b.open("optgroup").attr("disabled", "");
        b = el(b, "option", &[("id", "ino")]);
        b = b.close();
        b = b.close();
        b = b.open("select").attr("multiple", "");
        b = el(b, "option", &[("id", "m1")]);
        b = el(b, "option", &[("id", "m2"), ("selected", "")]);
        b = b.close();
        b.close().build()
    }

    type Arena2 = a11y_dom::ArenaBuilder;

    #[test]
    fn formularzustaende_im_ruhezustand() {
        let d = formular();
        let r = |s: &str| any_match(&d, s, Mode::Resting);
        assert_eq!(r("button:disabled"), Match::Yes);
        assert_eq!(r("#b1:disabled"), Match::No);
        assert_eq!(r("#b1:enabled"), Match::Yes);
        assert_eq!(r("button:not(:disabled)"), Match::Yes);
        assert_eq!(r("div:disabled"), Match::No);
        assert_eq!(r("div:enabled"), Match::No);
        assert_eq!(r("fieldset:disabled"), Match::Yes);
        assert_eq!(r("#inleg:disabled"), Match::No);
        assert_eq!(r("#inleg:enabled"), Match::Yes);
        assert_eq!(r("#inleg2:disabled"), Match::Yes);
        assert_eq!(r("#infs:disabled"), Match::Yes);
        assert_eq!(r("my-input:disabled"), Match::Unknown);
        assert_eq!(r("option:disabled"), Match::Yes);
        assert_eq!(r("#ino:enabled"), Match::No);
        assert_eq!(r("input:checked"), Match::Yes);
        assert_eq!(r("input[type=radio]:checked"), Match::No);
        assert_eq!(r("button:checked"), Match::No);
        assert_eq!(r("#s1 option:checked"), Match::Yes);
        assert_eq!(r("#m1:checked"), Match::No);
        assert_eq!(r("#m2:checked"), Match::Yes);
        assert_eq!(r("input[type=hidden]:required"), Match::No);
        assert_eq!(r("input[type=hidden]:optional"), Match::No);
        assert_eq!(r("input[type=range]:required"), Match::No);
        assert_eq!(r("input:required"), Match::Yes);
        assert_eq!(r("textarea:required"), Match::No);
        assert_eq!(r("textarea:optional"), Match::Yes);
        assert_eq!(r("div:required"), Match::No);
        // Möglich: Zustände gelten weiter als erfüllt.
        assert_eq!(any_match(&d, "#b1:disabled", Mode::Possible), Match::Yes);
    }

    #[test]
    fn liste_trifft_wenn_ein_teil_trifft() {
        assert_eq!(t(".spinner, .card"), Match::Yes);
        assert_eq!(t(".spinner, .loader"), Match::No);
    }
}
