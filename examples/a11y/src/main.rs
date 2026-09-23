//! Erzeugt die Beispielausgaben der Projektseite.
//!
//! Jedes Dokument wird als HTML-Ansicht geschrieben (aus dem Baum selbst
//! serialisiert) und dann zweimal geprüft: einmal nur mit Struktur, wie ein
//! Host ohne Semantik, und einmal mit Rolle und Accessible Name aus `accname`.
//! Dazu kommt `rules.json`, der deklarierte Regelbestand.
//!
//! Aufruf aus der Wurzel des Repositorys:
//! `cargo run --manifest-path examples/Cargo.toml`

use std::fs;
use std::path::Path;

use a11y_dom::{Arena, ArenaNode, Document, Node, NodeKind, Semantics};
use a11y_rules::{rendering_metas, run, run_with_semantics, semantics_metas, structure_metas, Meta};
use accname::IdIndex;
use serde_json::{json, Value};

/// Ein Host mit Semantik: die Arena plus Namensberechnung aus `accname`.
/// Derselbe Aufbau wie im Regeltest (`crates/a11y-rules/tests/rules.rs`).
struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl<'a> MitSemantik<'a> {
    fn new(doc: &'a Arena) -> Self {
        MitSemantik {
            ids: IdIndex::build(doc.root()),
            doc,
        }
    }
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

/// Eine Teaser-Seite mit typischen Fehlern.
fn teaser() -> Arena {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Neuheiten")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .open("body")
        .open("header")
        .open("nav")
        .open("a")
        .attr("href", "/")
        .text("Start")
        .close()
        .close()
        .close()
        .open("main")
        .open("h1")
        .text("Neuheiten")
        .close()
        .open("h3")
        .text("Regenjacke")
        .close()
        .open("img")
        .attr("src", "jacke.jpg")
        .close()
        .open("a")
        .attr("href", "/jacke")
        .text("mehr")
        .close()
        .open("h3")
        .text("Wanderhose")
        .close()
        .open("img")
        .attr("src", "hose.jpg")
        .attr("alt", "hose.jpg")
        .close()
        .open("a")
        .attr("href", "/hose")
        .text("mehr")
        .close()
        .open("form")
        .open("input")
        .attr("type", "email")
        .attr("placeholder", "E-Mail")
        .close()
        .open("button")
        .open("svg")
        .close()
        .close()
        .close()
        .close()
        .open("footer")
        .text("Impressum")
        .close()
        .close()
        .close()
        .build()
}

/// Eine vollständige Seite, wie sie der Regeltest als fehlerfrei führt.
fn vollstaendig() -> Arena {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .open("body")
        .open("a")
        .attr("href", "#inhalt")
        .attr("class", "skip-link")
        .text("Zum Inhalt springen")
        .close()
        .open("header")
        .open("nav")
        .open("a")
        .attr("href", "/")
        .text("Start")
        .close()
        .close()
        .close()
        .open("main")
        .attr("id", "inhalt")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .open("footer")
        .text("Impressum")
        .close()
        .close()
        .close()
        .build()
}

const VOID: &[&str] = &["meta", "img", "input", "br", "hr", "link"];

/// Schreibt den Baum als eingerücktes HTML — die Ansicht dessen, was geprüft wird.
fn html<'a>(n: ArenaNode<'a>, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    if n.kind() == NodeKind::Text {
        out.push_str(&format!("{pad}{}\n", n.text()));
        return;
    }
    let mut open = format!("<{}", n.local_name());
    for (k, v) in n.attributes() {
        open.push_str(&format!(" {k}=\"{v}\""));
    }
    open.push('>');
    let kids: Vec<_> = n.children().collect();
    if VOID.contains(&n.local_name()) {
        out.push_str(&format!("{pad}{open}\n"));
    } else if kids.is_empty() {
        out.push_str(&format!("{pad}{open}</{}>\n", n.local_name()));
    } else if kids.iter().all(|k| k.kind() == NodeKind::Text) {
        let text: String = kids.iter().map(|k| k.text()).collect();
        out.push_str(&format!("{pad}{open}{text}</{}>\n", n.local_name()));
    } else {
        out.push_str(&format!("{pad}{open}\n"));
        for k in kids {
            html(k, depth + 1, out);
        }
        out.push_str(&format!("{pad}</{}>\n", n.local_name()));
    }
}

fn metas_json(metas: &[Meta]) -> Vec<Value> {
    metas
        .iter()
        .flat_map(|m| {
            m.ids.iter().map(move |id| {
                json!({
                    "id": id,
                    "tier": m.tier.as_str(),
                    "wcag": m.wcag,
                    "severity": m.severity.as_str(),
                    "help": m.help,
                })
            })
        })
        .collect()
}

fn write(dir: &Path, name: &str, content: String) {
    fs::write(dir.join(name), content).expect("Beispieldatei schreiben");
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    for (slug, doc) in [("teaser", teaser()), ("vollstaendig", vollstaendig())] {
        let mut view = String::new();
        html(doc.root(), 0, &mut view);
        write(dir, &format!("{slug}.html"), view);

        let nur_struktur = run(&doc);
        write(
            dir,
            &format!("{slug}.struktur.json"),
            serde_json::to_string_pretty(&nur_struktur).unwrap() + "\n",
        );

        let host = MitSemantik::new(&doc);
        let mit_semantik = run_with_semantics(&host);
        write(
            dir,
            &format!("{slug}.semantik.json"),
            serde_json::to_string_pretty(&mit_semantik).unwrap() + "\n",
        );
    }

    let mut rules = metas_json(structure_metas());
    rules.extend(metas_json(semantics_metas()));
    rules.extend(metas_json(rendering_metas()));
    write(
        dir,
        "rules.json",
        serde_json::to_string_pretty(&rules).unwrap() + "\n",
    );
}
