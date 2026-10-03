//! `hreflang`: Sprachcodes, `x-default` und der Verweis auf die eigene Seite.
//!
//! Eingabe sind die `<link rel="alternate" hreflang>`-Einträge einer Seite als
//! [`Alternate`], so wie der Host sie gefunden hat. Relative `href` löst der
//! Host vorher auf — gegen die Seiten-URL, wie es der Browser tut. Beide Hosts
//! können das: auditmysite über `new URL()` in der Seite, astro-post-audit
//! über die Basis-URL des Builds.
//!
//! Was ein Host allein prüft, bleibt dort: die Rückverweise zwischen Seiten
//! (nur astro-post-audit, braucht den ganzen Build) und ob ein Ziel im Build
//! existiert (nur astro-post-audit, braucht das Dateisystem).

/// Ein `<link rel="alternate" hreflang="…" href="…">`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternate {
    pub hreflang: String,
    /// Absolut, vom Host aufgelöst.
    pub href: String,
}

/// Ob der Wert `x-default` ist. Groß- und Kleinschreibung zählt nicht —
/// Sprachcodes nach BCP 47 sind unabhängig davon, und Google wertet
/// `hreflang` ebenso aus. Vorher verglichen beide Hosts exakt.
pub fn is_x_default(hreflang: &str) -> bool {
    hreflang.trim().eq_ignore_ascii_case("x-default")
}

/// Ob ein `hreflang`-Wert ein Code ist, den Suchmaschinen auswerten:
/// `x-default` oder Sprache (ISO 639, zwei oder drei Buchstaben), optional
/// Schrift (ISO 15924, vier Buchstaben) und Region (ISO 3166-1 Alpha-2), mit
/// Bindestrich getrennt, Groß- und Kleinschreibung beliebig.
///
/// Das ist die Teilmenge von BCP 47, die Google für `hreflang` dokumentiert.
/// auditmysite prüfte vorher nur `sprache` und `sprache-REGION` mit großer
/// Region und hielt damit `de-de` und `zh-Hant-TW` für ungültig. Ob der Code
/// in den ISO-Listen tatsächlich vergeben ist, prüft diese Funktion nicht;
/// `xx-YY` besteht.
pub fn is_valid_code(hreflang: &str) -> bool {
    let wert = hreflang.trim();
    if is_x_default(wert) {
        return true;
    }
    let mut teile = wert.split('-');
    let sprache = teile.next().unwrap_or("");
    if !(2..=3).contains(&sprache.len()) || !buchstaben(sprache) {
        return false;
    }
    let mut rest: Vec<&str> = teile.collect();
    if rest.first().is_some_and(|s| s.len() == 4 && buchstaben(s)) {
        rest.remove(0);
    }
    match rest.as_slice() {
        [] => true,
        [region] => region.len() == 2 && buchstaben(region),
        _ => false,
    }
}

