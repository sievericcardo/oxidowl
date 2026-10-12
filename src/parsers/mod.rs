//! Ontology Parsers and Serializers Module
//!
//! This module contains parsers and serializers for various OWL 2 DL formats.

pub mod common;
pub mod dl_syntax;
pub mod functional;
pub mod krss;
pub mod latex;
pub mod manchester;
pub mod manchester_renderer;
pub mod ntriples;
pub mod obo;
pub mod owl_xml;
pub mod rdf_to_owl;
pub mod rdf_xml;
pub mod rio;
pub mod turtle;
pub mod validation;

/// RDF compatibility mode for parsing RDF-based formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RdfCompatibilityMode {
    /// RDF 1.1 mode - standard reification only
    RDF11,
    /// RDF 1.2 mode - rdf:reifies support
    RDF12,
    /// RDF-star mode - quoted triples with << >>
    RDFStar,
    /// Auto-detect based on content
    #[default]
    Auto,
}

/// Error verbosity level for parser error messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorVerbosity {
    /// Minimal: just the error message
    Minimal,
    /// Standard: message + line/column information
    #[default]
    Standard,
    /// Detailed: full context stack and token information
    Detailed,
}

/// Parser configuration
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Error verbosity level
    pub error_verbosity: ErrorVerbosity,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            error_verbosity: ErrorVerbosity::Standard,
        }
    }
}

impl ParserConfig {
    /// Create a new parser configuration with minimal error verbosity
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            error_verbosity: ErrorVerbosity::Minimal,
        }
    }

    /// Create a new parser configuration with standard error verbosity
    #[must_use]
    pub fn standard() -> Self {
        Self {
            error_verbosity: ErrorVerbosity::Standard,
        }
    }

    /// Create a new parser configuration with detailed error verbosity
    #[must_use]
    pub fn detailed() -> Self {
        Self {
            error_verbosity: ErrorVerbosity::Detailed,
        }
    }
}

// Re-export parser structs and functions
pub use common::{OntologyParser, OntologySerializer, SerializerConfig};
pub use dl_syntax::{DLSyntaxParser, DLSyntaxRenderer};
pub use functional::{
    FunctionalParser, FunctionalSyntaxSerializer, parse as parse_functional,
    parse_file as parse_functional_file, save_file as save_functional_file,
};
pub use krss::{KRSSParser, KRSSRenderer, KRSSVariant};
pub use latex::LatexRenderer;
pub use manchester::{ManchesterParser, ManchesterParserConfig};
pub use manchester_renderer::ManchesterRenderer;
pub use ntriples::{
    NTriplesParser, NTriplesSerializer, parse as parse_ntriples, parse_file as parse_ntriples_file,
    save_file as save_ntriples_file,
};
pub use owl_xml::{
    OwlXmlParser, OwlXmlSerializer, parse as parse_owl_xml, parse_file as parse_owl_xml_file,
    save_file as save_owl_xml_file,
};
pub use rdf_xml::{
    RdfXmlParser, RdfXmlSerializer, parse as parse_rdf_xml, parse_file as parse_rdf_xml_file,
    save_file as save_rdf_xml_file,
};
pub use turtle::{
    TurtleParser, TurtleSerializer, parse as parse_turtle, parse_file as parse_turtle_file,
    save_file as save_turtle_file,
};
pub use validation::SyntaxValidator;

#[cfg(test)]
mod phase2_tests;

use crate::{
    Error, Result,
    ontology::{Ontology, OntologyFormat},
};
use std::path::Path;

