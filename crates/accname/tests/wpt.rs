//! Der WPT-Korpus `accname/name/*.html` (vendort nach `tests/corpus/wpt`, siehe
//! dort `UPSTREAM.md`): Jedes Element mit `data-expectedlabel` ist ein Fall.
//!
//! Der Test-Host kennt vom Stil nur `display` und `visibility` aus dem
//! `style`-Attribut und die UA-Vorgabe für `hidden` und Kopf-Elemente —
//! `<style>`-Blöcke, `::before`/`::after`, `text-transform`, Shadow-DOM aus
//! Skripten und anderes CSS sind nicht modelliert. Die daran scheiternden
//! Fälle stehen in `tests/wpt_known_failures.txt`. Der Test prüft in beide
//! Richtungen: ein unbekannter Fehlschlag und ein Eintrag, der inzwischen
//! besteht, lassen ihn scheitern. Neu erzeugen nach Prüfung jedes Eintrags mit
//! `WPT_BLESS=1 cargo test -p accname --test wpt`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use a11y_dom::{
    Arena, ArenaBuilder, ArenaNode, ComputedStyle, Document, Node, Rect, Rendering, elements,
};
use accname::{IdIndex, name_rendered};
use html5_parser::NodeKind as H;

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/wpt");
const KNOWN_FAILURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/wpt_known_failures.txt");
const HEADER_END: &str = "# --- entries below, one per line ---\n";

struct Host<'d>(&'d Arena);

impl Document for Host<'_> {
    type N<'a>
        = ArenaNode<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

/// Der Wert einer Eigenschaft im `style`-Attribut des Elements.
fn inline_style(n: ArenaNode<'_>, prop: &str) -> Option<String> {
    n.attr("style")?.split(';').rev().find_map(|d| {
        let (k, v) = d.split_once(':')?;
        (k.trim().eq_ignore_ascii_case(prop)).then(|| v.trim().to_ascii_lowercase())
    })
}

fn ua_display(n: ArenaNode<'_>) -> &'static str {
    if n.has_attr("hidden") {
        return "none";
    }
    match n.local_name() {
        "head" | "title" | "style" | "script" | "meta" | "link" | "template" | "datalist" => "none",
        "a" | "span" | "strong" | "em" | "b" | "i" | "u" | "s" | "code" | "abbr" | "small"
        | "sub" | "sup" | "mark" | "q" | "cite" | "label" | "time" | "kbd" | "var" | "bdi"
        | "bdo" | "dfn" | "data" | "img" | "svg" | "br" | "wbr" | "input" | "button" | "select"
        | "textarea" | "output" | "iframe" | "embed" | "object" | "video" | "canvas" | "audio"
        | "picture" | "meter" | "progress" => "inline",
        _ => "block",
    }
}

impl Rendering for Host<'_> {
    fn computed_style<'n>(&'n self, node: Self::N<'n>) -> Option<ComputedStyle> {
        Some(ComputedStyle {
            display: Some(inline_style(node, "display").unwrap_or_else(|| ua_display(node).into())),
            visibility: Some(inline_style(node, "visibility").unwrap_or_else(|| "visible".into())),
            ..Default::default()
        })
    }

    fn bounds<'n>(&'n self, _node: Self::N<'n>) -> Option<Rect> {
        None
    }
}

fn uebernehmen(
    doc: &html5_parser::Document,
    id: html5_parser::NodeId,
    b: ArenaBuilder,
) -> ArenaBuilder {
    match &doc.node(id).kind {
        H::Element {
            name, attributes, ..
        } => {
            let mut b = b.open(name);
            for a in attributes {
                b = b.attr(&a.name, &a.value);
            }
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b.close()
        }
        H::DocumentFragment => {
            let mut b = b;
            for k in doc.children(id) {
                b = uebernehmen(doc, k, b);
            }
            b
        }
        H::Text { content } => b.text(content),
        _ => b,
    }
}

