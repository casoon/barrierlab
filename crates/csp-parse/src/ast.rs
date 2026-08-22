//! Structured representation of a serialized CSP (or CSP list).
//!
//! Phase 02 scope: the generic, directive-independent top-level split
//! only. A [`Directive`]'s value is kept as a raw string -- interpreting
//! it further (source lists, sandbox tokens, etc.) is later phases' job
//! (see `plan/03-source-list-grammar.md`, `plan/04-directive-registry.md`).

/// A parsed list of CSP policies, as found in a `Content-Security-Policy`
/// HTTP header (a comma-separated list of `serialized-policy`, CSP3 §2.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PolicyList {
    /// The policies, in the order they appeared in the input (comma-separated).
    pub policies: Vec<Policy>,
}

/// A single serialized CSP policy: an ordered list of directives
/// (CSP3 §2.2, `serialized-policy`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Policy {
    /// The directives, in the order they appeared in the input (semicolon-separated).
    pub directives: Vec<Directive>,
}

/// A single directive within a policy: a name and an optional raw value
/// (CSP3 §2.3, `serialized-directive`).
///
/// `raw_value` is exactly the substring that followed the directive name,
/// with only the separating whitespace run stripped -- it is not yet
/// parsed against any directive-specific grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Directive {
    /// The directive name, exactly as it appeared in the input (original
    /// casing preserved -- CSP3 directive-name matching is ASCII-case-
    /// insensitive, see [`Directive::name_is_valid`] and
    /// [`crate::registry_lookup`]).
    pub name: String,
    /// The raw value that followed the directive name, if any.
    pub raw_value: Option<String>,
}

impl Directive {
    /// Whether [`Directive::name`] matches the ABNF `directive-name`
    /// production (`1*( ALPHA / DIGIT / "-" )`, CSP3 §2.3).
    ///
    /// A `false` result does not mean this directive was dropped --
    /// `csp-parse` parses leniently (see `plan/DECISIONS.md`,
    /// 2026-08-22) and still reports it; callers that need strict
    /// conformance checking should check this explicitly.
    pub fn name_is_valid(&self) -> bool {
        !self.name.is_empty()
            && self
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }

    /// Whether [`Directive::raw_value`] (if present) matches the ABNF
    /// `directive-value` production (CSP3 §2.3): any run of ASCII
    /// whitespace or a byte in `%x21-2B / %x2D-3A / %x3C-7E` (printable
    /// ASCII excluding `,` and `;`).
    pub fn value_is_valid(&self) -> bool {
        match &self.raw_value {
            None => true,
            Some(value) => value.bytes().all(|b| {
                b.is_ascii_whitespace() || matches!(b, 0x21..=0x2B | 0x2D..=0x3A | 0x3C..=0x7E)
            }),
        }
    }
}