/// Extract the first section from a `CrossSyntax` multi-format file
/// Returns the content of the first section (after the ### marker)
#[must_use]
pub fn extract_first_crosssyntax_section(content: &str) -> String {
    let trimmed = content.trim();

    // Check if this is a CrossSyntax file
    if !trimmed.starts_with("###") {
        return content.to_string();
    }

    let mut result = String::new();
    let mut in_first_section = false;
    let mut section_count = 0;

    for line in content.lines() {
        let line_trimmed = line.trim();

        if line_trimmed.starts_with("###") {
            // Skip comment lines like "### invalid ..." or "### valid ..."
            // These don't represent actual syntax sections
            let after_hash = line_trimmed.trim_start_matches("###").trim();
            if after_hash.starts_with("invalid") || after_hash.starts_with("valid") {
                continue;
            }

            section_count += 1;

            if section_count == 1 {
                // This is the first actual section marker, skip it and start collecting
                in_first_section = true;
                continue;
            }
            // We've reached the second section, stop
            break;
        }
        if in_first_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Detect format from file content for ambiguous cases
fn detect_format_from_content<P: AsRef<Path>>(path: P) -> Result<OntologyFormat> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| Error::io(format!("Failed to read file for format detection: {e}")))?;

    let trimmed = content.trim();

    // Check for CrossSyntax multi-format files
    // These files start with ### followed by a format name
    if trimmed.starts_with("###") || content.contains("\n###") {
        // CrossSyntax files need special handling - we'll parse the first section
        // Find the first actual format marker (skip comment lines like "### invalid ..." or "### valid ...")
        for line in trimmed.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.starts_with("###") {
                // Skip comment lines - these don't represent actual syntax sections
                let after_hash = line_trimmed.trim_start_matches("###").trim();
                if after_hash.starts_with("invalid") || after_hash.starts_with("valid") {
                    continue;
                }

                // Extract format name from first non-comment ### line
                let format_name = after_hash.to_lowercase();
                return match format_name.as_str() {
                    "turtle" => Ok(OntologyFormat::Turtle),
                    "rdf/xml" | "rdf-xml" => Ok(OntologyFormat::RdfXml),
                    "owl/xml" | "owl-xml" => Ok(OntologyFormat::OwlXml),
                    "functional" => Ok(OntologyFormat::Functional),
                    "manchester" => Ok(OntologyFormat::Manchester),
                    _ => Ok(OntologyFormat::Functional), // Default fallback
                };
            }
        }
    }

    // Check for Functional syntax - more patterns
    // Functional syntax uses parentheses and specific keywords
    if trimmed.starts_with("Ontology(")
        || trimmed.starts_with("Prefix(")
        || trimmed.starts_with("Import(")
        || content.contains("Declaration(")
        || content.contains("SubClassOf(")
        || (trimmed.starts_with("Import(") && content.contains("Ontology("))
    {
        return Ok(OntologyFormat::Functional);
    }

    // Check for Manchester syntax - expanded patterns
    // Manchester uses colons after keywords
    if trimmed.starts_with("Prefix:")
        || trimmed.starts_with("Ontology:")
        || trimmed.starts_with("Class:")
        || trimmed.starts_with("ObjectProperty:")
        || trimmed.starts_with("DataProperty:")
        || trimmed.starts_with("Individual:")
        || trimmed.starts_with("Import:")
        || content.contains("\nClass:")
        || content.contains("\nObjectProperty:")
        || content.contains("\nDataProperty:")
    {
        return Ok(OntologyFormat::Manchester);
    }

    // Check for XML-based formats before Turtle.
    // RDF/XML and OWL/XML documents may use rdf:type and owl: prefixes, which
    // would otherwise match the Turtle heuristics below. An XML declaration or
    // RDF/XML/OWL root element is unambiguous and never starts a Turtle file.
    if trimmed.starts_with("<?xml")
        || trimmed.starts_with("<rdf:RDF")
        || trimmed.starts_with("<RDF")
        || trimmed.starts_with("<owl:")
    {
        // RDF/XML is identified by the rdf:RDF root element. Check this first,
        // since RDF/XML files may also use owl: and rdf:type prefixes.
        if content.contains("rdf:RDF") || content.contains("<rdf:RDF") || content.contains("<RDF") {
            return Ok(OntologyFormat::RdfXml);
        }
        return Ok(OntologyFormat::OwlXml);
    }

    // Check for Turtle syntax
    // Turtle uses @prefix and @base directives
    if trimmed.starts_with("@prefix")
        || trimmed.starts_with("@base")
        || (content.contains("@prefix") && content.contains("<http"))
        || (content.contains("rdf:type") && content.contains("owl:"))
    {
        return Ok(OntologyFormat::Turtle);
    }

    // Check for remaining XML-based formats (OWL/XML documents without an XML
    // declaration that use unprefixed elements such as <Ontology or <Class)
    if trimmed.starts_with('<') {
        let owl_xml_elements = [
            "<Ontology",
            "<Declaration",
            "<Class",
            "<ObjectProperty",
            "<DataProperty",
            "<AnnotationProperty",
            "<Individual",
            "<Import",
        ];
        if owl_xml_elements.iter().any(|&elem| content.contains(elem)) {
            return Ok(OntologyFormat::OwlXml);
        }
        // Default to OWL/XML for XML files (safer than RDF/XML)
        return Ok(OntologyFormat::OwlXml);
    }

    // For .txt files or unknown content, try heuristics
    // Look for common OWL patterns to guess the format

    // Check if it looks like Functional syntax (has parentheses and OWL keywords)
    let functional_keywords = [
        "Declaration",
        "SubClassOf",
        "EquivalentClasses",
        "DisjointClasses",
    ];
    if functional_keywords.iter().any(|&kw| content.contains(kw)) && content.contains('(') {
        return Ok(OntologyFormat::Functional);
    }

    // Check if it looks like Manchester (colon-based declarations)
    let manchester_keywords = [
        "Class:",
        "ObjectProperty:",
        "DataProperty:",
        "SubClassOf:",
        "EquivalentTo:",
    ];
    if manchester_keywords.iter().any(|&kw| content.contains(kw)) {
        return Ok(OntologyFormat::Manchester);
    }

    // Check if it looks like Turtle (prefix declarations and triples)
    if content.contains("@prefix") || (content.contains(':') && content.contains('.')) {
        return Ok(OntologyFormat::Turtle);
    }

    // Default to Functional syntax for unknown content
    Ok(OntologyFormat::Functional)
}

/// Detect RDF version from content based on syntactic markers
fn detect_rdf_version_from_content(content: &str) -> RdfCompatibilityMode {
    // Check for RDF-star syntax (quoted triples with << >>)
    // Look for << followed by content and >> on the same or nearby lines
    if content.contains("<<") && content.contains(">>") {
        // More robust check: ensure these are not just in comments or strings
        let lines: Vec<&str> = content.lines().collect();
        for line in &lines {
            let trimmed = line.trim();
            // Skip comment lines
            if trimmed.starts_with('#') {
                continue;
            }
            // Check for << >> pattern that looks like RDF-star
            if trimmed.contains("<<") && trimmed.contains(">>") {
                return RdfCompatibilityMode::RDFStar;
            }
        }
    }

    // Check for RDF 1.2 rdf:reifies predicate
    if content.contains("rdf:reifies") || content.contains("<rdf:reifies") {
        return RdfCompatibilityMode::RDF12;
    }

    // Check for RDF 1.1 standard reification vocabulary
    // These alone don't indicate RDF 1.2, so default to RDF 1.1
    if content.contains("rdf:Statement")
        || content.contains("rdf:subject")
        || content.contains("rdf:predicate")
        || content.contains("rdf:object")
    {
        return RdfCompatibilityMode::RDF11;
    }

    // Default to auto-detection (let the parser decide)
    RdfCompatibilityMode::Auto
}

