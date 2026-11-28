//! robots.txt: Grammatik, Bot-Einordnung, Pfadauswertung.
//!
//! Reine Berechnung über den Text einer robots.txt. Wie der Text hereinkommt —
//! über HTTP geholt oder aus einem gebauten `dist/` gelesen — bleibt Sache des
//! Hosts. Dieses Modul formuliert auch keine Befunde: es liefert die Daten, aus
//! denen ein Host seine Regeln bildet, weil die Befundkennungen zwischen den
//! Werkzeugen heute verschieden lauten.

use serde::{Deserialize, Serialize};

/// Eine geparste robots.txt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotsTxt {
    /// Regelgruppen, eine je `User-agent`-Block.
    pub groups: Vec<Group>,
    /// `Sitemap:`-Angaben, in der Reihenfolge des Vorkommens.
    pub sitemaps: Vec<String>,
    /// `Content-Signal:`-Direktiven (IETF AI Preferences / Cloudflare), in der Reihenfolge des Vorkommens.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_signals: Vec<String>,
}

/// Eine Regelgruppe: ein `User-agent` mit seinen Regeln.
///
/// Mehrere `User-agent`-Zeilen vor demselben Regelblock ergeben mehrere
/// Gruppen mit denselben Regeln — so bleibt jede Gruppe für sich auswertbar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub user_agent: String,
    pub bot_class: BotClass,
    pub allows: Vec<String>,
    pub disallows: Vec<String>,
    pub crawl_delay: Option<u32>,
}

/// Umfang einer Sperre für eine Regelgruppe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", content = "paths", rename_all = "snake_case")]
pub enum BlockScope {
    /// Keine wirksame Sperre (`Disallow` leer oder durch `Allow: /` aufgehoben).
    None,
    /// Teilsperre bestimmter Pfade oder Vollsperre mit `Allow`-Ausnahmen.
    Partial(Vec<String>),
    /// Vollständige Sperre (`Disallow: /` ohne aufhebendes `Allow`).
    Full,
}

impl Group {
    /// `Disallow: /` — die Gruppe sperrt alles.
    pub fn disallows_all(&self) -> bool {
        self.disallows.iter().any(|d| d == "/")
    }

    /// Ist dieser Pfad für diese Gruppe gesperrt?
    pub fn disallows_path(&self, path: &str) -> bool {
        path_is_disallowed(&self.disallows, &self.allows, path)
    }

    /// Sperrumfang dieser Gruppe unter Berücksichtigung von `Allow`-Ausnahmen
    /// und RFC-9309-Spezifität.
    pub fn block_scope(&self) -> BlockScope {
        block_scope(self)
    }
}

/// Berechnet den Sperrumfang einer Regelgruppe (`None`, `Partial(paths)`, `Full`).
pub fn block_scope(group: &Group) -> BlockScope {
    let active_disallows: Vec<String> = group
        .disallows
        .iter()
        .filter(|d| !d.is_empty())
        .cloned()
        .collect();
    if active_disallows.is_empty() {
        return BlockScope::None;
    }

    let root_blocked = path_is_disallowed(&active_disallows, &group.allows, "/");
    if root_blocked {
        // Gibt es spezifische Allow-Ausnahmen unterhalb von `/`?
        let has_subpath_allow = group
            .allows
            .iter()
            .any(|a| !a.is_empty() && a != "/");
        if has_subpath_allow {
            return BlockScope::Partial(active_disallows);
        }
        return BlockScope::Full;
    }

    // Root selbst ist nicht gesperrt (entweder nur Teilpfade oder `Allow: /` hebt `Disallow: /` auf).
    let non_root: Vec<String> = active_disallows
        .into_iter()
        .filter(|d| d != "/")
        .collect();
    if non_root.is_empty() {
        BlockScope::None
    } else {
        BlockScope::Partial(non_root)
    }
}

