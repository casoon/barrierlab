//! Tree construction has no nesting limit (WHATWG HTML §13.2.6), so the
//! parser must handle any depth without one call frame per level. Runs on
//! a 2 MiB thread, the default for tokio's worker threads (auditmysite#783).

use html5_parser::{Document, NodeId, NodeKind, parse};

const DEPTH: usize = 10_000;

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(f)
        .expect("spawn test thread")
        .join()
        .expect("parsing must not overflow the stack")
}

fn is_html_element(document: &Document, id: NodeId, local: &str) -> bool {
    matches!(&document.node(id).kind, NodeKind::Element { name, .. } if name == local)
}

/// The deepest chain of `div`s below `id`, following first children.
fn div_chain_below(document: &Document, id: NodeId) -> usize {
    let mut depth = 0;
    let mut current = id;
    while let Some(child) = document
        .children(current)
        .find(|&child| is_html_element(document, child, "div"))
    {
        depth += 1;
        current = child;
    }
    depth
}

fn find_element(document: &Document, local: &str) -> Option<NodeId> {
    let mut pending = vec![document.root()];
    while let Some(id) = pending.pop() {
        if is_html_element(document, id, local) {
            return Some(id);
        }
        pending.extend(document.children(id));
    }
    None
}

#[test]
fn deeply_nested_divs_parse() {
    let depth = on_small_stack(|| {
        let html = format!("<body>{}x{}", "<div>".repeat(DEPTH), "</div>".repeat(DEPTH));
        let document = parse(&html).document;
        let body = find_element(&document, "body").expect("body");
        div_chain_below(&document, body)
    });
    assert_eq!(depth, DEPTH);
}

/// Popping the selected `option` clones its children into `selectedcontent`
/// (form-elements §4.10.10) — both the search for `selectedcontent` and the
/// deep clone used to recurse per level.
#[test]
fn deep_option_content_is_cloned_into_selectedcontent() {
    let depth = on_small_stack(|| {
        let html = format!(
            "<select><button><selectedcontent></selectedcontent></button>\
             <option>{}x{}</option></select>",
            "<div>".repeat(DEPTH),
            "</div>".repeat(DEPTH)
        );
        let document = parse(&html).document;
        let selectedcontent = find_element(&document, "selectedcontent").expect("selectedcontent");
        div_chain_below(&document, selectedcontent)
    });
    assert_eq!(depth, DEPTH);
}
