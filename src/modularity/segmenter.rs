//! Ontology Segmenter — high-level API for module extraction.

use super::extractor::{ModuleExtractor, ModuleExtractorConfig, ModuleType};
use super::locality::LocalityClass;
use crate::Result;
use crate::manager::OntologyManager;
use crate::ontology::axioms::Entity;
use crate::ontology::{IRI, OntologyRef};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// High-level API for extracting ontology modules given a signature.
pub struct OntologySegmenter {
    extractor: ModuleExtractor,
    manager: Arc<RwLock<OntologyManager>>,
}

impl OntologySegmenter {
    #[must_use]
    pub fn new(manager: Arc<RwLock<OntologyManager>>, module_type: ModuleType) -> Self {
        let config = ModuleExtractorConfig {
            module_type,
            max_iterations: 1000,
        };
        Self {
            extractor: ModuleExtractor::new_syntactic(LocalityClass::Star, config),
            manager,
        }
    }

    /// Extract a module for a given signature.
    pub fn segment(
        &self,
        ontology: &OntologyRef,
        signature: &HashSet<IRI>,
        _module_type: ModuleType,
    ) -> Result<OntologyRef> {
        let guard = ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("{e}"),
        })?;
        let module = self.extractor.extract_module(&guard, signature);
        let iri = guard
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:module"));
        drop(guard);

        let mut manager = self.manager.write().map_err(|e| crate::Error::Internal {
            message: format!("{e}"),
        })?;
        let onto_ref = manager.create_ontology_with_axioms(iri, module.axioms.clone());
        Ok(onto_ref)
    }

    /// Extract a module for a given set of entities.
    pub fn segment_by_entities(
        &self,
        ontology: &OntologyRef,
        entities: &[Entity],
        module_type: ModuleType,
    ) -> Result<OntologyRef> {
        let signature: HashSet<IRI> = entities.iter().map(|e| e.iri().clone()).collect();
        self.segment(ontology, &signature, module_type)
    }
}
