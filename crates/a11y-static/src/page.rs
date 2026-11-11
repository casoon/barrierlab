use std::collections::HashMap;

use a11y_dom::{Node, NodeKind, ancestors, elements};
use a11y_report::{InputKind, InputNote, Report, SourcePosition};
use a11y_rules::Locale;
use style_cascade::{Environment, ExpandedImports, Resolution, expand_imports, wrap_media};

use crate::help::rule_help_url;
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
    /// The host declares: this sheet contains no rules that set a style (a
    /// web-font stylesheet with only `@font-face`). An `@import` it answers
    /// is dropped instead of making every style undetermined. See
    /// [`Sheet::inert`].
    pub inert: bool,
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
            inert: false,
        }
    }

    /// A sheet the host declares as **without effect on styles**: it holds
    /// only things like `@font-face`, no rule that sets a property. Typical
    /// for the `@import` of a web-font stylesheet on a host the crate cannot
    /// know (`fonts.googleapis.com`). The host decides; the crate never
    /// guesses from a URL. Nothing is fetched or read. The `@import` it
    /// answers disappears, so colours stay determined (and, if the sheet does
    /// set colours after all, the findings are wrong, not undetermined).
    /// As a `<link>` answer it contributes no rules either.
    pub fn inert(source: impl Into<String>) -> Self {
        Sheet {
            inert: true,
            ..Sheet::new(source, "")
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
    ///
    /// Findings with a node get `Location::position` (the `<` of the start
    /// tag) and `Location::end`. The `end` is the end of the **start tag**
    /// (exclusive), not of the whole element; it is empty for text nodes.
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
                    let out = expand(&css, sheets);
                    import_failures.extend(out.failures);
                    parsed.push(wrap_media(out.rules, media.as_deref()));
                }
                Ref::Public(StylesheetRef::Link { href, media }) => {
                    match find_sheet(sheets, &href, media.as_deref()) {
                        Some(s) => match &s.failure {
                            None => {
                                let out = expand(&s.css, &s.imports);
                                import_failures.extend(out.failures);
                                parsed.push(wrap_media(out.rules, media.as_deref()));
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
        for f in &mut report.findings {
            if f.help_url.is_none() {
                f.help_url = rule_help_url(&f.rule_id);
            }
        }
        report
    }

    /// Sets `Location::position` and `Location::end` on every finding that
    /// names a node, from the parser's source offsets. `end` is the end of
    /// the start tag (exclusive), not of the whole element, and stays empty
    /// for nodes without a start tag (text). A node the parser synthesized
    /// (implied `<tbody>`, `<html>`, `<head>`, `<body>`) takes the position
    /// and end of its nearest ancestor that comes from the source; without
    /// one both stay empty. Nothing is invented.
    fn fill_positions(&self, report: &mut Report) {
        type Span = (SourcePosition, Option<SourcePosition>);
        let to_pos =
            |p: html5_parser::Position| SourcePosition::new(p.line, p.column, p.byte_offset);
        let mut spans: HashMap<u32, Span> = HashMap::new();
        let mut stack = vec![(self.doc.root(), None::<Span>)];
        while let Some((id, inherited)) = stack.pop() {
            let node = self.doc.node(id);
            let own = node
                .position
                .map(|p| (to_pos(p), node.end_position().map(to_pos)));
            let here = own.or(inherited);
            if let Some(span) = here {
                spans.insert(id.as_u32(), span);
            }
            stack.extend(self.doc.children(id).map(|c| (c, here)));
        }
        for f in &mut report.findings {
            if f.location.position.is_some() {
                continue;
            }
            let span = f
                .location
                .node
                .as_deref()
                .and_then(|n| n.parse::<u32>().ok())
                .and_then(|n| spans.get(&n).copied());
            if let Some((start, end)) = span {
                f.location.position = Some(start);
                f.location.end = end;
            }
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

/// Answers an `@import` from the sheets the host handed over: the sheet whose
/// `source` equals the URL, looked up in `level` for a top-level import and in
/// the `imports` of the sheets along `chain` below that.
fn answer(level: &[Sheet], chain: &[String], url: &str) -> Resolution {
    let mut level = level;
    for source in chain {
        match level.iter().find(|s| &s.source == source) {
            Some(s) => level = &s.imports,
            None => return Resolution::Unresolved,
        }
    }
    match level.iter().find(|s| s.source == url) {
        None => Resolution::Unresolved,
        Some(s) => match (&s.failure, s.inert) {
            (Some(reason), _) => Resolution::Failed(reason.clone()),
            (None, true) => Resolution::Inert,
            (None, false) => Resolution::Css(s.css.clone()),
        },
    }
}

/// `level` is where the imports of `css` are looked up: the top-level sheets
/// for an inline `<style>`, `Sheet::imports` for a linked sheet.
fn expand(css: &str, level: &[Sheet]) -> ExpandedImports {
    expand_imports(css, &mut |import, chain| answer(level, chain, &import.url))
}

/// One call: parses `html` and audits it with the default [`Locale`] (English).
pub fn audit(html: &str, sheets: &[Sheet], env: Environment) -> Report {
    Page::parse(html).audit(sheets, env, Locale::default())
}
