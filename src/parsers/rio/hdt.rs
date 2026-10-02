//! HDT (Header Dictionary Triples) parser and renderer — B5 Slice 5.
//!
//! Delegates both directions to the **`hdt` crate** (hdt-rs by Konrad
//! Höffner, <https://crates.io/crates/hdt>), which implements the real
//! binary HDT format (<https://www.rdfhdt.org/hdt-binary-format/>): the
//! `$HDT` control-information chunks, the N-Triples header section, the
//! four-section plain-front-coded dictionary and the bitmap triples section.
//! The previous implementation was a bespoke plaintext “format” that only
//! shared the `$HDT` magic string — it could not read files produced by
//! hdt-cpp / hdt-java / hdt-rs, and neither could any real tool read its
//! output.
//!
//! * **Parse** — [`hdt::Hdt::read`] over the raw bytes, then iterate all
//!   triples with [`hdt::Hdt::triples_all`] (the crate’s native string-triple
//!   iterator).  Terms come back in the *HDT dictionary string format*:
//!   IRIs without angle brackets, literals with quotes (`"top class"@en`,
//!   `"42"^^<…integer>`), blank nodes as `_:b1`.  They are converted to
//!   [`RdfTerm`]s and mapped into the ontology by the **shared** RDF→OWL
//!   mapping ([`crate::parsers::rdf_to_owl::map_rdf_triples`]) — the same
//!   code Turtle / RDF/XML / JSON-LD / N3 use.
//! * **Serialize** — the ontology is converted to the canonical RDF triple
//!   set by the shared serializer-side emission
//!   ([`crate::semantics::owl_rdf_mapping::ontology_to_triples`]), formatted
//!   into HDT dictionary-string terms, and compiled to a real HDT file with
//!   [`hdt::Hdt::from_triples`] (the crate’s N-Triples/in-memory → HDT
//!   builder, behind its `nt` feature, which this crate enables) followed by
//!   [`hdt::Hdt::write`].
//!
//! ## `save_to_string` contract
//!
//! HDT is a binary format: a real HDT file is generally not valid UTF-8, so
//! [`crate::parsers::save_to_string`] **refuses** `OntologyFormat::Hdt` with
//! a clear error and `save_to_bytes` / `save_file` (byte-oriented I/O) is
//! the supported path.  The free [`parse`] entry point takes `&str` for API
//! compatibility and interprets the string’s bytes as an HDT file.
//!
//! ## Known limits (documented, never silent)
//!
//! * RDF-star / RDF 1.2 triple terms cannot be expressed in the HDT
//!   dictionary term grammar — serializing an ontology that contains one
//!   fails with a clear error instead of corrupting the file.
//! * HDT has no named-graph (quad) structure: statements are a single graph,
//!   matching the `Ontology` model.
//! * The `nt` builder feature is flagged *experimental* upstream; reading
//!   ([`hdt::Hdt::read`]) and writing ([`hdt::Hdt::write`]) use the crate’s
//!   stable, featureless API.

use std::io::{BufReader, Write};

use crate::ontology::{IRI, Ontology};
use crate::parsers::rdf_to_owl;
use crate::semantics::owl_rdf_mapping::ontology_to_triples;
use crate::semantics::{RdfTerm, Triple};
use crate::{Error, Result};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Fallback dataset base IRI written into the HDT header when the ontology
/// itself has no IRI.
const DEFAULT_BASE_IRI: &str = "urn:oxidowl:hdt";

// ═══════════════════════════════════════════════════════════════════════════
// Parser
// ═══════════════════════════════════════════════════════════════════════════

/// HDT parser (real binary HDT files via the `hdt` crate).
#[derive(Debug, Clone, Default)]
pub struct HdtParser;

