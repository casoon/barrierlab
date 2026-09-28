//! Strukturierte Daten (JSON-LD, schema.org): Blöcke lesen, Knoten bewerten.
//!
//! Eingabe ist der Text jedes `<script type="application/ld+json">`, wie der
//! Host ihn gefunden hat — aus dem Markup oder aus dem DOM einer laufenden
//! Seite. Zwei Schritte:
//!
//! 1. [`parse_blocks`] liest jeden Text, löst Wurzel-Listen und `@graph` in
//!    Knoten auf, normalisiert alle `@type`-Einträge und hält Strukturprobleme
//!    als [`StructuralIssue`] fest.
//! 2. [`assess_node`] bewertet einen Knoten je Typ gegen das dokumentierte
//!    Profil des Suchmerkmals: fehlende Pflicht- und Empfehlungsangaben als
//!    Pfade, Quelle, Stand.
//!
//! [`duplicate_types`] meldet Typen, die in mehreren Blöcken derselben Seite
//! stehen.
//!
//! Was draußen bleibt: Abgleich mit sichtbarem Inhalt (braucht die gerenderte
//! Seite), Konsistenz über mehrere Seiten (braucht den ganzen Build),
//! Microdata und RDFa. Befundkennungen, Schweregrade und Texte bildet der Host.

mod parse;
mod rules;

pub use parse::{
    Block, DuplicateType, Node, StructuralIssue, duplicate_types, extract_types,
    has_schema_org_context, normalize_schema_type, parse_block, parse_blocks,
};
pub use rules::{
    ManualReview, ProductRuleContext, RULESET_VERSION, SchemaFeature, SchemaFeatureAvailability,
    SchemaRequirementStatus, SchemaRuleAssessment, assess_node, inventory_fields,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assess_all(script: serde_json::Value) -> Vec<SchemaRuleAssessment> {
        let block = parse_block(&script.to_string());
        block
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(index, node)| {
                node.types.iter().flat_map(move |schema_type| {
                    assess_node(
                        index,
                        schema_type,
                        &node.content,
                        ProductRuleContext::Indeterminate,
                    )
                })
            })
            .collect()
    }

    #[test]
    fn graph_nodes_have_no_container_false_positives() {
        let assessments = assess_all(json!({
            "@context": "https://schema.org",
            "@graph": [
                {
                    "@type": "Product",
                    "name": "Gadget",
                    "image": "https://example.com/gadget.jpg",
                    "offers": {"@type": "Offer", "price": "29.99", "priceCurrency": "EUR"}
                },
                {
                    "@type": "BreadcrumbList",
                    "itemListElement": [
                        {"@type": "ListItem", "position": 1, "name": "Home", "item": "https://example.com"},
                        {"@type": "ListItem", "position": 2, "name": "Products"}
                    ]
                }
            ]
        }));
        assert!(assessments
            .iter()
            .all(|a| a.requirement_status != SchemaRequirementStatus::MissingRequiredProperties));
    }

    #[test]
    fn full_type_iri_is_assessed_as_known_type() {
        let assessments = assess_all(json!({
            "@context": "https://schema.org",
            "@type": "https://schema.org/Product",
            "name": "Gadget",
            "offers": {"@type": "Offer", "price": "29.99", "priceCurrency": "EUR"}
        }));
        assert_eq!(assessments[0].schema_type, "Product");
        assert_eq!(assessments[0].feature, SchemaFeature::ProductSnippet);
    }

    #[test]
    fn every_type_entry_is_assessed() {
        let assessments = assess_all(json!({
            "@context": "https://schema.org",
            "@type": ["Organization", "LocalBusiness"],
            "name": "Shop"
        }));
        let features: Vec<_> = assessments.iter().map(|a| a.feature).collect();
        assert_eq!(
            features,
            [SchemaFeature::Organization, SchemaFeature::LocalBusiness]
        );
    }

    #[test]
    fn article_reports_recommended_properties_separately() {
        let assessments = assess_all(json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Accessibility audit",
            "image": "https://example.com/article.png",
            "author": {"@type": "Person", "name": "Ada"},
            "datePublished": "2026-01-01"
        }));
        assert!(assessments[0].missing_required.is_empty());
        assert_eq!(assessments[0].missing_recommended, ["dateModified"]);
    }
}
