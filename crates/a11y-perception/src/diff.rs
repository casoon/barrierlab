//! Die Differenz zweier [`AXSnapshot`]s — das Primitiv, mit dem der
//! Journey-Layer eine Interaktion bewertet: aufnehmen, handeln, aufnehmen,
//! vergleichen.
//!
//! Erfasst werden hinzugekommene und verschwundene Knoten, Änderungen an
//! ARIA-Zustandseigenschaften je Knoten, Fokusbewegungen sowie Titel- und
//! URL-Wechsel.
//!
//! # Knotenidentität
//!
//! Verglichen wird über die **Backend-Node-ID**, nicht über die
//! AXTree-Knotenkennung. Chrome vergibt `nodeId` je `getFullAXTree`-Aufruf
//! neu; ein Vergleich darüber würde bei jedem zweiten Aufruf den halben Baum
//! als „hinzugekommen" und „verschwunden" melden und keine einzige
//! Eigenschaftsänderung finden. Die Backend-ID zeigt dagegen auf den
//! DOM-Knoten und bleibt über die Lebensdauer der Seite stabil. Nur Knoten
//! ohne Backend-ID fallen auf die AXTree-Kennung zurück.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::snapshot::AXSnapshot;
use crate::tree::AXNode;

/// Stabile Kennung eines Knotens über zwei Aufnahmen hinweg.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeKey {
    /// Der Regelfall: zeigt auf den DOM-Knoten, stabil über Aufnahmen.
    Backend(i64),
    /// Rückfall für Knoten ohne DOM-Gegenstück.
    Ax(String),
}

fn key_of(node: &AXNode) -> NodeKey {
    match node.backend_dom_node_id {
        Some(id) => NodeKey::Backend(id),
        None => NodeKey::Ax(node.node_id.clone()),
    }
}

fn index(snapshot: &AXSnapshot) -> HashMap<NodeKey, &AXNode> {
    snapshot
        .tree
        .iter_all()
        .map(|node| (key_of(node), node))
        .collect()
}

/// Difference between two captured snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AXTreeDiff {
    /// Knoten, die wahrnehmbar geworden sind: im `after`-Baum vorhanden und
    /// nicht ignoriert, vorher nicht vorhanden **oder** ignoriert.
    pub added: Vec<String>,
    /// Knoten, die aufgehört haben, wahrnehmbar zu sein — das Gegenstück.
    pub removed: Vec<String>,
    /// Per-node property changes (filled in Phase 2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub property_changes: Vec<PropertyChange>,
    /// Focus moved between snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_moved: Option<FocusMove>,
    /// `document.title` changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_changed: Option<(String, String)>,
    /// URL changed without a full page reload (SPA navigation indicator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_changed: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyChange {
    /// AXTree-Kennung aus der *späteren* Aufnahme. Nur zur Anzeige — die
    /// Zuordnung läuft über `backend_node_id`.
    pub node_id: String,
    /// Backend-Node-ID, sofern der Knoten ein DOM-Gegenstück hat. Der
    /// Schlüssel, über den ein Aufrufer eine Änderung dem Element zuordnet,
    /// das er bedient hat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<i64>,
    /// Property name, e.g. `"expanded"`.
    pub property: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusMove {
    pub before: Option<i64>,
    pub after: Option<i64>,
}

/// ARIA state properties tracked for property-level diffs.
///
/// Gelesen wird über [`AXNode::property_value_str`], nicht über
/// `get_property_bool`: `invalid` kommt aus Chrome als Token-String
/// ("false"/"true"/"grammar"/"spelling"), nie als `Bool`. Über den
/// Bool-Zugriff war diese Eigenschaft hier stillschweigend tot — sie stand
/// in der Liste, konnte aber nie eine Änderung erzeugen.
///
/// `checked` und `pressed` sind in Chrome Tristate-Token
/// ("true"/"false"/"mixed"). Ohne sie meldete der Diff bei Checkbox,
/// Switch und Toggle-Button keine Wirkung einer Bedienung (gefunden im
/// Relief-CDP-Spike: Checkbox angeklickt, Diff leer).
const TRACKED_PROPERTIES: &[&str] = &[
    "expanded", "hidden", "selected", "invalid", "modal", "checked", "pressed",
];