fn html_dateien(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut eintraege: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| {
            panic!(
                "{}: {e} — xtask/vendor-wpt-accname.sh ausführen",
                dir.display()
            )
        })
        .map(|e| e.unwrap().path())
        .collect();
    eintraege.sort();
    for p in eintraege {
        if p.is_dir() {
            html_dateien(&p, out);
        } else if p.extension().is_some_and(|e| e == "html") {
            out.push(p);
        }
    }
}

fn bekannte() -> HashSet<String> {
    fs::read_to_string(KNOWN_FAILURES)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn wpt_accname_name() {
    let mut dateien = Vec::new();
    html_dateien(Path::new(CORPUS), &mut dateien);

    let mut faelle = 0usize;
    // (Schlüssel, Testname, Detail)
    let mut fehlschlaege: Vec<(String, String, String)> = Vec::new();
    for pfad in &dateien {
        let rel = pfad
            .strip_prefix(CORPUS)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let quelle = fs::read_to_string(pfad).unwrap();
        let geparst = html5_parser::parse(&quelle).document;
        let wurzel = geparst
            .children(geparst.root())
            .find(|&c| matches!(geparst.node(c).kind, H::Element { .. }))
            .expect("Dokument ohne Wurzelelement");
        let arena = uebernehmen(&geparst, wurzel, Arena::builder()).build();
        let host = Host(&arena);
        let ids = IdIndex::build(arena.root());
        for (i, n) in elements(&arena)
            .filter(|n| n.has_attr("data-expectedlabel"))
            .enumerate()
        {
            faelle += 1;
            let erwartet = n.attr("data-expectedlabel").unwrap();
            let ist = name_rendered(&host, n, &ids).unwrap_or_default();
            if ist != erwartet {
                let testname = n.attr("data-testname").unwrap_or("").replace('\n', " ");
                fehlschlaege.push((
                    format!("{rel}#{i}"),
                    testname.clone(),
                    format!("{rel}#{i} {testname}: erwartet {erwartet:?}, erhalten {ist:?}"),
                ));
            }
        }
    }
    assert!(faelle > 0, "keine Fälle im Korpus gefunden");

    if std::env::var_os("WPT_BLESS").is_some() {
        fehlschlaege.sort();
        let alt = fs::read_to_string(KNOWN_FAILURES).unwrap_or_default();
        let kopf_ende = alt
            .find(HEADER_END)
            .unwrap_or_else(|| panic!("Kopf von {KNOWN_FAILURES} ohne Markierungszeile"))
            + HEADER_END.len();
        let mut out = alt[..kopf_ende].to_owned();
        out.push('\n');
        for (k, t, _) in &fehlschlaege {
            out.push_str(&format!("{k}  {t}\n"));
        }
        fs::write(KNOWN_FAILURES, out).unwrap();
        println!(
            "WPT_BLESS: {} von {faelle} Fällen eingetragen",
            fehlschlaege.len()
        );
        return;
    }

    let bekannt = bekannte();
    let schluessel: HashSet<&str> = fehlschlaege.iter().map(|(k, _, _)| k.as_str()).collect();
    let neu: Vec<&str> = fehlschlaege
        .iter()
        .filter(|(k, _, _)| !bekannt.contains(k))
        .map(|(_, _, d)| d.as_str())
        .collect();
    let veraltet: Vec<&String> = bekannt
        .iter()
        .filter(|k| !schluessel.contains(k.as_str()))
        .collect();
    assert!(
        neu.is_empty(),
        "{} Fall/Fälle nicht in tests/wpt_known_failures.txt:\n{}",
        neu.len(),
        neu.join("\n")
    );
    assert!(
        veraltet.is_empty(),
        "tests/wpt_known_failures.txt führt {} bestehende Fälle: {veraltet:?} — mit \
         `WPT_BLESS=1 cargo test -p accname --test wpt` neu erzeugen",
        veraltet.len()
    );
}
