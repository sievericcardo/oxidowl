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

pub mod adapter;
pub mod cache;
pub mod cache_lockfree;
pub mod cache_strategies;
pub mod config;
pub mod core;
pub mod debug;
pub mod distributed;
pub mod dl_clauses;
pub mod error;
pub mod factory;
pub mod functional_syntax_factory;
pub mod import;
pub mod inference;
pub mod manager;
pub mod modularity;
pub mod ontology;
pub mod owl_manager;
pub mod parsers;
pub mod performance;
pub mod prelude;
pub mod profiles;
pub mod profiling;
pub mod query;
pub mod reasoner_api;
pub mod reasoning;
pub mod render;
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
pub use crate::factory::functional_syntax::FunctionalSyntaxFactory;
pub use crate::import::{ImportDeclaration, ImportError, ImportManager};
pub use crate::manager::OntologyManager;
pub use crate::manager::changes::{
    ChangeAuditLog, ChangeData, ChangeRecord, OntologyChange, OntologyDocumentTarget,
};
pub use crate::manager::composite::{
    amalgamate_sub_class_axioms, convert_equivalent_to_sub_classes,
    convert_property_assertions_to_annotations, split_sub_class_axioms,
};
pub use crate::manager::convenience::OWLManager;
pub use crate::manager::history::ChangeHistory;
pub use crate::manager::iri_mapper::{
    AutoIRIMapper, CompositeIRIMapper, NonMappingOntologyIRIMapper, OntologyIRIMapper,
    SimpleIRIMapper, ZipIRIMapper,
};
pub use crate::manager::listeners::{
    ImportProgressListener, LoggingChangeListener, MissingImportListener, NoOpChangeListener,
    OntologyChangeListener, ReasonerChangeAwareListener,
};
pub use crate::manager::loader::OntologyLoader;
pub use crate::manager::loader_config::{LoaderConfig, MissingImportStrategy};
pub use crate::manager::sources::{
    FileDocumentSource, GzipDocumentSource, GzipStringDocumentSource, OntologyDocumentSource,
    StringDocumentSource, UrlDocumentSource,
};
pub use crate::manager::{
    ChangeApplied, ChangeBroadcastStrategy, ManagerConfig, OntologyManagerRef, Snapshot,
};
pub use crate::parsers::{
    DLSyntaxParser, DLSyntaxRenderer, ErrorVerbosity, FunctionalParser, FunctionalSyntaxSerializer,
    KRSSParser, KRSSRenderer, KRSSVariant, LatexRenderer, ManchesterParser, ManchesterParserConfig,
    ManchesterRenderer, NTriplesParser, NTriplesSerializer, OwlXmlParser, OwlXmlSerializer,
    ParserConfig, ParserFactory, RdfCompatibilityMode, RdfXmlParser, RdfXmlSerializer,
    SerializerConfig, SyntaxValidator, TurtleParser, TurtleSerializer, parse_functional_file,
    parse_ntriples_file, parse_owl_xml_file, parse_rdf_xml_file, parse_turtle_file, save_file,
    save_file_gzip, save_to_string,
};
pub use crate::profiles::{
    OWL2Profile as ProfileType, ProfileDetectionResult, ProfileValidationReport, ProfileValidator,
    ProfileViolation, ProfileViolationType, ValidationStatistics, el::ELValidator,
    el_reasoner::ELReasoner, rl_reasoner::RLReasoner, validator::OWL2ProfileValidator,
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
pub use crate::debug::definitions::DefinitionTracker;
pub use crate::debug::{BlackBoxOWLDebugger, DebuggerConfig, OWLDebugger};
pub use crate::error::{Error, Result};
pub use crate::explanation::blackbox::{BlackBoxConfig, BlackBoxExplanation};
pub use crate::explanation::converter::SatisfiabilityConverter;
pub use crate::explanation::generator::{Explanation as Justification, ExplanationGenerator};
pub use crate::explanation::hst::{HSTConfig, HSTExplanationGenerator};
pub use crate::explanation::ordering::{
    CompositeExplanationOrderer, ExplanationOrderer, ExplanationProgressMonitor,
    JustificationSizeOrderer, SilentExplanationProgressMonitor,
};
pub use crate::explanation::renderer::{
    ConciseExplanationRenderer, ExplanationRenderer, SilentExplanationRenderer,
};
pub use crate::inference::InferredAxiomGenerator;
pub use crate::inference::metrics::{
    AverageClassDepth, AverageNamedSuperclassCount, AverageNumberOfNamedSuperclasses,
    DLExpressivityMetric, MaxNamedSuperclassCount, MaxNumberOfNamedSuperclasses, MaximumClassDepth,
    Metric, NumberOfAnnotationAxioms, NumberOfAnnotationProperties,
    NumberOfAsymmetricPropertyAxioms, NumberOfAxioms, NumberOfAxiomsByType, NumberOfClasses,
    NumberOfDataProperties, NumberOfDatatypes, NumberOfDisjointClassesAxioms,
    NumberOfEquivalentClassAxioms, NumberOfFunctionalPropertyAxioms, NumberOfGCI,
    NumberOfGCIAxioms, NumberOfHasKeyAxioms, NumberOfHiddenGCI, NumberOfImports,
    NumberOfIndividuals, NumberOfInverseFunctionalPropertyAxioms,
    NumberOfIrreflexivePropertyAxioms, NumberOfLogicalAxioms, NumberOfObjectProperties,
    NumberOfSWRLRuleAxioms, NumberOfSWRLRules, NumberOfSubClassAxioms,
    NumberOfSymmetricPropertyAxioms, NumberOfTransitivePropertyAxioms, OntologyMetrics, OwlMetric,
    PerPropertyAxiomTypeCounts,
};
pub use crate::inference::{
    InferredClassAssertionAxiomGenerator, InferredDataPropertyAssertionGenerator,
    InferredDisjointClassesAxiomGenerator, InferredEquivalentClassAxiomGenerator,
    InferredEquivalentDataPropertyAxiomGenerator, InferredEquivalentObjectPropertyAxiomGenerator,
    InferredInverseObjectPropertiesAxiomGenerator, InferredObjectPropertyAssertionGenerator,
    InferredPropertyCharacteristicAxiomGenerator, InferredSubClassOfAxiomGenerator,
    InferredSubDataPropertyAxiomGenerator, InferredSubObjectPropertyAxiomGenerator,
};
pub use crate::modularity::decomposer::{AtomicDecomposer, DecomposerConfig};
pub use crate::modularity::decomposition::{Atom, AtomicDecomposition};
pub use crate::modularity::extractor::{ModuleExtractor, ModuleExtractorConfig, ModuleType};
pub use crate::modularity::locality::{
    LocalityClass, LocalityEvaluator, SyntacticLocalityEvaluator,
};
pub use crate::modularity::segmenter::OntologySegmenter;
pub use crate::ontology::axioms::{
    Axiom, AxiomTrait, AxiomType, EntityType, SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule,
    SWRLVariable,
};
pub use crate::ontology::datatypes::{
    ConstrainingFacet, DataRange, DatatypeCategory, DatatypeDefinitionAxiom, FacetRestriction,
    OWL2Datatype, OWLFacet,
};
pub use crate::ontology::shortform::{
    AnnotationValueShortFormProvider, BidirectionalShortFormProvider, OntologyIRIShortFormProvider,
    QNameShortFormProvider, ShortFormProvider, SimpleShortFormProvider,
};
pub use crate::ontology::vocabulary::{Namespaces, PrefixManager};
pub use crate::ontology::{
    Annotation, AnnotationProperty, AnnotationSubject, AnnotationValue, Class, ClassExpression,
    DataProperty, DataPropertyExpression, IRI, ImportsDeclaration, Individual, Literal,
    NamedIndividual, ObjectProperty, ObjectPropertyExpression, Ontology, OntologyFormat,
    OntologyID, OntologyRef, Signature,
};
pub use crate::parsers::obo::{
    OBOOutputConfig, OBOParser, OBOParserConfig, OBOWriter, Obo2Owl, Owl2Obo,
};
pub use crate::parsers::rio::{
    binary_rdf::{BinaryRdfParser, BinaryRdfRenderer},
    hdt::{HDTParser, HDTRenderer},
    jsonld::{JsonLdParser, JsonLdRenderer},
    n3::{N3Parser, N3Renderer},
    nquads::{NQuadsParser, NQuadsRenderer},
    rdf_json::{RdfJsonParser, RdfJsonRenderer},
    rdfa::RDFaParser,
    trig::{TriGParser, TriGRenderer},
    trix::{TriXParser, TriXRenderer},
};
pub use crate::reasoner_api::structural::{StructuralReasoner, StructuralReasonerFactory};
pub use crate::reasoner_api::{
    BufferingMode, FreshEntityPolicy, IndividualNodeSetPolicy, InferenceDepth, InferenceType, Node,
    NodeSet, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, ReasonerProgressMonitor,
    TableauOWLReasoner, TableauReasonerFactory,
};
pub use crate::render::{ConciseObjectRenderer, OWLObjectRenderer};
pub use crate::searcher::{EntityIndex, EntitySearcher};
pub use crate::transform::cnf::ClausalNormalFormConverter;
pub use crate::transform::expressivity::{DLExpressivity, DLExpressivityChecker};
pub use crate::transform::nnf::NNFConverter;
pub use crate::transform::{OWLEntityRemover, OWLEntityRenamer, OWLObjectTransformer};
pub use crate::visitor::{OntologyChangeVisitor, dispatch_change};
pub use crate::walk::merge::OWLOntologyMerger;
pub use crate::walk::{OWLObjectVisitor, OntologyWalker, StructureWalker};

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
