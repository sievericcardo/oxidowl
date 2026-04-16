//! Reasoner module
//!
//! This module provides a modular reasoner implementation with clearly separated concerns.
//! The reasoner is split into several focused sub-modules:
//!
//! - `core`: Main reasoner structure and ontology loading
//! - `tasks`: Basic reasoning operations (consistency, satisfiability, subsumption)
//! - `classification`: Complex operations (classification, realization)
//! - `tableau`: Tableau algorithm integration and factories
//! - `consistency`: Pre-consistency checking for fast inconsistency detection
//! - `queries`: SPARQL and `OWLlink` query processing
//! - `explanation`: Explanation services for reasoning results
//! - `statistics`: Performance metrics and statistics
//! - `results`: Result types for various reasoning operations

pub mod classification;
pub mod consistency;
pub mod core;
pub mod datatype_validation;
pub mod explanation;
pub mod hypertableau_adapter;
#[cfg(feature = "sparql-store")]
pub mod oxigraph_store;
pub mod parallel_classification;
pub mod parallel_tableau;
pub mod queries;
pub mod results;
pub mod statistics;
pub mod tableau;
pub mod tasks;
pub mod abox_rules;
#[cfg(feature = "sparql-store")]
pub mod tbox_rules;

// Re-export the main types for backwards compatibility
// This allows existing code to continue using the same import paths
pub use self::abox_rules::abox_classification_rules;
#[cfg(feature = "sparql-store")]
pub use self::tbox_rules::extract_owl_rules_from_tbox;
pub use self::{
    classification::ClassificationService,
    consistency::PreConsistencyChecker,
    core::Reasoner,
    datatype_validation::DatatypeValidator,
    explanation::ExplanationService,
    hypertableau_adapter::HypertableauRunner,
    parallel_classification::{
        ParallelClassificationScheduler, SubsumptionResult, SubsumptionTask,
    },
    queries::{OwllinkRequest, QueryProcessor, SparqlQuery, TriplePattern},
    results::{
        ClassificationResult, PropertyClassificationResult, RealizationResult, ReasoningResult,
    },
    statistics::ReasoningStatistics,
    tableau::{
        ReasoningTask, TableauAlgorithmInstance, TableauFactory, TableauRunner,
        TraditionalTableauRunner,
    },
    tasks::ReasoningTaskService,
};
