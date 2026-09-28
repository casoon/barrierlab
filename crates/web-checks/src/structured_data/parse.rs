//! JSON-LD-Blöcke lesen und in Knoten zerlegen.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Ein `<script type="application/ld+json">`, gelesen und zerlegt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Die Knoten des Blocks in Dokumentreihenfolge: Wurzel, Einträge einer
    /// Wurzel-Liste, Einträge von `@graph`. Ein Knoten ohne `@type` steht mit
    /// leerer Typliste darin; ein `@graph`-Behälter ohne `@type` nicht.
    pub nodes: Vec<Node>,
    /// Strukturprobleme in der Reihenfolge ihres Auftretens.
    pub issues: Vec<StructuralIssue>,
    /// Meldung des JSON-Parsers, wenn der Text kein JSON ist. Technischer
    /// Parser-Text, keine Formulierung für Berichte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_error: Option<String>,
}

/// Ein JSON-LD-Knoten mit seinen normalisierten Typen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Alle Einträge von `@type`, volle IRIs auf den Typnamen gekürzt
    /// (`https://schema.org/Product` → `Product`).
    pub types: Vec<String>,
    /// Der Knoten, wie er im Block steht.
    pub content: Value,
}

/// Was an einem Block strukturell nicht stimmt. **Daten, keine Wertung** —
/// Kennung, Schweregrad und Text bildet der Host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralIssue {
    /// Der Script-Text ist leer oder besteht nur aus Leerraum.
    EmptyScript,
    /// Der Text ist kein JSON; die Meldung steht in [`Block::json_error`].
    InvalidJson,
    /// Die Wurzel oder ein Eintrag einer Wurzel-Liste ist kein Objekt.
    InvalidRoot,
    /// Eine leere Wurzel-Liste oder ein leeres `@graph`.
    EmptyDocument,
    /// `@graph` ist keine Liste.
    GraphNotArray,
    /// Ein Knoten hat kein `@context` und erbt keines.
    MissingContext,
    /// Ein `@context` ist angegeben, verweist aber nicht auf schema.org.
    NonSchemaOrgContext,
    /// Ein `@graph`-Behälter ohne `@context`. Seine Einträge erben dann
    /// keines und melden es, wo sie keines haben, einzeln.
    GraphWithoutContext,
    /// Ein Knoten ohne verwertbares `@type`.
    MissingType,
}

/// Liest jeden Script-Text für sich, in Eingabereihenfolge. Der Index eines
/// Blocks ist der Index seines Textes.
pub fn parse_blocks<S: AsRef<str>>(scripts: &[S]) -> Vec<Block> {
    scripts.iter().map(|s| parse_block(s.as_ref())).collect()
}

/// Liest einen Script-Text.
///
/// Eine Wurzel-Liste wird in ihre Einträge aufgelöst, `@graph` in seine
/// Knoten; ein `@context` gilt für die Einträge seines `@graph`. Als
/// schema.org-Kontext zählt nur `http(s)://schema.org` (Schrägstrich am Ende
/// gleichgültig), auch als Eintrag einer Liste oder als `@vocab`.
pub fn parse_block(text: &str) -> Block {
    let mut block = Block {
        nodes: Vec::new(),
        issues: Vec::new(),
        json_error: None,
    };
    if text.trim().is_empty() {
        block.issues.push(StructuralIssue::EmptyScript);
        return block;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value) => normalize_document(&value, &mut block),
        Err(error) => {
            block.issues.push(StructuralIssue::InvalidJson);
            block.json_error = Some(error.to_string());
        }
    }
    block
}

fn normalize_document(value: &Value, block: &mut Block) {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                block.issues.push(StructuralIssue::EmptyDocument);
                return;
            }
            for item in items {
                normalize_root(item, false, block);
            }
        }
        Value::Object(_) => normalize_root(value, false, block),
        _ => block.issues.push(StructuralIssue::InvalidRoot),
    }
}

fn normalize_root(value: &Value, context_inherited: bool, block: &mut Block) {
    let Some(object) = value.as_object() else {
        block.issues.push(StructuralIssue::InvalidRoot);
        return;
    };

    let graph = object.get("@graph");
    let context = object.get("@context");
    let has_context = context_inherited || has_schema_org_context(context);
    if !has_context {
        block.issues.push(match (context, graph) {
            (Some(_), _) => StructuralIssue::NonSchemaOrgContext,
            (None, Some(_)) => StructuralIssue::GraphWithoutContext,
            (None, None) => StructuralIssue::MissingContext,
        });
    }

    let types = extract_types(value);
    if !types.is_empty() {
        block.nodes.push(Node {
            types,
            content: value.clone(),
        });
    } else if graph.is_none() {
        block.nodes.push(Node {
            types: Vec::new(),
            content: value.clone(),
        });
        block.issues.push(StructuralIssue::MissingType);
    }

    if let Some(graph) = graph {
        let Some(items) = graph.as_array() else {
            block.issues.push(StructuralIssue::GraphNotArray);
            return;
        };
        if items.is_empty() {
            block.issues.push(StructuralIssue::EmptyDocument);
        }
        for item in items {
            normalize_root(item, has_context, block);
        }
    }
}