impl HdtParser {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Parse an HDT document from a string slice (the string’s *bytes* are
    /// the HDT file — byte callers should prefer [`HdtParser::parse_bytes`]).
    pub fn parse(&self, content: &str) -> Result<Ontology> {
        self.parse_bytes(content.as_bytes())
    }

    /// Parse a real binary HDT file from raw bytes.
    ///
    /// Returns an error (never a panic) for truncated or malformed input.
    pub fn parse_bytes(&self, bytes: &[u8]) -> Result<Ontology> {
        let hdt = ::hdt::Hdt::read(BufReader::new(bytes))
            .map_err(|e| Error::ParseError(format!("HDT: failed to read HDT file: {e}")))?;

        let mut triples: Vec<Triple> = Vec::new();
        for [s, p, o] in hdt.triples_all() {
            triples.push(Triple {
                subject: hdt_term_to_rdf_term(&s)?,
                predicate: hdt_term_to_rdf_term(&p)?,
                object: hdt_term_to_rdf_term(&o)?,
            });
        }

        let mut ontology = Ontology::new();
        rdf_to_owl::map_rdf_triples(&triples, &mut ontology)?;
        Ok(ontology)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderer
// ═══════════════════════════════════════════════════════════════════════════

/// HDT renderer (OWL → RDF triples → real binary HDT file).
#[derive(Debug, Clone, Default)]
pub struct HdtRenderer;

impl HdtRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Serialize the ontology as a real binary HDT file (byte-oriented; a
    /// `String` representation of HDT does not exist — see the module docs).
    pub fn serialize_to_bytes(&self, ontology: &Ontology) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.serialize_to_writer(&mut buf, ontology)?;
        Ok(buf)
    }

    /// Write a real binary HDT file for `ontology` to `out`.
    pub fn serialize_to_writer(&self, out: &mut dyn Write, ontology: &Ontology) -> Result<()> {
        // ── 1. Ontology → canonical RDF triples (shared emission) ───────
        let triples = ontology_to_triples(ontology)?;

        // ── 2. RDF terms → HDT dictionary string terms ──────────────────
        let mut dict_triples: Vec<[String; 3]> = Vec::with_capacity(triples.len());
        for t in &triples {
            dict_triples.push([
                rdf_term_to_hdt_term(&t.subject)?,
                rdf_term_to_hdt_term(&t.predicate)?,
                rdf_term_to_hdt_term(&t.object)?,
            ]);
        }

        // ── 3. Build the HDT file (dictionary + bitmap triples) ─────────
        let base = ontology.get_iri().map_or(DEFAULT_BASE_IRI, IRI::as_str);
        let hdt = ::hdt::Hdt::from_triples(dict_triples, base)
            .map_err(|e| Error::ontology_parsing(format!("HDT: failed to build HDT: {e}")))?;
        // `hdt::Hdt::write` takes `&mut impl Write` (i.e. a `Sized` type),
        // so adapt the `dyn Write` sink to a concrete writer wrapper.
        struct DynWriter<'a>(&'a mut dyn Write);
        impl Write for DynWriter<'_> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.0.flush()
            }
        }
        hdt.write(&mut DynWriter(out))
            .map_err(|e| Error::io(format!("HDT: failed to write HDT file: {e}")))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HDT dictionary-term grammar
// ═══════════════════════════════════════════════════════════════════════════

/// Convert one HDT dictionary string term into an [`RdfTerm`].
///
/// Grammar (the format `Hdt::triples_all` returns): a bare IRI (no angle
/// brackets), a blank node `_:label`, or a literal
/// `"label"`, `"label"@lang` or `"label"^^<datatype-iri>` with N-Triples
/// escapes (`\" \\ \n \r \t \b \f \uXXXX \UXXXXXXXX`).
fn hdt_term_to_rdf_term(term: &str) -> Result<RdfTerm> {
    if let Some(label) = term.strip_prefix("_:") {
        return Ok(RdfTerm::BlankNode(label.to_owned()));
    }
    if term.starts_with('"') {
        return parse_hdt_literal(term);
    }
    url::Url::parse(term).map(RdfTerm::Iri).map_err(|e| {
        Error::ParseError(format!("HDT: invalid IRI in dictionary term '{term}': {e}"))
    })
}

/// Parse an HDT dictionary literal into a literal [`RdfTerm`].
fn parse_hdt_literal(term: &str) -> Result<RdfTerm> {
    let bytes = term.as_bytes();
    let mut value: Vec<u8> = Vec::with_capacity(term.len());
    let mut i = 1; // skip opening quote
    let mut closed: Option<usize> = None;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                let escape = *bytes.get(i).ok_or_else(|| {
                    Error::ParseError(format!("HDT: dangling escape in literal '{term}'"))
                })?;
                match escape {
                    b'n' => value.push(b'\n'),
                    b'r' => value.push(b'\r'),
                    b't' => value.push(b'\t'),
                    b'b' => value.push(0x08),
                    b'f' => value.push(0x0C),
                    b'"' => value.push(b'"'),
                    b'\'' => value.push(b'\''),
                    b'\\' => value.push(b'\\'),
                    b'u' | b'U' => {
                        let digits = if escape == b'u' { 4 } else { 8 };
                        let hex = term.get(i + 1..i + 1 + digits).ok_or_else(|| {
                            Error::ParseError(format!("HDT: truncated \\u escape in '{term}'"))
                        })?;
                        let cp = u32::from_str_radix(hex, 16).map_err(|e| {
                            Error::ParseError(format!(
                                "HDT: bad \\u escape '{hex}' in '{term}': {e}"
                            ))
                        })?;
                        let ch = char::from_u32(cp).ok_or_else(|| {
                            Error::ParseError(format!(
                                "HDT: invalid code point U+{cp:04X} in '{term}'"
                            ))
                        })?;
                        let mut buf = [0u8; 4];
                        value.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        i += digits;
                    }
                    other => {
                        return Err(Error::ParseError(format!(
                            "HDT: unknown escape `\\{}` in literal '{term}'",
                            char::from(other)
                        )));
                    }
                }
                i += 1;
            }
            b'"' => {
                closed = Some(i);
                break;
            }
            b => {
                value.push(b);
                i += 1;
            }
        }
    }
    let Some(quote) = closed else {
        return Err(Error::ParseError(format!(
            "HDT: unterminated literal '{term}'"
        )));
    };
    let value = String::from_utf8(value).map_err(|e| {
        Error::ParseError(format!("HDT: literal in '{term}' is not valid UTF-8: {e}"))
    })?;

    let rest = &term[quote + 1..];
    if rest.is_empty() {
        // Plain literal == xsd:string in RDF 1.1; the OWL model stores it
        // with `datatype: None` (the shared mapping normalizes the rest).
        return Ok(RdfTerm::Literal {
            value,
            datatype: None,
            language: None,
            direction: None,
        });
    }
    if let Some(lang) = rest.strip_prefix('@') {
        return Ok(RdfTerm::Literal {
            value,
            datatype: None,
            language: Some(lang.to_owned()),
            direction: None,
        });
    }
    if let Some(dt) = rest.strip_prefix("^^<").and_then(|r| r.strip_suffix('>')) {
        let datatype = url::Url::parse(dt).map_err(|e| {
            Error::ParseError(format!(
                "HDT: invalid literal datatype '{dt}' in '{term}': {e}"
            ))
        })?;
        let datatype = if dt == XSD_STRING {
            None
        } else {
            Some(datatype)
        };
        return Ok(RdfTerm::Literal {
            value,
            datatype,
            language: None,
            direction: None,
        });
    }
    Err(Error::ParseError(format!(
        "HDT: malformed literal suffix '{rest}' in '{term}'"
    )))
}

