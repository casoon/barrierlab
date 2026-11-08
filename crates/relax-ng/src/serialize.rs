//! A stable binary form of a [`CompiledSchema`] (feature `serialize`), so a host can compile a
//! large schema once at build time and load the simplified pattern instead of parsing,
//! composing and simplifying the schema text in every process (barrierlab#72).
//!
//! # Format (version 1)
//!
//! ```text
//! magic "RNGC" | version u8 | string table | node table | start | definitions | FNV-1a 64 (LE)
//! ```
//!
//! Integers are LEB128 varints. The node table is written bottom-up: a node refers only to
//! nodes of an earlier position (per kind: contexts, name classes, pattern lists, patterns),
//! so a reader needs no recursion and cannot meet a cycle. Equal nodes are written once, so
//! the form keeps the sharing of the in-memory pattern. The encoding is canonical: the same
//! schema always gives the same bytes.
//!
//! Loading checks the magic, version, checksum, every index and length against the data
//! actually present, that every `ref` names a definition, and that nothing trails the data.
//! It does not re-check the RELAX NG restrictions of section 7; the producer already did that
//! when it compiled the schema.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::grammar::{Context, NameClass, Param, Pattern};
use crate::simplify::{CompiledSchema, Definitions};

const MAGIC: &[u8; 4] = b"RNGC";

/// The version of the binary form written by [`CompiledSchema::to_bytes`] and the only one
/// [`CompiledSchema::from_bytes`] reads. It changes whenever the encoding does.
pub const FORMAT_VERSION: u8 = 1;

const TAG_CONTEXT: u8 = 0;
const TAG_NAME: u8 = 1;
const TAG_NAME_ANY: u8 = 2;
const TAG_NAME_NS: u8 = 3;
const TAG_NAME_CHOICE: u8 = 4;
const TAG_LIST: u8 = 5;
const TAG_ELEMENT: u8 = 6;
const TAG_ATTRIBUTE: u8 = 7;
const TAG_GROUP: u8 = 8;
const TAG_INTERLEAVE: u8 = 9;
const TAG_CHOICE: u8 = 10;
const TAG_ONE_OR_MORE: u8 = 11;
const TAG_PATTERN_LIST: u8 = 12;
const TAG_EMPTY: u8 = 13;
const TAG_NOT_ALLOWED: u8 = 14;
const TAG_TEXT: u8 = 15;
const TAG_DATA: u8 = 16;
const TAG_VALUE: u8 = 17;
const TAG_REF: u8 = 18;

/// A schema that cannot be written in the binary form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializeError(&'static str);

impl fmt::Display for SerializeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cannot serialize schema: {}", self.0)
    }
}

impl std::error::Error for SerializeError {}

/// Why bytes could not be loaded as a [`CompiledSchema`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    /// The data does not start with the format's magic bytes.
    BadMagic,
    /// The data is in a format version this build does not read.
    UnsupportedVersion(u8),
    /// The checksum does not match the data.
    ChecksumMismatch,
    /// The data ends before the structure it announces does.
    Truncated,
    /// The data is well-framed but contradicts itself.
    Invalid(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(formatter, "not a serialized RELAX NG schema"),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "serialized schema is format version {version}, this build reads {FORMAT_VERSION}"
            ),
            Self::ChecksumMismatch => write!(formatter, "serialized schema is corrupted"),
            Self::Truncated => write!(formatter, "serialized schema is truncated"),
            Self::Invalid(reason) => write!(formatter, "serialized schema is invalid: {reason}"),
        }
    }
}

impl std::error::Error for LoadError {}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------------- writing

#[derive(Clone, Copy)]
enum Kind {
    Context,
    Name,
    List,
    Pattern,
}

struct Writer<'a> {
    strings: Vec<&'a str>,
    string_ids: HashMap<&'a str, u32>,
    nodes: Vec<u8>,
    node_count: u32,
    interned: [HashMap<Vec<u8>, u32>; 4],
    context_arcs: HashMap<usize, u32>,
    name_arcs: HashMap<usize, u32>,
    list_arcs: HashMap<(usize, usize), u32>,
}

