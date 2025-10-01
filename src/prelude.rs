//! Common imports and type aliases for internal use
//!
//! This module provides a convenient way to import commonly used types and
//! functions throughout the codebase, reducing boilerplate import statements.

// Core error handling
pub use crate::{Error, Result};

// Core ontology types - most commonly used
pub use crate::ontology::{
    Class, ClassExpression, DataProperty, DataPropertyExpression, IRI, Individual, ObjectProperty,
    ObjectPropertyExpression, Ontology, OntologyRef,
};

// Axiom types
pub use crate::ontology::axioms::{
    Axiom, AxiomId, AxiomTrait, ClassAssertionAxiom, DataPropertyAssertionAxiom, DeclarationAxiom,
    DisjointClassesAxiom, DisjointDataPropertiesAxiom, DisjointObjectPropertiesAxiom,
    EquivalentClassesAxiom, EquivalentDataPropertiesAxiom, EquivalentObjectPropertiesAxiom,
    ObjectPropertyAssertionAxiom, SubClassOfAxiom, SubDataPropertyOfAxiom,
    SubObjectPropertyOfAxiom,
};

// Query types - commonly used for query processing
pub use crate::query::{
    AdvancedQueryError,
    // Phase 2.1 Advanced Query Optimization
    AdvancedQueryOptimizer,
    ConjunctiveQuery,
    ConjunctiveQueryResult,
    DLQueryFeatureExtractor,
    IntelligentIndexingSystem,
    PerformanceMonitor,
    PerformancePredictor,
    QueryAtom,
    QueryEngine,
    QueryVariable,
};

// Configuration
pub use crate::config::ReasonerConfig;

// Standard library imports commonly used throughout the codebase
pub use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::{Debug, Display},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

// External crates commonly used
pub use log::{debug, error, info, trace, warn};
pub use serde::{Deserialize, Serialize};

// Re-export some frequently used utility types
pub type SharedOntology = Arc<RwLock<Ontology>>;
pub type SharedConfig = Arc<ReasonerConfig>;

/// Commonly used result type for reasoning operations
pub type ReasoningResult<T> = Result<T>;

/// Type alias for cache keys
pub type CacheKey = String;

/// Type alias for entity IRIs
pub type EntityIRI = String;