/// Unambiguous file extension → format table, shared by [`parse_file_auto`]
/// and the content-detection entry points ([`detect_format_from_content_public`]
/// / [`detect_format_from_bytes`]).
///
/// Every auto-detecting entry point — including
/// `OntologyLoader::load_file` — consults this table first, so a file whose
/// extension is unambiguous always parses the way `parse_file_auto` routes
/// it (CR-011 F-3: an XML-shaped HTML fragment in `page.html` used to be
/// content-sniffed as OWL/XML and silently yielded 0 axioms; it now parses
/// as `RDFa`, exactly as `parse_file_auto` does).
///
/// Returns `None` for extensions that must fall through to content
/// sniffing: `owl`, `owx`, `xml`, `txt`, and unknown/absent extensions.
#[must_use]
pub fn format_from_extension(extension: &str) -> Option<OntologyFormat> {
    match extension.to_lowercase().as_str() {
        "rdf" => Some(OntologyFormat::RdfXml),
        "ttl" => Some(OntologyFormat::Turtle),
        "nt" => Some(OntologyFormat::NTriples),
        "ofn" => Some(OntologyFormat::Functional),
        "omn" | "man" => Some(OntologyFormat::Manchester),
        "swrl" => Some(OntologyFormat::Functional), // SWRL uses functional-like syntax
        // ── B5 formats (format registry) ────────────────────────────────
        "jsonld" | "json-ld" => Some(OntologyFormat::JsonLd), // only `.jsonld`, not generic `.json`
        "n3" => Some(OntologyFormat::N3),
        "nq" | "nquads" => Some(OntologyFormat::NQuads),
        "trig" => Some(OntologyFormat::TriG),
        "trix" => Some(OntologyFormat::TriX),
        "rdfa" | "html" | "xhtml" => Some(OntologyFormat::Rdfa),
        "brdf" | "binaryrdf" => Some(OntologyFormat::BinaryRdf),
        "hdt" => Some(OntologyFormat::Hdt),
        "rj" | "rdfjson" | "rjson" => Some(OntologyFormat::RdfJson),
        _ => None,
    }
}

/// Auto-detect format and parse file
pub fn parse_file_auto<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let format = match extension.to_lowercase().as_str() {
        // .owl files can be OWL/XML, OWL Functional Syntax, or RDF/XML — detect from content
        "owl" | "owx" | "xml" | "txt" => detect_format_from_content(path)?,
        // Unambiguous extensions come from the shared table (so detection
        // and parse_file_auto cannot disagree); unknown extensions keep the
        // historical OWL/XML fallback.
        other => format_from_extension(other).unwrap_or(OntologyFormat::OwlXml),
    };

    // Binary formats carry non-UTF-8 bytes (BinaryRDF varints ≥ 0x80, HDT
    // binary payloads) — reading them with `read_to_string` would fail, so
    // they get a byte-oriented path.
    if matches!(format, OntologyFormat::BinaryRdf | OntologyFormat::Hdt) {
        let bytes =
            std::fs::read(path).map_err(|e| Error::io(format!("Failed to read file: {e}")))?;
        return parse_bytes(&bytes, format);
    }

    // Read the file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::io(format!("Failed to read file: {e}")))?;

    parse_str(&content, format)
}

/// Parse a UTF-8 ontology document in the given (already resolved) format.
///
/// Handles `CrossSyntax` section extraction and RDF-version sniffing before
/// dispatching to the format's parser.  Shared by [`parse_file_auto`],
/// [`parse_bytes`] and the [`ParserFactory`] bridge.
pub fn parse_str(content: &str, format: OntologyFormat) -> Result<Ontology> {
    // Check if this is a CrossSyntax file and extract first section
    let parsed_content = if content.trim().starts_with("###") {
        extract_first_crosssyntax_section(content)
    } else {
        content.to_string()
    };

    // Detect RDF version for RDF-based formats
    let rdf_version = detect_rdf_version_from_content(content);

    match format {
        OntologyFormat::OwlXml => {
            let parser = owl_xml::OwlXmlParser::new();
            parser.parse(&parsed_content)
        }
        OntologyFormat::Functional => {
            let parser = functional::FunctionalParser::new();
            parser.parse(&parsed_content)
        }
        OntologyFormat::RdfXml => {
            // Configure parser based on detected RDF version
            let config = match rdf_version {
                RdfCompatibilityMode::RDF11 => rdf_xml::RdfXmlParserConfig {
                    rdf_version: rdf_xml::RdfVersionMode::RDF11,
                    ..Default::default()
                },
                RdfCompatibilityMode::RDF12 => rdf_xml::RdfXmlParserConfig {
                    rdf_version: rdf_xml::RdfVersionMode::RDF12,
                    parse_rdf_reifies: true,
                    ..Default::default()
                },
                RdfCompatibilityMode::RDFStar => rdf_xml::RdfXmlParserConfig {
                    rdf_version: rdf_xml::RdfVersionMode::RDF12,
                    reification_mode: rdf_xml::ReificationMode::ConvertToQuotedTriples,
                    ..Default::default()
                },
                RdfCompatibilityMode::Auto => rdf_xml::RdfXmlParserConfig::default(),
            };
            let parser = rdf_xml::RdfXmlParser::with_config(config);
            parser.parse(&parsed_content)
        }
        OntologyFormat::Turtle => {
            // Configure parser based on detected RDF version
            let config = match rdf_version {
                RdfCompatibilityMode::RDF11 => turtle::TurtleParserConfig {
                    rdf_version: turtle::RdfVersionMode::RDF11,
                    parse_rdf_star: false,
                    strict_rdf11_mode: true,
                    ..Default::default()
                },
                RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => {
                    turtle::TurtleParserConfig {
                        rdf_version: turtle::RdfVersionMode::RDFStar,
                        parse_rdf_star: true,
                        strict_rdf11_mode: false,
                        ..Default::default()
                    }
                }
                RdfCompatibilityMode::Auto => turtle::TurtleParserConfig::default(),
            };
            let parser = turtle::TurtleParser::with_config(config);
            parser.parse(&parsed_content)
        }
        OntologyFormat::NTriples => {
            // Configure parser based on detected RDF version
            let config = match rdf_version {
                RdfCompatibilityMode::RDF11 => ntriples::NTriplesConfig {
                    rdf_version: ntriples::RdfVersionMode::RDF11,
                    parse_rdf_star: false,
                    strict_rdf11_mode: true,
                    iri_validation_mode: crate::semantics::IriValidationMode::RFC3986,
                    validate_blank_nodes: false, // Lenient for compatibility
                },
                RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => {
                    ntriples::NTriplesConfig {
                        rdf_version: ntriples::RdfVersionMode::RDFStar,
                        parse_rdf_star: true,
                        strict_rdf11_mode: false,
                        iri_validation_mode: crate::semantics::IriValidationMode::RFC3987,
                        validate_blank_nodes: false, // Lenient for compatibility
                    }
                }
                RdfCompatibilityMode::Auto => ntriples::NTriplesConfig::default(),
            };
            let parser = ntriples::NTriplesParser::with_config(config);
            parser.parse(&parsed_content)
        }
        OntologyFormat::Manchester => {
            let parser = ManchesterParser::new(ManchesterParserConfig::default());
            parser.parse(&parsed_content)
        }
        OntologyFormat::DL => dl_syntax::parse(&parsed_content),
        OntologyFormat::Krss => krss::parse(&parsed_content),
        OntologyFormat::Krss2 => krss::parse_krss2(&parsed_content),
        OntologyFormat::Latex => Err(Error::ontology_parsing(
            "LaTeX is a write-only format (no parser available)",
        )),
        OntologyFormat::Obo => obo::parse(&parsed_content),
        OntologyFormat::NQuads => rio::nquads::parse(&parsed_content),
        OntologyFormat::N3 => rio::n3::parse(&parsed_content),
        OntologyFormat::TriG => rio::trig::parse(&parsed_content),
        OntologyFormat::TriX => rio::trix::parse(&parsed_content),
        OntologyFormat::JsonLd => rio::jsonld::parse(&parsed_content),
        OntologyFormat::RdfJson => rio::rdf_json::parse(&parsed_content),
        OntologyFormat::Rdfa => rio::rdfa::parse(&parsed_content),
        OntologyFormat::BinaryRdf => rio::binary_rdf::parse(&parsed_content),
        // Binary formats reached through the text entry point: the string's
        // *bytes* are the file payload.  Real HDT/BinaryRDF files are rarely
        // valid UTF-8 — `parse_bytes` is the supported path for them.
        OntologyFormat::Hdt => rio::hdt::parse(&parsed_content),
        OntologyFormat::Auto => Err(Error::ontology_parsing(
            "Auto format should have been resolved",
        )),
    }
}