impl AXTreeDiff {
    /// Compute the structural diff between two snapshots.
    /// Phase 2: adds property-level diffing for key ARIA state properties.
    pub fn between(before: &AXSnapshot, after: &AXSnapshot) -> Self {
        let mut diff = AXTreeDiff::default();

        if before.document_title != after.document_title {
            diff.title_changed =
                Some((before.document_title.clone(), after.document_title.clone()));
        }
        if before.url != after.url {
            diff.url_changed = Some((before.url.clone(), after.url.clone()));
        }
        if before.focus.active_backend_node_id != after.focus.active_backend_node_id {
            diff.focus_moved = Some(FocusMove {
                before: before.focus.active_backend_node_id,
                after: after.focus.active_backend_node_id,
            });
        }

        let before_index = index(before);
        let after_index = index(after);

        // „Hinzugekommen" heißt **wahrnehmbar geworden**, nicht „neu im Baum".
        //
        // Chrome entfernt einen verborgenen Knoten nicht: er bleibt mit
        // derselben Backend-ID stehen, als `ignored` mit dem Grund
        // `notRendered` und ohne seine Rolle. Ein `<ul role="menu" hidden>`,
        // das sichtbar wird, wechselt von `ignored=true, role="none"` nach
        // `ignored=false, role="menu"` — dieselbe ID, also strukturell keine
        // Änderung. Genau so öffnen die meisten Menüs und Akkordeons.
        //
        // Über die reine Anwesenheit wäre dieser Knoten unsichtbar geblieben,
        // und mit ihm jede Aussage darüber, *was* sich geöffnet hat.
        for (key, node) in &after_index {
            let was_perceivable = before_index.get(key).is_some_and(|n| !n.ignored);
            if !node.ignored && !was_perceivable {
                diff.added.push(node.node_id.clone());
            }
        }
        for (key, node) in &before_index {
            let is_perceivable = after_index.get(key).is_some_and(|n| !n.ignored);
            if !node.ignored && !is_perceivable {
                diff.removed.push(node.node_id.clone());
            }
        }
        diff.added.sort();
        diff.removed.sort();

        for (key, after_node) in &after_index {
            let Some(before_node) = before_index.get(key) else {
                continue;
            };
            for prop_name in TRACKED_PROPERTIES {
                let before_val = before_node.property_value_str(prop_name);
                let after_val = after_node.property_value_str(prop_name);
                if before_val != after_val {
                    diff.property_changes.push(PropertyChange {
                        node_id: after_node.node_id.clone(),
                        backend_node_id: after_node.backend_dom_node_id,
                        property: prop_name.to_string(),
                        before: before_val.unwrap_or_default(),
                        after: after_val.unwrap_or_default(),
                    });
                }
            }
        }
        diff.property_changes.sort_by(|a, b| {
            (a.backend_node_id, &a.property).cmp(&(b.backend_node_id, &b.property))
        });

        diff
    }

    /// Die Änderung einer verfolgten Eigenschaft an genau dem Knoten, den der
    /// Aufrufer bedient hat.
    ///
    /// Das ist der Unterschied zwischen „irgendwo auf der Seite hat sich
    /// `expanded` geändert" und „der geklickte Auslöser hat sich geändert".
    pub fn property_change_for(
        &self,
        backend_node_id: i64,
        property: &str,
    ) -> Option<&PropertyChange> {
        self.property_changes
            .iter()
            .find(|c| c.backend_node_id == Some(backend_node_id) && c.property == property)
    }

    /// Ob durch die Handlung Inhalt wahrnehmbar geworden ist.
    ///
    /// Seitenweit: unterschieden wird nicht zwischen dem geöffneten Bereich
    /// und einem nachgeladenen Bild am Seitenende. Für eine schärfere Aussage
    /// müsste der gesteuerte Bereich bestimmt werden.
    pub fn has_additions(&self) -> bool {
        !self.added.is_empty()
    }

