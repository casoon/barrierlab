//! Measures peak heap and time of the tree path (`Schema::validate`) against
//! the push path (`StreamValidator`) on one generated document
//! (barrierlab#77).
//!
//! Run with `cargo run --release --example stream_vs_tree -- 1000000`
//! (argument: number of `item` elements, default 200000). Each path runs in
//! its own process so the peak of one does not carry into the other:
//! `... -- 1000000 tree` or `... -- 1000000 stream`; without the mode both
//! run one after the other in this process and the peak is reported per run
//! from a reset counter.
//!
//! The tree is the cheapest one a caller can hand over: nodes are shared
//! behind an `Rc`, `children()` clones only a pointer. It still has to
//! exist in full; the stream path never holds more than the open elements.

use relax_ng::{
    Content, DatatypeRegistry, Element, ExpandedName, ResolveError, Schema, SchemaResolver,
    SchemaSource, SchemaSyntax,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call unchanged to `System`; only bookkeeping on
// atomics is added.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            if new_size >= layout.size() {
                let now = CURRENT.fetch_add(new_size - layout.size(), Ordering::Relaxed)
                    + (new_size - layout.size());
                PEAK.fetch_max(now, Ordering::Relaxed);
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        new
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

struct NoResolver;
impl SchemaResolver for NoResolver {
    fn resolve(&self, href: &str, _base: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("no such resource: {href}")))
    }
}

struct Node {
    name: &'static str,
    attributes: Vec<(&'static str, String)>,
    children: Vec<Doc>,
    text: Option<String>,
}

#[derive(Clone)]
struct Doc(Rc<Node>);

impl Element for Doc {
    type Location = ();
    fn name(&self) -> ExpandedName {
        ExpandedName::new(None::<String>, self.0.name)
    }
    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
        self.0
            .attributes
            .iter()
            .map(|(n, v)| (ExpandedName::new(None::<String>, *n), v.clone()))
    }
    fn children(&self) -> impl Iterator<Item = Content<Self>> {
        self.0
            .children
            .iter()
            .cloned()
            .map(Content::Element)
            .chain(self.0.text.clone().map(Content::Text))
    }
}

fn name(local: &str) -> ExpandedName {
    ExpandedName::new(None::<String>, local)
}

fn measure<T>(label: &str, run: impl FnOnce() -> T) -> T {
    let base = CURRENT.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let start = Instant::now();
    let out = run();
    let elapsed = start.elapsed();
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    println!(
        "{label:<7} {:>8.1} MiB peak heap  {:>8.0} ms",
        peak as f64 / (1024.0 * 1024.0),
        elapsed.as_secs_f64() * 1000.0
    );
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let items: usize = args
        .next()
        .map_or(200_000, |a| a.parse().expect("item count"));
    let mode = args.next();

    let source = SchemaSource::new(
        "element list { element item { attribute id { text }, text }* }",
        "memory:/list.rnc",
        SchemaSyntax::Compact,
    );
    let schema = Schema::compile(&source, &NoResolver).expect("schema compiles");
    let registry = DatatypeRegistry::new();
    println!("{items} item elements");

    if mode.as_deref() != Some("stream") {
        let findings = measure("tree", || {
            // Building the tree is part of what the tree path costs.
            let root = Doc(Rc::new(Node {
                name: "list",
                attributes: Vec::new(),
                children: (0..items)
                    .map(|i| {
                        Doc(Rc::new(Node {
                            name: "item",
                            attributes: vec![("id", i.to_string())],
                            children: Vec::new(),
                            text: Some(format!("entry {i}")),
                        }))
                    })
                    .collect(),
                text: None,
            }));
            schema.validate(&registry, &root).expect("datatypes known")
        });
        assert!(findings.is_empty());
    }

    if mode.as_deref() != Some("tree") {
        let findings = measure("stream", || {
            let mut v = schema
                .stream_validator::<()>(&registry)
                .expect("datatypes known");
            v.start_element(name("list"), [], [], None).unwrap();
            for i in 0..items {
                v.start_element(name("item"), [(name("id"), i.to_string())], [], None)
                    .unwrap();
                v.text(&format!("entry {i}")).unwrap();
                v.end_element().unwrap();
            }
            v.end_element().unwrap();
            v.finish().unwrap()
        });
        assert!(findings.is_empty());
    }
}