/// Parse raw bytes in the given format.
///
/// `BinaryRdf` and `Hdt` are consumed as bytes and never go through a UTF-8
/// decode (their payloads are genuinely binary: `BinaryRDF` varints ≥ 0x80,
/// HDT control/dictionary/bitmap sections).  Every other format must be
/// valid UTF-8 and is parsed from its decoded text via [`parse_str`].
pub fn parse_bytes(bytes: &[u8], format: OntologyFormat) -> Result<Ontology> {
    match format {
        OntologyFormat::BinaryRdf => rio::binary_rdf::BinaryRdfParser::new().parse(bytes),
        OntologyFormat::Hdt => rio::hdt::HdtParser::new().parse_bytes(bytes),
        _ => {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| Error::io(format!("Ontology content is not valid UTF-8: {e}")))?;
            parse_str(text, format)
        }
    }
}

/// Public wrapper for format detection from content and path hint.
/// Used by the `OntologyLoader` and other consumers.
///
/// The path is no longer a mere hint: unambiguous extensions are resolved
/// through the shared [`format_from_extension`] table first — exactly what
/// [`parse_file_auto`] does — and only ambiguous/unknown extensions fall
/// through to the content heuristics (CR-011 F-3).
pub fn detect_format_from_content_public(
    path: &std::path::Path,
    content: &str,
) -> crate::Result<OntologyFormat> {
    // ── Extension first (CR-011 F-3): trust unambiguous extensions the
    //    same way `parse_file_auto` does, so `OntologyLoader::load_file`
    //    and friends cannot disagree with it.  Binary magic below still
    //    wins for `.brdf`/`.hdt` payloads routed through
    //    `detect_format_from_bytes` first; for text content the extension
    //    table agrees with the magic checks (`brdf`/`hdt`/`rdfa`…).
    if let Some(ext) = path.extension().and_then(std::ffi::OsStr::to_str)
        && let Some(format) = format_from_extension(ext)
    {
        return Ok(format);
    }

    let trimmed = content.trim();

    // ── B5 magic / structure detection (checked first — these are unambiguous
    //    byte patterns that must win over the generic text heuristics below) ──

    // HDT: `$HDT` magic — real binary HDT files start with these 4 bytes
    // (byte-exact detection uses `detect_format_from_bytes`); a UTF-8 view
    // that begins with `$HDT` is detected here as well.
    if trimmed.starts_with("$HDT") {
        return Ok(OntologyFormat::Hdt);
    }
    // Binary RDF: `BRDF` magic header (ASCII, so it survives a UTF-8 decode
    // of the leading bytes; byte-exact detection uses `detect_format_from_bytes`).
    if trimmed.starts_with("BRDF") {
        return Ok(OntologyFormat::BinaryRdf);
    }
    // HTML / XHTML → RDFa (before the generic `<…>` → OWL/XML fallback).
    {
        let head: String = trimmed
            .chars()
            .take(64)
            .collect::<String>()
            .to_ascii_lowercase();
        if head.starts_with("<!doctype html") || head.starts_with("<html") {
            return Ok(OntologyFormat::Rdfa);
        }
    }
    // JSON-LD: a JSON object carrying `@context` / `@graph` structure.
    if trimmed.starts_with('{')
        && (content.contains("\"@context\"") || content.contains("\"@graph\""))
    {
        return Ok(OntologyFormat::JsonLd);
    }

    if trimmed.starts_with("###") || content.contains("\n###") {
        for line in trimmed.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.starts_with("###") {
                let after_hash = line_trimmed.trim_start_matches("###").trim();
                if after_hash.starts_with("invalid") || after_hash.starts_with("valid") {
                    continue;
                }
                let format_name = after_hash.to_lowercase();
                return match format_name.as_str() {
                    "turtle" => Ok(OntologyFormat::Turtle),
                    "rdf/xml" | "rdf-xml" => Ok(OntologyFormat::RdfXml),
                    "owl/xml" | "owl-xml" => Ok(OntologyFormat::OwlXml),
                    "functional" => Ok(OntologyFormat::Functional),
                    "manchester" => Ok(OntologyFormat::Manchester),
                    _ => Ok(OntologyFormat::Functional),
                };
            }
        }
    }

    if trimmed.starts_with("Ontology(")
        || trimmed.starts_with("Prefix(")
        || trimmed.starts_with("Import(")
        || content.contains("Declaration(")
        || content.contains("SubClassOf(")
        || (trimmed.starts_with("Import(") && content.contains("Ontology("))
    {
        return Ok(OntologyFormat::Functional);
    }
    if trimmed.starts_with("Prefix:")
        || trimmed.starts_with("Ontology:")
        || trimmed.starts_with("Class:")
        || trimmed.starts_with("ObjectProperty:")
        || trimmed.starts_with("DataProperty:")
        || trimmed.starts_with("Individual:")
        || content.contains("\nClass:")
        || content.contains("\nObjectProperty:")
    {
        return Ok(OntologyFormat::Manchester);
    }
    if trimmed.starts_with("<?xml")
        || trimmed.starts_with("<rdf:RDF")
        || trimmed.starts_with("<RDF")
        || trimmed.starts_with("<owl:")
    {
        if content.contains("rdf:RDF") || content.contains("<rdf:RDF") || content.contains("<RDF") {
            return Ok(OntologyFormat::RdfXml);
        }
        // XHTML: an XML declaration in front of an HTML document → RDFa.
        if content.contains("<!DOCTYPE html")
            || content.contains("<html")
            || content.contains("<HTML")
        {
            return Ok(OntologyFormat::Rdfa);
        }
        return Ok(OntologyFormat::OwlXml);
    }
    if trimmed.starts_with("@prefix")
        || trimmed.starts_with("@base")
        || (content.contains("@prefix") && content.contains("<http"))
        || (content.contains("rdf:type") && content.contains("owl:"))
    {
        return Ok(OntologyFormat::Turtle);
    }
    if trimmed.starts_with('<') {
        if content.contains("<Ontology")
            || content.contains("<Declaration")
            || content.contains("<Class")
        {
            return Ok(OntologyFormat::OwlXml);
        }
        if content.contains("rdf:RDF") {
            return Ok(OntologyFormat::RdfXml);
        }
        return Ok(OntologyFormat::OwlXml);
    }

    // Heuristic fallbacks
    if (content.contains("Declaration") || content.contains("SubClassOf")) && content.contains('(')
    {
        return Ok(OntologyFormat::Functional);
    }
    if content.contains("Class:")
        || content.contains("ObjectProperty:")
        || content.contains("EquivalentTo:")
    {
        return Ok(OntologyFormat::Manchester);
    }
    if content.contains("@prefix") || (content.contains(':') && content.contains('.')) {
        return Ok(OntologyFormat::Turtle);
    }

    Ok(OntologyFormat::Functional)
}