    /// Ob Knoten aus dem Accessibility-Tree verschwunden sind.
    pub fn has_removals(&self) -> bool {
        !self.removed.is_empty()
    }

    /// True when nothing structural changed between the two snapshots.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.property_changes.is_empty()
            && self.focus_moved.is_none()
            && self.title_changed.is_none()
            && self.url_changed.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::super::snapshot::FocusSnapshot;
    use super::super::tree::{AXNode, AXTree};
    use super::*;

    fn snap(label: &str, title: &str, url: &str, focus: Option<i64>) -> AXSnapshot {
        AXSnapshot::new(
            label,
            url,
            title,
            0,
            AXTree::new(),
            FocusSnapshot {
                active_backend_node_id: focus,
                ..Default::default()
            },
        )
    }

    #[test]
    fn empty_diff_when_identical() {
        let a = snap("a", "T", "https://x", Some(1));
        let b = snap("b", "T", "https://x", Some(1));
        assert!(AXTreeDiff::between(&a, &b).is_empty());
    }

    #[test]
    fn detects_title_and_focus_change() {
        let a = snap("a", "Old", "https://x", Some(1));
        let b = snap("b", "New", "https://x", Some(2));
        let d = AXTreeDiff::between(&a, &b);
        assert_eq!(d.title_changed, Some(("Old".into(), "New".into())));
        assert_eq!(d.focus_moved.unwrap().after, Some(2));
        assert!(d.url_changed.is_none());
    }

    /// Baut einen AX-Knoten mit einer booleschen Eigenschaft.
    /// Ein Knoten mit einer Token-Eigenschaft, wie Chrome `aria-invalid`
    /// liefert: als String, nie als Bool.
    fn node_with_token(ax_id: &str, backend: i64, name: &str, value: &str) -> AXNode {
        let mut n = node(ax_id, backend, None);
        n.properties.push(super::super::tree::AXProperty {
            name: name.to_string(),
            value: super::super::tree::AXValue::String(value.to_string()),
        });
        n
    }

    /// Ein verborgener Knoten verschwindet nicht aus dem Baum — Chrome lässt
    /// ihn mit derselben Backend-ID als `ignored` stehen und nimmt ihm die
    /// Rolle. Wird er sichtbar, ist das strukturell keine Änderung, für die
    /// Wahrnehmung aber der ganze Vorgang. So öffnen die meisten Menüs.
    #[test]
    fn wahrnehmbar_gewordener_knoten_gilt_als_hinzugekommen() {
        let mut hidden = node("16", 16, None);
        hidden.ignored = true;
        hidden.role = Some("none".to_string());
        let mut shown = node("16", 16, None);
        shown.role = Some("menu".to_string());

        let mut before = snap("a", "T", "https://x", None);
        before.tree = AXTree::from_nodes(vec![hidden]);
        let mut after = snap("b", "T", "https://x", None);
        after.tree = AXTree::from_nodes(vec![shown]);

        let diff = AXTreeDiff::between(&before, &after);
        assert_eq!(diff.added, vec!["16".to_string()]);
        assert!(diff.has_additions());
    }

    /// Und die Gegenrichtung: verborgen werden heißt verschwinden.
    #[test]
    fn ignoriert_gewordener_knoten_gilt_als_verschwunden() {
        let shown = node("16", 16, None);
        let mut hidden = node("16", 16, None);
        hidden.ignored = true;

        let mut before = snap("a", "T", "https://x", None);
        before.tree = AXTree::from_nodes(vec![shown]);
        let mut after = snap("b", "T", "https://x", None);
        after.tree = AXTree::from_nodes(vec![hidden]);

        let diff = AXTreeDiff::between(&before, &after);
        assert_eq!(diff.removed, vec!["16".to_string()]);
        assert!(diff.has_removals());
    }

