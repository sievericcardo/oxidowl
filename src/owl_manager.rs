//! Static entry-point facade — equivalent to OWL API's `OWLManager`.
//!
//! Provides convenience methods for common workflows without requiring
//! manual manager/factory/loader wiring.

use crate::Result;
use crate::factory::DataFactory;
use crate::manager::OntologyManager;
use crate::ontology::{Ontology, OntologyFormat, OntologyRef};
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Central static entry point for common OWL API operations.
///
/// Mirrors OWL API v5's `OWLManager` class, providing static methods
/// for creating managers, factories, and loading/saving ontologies.
pub struct OWLManager;

impl OWLManager {
    /// Create a new ontology manager with default configuration.
    #[must_use]
    pub fn create_ontology_manager() -> OntologyManager {
        OntologyManager::new()
    }

    /// Create a new concurrent-ready ontology manager wrapped in `Arc<RwLock<>>`.
    #[must_use]
    pub fn create_concurrent_manager() -> Arc<RwLock<OntologyManager>> {
        Arc::new(RwLock::new(OntologyManager::new()))
    }

    /// Create a shared data factory with entity interning.
    #[must_use]
    pub fn create_data_factory() -> DataFactory {
        DataFactory::new()
    }

    /// Load an ontology from a file, auto-detecting the format.
    pub fn load_ontology(path: &Path) -> Result<Ontology> {
        crate::parsers::parse_owl_xml_file(path)
    }

    /// Load an ontology from a file into a shared reference.
    pub fn load_ontology_ref(path: &Path) -> Result<OntologyRef> {
        let ontology = Self::load_ontology(path)?;
        Ok(Arc::new(RwLock::new(ontology)))
    }

    /// Save an ontology to a file in the specified format.
    pub fn save_ontology(ontology: &Ontology, path: &Path, format: OntologyFormat) -> Result<()> {
        crate::parsers::save_file(ontology, path, format)
    }

    /// Save an ontology reference to a file.
    pub fn save_ontology_ref(
        ontology_ref: &OntologyRef,
        path: &Path,
        format: OntologyFormat,
    ) -> Result<()> {
        let guard = ontology_ref
            .read()
            .map_err(|e| crate::Error::internal(e.to_string()))?;
        Self::save_ontology(&guard, path, format)
    }

    /// Create a new empty ontology with the given IRI.
    #[must_use]
    pub fn create_ontology() -> Ontology {
        Ontology::new()
    }
}
