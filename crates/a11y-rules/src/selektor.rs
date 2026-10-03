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
//! `:nth-child()`, Namensräume, Escapes), ist [`Treffer::Unbekannt`]. Eine
//! Regel behandelt das wie einen Treffer — lieber ein Hinweis zu viel als
//! eine stille Lücke.
//!
//! Zustände (`:hover`, `:focus`, …) und Pseudo-Elemente (`::before`, …)
//! gelten als erfüllt: `.card:hover` trifft, wo `.card` steht. So rechnete
//! auch auditmysite.

use a11y_dom::{Document, Node, NodeKind};

/// Ergebnis eines Abgleichs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Treffer {
    Ja,
    Nein,
    Unbekannt,
}

impl Treffer {
    fn und(self, other: Treffer) -> Treffer {
        match (self, other) {
            (Treffer::Nein, _) | (_, Treffer::Nein) => Treffer::Nein,
            (Treffer::Ja, Treffer::Ja) => Treffer::Ja,
            _ => Treffer::Unbekannt,
        }
    }

    fn oder(self, other: Treffer) -> Treffer {
        match (self, other) {
            (Treffer::Ja, _) | (_, Treffer::Ja) => Treffer::Ja,
            (Treffer::Nein, Treffer::Nein) => Treffer::Nein,
            _ => Treffer::Unbekannt,
        }
    }

