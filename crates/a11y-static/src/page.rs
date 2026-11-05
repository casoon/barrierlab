use a11y_dom::{Node, NodeKind, ancestors, elements};
use a11y_report::{InputKind, InputNote, Report};
use a11y_rules::Locale;
use style_cascade::Environment;
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

/// A stylesheet text the host fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// The `href` of the [`StylesheetRef::Link`] this answers, verbatim.
    pub source: String,
    /// The stylesheet text.
    pub css: String,
    /// A media condition for this sheet. If `None`, the `media` of the
    /// matching reference applies.
    pub media: Option<String>,
}

fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
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
    /// A `<style>` always counts. Elements inside `<template>` are inert and
    /// skipped. Whether a `media` condition holds is decided later, by the
    /// cascade, against the [`Environment`].
    pub fn stylesheet_refs(&self) -> Vec<StylesheetRef> {
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
                    return Some(StylesheetRef::Inline { css, media });
                }
                let rel = n.attr("rel").unwrap_or("");
                let has = |t: &str| rel.split_whitespace().any(|r| r.eq_ignore_ascii_case(t));
                if n.is_element("link")
                    && has("stylesheet")
                    && !has("alternate")
                    && !n.has_attr("disabled")
                {
                    let href = non_empty(n.attr("href"))?;
                    return Some(StylesheetRef::Link { href, media });
                }
                None
            })
            .collect()
    }

    /// Runs every rule over the page.
    ///
    /// Inline `<style>` texts and the supplied `sheets` (matched to `<link>`
    /// references by `href`) form the cascade, in document order. A `sheets`
    /// entry that matches no reference is ignored. The report carries an
    /// [`InputKind::StylesheetUnavailable`] note for every `<link>` without a
    /// sheet and an [`InputKind::StylesheetUnresolved`] note for every
    /// `@import` the cascade could not resolve.
    pub fn audit(&self, sheets: &[Sheet], env: Environment, locale: Locale) -> Report {
        let mut parsed = Vec::new();
        let mut missing = Vec::new();
        for r in self.stylesheet_refs() {
            match r {
                StylesheetRef::Inline { css, media } => parsed.push(sheet(&css, media.as_deref())),
                StylesheetRef::Link { href, media } => {
                    match sheets.iter().find(|s| s.source == href) {
                        Some(s) => {
                            let media = non_empty(s.media.as_deref()).or(media);
                            parsed.push(sheet(&s.css, media.as_deref()));
                        }
                        None => missing.push(href),
                    }
                }
            }
        }
        let host = Host::new(&self.doc, &parsed, env, !missing.is_empty());
        let report = a11y_rules::run_full_in(&host, locale);
        let mut report = a11y_rules::run_stylesheets_in(report, &host, &parsed, locale);
        for href in missing {
            report.note_input(
                InputNote::new(InputKind::StylesheetUnavailable)
                    .with_subject(href)
                    .with_detail("no stylesheet text was supplied for this <link>"),
            );
        }
        for prelude in host.unresolved_imports() {
            report.note_input(
                InputNote::new(InputKind::StylesheetUnresolved)
                    .with_subject(prelude.clone())
                    .with_detail(
                        "@import is not resolved; styles that depend on it are undetermined",
                    ),
            );
        }
        report
    }
}

/// Parses `css`; with a media condition, the rules go into an `@media` block.
/// `@import` stays at the top level, where the cascade looks for it.
fn sheet(css: &str, media: Option<&str>) -> Stylesheet {
    let parsed = parse_stylesheet(css);
    let Some(condition) = media else {
        return parsed;
    };
    let is_import = |r: &Rule| matches!(r, Rule::Other(AtRule { name, .. }) if name == "import");
    let (mut rules, inner): (Vec<_>, Vec<_>) = parsed.rules.into_iter().partition(is_import);
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
