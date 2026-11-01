//! Generates the example outputs for the project site.
//!
//! Each document is written as an HTML view (serialized from the tree itself)
//! and then checked twice: once with structure only, like a host without
//! semantics, and once with role and accessible name from `accname`.
//! In addition there is `rules.json`, the declared rule set, and, from the same
//! set, the tables of the rule reference (`docs/a11y/reference/regeln.mdx`,
//! between the markers `generiert:anfang` and `generiert:ende`).
//!
//! Run from the repository root:
//! `cargo run --manifest-path examples/a11y/Cargo.toml`

use std::fs;
use std::path::Path;

use a11y_dom::{Arena, ArenaNode, Document, Node, NodeKind, Semantics};
use a11y_rules::{
    rendering_metas, run, run_with_semantics, semantics_metas, structure_metas, stylesheet_metas,
    Meta,
};
use accname::IdIndex;
use serde_json::{json, Value};

/// A host with semantics: the arena plus name computation from `accname`.
/// Same setup as in the rule test (`crates/a11y-rules/tests/rules.rs`).
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

/// A teaser page with typical errors.
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

/// A complete page, as the rule test treats it as error-free.
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

/// Writes the tree as indented HTML: the view of what is being checked.
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

/// Makes a rule hint table-safe for MDX: `<tag>` becomes code, characters
/// with meaning in Markdown, MDX or a table row are escaped.
fn mdx_text(help: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in help.chars() {
        match c {
            '<' if !in_tag => {
                in_tag = true;
                out.push_str("`<");
            }
            '>' if in_tag => {
                in_tag = false;
                out.push_str(">`");
            }
            _ if in_tag => out.push(c),
            '|' | '{' | '}' | '*' | '_' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// A section of the rule reference: heading, number of IDs, table.
fn mdx_tier(out: &mut String, heading: &str, needs: &str, metas: &[Meta]) {
    let ids: usize = metas.iter().map(|m| m.ids.len()).sum();
    out.push_str(&format!("## {heading}\n\n{ids} IDs, {needs}\n\n"));
    out.push_str("| ID | WCAG | Severity | Rule hint |\n|---|---|---|---|\n");
    for m in metas {
        for id in m.ids {
            out.push_str(&format!(
                "| `{id}` | {} | {} | {} |\n",
                if m.wcag.is_empty() {
                    "–".to_string()
                } else {
                    m.wcag.join(", ")
                },
                m.severity.as_str(),
                mdx_text(m.help),
            ));
        }
    }
    out.push('\n');
}

/// The version of a11y-rules from its manifest, which this is built against.
fn rules_version() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/a11y-rules/Cargo.toml");
    let toml = fs::read_to_string(manifest).expect("read the manifest of a11y-rules");
    let zeile = toml
        .lines()
        .find(|l| l.starts_with("version = "))
        .expect("version in crates/a11y-rules/Cargo.toml");
    zeile
        .trim_start_matches("version = ")
        .trim_matches('"')
        .to_string()
}

/// Replaces everything between the two markers in the rule reference.
fn regelreferenz(path: &Path) {
    const ANFANG: &str = "{/* generiert:anfang */}\n";
    const ENDE: &str = "{/* generiert:ende */}";
    let alt = fs::read_to_string(path).expect("read the rule reference");
    let a = alt.find(ANFANG).expect("marker generiert:anfang") + ANFANG.len();
    let e = alt.find(ENDE).expect("marker generiert:ende");

    let alle = [
        structure_metas(),
        semantics_metas(),
        rendering_metas(),
        stylesheet_metas(),
    ];
    let gesamt: usize = alle
        .iter()
        .flat_map(|t| t.iter())
        .map(|m| m.ids.len())
        .sum();
    let mut neu = format!(
        "a11y-rules {} declares {gesamt} IDs.\n\n",
        rules_version()
    );
    mdx_tier(
        &mut neu,
        "Tier 1: Structure",
        "`Document`, runs on every host.",
        alle[0],
    );
    mdx_tier(
        &mut neu,
        "Tier 2: Semantics",
        "needs `Semantics`, i.e. role and accessible name.",
        alle[1],
    );
    mdx_tier(
        &mut neu,
        "Tier 3: Rendering",
        "needs `Rendering`, i.e. computed styles and geometry.",
        alle[2],
    );
    mdx_tier(
        &mut neu,
        "Stylesheets",
        "needs the page's parsed stylesheets and runs via `run_stylesheets`.",
        alle[3],
    );

    fs::write(path, format!("{}{neu}{}", &alt[..a], &alt[e..])).expect("write the rule reference");
}

fn write(dir: &Path, name: &str, content: String) {
    fs::write(dir.join(name), content).expect("write the example file");
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
    rules.extend(metas_json(stylesheet_metas()));
    write(
        dir,
        "rules.json",
        serde_json::to_string_pretty(&rules).unwrap() + "\n",
    );

    regelreferenz(&dir.join("../../docs/a11y/reference/regeln.mdx"));
}
