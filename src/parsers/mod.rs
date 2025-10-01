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

use crate::{
    Error, Result,
    ontology::{Ontology, OntologyFormat},
};
use std::path::Path;

/// Auto-detect format and parse file
pub fn parse_file_auto<P: AsRef<Path>>(path: P) -> Result<Ontology> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    let format = match extension.to_lowercase().as_str() {
        "owl" | "owx" => OntologyFormat::OwlXml,
        "rdf" | "xml" => OntologyFormat::RdfXml,
        "ttl" => OntologyFormat::Turtle,
        "nt" => OntologyFormat::NTriples,
        "omn" | "txt" => OntologyFormat::Functional,
        "man" => OntologyFormat::Manchester,
        _ => OntologyFormat::OwlXml, // Default fallback
    };

    match format {
        OntologyFormat::OwlXml => owl_xml::parse_file(path),
        OntologyFormat::Functional => functional::parse_file(path),
        OntologyFormat::RdfXml => rdf_xml::parse_file(path),
        OntologyFormat::Turtle => turtle::parse_file(path),
        OntologyFormat::NTriples => ntriples::parse_file(path),
        OntologyFormat::Manchester => Err(Error::ontology_parsing(
            "Manchester syntax not yet implemented",
        )),
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
            "Manchester syntax not yet implemented",
        )),
        OntologyFormat::Auto => {
            // Auto-detect from file extension
            let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

            let detected_format = match extension.to_lowercase().as_str() {
                "owl" | "owx" => OntologyFormat::OwlXml,
                "rdf" | "xml" => OntologyFormat::RdfXml,
                "ttl" => OntologyFormat::Turtle,
                "nt" => OntologyFormat::NTriples,
                "omn" | "txt" => OntologyFormat::Functional,
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
