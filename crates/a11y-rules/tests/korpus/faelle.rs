//! Die Fälle, nach Regelgruppen in eigenen Dateien.

use a11y_report::Outcome;

mod gruppen {
    pub mod aria;
    pub mod checkliste;
    pub mod darstellung;
    pub mod dokument;
    pub mod formulare;
    pub mod landmarken;
    pub mod namen;
    pub mod regressionen;
    pub mod struktur;
}

/// Welche Art von Fall.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// Die Kennung wird gemeldet, jeder Befund mit genau diesem Ergebnis.
    Positive(Outcome),
    /// Ähnlich aussehend, aber konform: die Kennung lief und meldet nichts.
    Negative,
    /// Nichts, worauf die Regel zielt: die Kennung lief und meldet nichts.
    NotApplicable,
}

impl Kind {
    pub fn index(self) -> usize {
        match self {
            Kind::Positive(_) => 0,
            Kind::Negative => 1,
            Kind::NotApplicable => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Positive(_) => "positiv",
            Kind::Negative => "negativ",
            Kind::NotApplicable => "n/a",
        }
    }
}

/// Welche Tiers der Host für den Fall liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Struktur, Semantik, Darstellung und Stylesheets (`run_full` +
    /// `run_stylesheets`). Vorgabe.
    Full,
    /// Struktur und Semantik, wie auditmysite (`run_with_semantics`).
    Semantics,
}

/// Ein Fall: Kennung, Art, Dokument und die Norm, aus der die Erwartung
/// stammt.
#[derive(Debug, Clone)]
pub struct Fall {
    pub id: &'static str,
    pub kind: Kind,
    pub html: &'static str,
    pub source: &'static str,
    /// Grenzwert oder Variante, etwa „4,49:1" oder „aria-labelledby".
    pub variant: Option<&'static str>,
    pub mode: Mode,
    /// Nur bei positiven Fällen: genau so viele Befunde.
    pub count: Option<usize>,
}

impl Fall {
    pub fn variant(mut self, v: &'static str) -> Self {
        self.variant = Some(v);
        self
    }

    pub fn mode(mut self, m: Mode) -> Self {
        self.mode = m;
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = Some(n);
        self
    }
}

fn fall(id: &'static str, kind: Kind, html: &'static str, source: &'static str) -> Fall {
    Fall {
        id,
        kind,
        html,
        source,
        variant: None,
        mode: Mode::Full,
        count: None,
    }
}

/// Die Kennung wird mit `Fail` gemeldet.
pub fn fail(id: &'static str, html: &'static str, source: &'static str) -> Fall {
    fall(id, Kind::Positive(Outcome::Fail), html, source)
}

/// Die Kennung wird mit `Review` gemeldet.
pub fn review(id: &'static str, html: &'static str, source: &'static str) -> Fall {
    fall(id, Kind::Positive(Outcome::Review), html, source)
}

/// Die Kennung wird mit `Untested` gemeldet.
pub fn untested(id: &'static str, html: &'static str, source: &'static str) -> Fall {
    fall(id, Kind::Positive(Outcome::Untested), html, source)
}

/// Ähnlich, aber konform: nichts gemeldet.
pub fn negative(id: &'static str, html: &'static str, source: &'static str) -> Fall {
    fall(id, Kind::Negative, html, source)
}

/// Nichts, worauf die Regel zielt: nichts gemeldet.
pub fn na(id: &'static str, html: &'static str, source: &'static str) -> Fall {
    fall(id, Kind::NotApplicable, html, source)
}

pub fn all() -> Vec<Fall> {
    [
        gruppen::aria::cases(),
        gruppen::checkliste::cases(),
        gruppen::darstellung::cases(),
        gruppen::dokument::cases(),
        gruppen::formulare::cases(),
        gruppen::landmarken::cases(),
        gruppen::namen::cases(),
        gruppen::regressionen::cases(),
        gruppen::struktur::cases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
