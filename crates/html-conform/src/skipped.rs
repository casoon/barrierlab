//! Records the unchecked classes that actually occur in a document, so a
//! host can tell "nothing found" from "not looked at".

use html5_parser::{Document, NodeId, NodeKind};

use crate::{Finding, Severity, Skipped, SkippedClass};

pub(crate) fn detect(document: &Document, findings: &[Finding]) -> Vec<Skipped> {
    let mut skipped = Vec::new();
    if contains_style_element(document, document.root()) {
        skipped.push(Skipped {
            class: SkippedClass::StyleElementCss,
            detail: "the document has <style> elements; their CSS is not validated".to_owned(),
        });
    }
    // The CSP stage reports unevaluated hash sources as its only Info.
    if findings
        .iter()
        .any(|f| f.rule_id == "csp.meta-enforcement" && f.severity == Severity::Info)
    {
        skipped.push(Skipped {
            class: SkippedClass::CspHashSource,
            detail: "a Content-Security-Policy meta tag has hash sources; hashes are not \
                     evaluated against inline content"
                .to_owned(),
        });
    }
    skipped
}

fn contains_style_element(document: &Document, id: NodeId) -> bool {
    if let NodeKind::Element { name, .. } = &document.node(id).kind
        && name.eq_ignore_ascii_case("style")
    {
        return true;
    }
    document
        .children(id)
        .any(|child| contains_style_element(document, child))
}

#[cfg(test)]
mod tests {
    use crate::{SkippedClass, check};

    const HEAD: &str = "<!doctype html><html lang=en><head><title>t</title>";

    fn classes(html: &str) -> Vec<SkippedClass> {
        check(html)
            .expect("checks")
            .skipped
            .iter()
            .map(|s| s.class)
            .collect()
    }

    #[test]
    fn document_without_style_records_nothing() {
        assert!(classes(&format!("{HEAD}</head><body><p>x</p>")).is_empty());
    }

    #[test]
    fn style_element_is_recorded() {
        assert_eq!(
            classes(&format!(
                "{HEAD}<style>p{{color:red}}</style></head><body><p>x</p>"
            )),
            [SkippedClass::StyleElementCss]
        );
    }

    #[test]
    fn csp_hash_source_is_recorded() {
        let html = format!(
            "{HEAD}<meta http-equiv=\"Content-Security-Policy\" \
             content=\"script-src 'sha256-AAAA'\"></head><body><script>1</script>"
        );
        assert_eq!(classes(&html), [SkippedClass::CspHashSource]);
    }
}
