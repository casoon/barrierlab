//! Der Ebenenbaum (CSS Cascade 5 §6.4): benannte, verschachtelte und
//! anonyme `@layer`.
//!
//! Die Reihenfolge ergibt sich aus dem ersten Auftreten eines Namens, in
//! Dokumentreihenfolge über alle Blätter, gleich ob als Anweisung
//! (`@layer a, b.c;`) oder als Block. Unter einer Ebene stehen ihre
//! Unterebenen; die Regeln der Ebene selbst zählen wie eine letzte,
//! implizite Unterebene und schlagen deshalb deren normale Angaben (§6.4.2).
//!
//! Eine Ebene, die nur in einer Bedingung mit unbekanntem Ausgang
//! (`@media`, `@supports` …) zum ersten Mal auftaucht, hat keine sichere
//! Stelle in der Reihenfolge; ihre Regeln gelten als unbestimmt.

use std::collections::HashMap;

use stylesheet_parse::{Rule as CssRule, Stylesheet};

use crate::media::{Environment, eval};
use crate::selector::Match;

/// Wohin ein `@layer`-Block führt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Block {
    Node(usize),
    /// Kein gültiger Ebenenname: der Browser verwirft den Block. Wir lesen
    /// ihn nicht sicher genug, um das zu behaupten.
    Invalid,
}

struct Node {
    children: Vec<usize>,
    by_name: HashMap<String, usize>,
    parent: usize,
    /// Stelle unter dem Elternknoten.
    pos: u32,
    unsure: bool,
}

pub(crate) struct LayerTree {
    nodes: Vec<Node>,
    /// Block-Regel (Zeiger auf ihren Namen im Stylesheet) → Ebene.
    blocks: HashMap<*const u8, Block>,
}

const ROOT: usize = 0;

impl LayerTree {
    pub(crate) fn build(sheets: &[Stylesheet], env: &Environment) -> Self {
        let mut t = LayerTree {
            nodes: vec![Node {
                children: Vec::new(),
                by_name: HashMap::new(),
                parent: ROOT,
                pos: 0,
                unsure: false,
            }],
            blocks: HashMap::new(),
        };
        for s in sheets {
            t.walk(&s.rules, ROOT, Match::Yes, env);
        }
        t
    }

    fn child(&mut self, parent: usize, name: Option<&str>, cond: Match) -> usize {
        if let Some(n) = name
            && let Some(&i) = self.nodes[parent].by_name.get(n)
        {
            return i;
        }
        let i = self.nodes.len();
        let pos = self.nodes[parent].children.len() as u32;
        let unsure = self.nodes[parent].unsure || cond != Match::Yes;
        self.nodes.push(Node {
            children: Vec::new(),
            by_name: HashMap::new(),
            parent,
            pos,
            unsure,
        });
        self.nodes[parent].children.push(i);
        if let Some(n) = name {
            self.nodes[parent].by_name.insert(n.to_string(), i);
        }
        i
    }

    fn path(&mut self, parent: usize, segs: &[&str], cond: Match) -> usize {
        segs.iter()
            .fold(parent, |p, s| self.child(p, Some(s), cond))
    }

    fn walk(&mut self, rules: &[CssRule], parent: usize, cond: Match, env: &Environment) {
        if cond == Match::No {
            return;
        }
        for r in rules {
            match r {
                CssRule::Other(a) if a.name == "layer" => {
                    for n in a.prelude.split(',') {
                        if let Some(segs) = layer_name(n) {
                            self.path(parent, &segs, cond);
                        }
                    }
                }
                CssRule::Group(g) if g.name == "layer" => {
                    let p = g.prelude.trim();
                    let block = if p.is_empty() {
                        Block::Node(self.child(parent, None, cond))
                    } else {
                        match layer_name(p) {
                            Some(segs) => Block::Node(self.path(parent, &segs, cond)),
                            None => Block::Invalid,
                        }
                    };
                    self.blocks.insert(g.name.as_ptr(), block);
                    if let Block::Node(n) = block {
                        self.walk(&g.rules, n, cond, env);
                    }
                }
                CssRule::Group(g) => {
                    let c = cond.und(gruppe(&g.name, &g.prelude));
                    self.walk(&g.rules, parent, c, env);
                }
                CssRule::Media(m) => {
                    let c = cond.und(eval(&m.condition, env));
                    self.walk(&m.rules, parent, c, env);
                }
                CssRule::Style(s) => self.walk(&s.nested, parent, cond, env),
                _ => {}
            }
        }
    }

    /// Die Ebene einer Regel, gegeben die `@layer`-Gruppen um sie herum
    /// (von außen nach innen, wie in `Context::groups`).
    /// `Err(())`: nicht bestimmbar.
    pub(crate) fn node_of(&self, groups: &[(&str, &str)]) -> Result<Option<usize>, ()> {
        let mut node = None;
        for (name, _) in groups.iter().filter(|(n, _)| *n == "layer") {
            match self.blocks.get(&name.as_ptr()) {
                Some(Block::Node(i)) => node = Some(*i),
                _ => return Err(()),
            }
        }
        Ok(node)
    }

    pub(crate) fn is_unsure(&self, node: usize) -> bool {
        self.nodes[node].unsure
    }

    /// Sortierschlüssel: die Stellen von der Wurzel bis zur Ebene, dann ein
    /// Endstück, das die Regeln der Ebene hinter ihre Unterebenen setzt.
    pub(crate) fn key(&self, mut node: usize) -> Vec<u32> {
        let mut k = Vec::new();
        while node != ROOT {
            k.push(self.nodes[node].pos);
            node = self.nodes[node].parent;
        }
        k.reverse();
        k.push(u32::MAX);
        k
    }
}

/// Bedingung einer Gruppenregel außer `@layer`; dieselbe Auslegung wie in
/// der Kaskade.
pub(crate) fn gruppe(name: &str, prelude: &str) -> Match {
    match name {
        // Eine Bedingung für Funktionen, die moderne Browser kennen; nur
        // das Negierte (Ersatz für alte Browser) greift nicht.
        "supports" => {
            if prelude.trim_start().to_ascii_lowercase().starts_with("not") {
                Match::No
            } else {
                Match::Yes
            }
        }
        "starting-style" => Match::No,
        _ => Match::Unknown,
    }
}

/// `<layer-name>` = `<ident> ['.' <ident>]*`, ohne Leerraum dazwischen.
fn layer_name(s: &str) -> Option<Vec<&str>> {
    let s = s.trim();
    let segs: Vec<&str> = s.split('.').collect();
    segs.iter().all(|g| ident(g)).then_some(segs)
}

fn ident(g: &str) -> bool {
    let mut c = g.chars();
    let Some(first) = c.next() else { return false };
    let start = |c: char| c.is_ascii_alphabetic() || c == '_' || !c.is_ascii() || c == '\\';
    let ok = |c: char| start(c) || c.is_ascii_digit() || c == '-';
    let first_ok =
        start(first) || (first == '-' && g.chars().nth(1).is_some_and(|n| start(n) || n == '-'));
    first_ok && g.chars().all(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namen() {
        assert_eq!(layer_name("a"), Some(vec!["a"]));
        assert_eq!(layer_name(" a.b-c "), Some(vec!["a", "b-c"]));
        assert_eq!(layer_name("a..b"), None);
        assert_eq!(layer_name("a."), None);
        assert_eq!(layer_name("1a"), None);
        assert_eq!(layer_name("a b"), None);
        assert_eq!(layer_name(""), None);
    }
}
