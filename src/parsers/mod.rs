//! Ontology Parsers and Serializers Module
//!
//! This module contains parsers and serializers for various OWL 2 DL formats.

pub mod common;
pub mod functional;
pub mod manchester;
pub mod ntriples;
pub mod owl_xml;
pub mod rdf_xml;
pub mod turtle;
pub mod validation;

/// RDF compatibility mode for parsing RDF-based formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfCompatibilityMode {
    /// RDF 1.1 mode - standard reification only
    RDF11,
    /// RDF 1.2 mode - rdf:reifies support
    RDF12,
    /// RDF-star mode - quoted triples with << >>
    RDFStar,
    /// Auto-detect based on content
    Auto,
}

impl Default for RdfCompatibilityMode {
    fn default() -> Self {
        Self::Auto
    }
}

/// Error verbosity level for parser error messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorVerbosity {
    /// Minimal: just the error message
    Minimal,
    /// Standard: message + line/column information
    Standard,
    /// Detailed: full context stack and token information
    Detailed,
}

impl Default for ErrorVerbosity {
    fn default() -> Self {
        Self::Standard
    }
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
pub use common::{OntologyParser, OntologySerializer};
pub use functional::{
    FunctionalParser, FunctionalSyntaxSerializer, parse as parse_functional,
    parse_file as parse_functional_file, save_file as save_functional_file,
};
pub use manchester::{ManchesterParser, ManchesterParserConfig};
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

use crate::{
    Error, Result,
    ontology::{Ontology, OntologyFormat},
};
use std::path::Path;

/// Extract the first section from a CrossSyntax multi-format file
/// Returns the content of the first section (after the ### marker)
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
            } else {
                // We've reached the second section, stop
                break;
            }
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
        .map_err(|e| Error::io(format!("Failed to read file for format detection: {}", e)))?;

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

    // Check for Turtle syntax
    // Turtle uses @prefix and @base directives
    if trimmed.starts_with("@prefix")
        || trimmed.starts_with("@base")
        || (content.contains("@prefix") && content.contains("<http"))
        || (content.contains("rdf:type") && content.contains("owl:"))
    {
        return Ok(OntologyFormat::Turtle);
    }

    // Check for XML-based formats
    if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
        // Try to determine which XML type
        // Check for OWL/XML elements
        let owl_xml_elements = [
            "<Ontology",
            "<Declaration",
            "<Class",
            "<ObjectProperty",
            "<DataProperty",
            "<AnnotationProperty",
            "<Individual",
            "owl:Ontology",
            "<Import",
        ];
        if owl_xml_elements.iter().any(|&elem| content.contains(elem)) {
            return Ok(OntologyFormat::OwlXml);
        }
        // Check for RDF/XML
        if content.contains("rdf:RDF") || content.contains("<rdf:RDF") {
            return Ok(OntologyFormat::RdfXml);
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
        || content.contains("rdf:object") {
        return RdfCompatibilityMode::RDF11;
    }
    
    // Default to auto-detection (let the parser decide)
    RdfCompatibilityMode::Auto
}

/// Auto-detect format and parse file
pub fn parse_file_auto<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let format = match extension.to_lowercase().as_str() {
        "owl" | "owx" => OntologyFormat::OwlXml,
        "rdf" => OntologyFormat::RdfXml,
        "xml" => detect_format_from_content(path)?, // XML could be OWL/XML or RDF/XML
        "ttl" => OntologyFormat::Turtle,
        "nt" => OntologyFormat::NTriples,
        "ofn" => OntologyFormat::Functional,
        "omn" => OntologyFormat::Manchester,
        "man" => OntologyFormat::Manchester,
        "swrl" => OntologyFormat::Functional, // SWRL uses functional-like syntax
        "txt" => detect_format_from_content(path)?, // Could be any format
        _ => OntologyFormat::OwlXml,          // Default fallback
    };

    // Read the file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::io(format!("Failed to read file: {}", e)))?;

    // Check if this is a CrossSyntax file and extract first section
    let parsed_content = if content.trim().starts_with("###") {
        extract_first_crosssyntax_section(&content)
    } else {
        content.clone()
    };

    // Detect RDF version for RDF-based formats
    let rdf_version = detect_rdf_version_from_content(&content);

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
                RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => turtle::TurtleParserConfig {
                    rdf_version: turtle::RdfVersionMode::RDFStar,
                    parse_rdf_star: true,
                    strict_rdf11_mode: false,
                    ..Default::default()
                },
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
                },
                RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => ntriples::NTriplesConfig {
                    rdf_version: ntriples::RdfVersionMode::RDFStar,
                    parse_rdf_star: true,
                    strict_rdf11_mode: false,
                    iri_validation_mode: crate::semantics::IriValidationMode::RFC3987,
                },
                RdfCompatibilityMode::Auto => ntriples::NTriplesConfig::default(),
            };
            let parser = ntriples::NTriplesParser::with_config(config);
            parser.parse(&parsed_content)
        }
        OntologyFormat::Manchester => {
            let parser = ManchesterParser::new(ManchesterParserConfig::default());
            parser.parse(&parsed_content)
        }
        OntologyFormat::Auto => Err(Error::ontology_parsing(
            "Auto format should have been resolved",
        )),
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
        OntologyFormat::Manchester => Err(Error::ontology_parsing(
            "Manchester syntax serialization not yet implemented",
        )),
        OntologyFormat::Auto => {
            // Auto-detect from file extension
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

            let detected_format = match extension.to_lowercase().as_str() {
                "owl" | "owx" => OntologyFormat::OwlXml,
                "rdf" | "xml" => OntologyFormat::RdfXml,
                "ttl" => OntologyFormat::Turtle,
                "nt" => OntologyFormat::NTriples,
                "ofn" => OntologyFormat::Functional,
                "omn" | "man" => OntologyFormat::Manchester,
                "txt" => OntologyFormat::Functional, // Default .txt to Functional
                _ => OntologyFormat::OwlXml,         // Default fallback
            };

            save_file(ontology, path, detected_format)
        }
    }
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
                    },
                    RdfCompatibilityMode::RDFStar | RdfCompatibilityMode::RDF12 => {
                        ntriples::NTriplesConfig {
                            rdf_version: ntriples::RdfVersionMode::RDFStar,
                            parse_rdf_star: true,
                            strict_rdf11_mode: false,
                            iri_validation_mode: crate::semantics::IriValidationMode::RFC3987,
                        }
                    }
                    RdfCompatibilityMode::Auto => ntriples::NTriplesConfig::default(),
                };
                Ok(Box::new(ntriples::NTriplesParser::with_config(config)))
            }
            OntologyFormat::Manchester => Ok(Box::new(ManchesterParser::new(
                ManchesterParserConfig::default(),
            ))),
            OntologyFormat::Auto => Err(Error::ontology_parsing(
                "Auto format should be resolved before creating parser",
            )),
        }
    }

    /// Detect RDF version from content
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
            .map_err(|e| Error::ontology_parsing(&format!("Manchester parsing error: {:?}", e)))
    }

    fn parse_file(&self, path: &std::path::Path) -> Result<Ontology> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::ontology_parsing(&format!("Failed to read file: {}", e)))?;
        self.parse(&content)
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
