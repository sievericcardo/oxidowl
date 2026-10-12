//! Oxidowl: A Rust OWL 2 DL reasoner
//!
// Deliberately small keep-list of pedantic lints that encode intentional design
// decisions. Everything removed from this list is fixed (or thresholded in
// clippy.toml) rather than blanket-allowed.
#![allow(
    clippy::missing_errors_doc,          // large public API; error docs tracked separately
    clippy::unused_self,                 // methods kept on self for API/trait conformance
    clippy::unnecessary_wraps,           // Result/Option kept for API consistency
    clippy::cast_precision_loss,         // numeric stats accept minor precision loss
    clippy::cast_possible_truncation,    // intentional small-range casts (datetimes, counts)
    clippy::cast_sign_loss,              // intentional small-range casts
    clippy::unused_async,                // async kept for API symmetry
    clippy::unused_async_trait_impl,     // async trait-impls required by the trait contract; cannot be de-async'd
    clippy::match_same_arms,             // explicit arms preferred for clarity
    clippy::format_push_string,          // push_str(&format!(..)) is clearest here
    clippy::needless_pass_by_value,      // signatures fixed by API contract
    clippy::struct_excessive_bools,      // config/CLI structs legitimately have many bools
    clippy::self_only_used_in_recursion, // recursive helper shape
    clippy::ref_option,                  // &Option<T> in established API signatures
    clippy::inline_always,               // performance-critical explicit inlining
    clippy::struct_field_names,          // domain naming
    clippy::too_many_lines,              // complex domain logic (tableau/parsers); splitting these is high-risk
    clippy::large_enum_variant,          // changing variant sizes would break the public API
)]
// Guardrails: forbid vacuous/placeholder assertions in tests so that reasoning
// behaviour is always genuinely verified.
#![warn(
    clippy::assertions_on_constants,
    clippy::nonminimal_bool,
    clippy::absurd_extreme_comparisons
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
    DLSyntaxParser, DLSyntaxRenderer, ErrorVerbosity, FormatParser, FunctionalParser,
    FunctionalSyntaxSerializer, KRSSParser, KRSSRenderer, KRSSVariant, LatexRenderer,
    ManchesterParser, ManchesterParserConfig, ManchesterRenderer, NTriplesParser,
    NTriplesSerializer, OwlXmlParser, OwlXmlSerializer, ParserConfig, ParserFactory,
    RdfCompatibilityMode, RdfXmlParser, RdfXmlSerializer, SerializerConfig, SyntaxValidator,
    TurtleParser, TurtleSerializer, parse_bytes, parse_functional_file, parse_ntriples_file,
    parse_owl_xml_file, parse_rdf_xml_file, parse_str, parse_turtle_file, save_file,
    save_file_gzip, save_to_bytes, save_to_string,
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
    ConstrainingFacet, DatatypeCategory, DatatypeDefinitionAxiom, OWL2Datatype, OWLFacet,
};
pub use crate::ontology::shortform::{
    AnnotationValueShortFormProvider, BidirectionalShortFormProvider, OntologyIRIShortFormProvider,
    QNameShortFormProvider, ShortFormProvider, SimpleShortFormProvider,
};
pub use crate::ontology::vocabulary::{Namespaces, PrefixManager};
pub use crate::ontology::{
    Annotation, AnnotationProperty, AnnotationSubject, AnnotationValue, Class, ClassExpression,
    DataProperty, DataPropertyExpression, DataRange, FacetRestriction, IRI, ImportsDeclaration,
    Individual, Literal, NamedIndividual, ObjectProperty, ObjectPropertyExpression, Ontology,
    OntologyFormat, OntologyID, OntologyRef, Signature,
};
pub use crate::parsers::obo::{
    OBOOutputConfig, OBOParser, OBOParserConfig, OBOWriter, Obo2Owl, Owl2Obo,
};
pub use crate::parsers::rio::{
    binary_rdf::{BinaryRdfParser, BinaryRdfRenderer},
    hdt::{HdtParser, HdtRenderer},
    jsonld::{JsonLdParser, JsonLdRenderer},
    n3::{N3Parser, N3Renderer},
    nquads::{NQuadsParser, NQuadsRenderer},
    rdf_json::{RdfJsonParser, RdfJsonRenderer},
    rdfa::{RdfaParser, RdfaRenderer},
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
