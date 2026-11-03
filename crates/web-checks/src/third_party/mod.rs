//! Drittanbieter einordnen: Host oder URL → Anbieter, Kategorie, Sitz; Cookie-
//! oder Storage-Name → Anbieter.
//!
//! Das Crate liefert Daten, kein Urteil. Ob ein Anbieter vor einer
//! Einwilligung geladen werden darf, ist eine Rechtsfrage und bleibt beim
//! Host. Hier steht nur: *wer* ist das, *wozu* dient er, *wo* sitzt er —
//! jeweils mit Quelle. Ein unbekannter Host liefert `None`, nie eine
//! Vermutung; „unbekannt" ist kein Befund.
//!
//! # Zuordnung
//!
//! Hosts passen auf die Domain selbst und auf jede Subdomain, getrennt an der
//! Punktgrenze: `www.google-analytics.com` trifft, `google-analytics.com.evil.example`
//! und `notgoogle-analytics.com` nicht. Eine Public-Suffix-Liste ist nicht
//! nötig, weil die Domains der Liste vollständige Registrierungsdomains sind.
//!
//! Manche Hosts bedienen mehrere Zwecke (`facebook.net` liefert das
//! Like-Plugin und den Werbe-Pixel). Dort entscheidet [`classify_url`] über
//! einen Pfadteil; [`classify_host`] kennt nur die Kategorie des Hosts.

mod data;

pub use data::{PROVIDERS, RULESET_VERSION};

/// Wozu ein Anbieter dient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Analytics,
    Advertising,
    Social,
    TagManager,
    Fonts,
    Cdn,
    VideoEmbed,
    Maps,
    ConsentPlatform,
}

impl Category {
    /// Stabile Kennung für JSON und Tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analytics => "analytics",
            Self::Advertising => "advertising",
            Self::Social => "social",
            Self::TagManager => "tag_manager",
            Self::Fonts => "fonts",
            Self::Cdn => "cdn",
            Self::VideoEmbed => "video_embed",
            Self::Maps => "maps",
            Self::ConsentPlatform => "consent_platform",
        }
    }
}

/// Sitz des Anbieters laut dessen eigener Angabe. Sagt nichts über den
/// tatsächlichen Datenfluss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jurisdiction {
    Eu,
    Us,
    Other,
    /// Nicht belegt oder je nach Betrieb verschieden (selbst gehostete Software).
    Unknown,
}

impl Jurisdiction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eu => "eu",
            Self::Us => "us",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }
}

/// Eine Domain eines Anbieters, optional mit Pfadbedingung.
#[derive(Debug)]
pub struct Domain {
    /// Registrierungsdomain, klein geschrieben, ohne führenden Punkt.
    pub host: &'static str,
    /// Gilt nur, wenn Pfad samt Query diesen Text enthält (nur bei [`classify_url`]).
    pub path_contains: Option<&'static str>,
    /// Überschreibt die Kategorie des Anbieters für diese Domain.
    pub category: Option<Category>,
}

/// Name eines Cookies oder Storage-Eintrags. Groß-/Kleinschreibung zählt.
#[derive(Debug)]
pub enum StorageName {
    Exact(&'static str),
    Prefix(&'static str),
}

impl StorageName {
    fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exact(n) => name == *n,
            Self::Prefix(p) => name.starts_with(p),
        }
    }
}

#[derive(Debug)]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub jurisdiction: Jurisdiction,
    pub domains: &'static [Domain],
    pub storage: &'static [StorageName],
    /// Beleg für Zuordnung und Cookie-Namen (Dokumentation des Anbieters).
    pub source: &'static str,
}

/// Treffer einer Einordnung.
#[derive(Debug, Clone, Copy)]
pub struct Match {
    pub provider: &'static Provider,
    /// Kategorie des Treffers; bei Pfadregeln kann sie von `provider.category` abweichen.
    pub category: Category,
}

/// Host einer absoluten oder schemarelativen URL (`https://…`, `//…`), klein
/// geschrieben, ohne Port, Zugangsdaten und abschließenden Punkt. Alles andere
/// (relativ, `data:`, `blob:`) liefert `None`.
pub fn host_of(url: &str) -> Option<String> {
    split_url(url).map(|(host, _)| host)
}

/// Ordnet einen Host ein. Pfadregeln bleiben außen vor.
pub fn classify_host(host: &str) -> Option<Match> {
    classify(&normalize_host(host)?, None)
}

/// Ordnet eine URL ein, einschließlich der Pfadregeln.
pub fn classify_url(url: &str) -> Option<Match> {
    let (host, rest) = split_url(url)?;
    classify(&host, Some(rest))
}

/// Ordnet einen Cookie- oder Storage-Namen einem Anbieter zu. Der Name allein
/// trägt keine Domain; der Host prüft, ob der Eintrag zur Seite passt.
pub fn classify_storage_name(name: &str) -> Option<Match> {
    PROVIDERS
        .iter()
        .find(|p| p.storage.iter().any(|s| s.matches(name)))
        .map(|provider| Match {
            provider,
            category: provider.category,
        })
}

fn classify(host: &str, path_and_query: Option<&str>) -> Option<Match> {
    let mut best: Option<(usize, bool, Match)> = None;
    for provider in PROVIDERS {
        for d in provider.domains {
            if !host_in(host, d.host) {
                continue;
            }
            let path_rule = match (d.path_contains, path_and_query) {
                (None, _) => false,
                (Some(needle), Some(rest)) if rest.contains(needle) => true,
                _ => continue,
            };
            let m = Match {
                provider,
                category: d.category.unwrap_or(provider.category),
            };
            // Längere Domain vor kürzerer, bei Gleichstand die mit Pfadregel.
            let better = best
                .as_ref()
                .is_none_or(|(len, rule, _)| (d.host.len(), path_rule) > (*len, *rule));
            if better {
                best = Some((d.host.len(), path_rule, m));
            }
        }
    }
    best.map(|(_, _, m)| m)
}

fn host_in(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

/// Zerlegt eine URL in Host und Rest (Pfad plus Query).
fn split_url(url: &str) -> Option<(String, &str)> {
    let url = url.trim();
    let after_scheme = match url.find("://") {
        Some(i)
            if url[..i]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)) =>
        {
            &url[i + 3..]
        }
        _ => url.strip_prefix("//")?,
    };
    let end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let (authority, rest) = after_scheme.split_at(end);
    let rest = rest.split('#').next().unwrap_or("");
    // Zugangsdaten abtrennen: `https://google-analytics.com@evil.example/` hat den Host evil.example.
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    Some((normalize_host(host_port)?, rest))
}

fn normalize_host(host_port: &str) -> Option<String> {
    let host = if host_port.starts_with('[') {
        // IPv6-Literal: nie ein Anbieter, aber ein gültiger Host.
        host_port.split(']').next().map(|h| format!("{h}]"))?
    } else {
        host_port.split(':').next()?.to_string()
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

#[cfg(test)]
mod tests;
