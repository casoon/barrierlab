use super::*;
use std::collections::HashSet;

fn id(m: Option<Match>) -> Option<&'static str> {
    m.map(|m| m.provider.id)
}

#[test]
fn host_matches_domain_and_subdomain() {
    assert_eq!(
        id(classify_host("google-analytics.com")),
        Some("google-analytics")
    );
    assert_eq!(
        id(classify_host("www.google-analytics.com")),
        Some("google-analytics")
    );
    assert_eq!(
        id(classify_host("WWW.Google-Analytics.COM.")),
        Some("google-analytics")
    );
}

#[test]
fn look_alike_hosts_do_not_match() {
    for h in [
        "google-analytics.com.evil.example",
        "notgoogle-analytics.com",
        "google-analytics.co",
        "evil.example",
        "",
    ] {
        assert!(classify_host(h).is_none(), "{h}");
    }
}

#[test]
fn url_userinfo_trick_uses_the_real_host() {
    assert!(classify_url("https://google-analytics.com@evil.example/collect").is_none());
    assert_eq!(
        id(classify_url(
            "https://evil.example@www.google-analytics.com/collect"
        )),
        Some("google-analytics")
    );
}

#[test]
fn url_forms() {
    assert_eq!(
        id(classify_url(
            "https://fonts.googleapis.com:443/css2?family=Inter"
        )),
        Some("google-fonts")
    );
    assert_eq!(
        id(classify_url("//fonts.gstatic.com/s/inter.woff2")),
        Some("google-fonts")
    );
    assert_eq!(
        id(classify_url("https://fonts.gstatic.com")),
        Some("google-fonts")
    );
    assert_eq!(
        id(classify_url("https://fonts.gstatic.com#x")),
        Some("google-fonts")
    );
    for u in [
        "/local/path.js",
        "data:text/plain,google-analytics.com",
        "blob:https://x",
        "fonts.googleapis.com/css",
        "https://",
    ] {
        assert!(classify_url(u).is_none(), "{u}");
    }
}

#[test]
fn host_of_strips_port_credentials_and_dot() {
    assert_eq!(
        host_of("https://User:pw@Example.COM.:8443/a?b#c").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        host_of("//cdn.example.com/x").as_deref(),
        Some("cdn.example.com")
    );
    assert_eq!(host_of("https://[::1]:8080/x").as_deref(), Some("[::1]"));
    assert_eq!(host_of("/relative"), None);
}

#[test]
fn path_rule_changes_category_only_for_urls() {
    let sdk = classify_url("https://connect.facebook.net/en_US/sdk.js").unwrap();
    assert_eq!((sdk.provider.id, sdk.category), ("meta", Category::Social));
    let pixel = classify_url("https://connect.facebook.net/en_US/fbevents.js").unwrap();
    assert_eq!(
        (pixel.provider.id, pixel.category),
        ("meta-pixel", Category::Advertising)
    );
    let beacon = classify_url("https://www.facebook.com/tr?id=1&ev=PageView").unwrap();
    assert_eq!(beacon.category, Category::Advertising);
    // Ohne Pfad kennt der Host nur seine Grundkategorie.
    assert_eq!(
        classify_host("connect.facebook.net").unwrap().category,
        Category::Social
    );
}

#[test]
fn shared_host_is_only_claimed_by_its_path_rule() {
    assert!(classify_url("https://www.google.com/search?q=x").is_none());
    assert_eq!(
        classify_url("https://www.google.com/pagead/1p-user-list/1")
            .unwrap()
            .category,
        Category::Advertising
    );
    assert_eq!(
        classify_url("https://www.google.com/maps/embed?pb=1")
            .unwrap()
            .category,
        Category::Maps
    );
}

#[test]
fn meta_pixel_cookies_are_advertising_not_social() {
    for name in ["_fbp", "_fbc"] {
        let m = classify_storage_name(name).unwrap();
        assert_eq!(
            (m.provider.id, m.category),
            ("meta-pixel", Category::Advertising),
            "{name}"
        );
    }
}

#[test]
fn storage_names() {
    assert_eq!(id(classify_storage_name("_ga")), Some("google-analytics"));
    assert_eq!(
        id(classify_storage_name("_ga_ABC123")),
        Some("google-analytics")
    );
    assert_eq!(id(classify_storage_name("_gcl_au")), Some("google-ads"));
    assert_eq!(
        id(classify_storage_name("_hjSessionUser_123")),
        Some("hotjar")
    );
    assert_eq!(
        id(classify_storage_name("CookieConsent")),
        Some("cookiebot")
    );
    assert_eq!(id(classify_storage_name("euconsent-v2")), Some("iab-tcf"));
    // Groß-/Kleinschreibung zählt, Präfix ist kein Teiltreffer.
    assert!(classify_storage_name("cookieconsent").is_none());
    assert!(classify_storage_name("my_ga").is_none());
    assert!(classify_storage_name("session").is_none());
}

#[test]
fn consent_platforms_are_recognisable() {
    assert_eq!(
        classify_host("consent.cookiebot.com").unwrap().category,
        Category::ConsentPlatform
    );
    assert_eq!(
        classify_host("cdn.cookielaw.org").unwrap().category,
        Category::ConsentPlatform
    );
    assert_eq!(
        classify_storage_name("borlabs-cookie").unwrap().category,
        Category::ConsentPlatform
    );
}

// ── Datenpflege ──────────────────────────────────────────────────────────

#[test]
fn every_entry_has_a_https_source_and_a_unique_id() {
    let mut ids = HashSet::new();
    for p in PROVIDERS {
        assert!(p.source.starts_with("https://"), "{}: Quelle fehlt", p.id);
        assert!(ids.insert(p.id), "{}: doppelte Kennung", p.id);
        assert!(
            !p.domains.is_empty() || !p.storage.is_empty(),
            "{}: weder Domain noch Name",
            p.id
        );
    }
}

#[test]
fn domains_are_normalised_and_not_claimed_twice_without_path_rule() {
    let mut plain = HashSet::new();
    for p in PROVIDERS {
        for dom in p.domains {
            assert_eq!(dom.host, dom.host.to_ascii_lowercase(), "{}", dom.host);
            assert!(
                !dom.host.starts_with('.') && !dom.host.contains('/'),
                "{}",
                dom.host
            );
            if dom.path_contains.is_none() {
                assert!(
                    plain.insert(dom.host),
                    "{} ohne Pfadregel doppelt vergeben",
                    dom.host
                );
            } else {
                assert!(
                    dom.category.is_some(),
                    "{}: Pfadregel ohne Kategorie",
                    dom.host
                );
            }
        }
    }
}

#[test]
fn storage_names_are_not_claimed_twice() {
    let mut seen = HashSet::new();
    for p in PROVIDERS {
        for s in p.storage {
            let key = match s {
                StorageName::Exact(n) => format!("={n}"),
                StorageName::Prefix(n) => format!("^{n}"),
            };
            assert!(seen.insert(key.clone()), "{key} doppelt vergeben");
        }
    }
}