/// Verweist dieser `@context` auf schema.org? Zeichenkette, Liste oder Objekt
/// mit `@vocab`; eine Teilzeichenkette wie `example.com/schema.org-ish` zählt
/// nicht.
pub fn has_schema_org_context(context: Option<&Value>) -> bool {
    match context {
        Some(Value::String(value)) => {
            matches!(
                value.trim_end_matches('/'),
                "https://schema.org" | "http://schema.org"
            )
        }
        Some(Value::Array(values)) => values
            .iter()
            .any(|value| has_schema_org_context(Some(value))),
        Some(Value::Object(values)) => values
            .get("@vocab")
            .is_some_and(|value| has_schema_org_context(Some(value))),
        _ => false,
    }
}

/// Alle `@type`-Einträge eines Knotens, normalisiert mit
/// [`normalize_schema_type`]. Leere Einträge fallen weg.
pub fn extract_types(node: &Value) -> Vec<String> {
    match node.get("@type") {
        Some(Value::String(value)) => normalize_schema_type(value).into_iter().collect(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .filter_map(normalize_schema_type)
            .collect(),
        _ => Vec::new(),
    }
}

/// Kürzt eine Typangabe auf ihren Namen: `https://schema.org/Product` und
/// `schema:Product#` werden zu `Product`. Leer ergibt `None`.
pub fn normalize_schema_type(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(
        trimmed
            .rsplit(['/', '#'])
            .next()
            .unwrap_or(trimmed)
            .to_string(),
    )
}

/// Ein `@type`, der in mehr als einem Block derselben Seite vorkommt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateType {
    pub schema_type: String,
    /// Indizes der Blöcke mit diesem Typ, aufsteigend.
    pub blocks: Vec<usize>,
}

