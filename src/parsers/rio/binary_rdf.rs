//! Binary RDF 1.0 parser and renderer (B5 Slice 3, docs/PLAN.md).
//!
//! Implements the RDF4J **Binary RDF** format family
//! (`org.eclipse.rdf4j.rio.binary.BinaryRDFFormat`):
//!
//! * **8-byte header** — the ASCII magic `BRDF` followed by a 4-byte signed
//!   version number (big-endian, Java `DataOutputStream.writeInt` order) set
//!   to `1`.
//! * **Typed records**, each introduced by a single record-type byte:
//!
//!   | record | byte | payload |
//!   |---|---|---|
//!   | `NAMESPACE_DECL` | `0` | prefix string, namespace-IRI string |
//!   | `STATEMENT`      | `1` | subject, predicate, object, context value slots |
//!   | `COMMENT`        | `2` | comment string |
//!   | `VALUE_DECL`     | `3` | value id, full value |
//!   | `END_OF_DATA`    | `127`| (none) |
//!
//! * **Integer encoding** — unsigned 7-bit varint with the high bit as
//!   continuation flag (`IOUtil.writeVarInt`: least-significant 7 bits first,
//!   `0x80` = “another byte follows”; e.g. `421` → `A5 03`, `100000` →
//!   `A0 8D 06`).  Used for value ids and string byte-lengths (UTF-8).
//! * **Value types** — `NULL=0`, `URI=1`, `BNODE=2`, `PLAIN_LITERAL=3`,
//!   `LANG_LITERAL=4`, `DATATYPE_LITERAL=5`, `VALUE_REF=6`, `TRIPLE=7`
//!   (RDF-star/RDF 1.2 triple terms are decoded recursively).
//! * **Value references** — the *first* occurrence of a value is declared by
//!   a `VALUE_DECL` record carrying its id and full encoding; every
//!   occurrence (including that first one) is then written in the statement
//!   slot as `VALUE_REF` + id, so repeated terms cost two bytes per slot.
//!   The reader also accepts *inline* values in statement slots (a writer
//!   that inlines first occurrences, as RDF4J does) — those simply extend
//!   the statement and do not allocate ids, so `VALUE_DECL` ids stay the
//!   single authoritative id space.
//!
//! Statement **contexts** (graph names) are decoded: the 4th slot may carry
//! a named graph, whose statements are unioned into the ontology’s single
//! triple set (an [`Ontology`] has no named-graph structure — the same choice
//! the JSON-LD parser documents).  The writer always emits `NULL` for the
//! context.  `COMMENT` records are preserved as ontology-level
//! `rdfs:comment` annotations.
//!
//! ## `save_to_string` contract
//!
//! A Binary RDF payload is not valid UTF-8 in general (varint bytes ≥ 0x80),
//! so [`crate::parsers::save_to_string`] **refuses** this format with a clear
//! error; use `save_to_bytes` / `save_file` (byte-oriented I/O).  The free
//! [`parse`] entry point takes `&str` for API compatibility and interprets
//! the string’s bytes as the Binary RDF stream (only usable when the payload
//! happens to be valid UTF-8 — byte callers should use
//! [`BinaryRdfParser::parse`]).
//!
//! ## Conformance note
//!
//! Layout, record types, value types and varint encoding follow RDF4J’s
//! `BinaryRDFConstants` / `BinaryRDFWriter` / `IOUtil.writeVarInt`.  One
//! documented divergence: RDF4J itself versions the *string encoding*
//! (v1 = 4-byte lengths + UTF-16BE, v2 = varint lengths + UTF-8), while the
//! B5 spec profile fixed by `docs/PLAN.md` is “version 1 + varint integer
//! encoding”, which is what this module writes and reads (v2’s record
//! structure under the v1 version number).  Files written by RDF4J’s own
//! `FORMAT_V1` writer (4-byte/UTF-16 strings) are therefore not accepted;
//! the version check rejects them explicitly instead of misparsing.

use std::collections::HashMap;
use std::io::Write;

use crate::ontology::{IRI, Ontology};
use crate::parsers::rdf_to_owl;
use crate::semantics::owl_rdf_mapping::ontology_to_triples;
use crate::semantics::{RdfTerm, Triple};
use crate::{Error, Result};

