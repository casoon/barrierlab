//! Ein Stylesheet-Parser nach CSS Syntax Level 3 §5 („Parsing") mit
//! CSS Nesting, in sicherem Rust.
//!
//! Das Paket nimmt rohen CSS-Text entgegen — aus den Stylesheets eines
//! Browsers, aus `dist/*.css`, aus `<style>` — und liefert Regeln mit
//! serialisierten Präludien und Deklarationen. Es holt nichts, es bewertet
//! nichts. Tokenisiert wird mit dem Tokenizer aus `media-query-parse`.
//!
//! Fehlerbehandlung wie in der Spezifikation: Eine ungültige Deklaration
//! fällt bis zum nächsten `;` derselben Ebene weg, offene Blöcke schließt das
//! Dateiende, `<!--`/`-->` auf oberster Ebene zählen nicht. Keine Eingabe
//! führt zu einem Panic.
//!
//! # Beispiel
//!
//! ```
//! use css_parse::parse_stylesheet;
//!
//! let sheet = parse_stylesheet(
//!     "@media (prefers-reduced-motion: reduce) { .a { animation: none !important } }
//!      .b { &:focus { outline: none } }",
//! );
//! let rules = sheet.style_rules();
//! assert_eq!(rules[0].selectors, [".a"]);
//! assert_eq!(rules[0].context.media, ["(prefers-reduced-motion: reduce)"]);
//! assert!(rules[0].declarations[0].important);
//! assert_eq!(rules[2].selectors, [".b:focus"]);
//! assert_eq!(rules[2].declarations[0].value, "none");
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod parser;
mod serialize;

use std::ops::Range;

use media_query_parse::tokenizer::{Token, tokenize};

use crate::parser::{Block, Parser, Raw, components, contains_bad_tokens};
use crate::serialize::{Writer, serialize};

/// Ein geparstes Stylesheet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stylesheet {
    /// Die Regeln der obersten Ebene, in Quellreihenfolge.
    pub rules: Vec<Rule>,
}

/// Eine Regel.
#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// Eine Stilregel (`a, b { … }`).
    Style(StyleRule),
    /// `@media`.
    Media(MediaRule),
    /// `@keyframes`, auch mit Herstellerpräfix (`@-webkit-keyframes`).
    Keyframes(KeyframesRule),
    /// Block-At-Regeln, die Regeln enthalten: `@supports`, `@layer { … }`,
    /// `@container`, `@scope`, `@document`, `@-moz-document`,
    /// `@starting-style`.
    Group(GroupRule),
    /// Jede andere At-Regel: `@import`, `@font-face`, `@charset`,
    /// `@layer a, b;`, `@page` … sowie die obigen ohne Block.
    Other(AtRule),
}

/// Eine Stilregel.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    /// Die Selektoren, an Kommas der obersten Ebene getrennt. Verschachtelte
    /// Regeln tragen sie relativ (`&:hover`, `.kind`), aufgelöst werden sie
    /// erst in [`Stylesheet::style_rules`].
    pub selectors: Vec<String>,
    /// Die Deklarationen.
    pub declarations: Vec<Declaration>,
    /// Verschachtelte Regeln (CSS Nesting): Stilregeln sowie `@media` und
    /// Gruppenregeln. Deklarationen direkt in einer solchen verschachtelten
    /// At-Regel stehen dort als Stilregel mit dem Selektor `&`.
    pub nested: Vec<Rule>,
}

/// `@media`.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRule {
    /// Die Bedingung, serialisiert — zum Auswerten an
    /// `media_query_parse::parse_media_query_list` weiterreichen.
    pub condition: String,
    /// Die enthaltenen Regeln.
    pub rules: Vec<Rule>,
}

/// `@keyframes`.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframesRule {
    /// Der Name, in der Schreibung der Quelle; bei einem String-Namen dessen
    /// Inhalt.
    pub name: String,
    /// Das Herstellerpräfix ohne Bindestriche (`webkit` bei
    /// `@-webkit-keyframes`), sonst `None`.
    pub vendor: Option<String>,
    /// Die Keyframes.
    pub frames: Vec<Keyframe>,
}