/// Detect the format of a document from its raw bytes.
///
/// Binary magic bytes are checked first (`BRDF`, `$HDT`); otherwise the
/// content must be valid UTF-8 and is delegated to
/// [`detect_format_from_content_public`].
pub fn detect_format_from_bytes(
    path: &std::path::Path,
    bytes: &[u8],
) -> crate::Result<OntologyFormat> {
    if bytes.starts_with(b"BRDF") {
        return Ok(OntologyFormat::BinaryRdf);
    }
    if bytes.starts_with(b"$HDT") {
        return Ok(OntologyFormat::Hdt);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => detect_format_from_content_public(path, text),
        Err(e) => Err(Error::io(format!(
            "Cannot detect format: content is neither a known binary format nor valid UTF-8 ({e})"
        ))),
    }
}

/// Save ontology to a string using the specified format
pub fn save_to_string(ontology: &Ontology, format: OntologyFormat) -> Result<String> {
    match format {
        OntologyFormat::OwlXml => OwlXmlSerializer::new().serialize(ontology),
        OntologyFormat::Functional => FunctionalSyntaxSerializer.serialize(ontology),
        OntologyFormat::RdfXml => RdfXmlSerializer::new().serialize(ontology),
        OntologyFormat::Turtle => TurtleSerializer::new().serialize(ontology),
        OntologyFormat::NTriples => NTriplesSerializer::new().serialize(ontology),
        OntologyFormat::Manchester => {
            let renderer = manchester_renderer::ManchesterRenderer::new();
            renderer.serialize(ontology)
        }
        OntologyFormat::Latex => latex::serialize(ontology),
        OntologyFormat::DL => dl_syntax::render_to_string(ontology, true),
        OntologyFormat::Krss => krss::render_to_string(ontology),
        OntologyFormat::Krss2 => krss::render_to_string_krss2(ontology),
        OntologyFormat::Obo => {
            let writer = obo::OBOWriter::new(obo::OBOOutputConfig::default());
            Ok(writer.serialize(ontology))
        }
        OntologyFormat::NQuads => rio::nquads::NQuadsRenderer.serialize(ontology),
        OntologyFormat::N3 => rio::n3::N3Renderer.serialize(ontology),
        OntologyFormat::TriG => rio::trig::TriGRenderer.serialize(ontology),
        OntologyFormat::TriX => rio::trix::TriXRenderer.serialize(ontology),
        OntologyFormat::JsonLd => rio::jsonld::JsonLdRenderer.serialize(ontology),
        OntologyFormat::RdfJson => rio::rdf_json::RdfJsonRenderer.serialize(ontology),
        OntologyFormat::Rdfa => rio::rdfa::RdfaRenderer.serialize(ontology),
        OntologyFormat::BinaryRdf => {
            // Binary RDF's varint stream is not valid UTF-8 (bytes ≥ 0x80), so a
            // `String` cannot represent it losslessly — refuse instead of
            // corrupting the payload.  Use `save_to_bytes` / `save_file` for
            // this format (CR-008 / B5 binary-safe I/O).
            Err(Error::ontology_parsing(
                "Binary RDF cannot be represented as a String — use save_to_bytes() or save_file()",
            ))
        }
        OntologyFormat::Hdt => {
            // HDT is a binary format (B5 Slice 5): `$HDT` control chunks +
            // dictionary + bitmap sections are not valid UTF-8 in general.
            // Same contract as BinaryRDF: refuse, use `save_to_bytes` /
            // `save_file`.
            Err(Error::ontology_parsing(
                "HDT cannot be represented as a String — use save_to_bytes() or save_file()",
            ))
        }
        OntologyFormat::Auto => Err(Error::ontology_parsing(
            "Auto format must be resolved to a specific format for string serialization",
        )),
    }
}