// ═══════════════════════════════════════════════════════════════════════════
// Format constants (RDF4J `BinaryRDFConstants`)
// ═══════════════════════════════════════════════════════════════════════════

/// ASCII magic number starting every Binary RDF document.
pub const MAGIC: &[u8; 4] = b"BRDF";
/// Format version written in the 8-byte header (big-endian `i32`).
pub const VERSION: i32 = 1;
/// Header size: 4-byte magic + 4-byte version.
pub const HEADER_LEN: usize = 8;

// Record types
/// `prefix namespace-IRI` declaration record.
pub const NAMESPACE_DECL: u8 = 0;
/// `subject predicate object context` statement record.
pub const STATEMENT: u8 = 1;
/// Free-form comment record.
pub const COMMENT: u8 = 2;
/// `id value` declaration record.
pub const VALUE_DECL: u8 = 3;
/// Terminator record; must be the last byte of a well-formed document.
pub const END_OF_DATA: u8 = 127;

// Value types
pub const NULL_VALUE: u8 = 0;
pub const URI_VALUE: u8 = 1;
pub const BNODE_VALUE: u8 = 2;
pub const PLAIN_LITERAL_VALUE: u8 = 3;
pub const LANG_LITERAL_VALUE: u8 = 4;
pub const DATATYPE_LITERAL_VALUE: u8 = 5;
pub const VALUE_REF: u8 = 6;
pub const TRIPLE_VALUE: u8 = 7;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";

// ═══════════════════════════════════════════════════════════════════════════
// Varint (RDF4J `IOUtil.writeVarInt` / `readVarInt`)
// ═══════════════════════════════════════════════════════════════════════════

/// Write an unsigned varint: 7 data bits per byte, `0x80` continuation flag,
/// least-significant group first (identical to RDF4J `IOUtil.writeVarInt`).
///
/// Known encodings: `0 → 00`, `127 → 7F`, `128 → 80 01`, `421 → A5 03`,
/// `100000 → A0 8D 06`.
pub fn write_varint(out: &mut dyn Write, mut value: u64) -> Result<()> {
    while value >= 0x80 {
        out.write_all(&[((value as u8) & 0x7F) | 0x80])
            .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
        value >>= 7;
    }
    out.write_all(&[value as u8])
        .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))
}

/// Read an unsigned varint (see [`write_varint`]).  Returns an error on
/// truncation, on a run longer than 10 bytes, or on overflow — never panics
/// and never silently truncates high bits (CR-011 n6).
pub fn read_varint(data: &[u8], pos: &mut usize) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        let Some(&byte) = data.get(*pos) else {
            return Err(Error::ParseError(
                "BinaryRDF: unexpected end of data inside a varint".to_owned(),
            ));
        };
        *pos += 1;
        if shift >= 64 {
            return Err(Error::ParseError(
                "BinaryRDF: varint is too long (integer overflow)".to_owned(),
            ));
        }
        // At shift 63 only bit 0 of the group fits into u64 — a larger
        // group would be silently truncated by the shift, so reject it.
        if shift == 63 && (byte & 0x7F) > 1 {
            return Err(Error::ParseError(
                "BinaryRDF: varint overflows 64 bits".to_owned(),
            ));
        }
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Value model
// ═══════════════════════════════════════════════════════════════════════════

/// A Binary RDF value as encoded by a `VALUE_DECL` / inline value slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BrdfValue {
    Iri(String),
    BNode(String),
    Plain(String),
    Lang(String, String),
    Datatype(String, String),
    /// RDF-star / RDF 1.2 triple term (nested full values, per spec).
    Triple(Box<BrdfValue>, Box<BrdfValue>, Box<BrdfValue>),
}

