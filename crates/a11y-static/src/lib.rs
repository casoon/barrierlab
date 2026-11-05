//! Browser-free audit of static HTML.
//!
//! The host fetches; this crate computes. Hand it the HTML text and the texts
//! of the stylesheets the page references, get an [`a11y_report::Report`] back:
//!
//! ```
//! use a11y_static::{audit, Environment};
//!
//! let html = r#"<!doctype html><html lang="en"><head><title>t</title></head>
//!   <body><img src="a.png"></body></html>"#;
//! let report = audit(html, &[], Environment::default());
//! assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));
//! ```
//!
//! The chain behind [`Page::audit`] is: parse with `html5-parser`, read the
//! tree through `a11y-dom` (no copy), roles and names through `accname`,
//! computed styles through `style-cascade`, then every tier of `a11y-rules`
//! and its stylesheet rules.
//!
//! Stylesheets the host could not supply are not skipped silently: the report
//! carries an [`InputNote`](a11y_report::InputNote) for each, and the contrast
//! rules come out undecided rather than passed.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod host;
mod page;

pub use a11y_rules::Locale;
pub use page::{Page, Sheet, StylesheetRef, audit};
pub use style_cascade::Environment;
