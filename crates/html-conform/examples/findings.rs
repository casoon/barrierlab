//! Checks every `examples/showcase/*.html` file with `html_conform::check` and writes the
//! report next to it as `<name>.json`. The showcase on the project website renders these
//! files, so every finding it shows was produced by this crate.
//!
//! ```sh
//! cargo run --example findings
//! ```

use std::error::Error;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/showcase");
    let mut inputs = fs::read_dir(&dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    inputs.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "html")
    });
    inputs.sort();

    for input in inputs {
        let html = fs::read_to_string(&input)?;
        let report = html_conform::check(&html)?;
        let output = input.with_extension("json");
        fs::write(&output, serde_json::to_string_pretty(&report)? + "\n")?;
        println!(
            "{}: {} findings",
            output.strip_prefix(&dir)?.display(),
            report.findings.len()
        );
    }
    Ok(())
}