impl BrdfValue {
    fn to_rdf_term(&self) -> Result<RdfTerm> {
        match self {
            BrdfValue::Iri(s) => {
                Ok(RdfTerm::Iri(url::Url::parse(s).map_err(|e| {
                    Error::ParseError(format!("BinaryRDF: invalid IRI '{s}': {e}"))
                })?))
            }
            BrdfValue::BNode(id) => Ok(RdfTerm::BlankNode(id.clone())),
            BrdfValue::Plain(v) => Ok(RdfTerm::Literal {
                value: v.clone(),
                datatype: None,
                language: None,
                direction: None,
            }),
            BrdfValue::Lang(v, lang) => Ok(RdfTerm::Literal {
                value: v.clone(),
                datatype: None,
                language: Some(lang.clone()),
                direction: None,
            }),
            BrdfValue::Datatype(v, dt) => Ok(RdfTerm::Literal {
                value: v.clone(),
                datatype: Some(url::Url::parse(dt).map_err(|e| {
                    Error::ParseError(format!("BinaryRDF: invalid datatype IRI '{dt}': {e}"))
                })?),
                language: None,
                direction: None,
            }),
            BrdfValue::Triple(s, p, o) => {
                let subject = s.to_rdf_term()?;
                if !matches!(subject, RdfTerm::Iri(_) | RdfTerm::BlankNode(_)) {
                    return Err(Error::ParseError(
                        "BinaryRDF: triple-term subject must be an IRI or blank node".to_owned(),
                    ));
                }
                let predicate = p.to_rdf_term()?;
                if !matches!(predicate, RdfTerm::Iri(_)) {
                    return Err(Error::ParseError(
                        "BinaryRDF: triple-term predicate must be an IRI".to_owned(),
                    ));
                }
                Ok(RdfTerm::TripleTerm(Box::new(Triple {
                    subject,
                    predicate,
                    object: o.to_rdf_term()?,
                })))
            }
        }
    }

