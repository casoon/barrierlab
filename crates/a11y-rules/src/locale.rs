//! Die Sprache der Texte, die eine Regel ausgibt.
//!
//! Übersetzt wird nur, was ein Werkzeug einem Menschen zeigt: `message`, der
//! Grund eines nicht gelaufenen Vermerks und der Hinweis der Deklaration.
//! Kennungen, Outcomes, Schweregrade und WCAG-Bezüge hängen nicht von der
//! Sprache ab — ein Befund heißt in jeder Sprache gleich.

/// Sprache der Befundtexte.
///
/// Vorgabe ist Englisch; die Einstiege ohne `_in` laufen damit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Locale {
    #[default]
    En,
    De,
}

impl Locale {
    /// Wählt den Text zur Sprache.
    pub(crate) fn pick<'a>(self, en: &'a str, de: &'a str) -> &'a str {
        match self {
            Locale::En => en,
            Locale::De => de,
        }
    }
}

/// `format!` je Sprache: erst die englische, dann die deutsche Vorlage, danach
/// die Argumente für beide. Implizit eingefangene Namen (`{id}`) gehen wie bei
/// `format!`.
macro_rules! tr {
    ($locale:expr, $en:literal, $de:literal $(, $arg:expr)* $(,)?) => {
        match $locale {
            $crate::locale::Locale::En => format!($en $(, $arg)*),
            $crate::locale::Locale::De => format!($de $(, $arg)*),
        }
    };
}

pub(crate) use tr;
