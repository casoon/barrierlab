use std::collections::HashMap;

use a11y_dom::{Node, NodeKind, ancestors, elements};
use a11y_report::{InputKind, InputNote, Report, SourcePosition};
use a11y_rules::Locale;
use style_cascade::{Environment, Import};
use stylesheet_parse::{AtRule, MediaRule, Rule, Stylesheet, parse_stylesheet};

use crate::host::Host;

/// A parsed HTML document.
#[derive(Debug)]
pub struct Page {
    doc: html5_parser::Document,
}

/// A stylesheet the page references, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StylesheetRef {
    /// `<link rel=stylesheet href=…>`: the host fetches `href` and hands the
    /// text back as a [`Sheet`] with `source == href`.
    Link {
        /// The `href` attribute, as written.
        href: String,
        /// The `media` attribute, if present and not empty.
        media: Option<String>,
    },
    /// A `<style>` element; its text is already here.
    Inline {
        /// The text content.
        css: String,
        /// The `media` attribute, if present and not empty.
        media: Option<String>,
    },
}

/// A stylesheet the host fetched, or failed to fetch.
///
/// `source` is a URL **as written** in the document (or in the importing
/// sheet); this crate never resolves URLs. Resolving, fetching and size or
/// scheme policy are the host's job. Build one with [`Sheet::new`] or
/// [`Sheet::failed`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Sheet {
    /// The `href` of the [`StylesheetRef::Link`] this answers, verbatim.
    pub source: String,
    /// The stylesheet text. Ignored when `failure` is set.
    pub css: String,
    /// The `media` attribute of the `<link>` this answers, as written
    /// (compared trimmed and ASCII case-insensitively). With `None` the sheet
    /// answers any link with that `href`; an exact `href` plus `media` match
    /// wins over such a sheet.
    pub media: Option<String>,
    /// Why the host could not supply the text (HTTP 404, too large,
    /// unsupported scheme, ...). It becomes the detail of the input note.
    pub failure: Option<String>,
    /// The sheets this one imports with `@import`, for a host that loads
    /// them. Each answers the `@import` whose URL equals its `source`.
    pub imports: Vec<Sheet>,
}

impl Sheet {
    /// A fetched sheet.
    pub fn new(source: impl Into<String>, css: impl Into<String>) -> Self {
        Sheet {
            source: source.into(),
            css: css.into(),
            media: None,
            failure: None,
            imports: Vec::new(),
        }
    }

    /// A sheet the host tried to fetch and could not. `reason` ends up in the
    /// [`InputNote`] detail; contrast stays `untested`.
    pub fn failed(source: impl Into<String>, reason: impl Into<String>) -> Self {
        Sheet {
            failure: Some(reason.into()),
            ..Sheet::new(source, "")
        }
    }

    /// Restricts the sheet to links with this `media` attribute.
    pub fn with_media(mut self, media: impl Into<String>) -> Self {
        self.media = Some(media.into());
        self
    }

    /// Hands over the sheets this one imports.
    pub fn with_imports(mut self, imports: Vec<Sheet>) -> Self {
        self.imports = imports;
        self
    }
}

fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

enum Ref {
    Public(StylesheetRef),
    EmptyHref,
}

impl Page {
    /// Parses `html`. Never fails: the HTML parser recovers from any input.
    pub fn parse(html: &str) -> Page {
        Page {
            doc: html5_parser::parse(html).document,
        }
    }

    /// The stylesheets the page references, in document order.
    ///
    /// A `<link>` counts when its `rel` has the token `stylesheet` but not
    /// `alternate`, it has no `disabled` attribute and a non-empty `href`.
    /// A `<link>` that otherwise qualifies but has no usable `href` is not
    /// listed (there is nothing to fetch); [`Page::audit`] notes it.
    /// A `<style>` always counts. Elements inside `<template>` are inert and
    /// skipped. Whether a `media` condition holds is decided later, by the
    /// cascade, against the [`Environment`].
    pub fn stylesheet_refs(&self) -> Vec<StylesheetRef> {
        self.refs()
            .into_iter()
            .filter_map(|r| match r {
                Ref::Public(r) => Some(r),
                Ref::EmptyHref => None,
            })
            .collect()
    }

