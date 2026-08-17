//! DefinitionTracker — tracks class definitions in an ontology.

use crate::ontology::axioms::AxiomTrait;
use crate::ontology::axioms::*;
use crate::ontology::{ClassExpression, IRI, Ontology};
use std::collections::HashMap;

/// Tracks how each class is defined in terms of axioms.
#[derive(Debug, Clone, Default)]
pub struct DefinitionTracker {
    /// Class IRI → axioms that define this class.
    definitions: HashMap<IRI, Vec<Axiom>>,
}

impl DefinitionTracker {
    /// Build the tracker from an ontology.
    #[must_use]
    pub fn from_ontology(ontology: &Ontology) -> Self {
        let mut tracker = Self::default();
        for axiom in ontology.axioms() {
            tracker.index_axiom(axiom);
        }
        tracker
    }

    fn index_axiom(&mut self, axiom: &Axiom) {
        match axiom {
            Axiom::EquivalentClasses(a) => {
                for ce in &a.classes {
                    if let ClassExpression::Class(cls) = ce {
                        self.definitions
                            .entry(cls.iri.clone())
                            .or_default()
                            .push(axiom.clone());
                    }
                }
            }
            Axiom::SubClassOf(a) => {
                // Track both sides: definition can be on subclass side
                if let ClassExpression::Class(cls) = &a.subclass {
                    self.definitions
                        .entry(cls.iri.clone())
                        .or_default()
                        .push(axiom.clone());
                }
            }
            Axiom::DisjointUnion(a) => {
                if let ClassExpression::Class(cls) = &a.class {
                    self.definitions
                        .entry(cls.iri.clone())
                        .or_default()
                        .push(axiom.clone());
                }
            }
            _ => {}
        }
    }

    /// Get the axioms that define a class.
    #[must_use]
    pub fn get_definition(&self, class_iri: &IRI) -> Option<&Vec<Axiom>> {
        self.definitions.get(class_iri)
    }

    /// Check if a class is defined by a specific axiom.
    #[must_use]
    pub fn is_defined_by(&self, class_iri: &IRI, axiom: &Axiom) -> bool {
        self.definitions
            .get(class_iri)
            .is_some_and(|axs| axs.iter().any(|a| a.axiom_id() == axiom.axiom_id()))
    }

    /// Get all classes tracked.
    #[must_use]
    pub fn defined_classes(&self) -> Vec<&IRI> {
        self.definitions.keys().collect()
    }
}
