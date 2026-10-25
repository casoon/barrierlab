//! Identität über Vorkommen hinaus: Befund, Ursache, Auswirkung, Kriterium.
//!
//! Ein [`Finding`](crate::Finding) ist ein Ergebnis an einer Stelle. Berichte
//! fassen Vorkommen zu Befunden zusammen und Befunde zu Ursachen — und jeder
//! Bericht, der das selbst tut, vergibt andere Schlüssel. Dann lässt sich
//! weder zwischen zwei Läufen vergleichen noch zwischen zwei Werkzeugen. Die
//! Funktionen hier legen den Schlüssel einmal fest.
//!
//! ```
//! use a11y_report::{FindingId, RootCauseId};
//!
//! // Dieselbe Regel am selben Element-Muster: dieselbe ID, auf jeder Seite
//! // und unabhängig von der Reihenfolge der Vorkommen.
//! let a = FindingId::new("buttons/name-missing", ["nav > a.toggle-12", "nav > a.toggle-7"]);
//! let b = FindingId::new("buttons/name-missing", ["nav > a.toggle-3"]);
//! assert_eq!(a, b);
//! assert!(a.as_str().starts_with("F-"));
//!
//! let cause = RootCauseId::new("buttons/name-missing", "nav > a.toggle-1");
//! assert!(cause.as_str().starts_with("RC-"));
//! ```

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Platzhalter für Vorkommen ohne eigenes Element, etwa „dem Dokument fehlt
/// eine Sprachangabe".
pub const PAGEWIDE_SELECTOR: &str = "__pagewide__";

/// Bringt einen CSS-Selektor in die Form, in der zwei Seiten desselben
/// Templates übereinstimmen.
///
/// Ziffern werden zu `#` (Listenpositionen, generierte IDs und Klassen wie
/// `item-12` unterscheiden sich von Seite zu Seite), Großbuchstaben klein,
/// Leerraum zu einem Leerzeichen; Folgen von `#` bzw. Leerzeichen fallen zu
/// einem Zeichen zusammen. Leere Eingabe und `document` stehen für die ganze
/// Seite ([`PAGEWIDE_SELECTOR`]).
pub fn normalize_selector(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() || trimmed == "document" {
        return PAGEWIDE_SELECTOR.to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut last = None;
    for ch in trimmed.chars() {
        let mapped = if ch.is_ascii_digit() {
            '#'
        } else if ch.is_whitespace() {
            ' '
        } else {
            ch.to_ascii_lowercase()
        };
        if (mapped == '#' || mapped == ' ') && last == Some(mapped) {
            continue;
        }
        out.push(mapped);
        last = Some(mapped);
    }
    out
}

/// FNV-1a, 64 Bit. Bewusst kein `DefaultHasher`: dessen Ausgabe ist zwischen
/// Rust-Versionen nicht stabil, eine ID muss es sein.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn short_hash(input: &str) -> String {
    format!("{:08x}", fnv1a64(input.as_bytes()) >> 32)
}

/// Stabile Kennung eines Befunds: eine Regel an einem Element-Muster.
///
/// Gebildet aus der Regelkennung und der Menge der normalisierten Selektoren
/// (sortiert, ohne Doppel). Damit ist sie
/// - **seitenübergreifend gleich**, wenn dasselbe Template auf mehreren Seiten
///   dieselbe Regel verletzt,
/// - **unabhängig** von der Reihenfolge der Vorkommen und von Knoten-IDs,
/// - **verschieden**, sobald sich Regel oder Element-Muster ändern.
///
/// Form: `F-` und acht Hexziffern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FindingId(String);