fn varint(out: &mut Vec<u8>, mut value: u32) {
    while value >= 0x80 {
        out.push((value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn length(value: usize) -> Result<u32, SerializeError> {
    u32::try_from(value).map_err(|_| SerializeError("too many entries"))
}

impl<'a> Writer<'a> {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            string_ids: HashMap::new(),
            nodes: Vec::new(),
            node_count: 0,
            interned: Default::default(),
            context_arcs: HashMap::new(),
            name_arcs: HashMap::new(),
            list_arcs: HashMap::new(),
        }
    }

    fn string(&mut self, value: &'a str) -> u32 {
        if let Some(&id) = self.string_ids.get(value) {
            return id;
        }
        let id = self.strings.len() as u32;
        self.strings.push(value);
        self.string_ids.insert(value, id);
        id
    }

    fn put_string(&mut self, out: &mut Vec<u8>, value: &'a str) {
        let id = self.string(value);
        varint(out, id);
    }

    fn put_optional(&mut self, out: &mut Vec<u8>, value: &'a Option<String>) {
        match value {
            None => varint(out, 0),
            Some(text) => {
                let id = self.string(text);
                varint(out, id + 1);
            }
        }
    }

    fn intern(&mut self, kind: Kind, tag: u8, body: Vec<u8>) -> u32 {
        let table = &mut self.interned[kind as usize];
        let mut key = Vec::with_capacity(body.len() + 1);
        key.push(tag);
        key.extend_from_slice(&body);
        if let Some(&id) = table.get(&key) {
            return id;
        }
        let id = table.len() as u32;
        self.nodes.extend_from_slice(&key);
        self.node_count += 1;
        table.insert(key, id);
        id
    }

    fn context(&mut self, context: &'a Context) -> u32 {
        let mut body = Vec::new();
        self.put_string(&mut body, &context.base_uri);
        varint(&mut body, context.namespaces.len() as u32);
        for (prefix, uri) in &context.namespaces {
            self.put_string(&mut body, prefix);
            self.put_string(&mut body, uri);
        }
        self.put_optional(&mut body, &context.default_namespace);
        self.put_optional(&mut body, &context.datatype_library);
        self.put_optional(&mut body, &context.ns);
        self.intern(Kind::Context, TAG_CONTEXT, body)
    }

    fn context_arc(&mut self, context: &'a Arc<Context>) -> u32 {
        let key = Arc::as_ptr(context) as usize;
        if let Some(&id) = self.context_arcs.get(&key) {
            return id;
        }
        let id = self.context(context);
        self.context_arcs.insert(key, id);
        id
    }

    fn name_classes(&mut self, body: &mut Vec<u8>, classes: &'a [NameClass]) {
        varint(body, classes.len() as u32);
        for class in classes {
            let id = self.name_class(class);
            varint(body, id);
        }
    }

    fn name_class(&mut self, class: &'a NameClass) -> u32 {
        let mut body = Vec::new();
        let tag = match class {
            NameClass::Name {
                lexical,
                namespace,
                context,
            } => {
                self.put_string(&mut body, lexical);
                self.put_optional(&mut body, namespace);
                let context = self.context(context);
                varint(&mut body, context);
                TAG_NAME
            }
            NameClass::Any { except, context } => {
                self.name_classes(&mut body, except);
                let context = self.context(context);
                varint(&mut body, context);
                TAG_NAME_ANY
            }
            NameClass::Namespace {
                namespace,
                except,
                context,
            } => {
                self.put_optional(&mut body, namespace);
                self.name_classes(&mut body, except);
                let context = self.context(context);
                varint(&mut body, context);
                TAG_NAME_NS
            }
            NameClass::Choice(options) => {
                self.name_classes(&mut body, options);
                TAG_NAME_CHOICE
            }
        };
        self.intern(Kind::Name, tag, body)
    }

    fn name_class_arc(&mut self, class: &'a Arc<NameClass>) -> u32 {
        let key = Arc::as_ptr(class) as usize;
        if let Some(&id) = self.name_arcs.get(&key) {
            return id;
        }
        let id = self.name_class(class);
        self.name_arcs.insert(key, id);
        id
    }

    fn list(&mut self, list: &'a Arc<[Pattern]>) -> Result<u32, SerializeError> {
        let key = (list.as_ptr() as usize, list.len());
        if let Some(&id) = self.list_arcs.get(&key) {
            return Ok(id);
        }
        let mut body = Vec::new();
        varint(&mut body, length(list.len())?);
        for pattern in list.iter() {
            let id = self.pattern(pattern)?;
            varint(&mut body, id);
        }
        let id = self.intern(Kind::List, TAG_LIST, body);
        self.list_arcs.insert(key, id);
        Ok(id)
    }

    fn pattern(&mut self, pattern: &'a Pattern) -> Result<u32, SerializeError> {
        let mut body = Vec::new();
        let tag = match pattern {
            Pattern::Element {
                name,
                body: children,
                context,
            }
            | Pattern::Attribute {
                name,
                body: children,
                context,
            } => {
                let name = self.name_class_arc(name);
                let children = self.list(children)?;
                let context = self.context_arc(context);
                varint(&mut body, name);
                varint(&mut body, children);
                varint(&mut body, context);
                if matches!(pattern, Pattern::Element { .. }) {
                    TAG_ELEMENT
                } else {
                    TAG_ATTRIBUTE
                }
            }
            Pattern::Group(values)
            | Pattern::Interleave(values)
            | Pattern::Choice(values)
            | Pattern::OneOrMore(values)
            | Pattern::List(values) => {
                let list = self.list(values)?;
                varint(&mut body, list);
                match pattern {
                    Pattern::Group(_) => TAG_GROUP,
                    Pattern::Interleave(_) => TAG_INTERLEAVE,
                    Pattern::Choice(_) => TAG_CHOICE,
                    Pattern::OneOrMore(_) => TAG_ONE_OR_MORE,
                    _ => TAG_PATTERN_LIST,
                }
            }
            Pattern::Empty => TAG_EMPTY,
            Pattern::NotAllowed => TAG_NOT_ALLOWED,
            Pattern::Text => TAG_TEXT,
            Pattern::Data {
                datatype,
                datatype_library,
                params,
                except,
                context,
            } => {
                self.put_string(&mut body, datatype);
                self.put_optional(&mut body, datatype_library);
                varint(&mut body, length(params.len())?);
                for param in params {
                    self.put_string(&mut body, &param.name);
                    self.put_string(&mut body, &param.value);
                    let context = self.context(&param.context);
                    varint(&mut body, context);
                }
                let except = self.list(except)?;
                varint(&mut body, except);
                let context = self.context_arc(context);
                varint(&mut body, context);
                TAG_DATA
            }
            Pattern::Value {
                value,
                datatype,
                datatype_library,
                namespace,
                context,
            } => {
                self.put_string(&mut body, value);
                self.put_optional(&mut body, datatype);
                self.put_optional(&mut body, datatype_library);
                self.put_optional(&mut body, namespace);
                let context = self.context_arc(context);
                varint(&mut body, context);
                TAG_VALUE
            }
            Pattern::Ref(name) => {
                self.put_string(&mut body, name);
                TAG_REF
            }
            Pattern::Optional(_) | Pattern::ZeroOrMore(_) | Pattern::Mixed(_) => {
                return Err(SerializeError("pattern is not in simplified form"));
            }
            Pattern::ParentRef(_) | Pattern::ExternalRef { .. } | Pattern::Grammar(_) => {
                return Err(SerializeError("pattern is not in simplified form"));
            }
        };
        Ok(self.intern(Kind::Pattern, tag, body))
    }
}

impl CompiledSchema {
    /// Writes this schema in the binary form (see the [module](self) documentation); the bytes
    /// are the same for the same schema. [`CompiledSchema::from_bytes`] reads them back.
    ///
    /// Fails only for a schema that is not in simplified form, which [`crate::simplify`] never
    /// produces.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let mut writer = Writer::new();
        let start = writer.pattern(&self.start)?;
        let mut definitions = Vec::new();
        for (name, pattern) in self.definitions.named() {
            let name = writer.string(name);
            let pattern = writer.pattern(pattern)?;
            definitions.push((name, pattern));
        }

        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(FORMAT_VERSION);
        varint(&mut out, length(writer.strings.len())?);
        for string in &writer.strings {
            varint(&mut out, length(string.len())?);
            out.extend_from_slice(string.as_bytes());
        }
        varint(&mut out, writer.node_count);
        out.extend_from_slice(&writer.nodes);
        varint(&mut out, start);
        varint(&mut out, length(definitions.len())?);
        for (name, pattern) in definitions {
            varint(&mut out, name);
            varint(&mut out, pattern);
        }
        let sum = checksum(&out);
        out.extend_from_slice(&sum.to_le_bytes());
        Ok(out)
    }

    /// Reads a schema written by [`CompiledSchema::to_bytes`]. Malformed, truncated or
    /// corrupted input is an [`Err`], never a panic.
    pub fn from_bytes(bytes: &[u8]) -> Result<CompiledSchema, LoadError> {
        Reader::load(bytes)
    }
}

// ---------------------------------------------------------------------------------- reading

struct Cursor<'b> {
    bytes: &'b [u8],
    position: usize,
}