    fn refs(&self) -> Vec<Ref> {
        elements(&self.doc)
            .filter(|n| !ancestors(*n).any(|a| a.is_element("template")))
            .filter_map(|n| {
                let media = non_empty(n.attr("media"));
                if n.is_element("style") {
                    let css = n
                        .children()
                        .filter(|k| k.kind() == NodeKind::Text)
                        .map(|k| k.text())
                        .collect();
                    return Some(Ref::Public(StylesheetRef::Inline { css, media }));
                }
                let rel = n.attr("rel").unwrap_or("");
                let has = |t: &str| rel.split_whitespace().any(|r| r.eq_ignore_ascii_case(t));
                if n.is_element("link")
                    && has("stylesheet")
                    && !has("alternate")
                    && !n.has_attr("disabled")
                {
                    return Some(match non_empty(n.attr("href")) {
                        Some(href) => Ref::Public(StylesheetRef::Link { href, media }),
                        None => Ref::EmptyHref,
                    });
                }
                None
            })
            .collect()
    }

    /// Runs every rule over the page.
    ///
    /// Inline `<style>` texts and the supplied `sheets` form the cascade, in
    /// document order. A `<link>` is answered by the sheet with the same
    /// `href` and `media` (see [`Sheet::media`]). `@import` rules are replaced
    /// by the sheets handed over in [`Sheet::imports`] (for an inline
    /// `<style>`: by the top-level `sheets` with that `source`); imports are
    /// never fetched here. A `sheets` entry that answers nothing is ignored.
    ///
    /// The report carries an [`InputKind::StylesheetUnavailable`] note for
    /// every `<link>` without a usable sheet (the detail is the host's
    /// [`Sheet::failure`] if given) and for every `<link>` without `href`,
    /// and an [`InputKind::StylesheetUnresolved`] note for every `@import`
    /// that stays unresolved. While any such note exists the contrast rules
    /// stay `untested`.
    pub fn audit(&self, sheets: &[Sheet], env: Environment, locale: Locale) -> Report {
        let mut parsed = Vec::new();
        let mut missing: Vec<(Option<String>, String)> = Vec::new();
        let mut import_failures: Vec<(String, String)> = Vec::new();
        for r in self.refs() {
            match r {
                Ref::EmptyHref => missing.push((
                    None,
                    "<link rel=stylesheet> has an empty or missing href".into(),
                )),
                Ref::Public(StylesheetRef::Inline { css, media }) => {
                    let rules = expand(&css, sheets, &mut Vec::new(), &mut import_failures);
                    parsed.push(wrap(rules, media.as_deref()));
                }
                Ref::Public(StylesheetRef::Link { href, media }) => {
                    match find_sheet(sheets, &href, media.as_deref()) {
                        Some(s) => match &s.failure {
                            None => {
                                let rules = expand(
                                    &s.css,
                                    &s.imports,
                                    &mut vec![s.source.clone()],
                                    &mut import_failures,
                                );
                                parsed.push(wrap(rules, media.as_deref()));
                            }
                            Some(reason) => missing.push((Some(href), reason.clone())),
                        },
                        None => missing.push((
                            Some(href),
                            "no stylesheet text was supplied for this <link>".into(),
                        )),
                    }
                }
            }
        }
        let host = Host::new(&self.doc, &parsed, env, !missing.is_empty());
        let report = a11y_rules::run_full_in(&host, locale);
        let mut report = a11y_rules::run_stylesheets_in(report, &host, &parsed, locale);
        for (href, detail) in missing {
            let mut note = InputNote::new(InputKind::StylesheetUnavailable).with_detail(detail);
            if let Some(href) = href {
                note = note.with_subject(href);
            }
            report.note_input(note);
        }
        for prelude in host.unresolved_imports() {
            let detail = import_failures
                .iter()
                .find(|(p, _)| p == prelude)
                .map(|(_, d)| d.as_str())
                .unwrap_or("@import is not resolved; styles that depend on it are undetermined");
            report.note_input(
                InputNote::new(InputKind::StylesheetUnresolved)
                    .with_subject(prelude.clone())
                    .with_detail(detail),
            );
        }
        self.fill_positions(&mut report);
        report
    }

