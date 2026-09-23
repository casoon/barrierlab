//! Ein einzelnes Prüfergebnis.

use serde::{Deserialize, Serialize};

use crate::extra::Extra;
use crate::outcome::{Outcome, Severity, WcagLevel};

/// Wo ein Befund sitzt. Die drei Oberflächen verorten unterschiedlich:
/// astro-post-audit über Dateipfade, auditmysite über AXTree-Knoten, LiveAudit
/// über den Index in seiner Arena. Alle Felder sind deshalb optional — welche
/// gefüllt sind, hängt vom Host ab, nicht von der Regel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// Datei relativ zur Ausgabewurzel, z. B. `about/index.html`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// URL der geprüften Seite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// CSS-Selektor zum Wiederfinden des Elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    /// Knotenkennung des Hosts — AXTree-Backend-ID oder Arena-Index als Text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Hinweis auf die Quelldatei, wenn der Host sie zurückrechnen kann.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hint: Option<String>,
}

impl Location {
    pub fn file(path: impl Into<String>) -> Self {
        Location {
            file: Some(path.into()),
            ..Default::default()
        }
    }

    pub fn node(id: impl Into<String>) -> Self {
        Location {
            node: Some(id.into()),
            ..Default::default()
        }
    }

    pub fn with_selector(mut self, s: impl Into<String>) -> Self {
        self.selector = Some(s.into());
        self
    }

    pub fn with_url(mut self, u: impl Into<String>) -> Self {
        self.url = Some(u.into());
        self
    }

    /// Nichts gesetzt — der Befund gilt dem Dokument als Ganzem.
    pub fn is_empty(&self) -> bool {
        self == &Location::default()
    }
}

/// Woher ein Befund seine Tatsachenbasis hat. Macht nachvollziehbar, ob eine
/// Aussage aus dem nativen Accessibility-Tree stammt, aus einem Attribut oder
/// aus einer Messung — was je nach Oberfläche unterschiedlich verfügbar ist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// `ax_tree`, `dom_attribute`, `meta`, `css_property`, `http_header`, `computed`.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl Evidence {
    pub fn new(source: impl Into<String>) -> Self {
        Evidence {
            source: source.into(),
            field: None,
            value: None,
        }
    }

    pub fn ax_tree(value: impl Into<String>) -> Self {
        Evidence {
            source: "ax_tree".into(),
            field: None,
            value: Some(value.into()),
        }
    }

    pub fn dom_attribute(field: impl Into<String>, value: Option<String>) -> Self {
        Evidence {
            source: "dom_attribute".into(),
            field: Some(field.into()),
            value,
        }
    }

    /// Aus einer Messung abgeleitet statt direkt abgelesen, z. B. ein Kontrastwert.
    pub fn computed(field: impl Into<String>, value: impl Into<String>) -> Self {
        Evidence {
            source: "computed".into(),
            field: Some(field.into()),
            value: Some(value.into()),
        }
    }
}

/// Ein Prüfergebnis.
///
/// Wird über [`Finding::fail`], [`Finding::review`], [`Finding::pass`] oder
/// [`Finding::untested`] erzeugt und mit den `with_*`-Methoden angereichert —
/// der Zustand ist damit immer bewusst gesetzt und nie ein Vorgabewert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Stabile Regelkennung, z. B. `a11y/img-alt`. Über alle Oberflächen
    /// identisch — das ist der Sinn des gemeinsamen Modells.
    pub rule_id: String,
    /// Menschenlesbarer Name der Regel, für Berichte, die mehr als die Kennung
    /// zeigen wollen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_name: Option<String>,
    pub outcome: Outcome,
    pub severity: Severity,
    /// Was ist der Fall. Sachlich, ohne Handlungsanweisung.
    pub message: String,

    #[serde(default, skip_serializing_if = "Location::is_empty")]
    pub location: Location,

    /// Die berechnete Rolle des betroffenen Elements.
    ///
    /// Gehört zum Befund, nicht zur Verortung: Wer einen Bericht liest, will
    /// wissen, *was* das Element für die Assistenztechnik war — ein Selektor
    /// allein sagt das nicht. Leer, wenn der Host keine Semantik liefert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Der Accessible Name des betroffenen Elements, soweit vorhanden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Erfüllte bzw. verletzte WCAG-Erfolgskriterien, z. B. `["1.1.1"]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wcag: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wcag_level: Option<WcagLevel>,
    /// Freie Schlagworte, z. B. `["best-practice"]` oder `en301549:9.1.1.1`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Was zu tun ist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
    /// Konkreter Vorschlag in Prosa.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Konkreter Vorschlag als Code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_code: Option<String>,

    /// Das betroffene Markup, soweit der Host es liefern kann.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,

    /// Werkzeugspezifische Beigabe. Nie serialisiert, nicht Teil der Identität
    /// dieses Befunds — siehe [`Extra`].
    #[serde(skip)]
    pub extra: Extra,
}

impl Finding {
    fn new(outcome: Outcome, rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Finding {
            rule_id: rule_id.into(),
            rule_name: None,
            outcome,
            severity: Severity::default(),
            message: message.into(),
            location: Location::default(),
            role: None,
            name: None,
            wcag: Vec::new(),
            wcag_level: None,
            tags: Vec::new(),
            help: None,
            help_url: None,
            suggestion: None,
            suggested_code: None,
            snippet: None,
            evidence: Vec::new(),
            extra: Extra::none(),
        }
    }

    /// Automatisch festgestelltes Problem.
    pub fn fail(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Outcome::Fail, rule_id, message)
    }

    /// Heuristischer Verdacht — braucht menschliche Bestätigung.
    pub fn review(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Outcome::Review, rule_id, message)
    }

    /// Automatische Prüfung bestanden.
    pub fn pass(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Outcome::Pass, rule_id, message)
    }

    /// Automatisiert nicht beurteilbar — erzeugt einen Punkt auf der manuellen
    /// Prüfliste, kein Urteil.
    pub fn untested(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Outcome::Untested, rule_id, message)
    }

    pub fn with_severity(mut self, s: Severity) -> Self {
        self.severity = s;
        self
    }

    pub fn with_rule_name(mut self, n: impl Into<String>) -> Self {
        self.rule_name = Some(n.into());
        self
    }

    /// Rolle und Accessible Name des betroffenen Elements.
    pub fn with_element(mut self, role: Option<String>, name: Option<String>) -> Self {
        self.role = role;
        self.name = name;
        self
    }

    /// Hängt eine werkzeugspezifische Beigabe an. Siehe [`Extra`].
    pub fn with_extra<T: std::any::Any + Send + Sync>(mut self, value: T) -> Self {
        self.extra = Extra::new(value);
        self
    }

    pub fn at(mut self, l: Location) -> Self {
        self.location = l;
        self
    }

    pub fn with_wcag(mut self, criteria: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.wcag = criteria.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_wcag_level(mut self, l: WcagLevel) -> Self {
        self.wcag_level = Some(l);
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_help_url(mut self, url: impl Into<String>) -> Self {
        self.help_url = Some(url.into());
        self
    }

    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }

    pub fn with_suggested_code(mut self, s: impl Into<String>) -> Self {
        self.suggested_code = Some(s.into());
        self
    }

    pub fn with_snippet(mut self, s: impl Into<String>) -> Self {
        self.snippet = Some(s.into());
        self
    }

    pub fn with_evidence(mut self, e: impl IntoIterator<Item = Evidence>) -> Self {
        self.evidence = e.into_iter().collect();
        self
    }

    pub fn add_evidence(mut self, e: Evidence) -> Self {
        self.evidence.push(e);
        self
    }
}
