//! Throwaway harness for barrierlab#70: schema-phase timing plus allocation
//! counts over the cached real-world pages. Usage: attrbench [samples]
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size(), Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(n, Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static A: Counting = Counting;

fn main() {
    let samples: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("xtask/.cache/real-world");
    let mut pages: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .collect();
    pages.sort();
    for path in pages {
        let html = std::fs::read_to_string(&path).unwrap();
        html_conform::__bench_schema_only(&html);
        let a0 = ALLOCS.load(Relaxed);
        let b0 = BYTES.load(Relaxed);
        let mut times = Vec::new();
        for _ in 0..samples {
            let t = Instant::now();
            std::hint::black_box(html_conform::__bench_schema_only(std::hint::black_box(&html)));
            times.push(t.elapsed().as_secs_f64() * 1e3);
        }
        let allocs = (ALLOCS.load(Relaxed) - a0) / samples;
        let bytes = (BYTES.load(Relaxed) - b0) / samples;
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "{}\t{:.3}\t{:.3}\t{}\t{}",
            path.file_stem().unwrap().to_string_lossy(),
            times[0],
            times[times.len() / 2],
            allocs,
            bytes
        );
    }
}