/// Convert an [`RdfTerm`] into an HDT dictionary string term (inverse of
/// [`hdt_term_to_rdf_term`]).
fn rdf_term_to_hdt_term(term: &RdfTerm) -> Result<String> {
    match term {
        RdfTerm::Iri(url) => Ok(url.as_str().to_owned()),
        RdfTerm::BlankNode(id) => Ok(format!("_:{}", id.strip_prefix("_:").unwrap_or(id))),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let mut out = format!("\"{}\"", escape_hdt_string(value));
            if let Some(lang) = language {
                out.push('@');
                out.push_str(lang);
            } else if let Some(dt) = datatype
                && dt.as_str() != XSD_STRING
            {
                out.push_str("^^<");
                out.push_str(dt.as_str());
                out.push('>');
            }
            Ok(out)
        }
        RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_) => Err(Error::ontology_parsing(
            "HDT cannot represent RDF-star / RDF 1.2 triple terms \
             (no dictionary-term grammar for them)",
        )),
    }
}

/// N-Triples-style string escaping for HDT dictionary literals.
fn escape_hdt_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════════
// Free functions (registry entry points)
// ═══════════════════════════════════════════════════════════════════════════

/// Parse an HDT document from a string slice (the string’s *bytes* are the
/// HDT file — byte callers should prefer [`parse_bytes`]).
pub fn parse(content: &str) -> Result<Ontology> {
    HdtParser::new().parse(content)
}

/// Parse a real binary HDT file from raw bytes using the default parser.
pub fn parse_bytes(bytes: &[u8]) -> Result<Ontology> {
    HdtParser::new().parse_bytes(bytes)
}

/// Save an ontology to a file as a real binary HDT file.
pub fn save_file<P: AsRef<std::path::Path>>(ontology: &Ontology, path: P) -> Result<()> {
    let bytes = HdtRenderer::new().serialize_to_bytes(ontology)?;
    std::fs::write(path, bytes).map_err(|e| Error::io(format!("HDT: {e}")))
}