    /// Sets `Location::position` on every finding that names a node, from the
    /// parser's source offsets. A node the parser synthesized (implied
    /// `<tbody>`, `<html>`, `<head>`, `<body>`) takes the position of its
    /// nearest ancestor that comes from the source; without one it stays
    /// empty. Nothing is invented.
    fn fill_positions(&self, report: &mut Report) {
        let mut positions: HashMap<u32, SourcePosition> = HashMap::new();
        let mut stack = vec![(self.doc.root(), None::<SourcePosition>)];
        while let Some((id, inherited)) = stack.pop() {
            let own = self
                .doc
                .node(id)
                .position
                .map(|p| SourcePosition::new(p.line, p.column, p.byte_offset));
            let here = own.or(inherited);
            if let Some(p) = here {
                positions.insert(id.as_u32(), p);
            }
            stack.extend(self.doc.children(id).map(|c| (c, here)));
        }
        for f in &mut report.findings {
            if f.location.position.is_some() {
                continue;
            }
            f.location.position = f
                .location
                .node
                .as_deref()
                .and_then(|n| n.parse::<u32>().ok())
                .and_then(|n| positions.get(&n).copied());
        }
    }
}

fn same_media(a: &str, b: Option<&str>) -> bool {
    b.is_some_and(|b| a.trim().eq_ignore_ascii_case(b.trim()))
}

/// Exact `href` plus `media` first, then a sheet without `media`.
fn find_sheet<'s>(sheets: &'s [Sheet], href: &str, media: Option<&str>) -> Option<&'s Sheet> {
    let same_href = || sheets.iter().filter(|s| s.source == href);
    same_href()
        .find(|s| non_empty(s.media.as_deref()).is_some_and(|m| same_media(&m, media)))
        .or_else(|| same_href().find(|s| non_empty(s.media.as_deref()).is_none()))
}

const MAX_IMPORT_DEPTH: usize = 16;

fn is_import(r: &Rule) -> bool {
    matches!(r, Rule::Other(AtRule { name, .. }) if name == "import")
}

/// Parses `css` and replaces each `@import` the host answered by the rules of
/// the imported sheet. Imports left in place are reported by the cascade; a
/// failure reason goes to `failures`, keyed by the cascade's prelude text.
fn expand(
    css: &str,
    imports: &[Sheet],
    chain: &mut Vec<String>,
    failures: &mut Vec<(String, String)>,
) -> Vec<Rule> {
    let mut out = Vec::new();
    for rule in parse_stylesheet(css).rules {
        let Rule::Other(at) = &rule else {
            out.push(rule);
            continue;
        };
        if at.name != "import" {
            out.push(rule);
            continue;
        }
        let prelude = at.prelude.trim().to_string();
        let Some(import) = Import::parse(&prelude) else {
            out.push(rule);
            continue;
        };
        let Some(sheet) = imports.iter().find(|s| s.source == import.url) else {
            out.push(rule);
            continue;
        };
        if let Some(reason) = &sheet.failure {
            failures.push((prelude, reason.clone()));
            out.push(rule);
            continue;
        }
        if chain.contains(&sheet.source) || chain.len() >= MAX_IMPORT_DEPTH {
            failures.push((prelude, "@import cycle or nesting too deep".into()));
            out.push(rule);
            continue;
        }
        chain.push(sheet.source.clone());
        let inner = expand(&sheet.css, &sheet.imports, chain, failures);
        chain.pop();
        // Layer, `supports()` and media of the import become the matching
        // wrapper rules (Cascade 5 §6.4.1); the cascade ranks them.
        out.extend(import.wrap(inner));
    }
    out
}

/// With a media condition, the rules go into an `@media` block. `@import`
/// stays at the top level, where the cascade looks for it.
fn wrap(rules: Vec<Rule>, media: Option<&str>) -> Stylesheet {
    let Some(condition) = media else {
        return Stylesheet { rules };
    };
    let (mut rules, inner): (Vec<_>, Vec<_>) = rules.into_iter().partition(is_import);
    rules.push(Rule::Media(MediaRule {
        condition: condition.to_string(),
        rules: inner,
    }));
    Stylesheet { rules }
}

/// One call: parses `html` and audits it with the default [`Locale`] (English).
pub fn audit(html: &str, sheets: &[Sheet], env: Environment) -> Report {
    Page::parse(html).audit(sheets, env, Locale::default())
}