    /// Ein Knoten, der die ganze Zeit ignoriert ist, ist nie wahrnehmbar
    /// geworden — auch dann nicht, wenn er neu ins Dokument kommt.
    #[test]
    fn dauerhaft_ignorierter_knoten_zaehlt_nicht() {
        let mut ignored = node("99", 99, None);
        ignored.ignored = true;

        let before = snap("a", "T", "https://x", None);
        let mut after = snap("b", "T", "https://x", None);
        after.tree = AXTree::from_nodes(vec![ignored]);

        let diff = AXTreeDiff::between(&before, &after);
        assert!(diff.added.is_empty());
        assert!(!diff.has_additions());
    }

    /// `invalid` stand seit jeher in `TRACKED_PROPERTIES`, wurde aber über
    /// `get_property_bool` gelesen und konnte deshalb nie eine Änderung
    /// erzeugen. Derselbe Typ-Irrtum wie #566, eine Ebene tiefer.
    #[test]
    fn token_eigenschaften_erzeugen_eine_aenderung() {
        let mut before = snap("a", "T", "https://x", None);
        before.tree = AXTree::from_nodes(vec![node_with_token("1", 7, "invalid", "false")]);
        let mut after = snap("b", "T", "https://x", None);
        after.tree = AXTree::from_nodes(vec![node_with_token("1", 7, "invalid", "true")]);

        let diff = AXTreeDiff::between(&before, &after);
        let change = diff
            .property_change_for(7, "invalid")
            .expect("Wechsel an aria-invalid muss sichtbar sein");
        assert_eq!(change.before, "false");
        assert_eq!(change.after, "true");
    }

    /// Checkbox und Toggle-Button: Tristate-Token, auch „mixed“.
    #[test]
    fn checked_und_pressed_erzeugen_eine_aenderung() {
        for (prop, from, to) in [
            ("checked", "false", "true"),
            ("checked", "true", "mixed"),
            ("pressed", "false", "true"),
        ] {
            let mut before = snap("a", "T", "https://x", None);
            before.tree = AXTree::from_nodes(vec![node_with_token("1", 7, prop, from)]);
            let mut after = snap("b", "T", "https://x", None);
            after.tree = AXTree::from_nodes(vec![node_with_token("1", 7, prop, to)]);

            let diff = AXTreeDiff::between(&before, &after);
            let change = diff
                .property_change_for(7, prop)
                .unwrap_or_else(|| panic!("Wechsel an {prop} muss sichtbar sein"));
            assert_eq!((change.before.as_str(), change.after.as_str()), (from, to));
        }
    }

    /// Boolesche Eigenschaften bleiben unverändert in ihrer Darstellung.
    #[test]
    fn boolesche_eigenschaften_bleiben_true_false() {
        let mut before = snap("a", "T", "https://x", None);
        before.tree = AXTree::from_nodes(vec![node("1", 7, Some(("expanded", false)))]);
        let mut after = snap("b", "T", "https://x", None);
        after.tree = AXTree::from_nodes(vec![node("1", 7, Some(("expanded", true)))]);

        let change = AXTreeDiff::between(&before, &after)
            .property_change_for(7, "expanded")
            .cloned()
            .expect("Wechsel an expanded");
        assert_eq!(
            (change.before.as_str(), change.after.as_str()),
            ("false", "true")
        );
    }

    fn node(ax_id: &str, backend: i64, prop: Option<(&str, bool)>) -> AXNode {
        AXNode {
            node_id: ax_id.to_string(),
            ignored: false,
            ignored_reasons: Vec::new(),
            role: Some("button".to_string()),
            name: None,
            name_source: None,
            description: None,
            value: None,
            properties: prop
                .map(|(name, value)| {
                    vec![super::super::tree::AXProperty {
                        name: name.to_string(),
                        value: super::super::tree::AXValue::Bool(value),
                    }]
                })
                .unwrap_or_default(),
            child_ids: Vec::new(),
            parent_id: None,
            backend_dom_node_id: Some(backend),
        }
    }

    fn snap_with(label: &str, nodes: Vec<AXNode>) -> AXSnapshot {
        AXSnapshot::new(
            label,
            "https://x",
            "T",
            0,
            AXTree::from_nodes(nodes),
            FocusSnapshot::default(),
        )
    }