/// Serialize an ontology to raw bytes in the given format.
///
/// This is the byte-oriented counterpart of [`save_to_string`] and the
/// correct path for binary formats: `BinaryRdf` emits its `BRDF` record
/// stream directly (never routed through a `String`, which would fail on
/// varint bytes ≥ 0x80) and `Hdt` builds a real binary HDT file (control
/// information + dictionary + bitmap triples); text formats are encoded as
/// UTF-8.
pub fn save_to_bytes(ontology: &Ontology, format: OntologyFormat) -> Result<Vec<u8>> {
    match format {
        OntologyFormat::BinaryRdf => rio::binary_rdf::BinaryRdfRenderer::new().serialize(ontology),
        OntologyFormat::Hdt => rio::hdt::HdtRenderer::new().serialize_to_bytes(ontology),
        other => Ok(save_to_string(ontology, other)?.into_bytes()),
    }
}

/// Save ontology to file using specified format
pub fn save_file<P: AsRef<Path>>(
    ontology: &Ontology,
    path: P,
    format: OntologyFormat,
) -> Result<()> {
    let path = path.as_ref();

    match format {
        OntologyFormat::OwlXml => owl_xml::save_file(ontology, path),
        OntologyFormat::Functional => functional::save_file(ontology, path),
        OntologyFormat::RdfXml => rdf_xml::save_file(ontology, path),
        OntologyFormat::Turtle => turtle::save_file(ontology, path),
        OntologyFormat::NTriples => ntriples::save_file(ontology, path),
        OntologyFormat::Manchester => {
            let renderer = manchester_renderer::ManchesterRenderer::new();
            let content = renderer.serialize(ontology)?;
            std::fs::write(path, content)
                .map_err(|e| Error::io(format!("Failed to write Manchester: {e}")))
        }
        OntologyFormat::Latex => latex::save_file(ontology, path),
        OntologyFormat::DL => dl_syntax::save_file(ontology, path),
        OntologyFormat::Krss => krss::save_file(ontology, path),
        OntologyFormat::Krss2 => krss::save_file_krss2(ontology, path),
        OntologyFormat::Obo => obo::writer::save_file(ontology, path),
        OntologyFormat::NQuads => rio::nquads::save_file(ontology, path),
        OntologyFormat::N3 => rio::n3::save_file(ontology, path),
        OntologyFormat::TriG => rio::trig::save_file(ontology, path),
        OntologyFormat::TriX => rio::trix::save_file(ontology, path),
        OntologyFormat::JsonLd => rio::jsonld::save_file(ontology, path),
        OntologyFormat::RdfJson => rio::rdf_json::save_file(ontology, path),
        OntologyFormat::Rdfa => rio::rdfa::save_file(ontology, path),
        OntologyFormat::BinaryRdf => rio::binary_rdf::save_file(ontology, path),
        OntologyFormat::Hdt => rio::hdt::save_file(ontology, path),
        OntologyFormat::Auto => {
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            let detected_format = match extension.to_lowercase().as_str() {
                "owl" | "owx" => OntologyFormat::OwlXml,
                "rdf" | "xml" => OntologyFormat::RdfXml,
                "ttl" => OntologyFormat::Turtle,
                "nt" => OntologyFormat::NTriples,
                "ofn" => OntologyFormat::Functional,
                "omn" | "man" => OntologyFormat::Manchester,
                "tex" | "latex" => OntologyFormat::Latex,
                "dl" => OntologyFormat::DL,
                "krss" => OntologyFormat::Krss,
                "krss2" => OntologyFormat::Krss2,
                "obo" => OntologyFormat::Obo,
                "nq" | "nquads" => OntologyFormat::NQuads,
                "n3" => OntologyFormat::N3,
                "trig" => OntologyFormat::TriG,
                "trix" => OntologyFormat::TriX,
                "jsonld" | "json-ld" => OntologyFormat::JsonLd,
                "rj" | "rjson" => OntologyFormat::RdfJson,
                "html" | "xhtml" => OntologyFormat::Rdfa,
                "brdf" => OntologyFormat::BinaryRdf,
                "hdt" => OntologyFormat::Hdt,
                "txt" => OntologyFormat::Functional,
                _ => OntologyFormat::OwlXml,
            };
            save_file(ontology, path, detected_format)
        }
    }
}

