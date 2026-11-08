//! Der Regelkatalog: alle Deklarationen in einer festen Reihenfolge.
//!
//! Die vier `*_metas()`-Funktionen liefern je Tier einen Slice. Wer „was
//! bedeutet Kennung X, welches Kriterium, welche Schwere“ beantworten will,
//! braucht dagegen eine Liste und eine Suche über alle Tiers. Beides steht
//! hier, ohne Regellogik und ohne Host.

use std::sync::OnceLock;

use crate::registry::Meta;
use crate::{rendering_metas, semantics_metas, structure_metas, stylesheet_metas};

/// Alle Regeldeklarationen: Tier 1, Tier 2, Tier 3, Stylesheets, innerhalb
/// eines Tiers in Deklarationsreihenfolge. Die Reihenfolge ist stabil, solange
/// keine Regel hinzukommt oder wegfällt.
///
/// Mit dem Feature `serde` ist jeder Eintrag serialisierbar (`ids`, `tier`,
/// `scope`, `wcag`, `severity`, `help`, `help_de`).
pub fn catalogue() -> &'static [Meta] {
    static ALL: OnceLock<Vec<Meta>> = OnceLock::new();
    ALL.get_or_init(|| {
        [
            structure_metas(),
            semantics_metas(),
            rendering_metas(),
            stylesheet_metas(),
        ]
        .concat()
    })
}

/// Die Deklaration, die die Befund-Kennung `id` (einen Eintrag aus
/// [`Meta::ids`]) erzeugen kann.
pub fn meta_for(id: &str) -> Option<&'static Meta> {
    catalogue().iter().find(|m| m.ids.contains(&id))
}