    /// Der Kern der Identitätskorrektur: Chrome vergibt `nodeId` je Abruf neu.
    /// Über die Backend-ID bleibt derselbe DOM-Knoten trotzdem derselbe —
    /// sonst wäre der halbe Baum „hinzugekommen" und „verschwunden".
    #[test]
    fn neue_ax_kennungen_erzeugen_keine_scheinaenderung() {
        let before = snap_with("a", vec![node("ax-1", 42, Some(("expanded", false)))]);
        let after = snap_with("b", vec![node("ax-999", 42, Some(("expanded", false)))]);

        let d = AXTreeDiff::between(&before, &after);
        assert!(d.added.is_empty(), "added: {:?}", d.added);
        assert!(d.removed.is_empty(), "removed: {:?}", d.removed);
        assert!(d.property_changes.is_empty());
    }

    #[test]
    fn eigenschaftswechsel_wird_dem_richtigen_knoten_zugeordnet() {
        let before = snap_with(
            "a",
            vec![
                node("ax-1", 42, Some(("expanded", false))),
                node("ax-2", 77, Some(("expanded", false))),
            ],
        );
        // Nur Knoten 77 klappt auf; 42 bekommt zusätzlich eine neue Kennung.
        let after = snap_with(
            "b",
            vec![
                node("ax-9", 42, Some(("expanded", false))),
                node("ax-8", 77, Some(("expanded", true))),
            ],
        );

        let d = AXTreeDiff::between(&before, &after);
        assert_eq!(d.property_changes.len(), 1);
        assert!(d.property_change_for(42, "expanded").is_none());
        let change = d
            .property_change_for(77, "expanded")
            .expect("Änderung an 77");
        assert_eq!(change.before, "false");
        assert_eq!(change.after, "true");
    }

    #[test]
    fn neuer_inhalt_erscheint_als_zugang() {
        let before = snap_with("a", vec![node("ax-1", 42, None)]);
        let after = snap_with("b", vec![node("ax-1", 42, None), node("ax-2", 43, None)]);

        let d = AXTreeDiff::between(&before, &after);
        assert!(d.has_additions());
        assert!(!d.has_removals());
    }

    #[test]
    fn detects_spa_navigation() {
        let a = snap("a", "T", "https://x/one", Some(1));
        let b = snap("b", "T", "https://x/two", Some(1));
        let d = AXTreeDiff::between(&a, &b);
        assert!(d.url_changed.is_some());
    }

    // --- Mutationskatalog tests/mutations/diff.toml (Refs #257) ---

    fn without_backend(mut n: AXNode) -> AXNode {
        n.backend_dom_node_id = None;
        n
    }

    fn ignored(mut n: AXNode) -> AXNode {
        n.ignored = true;
        n
    }

    fn with_bool(mut n: AXNode, name: &str, value: bool) -> AXNode {
        n.properties.push(super::super::tree::AXProperty {
            name: name.to_string(),
            value: super::super::tree::AXValue::Bool(value),
        });
        n
    }

    #[test]
    fn changes_are_reported_as_before_then_after() {
        let before = AXSnapshot {
            focus: FocusSnapshot {
                active_backend_node_id: Some(1),
                ..Default::default()
            },
            ..snap("a", "Alt", "https://x/alt", None)
        };
        let after = AXSnapshot {
            focus: FocusSnapshot {
                active_backend_node_id: Some(2),
                ..Default::default()
            },
            ..snap("b", "Neu", "https://x/neu", None)
        };
        let d = AXTreeDiff::between(&before, &after);
        assert_eq!(
            d.title_changed,
            Some(("Alt".to_string(), "Neu".to_string()))
        );
        assert_eq!(
            d.url_changed,
            Some(("https://x/alt".to_string(), "https://x/neu".to_string()))
        );
        let moved = d.focus_moved.expect("Fokus bewegt");
        assert_eq!((moved.before, moved.after), (Some(1), Some(2)));
        assert!(AXTreeDiff::between(&before, &before).is_empty());
    }