impl FindingId {
    pub fn new<'a>(rule_id: &str, selectors: impl IntoIterator<Item = &'a str>) -> Self {
        let mut patterns: BTreeSet<String> =
            selectors.into_iter().map(normalize_selector).collect();
        if patterns.is_empty() {
            patterns.insert(PAGEWIDE_SELECTOR.to_string());
        }
        let mut key = String::from(rule_id);
        for pattern in &patterns {
            key.push('\n');
            key.push_str(pattern);
        }
        FindingId(format!("F-{}", short_hash(&key)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stabile Kennung einer Ursache: eine Regel an einem Selektor-Muster,
/// ohne Seitenbezug. Mehrere Seiten mit demselben Muster teilen sie.
///
/// Form: `RC-` und acht Hexziffern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RootCauseId(String);

impl RootCauseId {
    pub fn new(rule_id: &str, selector: &str) -> Self {
        let key = format!("{rule_id}\n{}", normalize_selector(selector));
        RootCauseId(format!("RC-{}", short_hash(&key)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RootCauseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Wie gut eine Ursache belegt ist.
///
/// Wird nie hochgestuft: Ein gleiches Selektor-Muster belegt noch keine
/// gemeinsame Komponente, erst gleiches Markup auf allen Seiten tut das.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseConfidence {
    /// Gleiches Markup auf allen betroffenen Seiten.
    Confirmed,
    /// Gleiches Selektor-Muster, Markup nicht überall gleich oder unbekannt.
    Likely,
    /// Wiederkehrend, aber unter der Schwelle für eine Aussage über eine
    /// gemeinsame Herkunft.
    Open,
}

/// Bereich, in dem ein Befund wirkt. Ein Befund verweist auf Bereiche —
/// ein Bereich erzeugt keinen eigenen Befund für dieselbe Beobachtung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactArea {
    Accessibility,
    AssistiveTechnology,
    Legal,
    Conversion,
    Trust,
    Seo,
    Ux,
}

/// Wie gut eine Auswirkung belegt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBasis {
    /// Die Folge ist gemessen — etwa: das Element hat keinen Namen, also
    /// nennt ein Screenreader keinen Zweck.
    Measured,
    /// Aus der Struktur abgeleitet, nicht gemessen — etwa: ein Ablauf über
    /// dieses Element ist erschwert.
    Plausible,
    /// Nicht geprüft — etwa: Conversion-Verlust oder Markenwirkung.
    Hypothetical,
}

/// Zustand eines Bewertungskriteriums.
///
/// „Nicht erkannt" ist nicht „nicht erfüllt": Ein Merkmal, das die Prüfung
/// nicht findet, kann trotzdem vorhanden sein. Nur [`CriterionState::NotMet`]
/// darf einen Wert senken; `NotApplicable` und `NotChecked` verlassen den
/// Nenner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionState {
    Met,
    NotMet,
    NotDetected,
    NotApplicable,
    NotChecked,
}

impl CriterionState {
    /// Ob das Kriterium in den Nenner einer Bewertung eingeht.
    pub fn counts(self) -> bool {
        !matches!(
            self,
            CriterionState::NotApplicable | CriterionState::NotChecked
        )
    }

    /// Ob das Kriterium Punkte kosten darf.
    pub fn may_deduct(self) -> bool {
        self == CriterionState::NotMet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisiert_ziffern_leerraum_und_schreibweise() {
        assert_eq!(normalize_selector("  NAV > A.Item-12  "), "nav > a.item-#");
        assert_eq!(
            normalize_selector("li:nth-child(3)  >  a"),
            "li:nth-child(#) > a"
        );
        assert_eq!(normalize_selector("document"), PAGEWIDE_SELECTOR);
        assert_eq!(normalize_selector(""), PAGEWIDE_SELECTOR);
    }

    #[test]
    fn befund_id_ist_reihenfolge_und_seitenunabhaengig() {
        let a = FindingId::new("r", ["a.x-1", "b.y"]);
        let b = FindingId::new("r", ["b.y", "a.x-99", "a.x-2"]);
        assert_eq!(a, b);
    }

    #[test]
    fn befund_id_unterscheidet_regel_und_muster() {
        let basis = FindingId::new("r", ["a.x"]);
        assert_ne!(basis, FindingId::new("s", ["a.x"]));
        assert_ne!(basis, FindingId::new("r", ["a.y"]));
        assert_ne!(basis, FindingId::new("r", ["a.x", "a.y"]));
    }

    /// Die ID ist Teil eines veröffentlichten Formats: Ändert sich der Wert,
    /// lassen sich alte Berichte nicht mehr zuordnen. Dieser Test hält ihn fest.
    #[test]
    fn befund_id_ist_ueber_versionen_stabil() {
        assert_eq!(
            FindingId::new("buttons/name-missing", ["a.mainnav-toggle"]).as_str(),
            FindingId::new("buttons/name-missing", ["A.MAINNAV-TOGGLE"]).as_str()
        );
        let id = FindingId::new("buttons/name-missing", ["a.mainnav-toggle"]);
        assert_eq!(id.as_str().len(), 10);
        assert_eq!(id.as_str(), FIXED_ID);
    }

    // Festgehaltener Wert aus der ersten Version (0.25.0).
    const FIXED_ID: &str = "F-684c1d32";

    #[test]
    fn ohne_selektor_gilt_die_ganze_seite() {
        assert_eq!(
            FindingId::new("document/lang-missing", std::iter::empty()),
            FindingId::new("document/lang-missing", ["document"])
        );
    }

    #[test]
    fn ursache_und_befund_haben_eigene_praefixe() {
        assert!(RootCauseId::new("r", "a.x").as_str().starts_with("RC-"));
        assert!(FindingId::new("r", ["a.x"]).as_str().starts_with("F-"));
    }

    #[test]
    fn nur_nicht_erfuellt_darf_abziehen() {
        use CriterionState::*;
        for state in [Met, NotMet, NotDetected, NotApplicable, NotChecked] {
            assert_eq!(state.may_deduct(), state == NotMet, "{state:?}");
        }
        assert!(!NotApplicable.counts() && !NotChecked.counts());
        assert!(NotDetected.counts() && Met.counts() && NotMet.counts());
    }

    #[test]
    fn serialisiert_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&CriterionState::NotApplicable).unwrap(),
            "\"not_applicable\""
        );
        assert_eq!(
            serde_json::to_string(&ImpactArea::AssistiveTechnology).unwrap(),
            "\"assistive_technology\""
        );
        assert_eq!(
            serde_json::to_string(&FindingId::new("r", ["a"])).unwrap(),
            format!("\"{}\"", FindingId::new("r", ["a"]))
        );
    }
}
