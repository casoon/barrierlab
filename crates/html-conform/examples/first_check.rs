//! Start-up harness: times the first `check()` call of a fresh process, which pays for
//! getting the schema and the rule sets ready (barrierlab#72). Run it many times from a shell
//! loop and compare wall time or `/usr/bin/time -l` instructions retired.
//!
//! ```sh
//! cargo run --release --example first_check
//! ```

use std::time::Instant;

fn main() {
    let html = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>t</title></head><body><p>x</p></body></html>";
    let start = Instant::now();
    let report = html_conform::check(html).expect("check");
    let elapsed = start.elapsed();
    println!(
        "first_check_ms={:.3} findings={}",
        elapsed.as_secs_f64() * 1000.0,
        report.findings.len()
    );
}