    #[test]
    fn nodes_without_backend_id_are_matched_by_their_ax_id() {
        let before = snap_with(
            "a",
            vec![
                without_backend(node("ax-1", 0, Some(("expanded", false)))),
                without_backend(node("ax-2", 0, None)),
            ],
        );
        let after = snap_with(
            "b",
            vec![without_backend(node("ax-1", 0, Some(("expanded", true))))],
        );
        let d = AXTreeDiff::between(&before, &after);
        assert_eq!(d.removed, vec!["ax-2"]);
        assert!(d.added.is_empty());
        assert_eq!(d.property_changes.len(), 1);
        assert_eq!(d.property_changes[0].node_id, "ax-1");
        assert_eq!(d.property_changes[0].backend_node_id, None);
    }

    #[test]
    fn ignored_nodes_never_count_as_added_or_removed() {
        // Nie wahrnehmbar: neu und ignoriert, vorher ignoriert und jetzt weg,
        // durchgehend ignoriert.
        let before = snap_with(
            "a",
            vec![ignored(node("1", 1, None)), ignored(node("2", 2, None))],
        );
        let after = snap_with(
            "b",
            vec![ignored(node("2", 2, None)), ignored(node("3", 3, None))],
        );
        let d = AXTreeDiff::between(&before, &after);
        assert!(d.added.is_empty(), "added: {:?}", d.added);
        assert!(d.removed.is_empty(), "removed: {:?}", d.removed);
        assert!(!d.has_additions() && !d.has_removals());
    }

    #[test]
    fn a_node_that_stays_perceivable_is_neither_added_nor_removed() {
        let before = snap_with("a", vec![node("1", 1, None)]);
        let after = snap_with("b", vec![node("9", 1, None)]);
        let d = AXTreeDiff::between(&before, &after);
        assert!(d.added.is_empty() && d.removed.is_empty());
    }

    #[test]
    fn added_and_removed_are_sorted_by_ax_id() {
        let ids: Vec<String> = (0..20).map(|i| format!("n{i:02}")).collect();
        let nodes = |offset: i64| -> Vec<AXNode> {
            ids.iter()
                .rev()
                .enumerate()
                .map(|(i, id)| node(id, offset + i as i64, None))
                .collect()
        };
        let d = AXTreeDiff::between(&snap_with("a", vec![]), &snap_with("b", nodes(0)));
        assert_eq!(d.added, ids);
        let d = AXTreeDiff::between(&snap_with("a", nodes(0)), &snap_with("b", vec![]));
        assert_eq!(d.removed, ids);
    }

    #[test]
    fn every_tracked_property_produces_a_change_and_others_do_not() {
        for prop in [
            "pressed", "checked", "modal", "invalid", "selected", "hidden", "expanded",
        ] {
            let before = snap_with("a", vec![node_with_token("1", 7, prop, "false")]);
            let after = snap_with("b", vec![node_with_token("1", 7, prop, "true")]);
            let d = AXTreeDiff::between(&before, &after);
            assert!(d.property_change_for(7, prop).is_some(), "{prop}");
            assert_eq!(d.property_changes.len(), 1, "{prop}");
        }
        let before = snap_with("a", vec![node_with_token("1", 7, "focusable", "false")]);
        let after = snap_with("b", vec![node_with_token("1", 7, "focusable", "true")]);
        assert!(
            AXTreeDiff::between(&before, &after)
                .property_changes
                .is_empty()
        );
    }

    #[test]
    fn a_property_change_names_the_later_ax_id_and_the_backend_id() {
        let before = snap_with("a", vec![node("alt", 7, Some(("expanded", false)))]);
        let after = snap_with("b", vec![node("neu", 7, Some(("expanded", true)))]);
        let c = &AXTreeDiff::between(&before, &after).property_changes[0];
        assert_eq!(c.node_id, "neu");
        assert_eq!(c.backend_node_id, Some(7));
        assert_eq!(c.property, "expanded");
    }

    #[test]
    fn an_appearing_or_vanishing_property_diffs_against_the_empty_string() {
        let before = snap_with("a", vec![node("1", 7, None)]);
        let after = snap_with("b", vec![node("1", 7, Some(("expanded", true)))]);
        let c = &AXTreeDiff::between(&before, &after).property_changes[0];
        assert_eq!((c.before.as_str(), c.after.as_str()), ("", "true"));
    }

