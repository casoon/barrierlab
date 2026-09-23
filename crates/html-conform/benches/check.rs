//! Benchmarks for the whole-pipeline `check()` entry point, using
//! representative fixtures already vendored under `tests/corpus/`
//! (read-only reference — see `CLAUDE.md` on not hand-editing vendored
//! corpus content).

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

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

criterion_group!(benches, bench_check);
criterion_main!(benches);
