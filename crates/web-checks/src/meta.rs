//! Länge von `<title>` und `<meta name="description">`.
//!
//! Gezählt werden **Zeichen**, nicht Bytes: Beide Hosts maßen vorher mit
//! `str::len`, womit jeder Umlaut doppelt zählte und „Überschrift" früher zu
//! lang war als „Ueberschrift". Suchmaschinen kürzen nach Pixelbreite; Zeichen
//! sind die nächstbessere Näherung, die ohne Schrift auskommt.
//!
//! Ob ein fehlender Titel ein Befund ist und wie schwer, entscheidet der Host.
//! Ebenso, welche Grenze er meldet — astro-post-audit etwa nur das Maximum.

use serde::{Deserialize, Serialize};

/// Empfohlene Länge eines Titels in Zeichen.
pub const TITLE: LengthRange = LengthRange { min: 30, max: 60 };

/// Empfohlene Länge einer Meta-Beschreibung in Zeichen.
pub const DESCRIPTION: LengthRange = LengthRange { min: 120, max: 160 };

/// Ein Bereich empfohlener Länge, beide Grenzen einschließlich.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LengthRange {
    pub min: usize,
    pub max: usize,
}

/// Wo eine Länge relativ zu einem [`LengthRange`] liegt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthClass {
    TooShort,
    Ok,
    TooLong,
}

impl LengthRange {
    /// Ordnet einen Text ein, gemessen mit [`length`].
    pub fn classify(&self, text: &str) -> LengthClass {
        let len = length(text);
        if len < self.min {
            LengthClass::TooShort
        } else if len > self.max {
            LengthClass::TooLong
        } else {
            LengthClass::Ok
        }
    }
}

/// Länge in Zeichen, nachdem Leerraum wie im Browser zusammengefasst ist:
/// Ränder fallen weg, Folgen von Leerraum zählen als ein Zeichen. So misst ein
/// Host mit dem Markup dasselbe wie einer mit `document.title`.
pub fn length(text: &str) -> usize {
    let words = text.split_whitespace();
    let mut count = 0;
    for (i, word) in words.enumerate() {
        if i > 0 {
            count += 1;
        }
        count += word.chars().count();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaehlt_zeichen_nicht_bytes() {
        assert_eq!(length("Überschrift"), 11);
        assert_eq!("Überschrift".len(), 12);
    }

    #[test]
    fn leerraum_wird_wie_im_browser_zusammengefasst() {
        assert_eq!(length("  Hallo \n\t Welt  "), 10);
        assert_eq!(length(""), 0);
        assert_eq!(length("   "), 0);
    }

    #[test]
    fn grenzen_sind_einschliesslich() {
        let r = LengthRange { min: 3, max: 5 };
        assert_eq!(r.classify("ab"), LengthClass::TooShort);
        assert_eq!(r.classify("abc"), LengthClass::Ok);
        assert_eq!(r.classify("abcde"), LengthClass::Ok);
        assert_eq!(r.classify("abcdef"), LengthClass::TooLong);
    }

    #[test]
    fn umlaute_kippen_die_einordnung_nicht_mehr() {
        // 60 Zeichen, davon zehn Umlaute: in Bytes 70, also früher „zu lang".
        let title = format!("{}{}", "ä".repeat(10), "a".repeat(50));
        assert_eq!(TITLE.classify(&title), LengthClass::Ok);
    }
}