/// Ein Keyframe in `@keyframes`.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    /// Die Keyframe-Selektoren (`from`, `50%` …), klein geschrieben.
    pub selectors: Vec<String>,
    /// Die Deklarationen.
    pub declarations: Vec<Declaration>,
}

/// Eine Block-At-Regel, die Regeln enthält (siehe [`Rule::Group`]).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRule {
    /// Der Name ohne `@`, klein geschrieben.
    pub name: String,
    /// Das Präludium, serialisiert.
    pub prelude: String,
    /// Die enthaltenen Regeln.
    pub rules: Vec<Rule>,
}

/// Jede andere At-Regel (siehe [`Rule::Other`]).
#[derive(Debug, Clone, PartialEq)]
pub struct AtRule {
    /// Der Name ohne `@`, klein geschrieben.
    pub name: String,
    /// Das Präludium, serialisiert.
    pub prelude: String,
    /// Deklarationen des Blocks (`@font-face`, `@page`), sonst leer.
    /// Regeln in einem solchen Block (Randbereiche von `@page`) fehlen.
    pub declarations: Vec<Declaration>,
}

/// Eine Deklaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    /// Der Eigenschaftsname, klein geschrieben; Custom Properties (`--x`)
    /// in der Schreibung der Quelle.
    pub name: String,
    /// Der Wert, serialisiert, ohne `!important`.
    pub value: String,
    /// Stand `!important` am Ende?
    pub important: bool,
}

/// Eine Stilregel mit aufgelösten Selektoren und ihrem Kontext, aus
/// [`Stylesheet::style_rules`].
#[derive(Debug, Clone, PartialEq)]
pub struct Scoped<'a> {
    /// Die Selektoren, Verschachtelung aufgelöst.
    pub selectors: Vec<String>,
    /// Die Deklarationen der Regel.
    pub declarations: &'a [Declaration],
    /// Die umgebenden At-Regeln.
    pub context: Context<'a>,
}

/// Die At-Regeln, in denen eine Regel steht, jeweils von außen nach innen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Context<'a> {
    /// Die Bedingungen umgebender `@media`-Regeln.
    pub media: Vec<&'a str>,
    /// Umgebende Gruppenregeln als `(name, prelude)`.
    pub groups: Vec<(&'a str, &'a str)>,
}

/// Parst ein Stylesheet (CSS Syntax Level 3 §5.3.3 „parse a stylesheet").
/// Scheitert nie; was nicht zu lesen ist, fällt nach den Regeln der
/// Spezifikation weg.
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    let toks = tokenize(css);
    let raws = Parser::new(&toks).stylesheet();
    Stylesheet {
        rules: convert(&toks, raws, false),
    }
}

/// Block-At-Regeln, die Regeln enthalten.
const GROUP_NAMES: &[&str] = &[
    "supports",
    "layer",
    "container",
    "scope",
    "document",
    "-moz-document",
    "starting-style",
];

/// `keyframes` → `None`, `-webkit-keyframes` → `Some("webkit")`.
fn keyframes_vendor(name: &str) -> Option<Option<String>> {
    if name == "keyframes" {
        return Some(None);
    }
    let vendor = name.strip_prefix('-')?.strip_suffix("-keyframes")?;
    if vendor.is_empty() || vendor.contains('-') {
        return None;
    }
    Some(Some(vendor.to_owned()))
}

