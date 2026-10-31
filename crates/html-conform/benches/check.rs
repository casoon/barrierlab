//! Benchmarks for the whole-pipeline `check()` entry point, using
//! representative fixtures already vendored under `tests/corpus/`
//! (read-only reference — see `CLAUDE.md` on not hand-editing vendored
//! corpus content), plus whatever real-world pages
//! `xtask/fetch-real-world.sh` has cached locally. The corpus files are
//! small; real generated sites have 500 to 10,000 tags per page, and that
//! is where schema validation cost shows (barrierlab#65). Without a cache
//! the real-world group is skipped.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::Path;

const SMALL_TYPICAL: &str = include_str!("../tests/corpus/html/elements/dl/dl-isvalid.html");
const LARGE_TABLE_HEAVY: &str = include_str!(
    "../tests/corpus/html/elements/table/integrity/Naser_al-Din_Shah_Qajar-novalid.html"
);

fn bench_check(c: &mut Criterion) {
    c.bench_function("check/small_typical", |b| {
        b.iter(|| html_conform::check(black_box(SMALL_TYPICAL)))
    });
    c.bench_function("check/large_table_heavy", |b| {
        b.iter(|| html_conform::check(black_box(LARGE_TABLE_HEAVY)))
    });
}

fn bench_real_world(c: &mut Criterion) {
    let cache = Path::new(env!("CARGO_MANIFEST_DIR")).join("xtask/.cache/real-world");
    let Ok(entries) = std::fs::read_dir(&cache) else {
        return;
    };
    let mut pages: Vec<_> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "html"))
        .collect();
    pages.sort();

    let mut group = c.benchmark_group("check/real_world");
    group.sample_size(10);
    for path in pages {
        let Ok(html) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        group.bench_function(name.as_ref(), |b| {
            b.iter(|| html_conform::check(black_box(&html)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_check, bench_real_world);
criterion_main!(benches);