/// Save ontology to a gzip-compressed file using the specified format
pub fn save_file_gzip<P: AsRef<Path>>(
    ontology: &Ontology,
    path: P,
    format: OntologyFormat,
) -> Result<()> {
    // Byte path: binary formats (BinaryRDF) must not be routed through a String.
    let content = save_to_bytes(ontology, format)?;
    let file = std::fs::File::create(path.as_ref())
        .map_err(|e| Error::io(format!("Failed to create gzip file: {e}")))?;
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &content)
        .map_err(|e| Error::io(format!("Failed to write gzip content: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::io(format!("Failed to finalize gzip: {e}")))?;
    Ok(())
}

/// Serialize an ontology to an xz-compressed byte buffer using the given format.
pub fn save_to_string_xz(ontology: &Ontology, format: OntologyFormat) -> Result<Vec<u8>> {
    // Byte path: binary formats (BinaryRDF) must not be routed through a String.
    let content = save_to_bytes(ontology, format)?;
    let mut encoder = xz2::write::XzEncoder::new(Vec::new(), 6);
    std::io::Write::write_all(&mut encoder, &content)
        .map_err(|e| Error::io(format!("Failed to write xz content: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::io(format!("Failed to finalize xz compression: {e}")))
}

/// Save ontology to an xz-compressed file using the specified format.
///
/// The file content is the given format compressed with `.xz` (LZMA2).
pub fn save_file_xz<P: AsRef<Path>>(
    ontology: &Ontology,
    path: P,
    format: OntologyFormat,
) -> Result<()> {
    let bytes = save_to_string_xz(ontology, format)?;
    std::fs::write(path.as_ref(), bytes)
        .map_err(|e| Error::io(format!("Failed to write xz file: {e}")))?;
    Ok(())
}

/// Load an ontology from an xz-compressed file, auto-detecting the format.
pub fn load_xz<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let path = path.as_ref();
    let compressed =
        std::fs::read(path).map_err(|e| Error::io(format!("Failed to read xz file: {e}")))?;
    let mut decoder = xz2::read::XzDecoder::new(&compressed[..]);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut content)
        .map_err(|e| Error::io(format!("Failed to decompress xz: {e}")))?;

    let format =
        detect_format_from_content_public(path, &content).unwrap_or(OntologyFormat::Functional);
    let parser = ParserFactory::create_parser(format)?;
    parser.parse(&content)
}

/// Load the decompressed text of an xz-compressed ontology document.
pub fn load_xz_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let compressed =
        std::fs::read(path).map_err(|e| Error::io(format!("Failed to read xz file: {e}")))?;
    let mut decoder = xz2::read::XzDecoder::new(&compressed[..]);
    let mut content = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut content)
        .map_err(|e| Error::io(format!("Failed to decompress xz: {e}")))?;
    Ok(content)
}

/// Factory for creating parsers based on format
pub struct ParserFactory;

impl ParserFactory {
    /// Create a parser for the specified format
    pub fn create_parser(format: OntologyFormat) -> Result<Box<dyn Parser>> {
        Self::create_parser_with_rdf_version(format, RdfCompatibilityMode::Auto)
    }

    /// Create a parser for the specified format with RDF version mode
    pub fn create_parser_with_rdf_version(
        format: OntologyFormat,
        rdf_version: RdfCompatibilityMode,
    ) -> Result<Box<dyn Parser>> {
        match format {
            OntologyFormat::OwlXml => Ok(Box::new(OwlXmlParser::new())),
            OntologyFormat::Functional => Ok(Box::new(FunctionalParser::new())),
            OntologyFormat::RdfXml => {
                let config = match rdf_version {
                    RdfCompatibilityMode::RDF11 => rdf_xml::RdfXmlParserConfig {
                        rdf_version: rdf_xml::RdfVersionMode::RDF11,
                        ..Default::default()
                    },
                    RdfCompatibilityMode::RDF12 => rdf_xml::RdfXmlParserConfig {
                        rdf_version: rdf_xml::RdfVersionMode::RDF12,
                        parse_rdf_reifies: true,
                        ..Default::default()
                    },
                    RdfCompatibilityMode::RDFStar => rdf_xml::RdfXmlParserConfig {
                        rdf_version: rdf_xml::RdfVersionMode::RDF12,
                        reification_mode: rdf_xml::ReificationMode::ConvertToQuotedTriples,
                        ..Default::default()
                    },
                    RdfCompatibilityMode::Auto => rdf_xml::RdfXmlParserConfig::default(),
                };
                Ok(Box::new(rdf_xml::RdfXmlParser::with_config(config)))
            }
            OntologyFormat::Turtle => {
                let config = match rdf_version {
                    RdfCompatibilityMode::RDF11 => turtle::TurtleParserConfig {
                        rdf_version: turtle::RdfVersionMode::RDF11,
                        parse_rdf_star: false,
                        strict_rdf11_mode: true,
                        ..Default::default()
                    },
                    RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => {
                        turtle::TurtleParserConfig {
                            rdf_version: turtle::RdfVersionMode::RDFStar,
                            parse_rdf_star: true,
                            strict_rdf11_mode: false,
                            ..Default::default()
                        }
                    }
                    RdfCompatibilityMode::Auto => turtle::TurtleParserConfig::default(),
                };
                Ok(Box::new(turtle::TurtleParser::with_config(config)))
            }
            OntologyFormat::NTriples => {
                let config = match rdf_version {
                    RdfCompatibilityMode::RDF11 => ntriples::NTriplesConfig {
                        rdf_version: ntriples::RdfVersionMode::RDF11,
                        parse_rdf_star: false,
                        strict_rdf11_mode: true,
                        iri_validation_mode: crate::semantics::IriValidationMode::RFC3986,
                        validate_blank_nodes: false, // Lenient for compatibility
                    },
                    RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => {
                        ntriples::NTriplesConfig {
                            rdf_version: ntriples::RdfVersionMode::RDFStar,
                            parse_rdf_star: true,
                            strict_rdf11_mode: false,
                            iri_validation_mode: crate::semantics::IriValidationMode::RFC3987,
                            validate_blank_nodes: false, // Lenient for compatibility
                        }
                    }
                    RdfCompatibilityMode::Auto => ntriples::NTriplesConfig::default(),
                };
                Ok(Box::new(ntriples::NTriplesParser::with_config(config)))
            }
            OntologyFormat::Manchester => Ok(Box::new(ManchesterParser::new(
                ManchesterParserConfig::default(),
            ))),
            // Formats without a dedicated parser *struct* are bridged through
            // `FormatParser`, which dispatches to the format's free `parse`
            // function (the RIO formats, DL/KRSS/OBO).  This makes them
            // reachable from `ParserFactory`, `OntologyLoader::load_from_string`
            // and any other `Box<dyn Parser>` consumer.
            OntologyFormat::DL
            | OntologyFormat::Krss
            | OntologyFormat::Krss2
            | OntologyFormat::Obo
            | OntologyFormat::NQuads
            | OntologyFormat::N3
            | OntologyFormat::TriG
            | OntologyFormat::TriX
            | OntologyFormat::JsonLd
            | OntologyFormat::RdfJson
            | OntologyFormat::Rdfa
            | OntologyFormat::BinaryRdf
            | OntologyFormat::Hdt => Ok(Box::new(FormatParser { format })),
            OntologyFormat::Latex => Err(Error::ontology_parsing(
                "Format does not support Parser trait",
            )),
            OntologyFormat::Auto => Err(Error::ontology_parsing(
                "Auto format should be resolved before creating parser",
            )),
        }
    }

    /// Detect RDF version from content
    #[must_use]
    pub fn detect_rdf_version(content: &str) -> RdfCompatibilityMode {
        detect_rdf_version_from_content(content)
    }
}

/// Parser trait for ontology parsing
pub trait Parser {
    /// Parse ontology from string content
    fn parse(&self, input: &str) -> Result<Ontology>;

    /// Parse ontology from file
    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology>;
}

/// Serializer trait for ontology serialization
pub trait Serializer {
    /// Serialize ontology to string
    fn serialize(&self, ontology: &Ontology) -> Result<String>;

    /// Serialize ontology to file
    fn serialize_to_file(&self, ontology: &Ontology, path: &std::path::Path) -> Result<()>;
}

// Implement Parser trait for all parsers
impl Parser for OwlXmlParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        owl_xml::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        owl_xml::parse_file(path)
    }
}

