//! Die Leseeinheiten der Browserprojektion.
//!
//! Ein [`ReadingItem`] ist ein Knoten in der Reihenfolge, in der ein
//! Screenreader ihn im Accessibility-Tree **vorfinden würde** — eine Näherung
//! über Browserdaten, keine Aufzeichnung dessen, was tatsächlich gesprochen
//! wurde. Was die Projektion übergeht, steht mit Grund als
//! [`IgnoredReadingNode`] daneben, damit eine Lücke sichtbar bleibt statt
//! stillschweigend zu verschwinden.

use serde::{Deserialize, Serialize};

/// A node in the order a screen reader would encounter it in the AXTree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingItem {
    pub seq: usize,
    pub role: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub states: Vec<String>,
    pub tab_stop: bool,
    pub depth: usize,
    pub node_id: String,
}

/// Diagnostic entry for ignored AXNodes skipped by the standard reading order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredReadingNode {
    pub node_id: String,
    pub role: Option<String>,
    pub name: Option<String>,
    pub depth: usize,
    pub reasons: Vec<String>,
}