    #[test]
    fn property_changes_are_sorted_by_backend_id_then_property() {
        let before = snap_with(
            "a",
            vec![
                with_bool(
                    with_bool(node("a", 9, None), "expanded", false),
                    "checked",
                    false,
                ),
                with_bool(
                    with_bool(node("b", 3, None), "expanded", false),
                    "checked",
                    false,
                ),
            ],
        );
        let after = snap_with(
            "b",
            vec![
                with_bool(
                    with_bool(node("a", 9, None), "expanded", true),
                    "checked",
                    true,
                ),
                with_bool(
                    with_bool(node("b", 3, None), "expanded", true),
                    "checked",
                    true,
                ),
            ],
        );
        let d = AXTreeDiff::between(&before, &after);
        let order: Vec<(Option<i64>, &str)> = d
            .property_changes
            .iter()
            .map(|c| (c.backend_node_id, c.property.as_str()))
            .collect();
        assert_eq!(
            order,
            vec![
                (Some(3), "checked"),
                (Some(3), "expanded"),
                (Some(9), "checked"),
                (Some(9), "expanded"),
            ]
        );
    }

    #[test]
    fn property_change_for_matches_node_and_property_together() {
        let before = snap_with(
            "a",
            vec![
                with_bool(
                    with_bool(node("a", 3, None), "expanded", false),
                    "checked",
                    false,
                ),
                with_bool(node("b", 9, None), "expanded", false),
            ],
        );
        let after = snap_with(
            "b",
            vec![
                with_bool(
                    with_bool(node("a", 3, None), "expanded", true),
                    "checked",
                    true,
                ),
                with_bool(node("b", 9, None), "expanded", true),
            ],
        );
        let d = AXTreeDiff::between(&before, &after);
        assert_eq!(
            d.property_change_for(3, "checked").unwrap().property,
            "checked"
        );
        assert_eq!(d.property_change_for(9, "expanded").unwrap().node_id, "b");
        assert!(d.property_change_for(9, "checked").is_none());
        assert!(d.property_change_for(4, "expanded").is_none());
    }

    #[test]
    fn is_empty_looks_at_every_kind_of_change() {
        use crate::diff::{FocusMove, PropertyChange};
        let cases: Vec<(&str, AXTreeDiff)> = vec![
            (
                "added",
                AXTreeDiff {
                    added: vec!["1".into()],
                    ..Default::default()
                },
            ),
            (
                "removed",
                AXTreeDiff {
                    removed: vec!["1".into()],
                    ..Default::default()
                },
            ),
            (
                "property",
                AXTreeDiff {
                    property_changes: vec![PropertyChange {
                        node_id: "1".into(),
                        backend_node_id: None,
                        property: "expanded".into(),
                        before: "false".into(),
                        after: "true".into(),
                    }],
                    ..Default::default()
                },
            ),
            (
                "focus",
                AXTreeDiff {
                    focus_moved: Some(FocusMove {
                        before: None,
                        after: Some(1),
                    }),
                    ..Default::default()
                },
            ),
            (
                "title",
                AXTreeDiff {
                    title_changed: Some(("a".into(), "b".into())),
                    ..Default::default()
                },
            ),
            (
                "url",
                AXTreeDiff {
                    url_changed: Some(("a".into(), "b".into())),
                    ..Default::default()
                },
            ),
        ];
        assert!(AXTreeDiff::default().is_empty());
        for (what, diff) in cases {
            assert!(!diff.is_empty(), "{what} allein macht den Diff nicht leer");
        }
    }

    #[test]
    fn has_additions_and_removals_follow_their_lists() {
        let d = AXTreeDiff {
            added: vec!["1".into()],
            ..Default::default()
        };
        assert!(d.has_additions() && !d.has_removals());
        let d = AXTreeDiff {
            removed: vec!["1".into()],
            ..Default::default()
        };
        assert!(!d.has_additions() && d.has_removals());
    }
}
