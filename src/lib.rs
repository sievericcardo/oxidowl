//! Oxidowl: A Rust OWL 2 DL reasoner
//!
// Suppress pedantic lints that represent intentional design decisions:
// - missing_errors_doc / missing_panics_doc: too many public functions to document individually
// - unused_self: kept for API extensibility and trait conformance
// - unnecessary_wraps: kept for API consistency across the codebase
// - cast_precision_loss: floating-point stats calculations accept minor precision loss
// - cast_possible_truncation / cast_sign_loss: SWRL datetime components and query counts
//   are small-range values; truncation is intentional (e.g., seconds < 60, months 0-11)
// - cast_possible_wrap: usize→i64 for string lengths / collection counts; values are
//   always well within i64::MAX range in any realistic ontology
// - unnecessary_literal_bound: lifetime parameter design choices
// - unused_async: async functions in trait impls / server APIs kept for API symmetry
// - too_many_lines: complex domain logic in single functions is acceptable
// - must_use_candidate: callers decide whether to use return values
// - similar_names: domain-specific naming (ci/cj, sub/sup, etc.) is intentional
// - if_not_else: both forms are equally readable in context
// - struct_excessive_bools: configuration structs with many flags are acceptable
// - self_only_used_in_recursion / items_after_statements: style preferences
// - wildcard_imports: used in internal modules for convenience
// - match_same_arms: some patterns require listing all arms for clarity
// - struct_field_names: domain-specific naming conventions
// - inline_always: performance-critical paths with explicit inline annotations
// - too_many_arguments: complex domain operations require many parameters
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::unused_self,
    clippy::unnecessary_wraps,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unnecessary_literal_bound,
    clippy::unused_async,
    clippy::too_many_lines,
    clippy::must_use_candidate,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::struct_excessive_bools,
    clippy::self_only_used_in_recursion,
    clippy::items_after_statements,
    clippy::wildcard_imports,
    clippy::match_same_arms,
    clippy::struct_field_names,
    clippy::inline_always,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::doc_markdown,    // match_wildcard_for_single_variants: wildcard arms on extensible enums are acceptable
    clippy::match_wildcard_for_single_variants,
    // ref_option: &Option<T> in many existing function signatures; refactoring callers is out-of-scope
    clippy::ref_option,
    // no_effect_underscore_binding: _name bindings are used to move ownership into closures
    clippy::no_effect_underscore_binding,
    // type_complexity: complex SPARQL async types in store APIs are acceptable
    clippy::type_complexity,
    // fn_params_excessive_bools: some domain configuration functions need many flags
    clippy::fn_params_excessive_bools,
    // enum_variant_names: domain naming with common prefix is intentional (e.g. GetXxx pattern)
    clippy::enum_variant_names,
    // large_enum_variant: boxing large enum variants is out-of-scope for this refactoring
    clippy::large_enum_variant,
    // missing_fields_in_debug: manually omitting fields from Debug impls is intentional
    clippy::missing_fields_in_debug,
    // format_collect: collecting formatted strings with map/format is the clearest approach
    clippy::format_collect,
    // default_trait_access: Default::default() is used where the type is clear from context
    clippy::default_trait_access,
    // non_std_lazy_statics: lazy_static! vocabulary module is widely used and stable
    clippy::non_std_lazy_statics,
)]
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
pub mod cache_lockfree;
pub mod cache_strategies;
pub mod config;
pub mod core;
pub mod debug;
pub mod dl_clauses;
pub mod error;
pub mod factory;
pub mod import;
pub mod manager;
pub mod modularity;
pub mod performance;
pub mod prelude;
pub mod profiling;
pub mod adapter;
pub mod distributed;
pub mod inference;
pub mod ontology;
pub mod parsers;
pub mod profiles;
pub mod query;
pub mod reasoning;
pub mod reasoner_api;
pub mod searcher;
pub mod semantics;
pub mod swrl;
pub mod transform;
pub mod validation;
pub mod visitor;
pub mod walk;

// Server interfaces (REST API, OWLlink, SPARQL)
#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "server")]
pub use server::ServerManager;

// Export explanation service (used by server)
pub mod explanation;

// Kani formal verification harnesses — compiled only under `cargo kani` or `--features kani`.
// Run with: cargo kani
#[cfg(any(kani, feature = "kani"))]
mod proofs;

// pub mod utils;

// Re-export main types for convenience
pub use crate::adapter::{HornedOwlAdapter, RdfStarCapable};
pub use crate::core::reasoner::Reasoner;
pub use crate::dl_clauses::{DLAtom, DLClause, DLClauseGenerator, DLClauseSet};
pub use crate::factory::DataFactory;
pub use crate::import::{ImportDeclaration, ImportError, ImportManager};
pub use crate::manager::OntologyManager;
pub use crate::manager::iri_mapper::{
    AutoIRIMapper, NonMappingOntologyIRIMapper, OntologyIRIMapper, SimpleIRIMapper,
};
pub use crate::manager::loader::OntologyLoader;
pub use crate::manager::changes::{ChangeData, OntologyChange};
pub use crate::manager::listeners::OntologyChangeListener;
pub use crate::manager::history::ChangeHistory;
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
pub use crate::validation::shacl::{
    ShaclConfig, ShaclSeverity, ShaclShape, ShaclValidationReport, ShaclValidationResult,
    ShaclValidator,
};
pub use crate::validation::{OWL2DLValidator, OWL2Profile, ValidationReport};