/// `@type`-Werte, die in mehr als einem Block vorkommen, in der Reihenfolge
/// ihres ersten Auftretens. Gezählt werden alle Typen aller Knoten, auch
/// innerhalb von `@graph`; mehrfach im selben Block zählt einmal — gemeint ist
/// „dieselbe Entität in getrennten Scripts", nicht ein `@graph` mit zwei
/// Personen.
pub fn duplicate_types(blocks: &[Block]) -> Vec<DuplicateType> {
    let mut seen: Vec<DuplicateType> = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        for schema_type in block.nodes.iter().flat_map(|node| &node.types) {
            match seen.iter_mut().find(|d| &d.schema_type == schema_type) {
                Some(entry) => {
                    if entry.blocks.last() != Some(&index) {
                        entry.blocks.push(index);
                    }
                }
                None => seen.push(DuplicateType {
                    schema_type: schema_type.clone(),
                    blocks: vec![index],
                }),
            }
        }
    }
    seen.retain(|entry| entry.blocks.len() > 1);
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: Value) -> Block {
        parse_block(&value.to_string())
    }

    #[test]
    fn astro_structured_data_exports_work_inline_and_as_graph() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/astro_structured_data_components.json"
        ))
        .unwrap();
        let components = fixture["components"].as_array().unwrap();
        let inline = fixture["inline"].as_array().unwrap();
        assert_eq!(components.len(), 18);
        assert_eq!(inline.len(), 17); // SchemaGraph ist der Behälter.

        for item in inline {
            let block = parse(item["payload"].clone());
            assert!(!block.nodes.is_empty(), "{} inline", item["component"]);
            assert!(block.issues.is_empty(), "{} inline", item["component"]);
        }

        let graph = parse(fixture["useGraph"].clone());
        assert_eq!(graph.nodes.len(), inline.len());
        assert!(graph.issues.is_empty());
    }

    #[test]
    fn graph_nodes_are_normalized_once_without_container() {
        let block = parse(json!({
            "@context": "https://schema.org",
            "@graph": [
                {"@type": "Product", "name": "Gadget"},
                {"@type": "BreadcrumbList", "itemListElement": []}
            ]
        }));
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].types, ["Product"]);
        assert_eq!(block.nodes[1].types, ["BreadcrumbList"]);
        assert!(block.issues.is_empty());
    }

    #[test]
    fn top_level_array_is_expanded_into_individual_nodes() {
        let block = parse(json!([
            {"@context": "https://schema.org", "@type": "WebSite", "name": "Example"},
            {"@context": "https://schema.org", "@type": "WebPage", "name": "About"}
        ]));
        assert_eq!(block.nodes.len(), 2);
        assert_eq!(block.nodes[0].types, ["WebSite"]);
        assert_eq!(block.nodes[1].types, ["WebPage"]);
        assert!(block.issues.is_empty());
    }

    #[test]
    fn every_type_entry_is_kept_and_iris_are_normalized() {
        let block = parse(json!({
            "@context": "https://schema.org",
            "@type": ["https://schema.org/Product", "http://schema.org/IndividualProduct/", ""]
        }));
        assert_eq!(block.nodes[0].types, ["Product", "IndividualProduct"]);
    }

    #[test]
    fn invalid_json_is_reported_with_parser_message() {
        let block = parse_block(r#"{"@context":"https://schema.org","@type":"Product""#);
        assert!(block.nodes.is_empty());
        assert_eq!(block.issues, [StructuralIssue::InvalidJson]);
        assert!(block.json_error.is_some());
    }

    #[test]
    fn empty_script_is_its_own_issue() {
        let block = parse_block("  \n ");
        assert_eq!(block.issues, [StructuralIssue::EmptyScript]);
        assert!(block.json_error.is_none());
    }

    #[test]
    fn missing_context_and_type_are_reported_separately() {
        let block = parse(json!({"name": "Untyped node"}));
        assert_eq!(
            block.issues,
            [
                StructuralIssue::MissingContext,
                StructuralIssue::MissingType
            ]
        );
        assert_eq!(block.nodes.len(), 1);
        assert!(block.nodes[0].types.is_empty());
    }

    #[test]
    fn non_schema_org_context_is_not_the_same_as_missing() {
        let block = parse(json!({
            "@context": "https://example.com/schema.org-ish",
            "@type": "Thing"
        }));
        assert_eq!(block.issues, [StructuralIssue::NonSchemaOrgContext]);
    }

    #[test]
    fn context_accepts_arrays_vocab_and_trailing_slash() {
        for context in [
            json!("http://schema.org/"),
            json!(["https://schema.org", {"ex": "https://example.com/"}]),
            json!({"@vocab": "https://schema.org/"}),
        ] {
            let block = parse(json!({"@context": context, "@type": "Thing"}));
            assert!(block.issues.is_empty(), "{context}");
        }
    }

    #[test]
    fn graph_without_context_is_reported_and_not_inherited() {
        let block = parse(json!({
            "@graph": [
                {"@type": "WebSite"},
                {"@context": "https://schema.org", "@type": "WebPage"}
            ]
        }));
        assert_eq!(
            block.issues,
            [
                StructuralIssue::GraphWithoutContext,
                StructuralIssue::MissingContext
            ]
        );
        assert_eq!(block.nodes.len(), 2);
    }

    #[test]
    fn invalid_roots_and_empty_documents() {
        assert_eq!(parse(json!("text")).issues, [StructuralIssue::InvalidRoot]);
        assert_eq!(parse(json!([])).issues, [StructuralIssue::EmptyDocument]);
        assert_eq!(
            parse(json!({"@context": "https://schema.org", "@graph": []})).issues,
            [StructuralIssue::EmptyDocument]
        );
        assert_eq!(
            parse(json!({"@context": "https://schema.org", "@graph": {}})).issues,
            [StructuralIssue::GraphNotArray]
        );
    }

    #[test]
    fn duplicate_types_count_blocks_not_nodes() {
        let blocks = parse_blocks(&[
            json!({"@context": "https://schema.org", "@type": "Organization"}).to_string(),
            json!({"@context": "https://schema.org", "@graph": [
                {"@type": "Organization"}, {"@type": "Person"}, {"@type": "Person"}
            ]})
            .to_string(),
            "{not json".to_string(),
            json!({"@context": "https://schema.org", "@type": ["WebPage", "Organization"]})
                .to_string(),
        ]);
        assert_eq!(
            duplicate_types(&blocks),
            [DuplicateType {
                schema_type: "Organization".to_string(),
                blocks: vec![0, 1, 3],
            }]
        );
    }

    #[test]
    fn single_block_has_no_duplicates() {
        let blocks = parse_blocks(&[json!({"@context": "https://schema.org", "@graph": [
            {"@type": "Person"}, {"@type": "Person"}
        ]})
        .to_string()]);
        assert!(duplicate_types(&blocks).is_empty());
    }
}