/// Ordnet rohe Regeln ein. `nested`: innerhalb einer Stilregel, wo nur
/// Stilregeln, `@media` und Gruppenregeln gelten (CSS Nesting §3.2).
fn convert(toks: &[Token], raws: Vec<Raw>, nested: bool) -> Vec<Rule> {
    let mut out = Vec::new();
    for raw in raws {
        match raw {
            Raw::Qualified { prelude, block } => {
                let Some(selectors) = selector_list(toks, prelude, false) else {
                    continue;
                };
                out.push(Rule::Style(StyleRule {
                    selectors,
                    declarations: block.declarations,
                    nested: convert(toks, block.rules, true),
                }));
            }
            Raw::At {
                name,
                prelude,
                block: Some(block),
            } if name == "media" || GROUP_NAMES.contains(&name.as_str()) => {
                let prelude = serialize(&toks[prelude]);
                let rules = group_body(toks, block, nested);
                out.push(if name == "media" {
                    Rule::Media(MediaRule {
                        condition: prelude,
                        rules,
                    })
                } else {
                    Rule::Group(GroupRule {
                        name,
                        prelude,
                        rules,
                    })
                });
            }
            // Andere At-Regeln sind in einer Stilregel ungültig.
            Raw::At { .. } if nested => {}
            Raw::At {
                name,
                prelude,
                block: Some(block),
            } if keyframes_vendor(&name).is_some() => {
                let vendor = keyframes_vendor(&name).flatten();
                out.push(Rule::Keyframes(KeyframesRule {
                    name: keyframes_name(toks, prelude),
                    vendor,
                    frames: keyframe_list(toks, block),
                }));
            }
            Raw::At {
                name,
                prelude,
                block,
            } => out.push(Rule::Other(AtRule {
                name,
                prelude: serialize(&toks[prelude]),
                declarations: block.map(|b| b.declarations).unwrap_or_default(),
            })),
        }
    }
    out
}

/// Inhalt von `@media` oder einer Gruppenregel. Verschachtelt gelten
/// Deklarationen direkt darin der umgebenden Stilregel (CSS Nesting §3.2);
/// sie werden zur Stilregel `&`. Auf oberster Ebene sind sie ungültig.
fn group_body(toks: &[Token], block: Block, nested: bool) -> Vec<Rule> {
    let mut rules = Vec::new();
    if nested && !block.declarations.is_empty() {
        rules.push(Rule::Style(StyleRule {
            selectors: vec!["&".to_owned()],
            declarations: block.declarations,
            nested: Vec::new(),
        }));
    }
    rules.extend(convert(toks, block.rules, nested));
    rules
}

fn keyframes_name(toks: &[Token], prelude: Range<usize>) -> String {
    let mut it = toks[prelude.clone()]
        .iter()
        .filter(|t| !matches!(t, Token::Whitespace));
    match (it.next(), it.next()) {
        (Some(Token::Ident(s) | Token::String(s)), None) => s.clone(),
        _ => serialize(&toks[prelude]),
    }
}

fn keyframe_list(toks: &[Token], block: Block) -> Vec<Keyframe> {
    block
        .rules
        .into_iter()
        .filter_map(|raw| match raw {
            Raw::Qualified { prelude, block } => Some(Keyframe {
                selectors: selector_list(toks, prelude, true)?,
                declarations: block.declarations,
            }),
            Raw::At { .. } => None,
        })
        .collect()
}

/// Trennt ein Präludium an Kommas der obersten Ebene. `None`, wenn die Liste
/// sicher ungültig ist — ein leerer Eintrag oder fehlerhafte Token, auf die
/// keine Selektorgrammatik passt; eine solche Regel verwirft auch ein
/// Browser. Weiter geht die Prüfung nicht.
fn selector_list(toks: &[Token], prelude: Range<usize>, lowercase: bool) -> Option<Vec<String>> {
    if contains_bad_tokens(&toks[prelude.clone()]) {
        return None;
    }
    let mut out = Vec::new();
    let mut start = prelude.start;
    let comps = components(toks, prelude.clone());
    let commas = comps
        .iter()
        .filter(|r| matches!(toks[r.start], Token::Comma))
        .map(|r| r.start)
        .chain(std::iter::once(prelude.end));
    for end in commas {
        let s = serialize(&toks[start..end]);
        if s.is_empty() {
            return None;
        }
        out.push(if lowercase { s.to_ascii_lowercase() } else { s });
        start = end + 1;
    }
    Some(out)
}

/// Obergrenze in Bytes für einen aufgelösten Selektor und für die
/// Selektorliste einer Regel. Verschachtelung mit mehreren `&` je Ebene
/// wächst exponentiell; was darüber liegt, fällt samt Nachkommen weg.
const MAX_SELECTOR_LEN: usize = 64 * 1024;