/// Die Art eines Bots. **Daten, keine Wertung** — ob eine Sperre sinnvoll ist,
/// entscheidet der Host, und die Beschriftung formuliert er auch selbst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotClass {
    /// `*`
    Wildcard,
    /// Suchmaschine (Googlebot, Bingbot, Applebot, …)
    SearchEngine,
    /// Sammelt Trainingsdaten (GPTBot, ClaudeBot, CCBot, …)
    AiTraining,
    /// KI-Suchindexierung für Antworten mit Quellenangabe (OAI-SearchBot, PerplexityBot, Claude-SearchBot, …)
    AiCitation,
    /// Vom Nutzer ausgelöster Live-Abruf durch Assistenten (ChatGPT-User, Claude-User, Perplexity-User, …)
    UserFetch,
    /// Beides: Trainingsdaten und Antworten/Grounding (Google-Extended, meta-externalagent, …)
    AiMixed,
    /// Web-Archiv (ia_archiver, archive.org_bot)
    Archive,
    /// Unverifizierter Scraper oder SEO-Bot
    UnknownAi,
    /// Bekannter Bot ohne KI-Bezug (Social-Preview etc.)
    General,
    /// Nicht zuzuordnen
    Unknown,
}

const SEARCH_BOTS: &[&str] = &[
    "googlebot",
    "googleother",
    "bingbot",
    "applebot",
    "slurp",
    "duckduckbot",
    "baiduspider",
    "yandexbot",
    "naverbot",
    "seznambot",
    "qwantify",
    "startpage",
];

/// Sammelt Trainingsdaten. Eine Sperre ist verbreitet und unauffällig.
const AI_TRAINING_BOTS: &[&str] = &[
    "gptbot",             // OpenAI — Training. Nicht die Suche, die heißt oai-searchbot.
    "claudebot",          // Anthropic — Training (getrennt von claude-searchbot / claude-user)
    "applebot-extended",  // Apple — Training
    "bytespider",         // ByteDance — Training
    "tiktokspider",       // ByteDance — Training
    "ccbot",              // Common Crawl — Training
    "omgili",             // Webz.io
    "mistralai-training", // Mistral AI — Training
    "grokbot",            // xAI — Training
    "deepseekbot",        // DeepSeek — Training
    "kimibot",            // Moonshot AI — Training
    "qwenbot",            // Alibaba — Training
    "hunyuan",            // Tencent — Training
    "erniebot",           // Baidu — Training
    "pangubot",           // Huawei — Training
    "cohere-training-data-crawler", // Cohere — Training
    "yandexadditional",   // Yandex — Training
    "ai2bot",             // Allen Institute for AI — Training
    "icc-crawler",        // NICT — Training
];

/// KI-Suchindexierung für Antworten mit Quellenangabe. Eine Sperre kostet KI-Suchsichtbarkeit.
const AI_CITATION_BOTS: &[&str] = &[
    "perplexitybot",    // Perplexity — Suchindex
    "youbot",           // You.com — Suche
    "amazonbot",        // Amazon — Suche/Alexa
    "oai-searchbot",    // OpenAI — Suche, getrennt vom trainierenden gptbot
    "claude-searchbot", // Anthropic — Suche
    "mistralai-index",  // Mistral AI — Suche
    "kimi-searchbot",   // Moonshot AI — Suche
    "petalbot",         // Huawei — Suche/KI
    "meta-webindexer",  // Meta — Suche
];

