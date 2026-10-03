//! Lauf über echte Stylesheets, nicht im Repository.
//!
//! ```sh
//! CSS_CORPUS=/pfad/zu/css cargo test -p stylesheet-parse --release --test corpus -- --ignored --nocapture
//! ```
//!
//! Liest jede `*.css` unter `CSS_CORPUS` (rekursiv) und prüft: kein Panic,
//! Regelzahlen, Laufzeit. Gibt je Datei eine Zeile aus.

use std::path::{Path, PathBuf};
use std::time::Instant;

use stylesheet_parse::parse_stylesheet;

fn css_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            css_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "css") {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "braucht CSS_CORPUS"]
fn real_corpus() {
    let dir = std::env::var("CSS_CORPUS").expect("CSS_CORPUS setzen");
    let mut files = Vec::new();
    css_files(Path::new(&dir), &mut files);
    files.sort();
    let (mut bytes, mut rules, mut kf, mut focus_none, mut reduced) = (0, 0, 0, 0, 0);
    let all = Instant::now();
    for f in &files {
        let raw = std::fs::read(f).unwrap();
        let css = String::from_utf8_lossy(&raw);
        let t = Instant::now();
        let sheet = parse_stylesheet(&css);
        let styles = sheet.style_rules();
        let keyframes = sheet.keyframes();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let fnone = styles
            .iter()
            .filter(|s| s.selectors.iter().any(|x| x.contains(":focus")))
            .filter(|s| {
                s.declarations.iter().any(|d| {
                    (d.name == "outline" || d.name == "outline-style")
                        && (d.value == "none" || d.value == "0")
                })
            })
            .count();
        let red = styles
            .iter()
            .filter(|s| {
                s.context
                    .media
                    .iter()
                    .any(|m| m.contains("prefers-reduced-motion"))
            })
            .count();
        println!(
            "{:>9} B {:>6} style {:>4} kf {:>4} focus-none {:>4} reduced {:>8.2} ms  {}",
            raw.len(),
            styles.len(),
            keyframes.len(),
            fnone,
            red,
            ms,
            f.strip_prefix(&dir).unwrap_or(f).display()
        );
        bytes += raw.len();
        rules += styles.len();
        kf += keyframes.len();
        focus_none += fnone;
        reduced += red;
    }
    println!(
        "{} Dateien, {bytes} B, {rules} Stilregeln, {kf} keyframes, {focus_none} :focus ohne outline, \
         {reduced} Regeln unter prefers-reduced-motion, {:.0} ms gesamt",
        files.len(),
        all.elapsed().as_secs_f64() * 1000.0
    );
}