    fn nicht(self) -> Treffer {
        match self {
            Treffer::Ja => Treffer::Nein,
            Treffer::Nein => Treffer::Ja,
            Treffer::Unbekannt => Treffer::Unbekannt,
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
    Wurzel,
    ErstesKind,
    LetztesKind,
    EinzigesKind,
    Leer,
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

    fn name(&mut self) -> Option<String> {
        let start = self.i;
        while self
            .sieh()
            .is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii())
        {
            self.i += 1;
        }
        (self.i > start).then(|| self.z[start..self.i].iter().collect())
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
            return Some(Einfach::Erfuellt);
        }
        Some(match (name.as_str(), argument) {
            ("not", Some(a)) => liste(&a).map_or(Einfach::Unbekannt, Einfach::Nicht),
            ("is" | "where" | "matches" | "-webkit-any" | "-moz-any", Some(a)) => {
                liste(&a).map_or(Einfach::Unbekannt, Einfach::Eines)
            }
            ("root", None) => Einfach::Wurzel,
            ("first-child", None) => Einfach::ErstesKind,
            ("last-child", None) => Einfach::LetztesKind,
            ("only-child", None) => Einfach::EinzigesKind,
            ("empty", None) => Einfach::Leer,
            (n, None) if ZUSTAENDE.contains(&n) => Einfach::Erfuellt,
            ("lang" | "dir", Some(_)) => Einfach::Unbekannt,
            _ => Einfach::Unbekannt,
        })
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

fn einfach<'a, N: Node<'a>>(n: N, e: &Einfach) -> Treffer {
    let ja = |b: bool| if b { Treffer::Ja } else { Treffer::Nein };
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
                return Treffer::Nein;
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
        Einfach::Nicht(l) => komplex_liste(n, l).nicht(),
        Einfach::Eines(l) => komplex_liste(n, l),
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
        Einfach::Erfuellt => Treffer::Ja,
        Einfach::Unbekannt => Treffer::Unbekannt,
    }
}

fn zusammen<'a, N: Node<'a>>(n: N, z: &Zusammengesetzt) -> Treffer {
    z.iter().fold(Treffer::Ja, |t, e| t.und(einfach(n, e)))
}

/// Gleicht `rest` (von rechts nach links) ab, ausgehend von `n`, das schon
/// zum zusammengesetzten Selektor rechts davon passt.
fn kette<'a, N: Node<'a>>(n: N, rest: &[(Kombinator, Zusammengesetzt)]) -> Treffer {
    let Some(((k, z), weiter)) = rest.split_first() else {
        return Treffer::Ja;
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
        .map(|c| zusammen(c, z).und(kette(c, weiter)))
        .fold(Treffer::Nein, Treffer::oder)
}

fn komplex<'a, N: Node<'a>>(n: N, k: &Komplex) -> Treffer {
    let t = zusammen(n, &k.subjekt);
    if t == Treffer::Nein {
        return t;
    }
    t.und(kette(n, &k.davor))
}

fn komplex_liste<'a, N: Node<'a>>(n: N, l: &[Komplex]) -> Treffer {
    l.iter()
        .map(|k| komplex(n, k))
        .fold(Treffer::Nein, Treffer::oder)
}

/// Eine einmal gelesene Selektorliste, für viele Abgleiche.
#[derive(Debug, Clone)]
pub(crate) struct Selektor(Option<Vec<Komplex>>);

impl Selektor {
    pub(crate) fn lies(text: &str) -> Self {
        Selektor(liste(text.trim()))
    }

    /// Trifft die Liste dieses Element?
    pub(crate) fn trifft<'a, N: Node<'a>>(&self, n: N) -> Treffer {
        match &self.0 {
            Some(l) => komplex_liste(n, l),
            None => Treffer::Unbekannt,
        }
    }
}

/// Trifft der Selektor ein Element im Dokument (in der Sicht der Regel)?
///
/// Ein Selektor, der sich nicht lesen lässt, ist [`Treffer::Unbekannt`].
pub(crate) fn trifft<D: Document>(doc: &D, selektor: &str) -> Treffer {
    let Some(l) = liste(selektor.trim()) else {
        return Treffer::Unbekannt;
    };
    let mut ergebnis = Treffer::Nein;
    for n in a11y_dom::elements(doc) {
        ergebnis = ergebnis.oder(komplex_liste(n, &l));
        if ergebnis == Treffer::Ja {
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

    fn t(sel: &str) -> Treffer {
        trifft(&seite(), sel)
    }

    #[test]
    fn einfache_selektoren() {
        assert_eq!(t("article"), Treffer::Ja);
        assert_eq!(t("ARTICLE.card"), Treffer::Ja);
        assert_eq!(t("#inhalt"), Treffer::Ja);
        assert_eq!(t(".spinner"), Treffer::Nein);
        assert_eq!(t("*"), Treffer::Ja);
        assert_eq!(t("video"), Treffer::Nein);
    }

    #[test]
    fn kombinatoren() {
        assert_eq!(t("main article > a"), Treffer::Ja);
        assert_eq!(t("main > a"), Treffer::Nein);
        assert_eq!(t("h2 + a"), Treffer::Ja);
        assert_eq!(t("a + h2"), Treffer::Nein);
        assert_eq!(t("li ~ li.aktiv"), Treffer::Ja);
        assert_eq!(t("body .card a"), Treffer::Ja);
    }

    #[test]
    fn attribute() {
        assert_eq!(t("a[href]"), Treffer::Ja);
        assert_eq!(t("[data-track^=teaser]"), Treffer::Ja);
        assert_eq!(t("[data-track$=\"link\"]"), Treffer::Ja);
        assert_eq!(t("[data-track*=x]"), Treffer::Nein);
        assert_eq!(t("[class~=teaser]"), Treffer::Ja);
        assert_eq!(t("[data-track='TEASER-LINK' i]"), Treffer::Ja);
        assert_eq!(t("[data-track='TEASER-LINK']"), Treffer::Nein);
    }

    #[test]
    fn zustaende_und_pseudo_elemente_gelten_als_erfuellt() {
        assert_eq!(t(".card:hover"), Treffer::Ja);
        assert_eq!(t("a:focus-visible::after"), Treffer::Ja);
        assert_eq!(t(".spinner:hover"), Treffer::Nein);
        assert_eq!(t("li:before"), Treffer::Ja);
    }

    #[test]
    fn logische_pseudoklassen() {
        assert_eq!(t("li:not(.aktiv)"), Treffer::Ja);
        assert_eq!(t("a:not([href])"), Treffer::Nein);
        assert_eq!(t(":is(video, .card)"), Treffer::Ja);
        assert_eq!(t(":where(.x, .y)"), Treffer::Nein);
        assert_eq!(t("li:first-child"), Treffer::Ja);
        assert_eq!(t("h2:last-child"), Treffer::Nein);
        assert_eq!(t("html:root"), Treffer::Ja);
    }

    #[test]
    fn unbekanntes_ist_unbekannt() {
        assert_eq!(t("article:has(a)"), Treffer::Unbekannt);
        assert_eq!(t("li:nth-child(2n)"), Treffer::Unbekannt);
        assert_eq!(t("svg|rect"), Treffer::Unbekannt);
        assert_eq!(t("a[href"), Treffer::Unbekannt);
        // Ein sicheres Nein bleibt Nein, auch neben Unbekanntem.
        assert_eq!(t("video:nth-child(2)"), Treffer::Nein);
    }

    #[test]
    fn liste_trifft_wenn_ein_teil_trifft() {
        assert_eq!(t(".spinner, .card"), Treffer::Ja);
        assert_eq!(t(".spinner, .loader"), Treffer::Nein);
    }
}
