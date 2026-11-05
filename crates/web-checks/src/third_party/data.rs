//! Die Anbieterliste. Jeder Eintrag nennt seine Quelle; ein Eintrag ohne Quelle
//! lässt `tests` scheitern.
//!
//! Aufnahme: nur Anbieter, deren Domains oder Cookie-Namen der Anbieter selbst
//! dokumentiert. Cookie-Namen, die sich nicht belegen ließen, stehen nicht
//! hier. Lücken sind besser als geratene Treffer.

use super::{Category as C, Domain, Jurisdiction as J, Provider, StorageName as S};

/// Stand der Liste (Datum der letzten Änderung). Hosts nennen ihn im Bericht.
pub const RULESET_VERSION: &str = "2026-11-04";

const fn d(host: &'static str) -> Domain {
    Domain {
        host,
        path_contains: None,
        category: None,
    }
}

const fn dp(host: &'static str, path_contains: &'static str, category: C) -> Domain {
    Domain {
        host,
        path_contains: Some(path_contains),
        category: Some(category),
    }
}

pub static PROVIDERS: &[Provider] = &[
    Provider {
        id: "google-analytics",
        name: "Google Analytics",
        category: C::Analytics,
        jurisdiction: J::Us,
        domains: &[d("google-analytics.com"), d("analytics.google.com")],
        storage: &[
            S::Exact("_ga"),
            S::Prefix("_ga_"),
            S::Exact("_gid"),
            S::Exact("_gat"),
            S::Prefix("_gat_"),
        ],
        source: "https://developers.google.com/analytics/devguides/collection/gtagjs/cookie-usage",
    },
    Provider {
        id: "google-tag-manager",
        name: "Google Tag Manager",
        category: C::TagManager,
        jurisdiction: J::Us,
        domains: &[d("googletagmanager.com")],
        storage: &[],
        source: "https://developers.google.com/tag-platform/tag-manager",
    },
    Provider {
        id: "google-ads",
        name: "Google Ads",
        category: C::Advertising,
        jurisdiction: J::Us,
        domains: &[
            d("doubleclick.net"),
            d("googleadservices.com"),
            d("googlesyndication.com"),
            dp("google.com", "/pagead/", C::Advertising),
        ],
        storage: &[S::Prefix("_gcl_"), S::Prefix("_gac_")],
        source: "https://policies.google.com/technologies/cookies",
    },
    Provider {
        id: "google-fonts",
        name: "Google Fonts",
        category: C::Fonts,
        jurisdiction: J::Us,
        domains: &[d("fonts.googleapis.com"), d("fonts.gstatic.com")],
        storage: &[],
        source: "https://developers.google.com/fonts/faq/privacy",
    },
    Provider {
        id: "google-maps",
        name: "Google Maps",
        category: C::Maps,
        jurisdiction: J::Us,
        domains: &[
            d("maps.googleapis.com"),
            d("maps.gstatic.com"),
            dp("google.com", "/maps/embed", C::Maps),
        ],
        storage: &[],
        source: "https://developers.google.com/maps/documentation/javascript/overview",
    },
    Provider {
        id: "google-hosted-libraries",
        name: "Google Hosted Libraries",
        category: C::Cdn,
        jurisdiction: J::Us,
        domains: &[d("ajax.googleapis.com")],
        storage: &[],
        source: "https://developers.google.com/speed/libraries",
    },
    Provider {
        id: "youtube",
        name: "YouTube",
        category: C::VideoEmbed,
        jurisdiction: J::Us,
        domains: &[d("youtube.com"), d("youtube-nocookie.com"), d("ytimg.com")],
        storage: &[],
        source: "https://support.google.com/youtube/answer/171780",
    },
    Provider {
        id: "vimeo",
        name: "Vimeo",
        category: C::VideoEmbed,
        jurisdiction: J::Us,
        domains: &[d("vimeo.com"), d("vimeocdn.com")],
        storage: &[S::Exact("vuid")],
        source: "https://vimeo.com/cookie_policy",
    },
    Provider {
        id: "meta",
        name: "Meta (Facebook)",
        category: C::Social,
        jurisdiction: J::Us,
        domains: &[d("facebook.com"), d("facebook.net")],
        storage: &[],
        source: "https://www.facebook.com/privacy/policies/cookies/",
    },
    // Der Werbe-Pixel hat dieselben Hosts wie das Plugin, aber einen anderen
    // Zweck. Pfadregeln trennen die Requests, die Cookie-Namen gehören hierher.
    Provider {
        id: "meta-pixel",
        name: "Meta Pixel",
        category: C::Advertising,
        jurisdiction: J::Us,
        domains: &[
            dp("facebook.net", "fbevents.js", C::Advertising),
            dp("facebook.com", "/tr?", C::Advertising),
            dp("facebook.com", "/tr/", C::Advertising),
        ],
        storage: &[S::Exact("_fbp"), S::Exact("_fbc")],
        source: "https://www.facebook.com/privacy/policies/cookies/",
    },
    Provider {
        id: "linkedin-insight",
        name: "LinkedIn Insight Tag",
        category: C::Advertising,
        jurisdiction: J::Us,
        domains: &[d("snap.licdn.com"), d("px.ads.linkedin.com")],
        storage: &[
            S::Exact("li_fat_id"),
            S::Exact("lidc"),
            S::Exact("bcookie"),
            S::Exact("UserMatchHistory"),
        ],
        source: "https://www.linkedin.com/legal/l/cookie-table",
    },
    Provider {
        id: "microsoft-clarity",
        name: "Microsoft Clarity",
        category: C::Analytics,
        jurisdiction: J::Us,
        domains: &[d("clarity.ms")],
        storage: &[S::Exact("_clck"), S::Exact("_clsk")],
        source: "https://learn.microsoft.com/en-us/clarity/setup-and-installation/cookie-consent",
    },
    Provider {
        id: "microsoft-ads",
        name: "Microsoft Advertising (UET)",
        category: C::Advertising,
        jurisdiction: J::Us,
        domains: &[d("bat.bing.com")],
        storage: &[S::Prefix("_uet")],
        source: "https://learn.microsoft.com/en-us/advertising/guides/universal-event-tracking",
    },
    Provider {
        id: "hotjar",
        name: "Hotjar",
        category: C::Analytics,
        jurisdiction: J::Eu,
        domains: &[d("hotjar.com"), d("hotjar.io")],
        storage: &[S::Prefix("_hj")],
        source: "https://help.hotjar.com/hc/en-us/articles/115011789248-Hotjar-Cookie-Information",
    },
    Provider {
        id: "tiktok-pixel",
        name: "TikTok Pixel",
        category: C::Advertising,
        jurisdiction: J::Other,
        domains: &[d("analytics.tiktok.com")],
        storage: &[S::Exact("_ttp"), S::Exact("_tt_enable_cookie")],
        source: "https://ads.tiktok.com/help/article/tiktok-pixel",
    },
    Provider {
        id: "matomo",
        name: "Matomo",
        category: C::Analytics,
        // Meist selbst gehostet: Der Sitz hängt vom Betreiber ab, Domains gibt es keine.
        jurisdiction: J::Unknown,
        domains: &[],
        storage: &[S::Prefix("_pk_")],
        source: "https://matomo.org/faq/general/faq_146/",
    },
    // Cookielos laut Anbieter; erkennbar nur am Host (Skript und Beacon über
    // cdn.usefathom.com). Eine eigene Domain des Betreibers (Custom Domain)
    // ist von außen nicht zu erkennen.
    Provider {
        id: "fathom",
        name: "Fathom Analytics",
        category: C::Analytics,
        jurisdiction: J::Other,
        domains: &[d("usefathom.com")],
        storage: &[],
        source: "https://usefathom.com/",
    },
    // Hosts laut Datenschutzerklärung des Anbieters; Cookie-Namen nennt er nicht.
    Provider {
        id: "carbon-ads",
        name: "Carbon Ads",
        category: C::Advertising,
        jurisdiction: J::Us,
        domains: &[d("carbonads.net"), d("syndicateads.net")],
        storage: &[],
        source: "https://www.carbonads.net/privacy",
    },
    // Die Track-Domain vergibt Mapp je Kunde (Beispiele der Doku: wt-eu02.net,
    // webtrekk.net). Eine per CNAME eingerichtete Domain des Betreibers bleibt
    // über den Host unerkannt; die Cookie-Namen (First-Party-Tracking) greifen dann.
    // Sitz nennt die Doku nicht.
    Provider {
        id: "mapp-intelligence",
        name: "Mapp Intelligence (Webtrekk)",
        category: C::Analytics,
        jurisdiction: J::Unknown,
        domains: &[d("wt-eu02.net"), d("webtrekk.net")],
        storage: &[
            S::Exact("wt3_eid"),
            S::Exact("wt3_sid"),
            S::Exact("wt3_sample"),
            S::Exact("wt_eid"),
            S::Exact("wt_sid"),
            S::Exact("wt_rla"),
            S::Exact("wt_geid"),
            S::Exact("wt_feid"),
            S::Exact("wt_fweid"),
            S::Exact("wt_cdbeid"),
            S::Prefix("wteid_"),
            S::Prefix("wtsid_"),
            S::Prefix("wtstp_"),
        ],
        source: "https://docs.mapp.com/docs/which-cookies-are-set-by-mapp-intelligence",
    },
    Provider {
        id: "jsdelivr",
        name: "jsDelivr",
        category: C::Cdn,
        jurisdiction: J::Unknown,
        domains: &[d("jsdelivr.net")],
        storage: &[],
        source: "https://www.jsdelivr.com/terms/privacy-policy-jsdelivr-net",
    },
    Provider {
        id: "cdnjs",
        name: "cdnjs (Cloudflare)",
        category: C::Cdn,
        jurisdiction: J::Us,
        domains: &[d("cdnjs.cloudflare.com")],
        storage: &[],
        source: "https://cdnjs.com/about",
    },
    Provider {
        id: "cookiebot",
        name: "Cookiebot (Usercentrics)",
        category: C::ConsentPlatform,
        jurisdiction: J::Eu,
        domains: &[d("cookiebot.com")],
        storage: &[S::Exact("CookieConsent"), S::Prefix("CookieConsentBulk")],
        source: "https://www.cookiebot.com/en/developer/",
    },
    Provider {
        id: "usercentrics",
        name: "Usercentrics",
        category: C::ConsentPlatform,
        jurisdiction: J::Eu,
        domains: &[d("usercentrics.eu")],
        storage: &[S::Exact("uc_settings"), S::Exact("uc_user_interaction")],
        source: "https://docs.usercentrics.com/",
    },
    Provider {
        id: "onetrust",
        name: "OneTrust",
        category: C::ConsentPlatform,
        jurisdiction: J::Us,
        domains: &[d("cookielaw.org"), d("onetrust.com")],
        storage: &[
            S::Exact("OptanonConsent"),
            S::Exact("OptanonAlertBoxClosed"),
        ],
        source: "https://developer.onetrust.com/",
    },
    Provider {
        id: "borlabs-cookie",
        name: "Borlabs Cookie",
        category: C::ConsentPlatform,
        jurisdiction: J::Eu,
        // WordPress-Plugin, läuft auf der Seite selbst: kein Fremd-Host.
        domains: &[],
        storage: &[S::Exact("borlabs-cookie")],
        source: "https://borlabs.io/",
    },
    Provider {
        id: "didomi",
        name: "Didomi",
        category: C::ConsentPlatform,
        jurisdiction: J::Eu,
        domains: &[d("privacy-center.org"), d("didomi.io")],
        storage: &[S::Exact("didomi_token")],
        source: "https://developers.didomi.io/",
    },
    Provider {
        id: "iab-tcf",
        name: "IAB Transparency & Consent Framework",
        category: C::ConsentPlatform,
        jurisdiction: J::Unknown,
        domains: &[],
        storage: &[S::Exact("euconsent-v2")],
        source: "https://github.com/InteractiveAdvertisingBureau/GDPR-Transparency-and-Consent-Framework/blob/master/TCFv2/IAB%20Tech%20Lab%20-%20CMP%20API%20v2.md",
    },
];