    fn from_rdf_term(term: &RdfTerm) -> Result<Self> {
        Ok(match term {
            RdfTerm::Iri(url) => BrdfValue::Iri(url.as_str().to_owned()),
            RdfTerm::BlankNode(id) => {
                BrdfValue::BNode(id.strip_prefix("_:").unwrap_or(id).to_owned())
            }
            RdfTerm::Literal {
                value,
                datatype,
                language,
                ..
            } => {
                if let Some(lang) = language {
                    BrdfValue::Lang(value.clone(), lang.clone())
                } else {
                    match datatype {
                        None => BrdfValue::Plain(value.clone()),
                        Some(dt) if dt.as_str() == XSD_STRING => BrdfValue::Plain(value.clone()),
                        Some(dt) => BrdfValue::Datatype(value.clone(), dt.as_str().to_owned()),
                    }
                }
            }
            RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => BrdfValue::Triple(
                Box::new(BrdfValue::from_rdf_term(&t.subject)?),
                Box::new(BrdfValue::from_rdf_term(&t.predicate)?),
                Box::new(BrdfValue::from_rdf_term(&t.object)?),
            ),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Parser
// ═══════════════════════════════════════════════════════════════════════════

/// Binary RDF 1.0 parser (records → RDF triples → shared OWL mapping).
#[derive(Debug, Clone, Default)]
pub struct BinaryRdfParser;

impl BinaryRdfParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Parse a Binary RDF 1.0 byte stream into an [`Ontology`].
    ///
    /// Returns an error (never a panic) for truncated, mis-versioned or
    /// otherwise malformed input.
    pub fn parse(&self, data: &[u8]) -> Result<Ontology> {
        let doc = decode(data)?;

        let mut ontology = Ontology::new();
        // Namespaces first: later mappings may rely on prefix names.
        for (prefix, iri) in doc.namespaces {
            // Mirror the Turtle/N3 filter: only plain prefix names are kept
            // (skip e.g. `@prefix`-style markers or `foo:bar` keys).
            if !prefix.starts_with('@') && !prefix.contains(':') {
                ontology.prefixes.insert(prefix, IRI::new(&iri));
            }
        }
        for comment in doc.comments {
            let already = ontology.annotations.iter().any(|a| {
                a.property.iri.as_str() == RDFS_COMMENT
                    && matches!(&a.value, crate::ontology::AnnotationValue::Literal(l) if l.value == comment)
            });
            if !already {
                ontology.annotations.push(crate::ontology::Annotation {
                    property: crate::ontology::AnnotationProperty {
                        iri: IRI::new(RDFS_COMMENT),
                    },
                    value: crate::ontology::AnnotationValue::Literal(
                        crate::ontology::Literal::new(comment),
                    ),
                    annotations: vec![],
                });
            }
        }
        rdf_to_owl::map_rdf_triples(&doc.statements, &mut ontology)?;
        Ok(ontology)
    }
}

/// A decoded Binary RDF document (records before OWL mapping).
struct DecodedDocument {
    namespaces: Vec<(String, String)>,
    comments: Vec<String>,
    statements: Vec<Triple>,
}

/// Decode the record stream (header, records, `END_OF_DATA`).
fn decode(data: &[u8]) -> Result<DecodedDocument> {
    if data.len() < HEADER_LEN {
        return Err(Error::ParseError(format!(
            "BinaryRDF: truncated header: need {HEADER_LEN} bytes, got {}",
            data.len()
        )));
    }
    if &data[..4] != MAGIC {
        return Err(Error::ParseError(format!(
            "BinaryRDF: bad magic number: expected `BRDF`, got {:?}",
            String::from_utf8_lossy(&data[..4])
        )));
    }
    let version = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if version != VERSION {
        return Err(Error::ParseError(format!(
            "BinaryRDF: unsupported version {version} (this reader implements version {VERSION})"
        )));
    }

    let mut doc = DecodedDocument {
        namespaces: Vec::new(),
        comments: Vec::new(),
        statements: Vec::new(),
    };
    let mut values: HashMap<u64, BrdfValue> = HashMap::new();
    let mut pos = HEADER_LEN;
    let mut ended = false;

    while pos < data.len() {
        let record = data[pos];
        pos += 1;
        match record {
            NAMESPACE_DECL => {
                let prefix = read_string(data, &mut pos)?;
                let iri = read_string(data, &mut pos)?;
                doc.namespaces.push((prefix, iri));
            }
            COMMENT => {
                doc.comments.push(read_string(data, &mut pos)?);
            }
            VALUE_DECL => {
                let id = read_varint(data, &mut pos)?;
                let value = read_full_value(data, &mut pos, &values)?;
                if values.insert(id, value).is_some() {
                    return Err(Error::ParseError(format!(
                        "BinaryRDF: duplicate value declaration for id {id}"
                    )));
                }
            }
            STATEMENT => {
                let subject = read_slot(data, &mut pos, &values)?
                    .ok_or_else(|| Error::ParseError("BinaryRDF: null subject".to_owned()))?;
                let predicate = read_slot(data, &mut pos, &values)?
                    .ok_or_else(|| Error::ParseError("BinaryRDF: null predicate".to_owned()))?;
                let object = read_slot(data, &mut pos, &values)?
                    .ok_or_else(|| Error::ParseError("BinaryRDF: null object".to_owned()))?;
                // Context (4th slot): `NULL` = default graph; a named graph
                // is unioned into the ontology’s single triple set (an
                // `Ontology` carries no named-graph structure).
                let _context = read_slot(data, &mut pos, &values)?;
                doc.statements.push(Triple {
                    subject: subject.to_rdf_term()?,
                    predicate: predicate.to_rdf_term()?,
                    object: object.to_rdf_term()?,
                });
            }
            END_OF_DATA => {
                ended = true;
                if pos != data.len() {
                    return Err(Error::ParseError(format!(
                        "BinaryRDF: {} trailing byte(s) after END_OF_DATA",
                        data.len() - pos
                    )));
                }
                break;
            }
            other => {
                return Err(Error::ParseError(format!(
                    "BinaryRDF: unknown record type {other} at byte offset {}",
                    pos - 1
                )));
            }
        }
    }
    if !ended {
        return Err(Error::ParseError(
            "BinaryRDF: missing END_OF_DATA terminator".to_owned(),
        ));
    }
    Ok(doc)
}

/// Read one statement slot: `NULL`, `VALUE_REF`, or an inline full value.
/// Returns `None` for the `NULL` context marker.
fn read_slot(
    data: &[u8],
    pos: &mut usize,
    values: &HashMap<u64, BrdfValue>,
) -> Result<Option<BrdfValue>> {
    let Some(&value_type) = data.get(*pos) else {
        return Err(Error::ParseError(
            "BinaryRDF: unexpected end of data inside a statement".to_owned(),
        ));
    };
    *pos += 1;
    if value_type == NULL_VALUE {
        return Ok(None);
    }
    Ok(Some(read_owned_value(value_type, data, pos, values)?))
}

/// Decode a full value (type byte consumed) as an owned [`BrdfValue`].
fn read_owned_value(
    value_type: u8,
    data: &[u8],
    pos: &mut usize,
    values: &HashMap<u64, BrdfValue>,
) -> Result<BrdfValue> {
    match value_type {
        URI_VALUE => Ok(BrdfValue::Iri(read_string(data, pos)?)),
        BNODE_VALUE => Ok(BrdfValue::BNode(read_string(data, pos)?)),
        PLAIN_LITERAL_VALUE => Ok(BrdfValue::Plain(read_string(data, pos)?)),
        LANG_LITERAL_VALUE => {
            let label = read_string(data, pos)?;
            let lang = read_string(data, pos)?;
            Ok(BrdfValue::Lang(label, lang))
        }
        DATATYPE_LITERAL_VALUE => {
            let label = read_string(data, pos)?;
            let datatype = read_string(data, pos)?;
            Ok(BrdfValue::Datatype(label, datatype))
        }
        VALUE_REF => {
            let id = read_varint(data, pos)?;
            values.get(&id).cloned().ok_or_else(|| {
                Error::ParseError(format!("BinaryRDF: reference to undeclared value id {id}"))
            })
        }
        TRIPLE_VALUE => {
            let s = read_full_value(data, pos, values)?;
            let p = read_full_value(data, pos, values)?;
            let o = read_full_value(data, pos, values)?;
            Ok(BrdfValue::Triple(Box::new(s), Box::new(p), Box::new(o)))
        }
        NULL_VALUE => Err(Error::ParseError(
            "BinaryRDF: NULL value is only valid as a statement context".to_owned(),
        )),
        other => Err(Error::ParseError(format!(
            "BinaryRDF: unknown value type {other}"
        ))),
    }
}

/// Read a nested full value: its type byte *and* payload.
fn read_full_value(
    data: &[u8],
    pos: &mut usize,
    values: &HashMap<u64, BrdfValue>,
) -> Result<BrdfValue> {
    let Some(&value_type) = data.get(*pos) else {
        return Err(Error::ParseError(
            "BinaryRDF: unexpected end of data inside a value".to_owned(),
        ));
    };
    *pos += 1;
    read_owned_value(value_type, data, pos, values)
}

/// Read a length-prefixed UTF-8 string (varint byte length).
fn read_string(data: &[u8], pos: &mut usize) -> Result<String> {
    let len = read_varint(data, pos)?;
    let Ok(len) = usize::try_from(len) else {
        return Err(Error::ParseError(
            "BinaryRDF: string length does not fit into memory".to_owned(),
        ));
    };
    let end = pos
        .checked_add(len)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| {
            Error::ParseError(format!(
                "BinaryRDF: truncated string: {len} bytes requested at offset {pos}, only {} left",
                data.len() - *pos
            ))
        })?;
    let bytes = &data[*pos..end];
    *pos = end;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| Error::ParseError(format!("BinaryRDF: string is not valid UTF-8: {e}")))
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderer
// ═══════════════════════════════════════════════════════════════════════════

/// Binary RDF 1.0 renderer (OWL → RDF triples → records).
#[derive(Debug, Clone, Default)]
pub struct BinaryRdfRenderer;

impl BinaryRdfRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Serialize the ontology as a Binary RDF 1.0 byte stream.
    pub fn serialize(&self, ontology: &Ontology) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.serialize_to_bytes(&mut buf, ontology)?;
        Ok(buf)
    }

