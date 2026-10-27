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
pub enum Art {
    /// Die Kennung wird gemeldet, jeder Befund mit genau diesem Ergebnis.
    Positiv(Outcome),
    /// Ähnlich aussehend, aber konform: die Kennung lief und meldet nichts.
    Negativ,
    /// Nichts, worauf die Regel zielt: die Kennung lief und meldet nichts.
    NichtAnwendbar,
}

impl Art {
    pub fn index(self) -> usize {
        match self {
            Art::Positiv(_) => 0,
            Art::Negativ => 1,
            Art::NichtAnwendbar => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Art::Positiv(_) => "positiv",
            Art::Negativ => "negativ",
            Art::NichtAnwendbar => "n/a",
        }
    }
}

/// Welche Tiers der Host für den Fall liefert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modus {
    /// Struktur, Semantik, Darstellung und Stylesheets (`run_full` +
    /// `run_stylesheets`). Vorgabe.
    Voll,
    /// Struktur und Semantik, wie auditmysite (`run_with_semantics`).
    Semantik,
}

/// Ein Fall: Kennung, Art, Dokument und die Norm, aus der die Erwartung
/// stammt.
#[derive(Debug, Clone)]
pub struct Fall {
    pub id: &'static str,
    pub art: Art,
    pub html: &'static str,
    pub quelle: &'static str,
    /// Grenzwert oder Variante, etwa „4,49:1" oder „aria-labelledby".
    pub variante: Option<&'static str>,
    pub modus: Modus,
    /// Nur bei positiven Fällen: genau so viele Befunde.
    pub anzahl: Option<usize>,
}

impl Fall {
    pub fn variante(mut self, v: &'static str) -> Self {
        self.variante = Some(v);
        self
    }

    pub fn modus(mut self, m: Modus) -> Self {
        self.modus = m;
        self
    }

    pub fn anzahl(mut self, n: usize) -> Self {
        self.anzahl = Some(n);
        self
    }
}

fn fall(id: &'static str, art: Art, html: &'static str, quelle: &'static str) -> Fall {
    Fall {
        id,
        art,
        html,
        quelle,
        variante: None,
        modus: Modus::Voll,
        anzahl: None,
    }
}

/// Die Kennung wird mit `Fail` gemeldet.
pub fn fail(id: &'static str, html: &'static str, quelle: &'static str) -> Fall {
    fall(id, Art::Positiv(Outcome::Fail), html, quelle)
}

/// Die Kennung wird mit `Review` gemeldet.
pub fn review(id: &'static str, html: &'static str, quelle: &'static str) -> Fall {
    fall(id, Art::Positiv(Outcome::Review), html, quelle)
}

/// Die Kennung wird mit `Untested` gemeldet.
pub fn untested(id: &'static str, html: &'static str, quelle: &'static str) -> Fall {
    fall(id, Art::Positiv(Outcome::Untested), html, quelle)
}

/// Ähnlich, aber konform: nichts gemeldet.
pub fn negativ(id: &'static str, html: &'static str, quelle: &'static str) -> Fall {
    fall(id, Art::Negativ, html, quelle)
}

/// Nichts, worauf die Regel zielt: nichts gemeldet.
pub fn na(id: &'static str, html: &'static str, quelle: &'static str) -> Fall {
    fall(id, Art::NichtAnwendbar, html, quelle)
}

pub fn alle() -> Vec<Fall> {
    [
        gruppen::aria::faelle(),
        gruppen::checkliste::faelle(),
        gruppen::darstellung::faelle(),
        gruppen::dokument::faelle(),
        gruppen::formulare::faelle(),
        gruppen::landmarken::faelle(),
        gruppen::namen::faelle(),
        gruppen::regressionen::faelle(),
        gruppen::struktur::faelle(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
