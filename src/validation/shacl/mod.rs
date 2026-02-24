//! SHACL (Shapes Constraint Language) implementation.
//!
//! Implements the full W3C SHACL Core and SHACL-SPARQL specifications
//! (https://www.w3.org/TR/shacl/).
//!
//! # Module layout
//!
//! | Module | Contents |
//! |--------|----------|
//! | `vocabulary` | All SHACL IRI constants |
//! | `model` | Internal data model types: shapes, paths, constraints, severity |
//! | `report` | `ShaclValidationReport` and `ShaclValidationResult` |
//! | `parser` | Shapes graph parser (Turtle → internal model) |
//! | `paths` | Path-to-SPARQL translation and path traversal |
//! | `targets` | Target resolution |
//! | `constraints` | Constraint component evaluators (one sub-module per category) |
//! | `sparql_constraints` | `sh:sparql` constraint evaluator |
//! | `sparql_components` | Custom SPARQL constraint component evaluator |
//! | `engine` | `ShaclValidator` orchestrator |
//!
//! # Quick start
//!
//! ```rust,no_run
//! use oxidowl::validation::shacl::engine::ShaclValidator;
//!
//! let shapes_ttl = r#"
//!   @prefix sh: <http://www.w3.org/ns/shacl#> .
//!   @prefix ex: <http://example.org/> .
//!
//!   ex:PersonShape a sh:NodeShape ;
//!     sh:targetClass ex:Person ;
//!     sh:property [
//!       sh:path ex:name ;
//!       sh:datatype xsd:string ;
//!       sh:minCount 1 ;
//!     ] .
//! "#;
//!
//! let data_ttl = r#"
//!   @prefix ex: <http://example.org/> .
//!   @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
//!   ex:Alice a ex:Person ; ex:name "Alice"^^xsd:string .
//!   ex:Bob   a ex:Person .   # missing ex:name → violation
//! "#;
//!
//! let mut validator = ShaclValidator::new(shapes_ttl, data_ttl).unwrap();
//! let report = validator.validate().unwrap();
//! assert!(!report.conforms);
//! ```

pub mod constraints;
pub mod engine;
pub mod model;
pub mod parser;
pub mod paths;
pub mod report;
pub mod sparql_components;
pub mod sparql_constraints;
pub mod targets;
pub mod vocabulary;

// Convenience re-exports
pub use engine::{ShaclConfig, ShaclValidator};
pub use model::{
    NodeShape, PropertyShape, ShaclConstraint, ShaclMessage, ShaclNodeKind, ShaclPath,
    ShaclSeverity, ShaclShape, ShaclTarget, SparqlConstraint,
};
pub use report::{ShaclValidationReport, ShaclValidationResult};
