//! A parser for the CSS Media Queries grammar, written in 100% safe Rust.
//!
//! A Media Queries Level 4 grammar/AST/parser on top of the
//! [`css_tokenize`] tokenizer, including the `<mf-range>` comparison syntax.
//!
//! # Example
//!
//! ```
//! use media_query_parse::{parse_media_query, parse_media_query_list};
//!
//! // A single, syntactically valid media query.
//! assert!(parse_media_query("screen and (min-width: 400px)").is_ok());
//!
//! // An invalid media query reports a `ParseError` instead of panicking:
//! // `only` requires a following `<media-type>`, which is missing here.
//! assert!(parse_media_query("only").is_err());
//!
//! // `parse_media_query_list` splits a comma-separated list and parses
//! // each entry independently, returning one `Result` per entry.
//! let results = parse_media_query_list("screen, only");
//! assert!(results[0].is_ok());
//! assert!(results[1].is_err());
//! ```

#![deny(missing_docs)]

pub mod ast;
pub mod parser;
/// The CSS tokenizer now lives in the [`css_tokenize`] crate; this path is
/// kept so existing code keeps compiling.
#[doc(hidden)]
pub use css_tokenize as tokenizer;

pub use ast::{
    GeneralEnclosed, MediaCondition, MediaConditionWithoutOr, MediaFeature, MediaInParens,
    MediaModifier, MediaQuery, MediaQueryList, MediaType, MfComparison, MfName, MfRange,
    MfRangeDirection, MfValue,
};
pub use parser::{ParseError, parse_media_query, parse_media_query_list};
