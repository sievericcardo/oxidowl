//! Reasoner module
//!
//! This module provides a modular reasoner implementation with clearly separated concerns.
//! The reasoner is split into several focused sub-modules:
//!
//! - `core`: Main reasoner structure and ontology loading
//! - `tasks`: Basic reasoning operations (consistency, satisfiability, subsumption)
//! - `classification`: Complex operations (classification, realization)
//! - `tableau`: Tableau algorithm integration and factories
//! - `queries`: SPARQL and OWLlink query processing
//! - `explanation`: Explanation services for reasoning results
//! - `statistics`: Performance metrics and statistics
//! - `results`: Result types for various reasoning operations

pub mod classification;
pub mod core;
pub mod explanation;
pub mod queries;
pub mod results;
pub mod statistics;
pub mod tableau;
pub mod tasks;

// Re-export the main types for backwards compatibility
// This allows existing code to continue using the same import paths
pub use self::{
    classification::ClassificationService,
    core::Reasoner,
    explanation::ExplanationService,
    queries::{OwllinkRequest, QueryProcessor, SparqlQuery, TriplePattern},
    results::{
        ClassificationResult, PropertyClassificationResult, RealizationResult, ReasoningResult,
    },
    statistics::ReasoningStatistics,
    tableau::{
        HyperTableauInterface, ReasoningTask, TableauAlgorithmInstance, TableauFactory,
        TableauRunner, TraditionalTableauRunner,
    },
    tasks::ReasoningTaskService,
};
