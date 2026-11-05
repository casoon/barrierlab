//! CSS-Kaskade ohne Browser.
//!
//! Aus einem Dokument ([`a11y_dom::Document`]) und den Stylesheets der Seite
//! ([`stylesheet_parse`]) entstehen die berechneten Stile, die Regeln des
//! Darstellungs-Tiers brauchen — ohne Layout, ohne Skripte.
//!
//! Was nicht sicher bestimmt werden kann, bleibt unbestimmt: ein Wert, der von
//! einer Regel mit unbekanntem Selektor oder unbekannter Medienbedingung
//! abhängt, ist kein geratener Wert, sondern keiner.

#![forbid(unsafe_code)]

mod media;
pub mod selector;
pub use media::Environment;
mod color;
pub use color::{luminance, over, parse_color};
mod cascade;
mod layers;
mod shorthand;
mod ua;
mod value;
pub use cascade::{Cascade, Specified};
mod style;
