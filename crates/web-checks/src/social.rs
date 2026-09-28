//! OpenGraph und Twitter Cards: welche Tags zählen, wann eines vorhanden ist,
//! wie vollständig ein Satz ist, und zwei Wertprüfungen.
//!
//! Eingabe ist je Tag sein `content`, wie der Host ihn gefunden hat — aus dem
//! Markup oder aus dem DOM einer laufenden Seite. Bildgröße, Erreichbarkeit
//! und Abgleich mit dem Seitentitel brauchen Ein-/Ausgabe oder weiteren
//! Kontext und bleiben beim Host.

/// OpenGraph-Tags, ohne die eine Vorschau unvollständig ist.
pub const OPEN_GRAPH_REQUIRED: [&str; 4] = ["og:title", "og:description", "og:image", "og:url"];

/// OpenGraph-Tags, über die [`completeness`] rechnet.
pub const OPEN_GRAPH_FIELDS: [&str; 6] = [
    "og:title",
    "og:description",
    "og:image",
    "og:url",
    "og:type",
    "og:site_name",
];

/// Twitter-Card-Tags, ohne die eine Karte unvollständig ist.
pub const TWITTER_REQUIRED: [&str; 3] = ["twitter:card", "twitter:title", "twitter:description"];

/// Twitter-Card-Tags, über die [`completeness`] rechnet.
pub const TWITTER_FIELDS: [&str; 4] = [
    "twitter:card",
    "twitter:title",
    "twitter:description",
    "twitter:image",
];

/// Zulässige Werte für `twitter:card`.
pub const TWITTER_CARD_TYPES: [&str; 4] = ["summary", "summary_large_image", "app", "player"];

/// Ein Tag ist vorhanden, wenn es existiert **und** Inhalt trägt. Ein leeres
/// `og:image` liefert keiner Vorschau ein Bild.
pub fn is_present(content: Option<&str>) -> bool {
    content.is_some_and(|c| !c.trim().is_empty())
}

/// Sind alle `required` vorhanden? `content` liefert den Inhalt je Tagname.
pub fn is_complete<'a>(required: &[&str], content: impl Fn(&str) -> Option<&'a str>) -> bool {
    required.iter().all(|tag| is_present(content(tag)))
}

/// Anteil vorhandener `fields` in Prozent, abgerundet.
pub fn completeness<'a>(fields: &[&str], content: impl Fn(&str) -> Option<&'a str>) -> u32 {
    if fields.is_empty() {
        return 0;
    }
    let present = fields.iter().filter(|tag| is_present(content(tag))).count();
    (present * 100 / fields.len()) as u32
}

/// Ist der Wert ein zulässiger `twitter:card`-Typ? Groß-/Kleinschreibung
/// zählt; Twitter kennt nur die kleingeschriebenen Werte.
pub fn is_valid_twitter_card(value: &str) -> bool {
    TWITTER_CARD_TYPES.contains(&value.trim())
}

/// Ist die URL absolut (`http://` oder `https://`)? Plattformen lösen
/// relative `og:image`-Werte nicht gegen die Seite auf.
pub fn is_absolute_url(value: &str) -> bool {
    let v = value.trim();
    v.starts_with("https://") || v.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tags<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<&'a str> {
        let map: HashMap<&str, &str> = pairs.iter().copied().collect();
        move |tag| map.get(tag).copied()
    }

    #[test]
    fn leerer_inhalt_gilt_als_fehlend() {
        assert!(!is_present(None));
        assert!(!is_present(Some("")));
        assert!(!is_present(Some("  ")));
        assert!(is_present(Some("x")));
    }

    #[test]
    fn vollstaendigkeit_und_pflichtangaben() {
        let og = tags(&[
            ("og:title", "T"),
            ("og:description", "D"),
            ("og:image", "https://e.x/i.png"),
            ("og:url", ""),
        ]);
        assert!(!is_complete(&OPEN_GRAPH_REQUIRED, &og));
        assert_eq!(completeness(&OPEN_GRAPH_FIELDS, &og), 50);

        let twitter = tags(&[
            ("twitter:card", "summary"),
            ("twitter:title", "T"),
            ("twitter:description", "D"),
        ]);
        assert!(is_complete(&TWITTER_REQUIRED, &twitter));
        assert_eq!(completeness(&TWITTER_FIELDS, &twitter), 75);
    }

    #[test]
    fn twitter_card_werte() {
        assert!(is_valid_twitter_card("summary_large_image"));
        assert!(is_valid_twitter_card(" summary "));
        assert!(!is_valid_twitter_card("Summary"));
        assert!(!is_valid_twitter_card("large"));
    }

    #[test]
    fn absolute_urls() {
        assert!(is_absolute_url("https://example.com/og.png"));
        assert!(is_absolute_url("http://example.com/og.png"));
        assert!(!is_absolute_url("/og.png"));
        assert!(!is_absolute_url("//example.com/og.png"));
    }
}
