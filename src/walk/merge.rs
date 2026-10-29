//! OWL Ontology Merger — combines multiple ontologies into one.

use crate::Result;
use crate::manager::OntologyManager;
use crate::ontology::axioms::AxiomTrait;
use crate::ontology::{IRI, Ontology, OntologyRef};
use std::collections::HashSet;

/// Merges multiple source ontologies into a single target ontology.
pub struct OWLOntologyMerger {
    target_iri: IRI,
    copy_annotations: bool,
    create_imports: bool,
}

impl OWLOntologyMerger {
    /// Create a merger that produces an ontology with the given target IRI.
    #[must_use]
    pub fn new(target_iri: IRI) -> Self {
        Self {
            target_iri,
            copy_annotations: true,
            create_imports: false,
        }
    }

    /// Whether to copy annotations from source ontologies.
    #[must_use]
    pub fn with_copy_annotations(mut self, yes: bool) -> Self {
        self.copy_annotations = yes;
        self
    }

    /// Whether to create owl:imports declarations to the sources.
    #[must_use]
    pub fn with_create_imports(mut self, yes: bool) -> Self {
        self.create_imports = yes;
        self
    }

    /// Merge all source ontologies into a new ontology.
    pub fn merge(
        &self,
        sources: &[OntologyRef],
        manager: &mut OntologyManager,
    ) -> Result<OntologyRef> {
        let mut merged = Ontology::new();
        merged.set_iri(self.target_iri.clone());

        let mut seen_axiom_ids = HashSet::new();

        for source_ref in sources {
            let guard = source_ref.read().map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;

            // Copy axioms (deduplicate by ID)
            for axiom in guard.axioms() {
                let id = axiom.axiom_id();
                if seen_axiom_ids.insert(id) {
                    merged.add_axiom(axiom.clone());
                }
            }

            // Optionally copy annotations
            if self.copy_annotations {
                for ann in &guard.annotations {
                    merged.annotations.push(ann.clone());
                }
            }

            // Optionally create import declarations
            if self.create_imports
                && let Some(iri) = guard.get_iri()
            {
                merged.imports.push(crate::ontology::ImportsDeclaration {
                    imported_ontology_iri: iri.clone(),
                });
            }
        }

        let ont_ref =
            manager.create_ontology_with_axioms(self.target_iri.clone(), merged.axioms.clone());
        Ok(ont_ref)
    }
}
