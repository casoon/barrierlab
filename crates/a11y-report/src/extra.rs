//! Der Erweiterungsslot.
//!
//! Der gemeinsame Vertrag kann nicht jedes Feld jedes Werkzeugs aufnehmen —
//! auditmysite hängt etwa einen Element-Screenshot an einen Befund, den es für
//! seinen PDF-Bericht zuschneidet. Solche Daten gehören nicht in den
//! JSON-Vertrag: Sie sind werkzeugspezifisch, oft nicht serialisierbar und für
//! einen fremden Auswerter bedeutungslos.
//!
//! Die naheliegende Alternative — eine Seitentabelle, die Befunde über ihren
//! Index adressiert — trägt nicht: Befunde werden gefiltert, sortiert und aus
//! mehreren Läufen zusammengeführt, und dabei verschieben sich Indizes.
//!
//! Deshalb ein undurchsichtiger Slot am Befund selbst: Er reist mit, wird nie
//! serialisiert und ist **nicht Teil der Identität** eines Befunds. Zwei
//! Befunde mit gleichem Inhalt und verschiedenen Beigaben gelten als gleich.

use std::any::Any;
use std::sync::Arc;

/// Werkzeugspezifische Beigabe an einem [`Finding`](crate::Finding) oder
/// [`RuleRun`](crate::RuleRun). Nie serialisiert.
///
/// ```
/// use a11y_report::{Extra, Finding};
///
/// struct Screenshot(Vec<u8>);
///
/// let f = Finding::fail("images/alt-missing", "kein alt")
///     .with_extra(Screenshot(vec![0x89, 0x50, 0x4e, 0x47]));
///
/// assert_eq!(f.extra.get::<Screenshot>().map(|s| s.0.len()), Some(4));
/// // Eine andere Sorte liegt nicht darin.
/// assert!(f.extra.get::<String>().is_none());
/// ```
#[derive(Clone, Default)]
pub struct Extra(Option<Arc<dyn Any + Send + Sync>>);

impl Extra {
    /// Leer — der Normalfall.
    pub fn none() -> Self {
        Extra(None)
    }

    /// Legt einen Wert ab. Ein bereits vorhandener wird ersetzt; der Slot hält
    /// genau eine Sorte. Wer mehreres anhängen will, legt eine eigene Struktur
    /// hinein.
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Extra(Some(Arc::new(value)))
    }

    /// Holt den Wert zurück, wenn er von der erwarteten Sorte ist.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.0.as_ref()?.downcast_ref::<T>()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

impl std::fmt::Debug for Extra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(_) => f.write_str("Extra(…)"),
            None => f.write_str("Extra(leer)"),
        }
    }
}

/// Beigaben zählen nicht zur Identität eines Befunds.
///
/// Sonst wären zwei inhaltlich gleiche Befunde ungleich, nur weil an einem ein
/// Screenshot hängt — und Vergleiche in Tests und beim Zusammenführen von
/// Läufen würden an etwas scheitern, das gar nicht Teil der Aussage ist.
impl PartialEq for Extra {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Extra {}