/// Vom Nutzer ausgelöster Live-Abruf durch KI-Assistenten/Agenten (kein Modelltraining).
const USER_FETCH_BOTS: &[&str] = &[
    "chatgpt-user",          // OpenAI — Nutzerabruf
    "claude-user",           // Anthropic — Nutzerabruf
    "claude-web",            // Anthropic — älterer Name desselben Zwecks
    "perplexity-user",       // Perplexity — Nutzerabruf
    "duckassistbot",         // DuckDuckGo — KI-Antworten
    "mistralai-user",        // Mistral AI — Nutzerabruf
    "kimi-user",             // Moonshot AI — Nutzerabruf
    "tongyibot",             // Alibaba — Nutzerabruf
    "cohere-ai",             // Cohere — Nutzerabruf
    "doubaobot",             // ByteDance — Nutzerabruf
    "meta-externalfetcher",  // Meta — Nutzerabruf
    "kagi-fetcher",          // Kagi — Nutzerabruf
    "google-cloudvertexbot", // Google — Vertex AI Grounding
    "bedrockbot",            // Amazon — Bedrock Live-Abruf
];

/// Beides: Trainingsdaten und Antworten/Grounding.
const AI_MIXED_BOTS: &[&str] = &[
    "google-extended", // Google — Gemini-Training UND Gemini-App/Vertex-Grounding (nicht Google Search / AI Overviews)
    "anthropic-ai",
    "xai-grok",
    "meta-externalagent",
    "facebookbot",
    "diffbot",
];

/// Web-Archive.
const ARCHIVE_BOTS: &[&str] = &[
    "archive.org_bot", // Internet Archive
    "ia_archiver",     // Internet Archive
];

/// Unverifizierte Scraper und SEO-Bots.
const UNKNOWN_AI_BOTS: &[&str] = &[
    "dotbot",
    "wpbot",
    "semrushbot",
    "ahrefsbot",
    "mj12bot",
    "rogerbot",
    "webzio-extended",
    "imagesiftbot",
    "timpibot",
    "firecrawlagent",
    "crawlspace",
    "velenpublicwebcrawler",
];

const GENERAL_BOTS: &[&str] = &["linkedinbot", "twitterbot"];

/// Ordnet einen `User-agent` ein.
///
/// Die längere Kennung gewinnt: `claude-searchbot` wird als Citation erkannt,
/// obwohl `claudebot` als Teilzeichenkette nicht enthalten ist — aber
/// `chatgpt-user` darf nicht an einem allgemeineren Muster hängenbleiben.
/// Deshalb wird über alle Register nach der **längsten Übereinstimmung**
/// gesucht, nicht nach der ersten.
pub fn classify_bot(user_agent: &str) -> BotClass {
    if user_agent.trim() == "*" {
        return BotClass::Wildcard;
    }
    let ua = user_agent.to_lowercase();

    let register = [
        (SEARCH_BOTS, BotClass::SearchEngine),
        (AI_TRAINING_BOTS, BotClass::AiTraining),
        (AI_CITATION_BOTS, BotClass::AiCitation),
        (USER_FETCH_BOTS, BotClass::UserFetch),
        (AI_MIXED_BOTS, BotClass::AiMixed),
        (ARCHIVE_BOTS, BotClass::Archive),
        (UNKNOWN_AI_BOTS, BotClass::UnknownAi),
        (GENERAL_BOTS, BotClass::General),
    ];

    let mut best: Option<(usize, BotClass)> = None;
    for (patterns, class) in register {
        for pat in patterns {
            if ua.contains(pat) {
                let laenge = pat.len();
                if best.is_none_or(|(bisher, _)| laenge > bisher) {
                    best = Some((laenge, class));
                }
            }
        }
    }
    best.map_or(BotClass::Unknown, |(_, class)| class)
}

