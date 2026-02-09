//! Oxidowl: A Rust OWL 2 DL reasoner
//!
//! This crate provides a complete Description Logic reasoner for SROIQV(D),
//! supporting nearly all features of OWL 2 DL. It maintains the architecture
//! and behavior of the original C++ implementation while leveraging Rust's
//! memory safety and performance characteristics.
//!
//! # Main Components
//!
//! - [`core`] - Core reasoning engine with tableau algorithms
//! - [`ontology`] - Ontology representation and management
//! - [`parsers`] - Input format parsers (OWL XML, Functional, RDF)
//! - [`reasoning`] - High-level reasoning tasks and coordination
//! - [`network`] - HTTP servers for `OWLlink` and SPARQL (to be implemented)
//! - [`config`] - Configuration management
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use oxidowl::{Reasoner, ReasonerConfig, OntologyFormat};
//!
//! // Create a reasoner with default configuration
//! let config = ReasonerConfig::default();
//! let mut reasoner = Reasoner::new(config)?;
//!
//! // Load an ontology
//! reasoner.load_ontology_from_file("example.owl", OntologyFormat::OwlXml)?;
//!
//! // Check consistency
//! let is_consistent = reasoner.is_consistent()?;
//! println!("Ontology is consistent: {}", is_consistent);
//!
//! // Perform classification
//! let class_hierarchy = reasoner.classify()?;
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cache;
pub mod cache_lockfree; // Lock-free cache using DashMap
pub mod cache_strategies; // Advanced cache eviction strategies (LRU, LFU)
pub mod config;
pub mod core;
pub mod dl_clauses; // DL clause generation and dumping
pub mod error;
pub mod import;
pub mod performance; // Performance monitoring and profiling
pub mod profiling; // Flamegraph and heap profiling infrastructure
pub mod prelude; // Common imports and type aliases for internal use // Import management and dependency resolution
// pub mod network;
pub mod adapter; // Horned-OWL adapter for enhanced parsing
pub mod distributed;
pub mod ontology;
// pub mod ontology_lockfree; // Lock-free ontology access using ArcSwap - TODO: Create file
pub mod parsers;
pub mod profiles; // OWL 2 profiles support and validation
pub mod query;
pub mod reasoning;
pub mod semantics; // RDF, RDFS, and OWL 2 semantics implementation
pub mod swrl; // SWRL (Semantic Web Rule Language) support
pub mod validation; // OWL 2 DL validation and profile checking
pub mod visitor; // Visitor pattern for ontology traversal // Distributed query processing and cluster management

// Server interfaces (REST API, OWLlink, SPARQL)
#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
pub use server::ServerManager;

// Export explanation service (used by server)
pub mod explanation;

// pub mod utils;

// Re-export main types for convenience
pub use crate::core::reasoner::Reasoner;
pub use crate::dl_clauses::{DLAtom, DLClause, DLClauseGenerator, DLClauseSet};
pub use crate::import::{ImportDeclaration, ImportError, ImportManager};
pub use crate::profiles::{
    OWL2Profile as ProfileType, ProfileValidator, el::ELValidator, validator::OWL2ProfileValidator,
};
// Query system exports (both DL queries and advanced conjunctive queries)
pub use crate::query::{
    AdvancedQueryError,
    // Phase 2.1 Advanced Optimization exports
    AdvancedQueryOptimizer,
    // Advanced Query exports - re-exported from advanced module
    ConjunctiveQuery,
    ConjunctiveQueryResult,
    // DL Query exports for backward compatibility
    DLQuery,
    DLQueryEngine,
    DLQueryFeatureExtractor,
    DLQueryParser,
    IntelligentIndexingSystem,
    PerformanceMonitor,
    PerformancePredictor,
    QueryAtom,
    QueryEngine,
    QueryError,
    QueryResult,
    QueryService,
    QueryType,
};
pub use crate::reasoning::ReasoningService;
pub use crate::swrl::{BuiltInRegistry, SWRLInterpreter, SWRLRuleEngine, SWRLValidator};
pub use crate::validation::{OWL2DLValidator, OWL2Profile, ValidationReport};

// Re-export distributed reasoning components
pub use crate::distributed::{
    ClusterConfig, DistributedConfig, DistributedQueryProcessor, NodeCapabilities, NodeConfig,
    NodeSettings,
};

// Re-export error types
pub use crate::config::{ReasonerConfig, TableauAlgorithm, PerformanceConfig, PerformanceProfile};
pub use crate::core::reasoner::{ReasoningResult, ReasoningTask};
pub use crate::error::{Error, Result};
pub use crate::ontology::{
    ClassExpression, IRI, Individual, Ontology, OntologyFormat, OntologyRef,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "Oxidowl";
pub const DESCRIPTION: &str = "Rust OWL 2 DL reasoner";

/// Build information
#[must_use]
pub fn version_info() -> String {
    format!("{NAME} - {DESCRIPTION}, Version {VERSION}")
}

/// Get supported description logic expressivities
#[must_use]
pub fn supported_expressivities() -> Vec<&'static str> {
    vec![
        "ALC", "ALCH", "ALCHI", "ALCHIQ", "ALCHIF", "ALCHIQ", "SHIQ", "SHIF", "SHIN", "SHOIN",
        "SROIQ", "SROIQV",
    ]
}

/// Check if a specific expressivity is supported
#[must_use]
pub fn supports_expressivity(expressivity: &str) -> bool {
    supported_expressivities().contains(&expressivity)
}

// Removed convenience functions temporarily due to type system complexity
// They will be implemented in a future version with proper error handling

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains("Oxidowl"));
        assert!(info.contains(VERSION));
    }

    #[test]
    fn test_supported_expressivities() {
        let expressivities = supported_expressivities();
        assert!(expressivities.contains(&"SROIQ"));
        assert!(expressivities.contains(&"ALC"));
    }

    #[test]
    fn test_supports_expressivity() {
        assert!(supports_expressivity("SROIQ"));
        assert!(supports_expressivity("ALC"));
        assert!(!supports_expressivity("UNKNOWN"));
    }

    #[test]
    fn test_create_query_engine() -> Result<()> {
        let ontology = Ontology::new();
        let _namespace = Some("http://example.com/test#".to_string());
        let config = ReasonerConfig::default();

        // Create a reasoning service from the ontology
        let reasoning_service = Arc::new(ReasoningService::new(ontology.clone(), config));
        let query_engine = query::DLQueryEngine::new(reasoning_service);

        // Test basic functionality
        assert_eq!(query_engine.get_namespace(), None); // Default namespace should be None
        Ok(())
    }
}