// Re-export distributed reasoning components
pub use crate::distributed::{
    ClusterConfig, DistributedConfig, DistributedQueryProcessor, NodeCapabilities, NodeConfig,
    NodeSettings,
};

// Re-export error types
pub use crate::config::{PerformanceConfig, PerformanceProfile, ReasonerConfig, TableauAlgorithm};
#[cfg(feature = "sparql-store")]
pub use crate::core::reasoner::extract_owl_rules_from_tbox;
pub use crate::core::reasoner::{ReasoningResult, ReasoningTask, abox_classification_rules};
pub use crate::error::{Error, Result};
pub use crate::ontology::{
    ClassExpression, IRI, Individual, Ontology, OntologyFormat, OntologyRef,
};
pub use crate::ontology::axioms::EntityType;
pub use crate::reasoner_api::{
    OWLReasoner, ReasonerFactory, TableauReasonerFactory, TableauOWLReasoner,
    Node, NodeSet, InferenceType, InferenceDepth,
    OWLReasonerConfiguration, BufferingMode, FreshEntityPolicy, IndividualNodeSetPolicy,
    ReasonerProgressMonitor,
};
pub use crate::reasoner_api::structural::{StructuralReasoner, StructuralReasonerFactory};
pub use crate::searcher::{EntityIndex, EntitySearcher};
pub use crate::transform::{OWLObjectTransformer, OWLEntityRenamer, OWLEntityRemover};
pub use crate::transform::nnf::NNFConverter;
pub use crate::transform::expressivity::{DLExpressivity, DLExpressivityChecker};
pub use crate::walk::{OWLObjectVisitor, OntologyWalker, StructureWalker};
pub use crate::walk::merge::OWLOntologyMerger;
pub use crate::inference::metrics::{OntologyMetrics, OwlMetric};
pub use crate::inference::InferredAxiomGenerator;
pub use crate::inference::{
    InferredSubClassOfAxiomGenerator, InferredEquivalentClassAxiomGenerator,
    InferredDisjointClassesAxiomGenerator, InferredClassAssertionAxiomGenerator,
    InferredSubObjectPropertyAxiomGenerator, InferredSubDataPropertyAxiomGenerator,
};
pub use crate::explanation::generator::{Explanation as Justification, ExplanationGenerator};
pub use crate::explanation::blackbox::{BlackBoxExplanation, BlackBoxConfig};
pub use crate::explanation::hst::{HSTExplanationGenerator, HSTConfig};
pub use crate::explanation::converter::SatisfiabilityConverter;
pub use crate::debug::{OWLDebugger, BlackBoxOWLDebugger, DebuggerConfig};
pub use crate::debug::definitions::DefinitionTracker;
pub use crate::modularity::decomposition::{Atom, AtomicDecomposition};
pub use crate::modularity::decomposer::{AtomicDecomposer, DecomposerConfig};
pub use crate::modularity::locality::{LocalityEvaluator, SyntacticLocalityEvaluator, LocalityClass};
pub use crate::modularity::extractor::{ModuleExtractor, ModuleExtractorConfig, ModuleType};
pub use crate::modularity::segmenter::OntologySegmenter;
pub use crate::parsers::obo::{OBOParser, OBOParserConfig, OBOWriter, OBOOutputConfig, Obo2Owl, Owl2Obo};
pub use crate::parsers::rio::{
    nquads::{NQuadsParser, NQuadsRenderer},
    n3::{N3Parser, N3Renderer},
    trig::{TriGParser, TriGRenderer},
    trix::{TriXParser, TriXRenderer},
    jsonld::{JsonLdParser, JsonLdRenderer},
    rdf_json::{RdfJsonParser, RdfJsonRenderer},
    rdfa::RDFaParser,
    binary_rdf::{BinaryRdfParser, BinaryRdfRenderer},
    hdt::{HDTParser, HDTRenderer},
};
pub use crate::ontology::datatypes::{DatatypeCategory, OWLFacet};
pub use crate::ontology::vocabulary::{Namespaces, PrefixManager};

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

    #[tokio::test]
    async fn test_create_query_engine() -> Result<()> {
        let ontology = Ontology::new();
        let _namespace = Some("http://example.com/test#".to_string());
        let config = ReasonerConfig::default();

        // Create a reasoning service from the ontology
        let reasoning_service = Arc::new(ReasoningService::new(ontology.clone(), config)?);
        let query_engine = query::DLQueryEngine::new(reasoning_service);

        // Test basic functionality
        assert_eq!(query_engine.get_namespace(), None); // Default namespace should be None
        Ok(())
    }
}