/// Parst eine robots.txt.
///
/// Unbekannte Direktiven werden übergangen, `Crawl-delay` gilt für die Agenten
/// des laufenden Blocks. Eine Leerzeile beendet einen Block.
pub fn parse(text: &str) -> RobotsTxt {
    let mut robots = RobotsTxt::default();
    let mut agents: Vec<String> = Vec::new();
    let mut allows: Vec<String> = Vec::new();
    let mut disallows: Vec<String> = Vec::new();
    let mut delay: Option<u32> = None;
    let mut saw_rule = false;

    let abschliessen = |agents: &mut Vec<String>,
                        allows: &mut Vec<String>,
                        disallows: &mut Vec<String>,
                        delay: &mut Option<u32>,
                        saw_rule: &mut bool,
                        groups: &mut Vec<Group>| {
        for agent in agents.drain(..) {
            groups.push(Group {
                bot_class: classify_bot(&agent),
                user_agent: agent,
                allows: allows.clone(),
                disallows: disallows.clone(),
                crawl_delay: *delay,
            });
        }
        allows.clear();
        disallows.clear();
        *delay = None;
        *saw_rule = false;
    };

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if !agents.is_empty() {
                abschliessen(
                    &mut agents,
                    &mut allows,
                    &mut disallows,
                    &mut delay,
                    &mut saw_rule,
                    &mut robots.groups,
                );
            }
            continue;
        }
        // Reine Kommentarzeilen beenden keinen Block (RFC 9309).
        if trimmed.starts_with('#') {
            continue;
        }
        // Inline-Kommentare abschneiden.
        let line = trimmed.split('#').next().unwrap_or("").trim();

        let Some(trenner) = line.find(':') else {
            continue;
        };
        let key = line[..trenner].trim().to_lowercase();
        let value = line[trenner + 1..].trim().to_string();

        match key.as_str() {
            "user-agent" => {
                // Ein neuer Agent nach Regeln beginnt einen neuen Block.
                if saw_rule {
                    abschliessen(
                        &mut agents,
                        &mut allows,
                        &mut disallows,
                        &mut delay,
                        &mut saw_rule,
                        &mut robots.groups,
                    );
                }
                if !value.is_empty() {
                    agents.push(value);
                }
            }
            "allow" => {
                saw_rule = true;
                if !value.is_empty() {
                    allows.push(value);
                }
            }
            "disallow" => {
                saw_rule = true;
                if !value.is_empty() {
                    disallows.push(value);
                }
            }
            "sitemap" if !value.is_empty() => robots.sitemaps.push(value),
            "content-signal" if !value.is_empty() => robots.content_signals.push(value),
            "crawl-delay" => {
                saw_rule = true;
                delay = value.parse().ok();
            }
            _ => {}
        }
    }
    if !agents.is_empty() {
        abschliessen(
            &mut agents,
            &mut allows,
            &mut disallows,
            &mut delay,
            &mut saw_rule,
            &mut robots.groups,
        );
    }
    robots
}

impl RobotsTxt {
    /// Die Gruppe für `*`, falls vorhanden.
    pub fn wildcard(&self) -> Option<&Group> {
        self.groups
            .iter()
            .find(|g| g.bot_class == BotClass::Wildcard)
    }

    /// Sperrt `User-agent: *` die ganze Seite?
    pub fn wildcard_disallows_all(&self) -> bool {
        self.wildcard().is_some_and(Group::disallows_all)
    }

    /// Ist der Pfad für einen Bot ohne eigene Gruppe gesperrt?
    ///
    /// Es kann mehrere `*`-Gruppen geben; ihre Regeln gelten zusammen. Genau
    /// das brauchten beide Hosts, und beide hatten es sich vorher selbst
    /// zusammengebaut — astro-post-audit als `wildcard_rules`, auditmysite über
    /// die Gruppenliste.
    pub fn wildcard_disallows_path(&self, path: &str) -> bool {
        let mut allows: Vec<String> = Vec::new();
        let mut disallows: Vec<String> = Vec::new();
        for gruppe in self
            .groups
            .iter()
            .filter(|g| g.bot_class == BotClass::Wildcard)
        {
            allows.extend(gruppe.allows.iter().cloned());
            disallows.extend(gruppe.disallows.iter().cloned());
        }
        path_is_disallowed(&disallows, &allows, path)
    }

    /// Wird mindestens ein Bot dieser Art gesperrt?
    pub fn blocks(&self, class: BotClass) -> bool {
        self.groups
            .iter()
            .any(|g| g.bot_class == class && g.disallows.iter().any(|d| !d.is_empty()))
    }