fn buchstaben(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Ob die Menge einen `x-default`-Eintrag hat.
pub fn has_x_default(alternates: &[Alternate]) -> bool {
    alternates.iter().any(|a| is_x_default(&a.hreflang))
}

/// Die Einträge, deren Code [`is_valid_code`] nicht besteht.
pub fn invalid_codes(alternates: &[Alternate]) -> Vec<&Alternate> {
    alternates
        .iter()
        .filter(|a| !is_valid_code(&a.hreflang))
        .collect()
}

/// Ob ein Eintrag (außer `x-default`) auf die Seite selbst zeigt. Verglichen
/// wird mit [`same_page`].
///
/// `x-default` zählt nicht: Google verlangt, dass jede Sprachfassung sich
/// selbst unter ihrem Sprachcode nennt; `x-default` ist keine Sprachfassung.
/// Beide Hosts zählten es vorher mit — die deutsche Startseite von heise.de
/// nennt sich nur als `x-default` und galt damit als vollständig.
pub fn has_self_reference(alternates: &[Alternate], page_url: &str) -> bool {
    alternates
        .iter()
        .filter(|a| !is_x_default(&a.hreflang))
        .any(|a| same_page(&a.href, page_url))
}

/// Ob zwei absolute URLs dieselbe Seite meinen.
///
/// Schema und Host ohne Rücksicht auf Groß- und Kleinschreibung, das Fragment
/// fällt weg, ein abschließender Schrägstrich am Pfad zählt nicht. Die
/// Abfrage bleibt: Manche Seiten unterscheiden ihre Sprachfassungen nur
/// darüber (`?lang=de`). astro-post-audit verwarf sie vorher und hätte damit
/// jede solche Fassung als Verweis auf sich selbst gezählt; auditmysite
/// behielt sie.
pub fn same_page(a: &str, b: &str) -> bool {
    zerlege(a) == zerlege(b)
}

fn zerlege(url: &str) -> (String, String, String) {
    let ohne_fragment = url.trim().split('#').next().unwrap_or("");
    let (vorn, abfrage) = match ohne_fragment.split_once('?') {
        Some((v, q)) => (v, q),
        None => (ohne_fragment, ""),
    };
    let (ursprung, pfad) = match vorn.split_once("://") {
        Some((schema, rest)) => {
            let (host, pfad) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
            (format!("{schema}://{host}").to_ascii_lowercase(), pfad)
        }
        None => (String::new(), vorn),
    };
    let pfad = pfad.trim_end_matches('/');
    (ursprung, pfad.to_string(), abfrage.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alt(hreflang: &str, href: &str) -> Alternate {
        Alternate {
            hreflang: hreflang.into(),
            href: href.into(),
        }
    }

    /// Die Werte aller Seiten mit `hreflang` unter 48 echten Seiten (Abruf
    /// 2026-10-03): bahn.de, gmx.net, check24.de, mediamarkt.de, wetter.com,
    /// heise.de, berlin.de, bundesregierung.de, lidl.de. Alle gültig.
    #[test]
    fn codes_echter_seiten_sind_gueltig() {
        for code in [
            "cs",
            "da",
            "de",
            "en",
            "es",
            "fr",
            "it",
            "nl",
            "pl",
            "ar",
            "ru",
            "tr",
            "uk",
            "de-AT",
            "de-CH",
            "de-DE",
            "en-GB",
            "es-ES",
            "fr-BE",
            "fr-LU",
            "hu-HU",
            "pl-PL",
            "x-default",
        ] {
            assert!(is_valid_code(code), "{code}");
        }
    }

    #[test]
    fn codes_nach_bcp47_ohne_rueksicht_auf_schreibung() {
        for code in [
            "de-de",
            "EN-gb",
            "zh-Hant",
            "zh-Hant-TW",
            "X-Default",
            "fil",
        ] {
            assert!(is_valid_code(code), "{code}");
        }
        for code in [
            "", "d", "deutsch", "de_DE", "de-", "de-DEU", "es-419", "en-GB-x", "1a", "de DE",
        ] {
            assert!(!is_valid_code(code), "{code}");
        }
    }

    /// heise.de (Abruf 2026-10-03): `en` und `x-default`, aber kein `de` für
    /// die deutsche Startseite selbst. Beide Hosts zählten `x-default` als
    /// Verweis auf sich selbst und meldeten nichts.
    #[test]
    fn heise_de_verweist_nur_per_x_default_auf_sich() {
        let menge = [
            alt("en", "https://www.heise.de/en"),
            alt("x-default", "https://www.heise.de"),
        ];
        assert!(has_x_default(&menge));
        assert!(!has_x_default(&menge[..1]));
        assert!(!has_self_reference(&menge, "https://www.heise.de/"));
    }

    /// bahn.de: `de` zeigt ohne Schrägstrich auf die Seite, die der Browser
    /// als `https://www.bahn.de/` meldet.
    #[test]
    fn eigener_verweis_bahn_de() {
        let menge = [
            alt("de", "https://www.bahn.de"),
            alt("x-default", "https://int.bahn.de"),
            alt("en", "https://int.bahn.de/en"),
        ];
        assert!(has_self_reference(&menge, "https://www.bahn.de/"));
        assert!(has_self_reference(&menge, "https://WWW.bahn.de/#top"));
        assert!(!has_self_reference(&menge, "https://int.bahn.de/en/reisen"));
        assert!(!has_self_reference(&menge, "https://int.bahn.de/"));
    }

    #[test]
    fn x_default_allein_ist_kein_eigener_verweis() {
        let menge = [
            alt("en", "https://example.org/en/"),
            alt("x-default", "https://example.org/"),
        ];
        assert!(!has_self_reference(&menge, "https://example.org/"));
    }

    #[test]
    fn abfrage_unterscheidet_fassungen() {
        let menge = [alt("en", "https://example.org/?lang=en")];
        assert!(!has_self_reference(&menge, "https://example.org/?lang=de"));
        assert!(has_self_reference(&menge, "https://example.org?lang=en"));
    }

    #[test]
    fn ungueltige_codes_werden_benannt() {
        let menge = [alt("de", "https://a/"), alt("de_DE", "https://b/")];
        assert_eq!(invalid_codes(&menge), [&menge[1]]);
    }
}