impl<'b> Cursor<'b> {
    fn byte(&mut self) -> Result<u8, LoadError> {
        let byte = *self.bytes.get(self.position).ok_or(LoadError::Truncated)?;
        self.position += 1;
        Ok(byte)
    }

    fn varint(&mut self) -> Result<u32, LoadError> {
        let mut value: u32 = 0;
        for shift in (0..35).step_by(7) {
            let byte = self.byte()?;
            let bits = u32::from(byte & 0x7f);
            if shift == 28 && bits > 0x0f {
                return Err(LoadError::Invalid("varint overflows 32 bits"));
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(LoadError::Invalid("varint overflows 32 bits"))
    }

    /// A length or count, which must be backed by at least that many remaining bytes (every
    /// counted entry takes at least one) — so a corrupt count cannot make us allocate wildly.
    fn count(&mut self) -> Result<usize, LoadError> {
        let count = self.varint()? as usize;
        if count > self.bytes.len() - self.position {
            return Err(LoadError::Truncated);
        }
        Ok(count)
    }

    fn take(&mut self, length: usize) -> Result<&'b [u8], LoadError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(LoadError::Truncated)?;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or(LoadError::Truncated)?;
        self.position = end;
        Ok(slice)
    }
}

struct Reader<'b> {
    cursor: Cursor<'b>,
    strings: Vec<String>,
    contexts: Vec<Arc<Context>>,
    names: Vec<Arc<NameClass>>,
    lists: Vec<Arc<[Pattern]>>,
    patterns: Vec<Pattern>,
    refs: HashSet<u32>,
}

