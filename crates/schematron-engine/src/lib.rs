#![doc = include_str!("../README.md")]

mod context;
mod engine;
mod parser;
mod resolver;
mod schema;
mod svrl;

pub use engine::{
    Report, SchematronEvalError, evaluate, evaluate_trace, evaluate_trace_with_phase,
    evaluate_with_cache, evaluate_with_phase,
};
pub use parser::{ParseError, Position, parse, parse_with_resolver};
pub use resolver::{ResolveError, SchemaResolver, SchemaSource};
pub use schema::{
    Check, CheckKind, Diagnostic, LetBinding, LetValue, MessagePart, NamespaceBinding, Pattern,
    Phase, Property, Rule, Schema,
};
pub use svrl::{Event, location_path, to_svrl, to_svrl_with};
