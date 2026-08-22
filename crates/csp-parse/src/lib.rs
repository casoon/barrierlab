//! A pure-Rust parser for the Content Security Policy (CSP) directive
//! grammar.
//!
//! Phases 02-05 (see `plan/`): the generic top-level split (policy-list
//! -> policy -> directive name/raw value), the `serialized-source-list`
//! value grammar, and the CSP3 directive registry are implemented so
//! far.

#![warn(missing_docs)]

mod ast;
mod directive;
mod hash;
mod parser;
mod source_list;

pub use ast::{Directive, Policy, PolicyList};
pub use directive::{
    DirectiveStatus, DirectiveValue, ValueGrammar, ancestor_source_list_is_valid, registry_lookup,
};
pub use hash::{HashAlgorithm, HashExpression, parse_hash_expression};
pub use parser::parse_policy_list;
pub use source_list::{
    HostPart, HostSource, Keyword, PortPart, SourceExpression, SourceList, SourceListEntry,
    parse_source_list,
};