fn at<'v, T>(values: &'v [T], index: u32, what: &'static str) -> Result<&'v T, LoadError> {
    values.get(index as usize).ok_or(LoadError::Invalid(what))
}

impl Reader<'_> {
    fn load(bytes: &[u8]) -> Result<CompiledSchema, LoadError> {
        if bytes.len() < MAGIC.len() + 1 + 8 || &bytes[..MAGIC.len()] != MAGIC {
            return Err(if bytes.len() < MAGIC.len() {
                LoadError::Truncated
            } else {
                LoadError::BadMagic
            });
        }
        let version = bytes[MAGIC.len()];
        if version != FORMAT_VERSION {
            return Err(LoadError::UnsupportedVersion(version));
        }
        let (body, sum) = bytes.split_at(bytes.len() - 8);
        if sum != checksum(body).to_le_bytes() {
            return Err(LoadError::ChecksumMismatch);
        }
        let mut reader = Reader {
            cursor: Cursor {
                bytes: body,
                position: MAGIC.len() + 1,
            },
            strings: Vec::new(),
            contexts: Vec::new(),
            names: Vec::new(),
            lists: Vec::new(),
            patterns: Vec::new(),
            refs: HashSet::new(),
        };
        reader.read()
    }

    fn string(&mut self) -> Result<String, LoadError> {
        let id = self.cursor.varint()?;
        Ok(at(&self.strings, id, "string index out of range")?.clone())
    }

    fn optional(&mut self) -> Result<Option<String>, LoadError> {
        match self.cursor.varint()? {
            0 => Ok(None),
            id => Ok(Some(
                at(&self.strings, id - 1, "string index out of range")?.clone(),
            )),
        }
    }

    fn context_by_value(&mut self) -> Result<Context, LoadError> {
        let id = self.cursor.varint()?;
        Ok((**at(&self.contexts, id, "context index out of range")?).clone())
    }

    fn context_arc(&mut self) -> Result<Arc<Context>, LoadError> {
        let id = self.cursor.varint()?;
        Ok(Arc::clone(at(
            &self.contexts,
            id,
            "context index out of range",
        )?))
    }

    fn name_classes(&mut self) -> Result<Vec<NameClass>, LoadError> {
        let count = self.cursor.count()?;
        let mut classes = Vec::with_capacity(count);
        for _ in 0..count {
            let id = self.cursor.varint()?;
            classes.push((**at(&self.names, id, "name class index out of range")?).clone());
        }
        Ok(classes)
    }

    fn list(&mut self) -> Result<Arc<[Pattern]>, LoadError> {
        let id = self.cursor.varint()?;
        Ok(Arc::clone(at(
            &self.lists,
            id,
            "pattern list index out of range",
        )?))
    }

    fn read(&mut self) -> Result<CompiledSchema, LoadError> {
        let string_count = self.cursor.count()?;
        self.strings.reserve(string_count);
        for _ in 0..string_count {
            let length = self.cursor.count()?;
            let text = std::str::from_utf8(self.cursor.take(length)?)
                .map_err(|_| LoadError::Invalid("string is not UTF-8"))?;
            self.strings.push(text.to_owned());
        }

        let node_count = self.cursor.count()?;
        for _ in 0..node_count {
            let tag = self.cursor.byte()?;
            self.node(tag)?;
        }

        let start_id = self.cursor.varint()?;
        let start = at(&self.patterns, start_id, "start pattern index out of range")?.clone();
        let definition_count = self.cursor.count()?;
        let mut definitions = BTreeMap::new();
        for _ in 0..definition_count {
            let name = self.string()?;
            let id = self.cursor.varint()?;
            let pattern = at(&self.patterns, id, "definition pattern index out of range")?.clone();
            if definitions.insert(name, pattern).is_some() {
                return Err(LoadError::Invalid("duplicate definition"));
            }
        }
        if self.cursor.position != self.cursor.bytes.len() {
            return Err(LoadError::Invalid("trailing data"));
        }
        for &id in &self.refs {
            if !definitions.contains_key(&self.strings[id as usize]) {
                return Err(LoadError::Invalid("ref to an undefined definition"));
            }
        }
        Ok(CompiledSchema {
            start,
            definitions: Definitions::new(definitions),
        })
    }

    fn node(&mut self, tag: u8) -> Result<(), LoadError> {
        match tag {
            TAG_CONTEXT => {
                let base_uri = self.string()?;
                let count = self.cursor.count()?;
                let mut namespaces = BTreeMap::new();
                for _ in 0..count {
                    let prefix = self.string()?;
                    let uri = self.string()?;
                    namespaces.insert(prefix, uri);
                }
                let default_namespace = self.optional()?;
                let datatype_library = self.optional()?;
                let ns = self.optional()?;
                self.contexts.push(Arc::new(Context {
                    base_uri,
                    namespaces,
                    default_namespace,
                    datatype_library,
                    ns,
                }));
            }
            TAG_NAME => {
                let lexical = self.string()?;
                let namespace = self.optional()?;
                let context = self.context_by_value()?;
                self.names.push(Arc::new(NameClass::Name {
                    lexical,
                    namespace,
                    context,
                }));
            }
            TAG_NAME_ANY => {
                let except = self.name_classes()?;
                let context = self.context_by_value()?;
                self.names.push(Arc::new(NameClass::Any { except, context }));
            }
            TAG_NAME_NS => {
                let namespace = self.optional()?;
                let except = self.name_classes()?;
                let context = self.context_by_value()?;
                self.names.push(Arc::new(NameClass::Namespace {
                    namespace,
                    except,
                    context,
                }));
            }
            TAG_NAME_CHOICE => {
                let options = self.name_classes()?;
                self.names.push(Arc::new(NameClass::Choice(options)));
            }
            TAG_LIST => {
                let count = self.cursor.count()?;
                let mut patterns = Vec::with_capacity(count);
                for _ in 0..count {
                    let id = self.cursor.varint()?;
                    patterns.push(at(&self.patterns, id, "pattern index out of range")?.clone());
                }
                self.lists.push(patterns.into());
            }
            TAG_ELEMENT | TAG_ATTRIBUTE => {
                let id = self.cursor.varint()?;
                let name = Arc::clone(at(&self.names, id, "name class index out of range")?);
                let body = self.list()?;
                let context = self.context_arc()?;
                self.patterns.push(if tag == TAG_ELEMENT {
                    Pattern::Element {
                        name,
                        body,
                        context,
                    }
                } else {
                    Pattern::Attribute {
                        name,
                        body,
                        context,
                    }
                });
            }
            TAG_GROUP | TAG_INTERLEAVE | TAG_CHOICE | TAG_ONE_OR_MORE | TAG_PATTERN_LIST => {
                let list = self.list()?;
                self.patterns.push(match tag {
                    TAG_GROUP => Pattern::Group(list),
                    TAG_INTERLEAVE => Pattern::Interleave(list),
                    TAG_CHOICE => Pattern::Choice(list),
                    TAG_ONE_OR_MORE => Pattern::OneOrMore(list),
                    _ => Pattern::List(list),
                });
            }
            TAG_EMPTY => self.patterns.push(Pattern::Empty),
            TAG_NOT_ALLOWED => self.patterns.push(Pattern::NotAllowed),
            TAG_TEXT => self.patterns.push(Pattern::Text),
            TAG_DATA => {
                let datatype = self.string()?;
                let datatype_library = self.optional()?;
                let count = self.cursor.count()?;
                let mut params = Vec::with_capacity(count);
                for _ in 0..count {
                    let name = self.string()?;
                    let value = self.string()?;
                    let context = self.context_by_value()?;
                    params.push(Param {
                        name,
                        value,
                        context,
                    });
                }
                let except = self.list()?;
                let context = self.context_arc()?;
                self.patterns.push(Pattern::Data {
                    datatype,
                    datatype_library,
                    params,
                    except,
                    context,
                });
            }
            TAG_VALUE => {
                let value = self.string()?;
                let datatype = self.optional()?;
                let datatype_library = self.optional()?;
                let namespace = self.optional()?;
                let context = self.context_arc()?;
                self.patterns.push(Pattern::Value {
                    value,
                    datatype,
                    datatype_library,
                    namespace,
                    context,
                });
            }
            TAG_REF => {
                let id = self.cursor.varint()?;
                let name = at(&self.strings, id, "string index out of range")?.clone();
                self.refs.insert(id);
                self.patterns.push(Pattern::Ref(name));
            }
            _ => return Err(LoadError::Invalid("unknown node tag")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResolveError, Schema, SchemaResolver, SchemaSource, SchemaSyntax};

    struct NoResolver;
    impl SchemaResolver for NoResolver {
        fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
            Err(ResolveError::new(format!("no such resource: {href}")))
        }
    }

    const SCHEMA: &str = r#"
        default namespace = "urn:x"
        datatypes xsd = "http://www.w3.org/2001/XMLSchema-datatypes"
        start = doc
        doc = element doc { attribute id { xsd:ID }?, attribute kind { "a" | "b" }, (item | other)+ }
        item = element item { xsd:integer { minInclusive = "1" } }
        other = element * - item { (attribute * - id { text })*, (doc | text | (element lst { list { xsd:token+ } }))* }
    "#;

    fn compiled() -> CompiledSchema {
        let source = SchemaSource::new(SCHEMA, "memory:/t.rnc", SchemaSyntax::Compact);
        Schema::compile(&source, &NoResolver)
            .expect("compiles")
            .compiled()
            .clone()
    }

    #[test]
    fn round_trip_is_canonical_and_lossless() {
        let schema = compiled();
        let bytes = schema.to_bytes().expect("serializes");
        let loaded = CompiledSchema::from_bytes(&bytes).expect("loads");
        assert_eq!(loaded.start, schema.start);
        assert_eq!(loaded.definition_count(), schema.definition_count());
        for (name, pattern) in schema.definitions.named() {
            assert_eq!(loaded.definitions.get(name), Some(pattern));
        }
        assert_eq!(loaded.to_bytes().expect("serializes"), bytes);
    }

    #[test]
    fn truncated_input_is_an_error() {
        let bytes = compiled().to_bytes().expect("serializes");
        for length in 0..bytes.len() {
            assert!(CompiledSchema::from_bytes(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn corrupted_input_is_an_error_not_a_panic() {
        let bytes = compiled().to_bytes().expect("serializes");
        // The checksum catches any single flipped bit.
        for index in 0..bytes.len() {
            let mut damaged = bytes.clone();
            damaged[index] ^= 0x10;
            assert!(CompiledSchema::from_bytes(&damaged).is_err());
        }
    }

    /// Corruption that carries a valid checksum gets past the cheap check and must still be
    /// caught (or accepted) by the structural checks, without panicking.
    #[test]
    fn corrupted_input_with_a_valid_checksum_does_not_panic() {
        let bytes = compiled().to_bytes().expect("serializes");
        let body_len = bytes.len() - 8;
        let mut state: u32 = 0x1234_5678;
        for _ in 0..4000 {
            let mut damaged = bytes[..body_len].to_vec();
            for _ in 0..1 + state % 3 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let index = 5 + (state >> 8) as usize % (body_len - 5);
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                damaged[index] = (state >> 16) as u8;
            }
            if state % 5 == 0 {
                damaged.truncate(5 + (state >> 8) as usize % (body_len - 5));
            }
            let sum = checksum(&damaged);
            damaged.extend_from_slice(&sum.to_le_bytes());
            let _ = CompiledSchema::from_bytes(&damaged);
        }
    }

    #[test]
    fn wrong_magic_and_version_are_named() {
        let mut bytes = compiled().to_bytes().expect("serializes");
        let mut other_version = bytes.clone();
        other_version[4] = FORMAT_VERSION + 1;
        assert_eq!(
            CompiledSchema::from_bytes(&other_version).unwrap_err(),
            LoadError::UnsupportedVersion(FORMAT_VERSION + 1)
        );
        bytes[0] = b'X';
        assert_eq!(
            CompiledSchema::from_bytes(&bytes).unwrap_err(),
            LoadError::BadMagic
        );
        assert!(CompiledSchema::from_bytes(&[]).is_err());
    }
}
