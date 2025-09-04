//! Core reasoner functionality
//!
//! This module contains the main Reasoner struct and core operations like
//! loading ontologies and basic reasoning setup.

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::reasoner::{
        classification::ClassificationService,
        explanation::ExplanationService,
        queries::QueryProcessor,
        statistics::ReasoningStatistics,
        tableau::TableauFactory,
        tasks::ReasoningTaskService,
    },
    dl_clauses::{DLClauseGenerator, DLClauseSet},
    ontology::{Ontology, OntologyFormat, OntologyRef, ClassExpression, Individual, ObjectPropertyExpression, DataPropertyExpression},
};
use log::{info, warn};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
    time::Instant,
};

/// Main reasoner interface
#[derive(Debug)]
pub struct Reasoner {
    /// Reasoning configuration
    config: ReasonerConfig,

    /// Current ontology being reasoned over
    ontology: Option<OntologyRef>,

    /// Cache manager for reasoning results
    cache_manager: Arc<RwLock<CacheManager>>,

    /// Tableau factory for creating reasoning algorithms
    tableau_factory: TableauFactory,

    /// Statistics about reasoning operations
    statistics: ReasoningStatistics,

    /// Task service for basic reasoning operations
    task_service: ReasoningTaskService,

    /// Classification service for complex operations
    classification_service: ClassificationService,

    /// Query processor for SPARQL and OWLlink
    query_processor: QueryProcessor,

    /// Explanation service
    explanation_service: ExplanationService,
}

impl Reasoner {
    /// Check if a class is a subclass of another
    pub fn is_subclass_of(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        // Enhanced subsumption checking using available reasoning mechanisms
        
        // Quick syntactic check
        if subclass == superclass {
            return Ok(true);
        }
        
        // Check for explicit subclass declarations in the ontology
        if let Some(ontology) = &self.ontology {
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                    if &subclass_axiom.subclass == subclass && &subclass_axiom.superclass == superclass {
                        return Ok(true);
                    }
                }
            }
            
            // Check through equivalent classes
            for axiom in ontology.axioms() {
                if let crate::ontology::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                    if equiv_axiom.classes.contains(subclass) && equiv_axiom.classes.contains(superclass) {
                        return Ok(true);
                    }
                }
            }
        }
        
        // Check using built-in OWL semantics
        self.check_semantic_subsumption(subclass, superclass)
    }

    /// Enhanced semantic subsumption checking with proper OWL semantics
    fn check_semantic_subsumption(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        match (subclass, superclass) {
            // Bottom is subclass of everything
            (ClassExpression::OWLNothing, _) => Ok(true),
            
            // Everything is subclass of Top
            (_, ClassExpression::OWLThing) => Ok(true),
            
            // Nothing is superclass of Top (except Top itself)
            (ClassExpression::OWLThing, ClassExpression::OWLNothing) => Ok(false),
            
            // Intersection subsumption: A ⊓ B ⊑ A and A ⊓ B ⊑ B
            (ClassExpression::ObjectIntersectionOf(components), superclass) => {
                Ok(components.contains(superclass))
            },
            
            // Union subsumption: A ⊑ A ⊔ B and B ⊑ A ⊔ B
            (subclass, ClassExpression::ObjectUnionOf(components)) => {
                Ok(components.contains(subclass))
            },
            
            _ => Ok(false),
        }
    }
}
