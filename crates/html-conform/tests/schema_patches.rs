//! Offline part of the vendoring reproducibility check (barrierlab#180): every
//! patch in `xtask/schema-patches/` targets an existing module and its
//! post-image (the `+` lines) is present in the committed file, in order. The
//! full check against upstream is `xtask/vendor-schema.sh --check` (network).

use std::fs;
use std::path::Path;

#[test]
fn schema_patches_match_committed_tree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut patches: Vec<_> = fs::read_dir(root.join("xtask/schema-patches"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "patch"))
        .collect();
    patches.sort();
    assert!(!patches.is_empty(), "no schema patches found");

    for patch in patches {
        let text = fs::read_to_string(&patch).unwrap();
        let mut target: Option<String> = None;
        let mut committed = String::new();
        let mut pos = 0;
        for line in text.lines() {
            if let Some(path) = line.strip_prefix("+++ b/") {
                let file = root.join("schema/html5").join(path);
                assert!(file.is_file(), "{}: target {path} missing", patch.display());
                committed = fs::read_to_string(file).unwrap();
                target = Some(path.to_string());
                pos = 0;
            } else if line.starts_with("+++") || line.starts_with('#') {
                continue;
            } else if let Some(added) = line.strip_prefix('+') {
                let name = target.as_deref().expect("hunk before +++ header");
                let found = committed[pos..]
                    .find(&format!("{added}\n"))
                    .unwrap_or_else(|| {
                        panic!(
                            "{}: added line not in committed {name}: {added:?}",
                            patch.display()
                        )
                    });
                pos += found + added.len() + 1;
            }
        }
        assert!(target.is_some(), "{}: no target file", patch.display());
    }
}
