//! OWL Manager convenience facade — static methods for common operations.
//!
//! Models the pattern from OWL API v5's `OWLManager.createOWLOntologyManager()`.

use std::path::Path;

use crate::Result;
use crate::factory::DataFactory;
use crate::manager::{ManagerConfig, OntologyManager, OntologyManagerRef};
use crate::ontology::Ontology;
use crate::ontology::OntologyFormat;
use crate::parsers::{
    ManchesterParser, ManchesterParserConfig, parse_functional, parse_ntriples, parse_owl_xml,
    parse_rdf_xml, parse_turtle, save_file, save_to_string,
};

/// Static convenience entry point for common OWL API operations.
///
/// Models the pattern from OWL API v5's `OWLManager` — provides
/// quick access to create managers, data factories, and load/save
/// ontologies without manually wiring dependencies.
pub struct OWLManager;

impl OWLManager {
    /// Create a new ontology manager with default configuration.
    #[must_use]
    pub fn create_ontology_manager() -> OntologyManager {
        OntologyManager::new()
    }

    /// Create a new ontology manager with custom configuration.
    #[must_use]
    pub fn create_ontology_manager_with_config(config: ManagerConfig) -> OntologyManager {
        OntologyManager::new_with_config(config)
    }

    /// Create a concurrent (thread-safe) ontology manager.
    #[must_use]
    pub fn create_concurrent_manager() -> OntologyManagerRef {
        OntologyManager::new_concurrent()
    }

    /// Create a new data factory.
    #[must_use]
    pub fn create_data_factory() -> DataFactory {
        DataFactory::new()
    }

    /// Load an ontology from a file, auto-detecting the format.
    pub fn load_ontology_from_file<P: AsRef<Path>>(path: P) -> Result<Ontology> {
        crate::parsers::parse_file_auto(path)
    }

    /// Load an ontology from a string in the given format.
    pub fn load_ontology_from_string(content: &str, format: OntologyFormat) -> Result<Ontology> {
        match format {
            OntologyFormat::Functional => parse_functional(content),
            OntologyFormat::OwlXml => parse_owl_xml(content),
            OntologyFormat::RdfXml => parse_rdf_xml(content),
            OntologyFormat::Turtle => parse_turtle(content),
            OntologyFormat::NTriples => parse_ntriples(content),
            OntologyFormat::Manchester => {
                let mut parser = ManchesterParser::new(ManchesterParserConfig::default());
                parser.parse_string(content)
            }
            _ => Err(crate::Error::Unsupported {
                message: format!("Loading from string for {format:?} not supported"),
            }),
        }
    }

    /// Save an ontology to a file in the given format.
    pub fn save_ontology_to_file<P: AsRef<Path>>(
        ontology: &Ontology,
        path: P,
        format: OntologyFormat,
    ) -> Result<()> {
        save_file(ontology, path, format)
    }

    /// Save an ontology to a string in the given format.
    pub fn save_ontology_to_string(ontology: &Ontology, format: OntologyFormat) -> Result<String> {
        save_to_string(ontology, format)
    }
}