    /// Ermittelt nach RFC 9309 die tatsächlich geltende Regelgruppe für `user_agent`:
    /// die spezifischste passende Gruppe (längster Präfix-Treffer, mehrere Gruppen
    /// desselben Agenten zusammengeführt), sonst Fallback auf `*`.
    pub fn effective_group(&self, user_agent: &str) -> Option<Group> {
        let ua_lower = user_agent.trim().to_ascii_lowercase();
        let mut best_len: usize = 0;
        let mut matched: Vec<&Group> = Vec::new();

        for g in &self.groups {
            if g.bot_class == BotClass::Wildcard {
                continue;
            }
            let pat = g.user_agent.trim().to_ascii_lowercase();
            if pat.is_empty() {
                continue;
            }
            if ua_lower.starts_with(&pat) {
                if pat.len() > best_len {
                    best_len = pat.len();
                    matched.clear();
                    matched.push(g);
                } else if pat.len() == best_len {
                    matched.push(g);
                }
            }
        }

        if matched.is_empty() {
            matched = self
                .groups
                .iter()
                .filter(|g| g.bot_class == BotClass::Wildcard)
                .collect();
        }
        if matched.is_empty() {
            return None;
        }

        let mut allows = Vec::new();
        let mut disallows = Vec::new();
        let mut crawl_delay = None;
        for g in &matched {
            allows.extend(g.allows.iter().cloned());
            disallows.extend(g.disallows.iter().cloned());
            if crawl_delay.is_none() {
                crawl_delay = g.crawl_delay;
            }
        }

        Some(Group {
            user_agent: matched[0].user_agent.clone(),
            bot_class: matched[0].bot_class,
            allows,
            disallows,
            crawl_delay,
        })
    }

    /// `(User-agent, Sekunden)` aller `Crawl-delay`-Angaben.
    pub fn crawl_delays(&self) -> Vec<(String, u32)> {
        self.groups
            .iter()
            .filter_map(|g| g.crawl_delay.map(|d| (g.user_agent.clone(), d)))
            .collect()
    }
}

