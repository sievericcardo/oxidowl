//! Common imports and type aliases for internal use
//!
//! This module provides a convenient way to import commonly used types and
//! functions throughout the codebase, reducing boilerplate import statements.

// Core error handling
pub use crate::{Error, Result};

// Core ontology types - most commonly used
pub use crate::ontology::{
    ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression,
    Ontology, OntologyRef, IRI, Class, ObjectProperty, DataProperty,
};

// Axiom types
pub use crate::ontology::axioms::{
    Axiom, AxiomId, AxiomTrait,
    ClassAssertionAxiom, ObjectPropertyAssertionAxiom, DataPropertyAssertionAxiom,
    SubClassOfAxiom, EquivalentClassesAxiom, DisjointClassesAxiom,
    SubObjectPropertyOfAxiom, EquivalentObjectPropertiesAxiom, DisjointObjectPropertiesAxiom,
    SubDataPropertyOfAxiom, EquivalentDataPropertiesAxiom, DisjointDataPropertiesAxiom,
    DeclarationAxiom,
};

// Query types - commonly used for query processing  
pub use crate::query::{
    ConjunctiveQuery, QueryAtom, QueryVariable, QueryEngine,
    ConjunctiveQueryResult, AdvancedQueryError,
    // Phase 2.1 Advanced Query Optimization
    AdvancedQueryOptimizer, DLQueryFeatureExtractor,
    PerformancePredictor, IntelligentIndexingSystem, PerformanceMonitor,
};

// Configuration
pub use crate::config::ReasonerConfig;

// Standard library imports commonly used throughout the codebase
pub use std::{
    collections::{HashMap, HashSet, BTreeMap, BTreeSet},
    sync::{Arc, RwLock, Mutex},
    time::{Duration, Instant},
    fmt::{Debug, Display},
};

// External crates commonly used
pub use log::{debug, info, warn, error, trace};
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