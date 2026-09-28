//! Die Sprache der Texte, die eine Regel ausgibt.
//!
//! Übersetzt wird nur, was ein Werkzeug einem Menschen zeigt: `message`, der
//! Grund eines nicht gelaufenen Vermerks und der Hinweis der Deklaration.
//! Kennungen, Outcomes, Schweregrade und WCAG-Bezüge hängen nicht von der
//! Sprache ab — ein Befund heißt in jeder Sprache gleich.
//!
//! Die deutschen Texte hängen am Feature `de` (Vorgabe: an). Ohne das Feature
//! gibt es [`Locale::De`] nicht, und die deutschen Vorlagen kommen gar nicht
//! erst in den Binärcode — das zählt, wo jedes Kilobyte ausgeliefert wird
//! (`a11y-wasm`).

/// Sprache der Befundtexte.
///
/// Vorgabe ist Englisch; die Einstiege ohne `_in` laufen damit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Locale {
    #[default]
    En,
    /// Nur mit dem Feature `de`.
    #[cfg(feature = "de")]
    De,
}

/// Ein fester Text je Sprache: erst Englisch, dann Deutsch.
#[cfg(feature = "de")]
macro_rules! pick {
    ($locale:expr, $en:expr, $de:expr $(,)?) => {
        match $locale {
            $crate::locale::Locale::En => $en,
            $crate::locale::Locale::De => $de,
        }
    };
}

/// Ohne Feature `de`: nur der englische Text; der deutsche wird verworfen.
#[cfg(not(feature = "de"))]
macro_rules! pick {
    ($locale:expr, $en:expr, $de:expr $(,)?) => {{
        let _ = $locale;
        $en
    }};
}

/// `format!` je Sprache: erst die englische, dann die deutsche Vorlage, danach
/// die Argumente für beide. Implizit eingefangene Namen (`{id}`) gehen wie bei
/// `format!`.
#[cfg(feature = "de")]
macro_rules! tr {
    ($locale:expr, $en:literal, $de:literal $(, $arg:expr)* $(,)?) => {
        match $locale {
            $crate::locale::Locale::En => format!($en $(, $arg)*),
            $crate::locale::Locale::De => format!($de $(, $arg)*),
        }
    };
}

/// Ohne Feature `de`: nur die englische Vorlage; die deutsche wird verworfen.
#[cfg(not(feature = "de"))]
macro_rules! tr {
    ($locale:expr, $en:literal, $de:literal $(, $arg:expr)* $(,)?) => {{
        let _ = $locale;
        format!($en $(, $arg)*)
    }};
}

pub(crate) use {pick, tr};