/// Ist der Pfad gesperrt? Die längere Regel gewinnt, wie in der Spezifikation.
///
/// Bei gleicher Länge gewinnt `Allow` — das entspricht Googles Auslegung.
pub fn path_is_disallowed(disallows: &[String], allows: &[String], path: &str) -> bool {
    let staerkste_sperre = disallows
        .iter()
        .filter_map(|r| rule_match_len(r, path))
        .max();
    let staerkste_erlaubnis = allows.iter().filter_map(|r| rule_match_len(r, path)).max();
    match (staerkste_sperre, staerkste_erlaubnis) {
        (Some(sperre), Some(erlaubnis)) => sperre > erlaubnis,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Passt das Muster auf den Pfad? Dann seine Trennschärfe: die Zahl der
/// festen Zeichen. `*` und `$` werden unterstützt; ein leeres Muster
/// (`Disallow:`) passt auf nichts und erlaubt damit alles.
fn rule_match_len(pattern: &str, path: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    let verankert = pattern.ends_with('$');
    let pat = if verankert {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };

    let mut pos = 0usize;
    for (i, teil) in pat.split('*').enumerate() {
        if teil.is_empty() {
            continue;
        }
        if i == 0 {
            if !path[pos..].starts_with(teil) {
                return None;
            }
            pos += teil.len();
        } else {
            let idx = path[pos..].find(teil)?;
            pos += idx + teil.len();
        }
    }

    // Mit `$` muss der Pfad ganz aufgebraucht sein — außer das Muster endet
    // auf `*`, dann darf ein Rest folgen.
    if verankert && !pat.ends_with('*') && pos != path.len() {
        return None;
    }

    Some(pat.chars().filter(|&c| c != '*').count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gruppen_sitemaps_und_verzoegerung() {
        let robots = parse(
            "User-agent: *\nDisallow: /admin/\nCrawl-delay: 10\n\nSitemap: https://example.com/sitemap.xml\n",
        );
        assert_eq!(robots.groups.len(), 1);
        assert_eq!(robots.groups[0].user_agent, "*");
        assert_eq!(robots.groups[0].bot_class, BotClass::Wildcard);
        assert_eq!(robots.groups[0].disallows, ["/admin/"]);
        assert_eq!(robots.groups[0].crawl_delay, Some(10));
        assert_eq!(robots.sitemaps, ["https://example.com/sitemap.xml"]);
        assert_eq!(robots.crawl_delays(), [("*".to_string(), 10)]);
    }

    #[test]
    fn mehrere_agenten_teilen_einen_regelblock() {
        let robots = parse("User-agent: GPTBot\nUser-agent: CCBot\nDisallow: /\n");
        assert_eq!(robots.groups.len(), 2);
        assert!(robots.groups.iter().all(Group::disallows_all));
        assert!(robots.blocks(BotClass::AiTraining));
    }

    #[test]
    fn kommentare_und_unbekannte_direktiven_stoeren_nicht() {
        let robots = parse("# Kommentar\nUser-agent: *\nHost: example.com\nDisallow:\n");
        assert_eq!(robots.groups.len(), 1);
        assert!(!robots.wildcard_disallows_all());
    }

    /// Der Widerspruch, der beim Zusammenlegen auffiel: GPTBot trainiert,
    /// die Suche von OpenAI heißt oai-searchbot. Ein Werkzeug hatte GPTBot
    /// als Citation-Bot geführt und deshalb das Gegenteil gemeldet.
    #[test]
    fn openai_trainiert_und_sucht_unter_verschiedenen_namen() {
        assert_eq!(classify_bot("GPTBot"), BotClass::AiTraining);
        assert_eq!(classify_bot("OAI-SearchBot"), BotClass::AiCitation);
        assert_eq!(classify_bot("ChatGPT-User"), BotClass::UserFetch);
    }

    #[test]
    fn anthropic_ebenso() {
        assert_eq!(classify_bot("ClaudeBot"), BotClass::AiTraining);
        assert_eq!(classify_bot("Claude-SearchBot"), BotClass::AiCitation);
        assert_eq!(classify_bot("Claude-User"), BotClass::UserFetch);
    }

    #[test]
    fn google_extended_und_neue_bots() {
        assert_eq!(classify_bot("Google-Extended"), BotClass::AiMixed);
        assert_eq!(classify_bot("Applebot"), BotClass::SearchEngine);
        assert_eq!(classify_bot("Applebot-Extended"), BotClass::AiTraining);
        assert_eq!(classify_bot("Perplexity-User"), BotClass::UserFetch);
        assert_eq!(classify_bot("DuckAssistBot"), BotClass::UserFetch);
        assert_eq!(classify_bot("MistralAI-Training"), BotClass::AiTraining);
        assert_eq!(classify_bot("MistralAI-User"), BotClass::UserFetch);
        assert_eq!(classify_bot("ia_archiver"), BotClass::Archive);
        assert_eq!(classify_bot("archive.org_bot"), BotClass::Archive);
    }

    #[test]
    fn block_scope_unterscheidet_voll_teilweise_und_keine() {
        let robots = parse(
            "User-agent: GPTBot\nDisallow: /\n\n\
             User-agent: OAI-SearchBot\nDisallow: /admin/\n\n\
             User-agent: ClaudeBot\nDisallow: /\nAllow: /public/\n\n\
             User-agent: Googlebot\nAllow: /\n",
        );
        assert_eq!(
            robots.effective_group("GPTBot").unwrap().block_scope(),
            BlockScope::Full
        );
        assert_eq!(
            robots.effective_group("OAI-SearchBot").unwrap().block_scope(),
            BlockScope::Partial(vec!["/admin/".to_string()])
        );
        assert_eq!(
            robots.effective_group("ClaudeBot").unwrap().block_scope(),
            BlockScope::Partial(vec!["/".to_string()])
        );
        assert_eq!(
            robots.effective_group("Googlebot").unwrap().block_scope(),
            BlockScope::None
        );
    }

    #[test]
    fn effective_group_bevorzugt_eigene_gruppe_vor_wildcard() {
        let robots = parse(
            "User-agent: *\nDisallow: /\n\n\
             User-agent: OAI-SearchBot\nAllow: /\n",
        );
        assert_eq!(
            robots.effective_group("OAI-SearchBot").unwrap().block_scope(),
            BlockScope::None
        );
        assert_eq!(
            robots.effective_group("PerplexityBot").unwrap().block_scope(),
            BlockScope::Full
        );
    }

    #[test]
    fn kommentare_im_block_und_inline_kommentare() {
        let robots = parse(
            "User-agent: *\n# Admin-Bereich sperren\nDisallow: /admin/ # intern\nContent-Signal: ai-train=no, search=yes, ai-input=yes\n",
        );
        assert_eq!(robots.groups.len(), 1);
        assert_eq!(robots.groups[0].disallows, ["/admin/"]);
        assert_eq!(
            robots.content_signals,
            ["ai-train=no, search=yes, ai-input=yes"]
        );
    }

    #[test]
    fn leeres_disallow_zaehlt_nicht_als_sperre() {
        let robots = parse("User-agent: GPTBot\nDisallow:\n");
        assert!(!robots.blocks(BotClass::AiTraining));
    }

    #[test]
    fn laengste_uebereinstimmung_gewinnt() {
        // "claude-searchbot" enthaelt "claude-web" nicht, aber der Test haelt
        // fest, dass die Auswahl nicht von der Registerreihenfolge abhaengt.
        assert_eq!(classify_bot("Googlebot-Image"), BotClass::SearchEngine);
        assert_eq!(classify_bot("Mozilla/5.0 kein Bot"), BotClass::Unknown);
    }

    #[test]
    fn laengere_regel_schlaegt_kuerzere() {
        let disallows = vec!["/blog/".to_string()];
        let allows = vec!["/blog/oeffentlich/".to_string()];
        assert!(path_is_disallowed(&disallows, &allows, "/blog/intern/x"));
        assert!(!path_is_disallowed(
            &disallows,
            &allows,
            "/blog/oeffentlich/x"
        ));
    }

    #[test]
    fn platzhalter_und_endanker() {
        let d = vec!["/*.pdf$".to_string()];
        let a: Vec<String> = Vec::new();
        assert!(path_is_disallowed(&d, &a, "/kram/datei.pdf"));
        assert!(!path_is_disallowed(&d, &a, "/kram/datei.pdf.html"));
    }

    #[test]
    fn mehrere_wildcard_gruppen_gelten_zusammen() {
        let robots = parse(
            "User-agent: *\nDisallow: /intern/\n\nUser-agent: *\nDisallow: /tmp/\nAllow: /tmp/oeffentlich/\n",
        );
        assert!(robots.wildcard_disallows_path("/intern/seite"));
        assert!(robots.wildcard_disallows_path("/tmp/x"));
        assert!(!robots.wildcard_disallows_path("/tmp/oeffentlich/x"));
        assert!(!robots.wildcard_disallows_path("/start"));
    }

    #[test]
    fn ohne_wildcard_gruppe_ist_nichts_gesperrt() {
        let robots = parse("User-agent: GPTBot\nDisallow: /\n");
        assert!(!robots.wildcard_disallows_path("/start"));
    }

    #[test]
    fn leere_disallow_zeile_erlaubt_alles() {
        let d = vec![String::new()];
        let a: Vec<String> = Vec::new();
        assert!(!path_is_disallowed(&d, &a, "/irgendwas"));
    }
}
