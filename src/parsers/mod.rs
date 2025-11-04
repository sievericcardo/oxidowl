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

/// Detect format from file content for ambiguous cases
fn detect_format_from_content<P: AsRef<Path>>(path: P) -> Result<OntologyFormat> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        Error::io(format!("Failed to read file for format detection: {}", e))
    })?;
    
    let trimmed = content.trim();
    
    // Check for Functional syntax - more patterns
    // Functional syntax uses parentheses and specific keywords
    if trimmed.starts_with("Ontology(") 
        || trimmed.starts_with("Prefix(")
        || trimmed.starts_with("Import(")
        || content.contains("Declaration(")
        || content.contains("SubClassOf(")
        || (trimmed.starts_with("Import(") && content.contains("Ontology(")) {
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
        || content.contains("\nDataProperty:") {
        return Ok(OntologyFormat::Manchester);
    }
    
    // Check for Turtle syntax
    // Turtle uses @prefix and @base directives
    if trimmed.starts_with("@prefix") 
        || trimmed.starts_with("@base")
        || (content.contains("@prefix") && content.contains("<http"))
        || (content.contains("rdf:type") && content.contains("owl:")) {
        return Ok(OntologyFormat::Turtle);
    }
    
    // Check for XML-based formats
    if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
        // Try to determine which XML type
        // Check for OWL/XML elements
        let owl_xml_elements = ["<Ontology", "<Declaration", "<Class", "<ObjectProperty", 
                                 "<DataProperty", "<AnnotationProperty", "<Individual",
                                 "owl:Ontology", "<Import"];
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
    let functional_keywords = ["Declaration", "SubClassOf", "EquivalentClasses", "DisjointClasses"];
    if functional_keywords.iter().any(|&kw| content.contains(kw)) && content.contains('(') {
        return Ok(OntologyFormat::Functional);
    }
    
    // Check if it looks like Manchester (colon-based declarations)
    let manchester_keywords = ["Class:", "ObjectProperty:", "DataProperty:", "SubClassOf:", "EquivalentTo:"];
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
        _ => OntologyFormat::OwlXml, // Default fallback
    };

    match format {
        OntologyFormat::OwlXml => owl_xml::parse_file(path),
        OntologyFormat::Functional => functional::parse_file(path),
        OntologyFormat::RdfXml => rdf_xml::parse_file(path),
        OntologyFormat::Turtle => turtle::parse_file(path),
        OntologyFormat::NTriples => ntriples::parse_file(path),
        OntologyFormat::Manchester => {
            // Use Manchester parser
            let content = std::fs::read_to_string(path).map_err(|e| {
                Error::io(format!("Failed to read Manchester file: {}", e))
            })?;
            let parser = ManchesterParser::new(ManchesterParserConfig::default());
            parser.parse(&content)
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
                _ => OntologyFormat::OwlXml, // Default fallback
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
        match format {
            OntologyFormat::OwlXml => Ok(Box::new(OwlXmlParser::new())),
            OntologyFormat::Functional => Ok(Box::new(FunctionalParser::new())),
            OntologyFormat::RdfXml => Ok(Box::new(RdfXmlParser::new())),
            OntologyFormat::Turtle => Ok(Box::new(TurtleParser::new())),
            OntologyFormat::NTriples => Ok(Box::new(NTriplesParser::new())),
            OntologyFormat::Manchester => Ok(Box::new(ManchesterParser::new(
                ManchesterParserConfig::default(),
            ))),
            OntologyFormat::Auto => Err(Error::ontology_parsing(
                "Auto format should be resolved before creating parser",
            )),
        }
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