/// Löst einen verschachtelten Selektor gegen die Selektoren der
/// Elternregel auf (CSS Nesting §4). Ohne `&` gilt er als `& <selektor>`.
/// `&` am Anfang wird bei genau einem Elternselektor durch diesen ersetzt,
/// sonst durch `:is(<eltern>)` — eine Textersetzung mitten im Selektor
/// änderte, was er trifft (`.c &` mit `.a .b`).
fn resolve(selector: &str, parents: &[String]) -> Option<String> {
    let is_form = || format!(":is({})", parents.join(", "));
    let leading = || {
        if parents.len() == 1 {
            parents[0].clone()
        } else {
            is_form()
        }
    };
    let toks = tokenize(selector);
    let has_amp = toks.iter().any(|t| matches!(t, Token::Delim('&')));
    let resolved = if has_amp {
        let mut w = Writer::new();
        let mut first = true;
        for t in &toks {
            if matches!(t, Token::Delim('&')) {
                let text = if first { leading() } else { is_form() };
                if text.len() > MAX_SELECTOR_LEN {
                    return None;
                }
                w.raw(&text);
            } else {
                w.token(t);
            }
            if !matches!(t, Token::Whitespace) {
                first = false;
            }
            if w.len() > MAX_SELECTOR_LEN {
                return None;
            }
        }
        w.finish()
    } else {
        format!("{} {selector}", leading())
    };
    (resolved.len() <= MAX_SELECTOR_LEN).then_some(resolved)
}

impl Stylesheet {
    /// Jede Stilregel, durch `@media`, Gruppenregeln und Verschachtelung
    /// hindurch eingeebnet, mit aufgelösten Selektoren und Kontext. Reihenfolge
    /// wie in der Quelle, Elternregel vor ihren verschachtelten Regeln.
    pub fn style_rules(&self) -> Vec<Scoped<'_>> {
        self.collect().styles
    }

    /// Jede `@keyframes`-Regel, eingeebnet, mit Kontext.
    pub fn keyframes(&self) -> Vec<(&KeyframesRule, Context<'_>)> {
        self.collect().keyframes
    }

    fn collect(&self) -> Collect<'_> {
        let mut c = Collect {
            styles: Vec::new(),
            keyframes: Vec::new(),
        };
        c.walk(&self.rules, None, &Context::default());
        c
    }
}

struct Collect<'a> {
    styles: Vec<Scoped<'a>>,
    keyframes: Vec<(&'a KeyframesRule, Context<'a>)>,
}

impl<'a> Collect<'a> {
    fn walk(&mut self, rules: &'a [Rule], parents: Option<&[String]>, ctx: &Context<'a>) {
        for rule in rules {
            match rule {
                Rule::Style(style) => {
                    let selectors: Vec<String> = match parents {
                        None => style.selectors.clone(),
                        // `&` allein steht für die Elternselektoren selbst.
                        Some(p) => style
                            .selectors
                            .iter()
                            .flat_map(|s| {
                                if s == "&" {
                                    p.to_vec()
                                } else {
                                    resolve(s, p).into_iter().collect()
                                }
                            })
                            .collect(),
                    };
                    let total: usize = selectors.iter().map(String::len).sum();
                    if selectors.is_empty() || total > MAX_SELECTOR_LEN {
                        continue;
                    }
                    // Eltern vor Kindern, wie in der Quelle.
                    self.styles.push(Scoped {
                        selectors: selectors.clone(),
                        declarations: &style.declarations,
                        context: ctx.clone(),
                    });
                    self.walk(&style.nested, Some(&selectors), ctx);
                }
                Rule::Media(media) => {
                    let mut inner = ctx.clone();
                    inner.media.push(&media.condition);
                    self.walk(&media.rules, parents, &inner);
                }
                Rule::Group(group) => {
                    let mut inner = ctx.clone();
                    inner.groups.push((&group.name, &group.prelude));
                    self.walk(&group.rules, parents, &inner);
                }
                Rule::Keyframes(k) => self.keyframes.push((k, ctx.clone())),
                Rule::Other(_) => {}
            }
        }
    }
}