impl Parser for FunctionalParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        functional::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        functional::parse_file(path)
    }
}

impl Parser for RdfXmlParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        rdf_xml::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        rdf_xml::parse_file(path)
    }
}

impl Parser for TurtleParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        turtle::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        turtle::parse_file(path)
    }
}

impl Parser for NTriplesParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        ntriples::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        ntriples::parse_file(path)
    }
}

impl Parser for ManchesterParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        let mut parser = self.clone();
        parser
            .parse_string(input)
            .map_err(|e| Error::ontology_parsing(format!("Manchester parsing error: {e:?}")))
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ontology_parsing(format!("Failed to read file: {e}")))?;
        self.parse(&content)
    }
}

impl Parser for DLSyntaxParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        dl_syntax::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ontology_parsing(format!("Failed to read file: {e}")))?;
        self.parse(&content)
    }
}

impl Parser for KRSSParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        krss::parse(input)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ontology_parsing(format!("Failed to read file: {e}")))?;
        self.parse(&content)
    }
}

/// Type-erased [`Parser`] bound to an explicit [`OntologyFormat`].
///
/// Bridges formats that have no dedicated parser struct (the RIO formats plus
/// DL/KRSS/OBO) into the `Parser` trait.  `parse_file` reads **bytes** so
/// binary formats (`BinaryRdf`) are never forced through a UTF-8 decode.
#[derive(Debug, Clone, Copy)]
pub struct FormatParser {
    /// The format this parser is bound to.
    pub format: OntologyFormat,
}

impl Parser for FormatParser {
    fn parse(&self, input: &str) -> Result<Ontology> {
        parse_str(input, self.format)
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        let bytes = std::fs::read(path)
            .map_err(|e| Error::io(format!("Failed to read {}: {e}", path.display())))?;
        parse_bytes(&bytes, self.format)
    }
}

// ── WriterConfig / OntologyStorer ────────────────────────────────────────────

/// Configuration for ontology serialization/writing.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    pub banner_comments: bool,
    pub indent_size: usize,
    pub save_anonymous_individuals: bool,
    pub use_prefixes: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            banner_comments: true,
            indent_size: 4,
            save_anonymous_individuals: true,
            use_prefixes: true,
        }
    }
}

/// A type-erased ontology storage backend.
pub trait OntologyStorer {
    fn can_store(&self, format: OntologyFormat) -> bool;
    fn store_ontology(
        &self,
        ontology: &Ontology,
        format: OntologyFormat,
        writer: &mut dyn std::io::Write,
        config: &WriterConfig,
    ) -> Result<()>;
}

impl<T> OntologyStorer for T
where
    T: Serializer,
{
    fn can_store(&self, _format: OntologyFormat) -> bool {
        true
    }

    fn store_ontology(
        &self,
        ontology: &Ontology,
        format: OntologyFormat,
        writer: &mut dyn std::io::Write,
        _config: &WriterConfig,
    ) -> Result<()> {
        if !self.can_store(format) {
            return Err(Error::Unsupported {
                message: format!("Format {format:?} is not supported by this storer"),
            });
        }
        // Byte path so binary formats (BinaryRDF) are written losslessly.
        let content = crate::parsers::save_to_bytes(ontology, format)?;
        writer
            .write_all(&content)
            .map_err(|e| Error::io(format!("Failed to write serialized ontology: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rdf_star_syntax() {
        let content = r#"
            @prefix ex: <http://example.org/> .
            << ex:Alice ex:knows ex:Bob >> ex:certainty "high" .
        "#;
        let version = detect_rdf_version_from_content(content);
        assert_eq!(version, RdfCompatibilityMode::RDFStar);
    }

    #[test]
    fn test_detect_rdf_12_reifies() {
        let content = r##"
            <?xml version="1.0"?>
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Description rdf:about="http://example.org/statement1">
                <rdf:reifies rdf:resource="#triple1"/>
              </rdf:Description>
            </rdf:RDF>
        "##;
        let version = detect_rdf_version_from_content(content);
        assert_eq!(version, RdfCompatibilityMode::RDF12);
    }

    #[test]
    fn test_detect_rdf_11_reification() {
        let content = r##"
            <?xml version="1.0"?>
            <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
              <rdf:Statement rdf:about="#stmt1">
                <rdf:subject rdf:resource="http://example.org/alice"/>
                <rdf:predicate rdf:resource="http://example.org/knows"/>
                <rdf:object rdf:resource="http://example.org/bob"/>
              </rdf:Statement>
            </rdf:RDF>
        "##;
        let version = detect_rdf_version_from_content(content);
        assert_eq!(version, RdfCompatibilityMode::RDF11);
    }

    #[test]
    fn test_parser_factory_with_rdf_version() {
        // Test that parser factory can create parsers with different RDF versions
        let turtle_parser = ParserFactory::create_parser_with_rdf_version(
            OntologyFormat::Turtle,
            RdfCompatibilityMode::RDFStar,
        );
        assert!(turtle_parser.is_ok());

        let rdf_xml_parser = ParserFactory::create_parser_with_rdf_version(
            OntologyFormat::RdfXml,
            RdfCompatibilityMode::RDF12,
        );
        assert!(rdf_xml_parser.is_ok());

        let ntriples_parser = ParserFactory::create_parser_with_rdf_version(
            OntologyFormat::NTriples,
            RdfCompatibilityMode::RDF11,
        );
        assert!(ntriples_parser.is_ok());
    }

    #[test]
    fn test_detect_rdf_version_ignores_comments() {
        // Ensure << >> in comments doesn't trigger RDF-star detection
        let content = r#"
            # This is a comment with << fake quoted triple >>
            @prefix ex: <http://example.org/> .
            ex:Alice ex:knows ex:Bob .
        "#;
        let version = detect_rdf_version_from_content(content);
        // Should be Auto since there's no actual RDF-star syntax
        assert_eq!(version, RdfCompatibilityMode::Auto);
    }
}