    /// Write the Binary RDF 1.0 byte stream for `ontology` to `out`
    /// (the byte-oriented writer API; no `String` round-trip, so varint
    /// bytes ≥ 0x80 are written losslessly).
    pub fn serialize_to_bytes(&self, out: &mut dyn Write, ontology: &Ontology) -> Result<()> {
        // ── Header: `BRDF` + big-endian i32 version = 1 ─────────────────
        out.write_all(MAGIC)
            .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
        out.write_all(&VERSION.to_be_bytes())
            .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;

        // ── NAMESPACE_DECL records (deterministic order) ────────────────
        let mut prefixes: Vec<(&String, &IRI)> = ontology.prefixes.iter().collect();
        prefixes.sort_by(|a, b| a.0.cmp(b.0));
        for (name, iri) in prefixes {
            out.write_all(&[NAMESPACE_DECL])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, name)?;
            write_string(out, iri.as_str())?;
        }

        // ── STATEMENT records (with VALUE_DECL records emitted ahead of
        //    the first use of each value) ────────────────────────────────
        let triples = ontology_to_triples(ontology)?;
        let mut declared: HashMap<BrdfValue, u64> = HashMap::new();
        let mut next_id: u64 = 0;
        for triple in &triples {
            for term in [&triple.subject, &triple.predicate, &triple.object] {
                let value = BrdfValue::from_rdf_term(term)?;
                if !declared.contains_key(&value) {
                    declared.insert(value.clone(), next_id);
                    out.write_all(&[VALUE_DECL])
                        .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
                    write_varint(out, next_id)?;
                    write_value(out, &value)?;
                    next_id += 1;
                }
            }
            out.write_all(&[STATEMENT])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            for term in [&triple.subject, &triple.predicate, &triple.object] {
                let value = BrdfValue::from_rdf_term(term)?;
                // Writer invariant: every referenced value was declared in
                // the dictionary pass above — surface a clear error instead
                // of panicking if it somehow was not (CR-011 n7).
                let id = *declared.get(&value).ok_or_else(|| Error::Internal {
                    message: format!(
                        "BinaryRDF: dictionary value missing from declaration pass: {value:?}"
                    ),
                })?;
                out.write_all(&[VALUE_REF])
                    .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
                write_varint(out, id)?;
            }
            // Context: an OWL ontology is a single (default) graph.
            out.write_all(&[NULL_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
        }

        // ── END_OF_DATA ─────────────────────────────────────────────────
        out.write_all(&[END_OF_DATA])
            .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
        Ok(())
    }
}

/// Write a length-prefixed UTF-8 string (varint byte length).
fn write_string(out: &mut dyn Write, s: &str) -> Result<()> {
    write_varint(out, s.len() as u64)?;
    out.write_all(s.as_bytes())
        .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))
}

/// Write a full value (type byte + payload).
fn write_value(out: &mut dyn Write, value: &BrdfValue) -> Result<()> {
    match value {
        BrdfValue::Iri(s) => {
            out.write_all(&[URI_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, s)
        }
        BrdfValue::BNode(id) => {
            out.write_all(&[BNODE_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, id)
        }
        BrdfValue::Plain(v) => {
            out.write_all(&[PLAIN_LITERAL_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, v)
        }
        BrdfValue::Lang(v, lang) => {
            out.write_all(&[LANG_LITERAL_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, v)?;
            write_string(out, lang)
        }
        BrdfValue::Datatype(v, dt) => {
            out.write_all(&[DATATYPE_LITERAL_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_string(out, v)?;
            write_string(out, dt)
        }
        BrdfValue::Triple(s, p, o) => {
            out.write_all(&[TRIPLE_VALUE])
                .map_err(|e| Error::io(format!("BinaryRDF: write failed: {e}")))?;
            write_value(out, s)?;
            write_value(out, p)?;
            write_value(out, o)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Free functions (registry entry points)
// ═══════════════════════════════════════════════════════════════════════════

/// Parse Binary RDF from a string slice (the string’s *bytes* are the
/// Binary RDF stream — byte callers should prefer
/// [`BinaryRdfParser::parse`]).
pub fn parse(content: &str) -> Result<Ontology> {
    BinaryRdfParser::new().parse(content.as_bytes())
}

/// Parse Binary RDF from raw bytes using the default parser.
pub fn parse_bytes(bytes: &[u8]) -> Result<Ontology> {
    BinaryRdfParser::new().parse(bytes)
}

/// Save an ontology to a file as Binary RDF 1.0 bytes.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let bytes = BinaryRdfRenderer::new().serialize(ontology)?;
    std::fs::write(path, bytes).map_err(|e| Error::io(format!("BinaryRDF: {e}")))
}
